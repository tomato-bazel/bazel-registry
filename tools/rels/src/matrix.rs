//! `rels matrix` — emit a Markdown status table of every module.
//!
//! Columns:
//!   * Module
//!   * Latest registered version
//!   * Tag in sync with MODULE.bazel?
//!   * CHANGELOG present?
//!   * CI workflow present?
//!   * Stardoc setup present?
//!
//! Output goes to stdout so callers can `>> README.md` or pipe into
//! a status-page generator.

use std::path::Path;

use anyhow::Result;
use clap::Args as ClapArgs;

use crate::common::{version_cmp, Env, RegistryModule};

#[derive(ClapArgs, Debug)]
pub struct Args {}

pub fn run(env: &Env, _args: Args) -> Result<()> {
    let modules = RegistryModule::load_all(env)?;

    println!("| Module | Latest | CHANGELOG | CI | Stardoc |");
    println!("|---|---|---|---|---|");
    for module in &modules {
        let latest = module
            .metadata
            .versions
            .iter()
            .max_by(|a, b| version_cmp(a, b))
            .map(String::as_str)
            .unwrap_or("—");
        let checkout = env.checkout_path(&module.name);
        let changelog = badge(checkout.join("CHANGELOG.md").is_file());
        let ci = badge(checkout.join(".github/workflows/ci.yml").is_file());
        let stardoc = badge(checkout.join("docs/BUILD.bazel").is_file());
        println!(
            "| [`{name}`]({homepage}) | `{latest}` | {changelog} | {ci} | {stardoc} |",
            name = module.name,
            homepage = module.metadata.homepage,
            latest = latest,
            changelog = changelog,
            ci = ci,
            stardoc = stardoc,
        );
    }
    Ok(())
}

fn badge(present: bool) -> &'static str {
    if present {
        "✅"
    } else {
        "—"
    }
}

#[allow(dead_code)]
fn unused(_p: &Path) {}
