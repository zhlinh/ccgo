//! Android platform builder
//!
//! Builds native libraries and AAR packages for Android using CMake with NDK.
//! Supports three ABIs (arm64-v8a, armeabi-v7a, x86_64) — same set as OHOS.
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
        AndroidAbi::from_str(s).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid Android ABI: {}.\n\
                 Valid options: arm64-v8a, armeabi-v7a, x86_64\n\
                 Aliases: v8/a64/arm64/armv8/aarch64 → arm64-v8a;  \
                 v7/a32/arm32/armv7/aarch32 → armeabi-v7a;  x64 → x86_64",
                s
            )
        })
    }

    /// Configure CMake with common variables
    fn configure_cmake(
        &self,
        ctx: &BuildContext,
        cmake: CMakeConfig,
        build_shared: bool,
        cmake_vars: Vec<(String, String)>,
    ) -> CMakeConfig {
        let mut cmake = cmake
            .variable("CCGO_BUILD_STATIC", if build_shared { "OFF" } else { "ON" })
            .variable("CCGO_BUILD_SHARED", if build_shared { "ON" } else { "OFF" })
            .variable("CCGO_BUILD_SHARED_LIBS", if build_shared { "ON" } else { "OFF" })
            .variable("CCGO_LIB_NAME", ctx.lib_name())
            .variable("CCGO_CONFIG_PRESET_VISIBILITY", ctx.symbol_visibility().to_string());

        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        for (name, value) in cmake_vars {
            cmake = cmake.variable(&name, &value);
        }

        if ctx.options.release {
            cmake = cmake
                .variable("CMAKE_SKIP_INSTALL_RPATH", "ON")
                .variable("CMAKE_BUILD_WITH_INSTALL_RPATH", "OFF")
                .variable("CMAKE_STRIP", "");
        }

        if let Some(deps_map) = ctx.deps_map() {
            cmake = cmake.variable("CCGO_CONFIG_DEPS_MAP", deps_map);
        }

        if let Ok(feature_defines) = ctx.cmake_feature_defines() {
            if !feature_defines.is_empty() {
                cmake = cmake.feature_definitions(&feature_defines);
                if ctx.options.verbose {
                    eprintln!("    Enabled features: {}", feature_defines.replace(';', ", "));
                }
            }
        }

        if let Some(cache) = ctx.compiler_cache() {
            cmake = cmake.compiler_cache(cache);
        }

        cmake
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
        let cmake_vars = ndk.cmake_variables_for_abi(abi, api_level);

        let build_type = if ctx.options.release {
            BuildType::RelWithDebInfo
        } else {
            BuildType::Debug
        };

        let cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .build_type(build_type)
            .install_prefix(install_dir.clone())
            .jobs(ctx.jobs())
            .verbose(ctx.options.verbose);

        let cmake = self.configure_cmake(ctx, cmake, build_shared, cmake_vars);

        cmake.configure_build_install()?;

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

    /// Get merged_native_libs path for Gradle project
    fn get_merged_libs_path(ctx: &BuildContext) -> PathBuf {
        let android_project = ctx.project_root.join("android").join("main_android_sdk");
        let flavor = if ctx.options.release { "prodRelease" } else { "prodDebug" };
        let flavor_cap = if ctx.options.release { "ProdRelease" } else { "ProdDebug" };

        android_project
            .join("build/intermediates/merged_native_libs")
            .join(flavor)
            .join(format!("merge{}NativeLibs/out/lib", flavor_cap))
    }

    /// Copy .so files from a directory to symbols staging
    fn copy_so_files_to_symbols(
        abi_dir: &PathBuf,
        symbols_abi_dir: &PathBuf,
        verbose: bool,
    ) -> Result<()> {
        for entry in std::fs::read_dir(abi_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "so").unwrap_or(false) {
                let lib_name = path.file_name().unwrap();
                let dest = symbols_abi_dir.join(lib_name);
                std::fs::copy(&path, &dest).with_context(|| {
                    format!("Failed to copy {} to symbols", path.display())
                })?;

                if verbose {
                    eprintln!("  Copied {} to symbols", lib_name.to_string_lossy());
                }
            }
        }
        Ok(())
    }

    /// Copy unstripped libraries from Gradle's obj/local/ directory to symbols staging
    fn copy_obj_local_to_symbols(
        &self,
        ctx: &BuildContext,
        abis: &[AndroidAbi],
        symbols_staging: &PathBuf,
    ) -> Result<()> {
        if ctx.options.verbose {
            eprintln!("Copying unstripped libraries from merged_native_libs to symbols...");
        }

        let merged_libs = Self::get_merged_libs_path(ctx);

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

            let symbols_abi_dir = symbols_staging
                .join("symbols")
                .join("android")
                .join("obj")
                .join(abi.abi_string());
            std::fs::create_dir_all(&symbols_abi_dir)?;

            Self::copy_so_files_to_symbols(&abi_dir, &symbols_abi_dir, ctx.options.verbose)?;
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

    /// Run Gradle assemble task to build AAR
    fn run_gradle_assemble(&self, ctx: &BuildContext, android_project: &PathBuf) -> Result<()> {
        let gradlew = if cfg!(target_os = "windows") {
            "gradlew.bat"
        } else {
            "./gradlew"
        };

        let assemble_task = if ctx.options.release {
            ":main_android_sdk:assembleProdRelease"
        } else {
            ":main_android_sdk:assembleProdDebug"
        };

        eprintln!("  Running Gradle {} task...", assemble_task);

        let status = std::process::Command::new(gradlew)
            .arg(assemble_task)
            .arg("--no-daemon")
            .current_dir(android_project)
            .spawn()
            .with_context(|| format!("Failed to spawn Gradle in {}", android_project.display()))?
            .wait()
            .with_context(|| "Failed to wait for Gradle process")?;

        if !status.success() {
            bail!("Gradle {} failed with exit code: {:?}", assemble_task, status.code());
        }

        Ok(())
    }

    /// Find built AAR and copy to output directory with versioned naming
    fn find_and_copy_aar_to_output(
        &self,
        ctx: &BuildContext,
        android_project: &PathBuf,
        output_dir: &PathBuf,
    ) -> Result<PathBuf> {
        let flavor = if ctx.options.release { "release" } else { "debug" };
        let aar_dir = android_project.join("main_android_sdk/build/outputs/aar");

        let aar_glob_pattern = aar_dir.join(format!("*-prod-{}.aar", flavor));
        let aar_files: Vec<_> = glob::glob(aar_glob_pattern.to_str().unwrap())
            .context("Failed to glob AAR files")?
            .filter_map(|p| p.ok())
            .collect();

        if aar_files.is_empty() {
            bail!("No AAR file found after Gradle assemble in {}", aar_dir.display());
        }

        let project_name_upper = ctx.lib_name().to_uppercase();
        let dest_name = format!("{}_ANDROID_SDK-{}.aar", project_name_upper, ctx.version());
        let dest = output_dir.join(&dest_name);

        if let Some(aar_file) = aar_files.first() {
            std::fs::copy(aar_file, &dest).with_context(|| {
                format!("Failed to copy AAR from {} to {}", aar_file.display(), dest.display())
            })?;

            if ctx.options.verbose {
                eprintln!("  AAR generated: {}", dest.display());
            }
        }

        Ok(dest)
    }

    /// Clean up old AAR files in output directory
    fn cleanup_old_aar_files(&self, ctx: &BuildContext, output_dir: &PathBuf, current_aar: &PathBuf) -> Result<()> {
        let old_aar = output_dir.join(format!("{}.aar", ctx.lib_name()));
        if old_aar.exists() && old_aar != *current_aar {
            let _ = std::fs::remove_file(&old_aar);
        }

        if output_dir.exists() {
            for entry in std::fs::read_dir(output_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "aar" && path != *current_aar {
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

        // Sync CCGO.toml version -> android/gradle/libs.versions.toml
        crate::utils::version_sync::sync_gradle_version_catalog(
            &android_project.join("gradle").join("libs.versions.toml"),
            ctx.version(),
        );

        // Run Gradle assemble
        self.run_gradle_assemble(ctx, &android_project)?;

        // Find and copy AAR to output directory
        let dest = self.find_and_copy_aar_to_output(ctx, &android_project, output_dir)?;

        // Clean up old AAR files
        self.cleanup_old_aar_files(ctx, output_dir, &dest)?;

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

    /// Resolve ABIs from build options
    fn resolve_abis(ctx: &BuildContext) -> Result<Vec<AndroidAbi>> {
        if ctx.options.architectures.is_empty() {
            Ok(vec![AndroidAbi::Arm64V8a, AndroidAbi::ArmeabiV7a, AndroidAbi::X86_64])
        } else {
            ctx.options.architectures
                .iter()
                .map(|s| Self::parse_abi(s))
                .collect()
        }
    }

    /// Build static and shared libraries, returning built link types
    fn build_static_and_shared(
        &self,
        ctx: &BuildContext,
        ndk: &AndroidNdkToolchain,
        abis: &[AndroidAbi],
        api_level: u32,
        archive: &ArchiveBuilder,
        symbols_staging: &PathBuf,
    ) -> Result<Vec<&'static str>> {
        let mut built_link_types = Vec::new();

        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let results = self.build_link_type(ctx, ndk, "static", abis, api_level)?;
            self.add_libraries_to_archive(archive, &results, "static", false)?;
            built_link_types.push("static");
        }

        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            let results = self.build_link_type(ctx, ndk, "shared", abis, api_level)?;

            if ctx.options.release {
                self.copy_unstripped_to_obj_local(ctx, &results)?;
                self.strip_shared_libraries(ctx, ndk, &results, symbols_staging)?;
            }

            self.add_libraries_to_archive(archive, &results, "shared", true)?;
            built_link_types.push("shared");

            if ctx.options.verbose {
                eprintln!("Copying libraries to jniLibs for Gradle...");
            }
            self.copy_libraries_to_jnilibs(ctx, abis)?;
        }

        Ok(built_link_types)
    }

    /// Add AAR to archive if it exists
    fn add_aar_to_archive_if_needed(&self, ctx: &BuildContext, archive: &ArchiveBuilder) -> Result<()> {
        if ctx.options.native_only {
            return Ok(());
        }

        let project_name_upper = ctx.lib_name().to_uppercase();
        let aar_versioned_name = format!("{}_ANDROID_SDK-{}.aar", project_name_upper, ctx.version());
        let aar_path = ctx.output_dir.join(&aar_versioned_name);

        if aar_path.exists() {
            let aar_dest = format!("haars/android/{}", aar_versioned_name);
            archive.add_file(&aar_path, &aar_dest)?;
            if ctx.options.verbose {
                eprintln!("Added AAR to archive: {}", aar_dest);
            }
        }

        Ok(())
    }

    /// Create symbols archive if needed
    fn create_symbols_archive_if_needed(
        &self,
        ctx: &BuildContext,
        built_link_types: &[&str],
        symbols_staging: &PathBuf,
        archive: &ArchiveBuilder,
    ) -> Result<Option<PathBuf>> {
        if !built_link_types.contains(&"shared") {
            return Ok(None);
        }

        let symbols_dir = symbols_staging.join("symbols");
        let has_symbols = symbols_dir.exists() &&
            std::fs::read_dir(&symbols_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);

        if has_symbols {
            let sym_archive = self.create_symbols_archive_from_staging(archive, symbols_staging)?;
            if ctx.options.verbose {
                eprintln!("Created symbols archive: {}", sym_archive.display());
            }
            Ok(Some(sym_archive))
        } else {
            if ctx.options.verbose {
                eprintln!("No symbols to archive (obj directory is empty)");
            }
            Ok(None)
        }
    }

    /// Get AAR archive path if it exists
    fn get_aar_archive_path(ctx: &BuildContext) -> Option<PathBuf> {
        if ctx.options.native_only {
            return None;
        }

        let project_name_upper = ctx.lib_name().to_uppercase();
        let aar_versioned_name = format!("{}_ANDROID_SDK-{}.aar", project_name_upper, ctx.version());
        let aar_path = ctx.output_dir.join(&aar_versioned_name);

        if aar_path.exists() { Some(aar_path) } else { None }
    }

    /// Add include files to archive if they exist
    fn add_include_files_if_needed(&self, ctx: &BuildContext, archive: &ArchiveBuilder) -> Result<()> {
        let include_source = ctx.include_source_dir();
        if !include_source.exists() {
            return Ok(());
        }

        let include_path = get_unified_include_path(ctx.lib_name(), &include_source);
        archive.add_directory(&include_source, &include_path)?;
        if ctx.options.verbose {
            eprintln!("Added include files from {} to {}", include_source.display(), include_path);
        }
        Ok(())
    }

    /// Build AAR and copy symbols if shared libraries were built
    fn build_aar_and_symbols(
        &self,
        ctx: &BuildContext,
        abis: &[AndroidAbi],
        built_link_types: &[&str],
        symbols_staging: &PathBuf,
    ) -> Result<()> {
        if !built_link_types.contains(&"shared") {
            return Ok(());
        }

        if ctx.options.verbose {
            eprintln!("Building AAR package...");
        }
        self.build_aar(ctx, abis, &ctx.output_dir)?;
        if ctx.options.verbose {
            eprintln!("AAR package built successfully");
        }
        self.copy_obj_local_to_symbols(ctx, abis, symbols_staging)?;
        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        self.validate_prerequisites(ctx)?;
        let ndk = AndroidNdkToolchain::detect()?;

        if ctx.options.verbose {
            eprintln!("Building {} for Android...", ctx.lib_name());
        }

        let abis = Self::resolve_abis(ctx)?;
        let api_level = DEFAULT_API_LEVEL;

        std::fs::create_dir_all(&ctx.output_dir)?;

        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "Android",
            ctx.output_dir.clone(),
        )?;

        let symbols_staging = ctx.cmake_build_dir.join("symbols_staging");
        std::fs::create_dir_all(&symbols_staging)?;

        let built_link_types = self.build_static_and_shared(
            ctx, &ndk, &abis, api_level, &archive, &symbols_staging
        )?;

        self.add_include_files_if_needed(ctx, &archive)?;
        self.build_aar_and_symbols(ctx, &abis, &built_link_types, &symbols_staging)?;
        self.add_aar_to_archive_if_needed(ctx, &archive)?;

        let architectures: Vec<String> = abis.iter().map(|a| a.abi_string().to_string()).collect();
        let link_type_str = ctx.options.link_type.to_string();
        let sdk_archive = archive.create_sdk_archive(&architectures, &link_type_str)?;

        let symbols_archive = self.create_symbols_archive_if_needed(
            ctx, &built_link_types, &symbols_staging, &archive
        )?;

        std::fs::remove_dir_all(&symbols_staging).ok();

        let aar_archive = Self::get_aar_archive_path(ctx);
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
