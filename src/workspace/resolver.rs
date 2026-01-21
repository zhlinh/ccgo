//! Workspace dependency resolution

use anyhow::{Context, Result};

use crate::config::DependencyConfig;
use crate::workspace::{Workspace, WorkspaceMember};

/// Workspace dependency resolver
pub struct WorkspaceResolver<'a> {
    workspace: &'a Workspace,
}

impl<'a> WorkspaceResolver<'a> {
    /// Create a new workspace resolver
    pub fn new(workspace: &'a Workspace) -> Self {
        Self { workspace }
    }

    /// Resolve dependencies for a specific workspace member
    ///
    /// This resolves:
    /// 1. Member's direct dependencies
    /// 2. Workspace dependencies (when `workspace = true`)
    /// 3. Inter-workspace dependencies (other members as dependencies)
    pub fn resolve_member_dependencies(
        &self,
        member: &WorkspaceMember,
    ) -> Result<Vec<DependencyConfig>> {
        let mut resolved = Vec::new();

        // Process each dependency
        for dep in &member.config.dependencies {
            if dep.workspace {
                // Inherit from workspace dependencies
                let workspace_dep = self.find_workspace_dependency(&dep.name)
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Member '{}' declares dependency '{}' with workspace=true, \
                             but it's not defined in workspace dependencies",
                            member.name,
                            dep.name
                        )
                    })?;

                // Use workspace dependency, but allow member to override features
                let mut resolved_dep = workspace_dep;
                if !dep.features.is_empty() {
                    // Merge features from member
                    resolved_dep.features.extend(dep.features.clone());
                    resolved_dep.features.sort();
                    resolved_dep.features.dedup();
                }

                // Override default_features if specified by member
                if dep.default_features.is_some() {
                    resolved_dep.default_features = dep.default_features;
                }

                resolved.push(resolved_dep);
            } else {
                // Use member's own dependency
                resolved.push(dep.clone());
            }
        }

        Ok(resolved)
    }

    /// Find a workspace-level dependency by name
    fn find_workspace_dependency(&self, name: &str) -> Option<DependencyConfig> {
        self.workspace
            .config
            .dependencies
            .iter()
            .find(|dep| dep.name == name)
            .map(|workspace_dep| workspace_dep.to_dependency_config())
    }

    /// Resolve all workspace dependencies
    ///
    /// Returns a map of member name to resolved dependencies
    pub fn resolve_all(&self) -> Result<std::collections::HashMap<String, Vec<DependencyConfig>>> {
        let mut resolved = std::collections::HashMap::new();

        for member in self.workspace.members.all() {
            let deps = self.resolve_member_dependencies(member)?;
            resolved.insert(member.name.clone(), deps);
        }

        Ok(resolved)
    }

    /// Check if a member depends on another workspace member
    pub fn depends_on_member(&self, member: &WorkspaceMember, target: &str) -> Result<bool> {
        let deps = self.resolve_member_dependencies(member)?;

        Ok(deps.iter().any(|dep| {
            // Check if this is a path dependency pointing to another workspace member
            if let Some(ref path) = dep.path {
                // Resolve the path relative to member's directory
                let dep_path = member.path.join(path);

                // Check if it points to a workspace member
                if let Some(target_member) = self.workspace.get_member(target) {
                    // Compare canonical paths if possible
                    if let (Ok(dep_canon), Ok(target_canon)) = (
                        dep_path.canonicalize(),
                        target_member.path.canonicalize(),
                    ) {
                        return dep_canon == target_canon;
                    }
                }
            }

            // Also check by name
            dep.name == target
        }))
    }

    /// Get build order for workspace members based on inter-member dependencies
    ///
    /// Returns members in topological order (dependencies before dependents)
    pub fn build_order(&self) -> Result<Vec<&WorkspaceMember>> {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut visiting = std::collections::HashSet::new();

        for member in self.workspace.members.all() {
            self.visit_member(member, &mut visited, &mut visiting, &mut order)?;
        }

        Ok(order)
    }

    /// Depth-first search for topological sort
    fn visit_member<'b>(
        &self,
        member: &'b WorkspaceMember,
        visited: &mut std::collections::HashSet<String>,
        visiting: &mut std::collections::HashSet<String>,
        order: &mut Vec<&'b WorkspaceMember>,
    ) -> Result<()>
    where
        'a: 'b,
    {
        if visited.contains(&member.name) {
            return Ok(());
        }

        if visiting.contains(&member.name) {
            anyhow::bail!("Circular dependency detected involving '{}'", member.name);
        }

        visiting.insert(member.name.clone());

        // Visit dependencies first
        for other_member in self.workspace.members.all() {
            if other_member.name != member.name {
                if self.depends_on_member(member, &other_member.name)? {
                    self.visit_member(other_member, visited, visiting, order)?;
                }
            }
        }

        visiting.remove(&member.name);
        visited.insert(member.name.clone());
        order.push(member);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CcgoConfig, PackageConfig, WorkspaceConfig, WorkspaceDependency};
    use std::fs;
    use tempfile::TempDir;

    fn create_test_workspace_with_deps() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace root
        fs::write(
            root.join("CCGO.toml"),
            r#"
[workspace]
members = ["core", "utils"]

[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"

[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
"#,
        )
        .unwrap();

        // Create core package (uses workspace deps)
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

[[dependencies]]
name = "spdlog"
workspace = true
features = ["async"]
"#,
        )
        .unwrap();

        // Create utils package (own deps + workspace deps)
        let utils_dir = root.join("utils");
        fs::create_dir_all(&utils_dir).unwrap();
        fs::write(
            utils_dir.join("CCGO.toml"),
            r#"
[package]
name = "utils"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true

[[dependencies]]
name = "boost"
version = "1.80.0"
"#,
        )
        .unwrap();

        temp_dir
    }

    #[test]
    fn test_resolve_workspace_dependencies() {
        let temp_dir = create_test_workspace_with_deps();
        let workspace = Workspace::load(temp_dir.path()).unwrap();

        let core = workspace.get_member("core").unwrap();
        let resolver = WorkspaceResolver::new(&workspace);

        let deps = resolver.resolve_member_dependencies(core).unwrap();

        // Should have 2 dependencies (fmt and spdlog from workspace)
        assert_eq!(deps.len(), 2);

        // Check fmt
        let fmt = deps.iter().find(|d| d.name == "fmt").unwrap();
        assert_eq!(fmt.version, "10.0.0");

        // Check spdlog (with additional features)
        let spdlog = deps.iter().find(|d| d.name == "spdlog").unwrap();
        assert_eq!(spdlog.version, "1.12.0");
        assert!(spdlog.features.contains(&"async".to_string()));
    }

    #[test]
    fn test_resolve_mixed_dependencies() {
        let temp_dir = create_test_workspace_with_deps();
        let workspace = Workspace::load(temp_dir.path()).unwrap();

        let utils = workspace.get_member("utils").unwrap();
        let resolver = WorkspaceResolver::new(&workspace);

        let deps = resolver.resolve_member_dependencies(utils).unwrap();

        // Should have 2 dependencies (1 workspace, 1 own)
        assert_eq!(deps.len(), 2);

        // Check fmt (from workspace)
        let fmt = deps.iter().find(|d| d.name == "fmt").unwrap();
        assert_eq!(fmt.version, "10.0.0");

        // Check boost (own dependency)
        let boost = deps.iter().find(|d| d.name == "boost").unwrap();
        assert_eq!(boost.version, "1.80.0");
    }

    #[test]
    fn test_missing_workspace_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create workspace without defining 'missing' dependency
        fs::write(
            root.join("CCGO.toml"),
            r#"
[workspace]
members = ["member"]
"#,
        )
        .unwrap();

        // Create member that tries to use non-existent workspace dependency
        let member_dir = root.join("member");
        fs::create_dir_all(&member_dir).unwrap();
        fs::write(
            member_dir.join("CCGO.toml"),
            r#"
[package]
name = "member"
version = "1.0.0"

[[dependencies]]
name = "missing"
workspace = true
"#,
        )
        .unwrap();

        let workspace = Workspace::load(root).unwrap();
        let member = workspace.get_member("member").unwrap();
        let resolver = WorkspaceResolver::new(&workspace);

        let result = resolver.resolve_member_dependencies(member);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not defined in workspace dependencies"));
    }
}
