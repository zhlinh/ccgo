//! Build command implementation

use std::path::Path;

use anyhow::{bail, Result};
use clap::{Args, ValueEnum};

use crate::build::analytics::{
    count_files, get_artifact_size, get_cache_stats, BuildAnalytics, CacheStats, FileStats,
};
use crate::build::archive::{create_build_info_full, print_build_info_json};
use crate::build::platforms::{build_all, build_apple, get_builder};
use crate::build::{BuildContext, BuildOptions, BuildResult};
use crate::config::CcgoConfig;
use crate::workspace::{find_workspace_root, Workspace};

/// Target platform for building
#[derive(Debug, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum BuildTarget {
    /// Build for all platforms
    All,
    /// Build for all Apple platforms
    Apple,
    /// Android platform
    Android,
    /// iOS platform
    Ios,
    /// macOS platform
    Macos,
    /// Windows platform
    Windows,
    /// Linux platform
    Linux,
    /// OpenHarmony platform
    Ohos,
    /// tvOS platform
    Tvos,
    /// watchOS platform
    Watchos,
    /// Kotlin Multiplatform
    Kmp,
    /// Conan package
    Conan,
}

impl std::fmt::Display for BuildTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTarget::All => write!(f, "all"),
            BuildTarget::Apple => write!(f, "apple"),
            BuildTarget::Android => write!(f, "Android"), // Capital A to match Python pyccgo
            BuildTarget::Ios => write!(f, "ios"),
            BuildTarget::Macos => write!(f, "macos"),
            BuildTarget::Windows => write!(f, "windows"),
            BuildTarget::Linux => write!(f, "linux"),
            BuildTarget::Ohos => write!(f, "ohos"),
            BuildTarget::Tvos => write!(f, "tvos"),
            BuildTarget::Watchos => write!(f, "watchos"),
            BuildTarget::Kmp => write!(f, "kmp"),
            BuildTarget::Conan => write!(f, "conan"),
        }
    }
}

/// Parse the full value of the `--arch` flag into a concrete architecture list.
///
/// * Splits on commas, trims whitespace, lowercases.
/// * Expands shorthand aliases via [`normalize_arch_alias`].
/// * When any token resolves to `all`, returns an empty `Vec` — which signals
///   downstream platform builders to use their `default_architectures()`, i.e.
///   "every arch this platform builds by default". This matches what a user
///   expects from `--arch all` without duplicating the default list here.
pub fn parse_arch_arg(raw: &str, target: &BuildTarget) -> Vec<String> {
    let tokens: Vec<String> = raw
        .split(',')
        .map(|a| normalize_arch_alias(a, target))
        .filter(|a| !a.is_empty())
        .collect();

    if tokens.iter().any(|t| t == "all") {
        Vec::new()
    } else {
        tokens
    }
}

/// Normalize a user-supplied `--arch` token into the canonical name that
/// the target platform's toolchain understands.
///
/// Accepts shorthand aliases (case-insensitive) that map differently per
/// platform — `v8` means `arm64-v8a` on Android/OHOS but `arm64` on
/// macOS/iOS. Unrecognized strings pass through unchanged so the
/// platform's own validator produces the final error message.
pub fn normalize_arch_alias(raw: &str, target: &BuildTarget) -> String {
    let lower = raw.trim().to_lowercase();
    match target {
        BuildTarget::Android | BuildTarget::Ohos => match lower.as_str() {
            "v8" | "a64" | "arm64" | "armv8" | "aarch64" => "arm64-v8a".to_string(),
            "v7" | "a32" | "arm32" | "armv7" | "aarch32" => "armeabi-v7a".to_string(),
            "x64" => "x86_64".to_string(),
            _ => lower,
        },
        BuildTarget::Macos | BuildTarget::Ios | BuildTarget::Tvos | BuildTarget::Watchos => {
            match lower.as_str() {
                "v8" | "a64" | "armv8" | "aarch64" => "arm64".to_string(),
                "x64" => "x86_64".to_string(),
                _ => lower,
            }
        }
        BuildTarget::Linux | BuildTarget::Windows => match lower.as_str() {
            "x64" => "x86_64".to_string(),
            _ => lower,
        },
        // Meta-targets (All/Apple/Kmp/Conan) delegate to individual platforms
        // during dispatch; pass the token through and let the per-platform
        // step re-normalize against its concrete target.
        BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => lower,
    }
}

/// Library linking type
#[derive(Debug, Clone, Default, ValueEnum, PartialEq)]
pub enum LinkType {
    /// Static library only
    Static,
    /// Shared/dynamic library only
    Shared,
    /// Both static and shared
    #[default]
    Both,
}

impl std::fmt::Display for LinkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkType::Static => write!(f, "static"),
            LinkType::Shared => write!(f, "shared"),
            LinkType::Both => write!(f, "both"),
        }
    }
}

/// Windows toolchain selection
#[derive(Debug, Clone, Default, ValueEnum)]
pub enum WindowsToolchain {
    /// MSVC toolchain
    Msvc,
    /// MinGW toolchain
    Mingw,
    /// Auto-detect (both)
    #[default]
    Auto,
}

impl std::fmt::Display for WindowsToolchain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WindowsToolchain::Msvc => write!(f, "msvc"),
            WindowsToolchain::Mingw => write!(f, "mingw"),
            WindowsToolchain::Auto => write!(f, "auto"),
        }
    }
}

/// Build library for specific platform
#[derive(Args, Debug)]
pub struct BuildCommand {
    /// Target platform to build
    #[arg(value_enum)]
    pub target: BuildTarget,

    /// Architectures to build (comma-separated)
    ///
    /// Canonical values per platform:
    ///   * Android  arm64-v8a, armeabi-v7a, x86_64
    ///   * OHOS     arm64-v8a, armeabi-v7a, x86_64
    ///   * macOS    arm64, x86_64   (both by default — universal)
    ///   * iOS      arm64, x86_64   (device + sim built automatically; flag is a no-op)
    ///   * Linux    x86_64          (flag is a no-op)
    ///   * Windows  x86_64          (flag is a no-op)
    ///
    /// Accepted aliases (case-insensitive):
    ///   all                              ->  every arch this platform builds by default
    ///   v8, a64, arm64, armv8, aarch64   ->  arm64-v8a (Android/OHOS) | arm64 (macOS/iOS)
    ///   v7, a32, arm32, armv7, aarch32   ->  armeabi-v7a (Android/OHOS)
    ///   x64                              ->  x86_64
    ///
    /// If omitted entirely, the same platform defaults apply (equivalent to `--arch all`).
    ///
    /// Examples:
    ///   --arch all          (every default arch — same as omitting the flag)
    ///   --arch v8           (arm64-v8a on Android/OHOS)
    ///   --arch v8,v7,x64    (all three ABIs on Android/OHOS)
    ///   --arch a64,x64      (universal macOS)
    #[arg(long, verbatim_doc_comment)]
    pub arch: Option<String>,

    /// Link type
    #[arg(long, value_enum, default_value_t = LinkType::Both)]
    pub link_type: LinkType,

    /// Build all workspace members
    #[arg(long)]
    pub workspace: bool,

    /// Build only the specified package (in a workspace)
    #[arg(long, short = 'p')]
    pub package: Option<String>,

    /// Build using Docker container
    #[arg(long)]
    pub docker: bool,

    /// Automatically use Docker when native build is not possible
    ///
    /// For example, on macOS building for Linux or Windows requires Docker.
    /// This flag will automatically detect and use Docker when needed.
    #[arg(long)]
    pub auto_docker: bool,

    /// Number of parallel jobs
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Generate IDE project files
    #[arg(long)]
    pub ide_project: bool,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Build only native libraries without packaging (AAR/HAR)
    #[arg(long)]
    pub native_only: bool,

    /// Windows toolchain selection
    #[arg(long, value_enum, default_value_t = WindowsToolchain::Auto)]
    pub toolchain: WindowsToolchain,

    /// Development mode: use pre-built ccgo binary from GitHub releases in Docker builds
    #[arg(long)]
    pub dev: bool,

    /// Features to enable (comma-separated)
    ///
    /// Example: --features networking,advanced
    #[arg(long, short = 'F', value_delimiter = ',')]
    pub features: Vec<String>,

    /// Do not enable default features
    ///
    /// By default, the features listed in [features].default are enabled.
    /// Use this flag to disable them.
    #[arg(long)]
    pub no_default_features: bool,

    /// Enable all available features
    #[arg(long)]
    pub all_features: bool,

    /// Compiler cache to use (ccache, sccache, auto, none)
    ///
    /// Default: auto - automatically detect and use available cache
    /// Use 'none' to disable caching
    #[arg(long, default_value = "auto")]
    pub cache: String,

    /// Show build analytics summary after build
    ///
    /// Displays timing breakdown, cache statistics, and file metrics.
    /// Analytics data is saved to ~/.ccgo/analytics/ for historical tracking.
    #[arg(long)]
    pub analytics: bool,
}

impl BuildCommand {
    /// Check if a platform can be built natively on the current host
    fn can_build_natively(target: &BuildTarget) -> bool {
        let host_os = std::env::consts::OS;

        match target {
            // All/Apple/Kmp/Conan are meta-targets, check individual platforms
            BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => true,

            // Linux can only be built natively on Linux
            BuildTarget::Linux => host_os == "linux",

            // Windows can only be built natively on Windows
            BuildTarget::Windows => host_os == "windows",

            // Apple platforms can only be built on macOS (Xcode required)
            BuildTarget::Macos | BuildTarget::Ios | BuildTarget::Tvos | BuildTarget::Watchos => {
                host_os == "macos"
            }

            // Android can be built on any platform with NDK
            BuildTarget::Android => true,

            // OHOS can be built on any platform with OHOS SDK
            BuildTarget::Ohos => true,
        }
    }

    /// Determine if Docker should be used for this build
    fn should_use_docker(&self, target: &BuildTarget) -> bool {
        // Explicit --docker flag always uses Docker
        if self.docker {
            return true;
        }

        // --auto-docker enables automatic Docker detection
        if self.auto_docker && !Self::can_build_natively(target) {
            return true;
        }

        false
    }

    /// Create build options from command arguments
    fn create_build_options(&self, verbose: bool) -> BuildOptions {
        let architectures = self
            .arch
            .clone()
            .map(|s| parse_arch_arg(&s, &self.target))
            .unwrap_or_default();

        let use_docker = self.should_use_docker(&self.target);

        BuildOptions {
            target: self.target.clone(),
            architectures,
            link_type: self.link_type.clone(),
            use_docker,
            auto_docker: self.auto_docker,
            jobs: self.jobs,
            ide_project: self.ide_project,
            release: self.release,
            native_only: self.native_only,
            toolchain: self.toolchain.clone(),
            verbose,
            dev: self.dev,
            features: self.features.clone(),
            use_default_features: !self.no_default_features,
            all_features: self.all_features,
            cache: Some(self.cache.clone()),
            analytics: self.analytics,
        }
    }

    /// Handle Docker build execution
    fn handle_docker_build(
        &self,
        ctx: BuildContext,
        package: &crate::config::PackageConfig,
        verbose: bool,
    ) -> Result<()> {
        use crate::build::docker::DockerBuilder;

        match self.target {
            BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => {
                if self.auto_docker {
                    eprintln!(
                        "ℹ Auto-docker with '{}' will build each platform with Docker as needed",
                        self.target
                    );
                    Ok(())
                } else {
                    bail!(
                        "Docker builds are not supported for '{}' target.\n\n\
                         Docker builds support: linux, windows, macos, ios, tvos, watchos, android\n\
                         Build these platforms individually with --docker flag.\n\
                         Or use --auto-docker to automatically use Docker when needed.",
                        self.target
                    )
                }
            }
            _ => {
                let docker_project_root = ctx.project_root.clone();
                let cache_tool = ctx.compiler_cache().map(|c| c.tool_name().to_string());
                let jobs = ctx.jobs();

                let docker_builder = DockerBuilder::new(ctx)?;
                let result = docker_builder.execute()?;

                Self::print_results(
                    &package.name,
                    &package.version,
                    &self.target.to_string(),
                    &docker_project_root,
                    &[result],
                    verbose,
                    self.analytics,
                    cache_tool.as_deref(),
                    jobs,
                );
                Ok(())
            }
        }
    }

    /// Execute native build (non-Docker)
    fn execute_native_build(
        &self,
        ctx: &BuildContext,
        package: &crate::config::PackageConfig,
        verbose: bool,
    ) -> Result<()> {
        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            if verbose {
                eprintln!("Using CCGO cmake directory: {}", cmake_dir.display());
            }
        } else {
            eprintln!(
                "Warning: CCGO cmake directory not found. Set CCGO_CMAKE_DIR environment variable \
                 or install the ccgo package."
            );
        }

        let cache_tool = ctx.compiler_cache().map(|c| c.tool_name().to_string());
        let jobs = ctx.jobs();

        let results = match self.target {
            BuildTarget::All => build_all(ctx)?,
            BuildTarget::Apple => build_apple(ctx)?,
            _ => {
                let builder = get_builder(&self.target)?;
                vec![builder.build(ctx)?]
            }
        };

        Self::print_results(
            &package.name,
            &package.version,
            &self.target.to_string(),
            &ctx.project_root,
            &results,
            verbose,
            self.analytics,
            cache_tool.as_deref(),
            jobs,
        );

        Ok(())
    }

    /// Execute the build command
    pub fn execute(self, verbose: bool) -> Result<()> {
        let current_dir = std::env::current_dir()?;

        if self.workspace || self.package.is_some() {
            return self.execute_workspace_build(&current_dir, verbose);
        }

        if Workspace::is_workspace(&current_dir) {
            eprintln!(
                "ℹ️  In workspace root. Use --workspace to build all members, \
                 or --package <name> to build a specific member."
            );
        }

        let config = CcgoConfig::load()?;
        let project_root = current_dir;
        let package = config.require_package()?.clone();

        if verbose {
            eprintln!("Building {} for {} platform...", package.name, self.target);
        }

        let use_docker = self.should_use_docker(&self.target);

        if self.auto_docker && use_docker && !self.docker {
            let host_os = std::env::consts::OS;
            eprintln!(
                "🐳 Auto-docker: {} cannot be built natively on {} - using Docker",
                self.target, host_os
            );
        }

        let options = self.create_build_options(verbose);
        let ctx = BuildContext::new(project_root, config, options);

        if use_docker {
            return self.handle_docker_build(ctx, &package, verbose);
        }

        self.execute_native_build(&ctx, &package, verbose)
    }

    /// Execute build for workspace members
    fn execute_workspace_build(&self, current_dir: &Path, verbose: bool) -> Result<()> {
        // Find workspace root
        let workspace_root = find_workspace_root(current_dir)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Not in a workspace. Use --workspace or --package only within a workspace."
            )
        })?;

        // Load workspace
        let workspace = Workspace::load(&workspace_root)?;

        if verbose {
            workspace.print_summary();
        }

        // Determine which members to build
        let members_to_build = if let Some(ref package_name) = self.package {
            // Build specific package
            let member = workspace.get_member(package_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Package '{}' not found in workspace. Available: {}",
                    package_name,
                    workspace.members.names().join(", ")
                )
            })?;
            vec![member]
        } else {
            // Build default members (or all if no default_members specified)
            workspace.default_members()
        };

        if members_to_build.is_empty() {
            bail!("No workspace members to build");
        }

        eprintln!("\n{}", "=".repeat(80));
        eprintln!(
            "CCGO Workspace Build - Building {} member(s)",
            members_to_build.len()
        );
        eprintln!("{}", "=".repeat(80));

        let mut all_results: Vec<(String, Vec<BuildResult>)> = Vec::new();
        let mut failed_members: Vec<String> = Vec::new();

        for member in members_to_build {
            eprintln!("\n📦 Building {} ({})...", member.name, member.version);
            eprintln!("{}", "-".repeat(60));

            // Execute build in member's directory
            match self.build_member(&workspace_root, member, verbose) {
                Ok(results) => {
                    all_results.push((member.name.clone(), results));
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to build {}: {}", member.name, e);
                    failed_members.push(member.name.clone());
                }
            }
        }

        // Print summary
        eprintln!("\n{}", "=".repeat(80));
        eprintln!("Workspace Build Summary");
        eprintln!("{}", "=".repeat(80));

        let success_count = all_results.len();
        let fail_count = failed_members.len();

        eprintln!("\n✓ Successfully built: {}", success_count);
        for (name, results) in &all_results {
            let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();
            eprintln!("  - {} ({:.2}s)", name, total_duration);
        }

        if !failed_members.is_empty() {
            eprintln!("\n✗ Failed: {}", fail_count);
            for name in &failed_members {
                eprintln!("  - {}", name);
            }
            bail!("{} workspace member(s) failed to build", fail_count);
        }

        Ok(())
    }

    /// Execute build based on target
    fn execute_build_by_target(&self, ctx: &BuildContext) -> Result<Vec<BuildResult>> {
        match self.target {
            BuildTarget::All => build_all(ctx),
            BuildTarget::Apple => build_apple(ctx),
            _ => {
                let builder = get_builder(&self.target)?;
                Ok(vec![builder.build(ctx)?])
            }
        }
    }

    /// Execute Docker or native build for workspace member
    ///
    /// Takes `ctx` by value so it can be handed to `DockerBuilder::new`,
    /// which stores the context internally. Native paths only need a
    /// borrow, so we pass `&ctx` through to `execute_build_by_target`.
    fn execute_member_build(&self, ctx: BuildContext) -> Result<Vec<BuildResult>> {
        if self.should_use_docker(&self.target) {
            use crate::build::docker::DockerBuilder;

            match self.target {
                BuildTarget::All | BuildTarget::Apple | BuildTarget::Kmp | BuildTarget::Conan => {
                    self.execute_build_by_target(&ctx)
                }
                _ => {
                    let docker_builder = DockerBuilder::new(ctx)?;
                    Ok(vec![docker_builder.execute()?])
                }
            }
        } else {
            self.execute_build_by_target(&ctx)
        }
    }

    /// Build a single workspace member
    fn build_member(
        &self,
        workspace_root: &Path,
        member: &crate::workspace::WorkspaceMember,
        verbose: bool,
    ) -> Result<Vec<BuildResult>> {
        let member_path = workspace_root.join(&member.name);
        let config_path = member_path.join("CCGO.toml");
        let config = CcgoConfig::load_from(&config_path)?;
        let package = config.require_package()?.clone();

        let options = self.create_build_options(verbose);
        let ctx = BuildContext::new(member_path.clone(), config, options);

        let cache_tool = ctx.compiler_cache().map(|c| c.tool_name().to_string());
        let jobs = ctx.jobs();

        let results = self.execute_member_build(ctx)?;

        Self::print_results(
            &package.name,
            &package.version,
            &self.target.to_string(),
            &member_path,
            &results,
            verbose,
            self.analytics,
            cache_tool.as_deref(),
            jobs,
        );

        Ok(results)
    }

    /// Print archive tree for a single result
    fn print_archive_tree(result: &BuildResult) {
        if let Err(e) = crate::build::archive::print_zip_tree(&result.sdk_archive, "      ") {
            eprintln!("      Warning: Failed to print archive contents: {}", e);
        }

        if let Some(symbols_path) = &result.symbols_archive {
            eprintln!("\n      Symbols archive:");
            if let Err(e) = crate::build::archive::print_zip_tree(symbols_path, "      ") {
                eprintln!("      Warning: Failed to print symbols archive contents: {}", e);
            }
        }

        if let Some(archive_path) = &result.aar_archive {
            let archive_type = if archive_path.extension().is_some_and(|e| e == "har") {
                "HAR"
            } else {
                "AAR"
            };
            eprintln!("\n      {} contents:", archive_type);
            if let Err(e) = crate::build::archive::print_zip_tree(archive_path, "      ") {
                eprintln!("      Warning: Failed to print {} contents: {}", archive_type, e);
            }
        }
    }

    /// Print result details for a single build
    fn print_result_details(result: &BuildResult) {
        let is_ide_project = result.sdk_archive.is_dir();

        if is_ide_project {
            eprintln!("  IDE Project: {}", result.sdk_archive.display());
        } else {
            eprintln!("  SDK: {}", result.sdk_archive.display());
            if let Some(symbols) = &result.symbols_archive {
                eprintln!("  Symbols: {}", symbols.display());
            }
            if let Some(aar) = &result.aar_archive {
                eprintln!("  AAR: {}", aar.display());
            }
            Self::print_archive_tree(result);
        }
    }

    /// Print build results summary
    #[allow(clippy::too_many_arguments)]
    fn print_results(
        lib_name: &str,
        version: &str,
        platform: &str,
        project_root: &Path,
        results: &[BuildResult],
        verbose: bool,
        show_analytics: bool,
        cache_tool: Option<&str>,
        jobs: usize,
    ) {
        let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();

        if results.is_empty() {
            eprintln!("No builds completed.");
            return;
        }

        if verbose {
            eprintln!("\n=== Build Summary ===");
            for result in results {
                eprintln!(
                    "  {} ({:.2}s): {}",
                    result.architectures.join(", "),
                    result.duration_secs,
                    result.sdk_archive.display()
                );
            }
        }

        let build_info = create_build_info_full(lib_name, version, platform, project_root);
        print_build_info_json(&build_info);

        eprintln!("\n✓ {} built successfully in {:.2}s", lib_name, total_duration);

        for result in results {
            Self::print_result_details(result);
        }

        if show_analytics {
            Self::collect_and_display_analytics(lib_name, platform, project_root, results, cache_tool, jobs);
        }
    }

    /// Collect and display build analytics
    fn collect_and_display_analytics(
        lib_name: &str,
        platform: &str,
        project_root: &Path,
        results: &[BuildResult],
        cache_tool: Option<&str>,
        jobs: usize,
    ) {
        let total_duration: f64 = results.iter().map(|r| r.duration_secs).sum();
        let _all_archs: Vec<String> = results
            .iter()
            .flat_map(|r| r.architectures.clone())
            .collect();

        // Collect file statistics
        let src_dir = project_root.join("src");
        let (source_files, header_files) = count_files(&src_dir).unwrap_or((0, 0));

        // Get artifact size (use first result's SDK archive)
        let artifact_size = results
            .first()
            .map(|r| get_artifact_size(&r.sdk_archive).unwrap_or(0))
            .unwrap_or(0);

        let file_stats = FileStats {
            source_files,
            header_files,
            total_lines: 0, // Would need to count lines
            artifact_size_bytes: artifact_size,
        };

        // Get cache statistics
        let cache_stats = cache_tool
            .and_then(|tool| get_cache_stats(Some(tool)))
            .unwrap_or(CacheStats {
                tool: cache_tool.map(|s| s.to_string()),
                hits: 0,
                misses: 0,
                hit_rate: 0.0,
            });

        // Create analytics record
        let analytics = BuildAnalytics {
            project: lib_name.to_string(),
            platform: platform.to_string(),
            timestamp: chrono::Local::now().to_rfc3339(),
            total_duration_secs: total_duration,
            phases: Vec::new(), // Phase timing would require deeper integration
            cache_stats,
            file_stats,
            parallel_jobs: jobs,
            peak_memory_mb: None,
            success: true,
            error_count: 0,
            warning_count: 0,
        };

        // Save analytics to history
        if let Err(e) = analytics.save() {
            eprintln!("\n⚠️  Failed to save build analytics: {}", e);
        }

        // Display analytics summary
        analytics.print_summary();

        // Show historical comparison if available
        if let Ok(Some(avg)) = BuildAnalytics::average_build_time(lib_name) {
            let diff = total_duration - avg;
            let pct = (diff / avg) * 100.0;
            if diff.abs() > 0.5 {
                if diff > 0.0 {
                    eprintln!(
                        "📊 This build was {:.1}s ({:.1}%) slower than average ({:.2}s)",
                        diff, pct, avg
                    );
                } else {
                    eprintln!(
                        "📊 This build was {:.1}s ({:.1}%) faster than average ({:.2}s)",
                        diff.abs(),
                        pct.abs(),
                        avg
                    );
                }
            }
        }
    }
}
