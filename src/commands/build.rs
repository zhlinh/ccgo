//! Build command implementation

use anyhow::{bail, Result};
use clap::{Args, ValueEnum};

use crate::build::platforms::{build_all, build_apple, get_builder};
use crate::build::{BuildContext, BuildOptions, BuildResult};
use crate::config::CcgoConfig;

/// Target platform for building
#[derive(Debug, Clone, PartialEq, ValueEnum)]
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

    /// Build using Docker container
    #[arg(long)]
    pub docker: bool,

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
}

impl BuildCommand {
    /// Execute the build command
    pub fn execute(self, verbose: bool) -> Result<()> {
        // Load project configuration
        let config = CcgoConfig::load()?;

        // Get project root (where CCGO.toml is located)
        let project_root = std::env::current_dir()?;

        if verbose {
            eprintln!(
                "Building {} for {} platform...",
                config.package.name, self.target
            );
        }

        // Parse architectures from comma-separated string
        let architectures = self
            .arch
            .map(|s| s.split(',').map(|a| a.trim().to_string()).collect())
            .unwrap_or_default();

        // Create build options
        let options = BuildOptions {
            target: self.target.clone(),
            architectures,
            link_type: self.link_type,
            use_docker: self.docker,
            jobs: self.jobs,
            ide_project: self.ide_project,
            release: self.release,
            native_only: self.native_only,
            toolchain: self.toolchain,
            verbose,
        };

        // Create build context
        let ctx = BuildContext::new(project_root, config.clone(), options);

        // Check if Docker build is requested
        if self.docker {
            use crate::build::docker::DockerBuilder;

            // Docker builds only support specific platforms
            match self.target {
                BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => {
                    bail!(
                        "Docker builds are not supported for '{}' target.\n\n\
                         Docker builds support: linux, windows, macos, ios, tvos, watchos, android\n\
                         Build these platforms individually with --docker flag.",
                        self.target
                    );
                }
                _ => {}
            }

            // Create Docker builder and execute
            let docker_builder = DockerBuilder::new(ctx)?;
            let result = docker_builder.execute()?;

            // Print results summary (same as non-Docker builds)
            Self::print_results(&config.package.name, &[result], verbose);
            return Ok(());
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
        Self::print_results(&config.package.name, &results, verbose);

        Ok(())
    }

    /// Print build results summary
    fn print_results(lib_name: &str, results: &[BuildResult], verbose: bool) {
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
        }
    }
}
