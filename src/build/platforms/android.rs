//! Android platform builder
//!
//! Builds native libraries and AAR packages for Android using CMake with NDK.
//! Supports multiple ABIs (arm64-v8a, armeabi-v7a, x86_64, x86).
//!
//! Archive structure matches Python pyccgo output:
//! - lib/{static|shared}/{arch}/ - stripped libraries
//! - symbols/android/obj/{arch}/ - unstripped libraries (in symbols archive)
//! - include/{lib_name}/ - header files

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{get_unified_include_path, ArchiveBuilder};
use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::toolchains::android_ndk::{AndroidAbi, AndroidNdkToolchain, DEFAULT_API_LEVEL};
use crate::build::toolchains::Toolchain;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// Android platform builder
pub struct AndroidBuilder;

impl AndroidBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Parse ABI string to AndroidAbi enum
    fn parse_abi(s: &str) -> Result<AndroidAbi> {
        AndroidAbi::from_str(s)
            .ok_or_else(|| anyhow::anyhow!("Invalid Android ABI: {}. Valid options: arm64-v8a, armeabi-v7a, x86_64, x86", s))
    }

    /// Build for a single ABI
    fn build_abi(
        &self,
        ctx: &BuildContext,
        ndk: &AndroidNdkToolchain,
        abi: AndroidAbi,
        link_type: &str,
        api_level: u32,
    ) -> Result<PathBuf> {
        let build_dir = ctx
            .cmake_build_dir
            .join(format!("{}/{}", link_type, abi.abi_string()));
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";

        // Get NDK CMake variables for this ABI
        let cmake_vars = ndk.cmake_variables_for_abi(abi, api_level);

        // Configure and build with CMake
        // For Release builds, use RelWithDebInfo to get debug symbols (-O2 -g -DNDEBUG)
        // This ensures SYMBOLS.zip contains unstripped libraries with full debug info
        let build_type = if ctx.options.release {
            BuildType::RelWithDebInfo
        } else {
            BuildType::Debug
        };

        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .build_type(build_type)
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

        // Add NDK-specific variables
        for (name, value) in cmake_vars {
            cmake = cmake.variable(&name, &value);
        }

        // Prevent CMake from stripping during install (we control stripping manually)
        if ctx.options.release {
            cmake = cmake
                .variable("CMAKE_SKIP_INSTALL_RPATH", "ON")
                .variable("CMAKE_BUILD_WITH_INSTALL_RPATH", "OFF")
                .variable("CMAKE_STRIP", "");
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

        // Return build_dir since CCGO cmake installs to build_dir/out/
        Ok(build_dir)
    }

    /// Find library files in install directory
    /// Checks multiple possible locations including combined library output
    fn find_libraries(&self, build_dir: &PathBuf, is_shared: bool, link_type: &str, abi: AndroidAbi) -> Result<Vec<PathBuf>> {
        let extension = if is_shared { "so" } else { "a" };
        let mut libs = Vec::new();

        // CCGO cmake puts the combined library in {link_type}/{arch}/out/
        // e.g., shared/arm64-v8a/out/libccgonow.so
        let combined_lib_dir = build_dir.join(format!("{}/{}/out", link_type, abi.abi_string()));

        // Check multiple possible directories
        let possible_dirs = vec![
            combined_lib_dir,  // Combined library (libccgonow.so) - PRIORITY
            build_dir.join("install/lib"),
            build_dir.join("out"),
            build_dir.join("lib"),
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

    /// Build a specific link type for all ABIs
    fn build_link_type(
        &self,
        ctx: &BuildContext,
        ndk: &AndroidNdkToolchain,
        link_type: &str,
        abis: &[AndroidAbi],
        api_level: u32,
    ) -> Result<Vec<(AndroidAbi, PathBuf)>> {
        if ctx.options.verbose {
            eprintln!("Building {} library for Android...", link_type);
        }

        let mut results = Vec::new();

        for abi in abis {
            if ctx.options.verbose {
                eprintln!("  Building for {}...", abi.abi_string());
            }

            // Validate API level for this ABI
            AndroidNdkToolchain::validate_api_level(*abi, api_level)?;

            let install_dir = self.build_abi(ctx, ndk, *abi, link_type, api_level)?;
            results.push((*abi, install_dir));
        }

        Ok(results)
    }

    /// Copy libraries to archive structure (per-ABI directories)
    ///
    /// Archive path: lib/android/{static|shared}/{arch}/{lib_name}
    fn add_libraries_to_archive(
        &self,
        archive: &ArchiveBuilder,
        abi_results: &[(AndroidAbi, PathBuf)],
        link_type: &str,
        is_shared: bool,
    ) -> Result<()> {
        for (abi, install_dir) in abi_results {
            let libs = self.find_libraries(install_dir, is_shared, link_type, *abi)?;

            for lib in &libs {
                let lib_name = lib.file_name().unwrap().to_str().unwrap();
                // Use lowercase "android" for archive paths to match Python ccgo
                let dest = format!("lib/android/{}/{}/{}", link_type, abi.abi_string(), lib_name);
                archive.add_file(lib, &dest)?;
            }
        }

        Ok(())
    }

    /// Strip shared libraries for release builds and copy unstripped to symbols dir
    ///
    /// This method:
    /// 1. Copies unstripped libraries to symbols staging directory (symbols/android/obj/{arch}/)
    /// 2. Strips the original libraries in place using llvm-strip
    /// 3. Copies STL library (libc++_shared.so) to each architecture's lib directory
    fn strip_shared_libraries(
        &self,
        ctx: &BuildContext,
        ndk: &AndroidNdkToolchain,
        abi_results: &[(AndroidAbi, PathBuf)],
        symbols_staging: &PathBuf,
    ) -> Result<()> {
        if ctx.options.verbose {
            eprintln!("Stripping shared libraries...");
        }

        for (abi, build_dir) in abi_results {
            let libs = self.find_libraries(build_dir, true, "shared", *abi)?;

            // Create symbols directory for this ABI (symbols/android/obj/{arch}/)
            let symbols_abi_dir = symbols_staging
                .join("symbols")
                .join("android")
                .join("obj")
                .join(abi.abi_string());
            std::fs::create_dir_all(&symbols_abi_dir)?;

            for lib in libs {
                // Only strip .so files
                if let Some(ext) = lib.extension() {
                    if ext == "so" {
                        // Copy unstripped library to symbols staging
                        let lib_name = lib.file_name().unwrap().to_str().unwrap();
                        let symbols_lib = symbols_abi_dir.join(lib_name);
                        std::fs::copy(&lib, &symbols_lib).with_context(|| {
                            format!("Failed to copy {} to symbols", lib.display())
                        })?;

                        // Strip the library in place
                        ndk.strip_library(&lib, ctx.options.verbose)?;
                    }
                }
            }

            // Copy STL library to the build output directory
            if let Some(lib_dir) = self.find_lib_dir(build_dir) {
                if ctx.options.verbose {
                    eprintln!("  Copying libc++_shared.so for {}...", abi.abi_string());
                }
                ndk.copy_stl_library(*abi, &lib_dir)?;
            }
        }

        Ok(())
    }

    /// Copy unstripped libraries to android/main_android_sdk/obj/local/{arch}/
    ///
    /// This must be called BEFORE stripping. It saves the unstripped version
    /// to obj/local/ so that symbols can be archived later.
    ///
    /// Matches Python ccgo's flow: copy unstripped → strip → use stripped for AAR
    fn copy_unstripped_to_obj_local(
        &self,
        ctx: &BuildContext,
        abi_results: &[(AndroidAbi, PathBuf)],
    ) -> Result<()> {
        if ctx.options.verbose {
            eprintln!("Saving unstripped libraries to obj/local/ before stripping...");
        }

        let android_project = ctx.project_root.join("android").join("main_android_sdk");
        let obj_local = android_project.join("obj").join("local");

        for (abi, build_dir) in abi_results {
            let libs = self.find_libraries(build_dir, true, "shared", *abi)?;

            // Create obj/local/{arch}/ directory
            let obj_abi_dir = obj_local.join(abi.abi_string());
            std::fs::create_dir_all(&obj_abi_dir)?;

            for lib in libs {
                if let Some(ext) = lib.extension() {
                    if ext == "so" {
                        let lib_name = lib.file_name().unwrap();
                        let dest = obj_abi_dir.join(lib_name);
                        std::fs::copy(&lib, &dest).with_context(|| {
                            format!("Failed to copy {} to obj/local", lib.display())
                        })?;

                        if ctx.options.verbose {
                            eprintln!("  Saved {} to obj/local", lib_name.to_string_lossy());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Copy unstripped libraries from Gradle's obj/local/ directory to symbols staging
    ///
    /// Gradle's buildAAR task automatically places unstripped shared libraries in
    /// android/main_android_sdk/obj/local/{arch}/. This method reads from that location
    /// and copies to symbols_staging/obj/{arch}/ for SYMBOLS.zip packaging.
    ///
    /// This matches Python ccgo's behavior where symbols are collected from the
    /// Gradle-generated merged_native_libs directory rather than manually copied during build.
    fn copy_obj_local_to_symbols(
        &self,
        ctx: &BuildContext,
        abis: &[AndroidAbi],
        symbols_staging: &PathBuf,
    ) -> Result<()> {
        if ctx.options.verbose {
            eprintln!("Copying unstripped libraries from merged_native_libs to symbols...");
        }

        // Path to Gradle-generated merged_native_libs directory
        let android_project = ctx.project_root.join("android").join("main_android_sdk");

        // Determine flavor (prodRelease or prodDebug)
        let flavor = if ctx.options.release { "prodRelease" } else { "prodDebug" };
        let flavor_cap = if ctx.options.release { "ProdRelease" } else { "ProdDebug" };

        // Path: build/intermediates/merged_native_libs/{flavor}/merge{FlavorCap}NativeLibs/out/lib
        let merged_libs = android_project
            .join("build/intermediates/merged_native_libs")
            .join(flavor)
            .join(format!("merge{}NativeLibs/out/lib", flavor_cap));

        if !merged_libs.exists() {
            if ctx.options.verbose {
                eprintln!("  Warning: merged_native_libs not found - Gradle may not have created it yet");
            }
            return Ok(());
        }

        for abi in abis {
            let abi_dir = merged_libs.join(abi.abi_string());
            if !abi_dir.exists() {
                if ctx.options.verbose {
                    eprintln!("  Skipping {} - merged_native_libs directory not found", abi.abi_string());
                }
                continue;
            }

            // Create symbols directory for this ABI (symbols/android/obj/{arch}/)
            let symbols_abi_dir = symbols_staging
                .join("symbols")
                .join("android")
                .join("obj")
                .join(abi.abi_string());
            std::fs::create_dir_all(&symbols_abi_dir)?;

            // Copy all .so files from merged_native_libs/{arch}/
            for entry in std::fs::read_dir(&abi_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "so" {
                            let lib_name = path.file_name().unwrap();
                            let dest = symbols_abi_dir.join(lib_name);
                            std::fs::copy(&path, &dest).with_context(|| {
                                format!("Failed to copy {} to symbols", path.display())
                            })?;

                            if ctx.options.verbose {
                                eprintln!("  Copied {} to symbols", lib_name.to_string_lossy());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Find library directory, checking multiple possible locations
    fn find_lib_dir(&self, build_dir: &PathBuf) -> Option<PathBuf> {
        let possible_dirs = vec![
            build_dir.join("out"),
            build_dir.join("install/lib"),
            build_dir.join("lib"),
        ];

        for dir in possible_dirs {
            if dir.exists() && std::fs::read_dir(&dir).map(|d| d.count() > 0).unwrap_or(false) {
                return Some(dir);
            }
        }
        None
    }

    /// Copy shared libraries to jniLibs directory for Gradle AAR packaging
    ///
    /// This copies .so files from cmake_build to android/main_android_sdk/src/main/jniLibs/
    /// so that Gradle can package them into the AAR.
    fn copy_libraries_to_jnilibs(
        &self,
        ctx: &BuildContext,
        abis: &[AndroidAbi],
    ) -> Result<()> {
        let android_project = ctx.project_root.join("android");
        if !android_project.exists() {
            if ctx.options.verbose {
                eprintln!("  Android Gradle project not found, skipping jniLibs copy");
            }
            return Ok(());
        }

        let jni_libs_dir = android_project.join("main_android_sdk/src/main/jniLibs");
        let libs_dir = android_project.join("main_android_sdk/libs");

        // Clean jniLibs directory to avoid conflicts with old builds
        if jni_libs_dir.exists() {
            if ctx.options.verbose {
                eprintln!("  Cleaning jniLibs directory...");
            }
            std::fs::remove_dir_all(&jni_libs_dir)?;
        }
        std::fs::create_dir_all(&jni_libs_dir)?;

        // Clean libs directory to avoid duplicate resources with Gradle
        if libs_dir.exists() {
            if ctx.options.verbose {
                eprintln!("  Cleaning libs directory...");
            }
            std::fs::remove_dir_all(&libs_dir)?;
        }

        // Copy .so files to jniLibs
        for abi in abis {
            let build_dir = ctx.cmake_build_dir.join(format!("shared/{}", abi.abi_string()));
            let libs = self.find_libraries(&build_dir, true, "shared", *abi)?;

            if libs.is_empty() {
                continue;
            }

            let abi_dir = jni_libs_dir.join(abi.abi_string());
            std::fs::create_dir_all(&abi_dir)?;

            for lib in libs {
                let lib_name = lib.file_name().unwrap();
                let dest = abi_dir.join(lib_name);
                std::fs::copy(&lib, &dest).with_context(|| {
                    format!("Failed to copy {} to jniLibs", lib.display())
                })?;

                if ctx.options.verbose {
                    eprintln!("  Copied {} to {}", lib_name.to_str().unwrap(), dest.display());
                }
            }
        }

        if ctx.options.verbose {
            eprintln!("  Successfully copied libraries to jniLibs for Gradle packaging");
        }

        Ok(())
    }

    /// Build AAR package using Gradle buildAAR task
    ///
    /// Steps:
    /// 1. Libraries should already be copied to jniLibs/ by copy_libraries_to_jnilibs()
    /// 2. Run Gradle buildAAR task (uses CCGO Gradle plugin)
    /// 3. Copy AAR from target/{debug|release}/android/ to output_dir
    ///
    /// The buildAAR task (from CCGO Gradle plugin):
    /// - Builds AAR with prod flavor: {project}-prod-release.aar
    /// - Renames to: {PROJECT}_ANDROID_SDK-{version}.aar
    /// - Copies to: target/{debug|release}/android/
    fn build_aar(
        &self,
        ctx: &BuildContext,
        _abis: &[AndroidAbi],
        output_dir: &PathBuf,
    ) -> Result<()> {
        let android_project = ctx.project_root.join("android");
        if !android_project.exists() {
            bail!("Android Gradle project not found at {}", android_project.display());
        }

        // Libraries should already be copied to jniLibs by copy_libraries_to_jnilibs()
        // Just run Gradle buildAAR task
        // The buildAAR task automatically:
        // - Builds the AAR with correct flavor (prod)
        // - Copies to target/{debug|release}/android/ with proper naming
        // - Uses CCGO Gradle plugin's naming convention
        let gradlew = if cfg!(target_os = "windows") {
            "gradlew.bat"
        } else {
            "./gradlew"
        };

        // Use standard Android assemble task instead of custom buildAAR
        // This avoids triggering buildLibrariesForMain which would spawn new ccgo builds
        // The assembleProdRelease/assembleProdDebug tasks just package jniLibs into AAR
        let assemble_task = if ctx.options.release {
            ":main_android_sdk:assembleProdRelease"
        } else {
            ":main_android_sdk:assembleProdDebug"
        };

        eprintln!("  Running Gradle {} task...", assemble_task);

        // Use spawn + wait to show real-time output instead of capturing it
        let status = std::process::Command::new(gradlew)
            .arg(assemble_task)
            .arg("--no-daemon")
            .current_dir(&android_project)
            .spawn()
            .with_context(|| format!("Failed to spawn Gradle in {}", android_project.display()))?
            .wait()
            .with_context(|| "Failed to wait for Gradle process")?;

        if !status.success() {
            bail!("Gradle {} failed with exit code: {:?}", assemble_task, status.code());
        }

        // Step 3: Find AAR from standard Android build output directory
        // The assembleProd* tasks create AAR at:
        // android/main_android_sdk/build/outputs/aar/main_android_sdk-prod-{release|debug}.aar
        let flavor = if ctx.options.release { "release" } else { "debug" };
        let aar_dir = android_project
            .join("main_android_sdk/build/outputs/aar");

        let aar_glob_pattern = aar_dir.join(format!("*-prod-{}.aar", flavor));
        let aar_files: Vec<_> = glob::glob(aar_glob_pattern.to_str().unwrap())
            .context("Failed to glob AAR files")?
            .filter_map(|p| p.ok())
            .collect();

        if aar_files.is_empty() {
            bail!("No AAR file found after Gradle assemble in {}", aar_dir.display());
        }

        // Copy AAR to output_dir with versioned naming format
        // Format: {PROJECT}_ANDROID_SDK-{version}-{publish_suffix}.aar
        // Example: CCGONOW_ANDROID_SDK-1.0.2-beta.18-dirty.aar
        let project_name_upper = ctx.config.package.name.to_uppercase();
        let dest_name = format!(
            "{}_ANDROID_SDK-{}-{}.aar",
            project_name_upper,
            ctx.version(),
            ctx.publish_suffix()
        );
        let dest = output_dir.join(&dest_name);

        // Copy the first AAR file (should only be one anyway)
        if let Some(aar_file) = aar_files.first() {
            std::fs::copy(aar_file, &dest).with_context(|| {
                format!("Failed to copy AAR from {} to {}", aar_file.display(), dest.display())
            })?;

            if ctx.options.verbose {
                eprintln!("  AAR generated: {}", dest.display());
            }
        }

        // Clean up old simple-named AAR files if they exist
        let old_aar = output_dir.join(format!("{}.aar", ctx.lib_name()));
        if old_aar.exists() && old_aar != dest {
            let _ = std::fs::remove_file(&old_aar);
        }

        // Clean up any other .aar files in output_dir (from previous builds)
        // This ensures only the renamed AAR exists, matching Python ccgo behavior
        if output_dir.exists() {
            for entry in std::fs::read_dir(&output_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "aar" && path != dest {
                            if ctx.options.verbose {
                                eprintln!("  Removing old AAR: {}", path.display());
                            }
                            std::fs::remove_file(&path).ok();
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Create symbols archive from staging directory
    fn create_symbols_archive_from_staging(
        &self,
        archive: &ArchiveBuilder,
        symbols_staging: &PathBuf,
    ) -> Result<PathBuf> {
        archive.create_symbols_archive(symbols_staging)
    }
}

impl PlatformBuilder for AndroidBuilder {
    fn platform_name(&self) -> &str {
        "Android"
    }

    fn default_architectures(&self) -> Vec<String> {
        vec![
            "arm64-v8a".to_string(),
            "armeabi-v7a".to_string(),
            "x86_64".to_string(),
        ]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        // Check for CMake
        if !crate::build::cmake::is_cmake_available() {
            bail!("CMake is required for Android builds. Please install CMake.");
        }

        // Check for Android NDK
        let ndk = AndroidNdkToolchain::detect()
            .context("Android NDK is required. Please set ANDROID_NDK_HOME environment variable.")?;

        ndk.validate()?;

        if ctx.options.verbose {
            eprintln!("Using Android NDK {} at {}", ndk.version(), ndk.path().unwrap().display());
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Validate prerequisites first
        self.validate_prerequisites(ctx)?;

        let ndk = AndroidNdkToolchain::detect()?;

        if ctx.options.verbose {
            eprintln!("Building {} for Android...", ctx.lib_name());
        }

        // Determine ABIs to build
        let abis: Vec<AndroidAbi> = if ctx.options.architectures.is_empty() {
            vec![AndroidAbi::Arm64V8a, AndroidAbi::ArmeabiV7a, AndroidAbi::X86_64]
        } else {
            ctx.options.architectures
                .iter()
                .map(|s| Self::parse_abi(s))
                .collect::<Result<Vec<_>>>()?
        };

        // Get API level (default to 24)
        let api_level = DEFAULT_API_LEVEL;

        // Create output directory
        std::fs::create_dir_all(&ctx.output_dir)?;

        // Create archive builder with "Android" platform name (capital A to match Python)
        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "Android",
            ctx.output_dir.clone(),
        )?;

        // Create symbols staging directory for unstripped libraries
        let symbols_staging = ctx.cmake_build_dir.join("symbols_staging");
        std::fs::create_dir_all(&symbols_staging)?;

        let mut built_link_types = Vec::new();
        let mut symbols_archive: Option<PathBuf> = None;

        // Build static libraries
        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let results = self.build_link_type(ctx, &ndk, "static", &abis, api_level)?;
            self.add_libraries_to_archive(&archive, &results, "static", false)?;
            built_link_types.push("static");
        }

        // Build shared libraries
        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            let results = self.build_link_type(ctx, &ndk, "shared", &abis, api_level)?;

            // For release builds: First copy unstripped to obj/local, then strip, then copy to jniLibs
            // For debug builds: Just copy to jniLibs (Gradle will handle obj/local)
            if ctx.options.release {
                // Save unstripped libraries to android/main_android_sdk/obj/local/{arch}/
                // before stripping (Python ccgo does this)
                self.copy_unstripped_to_obj_local(ctx, &results)?;
                // Now strip the libraries in cmake_build directory
                self.strip_shared_libraries(ctx, &ndk, &results, &symbols_staging)?;
            }

            self.add_libraries_to_archive(&archive, &results, "shared", true)?;
            built_link_types.push("shared");

            // Copy shared libraries to jniLibs for Gradle AAR packaging
            // This is needed for both native-only and full build modes
            if ctx.options.verbose {
                eprintln!("Copying libraries to jniLibs for Gradle...");
            }
            self.copy_libraries_to_jnilibs(ctx, &abis)?;
        }

        // Add include files from project's include directory (matching pyccgo behavior)
        // Path: include/{lib_name}/ (if source has project subdir) or include/ (if not)
        let include_source = ctx.project_root.join("include");
        if include_source.exists() {
            let include_path = get_unified_include_path(ctx.lib_name(), &include_source);
            archive.add_directory(&include_source, &include_path)?;
            if ctx.options.verbose {
                eprintln!("Added include files from {} to {}", include_source.display(), include_path);
            }
        }

        // Build AAR package if shared libraries were built
        // In native-only mode, we still build AAR since jniLibs are already populated
        if built_link_types.contains(&"shared") {
            if ctx.options.verbose {
                eprintln!("Building AAR package...");
            }
            if let Err(e) = self.build_aar(ctx, &abis, &ctx.output_dir) {
                eprintln!("Warning: Failed to build AAR: {}. Continuing without AAR.", e);
                let gradle_cmd = if ctx.options.release {
                    "cd android && ./gradlew buildAAR -PisRelease=true"
                } else {
                    "cd android && ./gradlew buildAAR -PisRelease=false"
                };
                eprintln!("To build AAR manually, run: {}", gradle_cmd);
            } else {
                if ctx.options.verbose {
                    eprintln!("AAR package built successfully");
                }
                // After successful AAR build, copy unstripped libraries from obj/local/ to symbols
                // This works for both release and debug builds
                self.copy_obj_local_to_symbols(ctx, &abis, &symbols_staging)?;
            }
        }

        // Add AAR to archive if it exists (not in native-only mode)
        // Only add to haars/android/ subdirectory, not to root
        if !ctx.options.native_only {
            // AAR file now uses versioned naming from build_aar()
            // Format: {PROJECT}_ANDROID_SDK-{version}-{publish_suffix}.aar
            let project_name_upper = ctx.config.package.name.to_uppercase();
            let aar_versioned_name = format!(
                "{}_ANDROID_SDK-{}-{}.aar",
                project_name_upper,
                ctx.version(),
                ctx.publish_suffix()
            );
            let aar_path = ctx.output_dir.join(&aar_versioned_name);

            if aar_path.exists() {
                // Add to haars/android/ subdirectory only
                let aar_dest = format!("haars/android/{}", aar_versioned_name);
                archive.add_file(&aar_path, &aar_dest)?;
                if ctx.options.verbose {
                    eprintln!("Added AAR to archive: {}", aar_dest);
                }
            }
        }

        // Create the SDK archive
        let architectures: Vec<String> = abis.iter().map(|a| a.abi_string().to_string()).collect();
        let link_type_str = ctx.options.link_type.to_string();
        let sdk_archive = archive.create_sdk_archive(&architectures, &link_type_str)?;

        // Create symbols archive if we have stripped libraries (for release shared builds)
        if built_link_types.contains(&"shared") {
            // Check if symbols staging has content
            let symbols_dir = symbols_staging.join("symbols");
            let has_symbols = symbols_dir.exists() &&
                std::fs::read_dir(&symbols_dir)
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false);

            if has_symbols {
                let sym_archive = self.create_symbols_archive_from_staging(&archive, &symbols_staging)?;
                if ctx.options.verbose {
                    eprintln!("Created symbols archive: {}", sym_archive.display());
                }
                symbols_archive = Some(sym_archive);
            } else if ctx.options.verbose {
                eprintln!("No symbols to archive (obj directory is empty)");
            }
        }

        // Clean up symbols staging directory
        std::fs::remove_dir_all(&symbols_staging).ok();

        // Check if AAR was generated (not in native-only mode)
        let aar_archive = if !ctx.options.native_only {
            let project_name_upper = ctx.config.package.name.to_uppercase();
            let aar_versioned_name = format!(
                "{}_ANDROID_SDK-{}-{}.aar",
                project_name_upper,
                ctx.version(),
                ctx.publish_suffix()
            );
            let aar_path = ctx.output_dir.join(&aar_versioned_name);
            if aar_path.exists() {
                Some(aar_path)
            } else {
                None
            }
        } else {
            None
        };

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "Android build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive,
            aar_archive,
            duration_secs: duration.as_secs_f64(),
            architectures,
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean new directory structure: cmake_build/{release|debug}/android
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("android");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/Android, cmake_build/android
        for old_dir in &[
            ctx.project_root.join("cmake_build/Android"),
            ctx.project_root.join("cmake_build/android"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/android"),
            ctx.project_root.join("target/debug/android"),
            ctx.project_root.join("target/release/Android"),
            ctx.project_root.join("target/debug/Android"),
            ctx.project_root.join("target/android"),
            ctx.project_root.join("target/Android"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for AndroidBuilder {
    fn default() -> Self {
        Self::new()
    }
}
