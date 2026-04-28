//! OpenHarmony (OHOS) platform builder
//!
//! Builds native libraries and HAR packages for OpenHarmony using CMake with OHOS SDK.
//! Supports multiple ABIs (arm64-v8a, armeabi-v7a, x86_64).

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::{get_unified_include_path, ArchiveBuilder};
use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::toolchains::ohos::{OhosAbi, OhosSdkToolchain, DEFAULT_MIN_SDK_VERSION};
use crate::build::toolchains::Toolchain;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// OHOS platform builder
pub struct OhosBuilder;

impl OhosBuilder {
    pub fn new() -> Self {
        Self
    }

    /// Parse ABI string to OhosAbi enum
    fn parse_abi(s: &str) -> Result<OhosAbi> {
        OhosAbi::from_str(s).ok_or_else(|| {
            anyhow::anyhow!(
                "Invalid OHOS ABI: {}.\n\
                 Valid options: arm64-v8a, armeabi-v7a, x86_64\n\
                 Aliases: v8/a64/arm64/armv8/aarch64 → arm64-v8a;  \
                 v7/a32/arm32/armv7/aarch32 → armeabi-v7a;  x64 → x86_64",
                s
            )
        })
    }

    /// Merge all module static libraries (from out/) into a single library
    fn merge_module_static_libs(
        &self,
        sdk: &OhosSdkToolchain,
        build_dir: &PathBuf,
        lib_name: &str,
        verbose: bool,
    ) -> Result<()> {
        let out_dir = build_dir.join("out");
        if !out_dir.exists() {
            return Ok(());
        }

        let main_lib_name = format!("lib{}.a", lib_name);
        let main_lib_path = out_dir.join(&main_lib_name);

        // Collect module libs, excluding the main one
        let mut module_libs: Vec<PathBuf> = Vec::new();
        for entry in std::fs::read_dir(&out_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "a" && path != main_lib_path {
                        module_libs.push(path);
                    }
                }
            }
        }

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

        sdk.merge_static_libs(&module_libs, &main_lib_path)?;

        for lib in &module_libs {
            let _ = std::fs::remove_file(lib);
        }

        Ok(())
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
            .variable(
                "CCGO_BUILD_SHARED_LIBS",
                if build_shared { "ON" } else { "OFF" },
            )
            .variable("CCGO_LIB_NAME", ctx.lib_name())
            .variable(
                "CCGO_CONFIG_PRESET_VISIBILITY",
                ctx.symbol_visibility().to_string(),
            );

        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        for (name, value) in cmake_vars {
            cmake = cmake.variable(&name, &value);
        }

        if let Some(deps_map) = ctx.deps_map() {
            cmake = cmake.variable("CCGO_CONFIG_DEPS_MAP", deps_map);
        }

        if let Ok(feature_defines) = ctx.cmake_feature_defines() {
            if !feature_defines.is_empty() {
                cmake = cmake.feature_definitions(&feature_defines);
                if ctx.options.verbose {
                    eprintln!(
                        "    Enabled features: {}",
                        feature_defines.replace(';', ", ")
                    );
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
        sdk: &OhosSdkToolchain,
        abi: OhosAbi,
        link_type: &str,
        min_sdk_version: u32,
    ) -> Result<PathBuf> {
        let build_dir = ctx
            .cmake_build_dir
            .join(format!("{}/{}", link_type, abi.abi_string()));
        let install_dir = build_dir.join("install");

        let build_shared = link_type == "shared";
        let cmake_vars = sdk.cmake_variables_for_abi(abi, min_sdk_version);

        let build_type = if ctx.options.release {
            BuildType::Release
        } else {
            BuildType::Debug
        };

        let cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .build_type(build_type)
            .install_prefix(install_dir.clone())
            .jobs(ctx.jobs())
            .verbose(ctx.options.verbose);

        // Resolve and pass per-dep linkage as a semicolon-separated list of
        // <NAME>=<VALUE> pairs. CMake parses this in FindCCGODependencies.cmake
        // to populate CCGO_DEPENDENCY_<NAME>_LINKAGE for each dep, which then
        // drives the choice between target_link_libraries(... shared) and
        // target_link_libraries(... static) per dep.
        let linkages = ctx.resolved_dep_linkages(self.platform_name())?;
        let linkages_var = if linkages.is_empty() {
            None
        } else {
            Some(
                linkages
                    .iter()
                    .map(|(name, l)| format!("{name}={l}"))
                    .collect::<Vec<_>>()
                    .join(";"),
            )
        };

        let cmake = self.configure_cmake(ctx, cmake, build_shared, cmake_vars);

        let cmake = if let Some(v) = linkages_var {
            cmake.variable("CCGO_DEPENDENCY_LINKAGES", v)
        } else {
            cmake
        };

        cmake.configure_build_install()?;

        if !build_shared {
            // Static consumer = thin .a: merge intra-project module libs only.
            // Third-party dep symbols are not archived in; the downstream linker
            // resolves them. Per-dep static-embed (static-embedded linkage) is
            // handled by the CMake template, not via Rust-side merging.
            self.merge_module_static_libs(sdk, &build_dir, ctx.lib_name(), ctx.options.verbose)?;
        }

        Ok(build_dir)
    }

    /// Find library files for a specific ABI
    /// Prioritizes the combined library output directory
    fn find_libraries(
        &self,
        build_dir: &PathBuf,
        is_shared: bool,
        _link_type: &str,
        _abi: OhosAbi,
        lib_name: &str,
    ) -> Result<Vec<PathBuf>> {
        let extension = if is_shared { "so" } else { "a" };
        let mut libs = Vec::new();

        // build_dir is already the per-ABI dir (e.g. cmake_build/debug/ohos/static/arm64-v8a),
        // so the merged library from merge_module_static_libs lives in build_dir/out/.
        let possible_dirs = vec![
            build_dir.join("out"), // Merged library — highest priority
            build_dir.join("install/lib"),
            build_dir.join("lib"),
        ];

        // Main library filename to look for
        let main_lib_name = format!("lib{}.{}", lib_name, extension);

        for lib_dir in possible_dirs {
            if !lib_dir.exists() {
                continue;
            }

            for entry in std::fs::read_dir(&lib_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(file_name) = path.file_name() {
                        // Only include the main library, skip intermediate libraries (e.g., libccgonow-api.so)
                        if file_name == main_lib_name.as_str() {
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
                }
            }

            // If we found the main library, stop searching
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
        sdk: &OhosSdkToolchain,
        link_type: &str,
        abis: &[OhosAbi],
        min_sdk_version: u32,
    ) -> Result<Vec<(OhosAbi, PathBuf)>> {
        if ctx.options.verbose {
            eprintln!("Building {} library for OHOS...", link_type);
        }

        let mut results = Vec::new();

        for abi in abis {
            if ctx.options.verbose {
                eprintln!("  Building for {}...", abi.abi_string());
            }

            let build_dir = self.build_abi(ctx, sdk, *abi, link_type, min_sdk_version)?;
            results.push((*abi, build_dir));
        }

        Ok(results)
    }

    /// Copy libraries to archive structure (per-ABI directories)
    fn add_libraries_to_archive(
        &self,
        archive: &ArchiveBuilder,
        abi_results: &[(OhosAbi, PathBuf)],
        link_type: &str,
        is_shared: bool,
        lib_name: &str,
    ) -> Result<()> {
        for (abi, build_dir) in abi_results {
            let libs = self.find_libraries(build_dir, is_shared, link_type, *abi, lib_name)?;

            for lib in &libs {
                let file_name = lib.file_name().unwrap().to_str().unwrap();
                // Archive path: lib/ohos/{static|shared}/{arch}/{lib_name}
                let dest = format!(
                    "lib/{}/{}/{}/{}",
                    self.platform_name(),
                    link_type,
                    abi.abi_string(),
                    file_name
                );
                archive.add_file(lib, &dest)?;
            }
        }

        Ok(())
    }

    /// Copy unstripped libraries to symbols staging
    ///
    /// This method copies unstripped libraries to symbols staging directory (symbols/ohos/obj/{arch}/)
    /// for both debug and release builds, before any stripping occurs.
    fn copy_unstripped_to_symbols(
        &self,
        abi_results: &[(OhosAbi, PathBuf)],
        lib_name: &str,
        symbols_staging: &PathBuf,
        verbose: bool,
    ) -> Result<()> {
        for (abi, build_dir) in abi_results {
            let libs = self.find_libraries(build_dir, true, "shared", *abi, lib_name)?;

            // Create symbols directory for this ABI (symbols/ohos/obj/{arch}/)
            let symbols_abi_dir = symbols_staging
                .join("symbols")
                .join("ohos")
                .join("obj")
                .join(abi.abi_string());
            std::fs::create_dir_all(&symbols_abi_dir)?;

            for lib in libs {
                // Only process .so files
                if let Some(ext) = lib.extension() {
                    if ext == "so" {
                        // Copy unstripped library to symbols staging
                        let lib_name = lib.file_name().unwrap();
                        let symbols_lib = symbols_abi_dir.join(lib_name);
                        std::fs::copy(&lib, &symbols_lib).with_context(|| {
                            format!("Failed to copy {} to symbols", lib.display())
                        })?;

                        if verbose {
                            eprintln!("  Saved {} to symbols", lib_name.to_string_lossy());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Strip shared libraries in place using llvm-strip
    ///
    /// This method strips debug symbols from libraries to reduce size for release builds.
    /// Should only be called AFTER copy_unstripped_to_symbols() has saved the originals.
    fn strip_shared_libraries(
        &self,
        sdk: &OhosSdkToolchain,
        abi_results: &[(OhosAbi, PathBuf)],
        lib_name: &str,
        verbose: bool,
    ) -> Result<()> {
        let strip_path = sdk.strip_path();
        if !strip_path.exists() {
            if verbose {
                eprintln!("Warning: llvm-strip not found, skipping symbol stripping");
            }
            return Ok(());
        }

        for (abi, build_dir) in abi_results {
            let libs = self.find_libraries(build_dir, true, "shared", *abi, lib_name)?;

            for lib in libs {
                // Only strip .so files
                if let Some(ext) = lib.extension() {
                    if ext == "so" {
                        if verbose {
                            eprintln!("  Stripping {}...", lib.display());
                        }

                        // Strip the library in place
                        let status = std::process::Command::new(&strip_path)
                            .arg("--strip-unneeded")
                            .arg(&lib)
                            .status()
                            .with_context(|| format!("Failed to strip {}", lib.display()))?;

                        if !status.success() && verbose {
                            eprintln!(
                                "Warning: Failed to strip {} for {}",
                                lib.display(),
                                abi.abi_string()
                            );
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Copy libraries to ohos/main_ohos_sdk/libs/{arch}/ for HAR packaging
    fn copy_libraries_to_libs(
        &self,
        ctx: &BuildContext,
        abis: &[OhosAbi],
        lib_name: &str,
    ) -> Result<()> {
        let ohos_project = ctx.project_root.join("ohos").join("main_ohos_sdk");
        if !ohos_project.exists() {
            if ctx.options.verbose {
                eprintln!(
                    "Warning: OHOS project not found at {}",
                    ohos_project.display()
                );
            }
            return Ok(());
        }

        let libs_dir = ohos_project.join("libs");

        // Clean existing libs directory to avoid stale libraries
        if libs_dir.exists() {
            std::fs::remove_dir_all(&libs_dir)?;
        }

        for abi in abis {
            // Find shared libraries for this ABI
            let build_dir = ctx
                .cmake_build_dir
                .join(format!("shared/{}", abi.abi_string()));

            let libs = self.find_libraries(&build_dir, true, "shared", *abi, lib_name)?;

            if libs.is_empty() {
                continue;
            }

            // Create libs/{arch}/ directory
            let abi_dir = libs_dir.join(abi.abi_string());
            std::fs::create_dir_all(&abi_dir)?;

            for lib in libs {
                let lib_name = lib.file_name().unwrap();
                let dest = abi_dir.join(lib_name);
                std::fs::copy(&lib, &dest)
                    .with_context(|| format!("Failed to copy {} to libs", lib.display()))?;

                if ctx.options.verbose {
                    eprintln!(
                        "  Copied {} to {}",
                        lib_name.to_str().unwrap(),
                        dest.display()
                    );
                }
            }
        }

        if ctx.options.verbose {
            eprintln!("  Successfully copied libraries to libs for Hvigor packaging");
        }

        Ok(())
    }

    /// Find hvigorw command (local or system)
    fn find_hvigorw_cmd(ctx: &BuildContext, ohos_project: &PathBuf) -> Result<String> {
        let hvigorw_name = if cfg!(target_os = "windows") {
            "hvigorw.bat"
        } else {
            "hvigorw"
        };

        let local_hvigorw = ohos_project.join(hvigorw_name);
        if local_hvigorw.exists() {
            if ctx.options.verbose {
                eprintln!("  Using local hvigorw: {}", local_hvigorw.display());
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let metadata = std::fs::metadata(&local_hvigorw)?;
                let mut permissions = metadata.permissions();
                let mode = permissions.mode();
                if mode & 0o100 == 0 {
                    permissions.set_mode(mode | 0o100);
                    std::fs::set_permissions(&local_hvigorw, permissions).with_context(|| {
                        format!("Failed to make {} executable", local_hvigorw.display())
                    })?;

                    if ctx.options.verbose {
                        eprintln!("  Made {} executable", local_hvigorw.display());
                    }
                }
            }

            return Ok(local_hvigorw.to_string_lossy().to_string());
        }

        // Fall back to system hvigorw from PATH
        if ctx.options.verbose {
            eprintln!("  Local hvigorw not found, using system hvigorw from PATH");
        }

        let which_result = std::process::Command::new("which")
            .arg(hvigorw_name)
            .output();

        match which_result {
            Ok(output) if output.status.success() => {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if ctx.options.verbose {
                    eprintln!("  Found system hvigorw: {}", path);
                }
                Ok(hvigorw_name.to_string())
            }
            _ => {
                bail!(
                    "Hvigor wrapper not found. Please ensure hvigorw is in your PATH or \
                     exists at {}",
                    local_hvigorw.display()
                );
            }
        }
    }

    /// Find HAR file and copy to output directory
    fn find_and_copy_har_to_output(
        ctx: &BuildContext,
        ohos_project: &PathBuf,
        output_dir: &PathBuf,
    ) -> Result<()> {
        let har_dir = ohos_project.join("main_ohos_sdk/build/default/outputs/default");

        let har_glob_pattern = har_dir.join("*.har");
        let har_files: Vec<_> = glob::glob(har_glob_pattern.to_str().unwrap())
            .context("Failed to glob HAR files")?
            .filter_map(|p| p.ok())
            .collect();

        if har_files.is_empty() {
            bail!(
                "No HAR file found after Hvigor assemble in {}",
                har_dir.display()
            );
        }

        let project_name_upper = ctx.lib_name().to_uppercase();
        let dest_name = format!("{}_OHOS_SDK-{}.har", project_name_upper, ctx.version());
        let dest = output_dir.join(&dest_name);

        if let Some(har_file) = har_files.first() {
            std::fs::copy(har_file, &dest).with_context(|| {
                format!(
                    "Failed to copy HAR from {} to {}",
                    har_file.display(),
                    dest.display()
                )
            })?;

            if ctx.options.verbose {
                eprintln!("  HAR generated: {}", dest.display());
            }
        }

        Ok(())
    }

    /// Build HAR package using hvigorw assembleHar task
    fn build_har(&self, ctx: &BuildContext, _abis: &[OhosAbi], output_dir: &PathBuf) -> Result<()> {
        let ohos_project = ctx.project_root.join("ohos");
        if !ohos_project.exists() {
            bail!(
                "OHOS Hvigor project not found at {}",
                ohos_project.display()
            );
        }

        // Sync CCGO.toml version -> ohos/main_ohos_sdk/oh-package.json5
        crate::utils::version_sync::sync_oh_package_version(
            &ohos_project.join("main_ohos_sdk").join("oh-package.json5"),
            ctx.version(),
        );

        let hvigorw_cmd = Self::find_hvigorw_cmd(ctx, &ohos_project)?;

        eprintln!("  Running Hvigor assembleHar task...");

        let status = std::process::Command::new(&hvigorw_cmd)
            .arg("assembleHar")
            .current_dir(&ohos_project)
            .spawn()
            .with_context(|| format!("Failed to spawn Hvigor: {}", hvigorw_cmd))?
            .wait()
            .with_context(|| "Failed to wait for Hvigor process")?;

        if !status.success() {
            bail!(
                "Hvigor assembleHar failed with exit code: {:?}",
                status.code()
            );
        }

        OhosBuilder::find_and_copy_har_to_output(ctx, &ohos_project, output_dir)?;

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

impl PlatformBuilder for OhosBuilder {
    fn platform_name(&self) -> &str {
        "ohos"
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
            bail!("CMake is required for OHOS builds. Please install CMake.");
        }

        // Check for OHOS SDK
        let sdk = OhosSdkToolchain::detect().context(
            "OHOS SDK is required. Please set OHOS_SDK_HOME or HOS_SDK_HOME environment variable.",
        )?;

        sdk.validate()?;

        if ctx.options.verbose {
            eprintln!(
                "Using OHOS SDK {} at {}",
                sdk.version(),
                sdk.path().unwrap().display()
            );
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        self.validate_prerequisites(ctx)?;

        let sdk = OhosSdkToolchain::detect()?;

        if ctx.options.verbose {
            eprintln!("Building {} for OHOS...", ctx.lib_name());
        }

        // Source-only deps: ensure they have artifacts before we compose link lines.
        // (Skips deps whose fingerprint matches and whose lib/<platform>/ already
        // has artifacts on disk; spawns `ccgo build` recursively otherwise.)
        ctx.materialize_source_deps(self.platform_name())?;

        let abis = Self::resolve_abis(ctx)?;
        let min_sdk_version = DEFAULT_MIN_SDK_VERSION;

        std::fs::create_dir_all(&ctx.output_dir)?;

        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "ohos",
            ctx.output_dir.clone(),
        )?;

        let symbols_staging = ctx.output_dir.join(".symbols_staging");
        std::fs::create_dir_all(&symbols_staging)?;

        let built_link_types = self.build_static_and_shared(
            ctx,
            &sdk,
            &abis,
            min_sdk_version,
            &archive,
            &symbols_staging,
        )?;

        let include_source = ctx.include_source_dir();
        if include_source.exists() {
            let include_path = get_unified_include_path(ctx.lib_name(), &include_source);
            archive.add_directory(&include_source, &include_path)?;
            if ctx.options.verbose {
                eprintln!(
                    "Added include files from {} to {}",
                    include_source.display(),
                    include_path
                );
            }
        }

        if built_link_types.contains(&"shared") {
            self.build_har_package(ctx, &abis);
        }

        self.add_har_to_archive_if_needed(ctx, &archive)?;

        let architectures: Vec<String> = abis.iter().map(|a| a.abi_string().to_string()).collect();
        let link_type_str = ctx.options.link_type.to_string();
        let sdk_archive = archive.create_sdk_archive(&architectures, &link_type_str)?;

        let symbols_archive = self.create_symbols_archive_if_needed(
            ctx,
            &built_link_types,
            &symbols_staging,
            &archive,
        )?;

        std::fs::remove_dir_all(&symbols_staging).ok();

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "OHOS build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive,
            aar_archive: Self::get_har_archive_path(ctx),
            duration_secs: duration.as_secs_f64(),
            architectures,
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean new directory structure: cmake_build/{release|debug}/ohos
        for subdir in &["release", "debug"] {
            let build_dir = ctx
                .project_root
                .join("cmake_build")
                .join(subdir)
                .join("ohos");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/OHOS, cmake_build/ohos
        for old_dir in &[
            ctx.project_root.join("cmake_build/OHOS"),
            ctx.project_root.join("cmake_build/ohos"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        // Clean target directories
        for old_dir in &[
            ctx.project_root.join("target/release/ohos"),
            ctx.project_root.join("target/debug/ohos"),
            ctx.project_root.join("target/release/OHOS"),
            ctx.project_root.join("target/debug/OHOS"),
            ctx.project_root.join("target/ohos"),
            ctx.project_root.join("target/OHOS"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

// Non-trait helpers used by the `build` impl above. Kept in an inherent
// impl — `PlatformBuilder` only exposes 5 methods.
impl OhosBuilder {
    /// Resolve ABIs from build options
    fn resolve_abis(ctx: &BuildContext) -> Result<Vec<OhosAbi>> {
        if ctx.options.architectures.is_empty() {
            Ok(vec![
                OhosAbi::Arm64V8a,
                OhosAbi::ArmeabiV7a,
                OhosAbi::X86_64,
            ])
        } else {
            ctx.options
                .architectures
                .iter()
                .map(|s| Self::parse_abi(s))
                .collect()
        }
    }

    /// Build static and shared libraries, returning built link types
    fn build_static_and_shared(
        &self,
        ctx: &BuildContext,
        sdk: &OhosSdkToolchain,
        abis: &[OhosAbi],
        min_sdk_version: u32,
        archive: &ArchiveBuilder,
        symbols_staging: &PathBuf,
    ) -> Result<Vec<&'static str>> {
        let mut built_link_types = Vec::new();

        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let results = self.build_link_type(ctx, sdk, "static", abis, min_sdk_version)?;
            self.add_libraries_to_archive(archive, &results, "static", false, ctx.lib_name())?;
            built_link_types.push("static");
        }

        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            let results = self.build_link_type(ctx, sdk, "shared", abis, min_sdk_version)?;

            if ctx.options.verbose {
                eprintln!("Saving unstripped libraries to symbols staging...");
            }
            self.copy_unstripped_to_symbols(
                &results,
                ctx.lib_name(),
                symbols_staging,
                ctx.options.verbose,
            )?;

            if ctx.options.release {
                if ctx.options.verbose {
                    eprintln!("Stripping shared libraries...");
                }
                self.strip_shared_libraries(sdk, &results, ctx.lib_name(), ctx.options.verbose)?;
            }

            self.add_libraries_to_archive(archive, &results, "shared", true, ctx.lib_name())?;
            built_link_types.push("shared");

            if ctx.options.verbose {
                eprintln!("Copying libraries to libs for Hvigor...");
            }
            self.copy_libraries_to_libs(ctx, abis, ctx.lib_name())?;
        }

        Ok(built_link_types)
    }

    /// Build HAR package with error handling
    fn build_har_package(&self, ctx: &BuildContext, abis: &[OhosAbi]) {
        if ctx.options.verbose {
            eprintln!("Building HAR package...");
        }
        if let Err(e) = self.build_har(ctx, abis, &ctx.output_dir) {
            eprintln!(
                "Warning: Failed to build HAR: {}. Continuing without HAR.",
                e
            );
            eprintln!("To build HAR manually, run: cd ohos && ./hvigorw assembleHar");
        } else if ctx.options.verbose {
            eprintln!("HAR package built successfully");
        }
    }

    /// Add HAR to archive if it exists
    fn add_har_to_archive_if_needed(
        &self,
        ctx: &BuildContext,
        archive: &ArchiveBuilder,
    ) -> Result<()> {
        let project_name_upper = ctx.lib_name().to_uppercase();
        let har_versioned_name = format!("{}_OHOS_SDK-{}.har", project_name_upper, ctx.version());
        let har_path = ctx.output_dir.join(&har_versioned_name);

        if har_path.exists() {
            let har_dest = format!("haars/ohos/{}", har_versioned_name);
            archive.add_file(&har_path, &har_dest)?;
            if ctx.options.verbose {
                eprintln!("Added HAR to archive: {}", har_dest);
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
        let has_symbols = symbols_dir.exists()
            && std::fs::read_dir(&symbols_dir)
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

    /// Get HAR archive path if it exists
    fn get_har_archive_path(ctx: &BuildContext) -> Option<PathBuf> {
        let project_name_upper = ctx.lib_name().to_uppercase();
        let har_versioned_name = format!("{}_OHOS_SDK-{}.har", project_name_upper, ctx.version());
        let har_path = ctx.output_dir.join(&har_versioned_name);

        if har_path.exists() {
            Some(har_path)
        } else {
            None
        }
    }
}

impl Default for OhosBuilder {
    fn default() -> Self {
        Self::new()
    }
}

