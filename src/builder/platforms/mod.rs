//! Platform-specific build implementations
//!
//! Each platform has its own builder that implements the `PlatformBuilder` trait.
//! The builders handle:
//! - CMake configuration with platform-specific toolchains
//! - Multi-architecture builds
//! - Library packaging (frameworks, AARs, HARs, etc.)
//! - Archive creation
//!
//! ## Platform Grouping Strategy
//!
//! When building multiple platforms in parallel, we group platforms to avoid
//! toolchain conflicts:
//!
//! - **Apple Group** (iOS, macOS, tvOS, watchOS): Built sequentially to avoid Xcode conflicts
//! - **Gradle Group** (Android, KMP): Built sequentially to avoid Gradle lock/daemon conflicts
//! - **Other Platforms** (Linux, Windows, OHOS): Built in parallel

pub mod android;
mod apple_symbols;
pub mod benches;
pub mod conan;
mod elf_symbols;
pub mod ios;
pub mod kmp;
pub mod linux;
pub mod macos;
pub mod ohos;
pub mod openwrt;
pub mod tests;
pub mod tvos;
pub mod watchos;
pub mod windows;

use std::collections::HashSet;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use anyhow::{bail, Result};

use super::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::BuildTarget;

/// Get the appropriate platform builder for the target
pub fn get_builder(target: &BuildTarget) -> Result<Box<dyn PlatformBuilder>> {
    match target {
        BuildTarget::Linux => Ok(Box::new(linux::LinuxBuilder::new())),
        BuildTarget::Macos => Ok(Box::new(macos::MacosBuilder::new())),
        BuildTarget::Windows => Ok(Box::new(windows::WindowsBuilder::new())),
        BuildTarget::Ios => Ok(Box::new(ios::IosBuilder::new())),
        BuildTarget::Android => Ok(Box::new(android::AndroidBuilder::new())),
        BuildTarget::Ohos => Ok(Box::new(ohos::OhosBuilder::new())),
        BuildTarget::Tvos => Ok(Box::new(tvos::TvosBuilder::new())),
        BuildTarget::Watchos => Ok(Box::new(watchos::WatchosBuilder::new())),
        BuildTarget::Kmp => Ok(Box::new(kmp::KmpBuilder::new())),
        BuildTarget::Conan => Ok(Box::new(conan::ConanBuilder::new())),
        BuildTarget::Openwrt => Ok(Box::new(openwrt::OpenwrtBuilder::new())),
        BuildTarget::All => bail!("Use build_all() for building all platforms"),
        BuildTarget::Apple => bail!("Use build_apple() for building all Apple platforms"),
    }
}

/// Result of a subprocess platform build
#[derive(Debug)]
struct SubprocessBuildResult {
    platform: String,
    success: bool,
    duration_secs: f64,
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
}

/// Format duration in human-readable format (e.g., "2m 34s" or "45s")
fn format_duration(secs: f64) -> String {
    let total_secs = secs as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Check if a platform can be built natively on the current host
fn can_build_natively(target: &BuildTarget) -> bool {
    let host_os = std::env::consts::OS;

    match target {
        // Linux can only be built natively on Linux
        BuildTarget::Linux => host_os == "linux",

        // Windows can only be built natively on Windows
        BuildTarget::Windows => host_os == "windows",

        // Apple platforms can only be built on macOS (Xcode required)
        BuildTarget::Macos | BuildTarget::Ios | BuildTarget::Tvos | BuildTarget::Watchos => {
            host_os == "macos"
        }

        // OpenWrt requires musl cross-compilers — only native on Linux with toolchains installed
        BuildTarget::Openwrt => host_os == "linux",

        // Android, OHOS, Kmp, Conan can be built on any platform
        _ => true,
    }
}

/// Build a single platform using subprocess to isolate output
/// Returns (platform, success, duration, stdout, stderr)
fn build_platform_subprocess(
    target: &BuildTarget,
    project_root: &PathBuf,
    args: &BuildArgs,
) -> SubprocessBuildResult {
    let start = Instant::now();
    let platform_name = target.to_string().to_lowercase();

    // Get the ccgo executable path
    let ccgo_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ccgo"));

    let cmd_args = subprocess_command_args(&platform_name, args);

    // Run the subprocess
    let result = Command::new(&ccgo_exe)
        .args(&cmd_args)
        .current_dir(project_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    let duration_secs = start.elapsed().as_secs_f64();

    match result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            SubprocessBuildResult {
                platform: platform_name,
                success: output.status.success(),
                duration_secs,
                stdout,
                stderr,
                exit_code: output.status.code(),
            }
        }
        Err(e) => SubprocessBuildResult {
            platform: platform_name,
            success: false,
            duration_secs,
            stdout: String::new(),
            stderr: format!("Failed to execute subprocess: {}", e),
            exit_code: None,
        },
    }
}

/// Arguments passed to subprocess builds
#[derive(Clone, Default)]
struct BuildArgs {
    arch: Option<String>,
    native_only: bool,
    docker: bool,
    auto_docker: bool,
    toolchain: Option<String>,
    link_type: Option<String>,
    stl: Option<String>,
    release: bool,
    dev: bool,
    profile: Option<String>,
    asan: bool,
    tsan: bool,
    features: Vec<String>,
    use_default_features: bool,
    all_features: bool,
    variants: Vec<String>,
    jobs: Option<usize>,
    ide_project: bool,
    no_fetch: bool,
    verbose: bool,
    cache: Option<String>,
    linkage_default: Option<String>,
    linkage_overrides: Vec<(String, String)>,
    linkage_on_shared_default: Option<String>,
    linkage_on_shared_overrides: Vec<(String, String)>,
    linkage_on_static_default: Option<String>,
    linkage_on_static_overrides: Vec<(String, String)>,
}

impl BuildArgs {
    fn from_context(ctx: &BuildContext) -> Self {
        use crate::builder::sanitizer::SanitizerKind;

        let (asan, tsan) = match ctx.options.sanitizer {
            Some(SanitizerKind::Address) => (true, false),
            Some(SanitizerKind::Thread) => (false, true),
            None => (false, false),
        };

        Self {
            arch: if ctx.options.architectures.is_empty() {
                None
            } else {
                Some(ctx.options.architectures.join(","))
            },
            native_only: ctx.options.native_only,
            docker: ctx.options.use_docker,
            auto_docker: ctx.options.auto_docker,
            toolchain: Some(ctx.options.toolchain.to_string()),
            link_type: Some(ctx.options.link_type.to_string()),
            stl: ctx.options.stl.clone(),
            release: ctx.options.release,
            dev: ctx.options.dev,
            profile: ctx.options.profile_name.clone(),
            asan,
            tsan,
            features: ctx.options.features.clone(),
            use_default_features: ctx.options.use_default_features,
            all_features: ctx.options.all_features,
            variants: ctx.options.variants.clone(),
            jobs: ctx.options.jobs,
            ide_project: ctx.options.ide_project,
            no_fetch: ctx.options.no_fetch,
            verbose: ctx.options.verbose,
            cache: ctx.options.cache.clone().filter(|cache| cache != "auto"),
            linkage_default: ctx.options.linkage_default.map(|value| value.to_string()),
            linkage_overrides: sorted_overrides(&ctx.options.linkage_overrides),
            linkage_on_shared_default: ctx
                .options
                .linkage_on_shared_default
                .map(|value| value.to_string()),
            linkage_on_shared_overrides: sorted_overrides(&ctx.options.linkage_on_shared_overrides),
            linkage_on_static_default: ctx
                .options
                .linkage_on_static_default
                .map(|value| value.to_string()),
            linkage_on_static_overrides: sorted_overrides(&ctx.options.linkage_on_static_overrides),
        }
    }
}

fn sorted_overrides(
    map: &std::collections::HashMap<String, crate::config::Linkage>,
) -> Vec<(String, String)> {
    let mut overrides: Vec<_> = map
        .iter()
        .map(|(name, value)| (name.clone(), value.to_string()))
        .collect();
    overrides.sort_by(|left, right| left.0.cmp(&right.0));
    overrides
}

/// Build the child `ccgo build` argument list for one platform.
fn subprocess_command_args(platform: &str, args: &BuildArgs) -> Vec<String> {
    let mut cmd = vec!["build".to_string(), platform.to_string()];
    push_target_args(&mut cmd, args);
    push_mode_args(&mut cmd, args);
    push_feature_args(&mut cmd, args);
    push_linkage_args(&mut cmd, args);
    cmd
}

fn push_target_args(cmd: &mut Vec<String>, args: &BuildArgs) {
    if let Some(arch) = &args.arch {
        cmd.push("--arch".to_string());
        cmd.push(arch.clone());
    }
    if args.native_only {
        cmd.push("--native-only".to_string());
    }
    if args.docker {
        cmd.push("--docker".to_string());
    }
    if args.auto_docker {
        cmd.push("--auto-docker".to_string());
    }
    if let Some(toolchain) = &args.toolchain {
        cmd.push("--toolchain".to_string());
        cmd.push(toolchain.clone());
    }
    if let Some(link_type) = &args.link_type {
        cmd.push("--build-as".to_string());
        cmd.push(link_type.clone());
    }
    if let Some(stl) = &args.stl {
        cmd.push("--stl".to_string());
        cmd.push(stl.clone());
    }
    if args.ide_project {
        cmd.push("--ide-project".to_string());
    }
    if let Some(cache) = &args.cache {
        cmd.push("--cache".to_string());
        cmd.push(cache.clone());
    }
}

fn push_mode_args(cmd: &mut Vec<String>, args: &BuildArgs) {
    if args.release {
        cmd.push("--release".to_string());
    }
    if args.dev {
        cmd.push("--dev".to_string());
    }
    if let Some(profile) = &args.profile {
        cmd.push("--profile".to_string());
        cmd.push(profile.clone());
    }
    if args.asan {
        cmd.push("--asan".to_string());
    }
    if args.tsan {
        cmd.push("--tsan".to_string());
    }
    if let Some(jobs) = args.jobs {
        cmd.push("--jobs".to_string());
        cmd.push(jobs.to_string());
    }
    if args.no_fetch {
        cmd.push("--no-fetch".to_string());
    }
    if args.verbose {
        cmd.push("--verbose".to_string());
    }
}

fn push_feature_args(cmd: &mut Vec<String>, args: &BuildArgs) {
    if !args.features.is_empty() {
        cmd.push("--features".to_string());
        cmd.push(args.features.join(","));
    }
    if !args.use_default_features {
        cmd.push("--no-default-features".to_string());
    }
    if args.all_features {
        cmd.push("--all-features".to_string());
    }
    if !args.variants.is_empty() {
        cmd.push("--variant".to_string());
        cmd.push(args.variants.join(","));
    }
}

fn push_linkage_args(cmd: &mut Vec<String>, args: &BuildArgs) {
    push_linkage_values(
        cmd,
        "--linkage",
        &args.linkage_default,
        &args.linkage_overrides,
    );
    push_linkage_values(
        cmd,
        "--linkage-on-shared",
        &args.linkage_on_shared_default,
        &args.linkage_on_shared_overrides,
    );
    push_linkage_values(
        cmd,
        "--linkage-on-static",
        &args.linkage_on_static_default,
        &args.linkage_on_static_overrides,
    );
}

fn push_linkage_values(
    cmd: &mut Vec<String>,
    flag: &str,
    default: &Option<String>,
    overrides: &[(String, String)],
) {
    if let Some(value) = default {
        cmd.push(flag.to_string());
        cmd.push(value.clone());
    }
    for (name, value) in overrides {
        cmd.push(flag.to_string());
        cmd.push(format!("{name}={value}"));
    }
}

/// Print build error summary (last few lines of stderr/stdout)
fn print_build_error(stdout: &str, stderr: &str) {
    // Print last few lines of stderr first (usually contains the actual error)
    if !stderr.is_empty() {
        let stderr_lines: Vec<&str> = stderr.lines().collect();
        let start = stderr_lines.len().saturating_sub(10);
        eprintln!(
            "   Error output (last {} lines):",
            stderr_lines.len().min(10)
        );
        for line in &stderr_lines[start..] {
            eprintln!("   | {}", line);
        }
    } else if !stdout.is_empty() {
        // Fall back to stdout if stderr is empty
        let stdout_lines: Vec<&str> = stdout.lines().collect();
        let start = stdout_lines.len().saturating_sub(5);
        eprintln!("   Output (last {} lines):", stdout_lines.len().min(5));
        for line in &stdout_lines[start..] {
            eprintln!("   | {}", line);
        }
    }
}

/// Platforms included in `ccgo build all`.
fn all_build_platforms() -> Vec<BuildTarget> {
    vec![
        BuildTarget::Linux,
        BuildTarget::Windows,
        BuildTarget::Ohos,
        BuildTarget::Ios,
        BuildTarget::Macos,
        BuildTarget::Tvos,
        BuildTarget::Watchos,
        BuildTarget::Android,
    ]
}

/// Whether the project has a usable KMP module.
fn kmp_configured(project_root: &std::path::Path) -> bool {
    let kmp_dir = project_root.join("kmp");
    kmp_dir.join("gradlew").exists() || kmp_dir.join("gradlew.bat").exists()
}

/// Platforms for `ccgo build all`, including KMP when the project has one.
fn all_platforms_for(ctx: &BuildContext) -> Vec<BuildTarget> {
    let mut platforms = all_build_platforms();
    if kmp_configured(&ctx.project_root) {
        platforms.push(BuildTarget::Kmp);
    }
    platforms
}

/// Platforms built in the parallel phase of `ccgo build all`.
///
/// KMP is excluded because it links against the native libraries produced by
/// the other platforms and therefore runs afterwards.
fn parallel_build_platforms(platforms: &[BuildTarget]) -> Vec<BuildTarget> {
    platforms
        .iter()
        .filter(|target| **target != BuildTarget::Kmp)
        .cloned()
        .collect()
}

/// One `ccgo build all` invocation: which platforms run where.
struct BuildPlan {
    all: Vec<BuildTarget>,
    apple: Vec<BuildTarget>,
    gradle: Vec<BuildTarget>,
    other: Vec<BuildTarget>,
    deferred_kmp: bool,
}

impl BuildPlan {
    fn workers(&self) -> usize {
        usize::from(!self.apple.is_empty())
            + usize::from(!self.gradle.is_empty())
            + self.other.len()
    }
}

fn platform_list(platforms: &[BuildTarget]) -> String {
    platforms
        .iter()
        .map(|platform| platform.to_string().to_lowercase())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Split `all` into the Apple, Gradle and independent worker groups.
fn plan_builds(ctx: &BuildContext) -> BuildPlan {
    let all = all_platforms_for(ctx);
    let deferred_kmp = all.contains(&BuildTarget::Kmp);
    let parallel = parallel_build_platforms(&all);

    let apple_platforms: HashSet<_> = [
        BuildTarget::Ios,
        BuildTarget::Macos,
        BuildTarget::Tvos,
        BuildTarget::Watchos,
    ]
    .into_iter()
    .collect();
    let gradle_platforms: HashSet<_> = [BuildTarget::Android, BuildTarget::Kmp]
        .into_iter()
        .collect();

    let apple = parallel
        .iter()
        .filter(|target| apple_platforms.contains(target))
        .cloned()
        .collect();
    let gradle = parallel
        .iter()
        .filter(|target| gradle_platforms.contains(target))
        .cloned()
        .collect();
    let other = parallel
        .iter()
        .filter(|target| !apple_platforms.contains(target) && !gradle_platforms.contains(target))
        .cloned()
        .collect();

    BuildPlan {
        all,
        apple,
        gradle,
        other,
        deferred_kmp,
    }
}

fn print_build_plan(ctx: &BuildContext, plan: &BuildPlan, num_jobs: usize) {
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("Building library for ALL platforms");
    eprintln!(
        "Build Mode: {}",
        if ctx.options.release {
            "RELEASE"
        } else {
            "DEBUG/BETA"
        }
    );
    eprintln!("{}", "=".repeat(80));

    eprintln!(
        "\nWill build {} platforms: {}",
        plan.all.len(),
        platform_list(&plan.all)
    );
    eprintln!("Parallel platform workers: {}", plan.workers());
    eprintln!("Jobs per platform build: {}", num_jobs);
    eprintln!("{}", "=".repeat(80));

    eprintln!(
        "\n🚀 Starting parallel build with {} platform workers ({} jobs each)...",
        plan.workers(),
        num_jobs
    );
    if !plan.apple.is_empty() {
        eprintln!(
            "   ℹ️  Apple platforms ({}) will be built sequentially to avoid Xcode conflicts",
            platform_list(&plan.apple)
        );
    }
    if !plan.gradle.is_empty() {
        eprintln!(
            "   ℹ️  Gradle platforms ({}) will be built sequentially to avoid Gradle lock conflicts",
            platform_list(&plan.gradle)
        );
    }
    if plan.deferred_kmp {
        eprintln!(
            "   ℹ️  KMP will build after all native platforms so it can reuse their libraries"
        );
    }
    eprintln!();
}

/// Build one platform subprocess and report progress.
fn build_and_report(
    target: BuildTarget,
    args: BuildArgs,
    root: PathBuf,
    completed: Arc<AtomicUsize>,
    in_prog: Arc<Mutex<HashSet<String>>>,
    results_ref: Arc<Mutex<Vec<SubprocessBuildResult>>>,
    total: usize,
) {
    let result = build_platform_subprocess(&target, &root, &args);

    // Update progress atomically
    let count = completed.fetch_add(1, Ordering::SeqCst) + 1;
    {
        let mut progress = in_prog.lock().unwrap();
        progress.remove(&result.platform);

        let status = if result.success { "✅" } else { "❌" };
        let status_text = if result.success {
            "completed"
        } else {
            "failed"
        };

        eprintln!(
            "{} [{}/{}] {} {} ({})",
            status,
            count,
            total,
            result.platform.to_uppercase(),
            status_text,
            format_duration(result.duration_secs)
        );

        if !result.success {
            print_build_error(&result.stdout, &result.stderr);
        }

        if !progress.is_empty() {
            let mut remaining: Vec<_> = progress.iter().cloned().collect();
            remaining.sort();
            eprintln!("   ⏳ Still building: {}", remaining.join(", "));
        }
    }

    results_ref.lock().unwrap().push(result);
}

/// Platform name and elapsed seconds.
type BuildTimes = Vec<(String, f64)>;

fn split_results(results: &[SubprocessBuildResult]) -> (BuildTimes, BuildTimes) {
    let mut successful = Vec::new();
    let mut failed = Vec::new();
    for result in results {
        let entry = (result.platform.clone(), result.duration_secs);
        if result.success {
            successful.push(entry);
        } else {
            failed.push(entry);
        }
    }
    (successful, failed)
}

fn print_build_summary(
    plan: &BuildPlan,
    successful: &[(String, f64)],
    failed: &[(String, f64)],
    total_duration: f64,
    num_jobs: usize,
) {
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("BUILD ALL SUMMARY");
    eprintln!("{}", "=".repeat(80));
    eprintln!("\nTotal platforms: {}", plan.all.len());
    eprintln!("Successful: {}", successful.len());
    eprintln!("Failed: {}", failed.len());
    eprintln!("Parallel platform workers: {}", plan.workers());
    eprintln!("Jobs per platform build: {}", num_jobs);

    if !successful.is_empty() {
        eprintln!("\n✅ Successful builds:");
        for (platform, duration) in successful {
            eprintln!("   - {} ({})", platform, format_duration(*duration));
        }
    }

    if !failed.is_empty() {
        eprintln!("\n❌ Failed builds:");
        for (platform, duration) in failed {
            eprintln!("   - {} ({})", platform, format_duration(*duration));
        }
    }

    eprintln!(
        "\n⏱️  Build completed in {}",
        format_duration(total_duration)
    );

    if failed.is_empty() {
        eprintln!("\n🎉 All platforms built successfully!");
        eprintln!("   💡 To collect all artifacts, run: ccgo package");
    }
}

/// Build all platforms with smart grouping strategy using subprocess isolation
///
/// Platform grouping to avoid toolchain conflicts:
/// - Apple platforms (ios, macos, tvos, watchos): Built sequentially to avoid Xcode conflicts
/// - Gradle platform (android): Runs alongside other groups
/// - Other platforms (linux, windows, ohos): Built in parallel
/// - KMP: Built last, after every native platform, so it can reuse their libraries
///
/// Each platform is built in a subprocess to isolate output and prevent interleaving.
pub fn build_all(ctx: &BuildContext) -> Result<Vec<BuildResult>> {
    let plan = plan_builds(ctx);
    let num_jobs = ctx.jobs();
    print_build_plan(ctx, &plan, num_jobs);

    // Shared state for progress tracking
    let completed_count = Arc::new(AtomicUsize::new(0));
    let in_progress = Arc::new(Mutex::new(
        plan.all
            .iter()
            .map(|platform| platform.to_string().to_lowercase())
            .collect::<HashSet<_>>(),
    ));
    let results = Arc::new(Mutex::new(Vec::<SubprocessBuildResult>::new()));

    let build_args = BuildArgs::from_context(ctx);
    let project_root = ctx.project_root.clone();
    let total = plan.all.len();
    let mut handles = Vec::new();

    // Thread for Apple platforms (sequential within thread)
    if !plan.apple.is_empty() {
        let args = build_args.clone();
        let root = project_root.clone();
        let completed = Arc::clone(&completed_count);
        let in_prog = Arc::clone(&in_progress);
        let results_ref = Arc::clone(&results);
        let platforms = plan.apple.clone();

        handles.push(thread::spawn(move || {
            for target in platforms {
                build_and_report(
                    target,
                    args.clone(),
                    root.clone(),
                    Arc::clone(&completed),
                    Arc::clone(&in_prog),
                    Arc::clone(&results_ref),
                    total,
                );
            }
        }));
    }

    // Thread for Gradle platforms (sequential within thread)
    if !plan.gradle.is_empty() {
        let args = build_args.clone();
        let root = project_root.clone();
        let completed = Arc::clone(&completed_count);
        let in_prog = Arc::clone(&in_progress);
        let results_ref = Arc::clone(&results);
        let platforms = plan.gradle.clone();

        handles.push(thread::spawn(move || {
            for target in platforms {
                build_and_report(
                    target,
                    args.clone(),
                    root.clone(),
                    Arc::clone(&completed),
                    Arc::clone(&in_prog),
                    Arc::clone(&results_ref),
                    total,
                );
            }
        }));
    }

    // Individual threads for other platforms (parallel)
    for target in plan.other.clone() {
        let args = build_args.clone();
        let root = project_root.clone();
        let completed = Arc::clone(&completed_count);
        let in_prog = Arc::clone(&in_progress);
        let results_ref = Arc::clone(&results);

        handles.push(thread::spawn(move || {
            build_and_report(target, args, root, completed, in_prog, results_ref, total);
        }));
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // KMP needs the native libraries of the other platforms (including those
    // built through Docker), so it runs after every parallel worker.
    if plan.deferred_kmp {
        eprintln!("\n⏳ Building KMP after native platform builds...");
        build_and_report(
            BuildTarget::Kmp,
            build_args.clone(),
            project_root.clone(),
            Arc::clone(&completed_count),
            Arc::clone(&in_progress),
            Arc::clone(&results),
            total,
        );
    }

    let all_results = Arc::try_unwrap(results)
        .expect("All threads completed")
        .into_inner()
        .unwrap();
    let (successful, failed) = split_results(&all_results);
    let total_duration = all_results
        .iter()
        .map(|result| result.duration_secs)
        .max_by(|left, right| left.partial_cmp(right).unwrap())
        .unwrap_or(0.0);

    print_build_summary(&plan, &successful, &failed, total_duration, num_jobs);

    if !failed.is_empty() {
        bail!("{} platform(s) failed to build", failed.len());
    }

    // Subprocess builds write their own archives; the caller reads target/.
    Ok(Vec::new())
}

/// Build all Apple platforms (iOS, macOS, tvOS, watchOS) sequentially
///
/// Apple platforms share Xcode toolchain and must be built sequentially
/// to avoid conflicts. Uses subprocess to isolate output.
pub fn build_apple(ctx: &BuildContext) -> Result<Vec<BuildResult>> {
    let platforms = [
        BuildTarget::Ios,
        BuildTarget::Macos,
        BuildTarget::Tvos,
        BuildTarget::Watchos,
    ];

    let total_platforms = platforms.len();

    eprintln!("\n{}", "=".repeat(80));
    eprintln!("Building library for APPLE platforms (iOS, macOS, tvOS, watchOS)");
    eprintln!(
        "Build Mode: {}",
        if ctx.options.release {
            "RELEASE"
        } else {
            "DEBUG/BETA"
        }
    );
    eprintln!("{}", "=".repeat(80));

    eprintln!(
        "\nWill build {} Apple platforms: {}",
        total_platforms,
        platforms
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!("Build mode: sequential (recommended for Apple platforms)");
    eprintln!("{}", "=".repeat(80));

    let build_args = BuildArgs::from_context(ctx);
    let mut successful: Vec<(String, f64)> = Vec::new();
    let mut failed: Vec<(String, f64)> = Vec::new();

    for (index, target) in platforms.iter().enumerate() {
        eprintln!(
            "\n[{}/{}] Building {}...",
            index + 1,
            total_platforms,
            target.to_string().to_uppercase()
        );

        let result = build_platform_subprocess(target, &ctx.project_root, &build_args);

        if result.success {
            eprintln!(
                "✅ [{}/{}] {} completed ({})",
                index + 1,
                total_platforms,
                result.platform.to_uppercase(),
                format_duration(result.duration_secs)
            );
            successful.push((result.platform, result.duration_secs));
        } else {
            eprintln!(
                "❌ [{}/{}] {} failed ({})",
                index + 1,
                total_platforms,
                result.platform.to_uppercase(),
                format_duration(result.duration_secs)
            );
            print_build_error(&result.stdout, &result.stderr);
            failed.push((result.platform, result.duration_secs));
        }
    }

    // Calculate total duration
    let total_duration: f64 = successful
        .iter()
        .chain(failed.iter())
        .map(|(_, d)| *d)
        .sum();

    // Print summary
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("BUILD APPLE SUMMARY");
    eprintln!("{}", "=".repeat(80));
    eprintln!("\nTotal platforms: {}", total_platforms);
    eprintln!("Successful: {}", successful.len());
    eprintln!("Failed: {}", failed.len());

    if !successful.is_empty() {
        eprintln!("\n✅ Successful builds:");
        for (platform, duration) in &successful {
            eprintln!("   - {} ({})", platform, format_duration(*duration));
        }
    }

    if !failed.is_empty() {
        eprintln!("\n❌ Failed builds:");
        for (platform, duration) in &failed {
            eprintln!("   - {} ({})", platform, format_duration(*duration));
        }
    }

    eprintln!(
        "\n⏱️  Build completed in {}",
        format_duration(total_duration)
    );

    if !failed.is_empty() {
        bail!("{} Apple platform(s) failed to build", failed.len());
    }

    eprintln!("\n🎉 All Apple platforms built successfully!");
    eprintln!("   💡 To publish, run: ccgo publish apple --cocoapods / --spm");

    // Return empty vec - subprocess builds handle their own output
    Ok(Vec::new())
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    fn has_pair(cmd: &[String], flag: &str, value: &str) -> bool {
        cmd.windows(2)
            .any(|pair| pair[0] == flag && pair[1] == value)
    }

    #[test]
    fn all_build_subprocess_forwards_release_and_auto_docker() {
        let args = BuildArgs {
            release: true,
            auto_docker: true,
            ..BuildArgs::default()
        };

        for platform in [
            "linux", "windows", "ohos", "ios", "macos", "tvos", "watchos", "android", "kmp",
        ] {
            let cmd = subprocess_command_args(platform, &args);
            assert_eq!(cmd[0], "build");
            assert_eq!(cmd[1], platform);
            assert!(cmd.iter().any(|arg| arg == "--release"));
            assert!(cmd.iter().any(|arg| arg == "--auto-docker"));
        }
    }

    #[test]
    fn subprocess_forwards_profile_features_and_linkage_options() {
        let args = BuildArgs {
            profile: Some("ci".to_string()),
            asan: true,
            features: vec!["networking".to_string()],
            variants: vec!["oversea".to_string()],
            all_features: true,
            jobs: Some(4),
            no_fetch: true,
            cache: Some("sccache".to_string()),
            linkage_default: Some("shared-external".to_string()),
            linkage_overrides: vec![
                ("alpha".to_string(), "static-embedded".to_string()),
                ("zeta".to_string(), "shared-external".to_string()),
            ],
            ..BuildArgs::default()
        };

        let cmd = subprocess_command_args("linux", &args);

        assert!(has_pair(&cmd, "--profile", "ci"));
        assert!(has_pair(&cmd, "--features", "networking"));
        assert!(has_pair(&cmd, "--variant", "oversea"));
        assert!(has_pair(&cmd, "--jobs", "4"));
        assert!(has_pair(&cmd, "--cache", "sccache"));
        assert!(has_pair(&cmd, "--linkage", "shared-external"));
        assert!(has_pair(&cmd, "--linkage", "alpha=static-embedded"));
        assert!(has_pair(&cmd, "--linkage", "zeta=shared-external"));
        assert!(cmd.iter().any(|arg| arg == "--asan"));
        assert!(cmd.iter().any(|arg| arg == "--all-features"));
        assert!(cmd.iter().any(|arg| arg == "--no-fetch"));
    }

    fn build_ctx(root: &std::path::Path) -> BuildContext {
        let config = toml::from_str("[package]\nname = \"demo\"\nversion = \"1.0.0\"\n")
            .expect("toml should parse");
        BuildContext::new(
            root.to_path_buf(),
            config,
            crate::builder::BuildOptions::default(),
        )
    }

    #[test]
    fn all_build_includes_kmp_only_when_configured() {
        // A plain C++ project must not fail because `all` suddenly requires KMP.
        let plain = tempfile::tempdir().unwrap();
        let plain_platforms = all_platforms_for(&build_ctx(plain.path()));
        assert_eq!(plain_platforms.len(), 8);
        assert!(!plain_platforms.contains(&BuildTarget::Kmp));

        let kmp_project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(kmp_project.path().join("kmp")).unwrap();
        std::fs::write(kmp_project.path().join("kmp/gradlew"), b"").unwrap();

        let all = all_platforms_for(&build_ctx(kmp_project.path()));
        assert_eq!(all.len(), 9);
        assert!(all.contains(&BuildTarget::Kmp));

        let unique: HashSet<_> = all.iter().collect();
        assert_eq!(unique.len(), all.len(), "duplicate platform in `all`");

        // KMP is deferred until the native platforms finish.
        let parallel = parallel_build_platforms(&all);
        assert_eq!(parallel.len(), 8);
        assert!(!parallel.contains(&BuildTarget::Kmp));
    }
}
