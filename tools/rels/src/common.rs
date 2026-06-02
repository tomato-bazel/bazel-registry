//! Shared types + helpers used by every subcommand.
//!
//! The `Env` struct anchors every operation to a registry root and
//! a workspaces root, so subcommands don't have to re-resolve paths
//! each time. `RegistryModule` and `RepoCheckout` are the
//! filesystem views the audit / matrix subcommands walk.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Project root configuration shared by every subcommand.
#[derive(Debug, Clone)]
pub struct Env {
    /// Absolute path of the bazel-registry root (the directory
    /// containing `modules/` + `bazel_registry.json`).
    pub registry_root: PathBuf,
    /// Absolute path of the parent directory that holds the
    /// sibling rules_* checkouts (typically `<registry_root>/..`).
    pub workspaces_root: PathBuf,
}

impl Env {
    pub fn resolve(
        registry_root: Option<&Path>,
        workspaces_root: Option<&Path>,
    ) -> Result<Self> {
        let registry_root = match registry_root {
            Some(p) => fs::canonicalize(p)
                .with_context(|| format!("--registry-root: {}", p.display()))?,
            None => discover_registry_root()?,
        };
        let workspaces_root = match workspaces_root {
            Some(p) => fs::canonicalize(p)
                .with_context(|| format!("--workspaces-root: {}", p.display()))?,
            None => registry_root
                .parent()
                .ok_or_else(|| anyhow!("registry root has no parent: {}", registry_root.display()))?
                .to_path_buf(),
        };
        Ok(Self {
            registry_root,
            workspaces_root,
        })
    }

    pub fn modules_dir(&self) -> PathBuf {
        self.registry_root.join("modules")
    }

    /// Path to the sibling checkout for a given module name. Falls
    /// back to `<workspaces_root>/<module_name>` even if it doesn't
    /// exist on disk — the caller decides what to do on absence.
    pub fn checkout_path(&self, module_name: &str) -> PathBuf {
        self.workspaces_root.join(module_name)
    }
}

/// Walk argv[0]'s ancestors looking for the registry root marker
/// (`bazel_registry.json` + `modules/`). This lets `rels` run from
/// anywhere inside the registry checkout.
///
/// Under `bazel run`, the process cwd is the runfiles tree, not the
/// user's invocation directory — Bazel exposes the latter as
/// `BUILD_WORKING_DIRECTORY`. Prefer that env var when it's set so
/// `bazel run //tools/rels:rels -- audit` works the same as
/// invoking the binary directly.
fn discover_registry_root() -> Result<PathBuf> {
    let mut search_starts: Vec<PathBuf> = Vec::new();
    if let Ok(bwd) = std::env::var("BUILD_WORKING_DIRECTORY") {
        search_starts.push(PathBuf::from(bwd));
    }
    search_starts.push(std::env::current_dir()?);
    for start in &search_starts {
        for ancestor in start.ancestors() {
            if ancestor.join("bazel_registry.json").is_file()
                && ancestor.join("modules").is_dir()
            {
                return Ok(ancestor.to_path_buf());
            }
        }
    }
    bail!(
        "could not locate the bazel-registry root from {}. \
         Pass --registry-root, or run from inside the registry checkout.",
        search_starts[0].display()
    );
}

/// `modules/<name>/metadata.json` shape. We use `preserve_order` on
/// serde_json so the array ordering we write back matches the on-
/// disk format (helpful for diff readability).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub homepage: String,
    pub maintainers: Vec<Maintainer>,
    pub repository: Vec<String>,
    pub versions: Vec<String>,
    #[serde(default)]
    pub yanked_versions: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maintainer {
    pub name: String,
    pub github: String,
}

impl ModuleMetadata {
    pub fn read(path: &Path) -> Result<Self> {
        let text = fs::read_to_string(path)
            .with_context(|| format!("read {}", path.display()))?;
        let me: Self = serde_json::from_str(&text)
            .with_context(|| format!("parse {}", path.display()))?;
        Ok(me)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        fs::write(path, text)
            .with_context(|| format!("write {}", path.display()))
    }

    /// Insert + sort a version. Idempotent.
    pub fn upsert_version(&mut self, version: &str) {
        if !self.versions.iter().any(|v| v == version) {
            self.versions.push(version.to_string());
        }
        self.versions.sort_by(|a, b| version_cmp(a, b));
    }
}

/// `modules/<name>/<version>/source.json` shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceJson {
    pub integrity: String,
    pub strip_prefix: String,
    pub url: String,
}

impl SourceJson {
    pub fn write(&self, path: &Path) -> Result<()> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        fs::write(path, text)
            .with_context(|| format!("write {}", path.display()))
    }
}

/// Semver-ish comparison that handles pre-release suffixes.
/// `0.3.0-rc1` < `0.3.0` < `0.3.1`.
pub fn version_cmp(a: &str, b: &str) -> Ordering {
    let (a_base, a_pre) = split_pre(a);
    let (b_base, b_pre) = split_pre(b);
    let a_parts = parse_parts(a_base);
    let b_parts = parse_parts(b_base);

    let max_len = a_parts.len().max(b_parts.len());
    for i in 0..max_len {
        let ai = a_parts.get(i).copied().unwrap_or(0);
        let bi = b_parts.get(i).copied().unwrap_or(0);
        match ai.cmp(&bi) {
            Ordering::Equal => continue,
            other => return other,
        }
    }
    // Same base: pre-release sorts before non-pre.
    match (a_pre, b_pre) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(a), Some(b)) => a.cmp(b),
    }
}

fn split_pre(v: &str) -> (&str, Option<&str>) {
    match v.split_once('-') {
        Some((base, pre)) => (base, Some(pre)),
        None => (v, None),
    }
}

fn parse_parts(s: &str) -> Vec<u64> {
    s.split('.')
        .filter_map(|p| p.parse::<u64>().ok())
        .collect()
}

/// One registered module on disk. Loaded by `audit` and `matrix`.
#[derive(Debug, Clone)]
pub struct RegistryModule {
    pub name: String,
    pub metadata: ModuleMetadata,
}

impl RegistryModule {
    pub fn load_all(env: &Env) -> Result<Vec<Self>> {
        let mut out = Vec::new();
        let modules_dir = env.modules_dir();
        for entry in fs::read_dir(&modules_dir)
            .with_context(|| format!("read_dir {}", modules_dir.display()))?
        {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            let meta_path = entry.path().join("metadata.json");
            if !meta_path.is_file() {
                continue;
            }
            out.push(Self {
                name,
                metadata: ModuleMetadata::read(&meta_path)?,
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }
}

/// Run a git command in a checkout; return its trimmed stdout.
pub fn git_capture(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("git {} in {}", args.join(" "), dir.display()))?;
    if !out.status.success() {
        bail!(
            "git {} in {} failed:\n{}",
            args.join(" "),
            dir.display(),
            String::from_utf8_lossy(&out.stderr),
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Tags published at a local checkout's git remote (typically
/// origin). Returns just the tag names (`v0.1.0`, …) without
/// `refs/tags/` prefixes.
pub fn remote_tags(checkout: &Path) -> Result<Vec<String>> {
    let raw = git_capture(checkout, &["ls-remote", "--tags", "origin"])?;
    let mut tags = Vec::new();
    for line in raw.lines() {
        // Each line: "<sha>\trefs/tags/<name>" (optionally with ^{}).
        if let Some((_sha, refname)) = line.split_once('\t') {
            let name = refname
                .trim_start_matches("refs/tags/")
                .trim_end_matches("^{}");
            if !tags.contains(&name.to_string()) {
                tags.push(name.to_string());
            }
        }
    }
    Ok(tags)
}
