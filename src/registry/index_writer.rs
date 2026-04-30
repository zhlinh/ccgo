//! Shared helpers for "write to a package-index Git repository".
//!
//! Both `ccgo publish index` (append a new VersionEntry) and `ccgo yank`
//! (flip yanked: true on an existing entry) clone the same kind of
//! Git-based index repo, edit one JSON file, commit, and optionally push.
//! This module owns the clone-edit-commit-push plumbing so the two
//! command paths can share it without duplicating ~150 lines of git
//! subprocess wrangling.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use super::index::{IndexMetadata, PackageIndex};

/// Clone (or pull) a registry index repo into a working directory under
/// `~/.ccgo/registry/publish/<name>/`. Returns the local path.
///
/// First-time clone is `--depth 1`. If an existing clone is present, runs
/// `git pull --rebase`. Pull failures fall back to a fresh clone. If the
/// remote doesn't exist yet (typical on the very first publish to a new
/// index repo), initializes a local git repo with the remote configured
/// and seeds an empty `index.json`.
pub fn prepare_index_repo(repo_url: &str, name: &str, verbose: bool) -> Result<PathBuf> {
    let ccgo_home = PackageIndex::new().ccgo_home_path();
    let work_dir = ccgo_home.join("registry").join("publish").join(name);

    if work_dir.exists() {
        println!("📥 Updating existing index clone...");
        let mut cmd = Command::new("git");
        cmd.current_dir(&work_dir);
        cmd.args(["pull", "--rebase"]);
        if !verbose {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let status = cmd.status().context("Failed to pull index repository")?;
        if !status.success() {
            // Pull failed (probably divergent / corrupted local clone) —
            // wipe and re-clone. Recursion would be infinite if the
            // wipe-then-clone path itself fails, so just inline the
            // re-clone steps below.
            println!("⚠️  Pull failed, re-cloning...");
            std::fs::remove_dir_all(&work_dir)?;
            return clone_or_init(repo_url, name, &work_dir, verbose);
        }
        Ok(work_dir)
    } else {
        clone_or_init(repo_url, name, &work_dir, verbose)
    }
}

fn clone_or_init(repo_url: &str, name: &str, work_dir: &Path, verbose: bool) -> Result<PathBuf> {
    println!("📥 Cloning index repository...");
    if let Some(parent) = work_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut cmd = Command::new("git");
    cmd.args(["clone", "--depth", "1", repo_url, work_dir.to_str().unwrap()]);
    if !verbose {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let status = cmd
        .status()
        .context("Failed to clone index repository")?;

    if !status.success() {
        // Clone failed — typical reason is "remote repo doesn't exist yet"
        // (first publish bootstraps the index). Initialize a fresh local
        // repo with the remote configured and seed an empty index.json.
        println!("📝 Initializing new index repository...");
        std::fs::create_dir_all(work_dir)?;

        Command::new("git")
            .current_dir(work_dir)
            .args(["init"])
            .status()
            .context("Failed to init git repository")?;
        Command::new("git")
            .current_dir(work_dir)
            .args(["remote", "add", "origin", repo_url])
            .status()
            .context("Failed to add git remote")?;

        let metadata = IndexMetadata {
            version: 1,
            name: name.to_string(),
            description: format!("{} package index", name),
            homepage: None,
            package_count: 0,
            updated_at: chrono::Utc::now().to_rfc3339(),
        };
        let json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(work_dir.join("index.json"), json)?;
    }

    Ok(work_dir.to_path_buf())
}

/// Refresh `<index>/index.json` — recount packages, bump `updated_at`.
/// Idempotent. Caller invokes after editing per-package JSON files.
pub fn update_index_metadata(index_path: &Path, name: &str) -> Result<()> {
    let metadata_path = index_path.join("index.json");

    let mut metadata: IndexMetadata = if metadata_path.exists() {
        let content = std::fs::read_to_string(&metadata_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| IndexMetadata {
            version: 1,
            name: name.to_string(),
            description: format!("{} package index", name),
            homepage: None,
            package_count: 0,
            updated_at: String::new(),
        })
    } else {
        IndexMetadata {
            version: 1,
            name: name.to_string(),
            description: format!("{} package index", name),
            homepage: None,
            package_count: 0,
            updated_at: String::new(),
        }
    };

    let mut count = 0;
    for entry in walkdir::WalkDir::new(index_path)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().extension().and_then(|s| s.to_str()) == Some("json")
            && entry.file_name() != "index.json"
        {
            count += 1;
        }
    }
    metadata.package_count = count;
    metadata.updated_at = chrono::Utc::now().to_rfc3339();

    let json = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(metadata_path, json)?;
    println!("📊 Index metadata updated: {} package(s)", count);
    Ok(())
}

/// Stage all changes, commit with `message`. Skips the commit when
/// `git status --porcelain` is empty (no-op publish/yank).
pub fn commit_changes(index_path: &Path, message: &str, verbose: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(index_path).args(["add", "-A"]);
    if !verbose {
        cmd.stdout(Stdio::null());
    }
    cmd.status().context("Failed to stage changes")?;

    let output = Command::new("git")
        .current_dir(index_path)
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;
    if output.stdout.is_empty() {
        println!("ℹ️  No changes to commit");
        return Ok(());
    }

    let mut cmd = Command::new("git");
    cmd.current_dir(index_path).args(["commit", "-m", message]);
    if !verbose {
        cmd.stdout(Stdio::null());
    }
    let status = cmd.status().context("Failed to commit changes")?;
    if status.success() {
        println!("✅ Committed: {}", message);
    }
    Ok(())
}

/// Push `HEAD` to `origin`. Errors if the push fails.
pub fn push_changes(index_path: &Path, verbose: bool) -> Result<()> {
    let mut cmd = Command::new("git");
    cmd.current_dir(index_path).args(["push", "origin", "HEAD"]);
    if !verbose {
        cmd.stderr(Stdio::null());
    }
    let status = cmd.status().context("Failed to push changes")?;
    if !status.success() {
        anyhow::bail!("Failed to push to remote");
    }
    Ok(())
}
