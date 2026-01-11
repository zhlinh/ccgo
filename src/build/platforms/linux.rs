//! Linux platform builder
//!
//! Builds static and shared libraries for Linux using CMake with GCC or Clang.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{
    get_unified_include_path, ArchiveBuilder, ARCHIVE_DIR_OBJ, ARCHIVE_DIR_SHARED,
    ARCHIVE_DIR_STATIC,
};
use crate::build::cmake::{BuildType, CMakeConfig};
#[cfg(target_os = "linux")]
use crate::build::toolchains::detect_default_compiler;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// Linux platform builder
pub struct LinuxBuilder;

impl LinuxBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Merge all module static libraries into a single library
    /// This is essential for KMP cinterop which expects a single complete library
    fn merge_module_static_libs(
        &self,
        build_dir: &PathBuf,
        lib_name: &str,
        verbose: bool,
    ) -> Result<()> {
        use crate::build::toolchains::linux::LinuxToolchain;

        // Find the output directory where CMake puts libraries
        let out_dir = build_dir.join("out");
        if verbose {
            eprintln!("    [merge] Checking out directory: {}", out_dir.display());
        }
        if !out_dir.exists() {
            if verbose {
                eprintln!("    [merge] Out directory does not exist, skipping merge");
            }
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
                        eprintln!("    [merge] Main library {} already exists, skipping merge", main_lib_name);
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

        if verbose {
            eprintln!("    [merge] Found {} .a files:", module_libs.len());
            for lib in &module_libs {
                eprintln!("      - {}", lib.file_name().unwrap().to_string_lossy());
            }
        }

        if module_libs.is_empty() {
            if verbose {
                eprintln!("    [merge] No libraries found, skipping merge");
            }
            return Ok(());
        }

        // Check if we only have the main library (already merged or single module)
        if module_libs.len() == 1 && module_libs[0].file_name().map_or(false, |n| n == main_lib_name.as_str()) {
            if verbose {
                eprintln!("    [merge] Only main library exists, skipping merge");
            }
            return Ok(());
        }

        // Filter out the main library if it exists (we'll recreate it)
        module_libs.retain(|p| p != &main_lib_path);

        if verbose {
            eprintln!("    [merge] Module libraries to merge: {}", module_libs.len());
            for lib in &module_libs {
                eprintln!("      - {}", lib.file_name().unwrap().to_string_lossy());
            }
        }

        if module_libs.is_empty() {
            if verbose {
                eprintln!("    [merge] No module libraries after filtering, skipping merge");
            }
            return Ok(());
        }

        eprintln!(
            "    Merging {} module libraries into {}",
            module_libs.len(),
            main_lib_name
        );

        // Merge all module libraries into the main library
        let toolchain = LinuxToolchain::detect()?;
        toolchain.merge_static_libs(&module_libs, &main_lib_path)?;

        // Clean up module libraries after merge
        eprintln!("    Cleaning up {} module libraries...", module_libs.len());
        for lib in &module_libs {
            if lib != &main_lib_path {
                match std::fs::remove_file(lib) {
                    Ok(_) => {
                        if verbose {
                            eprintln!("      ✓ Removed: {}", lib.file_name().unwrap().to_string_lossy());
                        }
                    }
                    Err(e) => {
                        eprintln!("      ⚠ Failed to remove {}: {}", lib.file_name().unwrap().to_string_lossy(), e);
                    }
                }
            }
        }

        Ok(())
    }

    /// Build a specific link type (static or shared)
    /// Returns the build directory where output is located
    fn build_link_type(&self, ctx: &BuildContext, link_type: &str) -> Result<PathBuf> {
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

        // For static builds, merge all module libraries into a single library
        // This is essential for KMP cinterop which expects a single complete library
        if !build_shared {
            self.merge_module_static_libs(&build_dir, ctx.lib_name(), ctx.options.verbose)?;
        }

        // Return build_dir since CCGO cmake installs to build_dir/out/
        Ok(build_dir)
    }

    /// Find library directory in build output
    /// Prioritizes out/ directory where CCGO cmake puts the merged library
    fn find_lib_dir(&self, build_dir: &PathBuf) -> Option<PathBuf> {
        // CCGO cmake puts the combined/merged library in out/
        // e.g., static/out/libccgonow.a or shared/out/libccgonow.so
        // This is the preferred location as it contains only the final merged library
        let possible_dirs = vec![
            build_dir.join("out"),           // Merged library (priority)
            build_dir.join("install/lib"),   // Fallback: CMake install location
            build_dir.join("lib"),
        ];

        for dir in possible_dirs {
            if dir.exists() {
                return Some(dir);
            }
        }
        None
    }
}

impl PlatformBuilder for LinuxBuilder {
    fn platform_name(&self) -> &str {
        "linux"
    }

    fn default_architectures(&self) -> Vec<String> {
        vec!["x86_64".to_string()]
    }

    fn validate_prerequisites(&self, _ctx: &BuildContext) -> Result<()> {
        // Check if we're on Linux
        #[cfg(not(target_os = "linux"))]
        {
            bail!(
                "Linux builds can only be run on Linux systems.\n\
                 Current OS: {}\n\n\
                 To build for Linux from your current OS, use Docker:\n  \
                 ccgo build linux --docker",
                std::env::consts::OS
            );
        }

        #[cfg(target_os = "linux")]
        {
            // Check for CMake
            if !crate::build::cmake::is_cmake_available() {
                bail!("CMake is required for Linux builds. Please install CMake.");
            }

            // Check for C++ compiler
            let compiler = detect_default_compiler()
                .context("No C/C++ compiler found. Please install GCC or Clang.")?;

            if _ctx.options.verbose {
                eprintln!(
                    "Using {} compiler: {} ({})",
                    compiler.compiler_type,
                    compiler.cxx.display(),
                    compiler.version
                );
            }

            Ok(())
        }
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Validate prerequisites first
        self.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for Linux...", ctx.lib_name());
        }

        // Create output directory
        std::fs::create_dir_all(&ctx.output_dir)?;

        // Create archive builder
        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "linux",
            ctx.output_dir.clone(),
        )?;

        let mut built_link_types = Vec::new();

        // Build static libraries
        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            if ctx.options.verbose {
                eprintln!("Building static library...");
            }
            let build_dir = self.build_link_type(ctx, "static")?;

            // Add static library to archive: lib/linux/static/
            // find_lib_dir prioritizes out/ which contains only the merged library
            if let Some(lib_dir) = self.find_lib_dir(&build_dir) {
                let archive_path = format!("lib/{}/{}", self.platform_name(), ARCHIVE_DIR_STATIC);
                archive.add_directory_filtered(&lib_dir, &archive_path, &["a"])?;
            }
            built_link_types.push("static");
        }

        // Build shared libraries
        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            if ctx.options.verbose {
                eprintln!("Building shared library...");
            }
            let build_dir = self.build_link_type(ctx, "shared")?;

            // Add shared library to archive: lib/linux/shared/
            // find_lib_dir prioritizes out/ which contains only the merged library
            if let Some(lib_dir) = self.find_lib_dir(&build_dir) {
                let archive_path = format!("lib/{}/{}", self.platform_name(), ARCHIVE_DIR_SHARED);
                archive.add_directory_filtered(&lib_dir, &archive_path, &["so", "a"])?;
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
        let sdk_archive = archive.create_sdk_archive(&["x86_64".to_string()], &link_type_str)?;

        // Create symbols archive with unstripped binaries
        // Structure: obj/linux/x86_64/*.so (for shared libs)
        let mut symbols_archive_result = None;
        if ctx.options.link_type != LinkType::Static {
            // For shared builds, collect unstripped .so files
            let symbols_temp = std::env::temp_dir().join(format!("ccgo-symbols-{}", ctx.lib_name()));
            std::fs::create_dir_all(&symbols_temp)?;

            // Create obj/linux/x86_64/ structure
            let obj_arch_dir = symbols_temp
                .join(ARCHIVE_DIR_OBJ)
                .join(self.platform_name())
                .join("x86_64");
            std::fs::create_dir_all(&obj_arch_dir)?;

            // Find and copy unstripped .so files from build directory
            let shared_build_dir = ctx.cmake_build_dir.join("shared");
            if let Some(lib_dir) = self.find_lib_dir(&shared_build_dir) {
                for entry in std::fs::read_dir(&lib_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().map_or(false, |ext| ext == "so") {
                        let file_name = path.file_name().unwrap();
                        std::fs::copy(&path, obj_arch_dir.join(file_name))?;
                    }
                }

                // Only create symbols archive if we found .so files
                if obj_arch_dir.read_dir()?.next().is_some() {
                    let symbols_archive_path = archive.create_symbols_archive(&symbols_temp)?;
                    symbols_archive_result = Some(symbols_archive_path);
                }
            }

            // Clean up temp directory
            if symbols_temp.exists() {
                std::fs::remove_dir_all(&symbols_temp)?;
            }
        }

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "Linux build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
            if let Some(ref sym) = symbols_archive_result {
                eprintln!("  Symbols archive: {}", sym.display());
            }
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: symbols_archive_result,
            aar_archive: None,
            duration_secs: duration.as_secs_f64(),
            architectures: vec!["x86_64".to_string()],
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean new directory structure: cmake_build/{release|debug}/linux
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("linux");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/Linux, cmake_build/linux
        for old_dir in &[
            ctx.project_root.join("cmake_build/Linux"),
            ctx.project_root.join("cmake_build/linux"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/linux"),
            ctx.project_root.join("target/debug/linux"),
            ctx.project_root.join("target/release/Linux"),
            ctx.project_root.join("target/debug/Linux"),
            ctx.project_root.join("target/linux"),
            ctx.project_root.join("target/Linux"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for LinuxBuilder {
    fn default() -> Self {
        Self::new()
    }
}
