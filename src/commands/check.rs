//! Check command implementation
//!
//! Performs compilation checking without generating binaries.
//! Similar to `cargo check` - validates that the code compiles correctly
//! by running CMake configure and build with -fsyntax-only flag.

use std::time::Instant;

use anyhow::Result;
use clap::Args;

use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::BuildContext;
use crate::config::CcgoConfig;

/// Check compilation without generating binaries
#[derive(Args, Debug)]
pub struct CheckCommand {
    /// Build in release mode (affects compiler flags)
    #[arg(long)]
    pub release: bool,

    /// Features to enable (comma-separated)
    #[arg(long, short = 'F', value_delimiter = ',')]
    pub features: Vec<String>,

    /// Do not enable default features
    #[arg(long)]
    pub no_default_features: bool,

    /// Enable all available features
    #[arg(long)]
    pub all_features: bool,
}

impl CheckCommand {
    /// Execute the check command
    pub fn execute(self, verbose: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;

        // Load project configuration
        let config = CcgoConfig::load()?;
        let package = config.require_package()?;

        eprintln!("🔍 Checking {} for compilation errors...\n", package.name);

        let start = Instant::now();

        // Create a minimal BuildOptions for BuildContext
        let options = crate::build::BuildOptions {
            target: crate::commands::build::BuildTarget::Macos, // host platform for check
            architectures: Vec::new(),
            link_type: crate::commands::build::LinkType::Static,
            use_docker: false,
            auto_docker: false,
            jobs: None,
            ide_project: false,
            release: self.release,
            native_only: true,
            toolchain: crate::commands::build::WindowsToolchain::Auto,
            verbose,
            dev: false,
            features: self.features.clone(),
            use_default_features: !self.no_default_features,
            all_features: self.all_features,
            cache: Some("none".to_string()),
            analytics: false,
        };

        let ctx = BuildContext::new(current_dir.clone(), config.clone(), options);

        // Set up build directory for syntax check
        let build_dir = current_dir.join("cmake_build").join("check");

        let build_type = if self.release {
            BuildType::Release
        } else {
            BuildType::Debug
        };

        let mut cmake = CMakeConfig::new(current_dir.clone(), build_dir.clone())
            .build_type(build_type)
            .variable("CCGO_BUILD_STATIC", "ON")
            .variable("CCGO_BUILD_SHARED", "OFF")
            .variable("CCGO_LIB_NAME", ctx.lib_name())
            .variable("CCGO_SYNTAX_CHECK", "ON")
            .verbose(verbose);

        // Add CCGO_CMAKE_DIR
        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        // Add feature definitions
        if let Ok(feature_defines) = ctx.cmake_feature_defines() {
            if !feature_defines.is_empty() {
                cmake = cmake.feature_definitions(&feature_defines);
            }
        }

        // Add deps map
        if let Some(deps_map) = ctx.deps_map() {
            if !deps_map.is_empty() {
                cmake = cmake.variable("CCGO_CONFIG_DEPS_MAP", &deps_map);
            }
        }

        // Add symbol visibility
        cmake = cmake.variable(
            "CCGO_CONFIG_PRESET_VISIBILITY",
            ctx.symbol_visibility().to_string(),
        );

        // Run CMake configure
        if verbose {
            eprintln!("Running CMake configure...");
        }
        cmake.configure()?;

        // Run CMake build to verify compilation
        if verbose {
            eprintln!("Running compilation check...");
        }
        cmake.build()?;

        let duration = start.elapsed();
        eprintln!(
            "\n✅ {} compiles successfully ({:.2}s)",
            package.name,
            duration.as_secs_f64()
        );

        Ok(())
    }
}
