//! `rels` — CLI for managing the fastverk bazel-registry and its
//! sibling `rules_*` repos.
//!
//! The four subcommands are intentionally narrow in scope:
//!
//!   * `release` — port of the legacy `add_module.py`. Fetches the
//!     GitHub source tarball for a given (repo, version), computes
//!     its SRI integrity, extracts MODULE.bazel, and writes the
//!     registry entry under `modules/<name>/<version>/`.
//!
//!   * `audit`   — read-only consistency pass across the registry
//!     and the local sibling `rules_*` checkouts. Surfaces git-tag
//!     ↔ registry drift, missing CHANGELOG / CI / docs, dep version
//!     drift, and dirty trees.
//!
//!   * `bump`    — rewrites a `bazel_dep` version pin across every
//!     repo whose MODULE.bazel references it. Validates locally
//!     (clean tree, optional `bazel test //...`) and prints a
//!     per-repo summary; commits are left to the operator.
//!
//!   * `matrix`  — emits a Markdown status table of every module:
//!     latest version, dep pins, CHANGELOG/CI/stardoc presence.
//!     Useful for the bazel-registry README and ad-hoc dashboards.
//!
//! Conventions shared across subcommands:
//!
//! - `--registry-root` resolves to the directory containing
//!   `modules/`. Defaults to the registry root inferred from
//!   argv[0]'s ancestors (so running `cargo run -p rels --` from
//!   anywhere inside the registry works).
//! - `--workspaces-root` resolves to the parent directory holding
//!   the sibling `rules_*` checkouts (default: the parent of
//!   `--registry-root`).
//!
//! Each subcommand exits non-zero on failure with a human-readable
//! message on stderr; stdout is reserved for structured output
//! (Markdown, JSON, etc.) so callers can pipe.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

mod audit;
mod bump;
mod common;
mod matrix;
mod mcp;
mod release;
mod scaffold;

#[derive(Parser, Debug)]
#[command(
    name = "rels",
    version,
    about = "Manage the fastverk bazel-registry and its sibling rules_* repos.",
)]
struct Cli {
    /// Path to the bazel-registry checkout (the directory holding
    /// `modules/` and `bazel_registry.json`). Defaults to the
    /// nearest ancestor of argv[0] containing `modules/`.
    #[arg(long, global = true)]
    registry_root: Option<PathBuf>,

    /// Path to the parent directory holding the sibling rules_*
    /// checkouts. Defaults to `<registry_root>/..`.
    #[arg(long, global = true)]
    workspaces_root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Cut a new version: fetch the GitHub tarball, write
    /// modules/<name>/<version>/{source.json,MODULE.bazel} and
    /// upsert metadata.json.
    Release(release::Args),

    /// Cross-repo consistency check. Surfaces tag↔registry drift,
    /// missing CI/CHANGELOG/docs, MODULE.bazel dep drift, dirty
    /// trees. Exits non-zero on any finding.
    Audit(audit::Args),

    /// Rewrite a `bazel_dep` version pin across every repo whose
    /// MODULE.bazel references the named module.
    Bump(bump::Args),

    /// Emit a Markdown status table of every registered module.
    Matrix(matrix::Args),

    /// MCP server over stdio — exposes the registry + sibling
    /// rules_* repos as semantic tools for AI clients.
    #[command(subcommand)]
    Mcp(mcp::McpCommand),

    /// Bootstrap a new fastverk rules_* repo with the standard
    /// MODULE.bazel + CI + CHANGELOG + .gitignore baseline.
    Scaffold(scaffold::Args),
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let env = common::Env::resolve(cli.registry_root.as_deref(), cli.workspaces_root.as_deref())?;
    match cli.command {
        Command::Release(args) => release::run(&env, args),
        Command::Audit(args) => audit::run(&env, args),
        Command::Bump(args) => bump::run(&env, args),
        Command::Matrix(args) => matrix::run(&env, args),
        Command::Mcp(cmd) => mcp::run(&env, cmd),
        Command::Scaffold(args) => scaffold::run(&env, args),
    }
}
