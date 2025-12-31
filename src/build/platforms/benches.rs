//! Benches platform builder
//!
//! Builds and runs Google Benchmark benchmarks on the host platform.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::{BuildContext, BuildResult, PlatformBuilder};

/// Benches platform builder
pub struct BenchesBuilder {}

impl BenchesBuilder {
    pub fn new() -> Self {
        Self {}
    }

    /// Get build output directory
    fn build_dir(&self, ctx: &BuildContext) -> PathBuf {
        // Uses cmake_build/{release|debug}/benches/ structure
        let release_subdir = if ctx.options.release { "release" } else { "debug" };
        ctx.project_root.join("cmake_build").join(release_subdir).join("benches")
    }

    /// Get install directory
    fn install_dir(&self, ctx: &BuildContext) -> PathBuf {
        // CMake installs to cmake_build/{release|debug}/benches/out/
        self.build_dir(ctx).join("out")
    }

    /// Get CMake generator based on platform
    fn cmake_generator(&self) -> &str {
        if cfg!(target_os = "windows") {
            "Visual Studio 16 2019"
        } else {
            // Use Unix Makefiles for all Unix-like systems (macOS, Linux, etc.)
            "Unix Makefiles"
        }
    }

    /// Get CMake extra flags for benchmarks
    fn cmake_extra_flags(&self, ctx: &BuildContext) -> Result<Vec<String>> {
        let mut flags = vec![
            "-DBENCHMARK_SUPPORT=ON".to_string(),
        ];

        // Add CCGO_CMAKE_DIR if available
        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            flags.push(format!("-DCCGO_CMAKE_DIR={}", cmake_dir.display()));
        }

        // Add build type for single-config generators (Unix Makefiles, Ninja, etc.)
        // Don't set for multi-config generators (Visual Studio)
        if !cfg!(target_os = "windows") {
            let build_type = if ctx.options.release {
                "Release"
            } else {
                "Debug"
            };
            flags.push(format!("-DCMAKE_BUILD_TYPE={}", build_type));
        }

        // Add macOS specific flags
        if cfg!(target_os = "macos") {
            flags.push("-DCMAKE_OSX_DEPLOYMENT_TARGET:STRING=10.9".to_string());
            flags.push("-DENABLE_BITCODE=0".to_string());
        }

        Ok(flags)
    }

    /// Configure and build benchmarks using CMake
    fn build_benchmarks(&self, ctx: &BuildContext, incremental: bool) -> Result<()> {
        let build_dir = self.build_dir(ctx);
        let install_dir = self.install_dir(ctx);

        // Clean build directory if not incremental
        if !incremental && build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)
                .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
        }

        // Create build directory
        std::fs::create_dir_all(&build_dir)
            .with_context(|| format!("Failed to create {}", build_dir.display()))?;

        if ctx.options.verbose {
            eprintln!("Building benchmarks in {}...", build_dir.display());
        }

        // Configure with CMake
        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd
            .arg("../..")
            .arg("-G")
            .arg(self.cmake_generator())
            .current_dir(&build_dir);

        // Add extra flags
        let extra_flags = self.cmake_extra_flags(ctx)?;
        for flag in &extra_flags {
            cmake_cmd.arg(flag);
        }

        if ctx.options.verbose {
            eprintln!("CMake configure: {:?}", cmake_cmd);
        }

        let status = cmake_cmd.status().context("Failed to run CMake configure")?;
        if !status.success() {
            bail!("CMake configure failed");
        }

        // Build
        let mut build_cmd = Command::new("cmake");
        build_cmd
            .arg("--build")
            .arg(".")
            .arg("--target")
            .arg("install")
            .current_dir(&build_dir);

        // Add config for multi-config generators (Windows)
        if cfg!(target_os = "windows") {
            let build_type = if ctx.options.release {
                "Release"
            } else {
                "Debug"
            };
            build_cmd.arg("--config").arg(build_type);
        }

        // Add parallel jobs
        if let Some(jobs) = ctx.options.jobs {
            build_cmd.arg("--parallel").arg(jobs.to_string());
        } else {
            build_cmd.arg("--parallel").arg("8");
        }

        if ctx.options.verbose {
            eprintln!("CMake build: {:?}", build_cmd);
        }

        let status = build_cmd.status().context("Failed to run CMake build")?;
        if !status.success() {
            bail!("CMake build failed");
        }

        if ctx.options.verbose {
            eprintln!("Benchmarks built successfully: {}", install_dir.display());
        }

        Ok(())
    }

    /// Find benchmark executables in install directory
    fn find_benchmark_executables(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        let install_dir = self.install_dir(ctx);
        let mut executables = Vec::new();

        if !install_dir.exists() {
            bail!("Benchmark install directory not found: {}", install_dir.display());
        }

        // Search for benchmark executables
        for entry in std::fs::read_dir(&install_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path.file_name().unwrap().to_string_lossy();
                // Look for _googlebenchmark or _bench suffix
                if file_name.contains("_googlebenchmark") || file_name.contains("_bench") {
                    #[cfg(unix)]
                    {
                        // Check if executable
                        use std::os::unix::fs::PermissionsExt;
                        let metadata = std::fs::metadata(&path)?;
                        if metadata.permissions().mode() & 0o111 != 0 {
                            executables.push(path);
                        }
                    }

                    #[cfg(windows)]
                    {
                        // On Windows, check for .exe extension
                        if file_name.ends_with(".exe") {
                            executables.push(path);
                        }
                    }
                }
            }
        }

        if executables.is_empty() {
            bail!("No benchmark executables found in {}", install_dir.display());
        }

        Ok(executables)
    }

    /// Run benchmark executables with optional filter
    pub fn run_benchmarks(&self, ctx: &BuildContext, filter: Option<&str>) -> Result<()> {
        let executables = self.find_benchmark_executables(ctx)?;
        let build_dir = self.build_dir(ctx);

        // Generate JSON output filename with timestamp
        let now = chrono::Local::now();
        let timestamp = now.format("%Y%m%d_%H%M%S_%6f");
        let system_name = if cfg!(target_os = "macos") {
            "macos"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        let json_output = build_dir.join(format!(
            "benches_on_{}_result_{}.json",
            system_name, timestamp
        ));

        for exe in executables {
            eprintln!("\nRunning benchmark: {}", exe.display());

            let mut cmd = Command::new(&exe);

            // Add benchmark filter if provided
            if let Some(filter_str) = filter {
                cmd.arg(format!("--benchmark_filter={}", filter_str));
            }

            // Add JSON output
            cmd.arg("--benchmark_format=console");
            cmd.arg(format!("--benchmark_out={}", json_output.display()));

            if ctx.options.verbose {
                eprintln!("Executing: {:?}", cmd);
            }

            let status = cmd.status().with_context(|| {
                format!("Failed to run benchmark executable {}", exe.display())
            })?;

            if !status.success() {
                bail!("Benchmark {} failed", exe.display());
            }

            eprintln!("✓ Benchmark completed: {}", exe.display());
        }

        eprintln!("\nBenchmark results: {}", json_output.display());
        Ok(())
    }

    /// Generate IDE project for benchmarks
    pub fn generate_ide_project(&self, ctx: &BuildContext) -> Result<()> {
        let build_dir = self.build_dir(ctx);

        // Clean build directory
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)
                .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
        }

        // Create build directory
        std::fs::create_dir_all(&build_dir)
            .with_context(|| format!("Failed to create {}", build_dir.display()))?;

        eprintln!("Generating IDE project in {}...", build_dir.display());

        // Configure with CMake
        let mut cmake_cmd = Command::new("cmake");
        cmake_cmd
            .arg("../..")
            .arg("-G")
            .arg(self.cmake_generator())
            .current_dir(&build_dir);

        // Add extra flags
        let extra_flags = self.cmake_extra_flags(ctx)?;
        for flag in &extra_flags {
            cmake_cmd.arg(flag);
        }

        if ctx.options.verbose {
            eprintln!("CMake configure: {:?}", cmake_cmd);
        }

        let status = cmake_cmd.status().context("Failed to run CMake configure")?;
        if !status.success() {
            bail!("CMake configure failed");
        }

        // Find and report project file
        let project_file = if cfg!(target_os = "macos") {
            build_dir.join(format!("{}.xcodeproj", ctx.lib_name()))
        } else if cfg!(target_os = "windows") {
            build_dir.join(format!("{}.sln", ctx.lib_name()))
        } else {
            build_dir.join(format!("{}.workspace", ctx.lib_name()))
        };

        if project_file.exists() {
            eprintln!("\n✓ IDE project generated: {}", project_file.display());

            // Try to open the project
            #[cfg(target_os = "macos")]
            {
                let _ = Command::new("open").arg(&project_file).status();
            }

            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("cmd")
                    .arg("/C")
                    .arg("start")
                    .arg(&project_file)
                    .status();
            }
        } else {
            eprintln!("\n✓ IDE project files generated in: {}", build_dir.display());
        }

        Ok(())
    }
}

impl PlatformBuilder for BenchesBuilder {
    fn platform_name(&self) -> &str {
        "benches"
    }

    fn default_architectures(&self) -> Vec<String> {
        vec![]
    }

    fn validate_prerequisites(&self, _ctx: &BuildContext) -> Result<()> {
        // Check if cmake is available
        if !which::which("cmake").is_ok() {
            bail!("CMake is required but not found in PATH");
        }
        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        self.validate_prerequisites(ctx)?;
        self.build_benchmarks(ctx, false)?;

        let duration = start.elapsed();

        // Return a placeholder result
        Ok(BuildResult {
            sdk_archive: self.install_dir(ctx),
            symbols_archive: None,
            aar_archive: None,
            duration_secs: duration.as_secs_f64(),
            architectures: vec![],
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean new directory structure: cmake_build/{release|debug}/benches
        for subdir in &["release", "debug"] {
            let build_dir = ctx.project_root.join("cmake_build").join(subdir).join("benches");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        // Clean old structure for backwards compatibility: cmake_build/Benches, cmake_build/benches
        for old_dir in &[
            ctx.project_root.join("cmake_build/Benches"),
            ctx.project_root.join("cmake_build/benches"),
        ] {
            if old_dir.exists() {
                std::fs::remove_dir_all(old_dir)
                    .with_context(|| format!("Failed to clean {}", old_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for BenchesBuilder {
    fn default() -> Self {
        Self::new()
    }
}
