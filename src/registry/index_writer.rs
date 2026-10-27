//! Shared helpers for "write to a package-index Git repository".
//!
//! Both `ccgo publish package` (append a new VersionEntry) and `ccgo yank`
//! (flip yanked: true on an existing entry) clone the same kind of
//! Git-based index repo, edit one JSON file, commit, and optionally push.
//! This module owns the clone-edit-commit-push plumbing so the two
//! command paths can share it without duplicating ~150 lines of git
//! subprocess wrangling.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use super::index::{IndexMetadata, PackageIndex};

#[path = "index_transaction.rs"]
mod transaction;
pub use transaction::{lock_index, matches_pending_merge, reconcile_remote, IndexWriteLock};

pub fn publish_work_dir(name: &str) -> PathBuf {
    PackageIndex::new()
        .ccgo_home_path()
        .join("registry/publish")
        .join(name)
}

pub fn require_clean(index_path: &Path) -> Result<()> {
    let output = Command::new("git")
        .current_dir(index_path)
        .args(["status", "--porcelain"])
        .output()?;
    if !output.status.success() || !output.stdout.is_empty() {
        anyhow::bail!("Index has uncommitted changes; local changes are preserved. Resolve them before publishing");
    }
    Ok(())
}

pub fn verify_origin(path: &Path, repository: &str) -> Result<()> {
    let origin = Command::new("git")
        .current_dir(path)
        .args(["remote", "get-url", "origin"])
        .output()?;
    if !origin.status.success() || String::from_utf8_lossy(&origin.stdout).trim() != repository {
        anyhow::bail!(
            "Index clone {} has a different origin; use a separate registry name",
            path.display()
        );
    }
    Ok(())
}

/// Clone (or pull) a registry index repo into a working directory under
/// `~/.ccgo/registry/publish/<name>/`. Returns the local path.
///
/// First-time clone is `--depth 1`, including an existing empty remote.
/// Existing clones reconcile remote changes by version record. Failures
/// preserve local commits; unavailable remotes never become new local indexes.
pub fn prepare_index_repo(repo_url: &str, name: &str, verbose: bool) -> Result<PathBuf> {
    let work_dir = publish_work_dir(name);

    if work_dir.exists() {
        verify_origin(&work_dir, repo_url)?;
        println!("📥 Updating existing index clone...");
        reconcile_remote(&work_dir, name, verbose).with_context(|| format!(
            "Failed to update index clone {}; resolve Git/network issues there and retry. Local changes are preserved", work_dir.display()))?;
        Ok(work_dir)
    } else {
        clone_index(repo_url, &work_dir, verbose)
    }
}

fn clone_index(repo_url: &str, work_dir: &Path, verbose: bool) -> Result<PathBuf> {
    println!("📥 Cloning index repository...");
    if let Some(parent) = work_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut cmd = Command::new("git");
    cmd.args([
        "clone",
        "--depth",
        "1",
        repo_url,
        work_dir.to_str().unwrap(),
    ]);
    if !verbose {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }
    let status = cmd.status().context("Failed to clone index repository")?;

    if !status.success() {
        anyhow::bail!("Failed to clone index repository {repo_url}; create the remote or resolve access before retrying");
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
        .filter_entry(|entry| entry.depth() == 0 || entry.file_name() != ".git")
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
    if !cmd.status().context("Failed to stage changes")?.success() {
        anyhow::bail!("Failed to stage index changes");
    }

    let output = Command::new("git")
        .current_dir(index_path)
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;
    if !output.status.success() {
        anyhow::bail!("Failed to check index Git status");
    }
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
    if !status.success() {
        anyhow::bail!(
            "Failed to commit index changes; check Git identity and hooks before retrying"
        );
    }
    println!("✅ Committed: {}", message);
    Ok(())
}

/// Push `HEAD` to `origin`. Errors if the push fails.
pub fn push_changes(index_path: &Path, verbose: bool) -> Result<()> {
    push_changes_retry(index_path, "index", verbose)
}

pub fn push_changes_retry(index_path: &Path, name: &str, verbose: bool) -> Result<()> {
    for attempt in 0..3 {
        if try_push(index_path, verbose)? {
            return Ok(());
        }
        if attempt == 2 || !reconcile_remote(index_path, name, verbose)? {
            anyhow::bail!("Failed to push to remote; local commits are preserved in {}. Retry the same command after resolving remote access", index_path.display());
        }
    }
    unreachable!()
}

fn try_push(index_path: &Path, verbose: bool) -> Result<bool> {
    let mut cmd = Command::new("git");
    cmd.current_dir(index_path).args(["push", "origin", "HEAD"]);
    if !verbose {
        cmd.stderr(Stdio::null());
    }
    let status = cmd.status().context("Failed to push changes")?;
    Ok(status.success())
}
