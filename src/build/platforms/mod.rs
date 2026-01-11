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
//! - **Other Platforms** (Linux, Windows, OHOS, Conan): Built in parallel

pub mod android;
pub mod benches;
pub mod conan;
pub mod ios;
pub mod kmp;
pub mod linux;
pub mod macos;
pub mod ohos;
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

    // Build command arguments
    let mut cmd_args = vec!["build".to_string(), platform_name.clone()];

    // Pass through relevant arguments
    if let Some(arch) = &args.arch {
        cmd_args.push("--arch".to_string());
        cmd_args.push(arch.clone());
    }
    if args.native_only {
        cmd_args.push("--native-only".to_string());
    }
    if args.docker {
        cmd_args.push("--docker".to_string());
    }
    if args.auto_docker {
        cmd_args.push("--auto-docker".to_string());
    }
    if let Some(toolchain) = &args.toolchain {
        cmd_args.push("--toolchain".to_string());
        cmd_args.push(toolchain.clone());
    }
    if let Some(link_type) = &args.link_type {
        cmd_args.push("--link-type".to_string());
        cmd_args.push(link_type.clone());
    }
    if args.release {
        cmd_args.push("--release".to_string());
    }
    if args.dev {
        cmd_args.push("--dev".to_string());
    }

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
#[derive(Clone)]
struct BuildArgs {
    arch: Option<String>,
    native_only: bool,
    docker: bool,
    auto_docker: bool,
    toolchain: Option<String>,
    link_type: Option<String>,
    release: bool,
    dev: bool,
}

impl BuildArgs {
    fn from_context(ctx: &BuildContext) -> Self {
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
            release: ctx.options.release,
            dev: ctx.options.dev,
        }
    }
}

/// Print build error summary (last few lines of stderr/stdout)
fn print_build_error(stdout: &str, stderr: &str) {
    // Print last few lines of stderr first (usually contains the actual error)
    if !stderr.is_empty() {
        let stderr_lines: Vec<&str> = stderr.lines().collect();
        let start = stderr_lines.len().saturating_sub(10);
        eprintln!("   Error output (last {} lines):", stderr_lines.len().min(10));
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

/// Build all platforms with smart grouping strategy using subprocess isolation
///
/// Platform grouping to avoid toolchain conflicts:
/// - Apple platforms (ios, macos, tvos, watchos): Built sequentially to avoid Xcode conflicts
/// - Gradle platforms (android, kmp): Built sequentially to avoid Gradle lock conflicts
/// - Other platforms (linux, windows, ohos, conan): Built in parallel
///
/// Each platform is built in a subprocess to isolate output and prevent interleaving.
pub fn build_all(ctx: &BuildContext) -> Result<Vec<BuildResult>> {
    let all_platforms = vec![
        BuildTarget::Linux,
        BuildTarget::Windows,
        BuildTarget::Ohos,
        BuildTarget::Ios,
        BuildTarget::Macos,
        BuildTarget::Tvos,
        BuildTarget::Watchos,
        BuildTarget::Android,
    ];

    // Define platform groups
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

    // Categorize platforms
    let apple_to_build: Vec<_> = all_platforms
        .iter()
        .filter(|p| apple_platforms.contains(p))
        .cloned()
        .collect();
    let gradle_to_build: Vec<_> = all_platforms
        .iter()
        .filter(|p| gradle_platforms.contains(p))
        .cloned()
        .collect();
    let other_to_build: Vec<_> = all_platforms
        .iter()
        .filter(|p| !apple_platforms.contains(p) && !gradle_platforms.contains(p))
        .cloned()
        .collect();

    let total_platforms = all_platforms.len();
    let num_jobs = ctx.jobs();

    // Print build plan
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
        total_platforms,
        all_platforms
            .iter()
            .map(|p| p.to_string().to_lowercase())
            .collect::<Vec<_>>()
            .join(", ")
    );
    eprintln!("Parallel workers: {}", num_jobs);
    eprintln!("{}", "=".repeat(80));

    eprintln!("\n🚀 Starting parallel build with {} workers...", num_jobs);
    if !apple_to_build.is_empty() {
        eprintln!(
            "   ℹ️  Apple platforms ({}) will be built sequentially to avoid Xcode conflicts",
            apple_to_build
                .iter()
                .map(|p| p.to_string().to_lowercase())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !gradle_to_build.is_empty() {
        eprintln!(
            "   ℹ️  Gradle platforms ({}) will be built sequentially to avoid Gradle lock conflicts",
            gradle_to_build
                .iter()
                .map(|p| p.to_string().to_lowercase())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    eprintln!();

    // Shared state for progress tracking
    let completed_count = Arc::new(AtomicUsize::new(0));
    let in_progress = Arc::new(Mutex::new(
        all_platforms
            .iter()
            .map(|p| p.to_string().to_lowercase())
            .collect::<HashSet<_>>(),
    ));
    let results = Arc::new(Mutex::new(Vec::<SubprocessBuildResult>::new()));

    // Build arguments from context
    let build_args = BuildArgs::from_context(ctx);
    let project_root = ctx.project_root.clone();

    // Function to build and report progress
    let build_and_report = |target: BuildTarget,
                            args: BuildArgs,
                            root: PathBuf,
                            completed: Arc<AtomicUsize>,
                            in_prog: Arc<Mutex<HashSet<String>>>,
                            results_ref: Arc<Mutex<Vec<SubprocessBuildResult>>>,
                            total: usize| {
        let result = build_platform_subprocess(&target, &root, &args);

        // Update progress atomically
        let count = completed.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut progress = in_prog.lock().unwrap();
            progress.remove(&result.platform);

            let status = if result.success { "✅" } else { "❌" };
            let platform_upper = result.platform.to_uppercase();
            let duration_str = format_duration(result.duration_secs);
            let status_text = if result.success { "completed" } else { "failed" };

            eprintln!(
                "{} [{}/{}] {} {} ({})",
                status, count, total, platform_upper, status_text, duration_str
            );

            // Print error details if failed
            if !result.success {
                print_build_error(&result.stdout, &result.stderr);
            }

            // Show remaining platforms
            if !progress.is_empty() {
                let mut remaining: Vec<_> = progress.iter().cloned().collect();
                remaining.sort();
                eprintln!("   ⏳ Still building: {}", remaining.join(", "));
            }
        }

        results_ref.lock().unwrap().push(result);
    };

    // Spawn threads for parallel execution
    let mut handles = Vec::new();

    // Thread for Apple platforms (sequential within thread)
    if !apple_to_build.is_empty() {
        let args = build_args.clone();
        let root = project_root.clone();
        let completed = Arc::clone(&completed_count);
        let in_prog = Arc::clone(&in_progress);
        let results_ref = Arc::clone(&results);
        let platforms = apple_to_build.clone();
        let total = total_platforms;

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
    if !gradle_to_build.is_empty() {
        let args = build_args.clone();
        let root = project_root.clone();
        let completed = Arc::clone(&completed_count);
        let in_prog = Arc::clone(&in_progress);
        let results_ref = Arc::clone(&results);
        let platforms = gradle_to_build.clone();
        let total = total_platforms;

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
    for target in other_to_build {
        let args = build_args.clone();
        let root = project_root.clone();
        let completed = Arc::clone(&completed_count);
        let in_prog = Arc::clone(&in_progress);
        let results_ref = Arc::clone(&results);
        let total = total_platforms;

        handles.push(thread::spawn(move || {
            build_and_report(
                target,
                args,
                root,
                completed,
                in_prog,
                results_ref,
                total,
            );
        }));
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Collect results
    let all_results = Arc::try_unwrap(results)
        .expect("All threads completed")
        .into_inner()
        .unwrap();

    let mut successful: Vec<(String, f64)> = Vec::new();
    let mut failed: Vec<(String, f64)> = Vec::new();

    for pr in &all_results {
        if pr.success {
            successful.push((pr.platform.clone(), pr.duration_secs));
        } else {
            failed.push((pr.platform.clone(), pr.duration_secs));
        }
    }

    // Calculate total duration
    let total_duration: f64 = all_results.iter().map(|r| r.duration_secs).max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(0.0);

    // Print summary
    eprintln!("\n{}", "=".repeat(80));
    eprintln!("BUILD ALL SUMMARY");
    eprintln!("{}", "=".repeat(80));
    eprintln!("\nTotal platforms: {}", total_platforms);
    eprintln!("Successful: {}", successful.len());
    eprintln!("Failed: {}", failed.len());
    eprintln!("Parallel workers: {}", num_jobs);

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

    eprintln!("\n⏱️  Build completed in {}", format_duration(total_duration));

    if failed.is_empty() {
        eprintln!("\n🎉 All platforms built successfully!");
        eprintln!("   💡 To collect all artifacts, run: ccgo package");
    }

    // For build_all, we return empty results since subprocess builds
    // write their own archives. The caller should look in target/ directory.
    if !failed.is_empty() {
        bail!("{} platform(s) failed to build", failed.len());
    }

    // Return empty vec - subprocess builds handle their own output
    Ok(Vec::new())
}

/// Build all Apple platforms (iOS, macOS, tvOS, watchOS) sequentially
///
/// Apple platforms share Xcode toolchain and must be built sequentially
/// to avoid conflicts. Uses subprocess to isolate output.
pub fn build_apple(ctx: &BuildContext) -> Result<Vec<BuildResult>> {
    let platforms = vec![
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
    let total_duration: f64 = successful.iter().chain(failed.iter()).map(|(_, d)| *d).sum();

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

    eprintln!("\n⏱️  Build completed in {}", format_duration(total_duration));

    if !failed.is_empty() {
        bail!(
            "{} Apple platform(s) failed to build",
            failed.len()
        );
    }

    eprintln!("\n🎉 All Apple platforms built successfully!");
    eprintln!("   💡 To publish, run: ccgo publish apple --cocoapods / --spm");

    // Return empty vec - subprocess builds handle their own output
    Ok(Vec::new())
}
