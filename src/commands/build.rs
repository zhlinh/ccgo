//! Build command implementation

use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use clap::{Args, ValueEnum};

use crate::build::archive::{create_build_info_full, print_build_info_json};
use crate::build::platforms::{build_all, build_apple, get_builder};
use crate::build::{BuildContext, BuildOptions, BuildResult};
use crate::config::CcgoConfig;
use crate::workspace::{find_workspace_root, Workspace};

/// Target platform for building
#[derive(Debug, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum BuildTarget {
    /// Build for all platforms
    All,
    /// Build for all Apple platforms
    Apple,
    /// Android platform
    Android,
    /// iOS platform
    Ios,
    /// macOS platform
    Macos,
    /// Windows platform
    Windows,
    /// Linux platform
    Linux,
    /// OpenHarmony platform
    Ohos,
    /// tvOS platform
    Tvos,
    /// watchOS platform
    Watchos,
    /// Kotlin Multiplatform
    Kmp,
    /// Conan package
    Conan,
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTarget::All => write!(f, "all"),
            BuildTarget::Apple => write!(f, "apple"),
            BuildTarget::Android => write!(f, "Android"),  // Capital A to match Python pyccgo
            BuildTarget::Ios => write!(f, "ios"),
            BuildTarget::Macos => write!(f, "macos"),
            BuildTarget::Windows => write!(f, "windows"),
            BuildTarget::Linux => write!(f, "linux"),
            BuildTarget::Ohos => write!(f, "ohos"),
            BuildTarget::Tvos => write!(f, "tvos"),
            BuildTarget::Watchos => write!(f, "watchos"),
            BuildTarget::Kmp => write!(f, "kmp"),
            BuildTarget::Conan => write!(f, "conan"),
        }
    }
}

/// Library linking type
#[derive(Debug, Clone, Default, ValueEnum, PartialEq)]
pub enum LinkType {
    /// Static library only
    Static,
    /// Shared/dynamic library only
    Shared,
    /// Both static and shared
    #[default]
    Both,
}

impl std::fmt::Display for LinkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkType::Static => write!(f, "static"),
            LinkType::Shared => write!(f, "shared"),
            LinkType::Both => write!(f, "both"),
        }
    }
}

/// Windows toolchain selection
#[derive(Debug, Clone, Default, ValueEnum)]
pub enum WindowsToolchain {
    /// MSVC toolchain
    Msvc,
    /// MinGW toolchain
    Mingw,
    /// Auto-detect (both)
    #[default]
    Auto,
}

impl std::fmt::Display for WindowsToolchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowsToolchain::Msvc => write!(f, "msvc"),
            WindowsToolchain::Mingw => write!(f, "mingw"),
            WindowsToolchain::Auto => write!(f, "auto"),
        }
    }
}

/// Build library for specific platform
#[derive(Args, Debug)]
pub struct BuildCommand {
    /// Target platform to build
    #[arg(value_enum)]
    pub target: BuildTarget,

    /// Architectures to build (comma-separated)
    #[arg(long)]
    pub arch: Option<String>,

    /// Link type
    #[arg(long, value_enum, default_value_t = LinkType::Both)]
    pub link_type: LinkType,

    /// Build all workspace members
    #[arg(long)]
    pub workspace: bool,

    /// Build only the specified package (in a workspace)
    #[arg(long, short = 'p')]
    pub package: Option<String>,

    /// Build using Docker container
    #[arg(long)]
    pub docker: bool,

    /// Automatically use Docker when native build is not possible
    ///
    /// For example, on macOS building for Linux or Windows requires Docker.
    /// This flag will automatically detect and use Docker when needed.
    #[arg(long)]
    pub auto_docker: bool,

    /// Number of parallel jobs
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Generate IDE project files
    #[arg(long)]
    pub ide_project: bool,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Build only native libraries without packaging (AAR/HAR)
    #[arg(long)]
    pub native_only: bool,

    /// Windows toolchain selection
    #[arg(long, value_enum, default_value_t = WindowsToolchain::Auto)]
    pub toolchain: WindowsToolchain,

    /// Development mode: use pre-built ccgo binary from GitHub releases in Docker builds
    #[arg(long)]
    pub dev: bool,

    /// Features to enable (comma-separated)
    ///
    /// Example: --features networking,advanced
    #[arg(long, short = 'F', value_delimiter = ',')]
    pub features: Vec<String>,

    /// Do not enable default features
    ///
    /// By default, the features listed in [features].default are enabled.
    /// Use this flag to disable them.
    #[arg(long)]
    pub no_default_features: bool,

    /// Enable all available features
    #[arg(long)]
    pub all_features: bool,

    /// Compiler cache to use (ccache, sccache, auto, none)
    ///
    /// Default: auto - automatically detect and use available cache
    /// Use 'none' to disable caching
    #[arg(long, default_value = "auto")]
    pub cache: String,
}

impl BuildCommand {
    /// Check if a platform can be built natively on the current host
    fn can_build_natively(target: &BuildTarget) -> bool {
        let host_os = std::env::consts::OS;

        match target {
            // All/Apple/Kmp/Conan are meta-targets, check individual platforms
            BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => true,

            // Linux can only be built natively on Linux
            BuildTarget::Linux => host_os == "linux",

            // Windows can only be built natively on Windows
            BuildTarget::Windows => host_os == "windows",

            // Apple platforms can only be built on macOS (Xcode required)
            BuildTarget::Macos | BuildTarget::Ios | BuildTarget::Tvos | BuildTarget::Watchos => {
                host_os == "macos"
            }

            // Android can be built on any platform with NDK
            BuildTarget::Android => true,

            // OHOS can be built on any platform with OHOS SDK
            BuildTarget::Ohos => true,
        }
    }

    /// Determine if Docker should be used for this build
    fn should_use_docker(&self, target: &BuildTarget) -> bool {
        // Explicit --docker flag always uses Docker
        if self.docker {
            return true;
        }

        // --auto-docker enables automatic Docker detection
        if self.auto_docker && !Self::can_build_natively(target) {
            return true;
        }

        false
    }

    /// Execute the build command
    pub fn execute(self, verbose: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;

        // Check for workspace context
        if self.workspace || self.package.is_some() {
            return self.execute_workspace_build(&current_dir, verbose);
        }

        // Check if we're in a workspace root but --workspace not specified
        if Workspace::is_workspace(&current_dir) {
            eprintln!(
                "ℹ️  In workspace root. Use --workspace to build all members, \
                 or --package <name> to build a specific member."
            );
        }

        // Load project configuration
        let config = CcgoConfig::load()?;

        // Get project root (where CCGO.toml is located)
        let project_root = current_dir;

        // Get package info (required for builds)
        let package = config.require_package()?;

        if verbose {
            eprintln!(
                "Building {} for {} platform...",
                package.name, self.target
            );
        }

        // Check if we should use Docker (explicit or auto-detected)
        let use_docker = self.should_use_docker(&self.target);

        // If auto-docker detected Docker is needed, inform the user
        if self.auto_docker && use_docker && !self.docker {
            let host_os = std::env::consts::OS;
            eprintln!(
                "🐳 Auto-docker: {} cannot be built natively on {} - using Docker",
                self.target, host_os
            );
        }

        // Parse architectures from comma-separated string
        let architectures = self
            .arch
            .clone()
            .map(|s| s.split(',').map(|a| a.trim().to_string()).collect())
            .unwrap_or_default();

        // Create build options
        let options = BuildOptions {
            target: self.target.clone(),
            architectures,
            link_type: self.link_type.clone(),
            use_docker,
            auto_docker: self.auto_docker,
            jobs: self.jobs,
            ide_project: self.ide_project,
            release: self.release,
            native_only: self.native_only,
            toolchain: self.toolchain.clone(),
            verbose,
            dev: self.dev,
            features: self.features.clone(),
            use_default_features: !self.no_default_features,
            all_features: self.all_features,
            cache: Some(self.cache.clone()),
        };

        // Create build context
        let ctx = BuildContext::new(project_root, config.clone(), options);

        // Check if Docker build is requested (explicit or auto-detected)
        if use_docker {
            use crate::build::docker::DockerBuilder;

            // Docker builds only support specific platforms
            match self.target {
                BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => {
                    if self.auto_docker {
                        // For auto-docker with multi-platform targets, we should fall through
                        // to native build which will handle Docker per-platform
                        eprintln!(
                            "ℹ Auto-docker with '{}' will build each platform with Docker as needed",
                            self.target
                        );
                    } else {
                        bail!(
                            "Docker builds are not supported for '{}' target.\n\n\
                             Docker builds support: linux, windows, macos, ios, tvos, watchos, android\n\
                             Build these platforms individually with --docker flag.\n\
                             Or use --auto-docker to automatically use Docker when needed.",
                            self.target
                        );
                    }
                }
                _ => {
                    // Save project_root before ctx is moved
                    let docker_project_root = ctx.project_root.clone();

                    // Create Docker builder and execute
                    let docker_builder = DockerBuilder::new(ctx)?;
                    let result = docker_builder.execute()?;

                    // Print results summary (same as non-Docker builds)
                    Self::print_results(&package.name, &package.version, &self.target.to_string(), &docker_project_root, &[result], verbose);
                    return Ok(());
                }
            }
        }

        // Check CCGO_CMAKE_DIR availability
        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            if verbose {
                eprintln!("Using CCGO cmake directory: {}", cmake_dir.display());
            }
        } else {
            eprintln!(
                "Warning: CCGO cmake directory not found. Set CCGO_CMAKE_DIR environment variable \
                 or install the ccgo package."
            );
        }

        // Execute the build based on target
        let results = match self.target {
            BuildTarget::All => build_all(&ctx)?,
            BuildTarget::Apple => build_apple(&ctx)?,
            _ => {
                // Single platform build
                let builder = get_builder(&self.target)?;
                vec![builder.build(&ctx)?]
            }
        };

        // Print results summary
        Self::print_results(&package.name, &package.version, &self.target.to_string(), &ctx.project_root, &results, verbose);

        Ok(())
    }

    /// Execute build for workspace members
    fn execute_workspace_build(&self, current_dir: &Path, verbose: bool) -> Result<()> {
        // Find workspace root
        let workspace_root = find_workspace_root(current_dir)?
            .ok_or_else(|| anyhow::anyhow!(
                "Not in a workspace. Use --workspace or --package only within a workspace."
            ))?;

        // Load workspace
        let workspace = Workspace::load(&workspace_root)?;

        if verbose {
            workspace.print_summary();
        }

        // Determine which members to build
        let members_to_build = if let Some(ref package_name) = self.package {
            // Build specific package
            let member = workspace.get_member(package_name)
                .ok_or_else(|| anyhow::anyhow!(
                    "Package '{}' not found in workspace. Available: {}",
                    package_name,
                    workspace.members.names().join(", ")
                ))?;
            vec![member]
        } else {
            // Build default members (or all if no default_members specified)
            workspace.default_members()
        };

        if members_to_build.is_empty() {
            bail!("No workspace members to build");
        }

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("CCGO Workspace Build - Building {} member(s)", members_to_build.len());
        eprintln!("{}", "=".repeat(80));

        let mut all_results: Vec<(String, Vec<BuildResult>)> = Vec::new();
        let mut failed_members: Vec<String> = Vec::new();

        for member in members_to_build {
            eprintln!("\n📦 Building {} ({})...", member.name, member.version);
            eprintln!("{}", "-".repeat(60));

            // Execute build in member's directory
            match self.build_member(&workspace_root, member, verbose) {
                Ok(results) => {
                    all_results.push((member.name.clone(), results));
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to build {}: {}", member.name, e);
                    failed_members.push(member.name.clone());
                }
            }
        }

        // Print summary
        eprintln!("\n{}", "=".repeat(80));
        eprintln!("Workspace Build Summary");
        eprintln!("{}", "=".repeat(80));

        let success_count = all_results.len();
        let fail_count = failed_members.len();

        eprintln!("\n✓ Successfully built: {}", success_count);
        for (name, results) in &all_results {
            let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();
            eprintln!("  - {} ({:.2}s)", name, total_duration);
        }

        if !failed_members.is_empty() {
            eprintln!("\n✗ Failed: {}", fail_count);
            for name in &failed_members {
                eprintln!("  - {}", name);
            }
            bail!("{} workspace member(s) failed to build", fail_count);
        }

        Ok(())
    }

    /// Build a single workspace member
    fn build_member(
        &self,
        workspace_root: &Path,
        member: &crate::workspace::WorkspaceMember,
        verbose: bool,
    ) -> Result<Vec<BuildResult>> {
        // Construct member's directory path
        let member_path = workspace_root.join(&member.name);

        // Load member's configuration
        let config_path = member_path.join("CCGO.toml");
        let config = CcgoConfig::load_from(&config_path)?;

        // Get package info
        let package = config.require_package()?;

        // Parse architectures
        let architectures = self.arch.clone()
            .map(|s| s.split(',').map(|a| a.trim().to_string()).collect())
            .unwrap_or_default();

        // Check if we should use Docker
        let use_docker = self.should_use_docker(&self.target);

        // Create build options
        let options = BuildOptions {
            target: self.target.clone(),
            architectures,
            link_type: self.link_type.clone(),
            use_docker,
            auto_docker: self.auto_docker,
            jobs: self.jobs,
            ide_project: self.ide_project,
            release: self.release,
            native_only: self.native_only,
            toolchain: self.toolchain.clone(),
            verbose,
            dev: self.dev,
            features: self.features.clone(),
            use_default_features: !self.no_default_features,
            all_features: self.all_features,
            cache: Some(self.cache.clone()),
        };

        // Create build context with member's path
        let ctx = BuildContext::new(member_path.clone(), config.clone(), options);

        // Execute the build
        let results = if use_docker {
            use crate::build::docker::DockerBuilder;

            match self.target {
                BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => {
                    // Fall through to native build for meta-targets
                    match self.target {
                        BuildTarget::All => build_all(&ctx)?,
                        BuildTarget::Apple => build_apple(&ctx)?,
                        _ => {
                            let builder = get_builder(&self.target)?;
                            vec![builder.build(&ctx)?]
                        }
                    }
                }
                _ => {
                    let docker_builder = DockerBuilder::new(ctx)?;
                    vec![docker_builder.execute()?]
                }
            }
        } else {
            match self.target {
                BuildTarget::All => build_all(&ctx)?,
                BuildTarget::Apple => build_apple(&ctx)?,
                _ => {
                    let builder = get_builder(&self.target)?;
                    vec![builder.build(&ctx)?]
                }
            }
        };

        // Print results for this member
        Self::print_results(
            &package.name,
            &package.version,
            &self.target.to_string(),
            &member_path,
            &results,
            verbose,
        );

        Ok(results)
    }

    /// Print build results summary
    fn print_results(lib_name: &str, version: &str, platform: &str, project_root: &Path, results: &[BuildResult], verbose: bool) {
        let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();

        if results.is_empty() {
            eprintln!("No builds completed.");
            return;
        }

        if verbose {
            eprintln!("\n=== Build Summary ===");
            for result in results {
                eprintln!(
                    "  {} ({:.2}s): {}",
                    result.architectures.join(", "),
                    result.duration_secs,
                    result.sdk_archive.display()
                );
            }
        }

        // Print build info JSON before success message
        let build_info = create_build_info_full(lib_name, version, platform, project_root);
        print_build_info_json(&build_info);

        eprintln!(
            "\n✓ {} built successfully in {:.2}s",
            lib_name, total_duration
        );

        // Print archive locations and contents
        for result in results {
            eprintln!("  SDK: {}", result.sdk_archive.display());
            if let Some(symbols) = &result.symbols_archive {
                eprintln!("  Symbols: {}", symbols.display());
            }
            if let Some(aar) = &result.aar_archive {
                eprintln!("  AAR: {}", aar.display());
            }

            // Print archive tree structure
            if let Err(e) = crate::build::archive::print_zip_tree(&result.sdk_archive, "      ") {
                eprintln!("      Warning: Failed to print archive contents: {}", e);
            }

            // Print symbols archive tree if present
            if let Some(symbols_path) = &result.symbols_archive {
                eprintln!("\n      Symbols archive:");
                if let Err(e) = crate::build::archive::print_zip_tree(symbols_path, "      ") {
                    eprintln!("      Warning: Failed to print symbols archive contents: {}", e);
                }
            }

            // Print AAR/HAR archive tree if present (Android/OHOS)
            if let Some(archive_path) = &result.aar_archive {
                // Detect archive type from extension
                let archive_type = if archive_path.extension().map_or(false, |e| e == "har") {
                    "HAR"
                } else {
                    "AAR"
                };
                eprintln!("\n      {} contents:", archive_type);
                if let Err(e) = crate::build::archive::print_zip_tree(archive_path, "      ") {
                    eprintln!("      Warning: Failed to print {} contents: {}", archive_type, e);
                }
            }
        }
    }
}
