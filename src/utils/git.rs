//! Thin wrappers around the `git` CLI.
//!
//! We shell out via `std::process::Command` rather than linking libgit2 —
//! matches the existing pattern in `src/commands/tag.rs` and keeps the
//! dependency footprint small. Functions that can reasonably no-op (e.g.
//! looking up commits that may not exist) return `Option`; failures that
//! indicate git itself is broken or unreachable surface as `anyhow::Error`.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

/// Run `git` with the given args in `cwd`, capturing stdout.
///
/// Errors when the process fails to spawn; returns `Ok(None)` when git exits
/// non-zero (e.g. "no matching commit"), so callers can distinguish "question
/// was answered: no" from "git is broken".
fn run_capture(cwd: &Path, args: &[&str]) -> Result<Option<String>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to run: git {}", args.join(" ")))?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

/// Run `git` with the given args in `cwd`, streaming stdout/stderr to the
/// terminal. Fails when the exit code is non-zero.
fn run_status(cwd: &Path, args: &[&str], context: &str) -> Result<()> {
    let status = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .with_context(|| format!("failed to run: git {}", args.join(" ")))?;
    if !status.success() {
        bail!("{context} (git {} exited with {status})", args.join(" "));
    }
    Ok(())
}

/// Returns true if the working tree has no unstaged or staged changes.
///
/// Uses `git status --porcelain` — empty output means clean.
pub fn is_working_tree_clean(cwd: &Path) -> Result<bool> {
    let out = run_capture(cwd, &["status", "--porcelain"])?
        .ok_or_else(|| anyhow::anyhow!("`git status` failed — is this a git repository?"))?;
    Ok(out.is_empty())
}

/// Return the current branch name (e.g. `master`, `main`, `feature/x`).
pub fn current_branch(cwd: &Path) -> Result<String> {
    run_capture(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .ok_or_else(|| anyhow::anyhow!("failed to determine current branch"))
}

/// SHA of the most recent `chore: release v<…>` commit, or `None` if this
/// repo has never had a release commit.
pub fn last_release_sha(cwd: &Path) -> Result<Option<String>> {
    let out = run_capture(
        cwd,
        &["log", "--pretty=format:%H", "--grep=^chore: release v[0-9]", "-n", "1"],
    )?;
    Ok(out.filter(|s| !s.is_empty()))
}

/// SHA of the very first commit reachable from HEAD (repo root commit).
pub fn first_commit_sha(cwd: &Path) -> Result<String> {
    let out = run_capture(cwd, &["rev-list", "--max-parents=0", "HEAD"])?
        .ok_or_else(|| anyhow::anyhow!("failed to resolve first commit"))?;
    // Usually one line, but take the first if history has multiple roots.
    out.lines()
        .next()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("repository has no commits"))
}

/// List commit subjects (first line of each commit message) in `range`,
/// excluding merges. Returns one subject per element.
pub fn log_subjects(cwd: &Path, range: &str) -> Result<Vec<String>> {
    let out = run_capture(cwd, &["log", "--no-merges", "--pretty=format:%s", range])?
        .unwrap_or_default();
    Ok(out.lines().map(|s| s.to_string()).collect())
}

/// Stage the given paths. Paths may be relative or absolute; git resolves
/// them against `cwd`.
pub fn add_paths(cwd: &Path, paths: &[&Path]) -> Result<()> {
    let mut args: Vec<&str> = vec!["add", "--"];
    let path_strs: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
    for p in &path_strs {
        args.push(p);
    }
    run_status(cwd, &args, "git add failed")
}

/// Create a commit with `message` against whatever is staged.
pub fn commit(cwd: &Path, message: &str) -> Result<()> {
    run_status(cwd, &["commit", "-m", message], "git commit failed")
}

/// Create an annotated tag `tag` pointing at HEAD with `message`.
pub fn create_annotated_tag(cwd: &Path, tag: &str, message: &str) -> Result<()> {
    run_status(cwd, &["tag", "-a", tag, "-m", message], "git tag failed")
}

/// Push `refspec` to `remote`.
pub fn push(cwd: &Path, remote: &str, refspec: &str) -> Result<()> {
    run_status(cwd, &["push", remote, refspec], "git push failed")
}

/// Walk upward from `start` until a directory containing `marker` is found.
/// Returns the directory (not the marker file itself).
pub fn find_ancestor_with(start: &Path, marker: &str) -> Option<std::path::PathBuf> {
    let mut cur = start;
    loop {
        if cur.join(marker).is_file() {
            return Some(cur.to_path_buf());
        }
        match cur.parent() {
            Some(p) => cur = p,
            None => return None,
        }
    }
}
