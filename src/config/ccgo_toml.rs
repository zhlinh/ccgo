//! CCGO.toml configuration parsing
//!
//! These structs are parsed from TOML and will be used for native Rust implementation.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
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

    /// Version requirement
    pub version: String,

    /// Git repository URL
    pub git: Option<String>,

    /// Git branch name
    pub branch: Option<String>,

    /// Local path (for development)
    pub path: Option<String>,
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

    /// Parse configuration from TOML string
    pub fn parse(content: &str) -> Result<Self> {
        toml::from_str(content).context("Failed to parse CCGO.toml")
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
}
