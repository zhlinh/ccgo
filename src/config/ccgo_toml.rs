//! CCGO.toml configuration parsing
//!
//! These structs are parsed from TOML and will be used for native Rust implementation.
//!
//! # Workspace Support
//!
//! CCGO supports workspaces for managing multiple related packages. A workspace is defined
//! by a root CCGO.toml that contains a `[workspace]` section.
//!
//! ## Workspace Configuration Example
//!
//! ```toml
//! [workspace]
//! members = ["core", "utils", "examples/*"]
//! resolver = "2"
//!
//! [workspace.dependencies]
//! fmt = { version = "^10.0", git = "https://github.com/fmtlib/fmt.git" }
//! spdlog = { version = "^1.12" }
//! ```
//!
//! Member packages can inherit workspace dependencies:
//!
//! ```toml
//! [package]
//! name = "my-core"
//! version = "1.0.0"
//!
//! [[dependencies]]
//! name = "fmt"
//! workspace = true
//! ```

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Root configuration from CCGO.toml
///
/// This can be either a package configuration (with [package] section)
/// or a workspace configuration (with [workspace] section), or both.
#[derive(Debug, Clone, Deserialize)]
pub struct CcgoConfig {
    /// Package metadata (supports both [package] and [project] sections)
    /// Optional when this is a workspace-only configuration
    #[serde(alias = "project")]
    pub package: Option<PackageConfig>,

    /// Workspace configuration
    pub workspace: Option<WorkspaceConfig>,

    /// Project dependencies
    #[serde(default)]
    pub dependencies: Vec<DependencyConfig>,

    /// Features configuration
    #[serde(default)]
    pub features: FeaturesConfig,

    /// Build configuration
    pub build: Option<BuildConfig>,

    /// Platform-specific configurations
    pub platforms: Option<PlatformConfigs>,

    /// Binary targets
    #[serde(default, rename = "bin")]
    pub bins: Vec<BinConfig>,

    /// Example programs
    #[serde(default, rename = "example")]
    pub examples: Vec<ExampleConfig>,

    /// Dependency patches configuration
    /// Allows overriding dependency sources for specific crates
    #[serde(default)]
    pub patch: PatchConfig,
}

/// Patch configuration from [patch] section
///
/// Allows overriding dependency sources for specific packages.
/// Similar to Cargo's [patch] feature for dependency override.
///
/// # Example
///
/// ```toml
/// [patch.crates-io]
/// fmt = { git = "https://github.com/myorg/fmt.git", branch = "custom-fix" }
///
/// [patch."https://github.com/spdlog/spdlog"]
/// spdlog = { path = "../spdlog-local" }
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PatchConfig {
    /// Patches for registry dependencies (future: when we have package registry)
    #[serde(default, rename = "crates-io")]
    pub crates_io: HashMap<String, PatchDependency>,

    /// Patches for git repositories (keyed by repository URL)
    #[serde(flatten)]
    pub sources: HashMap<String, HashMap<String, PatchDependency>>,
}

/// A patched dependency specification
#[derive(Debug, Clone, Deserialize)]
pub struct PatchDependency {
    /// Git repository URL (alternative source)
    pub git: Option<String>,

    /// Git branch name
    pub branch: Option<String>,

    /// Git tag
    pub tag: Option<String>,

    /// Git revision (commit hash)
    pub rev: Option<String>,

    /// Local path (alternative source)
    pub path: Option<String>,

    /// Version requirement (for verification)
    #[serde(default)]
    pub version: String,
}

impl PatchConfig {
    /// Find a patch for a specific dependency
    ///
    /// Returns the patch specification if a patch is defined for this dependency,
    /// considering both registry patches and source-specific patches.
    pub fn find_patch(&self, dep_name: &str, dep_source: Option<&str>) -> Option<&PatchDependency> {
        // First check source-specific patches if a source is provided
        if let Some(source) = dep_source {
            // Check if we have patches for this specific source
            if let Some(source_patches) = self.sources.get(source) {
                if let Some(patch) = source_patches.get(dep_name) {
                    return Some(patch);
                }
            }
        }

        // Fall back to registry patches (crates-io)
        self.crates_io.get(dep_name)
    }

    /// Check if any patches are defined
    pub fn has_patches(&self) -> bool {
        !self.crates_io.is_empty() || !self.sources.is_empty()
    }

    /// Get all patched dependency names
    pub fn patched_dependencies(&self) -> Vec<&str> {
        let mut deps = Vec::new();

        // Collect from crates-io
        deps.extend(self.crates_io.keys().map(|s| s.as_str()));

        // Collect from source-specific patches
        for source_patches in self.sources.values() {
            deps.extend(source_patches.keys().map(|s| s.as_str()));
        }

        deps.sort();
        deps.dedup();
        deps
    }
}

/// Workspace configuration from [workspace] section
///
/// Workspaces allow managing multiple related packages together with shared
/// dependencies and unified builds.
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace member paths (relative to workspace root)
    ///
    /// Supports glob patterns like "examples/*" or "crates/**"
    #[serde(default)]
    pub members: Vec<String>,

    /// Paths to exclude from workspace membership
    ///
    /// Useful when using glob patterns in members
    #[serde(default)]
    pub exclude: Vec<String>,

    /// Dependency resolver version
    ///
    /// - "1": Legacy resolver (default)
    /// - "2": New resolver with better feature unification
    #[serde(default = "default_resolver")]
    pub resolver: String,

    /// Shared dependencies that workspace members can inherit
    #[serde(default)]
    pub dependencies: Vec<WorkspaceDependency>,

    /// Default members for workspace commands
    ///
    /// If not specified, all members are used
    #[serde(default)]
    pub default_members: Vec<String>,
}

fn default_resolver() -> String {
    "1".to_string()
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            members: Vec::new(),
            exclude: Vec::new(),
            resolver: default_resolver(),
            dependencies: Vec::new(),
            default_members: Vec::new(),
        }
    }
}

/// Workspace-level dependency definition
///
/// These dependencies can be inherited by workspace members using `workspace = true`
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceDependency {
    /// Dependency name
    pub name: String,

    /// Version requirement
    pub version: String,

    /// Git repository URL
    pub git: Option<String>,

    /// Git branch name
    pub branch: Option<String>,

    /// Git tag
    pub tag: Option<String>,

    /// Git revision (commit hash)
    pub rev: Option<String>,

    /// Local path (for development)
    pub path: Option<String>,

    /// Features to enable for this dependency
    #[serde(default)]
    pub features: Vec<String>,

    /// Whether to disable default features for this dependency
    #[serde(default)]
    pub default_features: Option<bool>,
}

impl WorkspaceDependency {
    /// Convert to a regular DependencyConfig
    pub fn to_dependency_config(&self) -> DependencyConfig {
        DependencyConfig {
            name: self.name.clone(),
            version: self.version.clone(),
            git: self.git.clone(),
            branch: self.branch.clone(),
            path: self.path.clone(),
            optional: false,
            features: self.features.clone(),
            default_features: self.default_features,
            workspace: false,
        }
    }
}

/// Package metadata from [package] section
#[derive(Debug, Clone, Deserialize)]
pub struct PackageConfig {
    /// Project name (must be valid C++ identifier)
    pub name: String,

    /// Semver version string
    pub version: String,

    /// Project description
    pub description: Option<String>,

    /// Author list
    pub authors: Option<Vec<String>>,

    /// SPDX license identifier
    pub license: Option<String>,

    /// Git repository URL
    pub repository: Option<String>,
}

/// Binary target configuration from [[bin]] section
///
/// Defines an executable binary target that can be built and run.
///
/// # Example
///
/// ```toml
/// [[bin]]
/// name = "my-cli"
/// path = "src/bin/cli.cpp"
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct BinConfig {
    /// Binary target name (used for executable name)
    pub name: String,

    /// Path to the main source file (relative to project root)
    pub path: String,
}

/// Example configuration from [[example]] section
///
/// Defines an example program that demonstrates library usage.
///
/// # Example
///
/// ```toml
/// [[example]]
/// name = "basic-usage"
/// path = "examples/basic.cpp"  # Optional, defaults to examples/{name}.cpp
/// ```
#[derive(Debug, Clone, Deserialize)]
pub struct ExampleConfig {
    /// Example name
    pub name: String,

    /// Path to the example source file (optional)
    /// Defaults to examples/{name}.cpp or examples/{name}/main.cpp
    pub path: Option<String>,
}

/// Dependency configuration from [[dependencies]] array
#[derive(Debug, Clone, Deserialize)]
pub struct DependencyConfig {
    /// Dependency name
    pub name: String,

    /// Version requirement (supports semver ranges like ^1.0, ~1.2.3, >=1.0,<2.0)
    /// Can be empty if `workspace = true` (inherited from workspace)
    #[serde(default)]
    pub version: String,

    /// Git repository URL
    pub git: Option<String>,

    /// Git branch name
    pub branch: Option<String>,

    /// Local path (for development)
    pub path: Option<String>,

    /// Whether this dependency is optional (only included when a feature enables it)
    #[serde(default)]
    pub optional: bool,

    /// Features to enable for this dependency
    #[serde(default)]
    pub features: Vec<String>,

    /// Whether to disable default features for this dependency
    #[serde(default)]
    pub default_features: Option<bool>,

    /// Whether to inherit this dependency from workspace
    ///
    /// When true, the dependency configuration is inherited from
    /// [workspace.dependencies] in the workspace root CCGO.toml.
    /// Additional features can be specified that will be merged
    /// with the workspace dependency's features.
    #[serde(default)]
    pub workspace: bool,
}

impl DependencyConfig {
    /// Merge with a workspace dependency, inheriting missing fields
    pub fn merge_with_workspace(&mut self, ws_dep: &WorkspaceDependency) {
        // Only merge if workspace inheritance is enabled
        if !self.workspace {
            return;
        }

        // Inherit version if not specified
        if self.version.is_empty() {
            self.version = ws_dep.version.clone();
        }

        // Inherit git if not specified
        if self.git.is_none() {
            self.git = ws_dep.git.clone();
        }

        // Inherit branch if not specified
        if self.branch.is_none() {
            self.branch = ws_dep.branch.clone();
        }

        // Inherit path if not specified
        if self.path.is_none() {
            self.path = ws_dep.path.clone();
        }

        // Merge features (local features are added to workspace features)
        let mut merged_features = ws_dep.features.clone();
        for feat in &self.features {
            if !merged_features.contains(feat) {
                merged_features.push(feat.clone());
            }
        }
        self.features = merged_features;

        // Use workspace default_features if not explicitly set
        if self.default_features.is_none() {
            self.default_features = ws_dep.default_features;
        }
    }
}

/// Features configuration from [features] section
///
/// Features allow conditional compilation and optional dependencies.
/// Similar to Cargo's features system.
///
/// # Example
///
/// ```toml
/// [features]
/// default = ["std"]
/// std = []
/// networking = ["http-client"]
/// advanced = ["networking", "async"]
/// full = ["networking", "advanced", "logging"]
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FeaturesConfig {
    /// Default features enabled when none are specified
    #[serde(default)]
    pub default: Vec<String>,

    /// Feature definitions - maps feature name to its dependencies
    /// The dependencies can be:
    /// - Other feature names (e.g., "networking")
    /// - Optional dependency names (e.g., "http-client")
    /// - Dependency feature syntax (e.g., "dep-name/feature-name")
    #[serde(flatten)]
    pub features: HashMap<String, Vec<String>>,
}

impl FeaturesConfig {
    /// Check if a feature is defined
    pub fn has_feature(&self, name: &str) -> bool {
        name == "default" || self.features.contains_key(name)
    }

    /// Get all feature names (excluding "default")
    pub fn feature_names(&self) -> Vec<&str> {
        self.features.keys().map(|s| s.as_str()).collect()
    }

    /// Resolve a feature and all its transitive dependencies
    pub fn resolve_feature(&self, name: &str, resolved: &mut HashSet<String>) -> Result<()> {
        // Avoid infinite loops from circular dependencies
        if resolved.contains(name) {
            return Ok(());
        }

        if name == "default" {
            // Resolve all default features
            for default_feature in &self.default {
                self.resolve_feature(default_feature, resolved)?;
            }
            return Ok(());
        }

        // Add this feature to resolved set
        resolved.insert(name.to_string());

        // Get dependencies of this feature
        if let Some(deps) = self.features.get(name) {
            for dep in deps {
                // Check if this is a dependency/feature syntax (e.g., "dep-name/feature")
                if dep.contains('/') {
                    // This is a dependency feature, add as-is for later processing
                    resolved.insert(dep.clone());
                } else if self.has_feature(dep) {
                    // This is another feature, resolve recursively
                    self.resolve_feature(dep, resolved)?;
                } else {
                    // This is likely an optional dependency name
                    resolved.insert(dep.clone());
                }
            }
        }

        Ok(())
    }

    /// Resolve multiple features and return the full set of enabled features
    pub fn resolve_features(&self, requested: &[String], use_defaults: bool) -> Result<HashSet<String>> {
        let mut resolved = HashSet::new();

        // Include default features if requested
        if use_defaults {
            self.resolve_feature("default", &mut resolved)?;
        }

        // Resolve each requested feature
        for feature in requested {
            if !self.has_feature(feature) && !feature.contains('/') {
                bail!("Unknown feature: '{}'. Available features: {:?}",
                    feature, self.feature_names());
            }
            self.resolve_feature(feature, &mut resolved)?;
        }

        Ok(resolved)
    }

    /// Get optional dependencies enabled by a set of resolved features
    pub fn get_enabled_optional_deps<'a>(
        &self,
        resolved_features: &HashSet<String>,
        dependencies: &'a [DependencyConfig],
    ) -> Vec<&'a DependencyConfig> {
        dependencies
            .iter()
            .filter(|dep| {
                if !dep.optional {
                    return true; // Non-optional deps are always included
                }
                // Check if this optional dep is enabled by any feature
                resolved_features.contains(&dep.name)
            })
            .collect()
    }
}

impl DependencyConfig {
    /// Validate the dependency configuration
    pub fn validate(&self) -> Result<()> {
        // Validate version requirement syntax
        crate::version::VersionReq::parse(&self.version)
            .with_context(|| format!("Invalid version requirement '{}' for dependency '{}'", self.version, self.name))?;

        Ok(())
    }
}

/// Build configuration from [build] section
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BuildConfig {
    /// Enable parallel builds
    #[serde(default)]
    pub parallel: bool,

    /// Number of parallel jobs
    pub jobs: Option<usize>,

    /// Symbol visibility (false = hidden, true = default)
    #[serde(default)]
    pub symbol_visibility: bool,

    /// Submodule internal dependencies for shared library linking
    /// Format: { "api" = ["base"], "feature" = ["base", "core"] }
    /// This maps to CCGO_CONFIG_DEPS_MAP CMake variable
    #[serde(default)]
    pub submodule_deps: std::collections::HashMap<String, Vec<String>>,
}

/// Platform-specific configurations
#[derive(Debug, Clone, Deserialize)]
pub struct PlatformConfigs {
    /// Android configuration
    pub android: Option<AndroidConfig>,

    /// iOS configuration
    pub ios: Option<IosConfig>,

    /// macOS configuration
    pub macos: Option<MacosConfig>,

    /// Windows configuration
    pub windows: Option<WindowsConfig>,

    /// Linux configuration
    pub linux: Option<LinuxConfig>,

    /// OpenHarmony configuration
    pub ohos: Option<OhosConfig>,
}

/// Android platform configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AndroidConfig {
    /// Minimum SDK version
    pub min_sdk: Option<u32>,

    /// Target architectures
    pub architectures: Option<Vec<String>>,
}

/// iOS platform configuration
#[derive(Debug, Clone, Deserialize)]
pub struct IosConfig {
    /// Minimum iOS version
    pub min_version: Option<String>,
}

/// macOS platform configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MacosConfig {
    /// Minimum macOS version
    pub min_version: Option<String>,
}

/// Windows platform configuration
#[derive(Debug, Clone, Deserialize)]
pub struct WindowsConfig {
    /// Toolchain (msvc, mingw, auto)
    pub toolchain: Option<String>,
}

/// Linux platform configuration
#[derive(Debug, Clone, Deserialize)]
pub struct LinuxConfig {
    /// Target architectures
    pub architectures: Option<Vec<String>>,
}

/// OpenHarmony platform configuration
#[derive(Debug, Clone, Deserialize)]
pub struct OhosConfig {
    /// Minimum API version
    pub min_api: Option<u32>,

    /// Target architectures
    pub architectures: Option<Vec<String>>,
}

impl CcgoConfig {
    /// Load configuration from CCGO.toml in current directory
    pub fn load() -> Result<Self> {
        Self::load_from_path("CCGO.toml")
    }

    /// Load configuration from a specific path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read configuration from {}", path.display()))?;

        Self::parse(&content)
    }

    /// Load configuration from a specific file path (alias for compatibility)
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::load_from_path(path)
    }

    /// Parse configuration from TOML string
    pub fn parse(content: &str) -> Result<Self> {
        let config: Self = toml::from_str(content).context("Failed to parse CCGO.toml")?;

        // Validate: must have either package or workspace
        if config.package.is_none() && config.workspace.is_none() {
            bail!("CCGO.toml must contain either [package] or [workspace] section");
        }

        // Validate dependencies (only non-workspace dependencies need version validation)
        for dep in &config.dependencies {
            if !dep.workspace {
                dep.validate().with_context(|| format!("Invalid dependency: {}", dep.name))?;
            }
        }

        Ok(config)
    }

    /// Find CCGO.toml by searching up from current directory
    pub fn find_config() -> Result<PathBuf> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;

        let mut dir = current_dir.as_path();
        loop {
            let config_path = dir.join("CCGO.toml");
            if config_path.exists() {
                return Ok(config_path);
            }

            match dir.parent() {
                Some(parent) => dir = parent,
                None => {
                    anyhow::bail!(
                        "Could not find CCGO.toml in current directory or any parent directory"
                    )
                }
            }
        }
    }

    /// Check if this is a workspace configuration
    pub fn is_workspace(&self) -> bool {
        self.workspace.is_some()
    }

    /// Check if this is a package configuration
    pub fn is_package(&self) -> bool {
        self.package.is_some()
    }

    /// Get package configuration, returning an error if not present
    pub fn require_package(&self) -> Result<&PackageConfig> {
        self.package.as_ref().context(
            "This operation requires a [package] section in CCGO.toml.\n\
             Workspace-only configurations cannot be used for this operation."
        )
    }

    /// Get workspace configuration, returning an error if not present
    pub fn require_workspace(&self) -> Result<&WorkspaceConfig> {
        self.workspace.as_ref().context(
            "This operation requires a [workspace] section in CCGO.toml."
        )
    }

    /// Find workspace root by searching up from the given directory
    ///
    /// Returns (workspace_root_path, workspace_config)
    pub fn find_workspace_root(start_dir: &Path) -> Result<Option<(PathBuf, Self)>> {
        let mut dir = start_dir;

        loop {
            let config_path = dir.join("CCGO.toml");
            if config_path.exists() {
                let config = Self::load_from_path(&config_path)?;
                if config.is_workspace() {
                    return Ok(Some((dir.to_path_buf(), config)));
                }
            }

            match dir.parent() {
                Some(parent) => dir = parent,
                None => return Ok(None),
            }
        }
    }

    /// Get workspace member paths
    ///
    /// Resolves glob patterns and returns absolute paths to member directories
    pub fn get_workspace_members(&self, workspace_root: &Path) -> Result<Vec<PathBuf>> {
        let workspace = self.require_workspace()?;
        let mut members = Vec::new();

        for pattern in &workspace.members {
            let full_pattern = workspace_root.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            // Use glob to expand patterns
            let paths = glob::glob(&pattern_str)
                .with_context(|| format!("Invalid glob pattern: {}", pattern))?;

            for path_result in paths {
                let path = path_result
                    .with_context(|| format!("Failed to resolve glob pattern: {}", pattern))?;

                // Check if it's a directory with CCGO.toml
                if path.is_dir() && path.join("CCGO.toml").exists() {
                    // Check if excluded
                    let relative = path.strip_prefix(workspace_root)
                        .unwrap_or(&path)
                        .to_string_lossy();

                    let is_excluded = workspace.exclude.iter().any(|exc| {
                        // Simple check - could be improved with proper glob matching
                        relative.starts_with(exc) || relative.as_ref() == exc
                    });

                    if !is_excluded {
                        members.push(path);
                    }
                }
            }
        }

        // Sort for consistent ordering
        members.sort();
        Ok(members)
    }

    /// Load all workspace member configurations
    ///
    /// Returns a list of (member_path, member_config) tuples
    pub fn load_workspace_members(&self, workspace_root: &Path) -> Result<Vec<(PathBuf, Self)>> {
        let member_paths = self.get_workspace_members(workspace_root)?;
        let mut members = Vec::new();

        for member_path in member_paths {
            let config_path = member_path.join("CCGO.toml");
            let config = Self::load_from_path(&config_path)
                .with_context(|| format!("Failed to load member config: {}", config_path.display()))?;
            members.push((member_path, config));
        }

        Ok(members)
    }

    /// Resolve workspace dependencies for this configuration
    ///
    /// If this is a member of a workspace, inherits dependencies marked with `workspace = true`
    /// from the workspace root configuration.
    pub fn resolve_workspace_dependencies(&mut self, workspace_config: &Self) -> Result<()> {
        let workspace = workspace_config.require_workspace()?;

        // Build a map of workspace dependencies for quick lookup
        let ws_deps: HashMap<&str, &WorkspaceDependency> = workspace
            .dependencies
            .iter()
            .map(|d| (d.name.as_str(), d))
            .collect();

        // Resolve each dependency that uses workspace inheritance
        for dep in &mut self.dependencies {
            if dep.workspace {
                if let Some(ws_dep) = ws_deps.get(dep.name.as_str()) {
                    dep.merge_with_workspace(ws_dep);
                } else {
                    bail!(
                        "Dependency '{}' is marked as workspace = true, but not found in \
                         [workspace.dependencies]",
                        dep.name
                    );
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"
"#;

        let config = CcgoConfig::parse(toml).unwrap();
        let package = config.package.as_ref().unwrap();
        assert_eq!(package.name, "mylib");
        assert_eq!(package.version, "1.0.0");
    }

    #[test]
    fn test_parse_full_config() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"
description = "My C++ library"
authors = ["Test Author"]
license = "MIT"

[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"

[build]
parallel = true
jobs = 4

[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a", "armeabi-v7a"]

[platforms.ios]
min_version = "12.0"
"#;

        let config = CcgoConfig::parse(toml).unwrap();
        let package = config.package.as_ref().unwrap();
        assert_eq!(package.name, "mylib");
        assert_eq!(config.dependencies.len(), 1);
        assert_eq!(config.dependencies[0].name, "fmt");
        assert!(config.build.is_some());
        assert!(config.platforms.is_some());
    }

    #[test]
    fn test_parse_features_config() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[features]
default = ["std"]
std = []
networking = ["http-client"]
advanced = ["networking", "async"]
full = ["networking", "advanced"]

[[dependencies]]
name = "http-client"
version = "^1.0"
optional = true

[[dependencies]]
name = "async"
version = "^2.0"
optional = true
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        // Check features parsed correctly
        assert_eq!(config.features.default, vec!["std"]);
        assert!(config.features.has_feature("std"));
        assert!(config.features.has_feature("networking"));
        assert!(config.features.has_feature("advanced"));
        assert!(config.features.has_feature("full"));

        // Check optional dependencies
        assert_eq!(config.dependencies.len(), 2);
        assert!(config.dependencies[0].optional);
        assert!(config.dependencies[1].optional);
    }

    #[test]
    fn test_features_resolution() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[features]
default = ["std"]
std = []
networking = ["http-client"]
advanced = ["networking"]
full = ["advanced", "logging"]
logging = []
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        // Test resolving single feature
        let resolved = config.features.resolve_features(&["networking".to_string()], false).unwrap();
        assert!(resolved.contains("networking"));
        assert!(resolved.contains("http-client"));
        assert!(!resolved.contains("std")); // No defaults

        // Test resolving with defaults
        let resolved = config.features.resolve_features(&["networking".to_string()], true).unwrap();
        assert!(resolved.contains("networking"));
        assert!(resolved.contains("std")); // Default included

        // Test resolving transitive features
        let resolved = config.features.resolve_features(&["advanced".to_string()], false).unwrap();
        assert!(resolved.contains("advanced"));
        assert!(resolved.contains("networking"));
        assert!(resolved.contains("http-client"));

        // Test resolving complex feature
        let resolved = config.features.resolve_features(&["full".to_string()], false).unwrap();
        assert!(resolved.contains("full"));
        assert!(resolved.contains("advanced"));
        assert!(resolved.contains("networking"));
        assert!(resolved.contains("logging"));
        assert!(resolved.contains("http-client"));
    }

    #[test]
    fn test_features_unknown_feature_error() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[features]
std = []
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        // Requesting unknown feature should error
        let result = config.features.resolve_features(&["unknown".to_string()], false);
        assert!(result.is_err());
    }

    #[test]
    fn test_optional_dependency_filtering() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[features]
networking = ["http-client"]

[[dependencies]]
name = "fmt"
version = "^10.0"

[[dependencies]]
name = "http-client"
version = "^1.0"
optional = true

[[dependencies]]
name = "unused-optional"
version = "^1.0"
optional = true
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        // Without networking feature, only non-optional deps
        let resolved = config.features.resolve_features(&[], false).unwrap();
        let enabled_deps = config.features.get_enabled_optional_deps(&resolved, &config.dependencies);
        assert_eq!(enabled_deps.len(), 1);
        assert_eq!(enabled_deps[0].name, "fmt");

        // With networking feature, http-client should be enabled
        let resolved = config.features.resolve_features(&["networking".to_string()], false).unwrap();
        let enabled_deps = config.features.get_enabled_optional_deps(&resolved, &config.dependencies);
        assert_eq!(enabled_deps.len(), 2);
        let names: Vec<_> = enabled_deps.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"fmt"));
        assert!(names.contains(&"http-client"));
        assert!(!names.contains(&"unused-optional"));
    }

    #[test]
    fn test_dependency_features() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[[dependencies]]
name = "serde"
version = "^1.0"
features = ["derive", "std"]
default_features = false
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        assert_eq!(config.dependencies[0].features, vec!["derive", "std"]);
        assert_eq!(config.dependencies[0].default_features, Some(false));
    }

    #[test]
    fn test_dependency_feature_syntax() {
        // Test dep/feature syntax in feature dependencies
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[features]
derive = ["serde/derive"]
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        let resolved = config.features.resolve_features(&["derive".to_string()], false).unwrap();
        assert!(resolved.contains("derive"));
        assert!(resolved.contains("serde/derive"));
    }

    #[test]
    fn test_parse_workspace_config() {
        let toml = r#"
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/deprecated"]
resolver = "2"

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"

[[workspace.dependencies]]
name = "spdlog"
version = "^1.12"
"#;

        let config = CcgoConfig::parse(toml).unwrap();
        assert!(config.is_workspace());
        assert!(!config.is_package());

        let workspace = config.workspace.as_ref().unwrap();
        assert_eq!(workspace.members, vec!["core", "utils", "examples/*"]);
        assert_eq!(workspace.exclude, vec!["examples/deprecated"]);
        assert_eq!(workspace.resolver, "2");
        assert_eq!(workspace.dependencies.len(), 2);
        assert_eq!(workspace.dependencies[0].name, "fmt");
        assert_eq!(workspace.dependencies[1].name, "spdlog");
    }

    #[test]
    fn test_parse_workspace_with_package() {
        // A CCGO.toml can have both workspace and package sections
        // (virtual workspace root that is also a package)
        let toml = r#"
[workspace]
members = ["crates/*"]

[package]
name = "my-workspace"
version = "1.0.0"
"#;

        let config = CcgoConfig::parse(toml).unwrap();
        assert!(config.is_workspace());
        assert!(config.is_package());

        let package = config.package.as_ref().unwrap();
        assert_eq!(package.name, "my-workspace");
    }

    #[test]
    fn test_workspace_dependency_inheritance() {
        // Workspace config
        let ws_toml = r#"
[workspace]
members = ["core"]

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]
"#;

        // Member config with workspace dependency inheritance
        let member_toml = r#"
[package]
name = "my-core"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true
features = ["extra"]
"#;

        let ws_config = CcgoConfig::parse(ws_toml).unwrap();
        let mut member_config = CcgoConfig::parse(member_toml).unwrap();

        // Before resolution
        assert!(member_config.dependencies[0].workspace);
        assert!(member_config.dependencies[0].version.is_empty());

        // Resolve workspace dependencies
        member_config.resolve_workspace_dependencies(&ws_config).unwrap();

        // After resolution
        let dep = &member_config.dependencies[0];
        assert_eq!(dep.version, "^10.0");
        assert_eq!(dep.git.as_ref().unwrap(), "https://github.com/fmtlib/fmt.git");
        // Features should be merged (workspace + local)
        assert!(dep.features.contains(&"std".to_string()));
        assert!(dep.features.contains(&"extra".to_string()));
    }

    #[test]
    fn test_workspace_dependency_not_found() {
        let ws_toml = r#"
[workspace]
members = ["core"]

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
"#;

        let member_toml = r#"
[package]
name = "my-core"
version = "1.0.0"

[[dependencies]]
name = "nonexistent"
workspace = true
"#;

        let ws_config = CcgoConfig::parse(ws_toml).unwrap();
        let mut member_config = CcgoConfig::parse(member_toml).unwrap();

        // Should fail because 'nonexistent' is not in workspace.dependencies
        let result = member_config.resolve_workspace_dependencies(&ws_config);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_requires_package_or_workspace() {
        // Empty config should fail
        let toml = r#"
[build]
parallel = true
"#;

        let result = CcgoConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_patch_config() {
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"

[patch.crates-io]
fmt = { git = "https://github.com/myorg/fmt.git", branch = "custom-fix" }

[patch."https://github.com/spdlog/spdlog"]
spdlog = { path = "../spdlog-local" }
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        // Check patches parsed correctly
        assert!(config.patch.has_patches());
        assert_eq!(config.patch.patched_dependencies(), vec!["fmt", "spdlog"]);

        // Check crates-io patch
        let fmt_patch = config.patch.find_patch("fmt", None).unwrap();
        assert_eq!(fmt_patch.git.as_ref().unwrap(), "https://github.com/myorg/fmt.git");
        assert_eq!(fmt_patch.branch.as_ref().unwrap(), "custom-fix");

        // Check source-specific patch
        let spdlog_patch = config
            .patch
            .find_patch("spdlog", Some("https://github.com/spdlog/spdlog"))
            .unwrap();
        assert_eq!(spdlog_patch.path.as_ref().unwrap(), "../spdlog-local");
    }

    #[test]
    fn test_patch_priority() {
        // Source-specific patches should take priority over registry patches
        let toml = r#"
[package]
name = "mylib"
version = "1.0.0"

[patch.crates-io]
fmt = { git = "https://github.com/fallback/fmt.git" }

[patch."https://github.com/fmtlib/fmt.git"]
fmt = { path = "../fmt-local" }
"#;

        let config = CcgoConfig::parse(toml).unwrap();

        // When querying with source, should get source-specific patch
        let fmt_with_source = config
            .patch
            .find_patch("fmt", Some("https://github.com/fmtlib/fmt.git"))
            .unwrap();
        assert!(fmt_with_source.path.is_some());

        // When querying without source, should get registry patch
        let fmt_without_source = config.patch.find_patch("fmt", None).unwrap();
        assert!(fmt_without_source.git.is_some());
    }
}
