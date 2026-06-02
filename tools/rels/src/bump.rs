//! `rels bump` — ripple a `bazel_dep` version pin across every
//! sibling rules_* repo that depends on the named module.
//!
//! Walks `<workspaces_root>/<repo>/MODULE.bazel` for every entry,
//! rewrites the version string inside the matching `bazel_dep(...)`
//! call, and (unless `--no-test`) runs `bazel test //...` in each
//! touched repo so failures surface immediately.
//!
//! Intentionally does NOT commit. The operator reviews the diff
//! per repo and commits manually — `bump` only does the
//! mechanical part. This keeps the tool composable with the
//! per-repo PR flow (review, CI, merge) instead of bypassing it.
//!
//! Multi-line `bazel_dep` blocks are handled — the regex matches
//! both
//!
//!     bazel_dep(name = "rules_x", version = "0.1.0")
//!
//! and
//!
//!     bazel_dep(
//!         name = "rules_x",
//!         version = "0.1.0",
//!     )
//!
//! and the `dev_dependency = True` variant either way.

use std::fs;
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use regex::Regex;

use crate::common::Env;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Module name to bump (e.g. `rules_jsonschema`).
    #[arg(long)]
    pub module: String,

    /// Version to pin to (e.g. `0.2.0`).
    #[arg(long)]
    pub to: String,

    /// Print the planned edits without writing them or running tests.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip `bazel test //...` after rewriting.
    #[arg(long)]
    pub no_test: bool,
}

pub fn run(env: &Env, args: Args) -> Result<()> {
    let edits = scan_workspaces(env, &args.module, &args.to)?;

    if edits.is_empty() {
        eprintln!("rels bump: no sibling repo depends on {}", args.module);
        return Ok(());
    }

    eprintln!(
        "Bumping {} to {} across {} repo(s)...",
        args.module,
        args.to,
        edits.len(),
    );
    for edit in &edits {
        eprintln!(
            "  {}: {} → {}{}",
            edit.repo,
            edit.from_version,
            args.to,
            if edit.was_dev_dep { " (dev_dependency)" } else { "" },
        );
    }

    if args.dry_run {
        eprintln!("dry-run: no files written.");
        return Ok(());
    }

    // Apply edits.
    for edit in &edits {
        fs::write(&edit.module_bazel, &edit.new_content)
            .with_context(|| format!("write {}", edit.module_bazel.display()))?;
    }
    eprintln!("Rewrote MODULE.bazel in {} repo(s).", edits.len());

    if args.no_test {
        eprintln!("--no-test: skipping `bazel test //...` per repo.");
        return Ok(());
    }

    // Run tests per touched repo. Report PASS/FAIL with elapsed time
    // so it's clear which bumps were verified.
    let mut any_failed = false;
    eprintln!("Running `bazel test //...` per repo:");
    for edit in &edits {
        let started = Instant::now();
        let status = Command::new("bazel")
            .args(["test", "//...", "--test_output=errors"])
            .current_dir(env.checkout_path(&edit.repo))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        let elapsed = started.elapsed();
        let elapsed_s = format!("{:.0}s", elapsed.as_secs_f64());
        match status {
            Ok(s) if s.success() => {
                eprintln!("  {}: PASS ({})", edit.repo, elapsed_s);
            }
            Ok(s) => {
                eprintln!("  {}: FAIL ({}, exit {})", edit.repo, elapsed_s, s);
                any_failed = true;
            }
            Err(e) => {
                eprintln!("  {}: ERROR ({}, {})", edit.repo, elapsed_s, e);
                any_failed = true;
            }
        }
    }

    if any_failed {
        bail!("one or more `bazel test` runs failed; review logs above");
    }
    eprintln!("All bumped repos passed `bazel test //...`.");
    Ok(())
}

struct Edit {
    repo: String,
    module_bazel: std::path::PathBuf,
    from_version: String,
    was_dev_dep: bool,
    new_content: String,
}

fn scan_workspaces(env: &Env, module: &str, to_version: &str) -> Result<Vec<Edit>> {
    let mut edits = Vec::new();
    for entry in fs::read_dir(&env.workspaces_root)
        .with_context(|| format!("read_dir {}", env.workspaces_root.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let repo = entry.file_name().to_string_lossy().to_string();
        // Skip the registry itself — it's a sibling but doesn't carry
        // bazel_deps the way rules_* do.
        if entry.path() == env.registry_root {
            continue;
        }
        let module_bazel = entry.path().join("MODULE.bazel");
        if !module_bazel.is_file() {
            continue;
        }
        let content = fs::read_to_string(&module_bazel)
            .with_context(|| format!("read {}", module_bazel.display()))?;
        if let Some((new_content, from_version, was_dev_dep)) =
            rewrite_dep(&content, module, to_version)
        {
            edits.push(Edit {
                repo,
                module_bazel,
                from_version,
                was_dev_dep,
                new_content,
            });
        }
    }
    edits.sort_by(|a, b| a.repo.cmp(&b.repo));
    Ok(edits)
}

/// Replace the version pin inside the `bazel_dep(...)` call for
/// `module` with `to_version`. Returns the rewritten content, the
/// old version, and whether the dep was marked `dev_dependency`.
/// Returns None if no matching `bazel_dep` is present.
fn rewrite_dep(content: &str, module: &str, to_version: &str) -> Option<(String, String, bool)> {
    // Pattern: bazel_dep(...) where the `...` contains a name match
    // and a version pin we can rewrite. We match the whole call so
    // multi-line forms still work. Non-greedy `[^)]*?` keeps us
    // inside a single call.
    let name_pattern = format!(
        r#"(?ms)bazel_dep\(\s*([^)]*?\bname\s*=\s*"{}"[^)]*?)\)"#,
        regex::escape(module),
    );
    let call_re = Regex::new(&name_pattern).ok()?;
    let cap = call_re.captures(content)?;
    let inner = cap.get(1)?.as_str();

    // Inside the call, find `version = "X.Y.Z"`.
    let version_re = Regex::new(r#"version\s*=\s*"([^"]+)""#).ok()?;
    let version_cap = version_re.captures(inner)?;
    let from = version_cap.get(1)?.as_str().to_string();
    if from == to_version {
        // Already at the target version. Treat as no-op.
        return None;
    }

    let new_inner = version_re.replace(inner, |_: &regex::Captures| {
        format!(r#"version = "{}""#, to_version)
    });

    let was_dev = inner.contains("dev_dependency = True")
        || inner.contains("dev_dependency=True");

    let full_match_start = cap.get(0)?.start();
    let full_match_end = cap.get(0)?.end();
    let mut out = String::with_capacity(content.len());
    out.push_str(&content[..full_match_start]);
    out.push_str("bazel_dep(");
    out.push_str(&new_inner);
    out.push(')');
    out.push_str(&content[full_match_end..]);
    Some((out, from, was_dev))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_single_line() {
        let input = r#"bazel_dep(name = "rules_jsonschema", version = "0.1.0")"#;
        let (out, from, dev) =
            rewrite_dep(input, "rules_jsonschema", "0.2.0").expect("match");
        assert_eq!(from, "0.1.0");
        assert!(!dev);
        assert!(out.contains(r#"version = "0.2.0""#));
        assert!(!out.contains("0.1.0"));
    }

    #[test]
    fn rewrites_multi_line() {
        let input = "bazel_dep(\n    name = \"rules_jsonschema\",\n    version = \"0.1.0\",\n)";
        let (out, from, _dev) =
            rewrite_dep(input, "rules_jsonschema", "0.2.0").expect("match");
        assert_eq!(from, "0.1.0");
        assert!(out.contains(r#"version = "0.2.0""#));
    }

    #[test]
    fn detects_dev_dependency() {
        let input =
            r#"bazel_dep(name = "rules_jsonschema", version = "0.1.0", dev_dependency = True)"#;
        let (_out, _from, dev) =
            rewrite_dep(input, "rules_jsonschema", "0.2.0").expect("match");
        assert!(dev);
    }

    #[test]
    fn no_op_when_already_at_target() {
        let input = r#"bazel_dep(name = "rules_jsonschema", version = "0.2.0")"#;
        assert!(rewrite_dep(input, "rules_jsonschema", "0.2.0").is_none());
    }

    #[test]
    fn skips_other_modules() {
        let input = r#"bazel_dep(name = "rules_python", version = "1.0.0")"#;
        assert!(rewrite_dep(input, "rules_jsonschema", "0.2.0").is_none());
    }
}
