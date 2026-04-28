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
use crate::build::toolchains::msvc::{is_msvc_available, MsvcToolchain};
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

    /// Remove all `.a` files in `out_dir` except for `keep_path`
    fn cleanup_module_libs(out_dir: &PathBuf, keep_path: &PathBuf) -> Result<()> {
        for entry in std::fs::read_dir(out_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && &path != keep_path {
                if let Some(ext) = path.extension() {
                    if ext == "a" {
                        let _ = std::fs::remove_file(&path);
                    }
                }
            }
        }
        Ok(())
    }

    /// Collect all `.a` files in `out_dir`, excluding `exclude_path`
    fn collect_a_files(dir: &PathBuf, exclude_path: Option<&PathBuf>) -> Result<Vec<PathBuf>> {
        let mut libs = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "a" {
                        if exclude_path.map_or(true, |excl| &path != excl) {
                            libs.push(path);
                        }
                    }
                }
            }
        }
        Ok(libs)
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

        let main_lib_name = format!("lib{}.a", lib_name);
        let main_lib_path = out_dir.join(&main_lib_name);

        // Check if the main library already exists and is non-empty (CMake merged it)
        if Self::main_lib_already_merged(&main_lib_path) {
            if verbose {
                eprintln!(
                    "    Main library {} already exists, skipping merge",
                    main_lib_name
                );
            }
            Self::cleanup_module_libs(&out_dir, &main_lib_path)?;
            return Ok(());
        }

        // Find all .a files (module libraries), excluding the main lib
        let mut module_libs = Self::collect_a_files(&out_dir, None)?;

        if module_libs.is_empty() {
            return Ok(());
        }

        // If the only file is already the main library, nothing to merge
        if module_libs.len() == 1
            && module_libs[0]
                .file_name()
                .is_some_and(|n| n == main_lib_name.as_str())
        {
            return Ok(());
        }

        // Remove the main library from the list (we will recreate it)
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

        mingw.merge_static_libs(&module_libs, &main_lib_path)?;

        // Clean up module libraries after merge
        for lib in &module_libs {
            if lib != &main_lib_path {
                let _ = std::fs::remove_file(lib);
            }
        }

        Ok(())
    }

    /// Returns true when the main library already exists and is non-empty
    fn main_lib_already_merged(main_lib_path: &PathBuf) -> bool {
        if !main_lib_path.exists() {
            return false;
        }
        std::fs::metadata(main_lib_path).map_or(false, |m| m.len() > 0)
    }

    /// Merge third-party static libs from cmake build root into the main lib (MinGW)
    fn merge_third_party_static_libs_mingw(
        &self,
        mingw: &MingwToolchain,
        build_dir: &PathBuf,
        lib_name: &str,
        verbose: bool,
    ) -> Result<()> {
        let out_dir = build_dir.join("out");
        let main_lib_path = out_dir.join(format!("lib{}.a", lib_name));
        if !main_lib_path.exists() {
            return Ok(());
        }

        let placeholder_name = format!("lib{}.a", lib_name);
        let third_party_libs = Self::collect_third_party_libs(build_dir, &placeholder_name)?;

        if third_party_libs.is_empty() {
            return Ok(());
        }

        if verbose {
            eprintln!(
                "    Merging {} third-party libs into {}",
                third_party_libs.len(),
                placeholder_name
            );
        }

        let mut all_libs = vec![main_lib_path.clone()];
        all_libs.extend(third_party_libs);
        mingw.merge_static_libs(&all_libs, &main_lib_path)?;

        Ok(())
    }

    /// Collect `.a` files in `dir` whose filename differs from `exclude_name`
    fn collect_third_party_libs(dir: &PathBuf, exclude_name: &str) -> Result<Vec<PathBuf>> {
        let mut libs = Vec::new();
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "a" {
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or_default();
                        if fname != exclude_name {
                            libs.push(path);
                        }
                    }
                }
            }
        }
        Ok(libs)
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

    /// Apply shared cmake options common to both MinGW and MSVC builds
    fn apply_common_cmake_options(
        cmake: CMakeConfig,
        ctx: &BuildContext,
        build_shared: bool,
    ) -> Result<CMakeConfig> {
        let mut cmake = cmake
            .variable("CCGO_BUILD_STATIC", if build_shared { "OFF" } else { "ON" })
            .variable("CCGO_BUILD_SHARED", if build_shared { "ON" } else { "OFF" })
            .variable(
                "CCGO_BUILD_SHARED_LIBS",
                if build_shared { "ON" } else { "OFF" },
            )
            .variable("CCGO_LIB_NAME", ctx.lib_name())
            .jobs(ctx.jobs())
            .verbose(ctx.options.verbose);

        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        cmake = cmake.variable(
            "CCGO_CONFIG_PRESET_VISIBILITY",
            ctx.symbol_visibility().to_string(),
        );

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

        Ok(cmake)
    }

    /// Build for a specific link type with MinGW
    fn build_with_mingw(
        &self,
        ctx: &BuildContext,
        mingw: &MingwToolchain,
        link_type: &str,
    ) -> Result<PathBuf> {
        let build_dir = ctx.cmake_build_dir.join(format!("{}/mingw", link_type));
        let install_dir = build_dir.join("install");
        let build_shared = link_type == "shared";

        let cmake_vars = mingw.cmake_variables_for_arch();

        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .generator("Unix Makefiles")
            .build_type(if ctx.options.release {
                BuildType::Release
            } else {
                BuildType::Debug
            })
            .install_prefix(install_dir.clone());

        cmake = Self::apply_common_cmake_options(cmake, ctx, build_shared)?;

        for (name, value) in cmake_vars {
            cmake = cmake.variable(&name, &value);
        }

        cmake.configure_build_install()?;

        // For static builds, merge all module libraries into a single library
        if !build_shared {
            self.merge_module_static_libs_mingw(
                mingw,
                &build_dir,
                ctx.lib_name(),
                ctx.options.verbose,
            )?;
            self.merge_third_party_static_libs_mingw(
                mingw,
                &build_dir,
                ctx.lib_name(),
                ctx.options.verbose,
            )?;
        }

        Ok(build_dir)
    }

    /// Build for a specific link type with MSVC
    /// Supports both native Windows (Visual Studio) and Linux (xwin + clang-cl)
    fn build_with_msvc(
        &self,
        ctx: &BuildContext,
        msvc: &MsvcToolchain,
        link_type: &str,
    ) -> Result<PathBuf> {
        let build_dir = ctx.cmake_build_dir.join(format!("{}/msvc", link_type));
        let install_dir = build_dir.join("install");
        let build_shared = link_type == "shared";

        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .generator(msvc.cmake_generator())
            .build_type(if ctx.options.release {
                BuildType::Release
            } else {
                BuildType::Debug
            })
            .install_prefix(install_dir.clone());

        for (name, value) in msvc.cmake_variables() {
            cmake = cmake.variable(&name, &value);
        }

        cmake = Self::apply_common_cmake_options(cmake, ctx, build_shared)?;

        cmake.configure_build_install()?;

        Ok(build_dir)
    }

    /// Scan `lib_dir` and add files with `extension` to `libs`, skipping duplicates by filename
    fn collect_libs_from_dir(
        lib_dir: &PathBuf,
        extension: &str,
        libs: &mut Vec<PathBuf>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(lib_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == extension {
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
        Ok(())
    }

    /// Scan `lib_dir` and add import library files (matching `import_ext` suffix) to `libs`
    fn collect_import_libs_from_dir(
        lib_dir: &PathBuf,
        import_ext: &str,
        libs: &mut Vec<PathBuf>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(lib_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_str().unwrap();
                if name.ends_with(import_ext)
                    && !libs
                        .iter()
                        .any(|p: &PathBuf| p.file_name() == path.file_name())
                {
                    libs.push(path);
                }
            }
        }
        Ok(())
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

        // Prioritize out/ directory (merged library), then fall back to install/lib, lib, bin
        let possible_dirs = [
            build_dir.join("out"),
            build_dir.join("install/lib"),
            build_dir.join("lib"),
            build_dir.join("bin"),
        ];

        for lib_dir in &possible_dirs {
            if !lib_dir.exists() {
                continue;
            }
            Self::collect_libs_from_dir(lib_dir, extension, &mut libs)?;
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

            for lib_dir in &[
                build_dir.join("out"),
                build_dir.join("install/lib"),
                build_dir.join("lib"),
            ] {
                if !lib_dir.exists() {
                    continue;
                }
                Self::collect_import_libs_from_dir(lib_dir, import_ext, &mut libs)?;
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
            WindowsToolchain::MSVC => {
                let msvc = MsvcToolchain::detect()?;
                self.build_with_msvc(ctx, &msvc, link_type)
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

    /// Run `cmake -S … -B … -G …` and return the exit status
    fn run_cmake_configure(ctx: &BuildContext, build_dir: &PathBuf, generator: &str) -> Result<()> {
        use std::process::Command;

        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd
            .arg("-S")
            .arg(&ctx.project_root)
            .arg("-B")
            .arg(build_dir)
            .arg("-G")
            .arg(generator)
            .arg("-DCMAKE_EXPORT_COMPILE_COMMANDS=ON");

        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake_cmd.arg(format!("-DCCGO_CMAKE_DIR={}", cmake_dir.display()));
        }

        cmake_cmd.arg(format!("-DCCGO_LIB_NAME={}", ctx.lib_name()));

        if ctx.options.verbose {
            eprintln!("CMake configure: {:?}", cmake_cmd);
        }

        let status = cmake_cmd
            .status()
            .context("Failed to run CMake configure")?;
        if !status.success() {
            bail!("CMake configure failed");
        }

        Ok(())
    }

    /// Print the location of generated IDE project files
    fn report_project_files(ctx: &BuildContext, build_dir: &PathBuf) {
        let sln_file = build_dir.join(format!("{}.sln", ctx.lib_name()));
        let workspace_file = build_dir.join(format!("{}.workspace", ctx.lib_name()));

        if sln_file.exists() {
            eprintln!(
                "\n✓ Visual Studio solution generated: {}",
                sln_file.display()
            );

            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("cmd")
                    .args(["/C", "start", ""])
                    .arg(&sln_file)
                    .status();
            }
        } else if workspace_file.exists() {
            eprintln!(
                "\n✓ CodeLite workspace generated: {}",
                workspace_file.display()
            );
        } else {
            eprintln!(
                "\n✓ IDE project files generated in: {}",
                build_dir.display()
            );
        }

        let compile_commands = build_dir.join("compile_commands.json");
        if compile_commands.exists() {
            eprintln!("   compile_commands.json: {}", compile_commands.display());
        }
    }

    /// Generate Visual Studio IDE project for Windows
    pub fn generate_ide_project(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let build_dir = ctx.cmake_build_dir.join("ide_project");

        // Clean build directory
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)
                .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
        }

        // Create build directory
        std::fs::create_dir_all(&build_dir)
            .with_context(|| format!("Failed to create {}", build_dir.display()))?;

        // Determine generator based on available toolchain
        let (generator, toolchain_name) = if is_msvc_available() {
            ("Visual Studio 17 2022", "MSVC")
        } else if is_mingw_available() {
            ("CodeLite - MinGW Makefiles", "MinGW")
        } else {
            bail!(
                "No Windows toolchain found for IDE project generation.\n\
                 - For Visual Studio: Install Visual Studio with C++ tools\n\
                 - For MinGW: Install MinGW-w64"
            );
        };

        eprintln!(
            "Generating {} project for Windows in {}...",
            toolchain_name,
            build_dir.display()
        );

        Self::run_cmake_configure(ctx, &build_dir, generator)?;
        Self::report_project_files(ctx, &build_dir);

        Ok(BuildResult {
            sdk_archive: build_dir,
            symbols_archive: None,
            aar_archive: None,
            duration_secs: 0.0,
            architectures: vec![],
        })
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

    /// Validate the MinGW toolchain and optionally print its version
    fn validate_mingw_toolchain(verbose: bool) -> Result<()> {
        let mingw = MingwToolchain::detect()?;
        mingw.validate()?;
        if verbose {
            eprintln!(
                "Using MinGW-w64 {} at {}",
                mingw.version(),
                mingw.path().unwrap().display()
            );
        }
        Ok(())
    }

    /// Validate the MSVC toolchain and optionally print its version
    fn validate_msvc_toolchain(verbose: bool) -> Result<()> {
        let msvc = MsvcToolchain::detect()?;
        msvc.validate()?;
        if verbose {
            eprintln!(
                "Using MSVC {} at {}",
                msvc.version(),
                msvc.path().unwrap().display()
            );
        }
        Ok(())
    }

    /// Build and archive the static link type
    fn build_and_archive_static(
        &self,
        ctx: &BuildContext,
        archive: &ArchiveBuilder,
        toolchain: WindowsToolchain,
    ) -> Result<()> {
        let build_dir = self.build_link_type(ctx, "static", toolchain)?;
        self.add_libraries_to_archive(archive, &build_dir, "static", false, toolchain)
    }

    /// Build and archive the shared link type, stripping when appropriate
    fn build_and_archive_shared(
        &self,
        ctx: &BuildContext,
        archive: &ArchiveBuilder,
        toolchain: WindowsToolchain,
    ) -> Result<()> {
        let build_dir = self.build_link_type(ctx, "shared", toolchain)?;

        // Strip shared libraries for release builds (MinGW only)
        if ctx.options.release && toolchain == WindowsToolchain::MinGW {
            if ctx.options.verbose {
                eprintln!("Stripping shared libraries...");
            }
            let mingw = MingwToolchain::detect()?;
            self.strip_libraries(&mingw, &build_dir, ctx.options.verbose)?;
        }

        self.add_libraries_to_archive(archive, &build_dir, "shared", true, toolchain)
    }

    /// Add include files from the project's include directory into the archive
    fn add_include_files(ctx: &BuildContext, archive: &ArchiveBuilder) -> Result<()> {
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
        Ok(())
    }

    /// Remove a list of directories if they exist, surfacing errors with context
    fn remove_dirs_if_exist(dirs: &[PathBuf]) -> Result<()> {
        for dir in dirs {
            if dir.exists() {
                std::fs::remove_dir_all(dir)
                    .with_context(|| format!("Failed to clean {}", dir.display()))?;
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

        let toolchain = Self::detect_toolchain()?;

        match toolchain {
            WindowsToolchain::MinGW => Self::validate_mingw_toolchain(ctx.options.verbose),
            WindowsToolchain::MSVC => Self::validate_msvc_toolchain(ctx.options.verbose),
        }
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        // Check for IDE project generation mode
        if ctx.options.ide_project {
            return self.generate_ide_project(ctx);
        }

        let start = Instant::now();
        self.validate_prerequisites(ctx)?;

        let toolchain = Self::detect_toolchain()?;

        if ctx.options.verbose {
            eprintln!("Building {} for Windows...", ctx.lib_name());
        }

        // Source-only deps: ensure they have artifacts before we compose link lines.
        // (Skips deps whose fingerprint matches and whose lib/<platform>/ already
        // has artifacts on disk; spawns `ccgo build` recursively otherwise.)
        ctx.materialize_source_deps(self.platform_name())?;

        std::fs::create_dir_all(&ctx.output_dir)?;

        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "windows",
            ctx.output_dir.clone(),
        )?;

        let mut built_link_types = Vec::new();

        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            self.build_and_archive_static(ctx, &archive, toolchain)?;
            built_link_types.push("static");
        }

        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            self.build_and_archive_shared(ctx, &archive, toolchain)?;
            built_link_types.push("shared");
        }

        Self::add_include_files(ctx, &archive)?;

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
        let new_build_dirs: Vec<PathBuf> = ["release", "debug"]
            .iter()
            .map(|s| ctx.project_root.join("cmake_build").join(s).join("windows"))
            .collect();
        Self::remove_dirs_if_exist(&new_build_dirs)?;

        // Clean old structure for backwards compatibility
        let old_build_dirs = [
            ctx.project_root.join("cmake_build/Windows"),
            ctx.project_root.join("cmake_build/windows"),
        ];
        Self::remove_dirs_if_exist(&old_build_dirs)?;

        // Clean target directories
        let target_dirs = [
            ctx.project_root.join("target/release/windows"),
            ctx.project_root.join("target/debug/windows"),
            ctx.project_root.join("target/release/Windows"),
            ctx.project_root.join("target/debug/Windows"),
            ctx.project_root.join("target/windows"),
            ctx.project_root.join("target/Windows"),
        ];
        Self::remove_dirs_if_exist(&target_dirs)?;

        Ok(())
    }
}

impl Default for WindowsBuilder {
    fn default() -> Self {
        Self::new()
    }
}
