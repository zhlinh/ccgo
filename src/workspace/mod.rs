//! Workspace support for managing multiple related packages
//!
//! Workspaces allow you to manage multiple related C++ packages in a single
//! repository with shared dependencies and coordinated builds.

mod members;
mod resolver;

pub use members::{WorkspaceMember, WorkspaceMembers};
pub use resolver::WorkspaceResolver;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{CcgoConfig, WorkspaceConfig};

/// A workspace containing multiple member packages
#[derive(Debug)]
pub struct Workspace {
    /// Workspace root directory (where root CCGO.toml is located)
    pub root: PathBuf,

    /// Workspace configuration
    pub config: WorkspaceConfig,

    /// Root CCGO.toml configuration
    pub root_config: CcgoConfig,

    /// Discovered workspace members
    pub members: WorkspaceMembers,
}

impl Workspace {
    /// Load workspace from root directory
    pub fn load(root: &Path) -> Result<Self> {
        // Load root CCGO.toml
        let config_path = root.join("CCGO.toml");
        if !config_path.exists() {
            anyhow::bail!("Workspace root must contain CCGO.toml");
        }

        let root_config = CcgoConfig::load_from(&config_path)
            .context("Failed to load workspace root CCGO.toml")?;

        let workspace_config = root_config.workspace.clone()
            .ok_or_else(|| anyhow::anyhow!("CCGO.toml does not define a workspace"))?;

        // Discover workspace members
        let members = WorkspaceMembers::discover(root, &workspace_config)?;

        Ok(Self {
            root: root.to_path_buf(),
            config: workspace_config,
            root_config,
            members,
        })
    }

    /// Check if a directory is a workspace root
    pub fn is_workspace(path: &Path) -> bool {
        let config_path = path.join("CCGO.toml");
        if !config_path.exists() {
            return false;
        }

        if let Ok(config) = CcgoConfig::load_from(&config_path) {
            config.workspace.is_some()
        } else {
            false
        }
    }

    /// Get a specific workspace member by name
    pub fn get_member(&self, name: &str) -> Option<&WorkspaceMember> {
        self.members.get(name)
    }

    /// Get default members (or all members if default_members is empty)
    pub fn default_members(&self) -> Vec<&WorkspaceMember> {
        if self.config.default_members.is_empty() {
            self.members.all()
        } else {
            self.config.default_members
                .iter()
                .filter_map(|name| self.members.get(name))
                .collect()
        }
    }

    /// Resolve workspace dependencies for a specific member
    pub fn resolve_member_dependencies(
        &self,
        member: &WorkspaceMember,
    ) -> Result<Vec<crate::config::DependencyConfig>> {
        let resolver = WorkspaceResolver::new(self);
        resolver.resolve_member_dependencies(member)
    }

    /// Print workspace summary
    pub fn print_summary(&self) {
        println!("\n📦 Workspace: {}", self.root.display());
        println!("   Members: {}", self.members.len());

        if !self.config.dependencies.is_empty() {
            println!("   Shared dependencies: {}", self.config.dependencies.len());
        }

        println!("\n   Packages:");
        for member in self.members.all() {
            println!("      - {} ({})", member.name, member.version);
        }
    }
}

/// Find the workspace root by searching parent directories
pub fn find_workspace_root(start_path: &Path) -> Result<Option<PathBuf>> {
    let mut current = start_path.to_path_buf();

    loop {
        if Workspace::is_workspace(&current) {
            return Ok(Some(current));
        }

        // Move up to parent directory
        if let Some(parent) = current.parent() {
            current = parent.to_path_buf();
        } else {
            // Reached filesystem root without finding workspace
            return Ok(None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace root CCGO.toml
        fs::write(
            root.join("CCGO.toml"),
            r#"
[workspace]
members = ["core", "utils"]

[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
"#,
        )
        .unwrap();

        // Create core package
        let core_dir = root.join("core");
        fs::create_dir_all(&core_dir).unwrap();
        fs::write(
            core_dir.join("CCGO.toml"),
            r#"
[package]
name = "core"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true
"#,
        )
        .unwrap();

        // Create utils package
        let utils_dir = root.join("utils");
        fs::create_dir_all(&utils_dir).unwrap();
        fs::write(
            utils_dir.join("CCGO.toml"),
            r#"
[package]
name = "utils"
version = "1.0.0"
"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_is_workspace() {
        let temp_dir = create_test_workspace();
        assert!(Workspace::is_workspace(temp_dir.path()));
    }

    #[test]
    fn test_load_workspace() {
        let temp_dir = create_test_workspace();
        let workspace = Workspace::load(temp_dir.path()).unwrap();

        assert_eq!(workspace.members.len(), 2);
        assert!(workspace.get_member("core").is_some());
        assert!(workspace.get_member("utils").is_some());
    }

    #[test]
    fn test_find_workspace_root() {
        let temp_dir = create_test_workspace();
        let core_dir = temp_dir.path().join("core");

        let root = find_workspace_root(&core_dir).unwrap();
        assert!(root.is_some());
        assert_eq!(root.unwrap(), temp_dir.path());
    }
}
