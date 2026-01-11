//! Windows platform builder
//!
//! Builds static and dynamic libraries for Windows using CMake with MinGW or MSVC.
//! Supports cross-compilation from macOS/Linux using MinGW-w64.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{get_unified_include_path, ArchiveBuilder};
use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::toolchains::mingw::{is_mingw_available, MingwToolchain};
use crate::build::toolchains::msvc::is_msvc_available;
#[cfg(target_os = "windows")]
use crate::build::toolchains::msvc::MsvcToolchain;
use crate::build::toolchains::Toolchain;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// Windows toolchain type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowsToolchain {
    /// MinGW-w64 (cross-compilation)
    MinGW,
    /// Microsoft Visual C++ (Windows only)
    MSVC,
}

impl WindowsToolchain {
    /// Get the toolchain name for archive paths
    pub fn name(&self) -> &str {
        match self {
            WindowsToolchain::MinGW => "mingw",
            WindowsToolchain::MSVC => "msvc",
        }
    }
}

/// Windows platform builder
pub struct WindowsBuilder;

impl WindowsBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Merge all module static libraries into a single library (MinGW)
    /// This is essential for KMP cinterop which expects a single complete library
    fn merge_module_static_libs_mingw(
        &self,
        mingw: &MingwToolchain,
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

        // Check if the main library already exists (CMake may have already merged it)
        let main_lib_name = format!("lib{}.a", lib_name);
        let main_lib_path = out_dir.join(&main_lib_name);

        if main_lib_path.exists() {
            // Check if it's a non-empty file (CMake already created the merged library)
            if let Ok(metadata) = std::fs::metadata(&main_lib_path) {
                if metadata.len() > 0 {
                    if verbose {
                        eprintln!("    Main library {} already exists, skipping merge", main_lib_name);
                    }
                    // Clean up module libraries (keep output directory clean)
                    for entry in std::fs::read_dir(&out_dir)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_file() && path != main_lib_path {
                            if let Some(ext) = path.extension() {
                                if ext == "a" {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                    return Ok(());
                }
            }
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
        if module_libs.len() == 1 && module_libs[0].file_name().map_or(false, |n| n == main_lib_name.as_str()) {
            // Already a single main library, nothing to merge
            return Ok(());
        }

        // Filter out the main library if it exists (we'll recreate it)
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
        mingw.merge_static_libs(&module_libs, &main_lib_path)?;

        // Clean up module libraries after merge (optional, keeps output clean)
        for lib in &module_libs {
            if lib != &main_lib_path {
                let _ = std::fs::remove_file(lib);
            }
        }

        Ok(())
    }

    /// Detect available Windows toolchain
    fn detect_toolchain() -> Result<WindowsToolchain> {
        // Prefer MinGW for cross-platform builds
        if is_mingw_available() {
            return Ok(WindowsToolchain::MinGW);
        }

        // Fall back to MSVC on Windows
        if is_msvc_available() {
            return Ok(WindowsToolchain::MSVC);
        }

        bail!(
            "No Windows toolchain found.\n\
             - For cross-compilation: Install MinGW-w64 (x86_64-w64-mingw32-gcc)\n\
             - For native builds: Install Visual Studio with C++ tools"
        )
    }

    /// Build for a specific link type with MinGW
    fn build_with_mingw(
        &self,
        ctx: &BuildContext,
        mingw: &MingwToolchain,
        link_type: &str,
    ) -> Result<PathBuf> {
        let build_dir = ctx
            .cmake_build_dir
            .join(format!("{}/mingw", link_type));
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";

        // Get MinGW CMake variables
        let cmake_vars = mingw.cmake_variables_for_arch();

        // Configure and build with CMake
        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .generator("Unix Makefiles")
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

        // Add MinGW-specific variables
        for (name, value) in cmake_vars {
            cmake = cmake.variable(&name, &value);
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
            self.merge_module_static_libs_mingw(mingw, &build_dir, ctx.lib_name(), ctx.options.verbose)?;
        }

        Ok(build_dir)
    }

    /// Build for a specific link type with MSVC
    #[cfg(target_os = "windows")]
    fn build_with_msvc(
        &self,
        ctx: &BuildContext,
        msvc: &MsvcToolchain,
        link_type: &str,
    ) -> Result<PathBuf> {
        let build_dir = ctx
            .cmake_build_dir
            .join(format!("{}/msvc", link_type));
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";

        // Configure and build with CMake using Visual Studio generator
        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .generator(msvc.cmake_generator())
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
            .variable("CMAKE_GENERATOR_PLATFORM", "x64")
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

    /// Find library files in build directory
    fn find_libraries(
        &self,
        build_dir: &PathBuf,
        is_shared: bool,
        toolchain: WindowsToolchain,
    ) -> Result<Vec<PathBuf>> {
        let (static_ext, shared_ext) = match toolchain {
            WindowsToolchain::MinGW => ("a", "dll"),
            WindowsToolchain::MSVC => ("lib", "dll"),
        };

        let extension = if is_shared { shared_ext } else { static_ext };
        let mut libs = Vec::new();

        // Check multiple possible directories
        // Prioritize out/ directory where CCGO cmake puts the merged library
        // This avoids including intermediate module libs (e.g., lib{name}-api.a)
        let possible_dirs = vec![
            build_dir.join("out"),           // Merged library (priority)
            build_dir.join("install/lib"),   // Fallback: CMake install location
            build_dir.join("lib"),
            build_dir.join("bin"),           // DLLs often go to bin/
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
                            if !libs
                                .iter()
                                .any(|p: &PathBuf| p.file_name() == path.file_name())
                            {
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

        // For shared libraries, also look for import libraries (.dll.a or .lib)
        if is_shared {
            let import_ext = match toolchain {
                WindowsToolchain::MinGW => "dll.a",
                WindowsToolchain::MSVC => "lib",
            };

            // Prioritize out/ directory for import libraries as well
            for lib_dir in &[
                build_dir.join("out"),
                build_dir.join("install/lib"),
                build_dir.join("lib"),
            ] {
                if !lib_dir.exists() {
                    continue;
                }

                for entry in std::fs::read_dir(lib_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let name = path.file_name().unwrap().to_str().unwrap();
                        if name.ends_with(import_ext) {
                            if !libs.iter().any(|p: &PathBuf| p.file_name() == path.file_name()) {
                                libs.push(path);
                            }
                        }
                    }
                }
            }
        }

        Ok(libs)
    }

    /// Build for a specific link type
    fn build_link_type(
        &self,
        ctx: &BuildContext,
        link_type: &str,
        toolchain: WindowsToolchain,
    ) -> Result<PathBuf> {
        if ctx.options.verbose {
            eprintln!(
                "Building {} library for Windows ({})...",
                link_type,
                toolchain.name()
            );
        }

        match toolchain {
            WindowsToolchain::MinGW => {
                let mingw = MingwToolchain::detect()?;
                self.build_with_mingw(ctx, &mingw, link_type)
            }
            #[cfg(target_os = "windows")]
            WindowsToolchain::MSVC => {
                let msvc = MsvcToolchain::detect()?;
                self.build_with_msvc(ctx, &msvc, link_type)
            }
            #[cfg(not(target_os = "windows"))]
            WindowsToolchain::MSVC => {
                bail!("MSVC builds are only available on Windows")
            }
        }
    }

    /// Add libraries to archive with toolchain-specific paths
    fn add_libraries_to_archive(
        &self,
        archive: &ArchiveBuilder,
        build_dir: &PathBuf,
        link_type: &str,
        is_shared: bool,
        toolchain: WindowsToolchain,
    ) -> Result<()> {
        let libs = self.find_libraries(build_dir, is_shared, toolchain)?;

        for lib in &libs {
            let lib_name = lib.file_name().unwrap().to_str().unwrap();
            // Archive path: lib/windows/{static|shared}/{toolchain}/{lib_name}
            let dest = format!(
                "lib/{}/{}/{}/{}",
                self.platform_name(),
                link_type,
                toolchain.name(),
                lib_name
            );
            archive.add_file(lib, &dest)?;
        }

        Ok(())
    }

    /// Strip shared libraries (MinGW only)
    fn strip_libraries(
        &self,
        mingw: &MingwToolchain,
        build_dir: &PathBuf,
        verbose: bool,
    ) -> Result<()> {
        let strip_path = mingw.strip_path();
        let libs = self.find_libraries(build_dir, true, WindowsToolchain::MinGW)?;

        for lib in libs {
            // Only strip DLLs
            if let Some(ext) = lib.extension() {
                if ext == "dll" {
                    if verbose {
                        eprintln!("  Stripping {}...", lib.display());
                    }

                    let status = std::process::Command::new(&strip_path)
                        .arg("--strip-unneeded")
                        .arg(&lib)
                        .status()
                        .with_context(|| format!("Failed to strip {}", lib.display()))?;

                    if !status.success() && verbose {
                        eprintln!("Warning: Failed to strip {}", lib.display());
                    }
                }
            }
        }

        Ok(())
    }
}

impl PlatformBuilder for WindowsBuilder {
    fn platform_name(&self) -> &str {
        "windows"
    }

    fn default_architectures(&self) -> Vec<String> {
        vec!["x86_64".to_string()]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        // Check for CMake
        if !crate::build::cmake::is_cmake_available() {
            bail!("CMake is required for Windows builds. Please install CMake.");
        }

        // Check for a Windows toolchain
        let toolchain = Self::detect_toolchain()?;

        match toolchain {
            WindowsToolchain::MinGW => {
                let mingw = MingwToolchain::detect()?;
                mingw.validate()?;

                if ctx.options.verbose {
                    eprintln!(
                        "Using MinGW-w64 {} at {}",
                        mingw.version(),
                        mingw.path().unwrap().display()
                    );
                }
            }
            WindowsToolchain::MSVC => {
                #[cfg(target_os = "windows")]
                {
                    let msvc = MsvcToolchain::detect()?;
                    msvc.validate()?;

                    if ctx.options.verbose {
                        eprintln!(
                            "Using MSVC {} at {}",
                            msvc.version(),
                            msvc.path().unwrap().display()
                        );
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    bail!("MSVC is only available on Windows");
                }
            }
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Validate prerequisites first
        self.validate_prerequisites(ctx)?;

        let toolchain = Self::detect_toolchain()?;

        if ctx.options.verbose {
            eprintln!("Building {} for Windows...", ctx.lib_name());
        }

        // Create output directory
        std::fs::create_dir_all(&ctx.output_dir)?;

        // Create archive builder
        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "windows",
            ctx.output_dir.clone(),
        )?;

        let mut built_link_types = Vec::new();

        // Build static libraries
        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let build_dir = self.build_link_type(ctx, "static", toolchain)?;
            self.add_libraries_to_archive(&archive, &build_dir, "static", false, toolchain)?;
            built_link_types.push("static");
        }

        // Build shared libraries
        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            let build_dir = self.build_link_type(ctx, "shared", toolchain)?;

            // Strip shared libraries for release builds (MinGW only)
            if ctx.options.release && toolchain == WindowsToolchain::MinGW {
                if ctx.options.verbose {
                    eprintln!("Stripping shared libraries...");
                }
                let mingw = MingwToolchain::detect()?;
                self.strip_libraries(&mingw, &build_dir, ctx.options.verbose)?;
            }

            self.add_libraries_to_archive(&archive, &build_dir, "shared", true, toolchain)?;
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
        let architectures = vec!["x86_64".to_string()];
        let link_type_str = ctx.options.link_type.to_string();
        let sdk_archive = archive.create_sdk_archive(&architectures, &link_type_str)?;

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "Windows build completed in {:.2}s: {}",
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
        // Clean new directory structure: cmake_build/{release|debug}/windows
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("windows");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/Windows, cmake_build/windows
        for old_dir in &[
            ctx.project_root.join("cmake_build/Windows"),
            ctx.project_root.join("cmake_build/windows"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/windows"),
            ctx.project_root.join("target/debug/windows"),
            ctx.project_root.join("target/release/Windows"),
            ctx.project_root.join("target/debug/Windows"),
            ctx.project_root.join("target/windows"),
            ctx.project_root.join("target/Windows"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for WindowsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
