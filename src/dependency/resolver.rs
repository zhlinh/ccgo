//! Dependency resolver for transitive dependency resolution
//!
//! This module provides functionality to resolve transitive dependencies
//! by recursively reading CCGO.toml files from dependencies.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{CcgoConfig, DependencyConfig};
use crate::dependency::graph::{DependencyGraph, DependencyNode};

/// Maximum recursion depth to prevent infinite loops
const MAX_DEPTH: usize = 50;

/// Dependency resolver for resolving transitive dependencies
pub struct DependencyResolver {
    /// The dependency graph being built
    graph: DependencyGraph,

    /// Cache of already processed dependencies (name -> version)
    visited: HashMap<String, String>,

    /// Project root directory
    project_root: PathBuf,

    /// Global CCGO cache directory
    ccgo_home: PathBuf,
}

impl DependencyResolver {
    /// Create a new dependency resolver
    pub fn new(project_root: PathBuf, ccgo_home: PathBuf) -> Self {
        Self {
            graph: DependencyGraph::new(),
            visited: HashMap::new(),
            project_root,
            ccgo_home,
        }
    }

    /// Resolve all transitive dependencies starting from the given dependencies
    pub fn resolve(&mut self, dependencies: &[DependencyConfig]) -> Result<DependencyGraph> {
        println!("\n📊 Resolving dependency graph...");

        // Mark root dependencies
        for dep in dependencies {
            self.graph.add_root(dep.name.clone());
        }

        // Resolve each root dependency
        for dep in dependencies {
            self.resolve_dependency(dep, 0)?;
        }

        // Check for cycles
        if let Some(cycle) = self.graph.detect_cycles() {
            anyhow::bail!(
                "Circular dependency detected: {} -> {}",
                cycle.join(" -> "),
                cycle[0]
            );
        }

        println!("   ✓ Dependency graph resolved");

        Ok(self.graph.clone())
    }

    /// Resolve a single dependency and its transitive dependencies
    fn resolve_dependency(&mut self, dep: &DependencyConfig, depth: usize) -> Result<()> {
        // Check depth limit
        if depth >= MAX_DEPTH {
            anyhow::bail!(
                "Maximum dependency depth ({}) exceeded for '{}'",
                MAX_DEPTH,
                dep.name
            );
        }

        // Check if already processed
        if let Some(existing_version) = self.visited.get(&dep.name) {
            // Already processed, check version compatibility
            if !existing_version.is_empty() && !dep.version.is_empty() {
                if existing_version != &dep.version {
                    eprintln!(
                        "   ⚠️  Version conflict for '{}': have {}, need {}",
                        dep.name, existing_version, dep.version
                    );
                    // For now, use the first version encountered
                    // TODO: Implement smart version resolution
                }
            }
            return Ok(());
        }

        // Mark as visited with current version
        self.visited
            .insert(dep.name.clone(), dep.version.clone());

        // Find the dependency's CCGO.toml
        let dep_path = self.locate_dependency(dep)?;

        // Read the dependency's CCGO.toml
        let dep_config_path = dep_path.join("CCGO.toml");
        if !dep_config_path.exists() {
            // No CCGO.toml means no transitive dependencies
            self.add_node_to_graph(dep, vec![], depth);
            return Ok(());
        }

        let dep_config = CcgoConfig::load_from(&dep_config_path)
            .with_context(|| format!("Failed to load CCGO.toml for '{}'", dep.name))?;

        // Get the dependency's dependencies
        let mut transitive_deps = dep_config.dependencies;

        // Resolve relative paths in transitive dependencies relative to this dependency's directory
        for trans_dep in &mut transitive_deps {
            if let Some(ref path) = trans_dep.path {
                if !Path::new(path).is_absolute() {
                    // Resolve relative path from the current dependency's directory
                    let resolved_path = dep_path.join(path);
                    trans_dep.path = Some(resolved_path.to_string_lossy().to_string());
                }
            }
        }

        // Collect dependency names for the node
        let dep_names: Vec<String> = transitive_deps.iter().map(|d| d.name.clone()).collect();

        // Add this node to the graph
        self.add_node_to_graph(dep, dep_names.clone(), depth);

        // Recursively resolve transitive dependencies
        for trans_dep in &transitive_deps {
            // Add edge to graph: trans_dep -> dep (dependency must come before dependent)
            self.graph.add_edge(&trans_dep.name, &dep.name);

            // Resolve the transitive dependency
            self.resolve_dependency(trans_dep, depth + 1)?;
        }

        Ok(())
    }

    /// Locate the dependency's directory
    fn locate_dependency(&self, dep: &DependencyConfig) -> Result<PathBuf> {
        if let Some(ref path) = dep.path {
            // Path dependency
            let dep_path = if Path::new(path).is_absolute() {
                PathBuf::from(path)
            } else {
                self.project_root.join(path)
            };

            if !dep_path.exists() {
                anyhow::bail!("Path dependency '{}' not found at: {}", dep.name, dep_path.display());
            }

            Ok(dep_path)
        } else if dep.git.is_some() {
            // Git dependency - should be in global cache
            let hash_input = format!("{}:{}", dep.name, dep.git.as_ref().unwrap());
            let hash = format!("{:x}", md5::compute(hash_input.as_bytes()));
            let registry_name = format!("{}-{}", dep.name, &hash[..16]);
            let dep_path = self.ccgo_home.join("registry").join(&registry_name);

            if !dep_path.exists() {
                anyhow::bail!(
                    "Git dependency '{}' not found in cache. Run 'ccgo install' first.",
                    dep.name
                );
            }

            Ok(dep_path)
        } else {
            anyhow::bail!(
                "Dependency '{}' has no valid source (git or path)",
                dep.name
            );
        }
    }

    /// Add a node to the dependency graph
    fn add_node_to_graph(&mut self, dep: &DependencyConfig, dependencies: Vec<String>, depth: usize) {
        let source = self.build_source_string(dep);

        let node = DependencyNode {
            name: dep.name.clone(),
            version: dep.version.clone(),
            source,
            dependencies,
            depth,
            config: dep.clone(),
        };

        self.graph.add_node(node);
    }

    /// Build source string from dependency config
    fn build_source_string(&self, dep: &DependencyConfig) -> String {
        if let Some(ref git) = dep.git {
            format!("git+{}", git)
        } else if let Some(ref path) = dep.path {
            format!("path+{}", path)
        } else {
            format!("registry+{}@{}", dep.name, dep.version)
        }
    }

    /// Get the resolved dependency graph
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }
}

/// Resolve dependencies and return the dependency graph
pub fn resolve_dependencies(
    dependencies: &[DependencyConfig],
    project_root: &Path,
    ccgo_home: &Path,
) -> Result<DependencyGraph> {
    let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home.to_path_buf());
    resolver.resolve(dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    fn create_test_dependency(name: &str, deps: Vec<&str>) -> DependencyConfig {
        DependencyConfig {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            git: None,
            branch: None,
            path: Some(format!("./{}", name)),
            optional: false,
            features: vec![],
            default_features: Some(true),
            workspace: false,
        }
    }

    #[test]
    fn test_simple_resolution() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create dependency directories with CCGO.toml
        let dep_a_dir = project_root.join("dep_a");
        fs::create_dir_all(&dep_a_dir).unwrap();
        fs::write(
            dep_a_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_a"
version = "1.0.0"
"#,
        )
        .unwrap();

        let ccgo_home = temp_dir.path().join(".ccgo");
        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let deps = vec![create_test_dependency("dep_a", vec![])];

        let result = resolver.resolve(&deps);
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.nodes().len(), 1);
        assert!(graph.get_node("dep_a").is_some());
    }

    #[test]
    fn test_max_depth_exceeded() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        let ccgo_home = temp_dir.path().join(".ccgo");

        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let dep = create_test_dependency("test", vec![]);

        // Try to resolve at max depth + 1
        let result = resolver.resolve_dependency(&dep, MAX_DEPTH);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Maximum dependency depth"));
    }

    #[test]
    fn test_transitive_dependencies() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a dependency chain: A -> B -> C
        // Create dep_c (no dependencies)
        let dep_c_dir = project_root.join("dep_c");
        fs::create_dir_all(&dep_c_dir).unwrap();
        fs::write(
            dep_c_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_c"
version = "1.0.0"
"#,
        )
        .unwrap();

        // Create dep_b (depends on dep_c)
        let dep_b_dir = project_root.join("dep_b");
        fs::create_dir_all(&dep_b_dir).unwrap();
        fs::write(
            dep_b_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_b"
version = "1.0.0"

[[dependencies]]
name = "dep_c"
version = "1.0.0"
path = "../dep_c"
"#,
        )
        .unwrap();

        // Create dep_a (depends on dep_b)
        let dep_a_dir = project_root.join("dep_a");
        fs::create_dir_all(&dep_a_dir).unwrap();
        fs::write(
            dep_a_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_a"
version = "1.0.0"

[[dependencies]]
name = "dep_b"
version = "1.0.0"
path = "../dep_b"
"#,
        )
        .unwrap();

        let ccgo_home = temp_dir.path().join(".ccgo");
        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let deps = vec![create_test_dependency("dep_a", vec![])];

        let result = resolver.resolve(&deps);
        assert!(result.is_ok());

        let graph = result.unwrap();
        // Should have all three dependencies
        assert_eq!(graph.nodes().len(), 3);
        assert!(graph.get_node("dep_a").is_some());
        assert!(graph.get_node("dep_b").is_some());
        assert!(graph.get_node("dep_c").is_some());

        // Verify topological sort puts dependencies before dependents
        let sorted = graph.topological_sort().unwrap();
        let pos_c = sorted.iter().position(|n| n == "dep_c").unwrap();
        let pos_b = sorted.iter().position(|n| n == "dep_b").unwrap();
        let pos_a = sorted.iter().position(|n| n == "dep_a").unwrap();
        assert!(pos_c < pos_b, "dep_c should come before dep_b");
        assert!(pos_b < pos_a, "dep_b should come before dep_a");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a circular dependency: A -> B -> C -> A
        let dep_a_dir = project_root.join("dep_a");
        fs::create_dir_all(&dep_a_dir).unwrap();
        fs::write(
            dep_a_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_a"
version = "1.0.0"

[[dependencies]]
name = "dep_b"
version = "1.0.0"
path = "../dep_b"
"#,
        )
        .unwrap();

        let dep_b_dir = project_root.join("dep_b");
        fs::create_dir_all(&dep_b_dir).unwrap();
        fs::write(
            dep_b_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_b"
version = "1.0.0"

[[dependencies]]
name = "dep_c"
version = "1.0.0"
path = "../dep_c"
"#,
        )
        .unwrap();

        let dep_c_dir = project_root.join("dep_c");
        fs::create_dir_all(&dep_c_dir).unwrap();
        fs::write(
            dep_c_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_c"
version = "1.0.0"

[[dependencies]]
name = "dep_a"
version = "1.0.0"
path = "../dep_a"
"#,
        )
        .unwrap();

        let ccgo_home = temp_dir.path().join(".ccgo");
        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let deps = vec![create_test_dependency("dep_a", vec![])];

        let result = resolver.resolve(&deps);
        // Should detect circular dependency
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Circular dependency detected"));
    }

    #[test]
    fn test_shared_dependency() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create a diamond dependency: A -> B -> D, A -> C -> D
        let dep_d_dir = project_root.join("dep_d");
        fs::create_dir_all(&dep_d_dir).unwrap();
        fs::write(
            dep_d_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_d"
version = "1.0.0"
"#,
        )
        .unwrap();

        let dep_b_dir = project_root.join("dep_b");
        fs::create_dir_all(&dep_b_dir).unwrap();
        fs::write(
            dep_b_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_b"
version = "1.0.0"

[[dependencies]]
name = "dep_d"
version = "1.0.0"
path = "../dep_d"
"#,
        )
        .unwrap();

        let dep_c_dir = project_root.join("dep_c");
        fs::create_dir_all(&dep_c_dir).unwrap();
        fs::write(
            dep_c_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_c"
version = "1.0.0"

[[dependencies]]
name = "dep_d"
version = "1.0.0"
path = "../dep_d"
"#,
        )
        .unwrap();

        let dep_a_dir = project_root.join("dep_a");
        fs::create_dir_all(&dep_a_dir).unwrap();
        fs::write(
            dep_a_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_a"
version = "1.0.0"

[[dependencies]]
name = "dep_b"
version = "1.0.0"
path = "../dep_b"

[[dependencies]]
name = "dep_c"
version = "1.0.0"
path = "../dep_c"
"#,
        )
        .unwrap();

        let ccgo_home = temp_dir.path().join(".ccgo");
        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let deps = vec![create_test_dependency("dep_a", vec![])];

        let result = resolver.resolve(&deps);
        assert!(result.is_ok());

        let graph = result.unwrap();
        // Should have 4 unique dependencies (A, B, C, D)
        assert_eq!(graph.nodes().len(), 4);

        let stats = graph.stats();
        // D is shared between B and C, so total > unique
        assert!(stats.shared_count > 0);
    }

    #[test]
    fn test_missing_ccgo_toml() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create dependency directory without CCGO.toml
        let dep_a_dir = project_root.join("dep_a");
        fs::create_dir_all(&dep_a_dir).unwrap();
        // Intentionally not creating CCGO.toml

        let ccgo_home = temp_dir.path().join(".ccgo");
        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let deps = vec![create_test_dependency("dep_a", vec![])];

        let result = resolver.resolve(&deps);
        // Should succeed - missing CCGO.toml means no transitive deps
        assert!(result.is_ok());

        let graph = result.unwrap();
        assert_eq!(graph.nodes().len(), 1);
        assert!(graph.get_node("dep_a").is_some());

        // Should have no dependencies
        let node = graph.get_node("dep_a").unwrap();
        assert_eq!(node.dependencies.len(), 0);
    }

    #[test]
    fn test_version_conflict_warning() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // Create dep_d with version 1.0.0
        let dep_d_dir = project_root.join("dep_d");
        fs::create_dir_all(&dep_d_dir).unwrap();
        fs::write(
            dep_d_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_d"
version = "2.0.0"
"#,
        )
        .unwrap();

        // Create dep_b requiring dep_d v1.0.0
        let dep_b_dir = project_root.join("dep_b");
        fs::create_dir_all(&dep_b_dir).unwrap();
        fs::write(
            dep_b_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_b"
version = "1.0.0"

[[dependencies]]
name = "dep_d"
version = "1.0.0"
path = "../dep_d"
"#,
        )
        .unwrap();

        // Create dep_c requiring dep_d v2.0.0
        let dep_c_dir = project_root.join("dep_c");
        fs::create_dir_all(&dep_c_dir).unwrap();
        fs::write(
            dep_c_dir.join("CCGO.toml"),
            r#"
[package]
name = "dep_c"
version = "1.0.0"

[[dependencies]]
name = "dep_d"
version = "2.0.0"
path = "../dep_d"
"#,
        )
        .unwrap();

        let ccgo_home = temp_dir.path().join(".ccgo");
        let mut resolver = DependencyResolver::new(project_root.to_path_buf(), ccgo_home);

        let deps = vec![
            create_test_dependency("dep_b", vec![]),
            create_test_dependency("dep_c", vec![]),
        ];

        // Should succeed but print warning (we can't easily test stderr in unit tests)
        let result = resolver.resolve(&deps);
        assert!(result.is_ok());

        let graph = result.unwrap();
        // dep_d should only appear once (first version wins for now)
        assert_eq!(graph.nodes().len(), 3); // b, c, d
    }
}
