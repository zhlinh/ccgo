//! Tree command - Display dependency tree
//!
//! Usage:
//!   ccgo tree                    # Show full dependency tree
//!   ccgo tree --depth 2          # Limit depth
//!   ccgo tree <package>          # Show specific package's dependencies
//!   ccgo tree --no-dedupe        # Show all occurrences (don't deduplicate)
//!   ccgo tree --format json      # Output as JSON
//!   ccgo tree --format dot       # Output as Graphviz DOT
//!   ccgo tree --invert <pkg>     # Show reverse dependencies
//!   ccgo tree --conflicts        # Highlight version conflicts
//!   ccgo tree --duplicates       # Show only duplicate dependencies

use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{CcgoConfig, DependencyConfig};

/// Output format for tree command
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum OutputFormat {
    /// Text format (default)
    #[default]
    Text,
    /// JSON format
    Json,
    /// Graphviz DOT format
    Dot,
}

/// Display dependency tree
#[derive(Args, Debug)]
pub struct TreeCommand {
    /// Show dependencies of a specific package
    pub package: Option<String>,

    /// Maximum depth to display (default: unlimited)
    #[arg(long, short = 'd')]
    pub depth: Option<usize>,

    /// Don't deduplicate repeated dependencies
    #[arg(long)]
    pub no_dedupe: bool,

    /// Show dependency versions from lock file
    #[arg(long, short = 'l')]
    pub locked: bool,

    /// Output format: text, json, dot
    #[arg(long, short = 'f', value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Show only duplicate dependencies
    #[arg(long)]
    pub duplicates: bool,

    /// Invert tree: show packages that depend on this package
    #[arg(long, short = 'i')]
    pub invert: Option<String>,

    /// Highlight version conflicts
    #[arg(long)]
    pub conflicts: bool,
}

// ============================================================================
// JSON Output Structures
// ============================================================================

/// Root structure for JSON output
#[derive(Serialize, Debug)]
struct TreeJson {
    name: String,
    version: String,
    dependencies: Vec<DepJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conflicts: Option<Vec<ConflictInfo>>,
}

/// Dependency node for JSON output
#[derive(Serialize, Debug, Clone)]
struct DepJson {
    name: String,
    version: String,
    source: DepSourceJson,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    dependencies: Vec<DepJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    duplicate: Option<bool>,
}

/// Dependency source for JSON output
#[derive(Serialize, Debug, Clone)]
#[serde(tag = "type")]
enum DepSourceJson {
    #[serde(rename = "registry")]
    Registry,
    #[serde(rename = "path")]
    Path { path: String },
    #[serde(rename = "git")]
    Git {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        branch: Option<String>,
    },
}

/// Version conflict information
#[derive(Serialize, Debug, Clone)]
struct ConflictInfo {
    package: String,
    versions: Vec<String>,
    locations: Vec<String>,
}

// ============================================================================
// Internal Structures
// ============================================================================

/// Resolved dependency with children (for internal processing)
#[derive(Debug, Clone)]
struct ResolvedDep {
    name: String,
    version: String,
    source: DepSourceInfo,
    children: Vec<ResolvedDep>,
    is_duplicate: bool,
}

/// Dependency source info (internal)
#[derive(Debug, Clone)]
enum DepSourceInfo {
    Registry,
    Path(String),
    Git { url: String, branch: Option<String> },
}

impl From<&DependencyConfig> for DepSourceInfo {
    fn from(dep: &DependencyConfig) -> Self {
        if let Some(ref path) = dep.path {
            DepSourceInfo::Path(path.clone())
        } else if let Some(ref git) = dep.git {
            DepSourceInfo::Git {
                url: git.clone(),
                branch: dep.branch.clone(),
            }
        } else {
            DepSourceInfo::Registry
        }
    }
}

impl From<&DepSourceInfo> for DepSourceJson {
    fn from(source: &DepSourceInfo) -> Self {
        match source {
            DepSourceInfo::Registry => DepSourceJson::Registry,
            DepSourceInfo::Path(path) => DepSourceJson::Path { path: path.clone() },
            DepSourceInfo::Git { url, branch } => DepSourceJson::Git {
                url: url.clone(),
                branch: branch.clone(),
            },
        }
    }
}

/// Lock file information
#[derive(Debug)]
struct LockInfo {
    dependencies: HashMap<String, LockedDep>,
}

#[derive(Debug, Clone)]
struct LockedDep {
    version: String,
    #[allow(dead_code)]
    source: String,
    install_path: String,
}

impl TreeCommand {
    /// Execute the tree command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        let project_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Load CCGO.toml
        let config = CcgoConfig::load().context("Failed to load CCGO.toml")?;

        // Get package info (required for tree command)
        let package = config.require_package()?;

        // Handle invert mode (reverse dependency lookup)
        if let Some(ref target) = self.invert {
            return self.execute_invert(target, &config, &project_dir);
        }

        if config.dependencies.is_empty() {
            if self.format == OutputFormat::Text {
                println!("\n✓ No dependencies defined in CCGO.toml");
            } else if self.format == OutputFormat::Json {
                let tree = TreeJson {
                    name: package.name.clone(),
                    version: package.version.clone(),
                    dependencies: vec![],
                    conflicts: None,
                };
                println!("{}", serde_json::to_string_pretty(&tree)?);
            }
            return Ok(());
        }

        // Load lock file if requested or available
        let lock_info = if self.locked {
            Some(Self::load_lock_file(&project_dir).context("Failed to load CCGO.toml.lock")?)
        } else {
            Self::load_lock_file(&project_dir).ok()
        };

        // Resolve all dependencies into ResolvedDep tree
        let resolved = self.resolve_dependencies(&config.dependencies, &lock_info, &project_dir)?;

        // Handle duplicates-only mode
        if self.duplicates {
            return self.execute_duplicates(&config, &resolved);
        }

        // Output based on format
        match self.format {
            OutputFormat::Json => self.output_json(&package.name, &package.version, &config, &resolved),
            OutputFormat::Dot => self.output_dot(&package.name, &package.version, &config, &resolved),
            OutputFormat::Text => {
                self.output_text(&package.name, &package.version, &config, &resolved, &lock_info, &project_dir)?;

                // Show conflicts if requested
                if self.conflicts {
                    let conflicts = self.detect_conflicts(&resolved);
                    if !conflicts.is_empty() {
                        self.print_conflicts(&conflicts);
                    } else {
                        println!("\n✓ No version conflicts detected");
                    }
                }
                Ok(())
            }
        }
    }

    // ========================================================================
    // Dependency Resolution
    // ========================================================================

    /// Resolve dependencies into a tree structure
    fn resolve_dependencies(
        &self,
        deps: &[DependencyConfig],
        lock_info: &Option<LockInfo>,
        project_dir: &Path,
    ) -> Result<Vec<ResolvedDep>> {
        let mut seen = HashSet::new();
        self.resolve_deps_recursive(deps, lock_info, project_dir, &mut seen, 0)
    }

    fn resolve_deps_recursive(
        &self,
        deps: &[DependencyConfig],
        lock_info: &Option<LockInfo>,
        project_dir: &Path,
        seen: &mut HashSet<String>,
        current_depth: usize,
    ) -> Result<Vec<ResolvedDep>> {
        let mut resolved = Vec::new();

        for dep in deps {
            // Get version from lock file or config
            let version = if let Some(ref lock) = lock_info {
                lock.dependencies
                    .get(&dep.name)
                    .map(|l| l.version.clone())
                    .unwrap_or_else(|| dep.version.clone())
            } else {
                dep.version.clone()
            };

            let dep_key = format!("{}@{}", dep.name, version);
            let is_duplicate = !seen.insert(dep_key);

            // Load children if not at depth limit and not duplicate
            let children = if is_duplicate {
                vec![]
            } else if let Some(max_depth) = self.depth {
                if current_depth >= max_depth {
                    vec![]
                } else {
                    let sub_deps = self.load_sub_dependencies(dep, lock_info, project_dir)?;
                    self.resolve_deps_recursive(
                        &sub_deps,
                        lock_info,
                        project_dir,
                        seen,
                        current_depth + 1,
                    )?
                }
            } else {
                let sub_deps = self.load_sub_dependencies(dep, lock_info, project_dir)?;
                self.resolve_deps_recursive(
                    &sub_deps,
                    lock_info,
                    project_dir,
                    seen,
                    current_depth + 1,
                )?
            };

            resolved.push(ResolvedDep {
                name: dep.name.clone(),
                version,
                source: DepSourceInfo::from(dep),
                children,
                is_duplicate,
            });
        }

        Ok(resolved)
    }

    // ========================================================================
    // Text Output (Original Format)
    // ========================================================================

    fn output_text(
        &self,
        pkg_name: &str,
        pkg_version: &str,
        config: &CcgoConfig,
        _resolved: &[ResolvedDep],
        lock_info: &Option<LockInfo>,
        project_dir: &Path,
    ) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Tree - Dependency Tree Viewer");
        println!("{}", "=".repeat(80));

        println!("\n{} v{}", pkg_name, pkg_version);

        // Filter dependencies if specific package requested
        let deps_to_show: Vec<_> = if let Some(ref pkg) = self.package {
            config
                .dependencies
                .iter()
                .filter(|d| &d.name == pkg)
                .collect()
        } else {
            config.dependencies.iter().collect()
        };

        if deps_to_show.is_empty() {
            if let Some(ref pkg) = self.package {
                bail!("Package '{}' not found in dependencies", pkg);
            }
            println!("\n✓ No dependencies to display");
            return Ok(());
        }

        // Track shown dependencies for deduplication
        let mut shown = if self.no_dedupe {
            None
        } else {
            Some(HashSet::new())
        };

        // Display dependency tree
        for (idx, dep) in deps_to_show.iter().enumerate() {
            let is_last = idx == deps_to_show.len() - 1;
            let prefix = if is_last { "└── " } else { "├── " };
            let continue_prefix = if is_last { "    " } else { "│   " };

            self.print_dependency(
                dep,
                prefix,
                continue_prefix,
                0,
                lock_info,
                &mut shown,
                project_dir,
            )?;
        }

        println!();
        Ok(())
    }

    /// Print a dependency and its children
    fn print_dependency(
        &self,
        dep: &DependencyConfig,
        prefix: &str,
        continue_prefix: &str,
        current_depth: usize,
        lock_info: &Option<LockInfo>,
        shown: &mut Option<HashSet<String>>,
        project_dir: &Path,
    ) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.depth {
            if current_depth >= max_depth {
                return Ok(());
            }
        }

        // Get version from lock file or CCGO.toml
        let version_info = if let Some(ref lock) = lock_info {
            if let Some(locked_dep) = lock.dependencies.get(&dep.name) {
                format!(" v{}", locked_dep.version)
            } else {
                format!(" v{}", dep.version)
            }
        } else {
            format!(" v{}", dep.version)
        };

        // Get source info
        let source_info = self.format_source(dep);

        // Check if already shown (for deduplication)
        let dep_key = format!("{}{}", dep.name, version_info);
        let already_shown = if let Some(ref mut set) = shown {
            !set.insert(dep_key.clone())
        } else {
            false
        };

        // Print this dependency
        if already_shown {
            println!("{}{}{}{}  (*)", prefix, dep.name, version_info, source_info);
            return Ok(());
        } else {
            println!("{}{}{}{}", prefix, dep.name, version_info, source_info);
        }

        // Try to load this dependency's CCGO.toml to find its dependencies
        let sub_deps = self.load_sub_dependencies(dep, lock_info, project_dir)?;

        if !sub_deps.is_empty() {
            for (idx, sub_dep) in sub_deps.iter().enumerate() {
                let is_last = idx == sub_deps.len() - 1;
                let sub_prefix = format!(
                    "{}{}",
                    continue_prefix,
                    if is_last { "└── " } else { "├── " }
                );
                let sub_continue = format!(
                    "{}{}",
                    continue_prefix,
                    if is_last { "    " } else { "│   " }
                );

                self.print_dependency(
                    sub_dep,
                    &sub_prefix,
                    &sub_continue,
                    current_depth + 1,
                    lock_info,
                    shown,
                    project_dir,
                )?;
            }
        }

        Ok(())
    }

    /// Format source information for display
    fn format_source(&self, dep: &DependencyConfig) -> String {
        if let Some(ref path) = dep.path {
            format!("  (path: {})", path)
        } else if let Some(ref git) = dep.git {
            if let Some(ref branch) = dep.branch {
                format!("  (git: {}, branch: {})", git, branch)
            } else {
                format!("  (git: {})", git)
            }
        } else {
            String::new()
        }
    }

    /// Load sub-dependencies from a dependency's CCGO.toml
    fn load_sub_dependencies(
        &self,
        dep: &DependencyConfig,
        lock_info: &Option<LockInfo>,
        project_dir: &Path,
    ) -> Result<Vec<DependencyConfig>> {
        // Determine where to look for the dependency's CCGO.toml
        let dep_path = if let Some(ref path) = dep.path {
            // Local path dependency
            let mut p = PathBuf::from(path);
            if p.is_relative() {
                p = project_dir.join(p);
            }
            p
        } else if let Some(ref lock) = lock_info {
            // Try to get from lock file install path
            if let Some(locked_dep) = lock.dependencies.get(&dep.name) {
                PathBuf::from(&locked_dep.install_path)
            } else {
                // Not installed yet
                return Ok(Vec::new());
            }
        } else {
            // No lock file and not a path dependency
            return Ok(Vec::new());
        };

        // Try to load CCGO.toml from dependency
        let ccgo_toml_path = dep_path.join("CCGO.toml");
        if !ccgo_toml_path.exists() {
            return Ok(Vec::new());
        }

        match CcgoConfig::load_from(&ccgo_toml_path) {
            Ok(config) => Ok(config.dependencies),
            Err(_) => Ok(Vec::new()), // Ignore parse errors for sub-dependencies
        }
    }

    /// Load CCGO.toml.lock file
    fn load_lock_file(project_dir: &Path) -> Result<LockInfo> {
        let lock_path = project_dir.join("CCGO.toml.lock");
        if !lock_path.exists() {
            bail!("CCGO.toml.lock not found. Run 'ccgo install' first.");
        }

        let content = fs::read_to_string(&lock_path).context("Failed to read CCGO.toml.lock")?;

        // Parse the lock file
        let mut dependencies = HashMap::new();

        // Simple TOML parsing - look for [dependencies.xxx] sections
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Look for [dependencies.xxx]
            if line.starts_with("[dependencies.") && line.ends_with(']') {
                let dep_name = line
                    .trim_start_matches("[dependencies.")
                    .trim_end_matches(']')
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_string();

                if dep_name.is_empty() || dep_name == "git" {
                    i += 1;
                    continue;
                }

                // Parse fields for this dependency
                let mut version = String::new();
                let mut source = String::new();
                let mut install_path = String::new();

                i += 1;
                while i < lines.len() {
                    let field_line = lines[i].trim();

                    if field_line.is_empty() || field_line.starts_with('#') {
                        i += 1;
                        continue;
                    }

                    if field_line.starts_with('[') {
                        // Next section
                        break;
                    }

                    if let Some((key, value)) = field_line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim().trim_matches('"');

                        match key {
                            "version" => version = value.to_string(),
                            "source" => source = value.to_string(),
                            "install_path" => install_path = value.to_string(),
                            _ => {}
                        }
                    }

                    i += 1;
                }

                if !version.is_empty() && !install_path.is_empty() {
                    dependencies.insert(
                        dep_name,
                        LockedDep {
                            version,
                            source,
                            install_path,
                        },
                    );
                }
            } else {
                i += 1;
            }
        }

        Ok(LockInfo { dependencies })
    }

    // ========================================================================
    // JSON Output
    // ========================================================================

    fn output_json(&self, pkg_name: &str, pkg_version: &str, _config: &CcgoConfig, resolved: &[ResolvedDep]) -> Result<()> {
        let conflicts = if self.conflicts {
            let c = self.detect_conflicts(resolved);
            if c.is_empty() {
                None
            } else {
                Some(c)
            }
        } else {
            None
        };

        let tree = TreeJson {
            name: pkg_name.to_string(),
            version: pkg_version.to_string(),
            dependencies: resolved.iter().map(|d| self.resolved_to_json(d)).collect(),
            conflicts,
        };

        println!("{}", serde_json::to_string_pretty(&tree)?);
        Ok(())
    }

    fn resolved_to_json(&self, dep: &ResolvedDep) -> DepJson {
        DepJson {
            name: dep.name.clone(),
            version: dep.version.clone(),
            source: DepSourceJson::from(&dep.source),
            dependencies: dep.children.iter().map(|c| self.resolved_to_json(c)).collect(),
            duplicate: if dep.is_duplicate { Some(true) } else { None },
        }
    }

    // ========================================================================
    // DOT (Graphviz) Output
    // ========================================================================

    fn output_dot(&self, pkg_name: &str, pkg_version: &str, _config: &CcgoConfig, resolved: &[ResolvedDep]) -> Result<()> {
        println!("digraph dependencies {{");
        println!("    rankdir=TB;");
        println!("    node [shape=box, style=filled, fillcolor=lightblue, fontname=\"Helvetica\"];");
        println!("    edge [fontname=\"Helvetica\", fontsize=10];");
        println!();

        // Root node
        let root_id = self.node_id(pkg_name, pkg_version);
        let root_label = format!("{}\\nv{}", pkg_name, pkg_version);
        println!(
            "    \"{}\" [label=\"{}\", fillcolor=lightgreen];",
            root_id, root_label
        );

        // Collect all edges and detect conflicts
        let mut edges: Vec<(String, String)> = Vec::new();
        let mut nodes: HashMap<String, (String, bool)> = HashMap::new(); // id -> (label, is_conflict)

        // Detect version conflicts for highlighting
        let conflicts = self.detect_conflicts(resolved);
        let conflict_packages: HashSet<String> = conflicts.iter().map(|c| c.package.clone()).collect();

        self.collect_dot_nodes(&root_id, resolved, &mut edges, &mut nodes, &conflict_packages);

        // Print nodes
        for (id, (label, is_conflict)) in &nodes {
            let color = if *is_conflict { "salmon" } else { "lightblue" };
            println!("    \"{}\" [label=\"{}\", fillcolor={}];", id, label, color);
        }

        println!();

        // Print edges
        for (from, to) in &edges {
            println!("    \"{}\" -> \"{}\";", from, to);
        }

        println!("}}");
        Ok(())
    }

    fn node_id(&self, name: &str, version: &str) -> String {
        format!("{}_{}", name.replace('-', "_"), version.replace('.', "_"))
    }

    fn collect_dot_nodes(
        &self,
        parent_id: &str,
        deps: &[ResolvedDep],
        edges: &mut Vec<(String, String)>,
        nodes: &mut HashMap<String, (String, bool)>,
        conflict_packages: &HashSet<String>,
    ) {
        for dep in deps {
            let node_id = self.node_id(&dep.name, &dep.version);
            let label = format!("{}\\nv{}", dep.name, dep.version);
            let is_conflict = conflict_packages.contains(&dep.name);

            // Add edge
            edges.push((parent_id.to_string(), node_id.clone()));

            // Add node if not already present
            nodes.entry(node_id.clone()).or_insert((label, is_conflict));

            // Recurse into children
            if !dep.children.is_empty() && !dep.is_duplicate {
                self.collect_dot_nodes(&node_id, &dep.children, edges, nodes, conflict_packages);
            }
        }
    }

    // ========================================================================
    // Invert (Reverse Dependencies)
    // ========================================================================

    fn execute_invert(&self, target: &str, config: &CcgoConfig, project_dir: &Path) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Tree - Reverse Dependencies for '{}'", target);
        println!("{}", "=".repeat(80));

        // Get package info (required)
        let package = config.require_package()?;

        // Build reverse dependency map
        let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();
        let lock_info = Self::load_lock_file(project_dir).ok();

        self.build_reverse_map(
            &package.name,
            &config.dependencies,
            &mut reverse_deps,
            &lock_info,
            project_dir,
        )?;

        // Find who depends on target
        if let Some(dependents) = reverse_deps.get(target) {
            println!("\n{}", target);
            let unique_dependents: Vec<_> = dependents.iter().collect::<HashSet<_>>().into_iter().collect();
            for (idx, dep) in unique_dependents.iter().enumerate() {
                let is_last = idx == unique_dependents.len() - 1;
                let prefix = if is_last { "└── " } else { "├── " };
                println!("{}{} (depends on {})", prefix, dep, target);
            }
            println!("\nTotal: {} package(s) depend on '{}'", unique_dependents.len(), target);
        } else {
            println!("\n✓ No packages depend on '{}'", target);
            println!("\nNote: '{}' may not exist in the dependency tree, or it's a leaf dependency.", target);
        }

        Ok(())
    }

    fn build_reverse_map(
        &self,
        parent: &str,
        deps: &[DependencyConfig],
        reverse_deps: &mut HashMap<String, Vec<String>>,
        lock_info: &Option<LockInfo>,
        project_dir: &Path,
    ) -> Result<()> {
        for dep in deps {
            // parent depends on dep.name
            reverse_deps
                .entry(dep.name.clone())
                .or_default()
                .push(parent.to_string());

            // Recursively process sub-dependencies
            let sub_deps = self.load_sub_dependencies(dep, lock_info, project_dir)?;
            if !sub_deps.is_empty() {
                self.build_reverse_map(&dep.name, &sub_deps, reverse_deps, lock_info, project_dir)?;
            }
        }
        Ok(())
    }

    // ========================================================================
    // Version Conflict Detection
    // ========================================================================

    fn detect_conflicts(&self, resolved: &[ResolvedDep]) -> Vec<ConflictInfo> {
        let mut version_map: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
        self.collect_versions(resolved, "", &mut version_map);

        version_map
            .into_iter()
            .filter(|(_, versions)| versions.len() > 1)
            .map(|(pkg, versions)| {
                let version_list: Vec<String> = versions.keys().cloned().collect();
                let locations: Vec<String> = versions.values().flatten().cloned().collect();
                ConflictInfo {
                    package: pkg,
                    versions: version_list,
                    locations,
                }
            })
            .collect()
    }

    fn collect_versions(
        &self,
        deps: &[ResolvedDep],
        path: &str,
        map: &mut HashMap<String, HashMap<String, Vec<String>>>,
    ) {
        for dep in deps {
            let current_path = if path.is_empty() {
                dep.name.clone()
            } else {
                format!("{} -> {}", path, dep.name)
            };

            map.entry(dep.name.clone())
                .or_default()
                .entry(dep.version.clone())
                .or_default()
                .push(current_path.clone());

            if !dep.children.is_empty() {
                self.collect_versions(&dep.children, &current_path, map);
            }
        }
    }

    fn print_conflicts(&self, conflicts: &[ConflictInfo]) {
        println!("\n{}", "=".repeat(80));
        println!("⚠️  Version Conflicts Detected");
        println!("{}", "=".repeat(80));

        for conflict in conflicts {
            println!(
                "\n  📦 {} has {} different versions:",
                conflict.package,
                conflict.versions.len()
            );
            for version in &conflict.versions {
                println!("      • v{}", version);
            }
            println!("  Found in:");
            for loc in &conflict.locations {
                println!("      └── {}", loc);
            }
        }

        println!("\n  💡 Tip: Consider aligning dependency versions to avoid conflicts.");
    }

    // ========================================================================
    // Duplicates Only Mode
    // ========================================================================

    fn execute_duplicates(&self, config: &CcgoConfig, resolved: &[ResolvedDep]) -> Result<()> {
        // Get package info (required)
        let package = config.require_package()?;

        let mut count_map: HashMap<String, usize> = HashMap::new();
        self.count_occurrences(resolved, &mut count_map);

        let mut duplicates: Vec<_> = count_map
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .collect();

        duplicates.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending

        if self.format == OutputFormat::Json {
            #[derive(Serialize)]
            struct DuplicatesJson {
                project: String,
                version: String,
                duplicates: Vec<DuplicateEntry>,
            }
            #[derive(Serialize)]
            struct DuplicateEntry {
                name: String,
                count: usize,
            }

            let json = DuplicatesJson {
                project: package.name.clone(),
                version: package.version.clone(),
                duplicates: duplicates
                    .iter()
                    .map(|(name, count)| DuplicateEntry {
                        name: name.clone(),
                        count: *count,
                    })
                    .collect(),
            };
            println!("{}", serde_json::to_string_pretty(&json)?);
            return Ok(());
        }

        println!("{}", "=".repeat(80));
        println!("CCGO Tree - Duplicate Dependencies");
        println!("{}", "=".repeat(80));

        if duplicates.is_empty() {
            println!("\n✓ No duplicate dependencies found");
            return Ok(());
        }

        println!("\n📦 Duplicate Dependencies:");
        println!("{}", "-".repeat(40));

        for (name, count) in &duplicates {
            println!("  {} (appears {} times)", name, count);
        }

        println!("\nTotal: {} duplicate package(s)", duplicates.len());

        Ok(())
    }

    fn count_occurrences(&self, deps: &[ResolvedDep], map: &mut HashMap<String, usize>) {
        for dep in deps {
            *map.entry(dep.name.clone()).or_insert(0) += 1;

            if !dep.children.is_empty() {
                self.count_occurrences(&dep.children, map);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_command() -> TreeCommand {
        TreeCommand {
            package: None,
            depth: None,
            no_dedupe: false,
            locked: false,
            format: OutputFormat::Text,
            duplicates: false,
            invert: None,
            conflicts: false,
        }
    }

    #[test]
    fn test_format_source_path() {
        let cmd = create_test_command();

        let dep = DependencyConfig {
            name: "mylib".to_string(),
            version: "1.0.0".to_string(),
            path: Some("../mylib".to_string()),
            git: None,
            branch: None,
            optional: false,
            features: vec![],
            default_features: None,
            workspace: false,
        };

        assert_eq!(cmd.format_source(&dep), "  (path: ../mylib)");
    }

    #[test]
    fn test_format_source_git() {
        let cmd = create_test_command();

        let dep = DependencyConfig {
            name: "mylib".to_string(),
            version: "1.0.0".to_string(),
            path: None,
            git: Some("https://github.com/user/repo.git".to_string()),
            branch: Some("main".to_string()),
            optional: false,
            features: vec![],
            default_features: None,
            workspace: false,
        };

        assert_eq!(
            cmd.format_source(&dep),
            "  (git: https://github.com/user/repo.git, branch: main)"
        );
    }

    #[test]
    fn test_dep_source_info_from_path() {
        let dep = DependencyConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: Some("../test".to_string()),
            git: None,
            branch: None,
            optional: false,
            features: vec![],
            default_features: None,
            workspace: false,
        };

        let source = DepSourceInfo::from(&dep);
        match source {
            DepSourceInfo::Path(p) => assert_eq!(p, "../test"),
            _ => panic!("Expected Path source"),
        }
    }

    #[test]
    fn test_dep_source_info_from_git() {
        let dep = DependencyConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: None,
            git: Some("https://github.com/test/repo.git".to_string()),
            branch: Some("main".to_string()),
            optional: false,
            features: vec![],
            default_features: None,
            workspace: false,
        };

        let source = DepSourceInfo::from(&dep);
        match source {
            DepSourceInfo::Git { url, branch } => {
                assert_eq!(url, "https://github.com/test/repo.git");
                assert_eq!(branch, Some("main".to_string()));
            }
            _ => panic!("Expected Git source"),
        }
    }

    #[test]
    fn test_dep_source_info_from_registry() {
        let dep = DependencyConfig {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            path: None,
            git: None,
            branch: None,
            optional: false,
            features: vec![],
            default_features: None,
            workspace: false,
        };

        let source = DepSourceInfo::from(&dep);
        match source {
            DepSourceInfo::Registry => {}
            _ => panic!("Expected Registry source"),
        }
    }

    #[test]
    fn test_node_id_generation() {
        let cmd = create_test_command();
        assert_eq!(cmd.node_id("my-lib", "1.2.3"), "my_lib_1_2_3");
        assert_eq!(cmd.node_id("foo_bar", "0.1.0"), "foo_bar_0_1_0");
    }

    #[test]
    fn test_detect_conflicts_no_conflict() {
        let cmd = create_test_command();
        let deps = vec![
            ResolvedDep {
                name: "libA".to_string(),
                version: "1.0.0".to_string(),
                source: DepSourceInfo::Registry,
                children: vec![],
                is_duplicate: false,
            },
            ResolvedDep {
                name: "libB".to_string(),
                version: "2.0.0".to_string(),
                source: DepSourceInfo::Registry,
                children: vec![],
                is_duplicate: false,
            },
        ];

        let conflicts = cmd.detect_conflicts(&deps);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_with_conflict() {
        let cmd = create_test_command();
        let deps = vec![
            ResolvedDep {
                name: "libA".to_string(),
                version: "1.0.0".to_string(),
                source: DepSourceInfo::Registry,
                children: vec![ResolvedDep {
                    name: "common".to_string(),
                    version: "1.0.0".to_string(),
                    source: DepSourceInfo::Registry,
                    children: vec![],
                    is_duplicate: false,
                }],
                is_duplicate: false,
            },
            ResolvedDep {
                name: "libB".to_string(),
                version: "2.0.0".to_string(),
                source: DepSourceInfo::Registry,
                children: vec![ResolvedDep {
                    name: "common".to_string(),
                    version: "2.0.0".to_string(),
                    source: DepSourceInfo::Registry,
                    children: vec![],
                    is_duplicate: false,
                }],
                is_duplicate: false,
            },
        ];

        let conflicts = cmd.detect_conflicts(&deps);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].package, "common");
        assert_eq!(conflicts[0].versions.len(), 2);
    }

    #[test]
    fn test_count_occurrences() {
        let cmd = create_test_command();
        let deps = vec![
            ResolvedDep {
                name: "libA".to_string(),
                version: "1.0.0".to_string(),
                source: DepSourceInfo::Registry,
                children: vec![ResolvedDep {
                    name: "common".to_string(),
                    version: "1.0.0".to_string(),
                    source: DepSourceInfo::Registry,
                    children: vec![],
                    is_duplicate: false,
                }],
                is_duplicate: false,
            },
            ResolvedDep {
                name: "common".to_string(),
                version: "1.0.0".to_string(),
                source: DepSourceInfo::Registry,
                children: vec![],
                is_duplicate: true,
            },
        ];

        let mut count_map = HashMap::new();
        cmd.count_occurrences(&deps, &mut count_map);

        assert_eq!(count_map.get("libA"), Some(&1));
        assert_eq!(count_map.get("common"), Some(&2));
    }

    #[test]
    fn test_resolved_to_json() {
        let cmd = create_test_command();
        let dep = ResolvedDep {
            name: "test-lib".to_string(),
            version: "1.0.0".to_string(),
            source: DepSourceInfo::Git {
                url: "https://github.com/test/lib.git".to_string(),
                branch: Some("main".to_string()),
            },
            children: vec![],
            is_duplicate: false,
        };

        let json = cmd.resolved_to_json(&dep);
        assert_eq!(json.name, "test-lib");
        assert_eq!(json.version, "1.0.0");
        assert!(json.duplicate.is_none());
    }

    #[test]
    fn test_resolved_to_json_duplicate() {
        let cmd = create_test_command();
        let dep = ResolvedDep {
            name: "dup-lib".to_string(),
            version: "1.0.0".to_string(),
            source: DepSourceInfo::Registry,
            children: vec![],
            is_duplicate: true,
        };

        let json = cmd.resolved_to_json(&dep);
        assert_eq!(json.duplicate, Some(true));
    }

    #[test]
    fn test_output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Text);
    }
}
