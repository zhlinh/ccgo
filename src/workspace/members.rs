//! Workspace member discovery and management

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use glob::glob;

use crate::config::{CcgoConfig, WorkspaceConfig};

/// A workspace member package
#[derive(Debug, Clone)]
pub struct WorkspaceMember {
    /// Package name
    pub name: String,

    /// Package version
    pub version: String,

    /// Absolute path to member directory
    pub path: PathBuf,

    /// Member's CCGO.toml configuration
    pub config: CcgoConfig,
}

impl WorkspaceMember {
    /// Load a workspace member from a directory
    pub fn load(path: &Path) -> Result<Self> {
        let config_path = path.join("CCGO.toml");
        let config = CcgoConfig::load_from(&config_path)
            .with_context(|| format!("Failed to load member at {}", path.display()))?;

        let package = config.package.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Workspace member must have [package] section"))?;

        Ok(Self {
            name: package.name.clone(),
            version: package.version.clone(),
            path: path.to_path_buf(),
            config,
        })
    }
}

/// Collection of workspace members
#[derive(Debug)]
pub struct WorkspaceMembers {
    /// Map of member name to member info
    members: HashMap<String, WorkspaceMember>,
}

impl WorkspaceMembers {
    /// Discover workspace members from workspace configuration
    pub fn discover(workspace_root: &Path, config: &WorkspaceConfig) -> Result<Self> {
        let mut members = HashMap::new();

        // Expand member patterns
        let member_paths = Self::expand_member_patterns(workspace_root, config)?;

        // Load each member
        for path in member_paths {
            let member = WorkspaceMember::load(&path)?;

            // Check for duplicate names
            if members.contains_key(&member.name) {
                anyhow::bail!(
                    "Duplicate workspace member name '{}' at {}",
                    member.name,
                    path.display()
                );
            }

            members.insert(member.name.clone(), member);
        }

        if members.is_empty() {
            println!("   ⚠️  No workspace members found");
        }

        Ok(Self { members })
    }

    /// Expand glob patterns in workspace members
    fn expand_member_patterns(workspace_root: &Path, config: &WorkspaceConfig) -> Result<Vec<PathBuf>> {
        let mut paths = Vec::new();

        for pattern in &config.members {
            // Convert relative pattern to absolute
            let abs_pattern = workspace_root.join(pattern);
            let pattern_str = abs_pattern.to_string_lossy();

            // Check if pattern contains glob wildcards
            if pattern_str.contains('*') || pattern_str.contains('?') || pattern_str.contains('[') {
                // Use glob to expand pattern
                for entry in glob(&pattern_str)
                    .with_context(|| format!("Invalid glob pattern: {}", pattern))?
                {
                    let path = entry
                        .with_context(|| format!("Failed to read glob entry for {}", pattern))?;

                    if path.is_dir() {
                        // Check if excluded
                        if !Self::is_excluded(&path, workspace_root, &config.exclude) {
                            // Check if it has CCGO.toml
                            if path.join("CCGO.toml").exists() {
                                paths.push(path);
                            }
                        }
                    }
                }
            } else {
                // No glob - direct path
                let path = workspace_root.join(pattern);
                if !path.exists() {
                    anyhow::bail!("Workspace member not found: {}", pattern);
                }
                if !path.is_dir() {
                    anyhow::bail!("Workspace member must be a directory: {}", pattern);
                }
                if !path.join("CCGO.toml").exists() {
                    anyhow::bail!("Workspace member missing CCGO.toml: {}", pattern);
                }

                if !Self::is_excluded(&path, workspace_root, &config.exclude) {
                    paths.push(path);
                }
            }
        }

        Ok(paths)
    }

    /// Check if a path should be excluded
    fn is_excluded(path: &Path, workspace_root: &Path, exclude: &[String]) -> bool {
        for pattern in exclude {
            // Make pattern relative to workspace root for comparison
            let abs_pattern = workspace_root.join(pattern);

            // Simple string comparison for now
            // TODO: Support glob patterns in exclude
            if path == abs_pattern {
                return true;
            }

            // Check if path starts with excluded directory
            if path.starts_with(&abs_pattern) {
                return true;
            }
        }

        false
    }

    /// Get a member by name
    pub fn get(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.get(name)
    }

    /// Get all members
    pub fn all(&self) -> Vec<&WorkspaceMember> {
        self.members.values().collect()
    }

    /// Get number of members
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Get member names
    pub fn names(&self) -> Vec<&str> {
        self.members.keys().map(|s| s.as_str()).collect()
    }

    /// Filter members by predicate
    pub fn filter<F>(&self, predicate: F) -> Vec<&WorkspaceMember>
    where
        F: Fn(&WorkspaceMember) -> bool,
    {
        self.members.values().filter(|m| predicate(m)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_load_member() {
        let temp_dir = TempDir::new().unwrap();
        let member_dir = temp_dir.path().join("test-member");
        fs::create_dir_all(&member_dir).unwrap();

        fs::write(
            member_dir.join("CCGO.toml"),
            r#"
[package]
name = "test-member"
version = "1.0.0"
"#,
        )
        .unwrap();

        let member = WorkspaceMember::load(&member_dir).unwrap();
        assert_eq!(member.name, "test-member");
        assert_eq!(member.version, "1.0.0");
    }

    #[test]
    fn test_discover_members_direct() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create member1
        let member1 = root.join("member1");
        fs::create_dir_all(&member1).unwrap();
        fs::write(
            member1.join("CCGO.toml"),
            r#"
[package]
name = "member1"
version = "1.0.0"
"#,
        )
        .unwrap();

        // Create member2
        let member2 = root.join("member2");
        fs::create_dir_all(&member2).unwrap();
        fs::write(
            member2.join("CCGO.toml"),
            r#"
[package]
name = "member2"
version = "2.0.0"
"#,
        )
        .unwrap();

        let config = WorkspaceConfig {
            members: vec!["member1".to_string(), "member2".to_string()],
            exclude: vec![],
            ..Default::default()
        };

        let members = WorkspaceMembers::discover(root, &config).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.get("member1").is_some());
        assert!(members.get("member2").is_some());
    }

    #[test]
    fn test_discover_members_glob() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create crates/foo
        let foo = root.join("crates/foo");
        fs::create_dir_all(&foo).unwrap();
        fs::write(
            foo.join("CCGO.toml"),
            r#"
[package]
name = "foo"
version = "1.0.0"
"#,
        )
        .unwrap();

        // Create crates/bar
        let bar = root.join("crates/bar");
        fs::create_dir_all(&bar).unwrap();
        fs::write(
            bar.join("CCGO.toml"),
            r#"
[package]
name = "bar"
version = "1.0.0"
"#,
        )
        .unwrap();

        let config = WorkspaceConfig {
            members: vec!["crates/*".to_string()],
            exclude: vec![],
            ..Default::default()
        };

        let members = WorkspaceMembers::discover(root, &config).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.get("foo").is_some());
        assert!(members.get("bar").is_some());
    }

    #[test]
    fn test_exclude_members() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create member1
        let member1 = root.join("member1");
        fs::create_dir_all(&member1).unwrap();
        fs::write(
            member1.join("CCGO.toml"),
            r#"
[package]
name = "member1"
version = "1.0.0"
"#,
        )
        .unwrap();

        // Create member2 (to be excluded)
        let member2 = root.join("member2");
        fs::create_dir_all(&member2).unwrap();
        fs::write(
            member2.join("CCGO.toml"),
            r#"
[package]
name = "member2"
version = "2.0.0"
"#,
        )
        .unwrap();

        let config = WorkspaceConfig {
            members: vec!["member*".to_string()],
            exclude: vec!["member2".to_string()],
            ..Default::default()
        };

        let members = WorkspaceMembers::discover(root, &config).unwrap();
        assert_eq!(members.len(), 1);
        assert!(members.get("member1").is_some());
        assert!(members.get("member2").is_none());
    }
}
