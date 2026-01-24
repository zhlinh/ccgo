//! Automatic version discovery from Git repositories
//!
//! This module provides functionality to:
//! - Discover available versions (tags) from Git repositories
//! - Parse semantic versioning from Git tags
//! - Find the latest version matching a requirement

use anyhow::{bail, Context, Result};
use std::cmp::Ordering;
use std::process::Command;

/// Information about a Git tag/version
#[derive(Debug, Clone)]
pub struct GitTagInfo {
    /// Tag name (e.g., "v1.0.0", "1.0.0")
    pub tag: String,
    /// Parsed semantic version (if parseable)
    pub semver: Option<SemVer>,
    /// Commit hash this tag points to
    pub commit: Option<String>,
}

/// Simple semantic version representation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemVer {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub prerelease: Option<String>,
}

impl SemVer {
    /// Parse a version string into SemVer
    pub fn parse(input: &str) -> Option<Self> {
        // Strip leading 'v' if present
        let version = input.strip_prefix('v').unwrap_or(input);
        let version = version.strip_prefix('V').unwrap_or(version);

        // Split by '-' to separate prerelease
        let (version_part, prerelease) = if let Some(dash_pos) = version.find('-') {
            (&version[..dash_pos], Some(version[dash_pos + 1..].to_string()))
        } else {
            (version, None)
        };

        // Parse major.minor.patch
        let parts: Vec<&str> = version_part.split('.').collect();
        if parts.is_empty() || parts.len() > 3 {
            return None;
        }

        let major = parts.first()?.parse().ok()?;
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);

        Some(SemVer {
            major,
            minor,
            patch,
            prerelease,
        })
    }

    /// Check if this is a stable release (no prerelease)
    pub fn is_stable(&self) -> bool {
        self.prerelease.is_none()
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> Ordering {
        // Compare major.minor.patch
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Compare prerelease (None > Some for stability)
        match (&self.prerelease, &other.prerelease) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater, // Stable > prerelease
            (Some(_), None) => Ordering::Less,    // Prerelease < stable
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.prerelease {
            write!(f, "-{}", pre)?;
        }
        Ok(())
    }
}

/// Version discovery service
pub struct VersionDiscovery {
    /// Git executable path
    git_path: String,
}

impl Default for VersionDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl VersionDiscovery {
    /// Create a new version discovery service
    pub fn new() -> Self {
        Self {
            git_path: "git".to_string(),
        }
    }

    /// Discover all versions (tags) from a remote Git repository
    ///
    /// Uses `git ls-remote --tags` to fetch tags without cloning.
    pub fn discover_versions(&self, git_url: &str) -> Result<Vec<GitTagInfo>> {
        let output = Command::new(&self.git_path)
            .args(["ls-remote", "--tags", "--refs", git_url])
            .output()
            .context("Failed to execute git ls-remote")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Failed to list remote tags from {}: {}", git_url, stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tags = Vec::new();

        for line in stdout.lines() {
            // Format: <commit>\trefs/tags/<tagname>
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() != 2 {
                continue;
            }

            let commit = parts[0].to_string();
            let tag_ref = parts[1];

            // Extract tag name from refs/tags/<tagname>
            if let Some(tag) = tag_ref.strip_prefix("refs/tags/") {
                let semver = SemVer::parse(tag);
                tags.push(GitTagInfo {
                    tag: tag.to_string(),
                    semver,
                    commit: Some(commit),
                });
            }
        }

        // Sort by semver (highest first)
        tags.sort_by(|a, b| {
            match (&b.semver, &a.semver) {
                (Some(b_ver), Some(a_ver)) => b_ver.cmp(a_ver),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => b.tag.cmp(&a.tag),
            }
        });

        Ok(tags)
    }

    /// Find the latest stable version from a Git repository
    pub fn find_latest_stable(&self, git_url: &str) -> Result<Option<GitTagInfo>> {
        let versions = self.discover_versions(git_url)?;

        // Find first stable version (they're sorted highest first)
        Ok(versions
            .into_iter()
            .find(|t| t.semver.as_ref().map_or(false, |v| v.is_stable())))
    }

    /// Find the latest version (including prereleases)
    pub fn find_latest(&self, git_url: &str) -> Result<Option<GitTagInfo>> {
        let versions = self.discover_versions(git_url)?;
        Ok(versions.into_iter().next())
    }

    /// Find versions matching a version requirement
    pub fn find_matching(
        &self,
        git_url: &str,
        version_req: &str,
    ) -> Result<Vec<GitTagInfo>> {
        let versions = self.discover_versions(git_url)?;
        let req = crate::version::VersionReq::parse(version_req)
            .with_context(|| format!("Invalid version requirement: {}", version_req))?;

        let matching: Vec<GitTagInfo> = versions
            .into_iter()
            .filter(|t| {
                if let Some(ref semver) = t.semver {
                    // Convert to crate::version::Version for matching
                    let pre = match &semver.prerelease {
                        Some(p) => p.split('.').map(|s| s.to_string()).collect(),
                        None => Vec::new(),
                    };
                    let version = crate::version::Version {
                        major: semver.major as u64,
                        minor: semver.minor as u64,
                        patch: semver.patch as u64,
                        pre,
                        build: Vec::new(),
                    };
                    req.matches(&version)
                } else {
                    false
                }
            })
            .collect();

        Ok(matching)
    }
}

/// Convenience function to discover the latest version
pub fn discover_latest_version(git_url: &str, include_prereleases: bool) -> Result<Option<GitTagInfo>> {
    let discovery = VersionDiscovery::new();
    if include_prereleases {
        discovery.find_latest(git_url)
    } else {
        discovery.find_latest_stable(git_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semver_parse() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
        assert!(v.prerelease.is_none());

        let v = SemVer::parse("v1.2.3").unwrap();
        assert_eq!(v.major, 1);

        let v = SemVer::parse("1.2.3-beta.1").unwrap();
        assert_eq!(v.prerelease, Some("beta.1".to_string()));
    }

    #[test]
    fn test_semver_parse_partial() {
        let v = SemVer::parse("1").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);

        let v = SemVer::parse("1.2").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_semver_ordering() {
        let v1 = SemVer::parse("1.0.0").unwrap();
        let v2 = SemVer::parse("2.0.0").unwrap();
        assert!(v2 > v1);

        let v1 = SemVer::parse("1.1.0").unwrap();
        let v2 = SemVer::parse("1.2.0").unwrap();
        assert!(v2 > v1);

        let v1 = SemVer::parse("1.0.0-alpha").unwrap();
        let v2 = SemVer::parse("1.0.0").unwrap();
        assert!(v2 > v1); // Stable > prerelease
    }

    #[test]
    fn test_semver_display() {
        let v = SemVer::parse("1.2.3").unwrap();
        assert_eq!(format!("{}", v), "1.2.3");

        let v = SemVer::parse("1.2.3-beta").unwrap();
        assert_eq!(format!("{}", v), "1.2.3-beta");
    }

    #[test]
    fn test_is_stable() {
        assert!(SemVer::parse("1.0.0").unwrap().is_stable());
        assert!(!SemVer::parse("1.0.0-beta").unwrap().is_stable());
        assert!(!SemVer::parse("1.0.0-rc.1").unwrap().is_stable());
    }

    // Note: Integration tests for discover_versions would require network access
    // and a real Git repository. Those are better suited for integration tests.
}
