//! `rels audit` — read-only consistency pass over the registry and
//! its sibling rules_* checkouts.
//!
//! Surfaces, with a non-zero exit if any are found:
//!
//!   * **Tag↔registry drift**: a tag pushed to the remote that the
//!     registry doesn't have an entry for, or a registry entry with
//!     no matching tag.
//!   * **Missing infrastructure**: CHANGELOG.md, .github/workflows/,
//!     docs/ stardoc setup, .gitignore for `.claude/` /
//!     `MODULE.bazel.lock`.
//!   * **MODULE.bazel version drift**: the on-disk
//!     `MODULE.bazel#version` for a checkout doesn't match the
//!     latest tag pushed to its remote.
//!   * **Dirty trees**: the local checkout has uncommitted
//!     changes.
//!
//! Each finding prints `<category> <module> :: <detail>` to stdout
//! so callers can grep, pipe to issue trackers, or post to Slack.

use std::fs;
use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

use crate::common::{git_capture, remote_tags, version_cmp, Env, RegistryModule};

#[derive(ClapArgs, Debug)]
pub struct Args {
    /// Skip the network-touching git ls-remote calls. Useful for
    /// CI where only the on-disk consistency matters.
    #[arg(long)]
    no_remote: bool,

    /// Print findings as a Markdown bulleted list (default: plain
    /// `category module :: detail` lines, one per finding).
    #[arg(long)]
    markdown: bool,
}

pub fn run(env: &Env, args: Args) -> Result<()> {
    let mut findings: Vec<Finding> = Vec::new();
    let modules = RegistryModule::load_all(env)?;

    for module in &modules {
        let checkout = env.checkout_path(&module.name);
        if !checkout.is_dir() {
            findings.push(Finding {
                category: "checkout_missing",
                module: module.name.clone(),
                detail: format!("expected at {}", checkout.display()),
            });
            continue;
        }

        // --- Tag ↔ registry drift (remote-touching) --------------------------
        if !args.no_remote {
            match remote_tags(&checkout) {
                Ok(tags) => {
                    let tag_versions: Vec<String> = tags
                        .iter()
                        .filter(|t| t.starts_with('v'))
                        .map(|t| t[1..].to_string())
                        .collect();

                    for v in &tag_versions {
                        if !module.metadata.versions.contains(v) {
                            findings.push(Finding {
                                category: "tag_not_registered",
                                module: module.name.clone(),
                                detail: format!("tag v{} exists on remote but missing in registry", v),
                            });
                        }
                    }
                    for v in &module.metadata.versions {
                        if !tag_versions.contains(v) {
                            findings.push(Finding {
                                category: "registry_orphan",
                                module: module.name.clone(),
                                detail: format!("registry has {} but no v{} tag on remote", v, v),
                            });
                        }
                    }

                    if let Some(latest_tag) = tag_versions
                        .iter()
                        .max_by(|a, b| version_cmp(a, b))
                    {
                        match read_module_version(&checkout) {
                            Ok(disk_version) => {
                                if &disk_version != latest_tag {
                                    findings.push(Finding {
                                        category: "module_version_drift",
                                        module: module.name.clone(),
                                        detail: format!(
                                            "MODULE.bazel says {}, latest remote tag is v{}",
                                            disk_version, latest_tag,
                                        ),
                                    });
                                }
                            }
                            Err(e) => findings.push(Finding {
                                category: "module_bazel_unreadable",
                                module: module.name.clone(),
                                detail: e.to_string(),
                            }),
                        }
                    }
                }
                Err(e) => findings.push(Finding {
                    category: "remote_tags_fetch_failed",
                    module: module.name.clone(),
                    detail: e.to_string(),
                }),
            }
        }

        // --- Dirty tree --------------------------------------------------------
        match git_capture(&checkout, &["status", "--porcelain"]) {
            Ok(out) if !out.is_empty() => findings.push(Finding {
                category: "dirty_tree",
                module: module.name.clone(),
                detail: format!("uncommitted changes:\n{}", indent(&out, 4)),
            }),
            Ok(_) => {}
            Err(e) => findings.push(Finding {
                category: "git_status_failed",
                module: module.name.clone(),
                detail: e.to_string(),
            }),
        }

        // --- Infrastructure presence ------------------------------------------
        for (path, kind) in &[
            ("CHANGELOG.md", "missing_changelog"),
            (".github/workflows/ci.yml", "missing_ci_workflow"),
            ("docs/BUILD.bazel", "missing_stardoc_setup"),
        ] {
            if !checkout.join(path).exists() {
                findings.push(Finding {
                    category: kind,
                    module: module.name.clone(),
                    detail: format!("expected {}", path),
                });
            }
        }

        if let Ok(gi) = fs::read_to_string(checkout.join(".gitignore")) {
            for want in &[".claude/", "MODULE.bazel.lock"] {
                if !gi.lines().any(|l| l.trim() == *want) {
                    findings.push(Finding {
                        category: "gitignore_missing_entry",
                        module: module.name.clone(),
                        detail: format!("{} not in .gitignore", want),
                    });
                }
            }
        }
    }

    findings.sort_by(|a, b| (a.module.as_str(), a.category).cmp(&(b.module.as_str(), b.category)));

    if findings.is_empty() {
        eprintln!("audit: no findings.");
        return Ok(());
    }

    if args.markdown {
        println!("# Audit findings ({})", findings.len());
        for f in &findings {
            println!("- **{}** · `{}` — {}", f.module, f.category, f.detail);
        }
    } else {
        for f in &findings {
            println!("{} {} :: {}", f.category, f.module, f.detail);
        }
    }
    std::process::exit(1);
}

struct Finding {
    category: &'static str,
    module: String,
    detail: String,
}

fn read_module_version(checkout: &Path) -> Result<String> {
    let module_bazel = checkout.join("MODULE.bazel");
    let text = fs::read_to_string(&module_bazel)?;
    for line in text.lines() {
        let trimmed = line.trim();
        // Match `version = "0.1.0",` or with single quotes / spaces.
        if let Some(rest) = trimmed.strip_prefix("version = ") {
            let value = rest.trim_end_matches(',').trim().trim_matches('"').trim_matches('\'');
            return Ok(value.to_string());
        }
    }
    anyhow::bail!("no `version = ...` line in {}", module_bazel.display())
}

fn indent(s: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    s.lines().map(|l| format!("{pad}{l}")).collect::<Vec<_>>().join("\n")
}
