//! Conan package manager platform builder
//!
//! Builds C/C++ library using Conan package manager.
//! Pure Rust implementation that directly invokes Conan CLI.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{get_unified_include_path, ArchiveBuilder, ARCHIVE_DIR_SHARED, ARCHIVE_DIR_STATIC};
use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// Conan platform builder
pub struct ConanBuilder;

impl ConanBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Check if Conan is installed and return version
    fn check_conan_installed() -> Result<String> {
        let output = Command::new("conan")
            .arg("--version")
            .output()
            .context("Failed to run 'conan --version'. Is Conan installed?")?;

        if !output.status.success() {
            bail!("Conan is not installed or not in PATH.\nPlease install: pip install conan");
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }

    /// Detect host architecture
    fn detect_host_arch() -> String {
        #[cfg(target_arch = "x86_64")]
        {
            "x86_64".to_string()
        }
        #[cfg(target_arch = "aarch64")]
        {
            "arm64".to_string()
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            std::env::consts::ARCH.to_string()
        }
    }

    /// Build a specific link type (static or shared) using Conan
    fn build_link_type(&self, ctx: &BuildContext, link_type: &str) -> Result<PathBuf> {
        let build_dir = ctx.cmake_build_dir.join(link_type);
        let install_dir = build_dir.join("install");

        std::fs::create_dir_all(&build_dir)?;
        std::fs::create_dir_all(&install_dir)?;

        let build_shared = link_type == "shared";
        let build_type = if ctx.options.release { "Release" } else { "Debug" };

        // Check if conanfile.py exists in project root or conan/ subdirectory
        let conanfile = if ctx.project_root.join("conanfile.py").exists() {
            ctx.project_root.join("conanfile.py")
        } else if ctx.project_root.join("conan").join("conanfile.py").exists() {
            ctx.project_root.join("conan").join("conanfile.py")
        } else {
            // No conanfile.py, use CMake directly like Linux builder
            return self.build_with_cmake(ctx, link_type);
        };

        let conanfile_dir = conanfile.parent().unwrap();

        if ctx.options.verbose {
            eprintln!("Using conanfile: {}", conanfile.display());
        }

        // Get CCGO_CMAKE_DIR to pass to conan
        let ccgo_cmake_dir = ctx.ccgo_cmake_dir();
        if ctx.options.verbose {
            if let Some(ref dir) = ccgo_cmake_dir {
                eprintln!("CCGO_CMAKE_DIR: {}", dir.display());
            }
        }

        // Step 1: conan install - install dependencies and generate build files
        let mut install_cmd = Command::new("conan");
        install_cmd
            .current_dir(conanfile_dir)
            .arg("install")
            .arg(".")
            .arg("--output-folder")
            .arg(build_dir.display().to_string())
            .arg("--build=missing")
            .arg(format!("-s:h=build_type={}", build_type));

        // Pass CCGO_CMAKE_DIR environment variable
        if let Some(ref cmake_dir) = ccgo_cmake_dir {
            install_cmd.env("CCGO_CMAKE_DIR", cmake_dir);
        }

        // Add shared library setting
        if build_shared {
            install_cmd.arg("-o:h=*:shared=True");
        } else {
            install_cmd.arg("-o:h=*:shared=False");
        }

        // Pass CCGO_BUILD_SHARED environment variable for conanfile.py to read
        install_cmd.env("CCGO_BUILD_SHARED", if build_shared { "ON" } else { "OFF" });

        if ctx.options.verbose {
            eprintln!("Running: conan install {:?}", install_cmd.get_args().collect::<Vec<_>>());
        }

        let status = install_cmd
            .status()
            .context("Failed to run conan install")?;

        if !status.success() {
            bail!("conan install failed");
        }

        // Step 2: conan build - build the library
        // Note: conan build may regenerate toolchain files, so we need to pass settings again
        let mut build_cmd = Command::new("conan");
        build_cmd
            .current_dir(conanfile_dir)
            .arg("build")
            .arg(".")
            .arg("--output-folder")
            .arg(build_dir.display().to_string());

        // Pass CCGO_CMAKE_DIR environment variable
        if let Some(ref cmake_dir) = ccgo_cmake_dir {
            build_cmd.env("CCGO_CMAKE_DIR", cmake_dir);
        }

        // Pass CCGO_BUILD_SHARED environment variable for conanfile.py to read
        // This ensures the shared setting is preserved when conan build regenerates toolchain
        build_cmd.env("CCGO_BUILD_SHARED", if build_shared { "ON" } else { "OFF" });

        if ctx.options.verbose {
            eprintln!("Running: conan build {:?}", build_cmd.get_args().collect::<Vec<_>>());
        }

        let status = build_cmd
            .status()
            .context("Failed to run conan build")?;

        if !status.success() {
            bail!("conan build failed");
        }

        Ok(build_dir)
    }

    /// Build using CMake directly (fallback when no conanfile.py)
    fn build_with_cmake(&self, ctx: &BuildContext, link_type: &str) -> Result<PathBuf> {
        let build_dir = ctx.cmake_build_dir.join(link_type);
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";

        // Configure and build with CMake
        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .build_type(if ctx.options.release {
                BuildType::Release
            } else {
                BuildType::Debug
            })
            .install_prefix(install_dir.clone())
            .variable("CCGO_BUILD_STATIC", if build_shared { "OFF" } else { "ON" })
            .variable("CCGO_BUILD_SHARED", if build_shared { "ON" } else { "OFF" })
            .variable("CCGO_BUILD_SHARED_LIBS", if build_shared { "ON" } else { "OFF" })
            .variable("CCGO_LIB_NAME", ctx.lib_name())
            .jobs(ctx.jobs())
            .verbose(ctx.options.verbose);

        // Add CCGO_CMAKE_DIR if available
        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        // Add CCGO configuration variables
        cmake = cmake.variable(
            "CCGO_CONFIG_PRESET_VISIBILITY",
            ctx.symbol_visibility().to_string(),
        );

        // Add submodule dependencies for shared library linking
        if let Some(deps_map) = ctx.deps_map() {
            cmake = cmake.variable("CCGO_CONFIG_DEPS_MAP", deps_map);
        }

        cmake.configure_build_install()?;

        Ok(build_dir)
    }

    /// Find library directory in build output
    /// Checks for actual library files, not just directory existence
    fn find_lib_dir(&self, build_dir: &PathBuf, _is_release: bool) -> Option<PathBuf> {
        // Conan 2.x with cmake_layout puts libraries directly in build/Release or build/Debug
        // Always check Release first since Conan often builds Release even for debug configs
        let possible_dirs = vec![
            build_dir.join("out"),                          // CCGO cmake output
            build_dir.join("build/Release"),                // Conan Release build (most common)
            build_dir.join("build/Debug"),                  // Conan Debug build
            build_dir.join("install/lib"),                  // CMake install
            build_dir.join("lib"),                          // Direct lib output
            build_dir.join("build/lib"),                    // Conan build output
        ];

        // Library extensions to look for
        let lib_extensions = ["a", "so", "dylib", "lib", "dll"];

        for dir in &possible_dirs {
            if !dir.exists() {
                continue;
            }
            // Check if directory contains any library files
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if lib_extensions.contains(&ext) {
                                return Some(dir.clone());
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Find include directory in build output
    fn find_include_dir(&self, build_dir: &PathBuf) -> Option<PathBuf> {
        let possible_dirs = vec![
            build_dir.join("install/include"),
            build_dir.join("include"),
            build_dir.join("build/include"),
        ];

        for dir in possible_dirs {
            if dir.exists() {
                return Some(dir);
            }
        }
        None
    }
}

impl PlatformBuilder for ConanBuilder {
    fn platform_name(&self) -> &str {
        "conan"
    }

    fn default_architectures(&self) -> Vec<String> {
        // Conan builds for the host architecture
        vec![Self::detect_host_arch()]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        // Check if Conan is installed
        let version = Self::check_conan_installed()
            .context("Conan build requires Conan to be installed")?;

        if ctx.options.verbose {
            eprintln!("Found {}", version);
        }

        // Check for CMake
        if !crate::build::cmake::is_cmake_available() {
            bail!("CMake is required for Conan builds. Please install CMake.");
        }

        if ctx.options.verbose {
            eprintln!("Conan prerequisites validated");
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Validate prerequisites first
        self.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for Conan...", ctx.lib_name());
        }

        // Create output directory
        std::fs::create_dir_all(&ctx.output_dir)?;

        // Create archive builder
        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "conan",
            ctx.output_dir.clone(),
        )?;

        let host_arch = Self::detect_host_arch();
        let mut built_link_types = Vec::new();

        // Build static libraries
        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            if ctx.options.verbose {
                eprintln!("Building static library...");
            }
            let build_dir = self.build_link_type(ctx, "static")?;

            // Add static library to archive: lib/conan/static/
            // Use add_files_flat to only copy files at root level (merged library only)
            if let Some(lib_dir) = self.find_lib_dir(&build_dir, ctx.options.release) {
                let archive_path = format!("lib/{}/{}", self.platform_name(), ARCHIVE_DIR_STATIC);
                archive.add_files_flat(&lib_dir, &archive_path, &["a", "lib"])?;
            }
            built_link_types.push("static");
        }

        // Build shared libraries
        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            if ctx.options.verbose {
                eprintln!("Building shared library...");
            }
            let build_dir = self.build_link_type(ctx, "shared")?;

            // Add shared library to archive: lib/conan/shared/
            // Use add_files_flat to only copy files at root level (merged library only)
            if let Some(lib_dir) = self.find_lib_dir(&build_dir, ctx.options.release) {
                let archive_path = format!("lib/{}/{}", self.platform_name(), ARCHIVE_DIR_SHARED);
                // Include platform-appropriate shared library extensions
                #[cfg(target_os = "windows")]
                let extensions = &["dll", "lib"];
                #[cfg(target_os = "macos")]
                let extensions = &["dylib", "so", "a"]; // Include .a for static archives used with shared
                #[cfg(target_os = "linux")]
                let extensions = &["so", "a"];
                #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
                let extensions = &["so", "dylib", "dll", "a"];

                archive.add_files_flat(&lib_dir, &archive_path, extensions)?;
            }
            built_link_types.push("shared");
        }

        // Add include files from project's include directory (matching pyccgo behavior)
        let include_source = ctx.project_root.join("include");
        if include_source.exists() {
            let include_path = get_unified_include_path(ctx.lib_name(), &include_source);
            archive.add_directory(&include_source, &include_path)?;
            if ctx.options.verbose {
                eprintln!("Added include files from {} to {}", include_source.display(), include_path);
            }
        }

        // Create the SDK archive
        let link_type_str = ctx.options.link_type.to_string();
        let sdk_archive = archive.create_sdk_archive(&[host_arch.clone()], &link_type_str)?;

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "Conan build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: None,
            aar_archive: None,
            duration_secs: duration.as_secs_f64(),
            architectures: vec![host_arch],
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean cmake_build/{release|debug}/conan
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("conan");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure
        for old_dir in &[
            ctx.project_root.join("cmake_build/Conan"),
            ctx.project_root.join("cmake_build/conan"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/conan"),
            ctx.project_root.join("target/debug/conan"),
            ctx.project_root.join("target/conan"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean conan/ directory build artifacts
        let conan_dir = ctx.project_root.join("conan");
        if conan_dir.exists() {
            for subdir in &["build", "cmake-build-release", "cmake-build-debug"] {
                let build_subdir = conan_dir.join(subdir);
                if build_subdir.exists() {
                    std::fs::remove_dir_all(&build_subdir)
                        .with_context(|| format!("Failed to clean {}", build_subdir.display()))?;
                }
            }
        }

        Ok(())
    }
}

impl Default for ConanBuilder {
    fn default() -> Self {
        Self::new()
    }
}
