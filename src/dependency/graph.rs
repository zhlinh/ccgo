//! Dependency graph data structures and algorithms
//!
//! Provides dependency graph representation, cycle detection, and topological sorting.

use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{bail, Result};

use crate::config::DependencyConfig;

/// A node in the dependency graph
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Dependency name
    pub name: String,

    /// Resolved version
    pub version: String,

    /// Source specification (git URL, path, etc.)
    pub source: String,

    /// Direct dependencies of this node
    pub dependencies: Vec<String>,

    /// Depth in dependency tree (0 = root)
    pub depth: usize,

    /// Original dependency config
    pub config: DependencyConfig,
}

/// Dependency graph for managing project dependencies
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// All nodes in the graph (keyed by dependency name)
    nodes: HashMap<String, DependencyNode>,

    /// Edges: (from, to) representing dependency relationships
    edges: Vec<(String, String)>,

    /// Root dependencies (directly declared in CCGO.toml)
    roots: HashSet<String>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            roots: HashSet::new(),
        }
    }

    /// Add a root dependency (directly declared)
    pub fn add_root(&mut self, name: String) {
        self.roots.insert(name);
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: DependencyNode) {
        self.nodes.insert(node.name.clone(), node);
    }

    /// Add an edge from one dependency to another
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push((from.to_string(), to.to_string()));
    }

    /// Get a node by name
    pub fn get_node(&self, name: &str) -> Option<&DependencyNode> {
        self.nodes.get(name)
    }

    /// Get all nodes
    pub fn nodes(&self) -> &HashMap<String, DependencyNode> {
        &self.nodes
    }

    /// Get all root dependencies
    pub fn roots(&self) -> &HashSet<String> {
        &self.roots
    }

    /// Detect cycles in the dependency graph
    ///
    /// Returns Some(cycle) if a cycle is detected, None otherwise.
    /// The cycle is represented as a Vec of dependency names forming the cycle.
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        // Build adjacency list for efficient traversal
        let adj_list = self.build_adjacency_list();

        // Try to find a cycle starting from each node
        for node_name in self.nodes.keys() {
            if !visited.contains(node_name) {
                if let Some(cycle) = self.dfs_cycle_detection(
                    node_name,
                    &adj_list,
                    &mut visited,
                    &mut rec_stack,
                    &mut path,
                ) {
                    return Some(cycle);
                }
            }
        }

        None
    }

    /// DFS-based cycle detection helper
    fn dfs_cycle_detection(
        &self,
        node: &str,
        adj_list: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(neighbors) = adj_list.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if let Some(cycle) =
                        self.dfs_cycle_detection(neighbor, adj_list, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(neighbor) {
                    // Found a cycle! Extract it from the path
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    return Some(path[cycle_start..].to_vec());
                }
            }
        }

        rec_stack.remove(node);
        path.pop();
        None
    }

    /// Build adjacency list representation
    fn build_adjacency_list(&self) -> HashMap<String, Vec<String>> {
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();

        for (from, to) in &self.edges {
            adj_list
                .entry(from.clone())
                .or_insert_with(Vec::new)
                .push(to.clone());
        }

        adj_list
    }

    /// Perform topological sort to determine build order
    ///
    /// Returns a Vec of dependency names in the order they should be built.
    /// Dependencies with no dependents come first.
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        // First check for cycles
        if let Some(cycle) = self.detect_cycles() {
            bail!(
                "Circular dependency detected: {}",
                cycle.join(" -> ") + " -> " + &cycle[0]
            );
        }

        let adj_list = self.build_adjacency_list();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize in-degree for all nodes
        for node_name in self.nodes.keys() {
            in_degree.insert(node_name.clone(), 0);
        }

        // Calculate in-degrees
        for neighbors in adj_list.values() {
            for neighbor in neighbors {
                *in_degree.entry(neighbor.clone()).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no dependencies (in-degree = 0)
        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(name, _)| name.clone())
            .collect();

        let mut sorted = Vec::new();

        while let Some(node) = queue.pop_front() {
            sorted.push(node.clone());

            // Reduce in-degree for all neighbors
            if let Some(neighbors) = adj_list.get(&node) {
                for neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(neighbor.clone());
                        }
                    }
                }
            }
        }

        // If we haven't processed all nodes, there's a cycle
        // (should be caught by detect_cycles, but double-check)
        if sorted.len() != self.nodes.len() {
            bail!("Failed to resolve dependency order - possible circular dependency");
        }

        Ok(sorted)
    }

    /// Get the dependency tree as a string for display
    pub fn format_tree(&self, indent: usize) -> String {
        let mut output = String::new();
        let mut visited = HashSet::new();

        // Start with root dependencies
        for root_name in &self.roots {
            if let Some(node) = self.nodes.get(root_name) {
                self.format_node(node, &mut output, &mut visited, 0, indent, true);
            }
        }

        output
    }

    /// Format a single node and its children
    fn format_node(
        &self,
        node: &DependencyNode,
        output: &mut String,
        visited: &mut HashSet<String>,
        depth: usize,
        indent: usize,
        is_last: bool,
    ) {
        // Indentation
        let prefix = if depth == 0 {
            String::new()
        } else {
            "  ".repeat(depth - 1) + if is_last { "└── " } else { "├── " }
        };

        // Check if already visited (shared dependency)
        let already_visited = visited.contains(&node.name);
        let marker = if already_visited {
            " (already resolved)"
        } else {
            ""
        };

        output.push_str(&format!(
            "{}{} v{}{}",
            prefix, node.name, node.version, marker
        ));

        // Show source info if not a path
        if !node.source.starts_with("path+") {
            let source_info = if node.source.starts_with("git+") {
                let url = node
                    .source
                    .strip_prefix("git+")
                    .unwrap_or(&node.source)
                    .split('#')
                    .next()
                    .unwrap_or("");
                format!(" (git: {})", url)
            } else {
                format!(" ({})", node.source)
            };
            output.push_str(&source_info);
        }

        output.push('\n');

        // Don't recurse if already visited
        if already_visited {
            return;
        }

        visited.insert(node.name.clone());

        // Recurse for dependencies
        let deps_count = node.dependencies.len();
        for (i, dep_name) in node.dependencies.iter().enumerate() {
            if let Some(dep_node) = self.nodes.get(dep_name) {
                let is_last_child = i == deps_count - 1;
                self.format_node(
                    dep_node,
                    output,
                    visited,
                    depth + 1,
                    indent,
                    is_last_child,
                );
            }
        }
    }

    /// Get statistics about the dependency graph
    pub fn stats(&self) -> DependencyStats {
        let unique_count = self.nodes.len();
        let total_count = self.edges.len() + self.roots.len();
        let shared_count = if total_count > unique_count {
            total_count - unique_count
        } else {
            0
        };

        // Calculate max depth
        let max_depth = self.nodes.values().map(|n| n.depth).max().unwrap_or(0);

        DependencyStats {
            unique_count,
            total_count,
            shared_count,
            max_depth,
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about a dependency graph
#[derive(Debug, Clone)]
pub struct DependencyStats {
    /// Number of unique dependencies
    pub unique_count: usize,

    /// Total number of dependency references (including duplicates)
    pub total_count: usize,

    /// Number of shared dependencies (referenced by multiple packages)
    pub shared_count: usize,

    /// Maximum depth in the dependency tree
    pub max_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(name: &str, version: &str, deps: Vec<&str>) -> DependencyNode {
        DependencyNode {
            name: name.to_string(),
            version: version.to_string(),
            source: format!("git+https://github.com/test/{}", name),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
            depth: 0,
            config: DependencyConfig {
                name: name.to_string(),
                version: version.to_string(),
                git: Some(format!("https://github.com/test/{}", name)),
                branch: None,
                path: None,
                optional: false,
                features: vec![],
                default_features: Some(true),
                workspace: false,
            },
        }
    }

    #[test]
    fn test_simple_graph() {
        let mut graph = DependencyGraph::new();

        let node_a = create_test_node("a", "1.0.0", vec!["b", "c"]);
        let node_b = create_test_node("b", "1.0.0", vec!["c"]);
        let node_c = create_test_node("c", "1.0.0", vec![]);

        graph.add_root("a".to_string());
        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_c);
        // Edge (from, to) means "from must come before to"
        // Since a depends on b, b must come before a
        graph.add_edge("b", "a");
        graph.add_edge("c", "a");
        graph.add_edge("c", "b");

        let sorted = graph.topological_sort().unwrap();
        // c should come before b, and b before a
        let pos_c = sorted.iter().position(|n| n == "c").unwrap();
        let pos_b = sorted.iter().position(|n| n == "b").unwrap();
        let pos_a = sorted.iter().position(|n| n == "a").unwrap();

        assert!(pos_c < pos_b);
        assert!(pos_b < pos_a);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();

        let node_a = create_test_node("a", "1.0.0", vec!["b"]);
        let node_b = create_test_node("b", "1.0.0", vec!["c"]);
        let node_c = create_test_node("c", "1.0.0", vec!["a"]); // Cycle!

        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_c);
        // Edge (from, to) means "from must come before to"
        // a depends on b → b must come before a
        // b depends on c → c must come before b
        // c depends on a → a must come before c
        // This creates a cycle!
        graph.add_edge("b", "a");
        graph.add_edge("c", "b");
        graph.add_edge("a", "c");

        let cycle = graph.detect_cycles();
        assert!(cycle.is_some());

        let cycle_vec = cycle.unwrap();
        assert!(cycle_vec.contains(&"a".to_string()));
        assert!(cycle_vec.contains(&"b".to_string()));
        assert!(cycle_vec.contains(&"c".to_string()));
    }

    #[test]
    fn test_shared_dependency() {
        let mut graph = DependencyGraph::new();

        let node_a = create_test_node("a", "1.0.0", vec!["c"]);
        let node_b = create_test_node("b", "1.0.0", vec!["c"]);
        let node_c = create_test_node("c", "1.0.0", vec![]);

        graph.add_root("a".to_string());
        graph.add_root("b".to_string());
        graph.add_node(node_a);
        graph.add_node(node_b);
        graph.add_node(node_c);
        // Both a and b depend on c, so c must come before both
        graph.add_edge("c", "a");
        graph.add_edge("c", "b");

        let stats = graph.stats();
        assert_eq!(stats.unique_count, 3);
        assert!(stats.shared_count > 0);
    }
}
