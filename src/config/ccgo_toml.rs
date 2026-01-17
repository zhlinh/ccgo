//! CCGO.toml configuration parsing
//!
//! These structs are parsed from TOML and will be used for native Rust implementation.

#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// Root configuration from CCGO.toml
#[derive(Debug, Clone, Deserialize)]
pub struct CcgoConfig {
    /// Package metadata (supports both [package] and [project] sections)
    #[serde(alias = "project")]
    pub package: PackageConfig,

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

/// Dependency configuration from [[dependencies]] array
#[derive(Debug, Clone, Deserialize)]
pub struct DependencyConfig {
    /// Dependency name
    pub name: String,

    /// Version requirement (supports semver ranges like ^1.0, ~1.2.3, >=1.0,<2.0)
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

        // Validate dependencies
        for dep in &config.dependencies {
            dep.validate().with_context(|| format!("Invalid dependency: {}", dep.name))?;
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
        assert_eq!(config.package.name, "mylib");
        assert_eq!(config.package.version, "1.0.0");
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
        assert_eq!(config.package.name, "mylib");
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
}
