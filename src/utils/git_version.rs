// Git version information utilities
// Matches Python ccgo's git version calculation logic

use anyhow::{Context, Result};
use std::path::Path;
use std::process::Command;

/// Git version information
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GitVersion {
    /// Full version string with suffix (e.g., "1.0.2-beta.18-dirty")
    pub version_with_suffix: String,
    /// Version suffix only (e.g., "beta.18-dirty" or "release")
    pub publish_suffix: String,
    /// Git branch name
    pub branch_name: String,
    /// Git revision (short commit hash)
    pub revision: String,
    /// Whether working directory has uncommitted changes
    pub is_dirty: bool,
    /// Number of commits since last tag
    pub commits_since_tag: u32,
}

impl GitVersion {
    /// Calculate git version information from project root
    pub fn from_project_root(project_root: &Path, base_version: &str) -> Result<Self> {
        // Get git branch name
        let branch_name = get_git_branch(project_root)?;

        // Get git revision (short hash)
        let revision = get_git_revision(project_root)?;

        // Check if working directory is dirty
        let is_dirty = is_git_dirty(project_root)?;

        // Get commits since last tag
        let commits_since_tag = get_commits_since_tag(project_root)?;

        // Calculate publish suffix
        let publish_suffix = calculate_publish_suffix(commits_since_tag, is_dirty);

        // Build full version string
        let version_with_suffix = if publish_suffix == "release" {
            base_version.to_string()
        } else {
            format!("{}-{}", base_version, publish_suffix)
        };

        Ok(GitVersion {
            version_with_suffix,
            publish_suffix,
            branch_name,
            revision,
            is_dirty,
            commits_since_tag,
        })
    }
}

/// Get current git branch name
fn get_git_branch(project_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project_root)
        .output()
        .context("Failed to get git branch")?;

    if !output.status.success() {
        return Ok("unknown".to_string());
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if branch.is_empty() { "unknown".to_string() } else { branch })
}

/// Get current git revision (short commit hash)
fn get_git_revision(project_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .current_dir(project_root)
        .output()
        .context("Failed to get git revision")?;

    if !output.status.success() {
        return Ok("unknown".to_string());
    }

    let revision = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(if revision.is_empty() { "unknown".to_string() } else { revision })
}

/// Check if working directory has uncommitted changes
fn is_git_dirty(project_root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(project_root)
        .output()
        .context("Failed to check git status")?;

    if !output.status.success() {
        return Ok(false);
    }

    let status = String::from_utf8_lossy(&output.stdout);
    Ok(!status.trim().is_empty())
}

/// Get number of commits since last tag
fn get_commits_since_tag(project_root: &Path) -> Result<u32> {
    // Try to get the most recent tag
    let tag_output = Command::new("git")
        .args(&["describe", "--tags", "--abbrev=0"])
        .current_dir(project_root)
        .output()
        .context("Failed to get git tags")?;

    if !tag_output.status.success() {
        // No tags exist, count all commits
        return count_all_commits(project_root);
    }

    let latest_tag = String::from_utf8_lossy(&tag_output.stdout).trim().to_string();
    if latest_tag.is_empty() {
        return count_all_commits(project_root);
    }

    // Count commits since the tag
    let count_output = Command::new("git")
        .args(&["rev-list", &format!("{}..HEAD", latest_tag), "--count"])
        .current_dir(project_root)
        .output()
        .context("Failed to count commits since tag")?;

    if !count_output.status.success() {
        return Ok(0);
    }

    let count_str = String::from_utf8_lossy(&count_output.stdout).trim().to_string();
    Ok(count_str.parse::<u32>().unwrap_or(0))
}

/// Count all commits in the repository
fn count_all_commits(project_root: &Path) -> Result<u32> {
    let output = Command::new("git")
        .args(&["rev-list", "HEAD", "--count"])
        .current_dir(project_root)
        .output()
        .context("Failed to count all commits")?;

    if !output.status.success() {
        return Ok(0);
    }

    let count_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(count_str.parse::<u32>().unwrap_or(0))
}

/// Calculate publish suffix based on commits and dirty status
/// Matches Python ccgo logic:
/// - If 0 commits since tag and clean: "release"
/// - If N commits since tag: "beta.N" or "beta.N-dirty"
fn calculate_publish_suffix(commits_since_tag: u32, is_dirty: bool) -> String {
    if commits_since_tag == 0 && !is_dirty {
        "release".to_string()
    } else {
        let base = if commits_since_tag > 0 {
            format!("beta.{}", commits_since_tag)
        } else {
            "beta.0".to_string()
        };

        if is_dirty {
            format!("{}-dirty", base)
        } else {
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_publish_suffix_release() {
        assert_eq!(calculate_publish_suffix(0, false), "release");
    }

    #[test]
    fn test_publish_suffix_beta() {
        assert_eq!(calculate_publish_suffix(5, false), "beta.5");
        assert_eq!(calculate_publish_suffix(18, false), "beta.18");
    }

    #[test]
    fn test_publish_suffix_dirty() {
        assert_eq!(calculate_publish_suffix(0, true), "beta.0-dirty");
        assert_eq!(calculate_publish_suffix(18, true), "beta.18-dirty");
    }
}
