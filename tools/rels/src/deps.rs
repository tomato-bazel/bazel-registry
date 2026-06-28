//! `rels deps` — the dependency ratchet.
//!
//! Audits every sibling repo's `MODULE.bazel` against the canonical BOM
//! (`rules_fastverk//bom/versions.json`) and, with `--write`, bumps each
//! drifting `bazel_dep` to the BOM version. This is how ~60 polyrepo modules
//! stay in sync without a monorepo: one source of truth, floor-first.
//!
//! - `rels deps`          — audit; lists drift, exits non-zero if any.
//! - `rels deps --write`  — bump every drifting pin to the BOM, then
//!                          `bazel test //...` per touched repo (--no-test skips).
//!
//! Reuses `bump::rewrite_dep` (it returns the old version, so it doubles as the
//! drift detector + the fix). Like `bump`, it does NOT commit — the operator
//! reviews per-repo diffs and lands them through the normal PR flow.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use anyhow::{bail, Context, Result};
use clap::Args as ClapArgs;
use serde::Deserialize;

use crate::bump::rewrite_dep;
use crate::common::Env;

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Apply the bumps (default: audit only, exit non-zero on drift).
    #[arg(long)]
    pub write: bool,

    /// Skip `bazel test //...` after writing.
    #[arg(long)]
    pub no_test: bool,

    /// Override the BOM path (default: <rules_fastverk checkout>/bom/versions.json).
    #[arg(long)]
    pub bom: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Bom {
    modules: BTreeMap<String, String>,
}

struct Drift {
    repo: String,
    module: String,
    from: String,
    to: String,
    /// true = below the BOM (the ratchet bumps it up); false = above the BOM
    /// (advisory — a candidate to raise the BOM; never downgraded).
    behind: bool,
}

/// Compare two version strings numerically component-wise (3 < 10), with
/// non-numeric parts ordered after numeric ones. Good enough for the
/// `MAJOR.MINOR.PATCH[-pre]` pins in MODULE.bazel.
fn version_cmp(a: &str, b: &str) -> Ordering {
    fn key(v: &str) -> Vec<(u8, u64, String)> {
        v.replace('-', ".")
            .split('.')
            .map(|p| match p.parse::<u64>() {
                Ok(n) => (0, n, String::new()),
                Err(_) => (1, 0, p.to_string()),
            })
            .collect()
    }
    key(a).cmp(&key(b))
}

pub fn run(env: &Env, args: Args) -> Result<()> {
    let bom = load_bom(env, args.bom.as_deref())?;
    eprintln!("rels deps: BOM has {} canonical version(s)", bom.len());

    let mut drifts: Vec<Drift> = Vec::new();
    // (MODULE.bazel path, repo name, rewritten content) for repos that drifted.
    let mut writes: Vec<(PathBuf, String, String)> = Vec::new();
    let mut seen = HashSet::new();

    for root in env.workspace_search_roots() {
        if !root.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&root).with_context(|| format!("read_dir {}", root.display()))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path();
            if path == env.registry_root {
                continue;
            }
            let module_bazel = path.join("MODULE.bazel");
            if !module_bazel.is_file() {
                continue;
            }
            // Dedup repos reachable via multiple search roots.
            let key = fs::canonicalize(&module_bazel).unwrap_or_else(|_| module_bazel.clone());
            if !seen.insert(key) {
                continue;
            }

            let repo = entry.file_name().to_string_lossy().to_string();
            let content = fs::read_to_string(&module_bazel)
                .with_context(|| format!("read {}", module_bazel.display()))?;

            let mut new_content = content;
            let mut drifted = false;
            for (module, canonical) in &bom {
                if let Some((rewritten, from, _dev)) = rewrite_dep(&new_content, module, canonical) {
                    match version_cmp(&from, canonical) {
                        // Behind the BOM — the ratchet bumps it up.
                        Ordering::Less => {
                            drifts.push(Drift {
                                repo: repo.clone(),
                                module: module.clone(),
                                from,
                                to: canonical.clone(),
                                behind: true,
                            });
                            new_content = rewritten;
                            drifted = true;
                        }
                        // Ahead of the BOM — advisory only; never downgrade.
                        Ordering::Greater => drifts.push(Drift {
                            repo: repo.clone(),
                            module: module.clone(),
                            from,
                            to: canonical.clone(),
                            behind: false,
                        }),
                        Ordering::Equal => {}
                    }
                }
            }
            if drifted {
                writes.push((module_bazel, repo, new_content));
            }
        }
    }

    drifts.sort_by(|a, b| (&a.repo, &a.module).cmp(&(&b.repo, &b.module)));
    let behind: Vec<&Drift> = drifts.iter().filter(|d| d.behind).collect();
    let ahead: Vec<&Drift> = drifts.iter().filter(|d| !d.behind).collect();

    // Ahead = repos newer than the BOM. Advisory: raise the BOM (never downgrade).
    if !ahead.is_empty() {
        eprintln!("Ahead of the BOM ({} pin(s)) — consider raising the BOM:", ahead.len());
        for d in &ahead {
            eprintln!("  {:<24} {:<24} {} (BOM {})", d.repo, d.module, d.from, d.to);
        }
    }

    if behind.is_empty() {
        eprintln!("rels deps: no repo is behind the BOM. ✓");
        return Ok(());
    }

    eprintln!(
        "Behind the BOM — {} pin(s) across {} repo(s) (the ratchet bumps these up):",
        behind.len(),
        writes.len(),
    );
    for d in &behind {
        eprintln!("  {:<24} {:<24} {} → {}", d.repo, d.module, d.from, d.to);
    }

    if !args.write {
        bail!("{} dep(s) behind the BOM; re-run `rels deps --write` to bump", behind.len());
    }

    for (path, _repo, new_content) in &writes {
        fs::write(path, new_content).with_context(|| format!("write {}", path.display()))?;
    }
    eprintln!("Bumped {} repo(s) to the BOM.", writes.len());

    if args.no_test {
        eprintln!("--no-test: skipping `bazel test //...` per repo.");
        return Ok(());
    }

    let mut any_failed = false;
    eprintln!("Running `bazel test //...` per bumped repo:");
    for (_path, repo, _content) in &writes {
        let started = Instant::now();
        let status = Command::new("bazel")
            .args(["test", "//...", "--test_output=errors"])
            .current_dir(env.checkout_path(repo))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status();
        let elapsed = format!("{:.0}s", started.elapsed().as_secs_f64());
        match status {
            Ok(s) if s.success() => eprintln!("  {}: PASS ({})", repo, elapsed),
            Ok(s) => {
                eprintln!("  {}: FAIL ({}, exit {})", repo, elapsed, s);
                any_failed = true;
            }
            Err(e) => {
                eprintln!("  {}: ERROR ({}, {})", repo, elapsed, e);
                any_failed = true;
            }
        }
    }
    if any_failed {
        bail!("one or more `bazel test` runs failed after the bump; review logs above");
    }
    eprintln!("All bumped repos passed `bazel test //...`.");
    Ok(())
}

fn load_bom(env: &Env, override_path: Option<&Path>) -> Result<BTreeMap<String, String>> {
    let path = match override_path {
        Some(p) => p.to_path_buf(),
        None => env
            .checkout_path("rules_fastverk")
            .join("bom")
            .join("versions.json"),
    };
    let raw = fs::read_to_string(&path).with_context(|| {
        format!(
            "read BOM {} (is rules_fastverk checked out? else pass --bom)",
            path.display()
        )
    })?;
    let bom: Bom = serde_json::from_str(&raw).with_context(|| format!("parse BOM {}", path.display()))?;
    Ok(bom.modules)
}

#[cfg(test)]
mod tests {
    use super::version_cmp;
    use std::cmp::Ordering;

    #[test]
    fn numeric_components_compare_as_numbers() {
        assert_eq!(version_cmp("0.0.10", "1.0.0"), Ordering::Less);
        assert_eq!(version_cmp("1.8.2", "1.7.1"), Ordering::Greater);
        assert_eq!(version_cmp("2.3.0", "2.2.6"), Ordering::Greater);
        assert_eq!(version_cmp("0.40.0", "1.7.0"), Ordering::Less);
        // 9 > 3 numerically, not lexically.
        assert_eq!(version_cmp("0.9.0", "0.10.0"), Ordering::Less);
        assert_eq!(version_cmp("0.7.0", "0.7.0"), Ordering::Equal);
    }

    #[test]
    fn prerelease_orders_after_numeric() {
        // "6.6" vs "6.8"
        assert_eq!(version_cmp("6.6", "6.8"), Ordering::Less);
        // a pre-release tag sorts after a bare number at that position
        assert_eq!(version_cmp("1.0.0", "1.0.0-rc1"), Ordering::Less);
    }
}
