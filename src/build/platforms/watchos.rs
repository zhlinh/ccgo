//! watchOS platform builder
//!
//! Builds XCFrameworks for watchOS using CMake with Xcode toolchain.
//! Supports device (arm64_32, armv7k) and simulator (arm64) architectures.
//! Note: watchOS Simulator no longer supports x86_64 since Xcode 14.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{
    get_unified_include_path, ArchiveBuilder, ARCHIVE_DIR_FRAMEWORKS, ARCHIVE_DIR_SHARED,
    ARCHIVE_DIR_STATIC,
};
use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::toolchains::xcode::{ApplePlatform, XcodeToolchain};
use crate::build::toolchains::Toolchain;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// watchOS platform builder
pub struct WatchosBuilder {
    /// Xcode toolchain (lazily initialized)
    xcode: Option<XcodeToolchain>,
}

impl WatchosBuilder {
    pub fn new() -> Self {
        Self { xcode: None }
    }

    /// Get or detect Xcode toolchain
    fn get_xcode(&mut self) -> Result<&XcodeToolchain> {
        if self.xcode.is_none() {
            self.xcode = Some(XcodeToolchain::detect()?);
        }
        Ok(self.xcode.as_ref().unwrap())
    }

    /// Merge all module static libraries into a single library
    /// This is essential for KMP cinterop which expects a single complete library
    fn merge_module_static_libs(
        &self,
        xcode: &XcodeToolchain,
        build_dir: &PathBuf,
        lib_name: &str,
        verbose: bool,
    ) -> Result<()> {
        // Find the output directory where CMake puts libraries
        let out_dir = build_dir.join("out");
        if !out_dir.exists() {
            // No out directory means no libraries to merge
            return Ok(());
        }

        // Find all .a files (module libraries)
        let mut module_libs: Vec<PathBuf> = Vec::new();
        for entry in std::fs::read_dir(&out_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "a" {
                        module_libs.push(path);
                    }
                }
            }
        }

        if module_libs.is_empty() {
            return Ok(());
        }

        // Check if we only have the main library (already merged or single module)
        let main_lib_name = format!("lib{}.a", lib_name);
        if module_libs.len() == 1
            && module_libs[0]
                .file_name()
                .map_or(false, |n| n == main_lib_name.as_str())
        {
            // Already a single main library, nothing to merge
            return Ok(());
        }

        // Filter out the main library if it exists (we'll recreate it)
        let main_lib_path = out_dir.join(&main_lib_name);
        module_libs.retain(|p| p != &main_lib_path);

        if module_libs.is_empty() {
            return Ok(());
        }

        if verbose {
            eprintln!(
                "    Merging {} module libraries into {}",
                module_libs.len(),
                main_lib_name
            );
        }

        // Merge all module libraries into the main library
        xcode.merge_static_libs(&module_libs, &main_lib_path)?;

        // Clean up module libraries after merge (optional, keeps output clean)
        for lib in &module_libs {
            if lib != &main_lib_path {
                let _ = std::fs::remove_file(lib);
            }
        }

        Ok(())
    }

    /// Build for a single architecture
    fn build_arch(
        &self,
        ctx: &BuildContext,
        xcode: &XcodeToolchain,
        arch: &str,
        link_type: &str,
        sdk: &str,
    ) -> Result<PathBuf> {
        let build_dir = ctx
            .cmake_build_dir
            .join(format!("{}/{}/{}", link_type, sdk, arch));
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";

        // Get watchOS SDK path and CMake variables (use correct platform based on SDK type)
        let platform = if sdk == "simulator" {
            ApplePlatform::WatchOSSimulator
        } else {
            ApplePlatform::WatchOS
        };
        let cmake_vars = xcode.cmake_variables_for_platform(platform)?;

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
            .variable("CMAKE_OSX_ARCHITECTURES", arch)
            .jobs(ctx.jobs())
            .verbose(ctx.options.verbose);

        // Add CCGO_CMAKE_DIR if available
        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        // Add SDK-related variables
        for (name, value) in cmake_vars {
            if name != "CMAKE_OSX_ARCHITECTURES" {
                cmake = cmake.variable(&name, &value);
            }
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

        // For static builds, merge all module libraries into a single library
        // This is essential for KMP cinterop which expects a single complete library
        if !build_shared {
            self.merge_module_static_libs(xcode, &build_dir, ctx.lib_name(), ctx.options.verbose)?;
        }

        Ok(build_dir)
    }

    /// Create XCFramework from device and simulator libraries
    fn create_xcframework(
        &self,
        _xcode: &XcodeToolchain,
        device_lib: &PathBuf,
        simulator_lib: &PathBuf,
        output: &PathBuf,
        is_shared: bool,
        lib_name: &str,
    ) -> Result<()> {
        // Remove existing xcframework
        if output.exists() {
            std::fs::remove_dir_all(output)?;
        }

        // Build xcodebuild command
        let mut cmd = std::process::Command::new("xcodebuild");
        cmd.arg("-create-xcframework");

        // Add device and simulator library paths
        let extension = if is_shared { "dylib" } else { "a" };

        // Main library filename to look for
        let main_lib_name = format!("lib{}.{}", lib_name, extension);

        // Find device library
        let device_lib_dir = device_lib.join("out");
        let device_main_lib = device_lib_dir.join(&main_lib_name);
        if device_main_lib.exists() {
            cmd.arg("-library").arg(&device_main_lib);
        } else {
            bail!(
                "Device library not found: {}",
                device_main_lib.display()
            );
        }

        // Find simulator library
        let sim_lib_dir = simulator_lib.join("out");
        let sim_main_lib = sim_lib_dir.join(&main_lib_name);
        if sim_main_lib.exists() {
            cmd.arg("-library").arg(&sim_main_lib);
        } else {
            bail!(
                "Simulator library not found: {}",
                sim_main_lib.display()
            );
        }

        cmd.arg("-output").arg(output);

        let status = cmd.status().context("Failed to run xcodebuild")?;

        if !status.success() {
            bail!("xcodebuild -create-xcframework failed");
        }

        Ok(())
    }

    /// Build a specific link type for device and simulator
    fn build_link_type(
        &mut self,
        ctx: &BuildContext,
        link_type: &str,
        architectures: &[String],
    ) -> Result<(PathBuf, PathBuf)> {
        let xcode = XcodeToolchain::detect()?;

        if ctx.options.verbose {
            eprintln!("Building {} library for watchOS...", link_type);
        }

        // Separate device and simulator architectures
        let device_archs: Vec<&str> = architectures
            .iter()
            .filter(|a| a.contains("arm") && !a.contains("simulator"))
            .map(|s| s.as_str())
            .collect();

        let sim_archs: Vec<&str> = architectures
            .iter()
            .filter(|a| a.contains("simulator") || a.as_str() == "x86_64")
            .map(|s| s.as_str())
            .collect();

        if device_archs.is_empty() {
            bail!("No device architectures specified for watchOS");
        }

        if sim_archs.is_empty() {
            bail!("No simulator architectures specified for watchOS");
        }

        // Build device architecture (arm64_32 or armv7k)
        let device_arch = if device_archs.contains(&"arm64_32") {
            "arm64_32"
        } else {
            "armv7k"
        };
        let device_dir = self.build_arch(ctx, &xcode, device_arch, link_type, "device")?;

        // Build simulator architecture (x86_64 or arm64)
        let sim_arch = if sim_archs.contains(&"arm64-simulator") {
            "arm64"
        } else {
            "x86_64"
        };
        let simulator_dir = self.build_arch(ctx, &xcode, sim_arch, link_type, "simulator")?;

        Ok((device_dir, simulator_dir))
    }
}

impl PlatformBuilder for WatchosBuilder {
    fn platform_name(&self) -> &str {
        "watchos"
    }

    fn default_architectures(&self) -> Vec<String> {
        // watchOS Simulator only supports arm64 (Apple Silicon) since Xcode 14
        vec!["arm64_32".to_string(), "arm64-simulator".to_string()]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        // Check for CMake
        if !crate::build::cmake::is_cmake_available() {
            bail!("CMake is required for watchOS builds. Please install CMake.");
        }

        // Check for Xcode
        let xcode = XcodeToolchain::detect()
            .context("Xcode is required for watchOS builds. Please install Xcode.")?;

        xcode.validate()?;

        if ctx.options.verbose {
            eprintln!(
                "Using Xcode {} (build {})",
                xcode.version(),
                xcode.build_version()
            );
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Create a mutable copy for building
        let mut builder = WatchosBuilder::new();

        // Validate prerequisites first
        builder.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for watchOS...", ctx.lib_name());
        }

        // Determine architectures to build
        let architectures = if ctx.options.architectures.is_empty() {
            self.default_architectures()
        } else {
            ctx.options.architectures.clone()
        };

        // Create output directory
        std::fs::create_dir_all(&ctx.output_dir)?;

        // Create archive builder
        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "watchos",
            ctx.output_dir.clone(),
        )?;

        let mut built_link_types = Vec::new();

        // Build static libraries and create XCFramework
        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let (device_dir, sim_dir) = builder.build_link_type(ctx, "static", &architectures)?;

            // Create XCFramework
            let xcframework_path = ctx.cmake_build_dir.join("static/xcframework");
            let xcframework = xcframework_path.join(format!("{}.xcframework", ctx.lib_name()));
            builder.create_xcframework(
                &XcodeToolchain::detect()?,
                &device_dir,
                &sim_dir,
                &xcframework,
                false,
                ctx.lib_name(),
            )?;

            // Add to archive: frameworks/watchos/static/{lib_name}.xcframework
            if xcframework.exists() {
                let archive_path = format!(
                    "{}/{}/{}/{}.xcframework",
                    ARCHIVE_DIR_FRAMEWORKS,
                    self.platform_name(),
                    ARCHIVE_DIR_STATIC,
                    ctx.lib_name()
                );
                archive.add_directory(&xcframework, &archive_path)?;
            }
            built_link_types.push("static");
        }

        // Build shared libraries and create XCFramework
        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            let (device_dir, sim_dir) = builder.build_link_type(ctx, "shared", &architectures)?;

            // Create XCFramework
            let xcframework_path = ctx.cmake_build_dir.join("shared/xcframework");
            let xcframework = xcframework_path.join(format!("{}.xcframework", ctx.lib_name()));
            builder.create_xcframework(
                &XcodeToolchain::detect()?,
                &device_dir,
                &sim_dir,
                &xcframework,
                true,
                ctx.lib_name(),
            )?;

            // Add to archive: frameworks/watchos/shared/{lib_name}.xcframework
            if xcframework.exists() {
                let archive_path = format!(
                    "{}/{}/{}/{}.xcframework",
                    ARCHIVE_DIR_FRAMEWORKS,
                    self.platform_name(),
                    ARCHIVE_DIR_SHARED,
                    ctx.lib_name()
                );
                archive.add_directory(&xcframework, &archive_path)?;
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
        let sdk_archive = archive.create_sdk_archive(&architectures, &link_type_str)?;

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "watchOS build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: None,
            aar_archive: None,
            duration_secs: duration.as_secs_f64(),
            architectures,
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean new directory structure: cmake_build/{release|debug}/watchos
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("watchos");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/watchOS, cmake_build/watchos
        for old_dir in &[
            ctx.project_root.join("cmake_build/watchOS"),
            ctx.project_root.join("cmake_build/watchos"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/watchos"),
            ctx.project_root.join("target/debug/watchos"),
            ctx.project_root.join("target/release/watchOS"),
            ctx.project_root.join("target/debug/watchOS"),
            ctx.project_root.join("target/watchos"),
            ctx.project_root.join("target/watchOS"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for WatchosBuilder {
    fn default() -> Self {
        Self::new()
    }
}
