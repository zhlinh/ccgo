//! macOS platform builder
//!
//! Builds static and dynamic frameworks for macOS using CMake with Clang.
//! Supports universal binaries (x86_64 + arm64) via lipo.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{
    get_unified_include_path, ArchiveBuilder, ARCHIVE_DIR_FRAMEWORKS, ARCHIVE_DIR_SHARED,
    ARCHIVE_DIR_STATIC,
};
use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::toolchains::xcode::{ApplePlatform, XcodeToolchain};
#[cfg(target_os = "macos")]
use crate::build::toolchains::Toolchain;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// macOS platform builder
pub struct MacosBuilder {
    /// Xcode toolchain (lazily initialized)
    xcode: Option<XcodeToolchain>,
}

impl MacosBuilder {
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
    /// Returns the build directory where output is located (not install_dir, since CCGO cmake uses "out/")
    fn build_arch(
        &self,
        ctx: &BuildContext,
        xcode: &XcodeToolchain,
        arch: &str,
        link_type: &str,
    ) -> Result<PathBuf> {
        let build_dir = ctx
            .cmake_build_dir
            .join(format!("{}/{}", link_type, arch));
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";

        // Get macOS SDK path and CMake variables
        let cmake_vars = xcode.cmake_variables_for_platform(ApplePlatform::MacOS)?;

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
                // Skip archs, we set it above
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

        // Return build_dir since CCGO cmake installs to build_dir/out/
        Ok(build_dir)
    }

    /// Create universal binary from multiple architectures using lipo
    fn create_universal_binary(
        &self,
        xcode: &XcodeToolchain,
        arch_libs: &[(String, PathBuf)], // (arch, lib_path)
        output: &PathBuf,
    ) -> Result<()> {
        if arch_libs.len() == 1 {
            // Only one architecture, just copy
            std::fs::copy(&arch_libs[0].1, output)?;
            return Ok(());
        }

        let lib_paths: Vec<PathBuf> = arch_libs.iter().map(|(_, p)| p.clone()).collect();
        xcode.create_universal_binary(&lib_paths, output)?;

        Ok(())
    }

    /// Find library files in install directory
    /// Checks multiple possible locations: lib/, out/, and root
    fn find_libraries(&self, install_dir: &PathBuf, is_shared: bool) -> Result<Vec<PathBuf>> {
        let extension = if is_shared { "dylib" } else { "a" };
        let mut libs = Vec::new();

        // Check multiple possible directories (CCGO cmake uses "out/")
        let possible_dirs = vec![
            install_dir.join("lib"),
            install_dir.join("out"),
            install_dir.to_path_buf(),
        ];

        for lib_dir in possible_dirs {
            if !lib_dir.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&lib_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            // Avoid duplicates
                            if !libs.iter().any(|p: &PathBuf| p.file_name() == path.file_name()) {
                                libs.push(path);
                            }
                        }
                    }
                }
            }

            // If we found libraries, stop searching
            if !libs.is_empty() {
                break;
            }
        }

        Ok(libs)
    }

    /// Build a specific link type for all architectures
    fn build_link_type(
        &mut self,
        ctx: &BuildContext,
        link_type: &str,
        architectures: &[String],
    ) -> Result<PathBuf> {
        let xcode = XcodeToolchain::detect()?;

        if ctx.options.verbose {
            eprintln!("Building {} library for macOS...", link_type);
        }

        let is_shared = link_type == "shared";

        // Build each architecture
        let mut arch_results: Vec<(String, PathBuf)> = Vec::new();
        for arch in architectures {
            if ctx.options.verbose {
                eprintln!("  Building for {}...", arch);
            }
            let install_dir = self.build_arch(ctx, &xcode, arch, link_type)?;
            arch_results.push((arch.clone(), install_dir));
        }

        // Create universal output directory
        let universal_dir = ctx.cmake_build_dir.join(format!("{}/universal", link_type));
        let universal_lib_dir = universal_dir.join("lib");
        std::fs::create_dir_all(&universal_lib_dir)?;

        // Find and merge libraries for each architecture
        let first_install = &arch_results[0].1;
        let libs = self.find_libraries(first_install, is_shared)?;

        for lib in &libs {
            let lib_name = lib.file_name().unwrap().to_str().unwrap();
            let output_path = universal_lib_dir.join(lib_name);

            // Collect the same library from each architecture
            let mut arch_libs: Vec<(String, PathBuf)> = Vec::new();
            for (arch, install_dir) in &arch_results {
                // Check multiple possible locations
                let possible_paths = vec![
                    install_dir.join("lib").join(lib_name),
                    install_dir.join("out").join(lib_name),
                    install_dir.join(lib_name),
                ];
                for arch_lib in possible_paths {
                    if arch_lib.exists() {
                        arch_libs.push((arch.clone(), arch_lib));
                        break;
                    }
                }
            }

            if !arch_libs.is_empty() {
                self.create_universal_binary(&xcode, &arch_libs, &output_path)?;
            }
        }

        // Copy include files from first architecture
        let include_src = first_install.join("include");
        let include_dst = universal_dir.join("include");
        if include_src.exists() {
            copy_dir_all(&include_src, &include_dst)?;
        }

        Ok(universal_dir)
    }

    /// Find library directory, checking multiple possible locations
    fn find_lib_dir(&self, build_dir: &PathBuf) -> Option<PathBuf> {
        let possible_dirs = vec![
            build_dir.join("lib"),
            build_dir.join("out"),
            build_dir.to_path_buf(),
        ];

        for dir in possible_dirs {
            if dir.exists() && std::fs::read_dir(&dir).map(|d| d.count() > 0).unwrap_or(false) {
                return Some(dir);
            }
        }
        None
    }

    /// Create XCFramework from universal library
    fn create_xcframework(
        &self,
        xcode: &XcodeToolchain,
        universal_dir: &PathBuf,
        output: &PathBuf,
        is_shared: bool,
        lib_name: &str,
    ) -> Result<()> {
        // Remove existing XCFramework if present
        if output.exists() {
            std::fs::remove_dir_all(output)?;
        }

        let extension = if is_shared { "dylib" } else { "a" };

        // Main library filename to look for (e.g., "libccgonow.dylib" or "libccgonow.a")
        let main_lib_name = format!("lib{}.{}", lib_name, extension);

        // Find the library directory (check multiple possible locations)
        let lib_dir = self.find_lib_dir(universal_dir)
            .ok_or_else(|| anyhow::anyhow!("Universal library directory not found"))?;

        // Find main library - prefer exact name match
        let main_lib = lib_dir.join(&main_lib_name);
        if !main_lib.exists() {
            // Fallback: find first library with matching extension
            let mut found = false;
            for entry in std::fs::read_dir(&lib_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == extension {
                            // Use first matching library
                            xcode.create_xcframework(&[(path, None)], output)?;
                            found = true;
                            break;
                        }
                    }
                }
            }
            if !found {
                bail!("Main library {} not found in {}", main_lib_name, lib_dir.display());
            }
        } else {
            // Create XCFramework with main library
            xcode.create_xcframework(&[(main_lib, None)], output)?;
        }

        Ok(())
    }
}

impl PlatformBuilder for MacosBuilder {
    fn platform_name(&self) -> &str {
        "macos"
    }

    fn default_architectures(&self) -> Vec<String> {
        vec!["x86_64".to_string(), "arm64".to_string()]
    }

    fn validate_prerequisites(&self, _ctx: &BuildContext) -> Result<()> {
        // Check if we're on macOS - bail on other platforms
        #[cfg(not(target_os = "macos"))]
        {
            bail!(
                "macOS builds can only be run on macOS systems.\n\
                 Current OS: {}\n\n\
                 To build for macOS from your current OS, use Docker:\n  \
                 ccgo build macos --docker",
                std::env::consts::OS
            );
        }

        // All following checks only run on macOS
        #[cfg(target_os = "macos")]
        {
            // Check for CMake
            if !crate::build::cmake::is_cmake_available() {
                bail!("CMake is required for macOS builds. Please install CMake.");
            }

            // Check for Xcode
            let xcode = XcodeToolchain::detect()
                .context("Xcode is required for macOS builds. Please install Xcode.")?;

            xcode.validate()?;

            if _ctx.options.verbose {
                eprintln!(
                    "Using Xcode {} (build {})",
                    xcode.version(),
                    xcode.build_version()
                );
            }

            Ok(())
        }
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Create a mutable copy for building
        let mut builder = MacosBuilder::new();

        // Validate prerequisites first
        builder.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for macOS...", ctx.lib_name());
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
            "macos",
            ctx.output_dir.clone(),
        )?;

        let mut built_link_types = Vec::new();

        // Build static libraries and create XCFramework
        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let universal_dir = builder.build_link_type(ctx, "static", &architectures)?;

            // Get Xcode for XCFramework creation
            let xcode = XcodeToolchain::detect()?;

            // Create XCFramework
            let xcframework_path = ctx.cmake_build_dir.join("static/xcframework");
            let xcframework = xcframework_path.join(format!("{}.xcframework", ctx.lib_name()));
            builder.create_xcframework(&xcode, &universal_dir, &xcframework, false, ctx.lib_name())?;

            // Add to archive: frameworks/macos/static/{lib_name}.xcframework
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
            let universal_dir = builder.build_link_type(ctx, "shared", &architectures)?;

            // Get Xcode for XCFramework creation
            let xcode = XcodeToolchain::detect()?;

            // Create XCFramework
            let xcframework_path = ctx.cmake_build_dir.join("shared/xcframework");
            let xcframework = xcframework_path.join(format!("{}.xcframework", ctx.lib_name()));
            builder.create_xcframework(&xcode, &universal_dir, &xcframework, true, ctx.lib_name())?;

            // Add to archive: frameworks/macos/shared/{lib_name}.xcframework
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
                "macOS build completed in {:.2}s: {}",
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
        // Clean new directory structure: cmake_build/{release|debug}/macos
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("macos");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/macOS, cmake_build/macos
        for old_dir in &[
            ctx.project_root.join("cmake_build/macOS"),
            ctx.project_root.join("cmake_build/macos"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/macos"),
            ctx.project_root.join("target/debug/macos"),
            ctx.project_root.join("target/release/macOS"),
            ctx.project_root.join("target/debug/macOS"),
            ctx.project_root.join("target/macos"),
            ctx.project_root.join("target/macOS"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for MacosBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Copy a directory recursively
fn copy_dir_all(src: &PathBuf, dst: &PathBuf) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}
