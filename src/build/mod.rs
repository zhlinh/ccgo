#![allow(dead_code)] // Allow unused code during development

//! Native Rust build orchestration module
//!
//! This module provides platform-specific build logic that replaces the Python
//! build scripts with native Rust implementations. The goal is ZERO Python
//! dependency - all build logic is implemented in Rust.
//!
//! ## Architecture
//!
//! ```text
//! Rust CLI → build/mod.rs → platforms/<platform>.rs → CMake/Gradle/Hvigor
//! ```
//!
//! ## Modules
//!
//! - `platforms` - Platform-specific builders (linux, macos, windows, ios, android, ohos, etc.)
//! - `toolchains` - Toolchain detection (gcc, clang, xcode, ndk, msvc, mingw, etc.)
//! - `cmake` - CMake configuration and execution
//! - `docker` - Docker-based cross-platform builds
//! - `archive` - ZIP archive creation with build_info.json

pub mod archive;
pub mod cmake;
pub mod cmake_templates;
pub mod docker;
pub mod elf;
pub mod platforms;
pub mod toolchains;

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::commands::build::{BuildTarget, LinkType, WindowsToolchain};
use crate::config::CcgoConfig;

/// Build options passed to platform builders
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Target platform
    pub target: BuildTarget,
    /// Architectures to build (platform-specific)
    pub architectures: Vec<String>,
    /// Link type (static, shared, or both)
    pub link_type: LinkType,
    /// Use Docker for building
    pub use_docker: bool,
    /// Automatically use Docker when native build is not possible
    pub auto_docker: bool,
    /// Number of parallel jobs
    pub jobs: Option<usize>,
    /// Generate IDE project files
    pub ide_project: bool,
    /// Build in release mode
    pub release: bool,
    /// Build only native libraries without packaging (AAR/HAR)
    pub native_only: bool,
    /// Windows toolchain (msvc, mingw, auto)
    pub toolchain: WindowsToolchain,
    /// Verbose output
    pub verbose: bool,
    /// Development mode: use pre-built ccgo binary from GitHub releases in Docker
    pub dev: bool,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            target: BuildTarget::Linux,
            architectures: Vec::new(),
            link_type: LinkType::Both,
            use_docker: false,
            auto_docker: false,
            jobs: None,
            ide_project: false,
            release: true,
            native_only: false,
            toolchain: WindowsToolchain::Auto,
            verbose: false,
            dev: false,
        }
    }
}

/// Build context containing project configuration and build options
#[derive(Debug)]
pub struct BuildContext {
    /// Project root directory (where CCGO.toml is located)
    pub project_root: PathBuf,
    /// Loaded project configuration
    pub config: CcgoConfig,
    /// Build options
    pub options: BuildOptions,
    /// CMake build directory (cmake_build/{debug|release}/<platform>)
    pub cmake_build_dir: PathBuf,
    /// Output directory for final artifacts (target/<platform>)
    pub output_dir: PathBuf,
    /// Git version information
    pub git_version: Option<crate::utils::git_version::GitVersion>,
}

impl BuildContext {
    /// Create a new build context
    pub fn new(project_root: PathBuf, config: CcgoConfig, options: BuildOptions) -> Self {
        // Convert platform name to lowercase for consistent directory structure
        let platform_name = options.target.to_string().to_lowercase();

        // Both cmake_build and target use release/debug subdirectory for consistency:
        // cmake_build/release/android/ or cmake_build/debug/android/
        // target/release/android/ or target/debug/android/
        let release_subdir = if options.release { "release" } else { "debug" };
        let cmake_build_dir = project_root
            .join("cmake_build")
            .join(release_subdir)
            .join(&platform_name);
        let output_dir = project_root
            .join("target")
            .join(release_subdir)
            .join(&platform_name);

        // Calculate git version information (ignore errors - continue without git info)
        let git_version = crate::utils::git_version::GitVersion::from_project_root(
            &project_root,
            &config.package.version,
        ).ok();

        Self {
            project_root,
            config,
            options,
            cmake_build_dir,
            output_dir,
            git_version,
        }
    }

    /// Get the library name from config
    pub fn lib_name(&self) -> &str {
        &self.config.package.name
    }

    /// Get the version string
    pub fn version(&self) -> &str {
        &self.config.package.version
    }

    /// Get the publish suffix (e.g., "beta.18-dirty" or "release")
    /// Returns the base version if git version info is not available
    pub fn publish_suffix(&self) -> &str {
        self.git_version
            .as_ref()
            .map(|gv| gv.publish_suffix.as_str())
            .unwrap_or(&self.config.package.version)
    }

    /// Get the full version with suffix (e.g., "1.0.2-beta.18-dirty")
    pub fn version_with_suffix(&self) -> &str {
        self.git_version
            .as_ref()
            .map(|gv| gv.version_with_suffix.as_str())
            .unwrap_or(&self.config.package.version)
    }

    /// Get the number of parallel jobs (default to CPU count)
    pub fn jobs(&self) -> usize {
        self.options.jobs.unwrap_or_else(num_cpus)
    }

    /// Get the CCGO_CONFIG_DEPS_MAP string for CMake
    ///
    /// Format: "module1;dep1,dep2;module2;dep3"
    /// This tells CMake which submodules depend on which other submodules
    /// for proper shared library linking.
    pub fn deps_map(&self) -> Option<String> {
        let submodule_deps = self.config.build.as_ref()
            .map(|b| &b.submodule_deps)
            .filter(|deps| !deps.is_empty())?;

        let mut deps_list = Vec::new();
        for (module, deps) in submodule_deps {
            if !deps.is_empty() {
                deps_list.push(module.clone());
                deps_list.push(deps.join(","));
            }
        }

        if deps_list.is_empty() {
            None
        } else {
            Some(deps_list.join(";"))
        }
    }

    /// Get the symbol visibility setting (0 = hidden, 1 = default)
    pub fn symbol_visibility(&self) -> u8 {
        if self.config.build.as_ref().map(|b| b.symbol_visibility).unwrap_or(false) {
            1
        } else {
            0
        }
    }

    /// Get the CCGO cmake directory path
    ///
    /// Searches in the following order:
    /// 1. Embedded CMake templates (extracted to ~/.ccgo/cmake/)
    /// 2. CCGO_CMAKE_DIR environment variable
    /// 3. Relative to ccgo-rs binary (for development)
    /// 4. Installed ccgo package location
    pub fn ccgo_cmake_dir(&self) -> Option<PathBuf> {
        // 1. Use embedded CMake templates (primary method)
        if let Ok(cmake_dir) = cmake_templates::get_cmake_dir() {
            return Some(cmake_dir);
        }

        // 2. Check environment variable (fallback)
        if let Ok(dir) = std::env::var("CCGO_CMAKE_DIR") {
            let path = PathBuf::from(dir);
            if path.exists() {
                return Some(path);
            }
        }

        // 3. Check relative to current executable (development mode)
        if let Ok(exe) = std::env::current_exe() {
            // Go up from ccgo-rs/target/debug/ccgo to ccgo-rs/../ccgo/ccgo/build_scripts/cmake
            if let Some(ccgo_rs_root) = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
                let cmake_dir = ccgo_rs_root
                    .parent()
                    .map(|p| p.join("ccgo/ccgo/build_scripts/cmake"));
                if let Some(ref dir) = cmake_dir {
                    if dir.exists() {
                        return cmake_dir;
                    }
                }
            }
        }

        // 4. Try to find installed ccgo package via pip (last resort)
        if let Ok(output) = std::process::Command::new("pip3")
            .args(["show", "-f", "ccgo"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse "Location: /path/to/site-packages"
                for line in stdout.lines() {
                    if let Some(location) = line.strip_prefix("Location: ") {
                        let cmake_dir = PathBuf::from(location.trim())
                            .join("ccgo/build_scripts/cmake");
                        if cmake_dir.exists() {
                            return Some(cmake_dir);
                        }
                    }
                }
            }
        }

        None
    }
}

/// Get number of CPUs for parallel builds
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

/// Build information stored in build_info.json
///
/// Field names are aligned with Python ccgo for compatibility:
/// - `project` (not `name`) - Project name (lowercase)
/// - `build_time` (not `timestamp`) - ISO 8601 with microseconds, local time
/// - `build_host` - Build host OS (Darwin, Linux, Windows)
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Project name (lowercase) - renamed from 'name' to match Python ccgo
    pub project: String,
    /// Platform name (linux, windows, macos, ios, android, ohos, etc.)
    pub platform: String,
    /// Version string
    pub version: String,
    /// Link type (static, shared, both)
    pub link_type: String,
    /// Build timestamp (ISO 8601 with microseconds, local time) - renamed from 'timestamp'
    pub build_time: String,
    /// Build host OS (Darwin, Linux, Windows, etc.)
    pub build_host: String,
    /// Architectures built
    pub architectures: Vec<String>,
    /// Toolchain used (optional, e.g., "msvc", "mingw")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<String>,
    /// Git commit hash (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    /// Git branch (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

/// Comprehensive build information matching pyccgo format
///
/// This is the full JSON structure that matches Python ccgo's build_info output:
/// - build_metadata: version, generated_at, generator
/// - project: name, version
/// - git: branch, revision, revision_full, tag, is_dirty, remote_url
/// - build: time, timestamp, platform, and platform-specific info
/// - environment: os, os_version, python_version, ccgo_version
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildInfoFull {
    pub build_metadata: BuildMetadata,
    pub project: ProjectInfo,
    pub git: GitInfo,
    pub build: BuildDetails,
    pub environment: EnvironmentInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub version: String,
    pub generated_at: String,
    pub generator: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub revision: String,
    pub revision_full: String,
    pub tag: String,
    pub is_dirty: bool,
    pub remote_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildDetails {
    pub time: String,
    pub timestamp: i64,
    pub platform: String,
    /// Platform-specific build info (ios, android, etc.)
    #[serde(flatten)]
    pub platform_info: Option<PlatformBuildInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlatformBuildInfo {
    Ios { ios: IosBuildInfo },
    Android { android: AndroidBuildInfo },
    Macos { macos: MacosBuildInfo },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IosBuildInfo {
    pub xcode_version: String,
    pub xcode_build: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AndroidBuildInfo {
    pub ndk_version: String,
    pub stl: String,
    pub min_sdk_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MacosBuildInfo {
    pub xcode_version: String,
    pub xcode_build: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub os: String,
    pub os_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    pub ccgo_version: String,
}

/// Trait for platform-specific builders
pub trait PlatformBuilder {
    /// Get the platform name
    fn platform_name(&self) -> &str;

    /// Get default architectures for this platform
    fn default_architectures(&self) -> Vec<String>;

    /// Validate build prerequisites (toolchains, SDKs, etc.)
    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()>;

    /// Execute the build
    fn build(&self, ctx: &BuildContext) -> Result<BuildResult>;

    /// Clean build artifacts
    fn clean(&self, ctx: &BuildContext) -> Result<()>;
}

/// Result of a successful build
#[derive(Debug)]
pub struct BuildResult {
    /// Path to the main SDK archive
    pub sdk_archive: PathBuf,
    /// Path to the symbols archive (if generated)
    pub symbols_archive: Option<PathBuf>,
    /// Path to the AAR archive (Android only, if generated)
    pub aar_archive: Option<PathBuf>,
    /// Build duration in seconds
    pub duration_secs: f64,
    /// Architectures that were built
    pub architectures: Vec<String>,
}
