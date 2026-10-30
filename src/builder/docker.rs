//! Docker-based cross-platform build support
//!
//! Enables building all platform libraries on any OS using Docker containers
//! with the appropriate toolchains.
//!
//! Dockerfiles are embedded in the binary at compile time and extracted to
//! ~/.ccgo/dockers/ at runtime when needed.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use super::{BuildContext, BuildOptions, BuildResult};
use crate::commands::build::{BuildTarget, WindowsToolchain};

// Embed Dockerfiles at compile time
const DOCKERFILE_LINUX: &str = include_str!("../../dockers/Dockerfile.linux");
const DOCKERFILE_ANDROID: &str = include_str!("../../dockers/Dockerfile.android");
const DOCKERFILE_OHOS: &str = include_str!("../../dockers/Dockerfile.ohos");
const DOCKERFILE_APPLE: &str = include_str!("../../dockers/Dockerfile.apple");
const DOCKERFILE_WINDOWS_MINGW: &str = include_str!("../../dockers/Dockerfile.windows-mingw");
const DOCKERFILE_WINDOWS_MSVC: &str = include_str!("../../dockers/Dockerfile.windows-msvc");
const DOCKERFILE_WINDOWS_ALL: &str = include_str!("../../dockers/Dockerfile.windows-all");
const DOCKERFILE_OPENWRT: &str = include_str!("../../dockers/Dockerfile.openwrt");

// Embed CMake toolchain files at compile time
const CMAKE_TOOLCHAIN_WINDOWS_MSVC: &str = include_str!("../../cmake/windows-msvc.toolchain.cmake");

/// GitHub Container Registry organization for prebuilt images
const GHCR_REPO: &str = "ghcr.io/zhlinh";

/// Platform Docker configuration
pub struct PlatformDockerConfig {
    /// Dockerfile name
    pub dockerfile: &'static str,
    /// Embedded Dockerfile content
    pub dockerfile_content: &'static str,
    /// Local image name
    pub image_name: &'static str,
    /// Remote image URL (GHCR)
    pub remote_image: String,
    /// Estimated image size
    pub size_estimate: &'static str,
}

impl PlatformDockerConfig {
    /// Get Docker configuration for a platform
    pub fn for_platform(options: &super::BuildOptions) -> Option<Self> {
        match options.target {
            BuildTarget::Linux => Some(Self {
                dockerfile: "Dockerfile.linux",
                dockerfile_content: DOCKERFILE_LINUX,
                image_name: "ccgo-builder-linux",
                remote_image: format!("{}/ccgo-builder-linux:latest", GHCR_REPO),
                size_estimate: "~800MB",
            }),
            BuildTarget::Windows => {
                // Choose Docker image based on toolchain parameter
                match options.toolchain {
                    WindowsToolchain::Msvc => Some(Self {
                        dockerfile: "Dockerfile.windows-msvc",
                        dockerfile_content: DOCKERFILE_WINDOWS_MSVC,
                        image_name: "ccgo-builder-windows-msvc",
                        remote_image: format!("{}/ccgo-builder-windows-msvc:latest", GHCR_REPO),
                        size_estimate: "~2.5GB",
                    }),
                    WindowsToolchain::Mingw => Some(Self {
                        dockerfile: "Dockerfile.windows-mingw",
                        dockerfile_content: DOCKERFILE_WINDOWS_MINGW,
                        image_name: "ccgo-builder-windows-mingw",
                        remote_image: format!("{}/ccgo-builder-windows-mingw:latest", GHCR_REPO),
                        size_estimate: "~1.2GB",
                    }),
                    WindowsToolchain::All => Some(Self {
                        dockerfile: "Dockerfile.windows-all",
                        dockerfile_content: DOCKERFILE_WINDOWS_ALL,
                        image_name: "ccgo-builder-windows-all",
                        remote_image: format!("{}/ccgo-builder-windows-all:latest", GHCR_REPO),
                        size_estimate: "~4.5GB",
                    }),
                }
            }
            BuildTarget::Macos => Some(Self {
                dockerfile: "Dockerfile.apple",
                dockerfile_content: DOCKERFILE_APPLE,
                image_name: "ccgo-builder-apple",
                remote_image: format!("{}/ccgo-builder-apple:latest", GHCR_REPO),
                size_estimate: "~2.5GB",
            }),
            BuildTarget::Ios | BuildTarget::Tvos | BuildTarget::Watchos => Some(Self {
                dockerfile: "Dockerfile.apple",
                dockerfile_content: DOCKERFILE_APPLE,
                image_name: "ccgo-builder-apple",
                remote_image: format!("{}/ccgo-builder-apple:latest", GHCR_REPO),
                size_estimate: "~2.5GB",
            }),
            BuildTarget::Android => Some(Self {
                dockerfile: "Dockerfile.android",
                dockerfile_content: DOCKERFILE_ANDROID,
                image_name: "ccgo-builder-android",
                remote_image: format!("{}/ccgo-builder-android:latest", GHCR_REPO),
                size_estimate: "~3.5GB",
            }),
            BuildTarget::Ohos => Some(Self {
                dockerfile: "Dockerfile.ohos",
                dockerfile_content: DOCKERFILE_OHOS,
                image_name: "ccgo-builder-ohos",
                remote_image: format!("{}/ccgo-builder-ohos:latest", GHCR_REPO),
                size_estimate: "~2.5GB",
            }),
            BuildTarget::Openwrt => Some(Self {
                dockerfile: "Dockerfile.openwrt",
                dockerfile_content: DOCKERFILE_OPENWRT,
                image_name: "ccgo-builder-openwrt",
                remote_image: format!("{}/ccgo-builder-openwrt:latest", GHCR_REPO),
                size_estimate: "~1.5GB",
            }),
            _ => None,
        }
    }
}

/// Docker builder for cross-platform builds
pub struct DockerBuilder {
    /// Platform configuration
    config: PlatformDockerConfig,
    /// Build context
    ctx: BuildContext,
    /// Path to Dockerfiles directory (cache directory)
    docker_dir: PathBuf,
}

impl DockerBuilder {
    /// Create a new Docker builder
    pub fn new(ctx: BuildContext) -> Result<Self> {
        let config = PlatformDockerConfig::for_platform(&ctx.options).ok_or_else(|| {
            anyhow::anyhow!(
                "Platform {:?} does not support Docker builds",
                ctx.options.target
            )
        })?;

        // Get or create the Docker cache directory with embedded Dockerfiles
        let docker_dir = Self::ensure_docker_dir(&config)?;

        Ok(Self {
            config,
            ctx,
            docker_dir,
        })
    }

    /// Get the Docker cache directory path (~/.ccgo/dockers/)
    fn get_docker_cache_dir() -> Result<PathBuf> {
        // Check environment variable override first
        if let Ok(dir) = std::env::var("CCGO_DOCKER_DIR") {
            let path = PathBuf::from(dir);
            if path.exists() {
                return Ok(path);
            }
        }

        // Use ~/.ccgo/dockers/ as the default cache directory
        let base_dirs = directories::BaseDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        Ok(base_dirs.home_dir().join(".ccgo").join("dockers"))
    }

    /// Ensure the Docker directory exists and contains the required Dockerfile
    fn ensure_docker_dir(config: &PlatformDockerConfig) -> Result<PathBuf> {
        let docker_dir = Self::get_docker_cache_dir()?;

        // Create the directory if it doesn't exist
        fs::create_dir_all(&docker_dir).with_context(|| {
            format!(
                "Failed to create Docker cache directory: {}",
                docker_dir.display()
            )
        })?;

        // Write the embedded Dockerfile to the cache directory
        let dockerfile_path = docker_dir.join(config.dockerfile);
        fs::write(&dockerfile_path, config.dockerfile_content).with_context(|| {
            format!("Failed to write Dockerfile: {}", dockerfile_path.display())
        })?;

        // Extract embedded CMake toolchain files for MSVC-capable Docker builds
        if matches!(
            config.image_name,
            "ccgo-builder-windows-msvc" | "ccgo-builder-windows-all"
        ) {
            let cmake_dir = docker_dir.join("cmake");
            fs::create_dir_all(&cmake_dir).with_context(|| {
                format!("Failed to create cmake directory: {}", cmake_dir.display())
            })?;

            let toolchain_path = cmake_dir.join("windows-msvc.toolchain.cmake");
            fs::write(&toolchain_path, CMAKE_TOOLCHAIN_WINDOWS_MSVC).with_context(|| {
                format!(
                    "Failed to write toolchain file: {}",
                    toolchain_path.display()
                )
            })?;
        }

        Ok(docker_dir)
    }

    /// Check if Docker is installed and running
    pub fn check_docker(&self) -> Result<()> {
        eprintln!("Checking Docker installation...");

        // Check Docker CLI
        let output = Command::new("docker")
            .arg("--version")
            .output()
            .context("Docker is not installed or not in PATH.\nPlease install Docker Desktop from: https://www.docker.com/products/docker-desktop")?;

        if !output.status.success() {
            bail!("Docker CLI check failed");
        }

        eprintln!("✓ {}", String::from_utf8_lossy(&output.stdout).trim());

        // Check Docker daemon
        let output = Command::new("docker")
            .arg("info")
            .output()
            .context("Failed to connect to Docker daemon")?;

        if !output.status.success() {
            bail!(
                "Docker daemon is not running.\n\n\
                 To fix this:\n  \
                 1. Start Docker Desktop application\n  \
                 2. Wait for Docker to fully initialize (check the whale icon in system tray)\n  \
                 3. Run this command again"
            );
        }

        eprintln!("✓ Docker daemon is running");
        Ok(())
    }

    /// Try to pull prebuilt image from GHCR
    fn pull_prebuilt_image(&self) -> Result<bool> {
        eprintln!("\n=== Checking for prebuilt image ===");
        eprintln!("Image: {}", self.config.remote_image);
        eprintln!("Size: {}", self.config.size_estimate);

        eprintln!("Pulling prebuilt image...");
        eprintln!("(This is much faster than building from scratch)");

        let output = Command::new("docker")
            .args(["pull", &self.config.remote_image])
            .output()
            .context("Failed to pull Docker image")?;

        if !output.status.success() {
            eprintln!("⚠ Could not pull prebuilt image from GHCR");
            eprintln!(
                "  Reason: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
            return Ok(false);
        }

        // Tag the remote image with local name
        let status = Command::new("docker")
            .args(["tag", &self.config.remote_image, self.config.image_name])
            .status()
            .context("Failed to tag Docker image")?;

        if !status.success() {
            bail!("Failed to tag Docker image");
        }

        eprintln!(
            "✓ Successfully pulled prebuilt image: {}",
            self.config.remote_image
        );
        eprintln!("✓ Tagged as: {}", self.config.image_name);
        Ok(true)
    }

    /// Build Docker image if not exists
    pub fn build_image(&self, use_prebuilt: bool) -> Result<()> {
        eprintln!(
            "\n=== Preparing Docker image: {} ===",
            self.config.image_name
        );
        eprintln!("Platform: {}", self.ctx.options.target);

        // Check if image already exists locally
        let output = Command::new("docker")
            .args(["images", "-q", self.config.image_name])
            .output()
            .context("Failed to check Docker images")?;

        if !String::from_utf8_lossy(&output.stdout).trim().is_empty() {
            eprintln!(
                "✓ Image {} already exists locally (using cached image)",
                self.config.image_name
            );
            eprintln!("  To rebuild, run: docker rmi {}", self.config.image_name);
            return Ok(());
        }

        // Try to pull prebuilt image first
        if use_prebuilt {
            if self.pull_prebuilt_image()? {
                return Ok(()); // Successfully pulled
            }

            eprintln!("\n⚠ Prebuilt image not available, building from Dockerfile...");
            eprintln!("  (This will take 5-30 minutes depending on platform)");
        }

        // Build from Dockerfile
        eprintln!("\n=== Building Docker image from Dockerfile ===");
        eprintln!("Dockerfile: {}", self.config.dockerfile);
        eprintln!("Estimated size: {}", self.config.size_estimate);
        eprintln!("Building... (grab a coffee ☕)");

        let dockerfile_path = self.docker_dir.join(self.config.dockerfile);
        if !dockerfile_path.exists() {
            bail!("Dockerfile not found: {}", dockerfile_path.display());
        }

        let status = Command::new("docker")
            .arg("build")
            .arg("-f")
            .arg(&dockerfile_path)
            .arg("-t")
            .arg(self.config.image_name)
            .arg(&self.docker_dir)
            .env("DOCKER_BUILDKIT", "1")
            .status()
            .context("Failed to build Docker image")?;

        if !status.success() {
            bail!("Docker image build failed");
        }

        eprintln!("✓ Image {} built successfully", self.config.image_name);
        Ok(())
    }

    /// Find git root directory
    fn find_git_root(&self) -> Option<PathBuf> {
        let mut current = self.ctx.project_root.clone();
        for _ in 0..10 {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                if git_dir.is_dir() {
                    // Verify it's a valid git directory
                    if git_dir.join("HEAD").exists() {
                        return Some(git_dir);
                    }
                } else if git_dir.is_file() {
                    // Git worktree or submodule
                    return Some(git_dir);
                }
            }

            // Move to parent directory
            let parent = current.parent()?;
            if parent == current {
                break; // Reached filesystem root
            }
            current = parent.to_path_buf();
        }
        None
    }

    /// Flags the in-container `ccgo build` must receive to match the host
    /// build mode. Without these the container defaults to debug and writes
    /// under `target/debug/` even when the caller asked for `--release`.
    fn build_option_args(&self) -> String {
        option_args(&self.ctx.options)
    }

    /// Run build inside Docker container
    pub fn run_build(&self) -> Result<()> {
        eprintln!(
            "\n=== Running {} build in Docker container ===",
            self.ctx.options.target
        );
        eprintln!("Project directory: {}", self.ctx.project_root.display());

        self.clean_output_dir()?;

        // Build Docker run command
        let mut cmd = Command::new("docker");
        cmd.arg("run").arg("--rm").arg("--entrypoint="); // Clear the image's default entrypoint

        self.mount_project(&mut cmd);

        // Set working directory
        cmd.arg("-w").arg("/workspace");

        let use_local_source = self.mount_ccgo_source(&mut cmd);

        // Image name
        cmd.arg(self.config.image_name);

        let (setup_cmd, ccgo_bin) = self.ccgo_setup(use_local_source);
        let build_cmd = self.container_build_command(&setup_cmd, &ccgo_bin);

        cmd.arg("sh").arg("-c").arg(&build_cmd);

        if self.ctx.options.verbose {
            eprintln!("\nDocker command: {:?}", cmd);
        }

        // Run the command
        let status = cmd.status().context("Failed to run Docker container")?;

        if !status.success() {
            bail!("Docker build failed");
        }

        eprintln!("\n✓ Docker build completed successfully!");
        Ok(())
    }

    /// Remove the host output directory so stale artifacts are not collected.
    fn clean_output_dir(&self) -> Result<()> {
        let target_platform_dir = self.ctx.output_dir.parent().unwrap();
        if target_platform_dir.exists() {
            std::fs::remove_dir_all(target_platform_dir)
                .context("Failed to clean target directory")?;
            eprintln!("Cleaned up: {}", target_platform_dir.display());
        }
        Ok(())
    }

    /// Mount the project (and its `.git` when available) into the container.
    fn mount_project(&self, cmd: &mut Command) {
        cmd.arg("-v")
            .arg(format!("{}:/workspace", self.ctx.project_root.display()));

        if let Some(git_dir) = self.find_git_root() {
            cmd.arg("-v")
                .arg(format!("{}:/workspace/.git:ro", git_dir.display()));
            if let Some(root) = git_dir.parent() {
                eprintln!(
                    "Git repository: {} (mounted .git to container)",
                    root.display()
                );
            }
        } else {
            eprintln!("⚠ No git repository found (git info will be 'unknown')");
        }
    }

    /// Mount a local ccgo checkout when `--dev` can find one.
    ///
    /// Returns whether the container should build ccgo from that source.
    fn mount_ccgo_source(&self, cmd: &mut Command) -> bool {
        if !self.ctx.options.dev {
            eprintln!("Using pre-installed ccgo from Docker image (PyPI 3.x+ Rust version)...");
            return false;
        }

        let Some(path) = Self::find_ccgo_source() else {
            eprintln!("Using --dev mode: will download pre-built ccgo from GitHub releases");
            return false;
        };

        cmd.arg("-v").arg(format!(
            "{}:/ccgo-src:ro",
            path.canonicalize()
                .unwrap_or_else(|_| path.clone())
                .display()
        ));
        eprintln!(
            "Using --dev mode: mounting local ccgo source from {}",
            path.display()
        );
        self.mount_cargo_cache(cmd);
        true
    }

    /// Locate a ccgo source checkout from `CCGO_SRC_PATH` or the executable path.
    fn find_ccgo_source() -> Option<PathBuf> {
        let env_path = std::env::var("CCGO_SRC_PATH").ok().map(PathBuf::from);
        let exe_path = std::env::current_exe().ok().and_then(|exe| {
            exe.parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
                .map(PathBuf::from)
        });

        env_path
            .into_iter()
            .chain(exe_path)
            .find(|path| path.join("Cargo.toml").exists())
    }

    /// Mount the cargo registry cache to speed up repeated `--dev` builds.
    fn mount_cargo_cache(&self, cmd: &mut Command) {
        let cargo_cache_dir = Self::get_docker_cache_dir()
            .map(|dir| dir.parent().unwrap().join("cargo-cache"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/ccgo-cargo-cache"));

        if let Err(error) = fs::create_dir_all(&cargo_cache_dir) {
            eprintln!("⚠ Could not create cargo cache directory: {}", error);
            return;
        }

        cmd.arg("-v").arg(format!(
            "{}:/usr/local/cargo/registry",
            cargo_cache_dir.display()
        ));
        eprintln!("Using cargo cache: {}", cargo_cache_dir.display());
    }

    /// Shell fragment that provides the ccgo binary plus the path to run.
    fn ccgo_setup(&self, use_local_source: bool) -> (String, String) {
        if use_local_source {
            // Unset cross-compilers so the Linux ccgo binary is built with the
            // native host compiler.
            let build = "env -u CC -u CXX -u AR -u RANLIB \
                 CARGO_TARGET_DIR=/tmp/ccgo-build cargo build --release \
                 --manifest-path /ccgo-src/Cargo.toml"
                .to_string();
            return (build, "/tmp/ccgo-build/release/ccgo".to_string());
        }

        if self.ctx.options.dev {
            return (Self::ccgo_download_command(), "/tmp/ccgo-bin".to_string());
        }

        (
            "command -v ccgo >/dev/null 2>&1 || \
             (pip3 install -q ccgo) || \
             (echo 'ERROR: ccgo not found.' && exit 1)"
                .to_string(),
            "ccgo".to_string(),
        )
    }

    /// Download a prebuilt ccgo for the container's architecture.
    fn ccgo_download_command() -> String {
        r#"ARCH=$(uname -m) && \
                 if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then \
                   TRIPLE="aarch64-unknown-linux-gnu"; \
                 else \
                   TRIPLE="x86_64-unknown-linux-gnu"; \
                 fi && \
                 echo "Downloading pre-built ccgo from GitHub releases (${TRIPLE})..." && \
                 curl -fsSL "https://github.com/zhlinh/ccgo/releases/latest/download/ccgo-${TRIPLE}.tar.gz" -o /tmp/ccgo.tar.gz && \
                 tar xzf /tmp/ccgo.tar.gz -C /tmp && \
                 chmod +x "/tmp/ccgo-${TRIPLE}/ccgo" && \
                 ln -sf "/tmp/ccgo-${TRIPLE}/ccgo" /tmp/ccgo-bin || \
                 (echo 'ERROR: Failed to download ccgo from GitHub releases.' && \
                  echo 'No release found. Try without --dev flag to use pre-installed ccgo' && \
                  exit 1)"#
            .to_string()
    }

    /// Build the shell command executed inside the container.
    fn container_build_command(&self, setup_cmd: &str, ccgo_bin: &str) -> String {
        let link_type = self.ctx.options.link_type.to_string();
        let option_args = self.build_option_args();

        match self.ctx.options.target {
            BuildTarget::Android => {
                let arch = arch_arg(&self.ctx.options, Some("armeabi-v7a,arm64-v8a,x86_64"));
                format!(
                    "{} && {} build android --native-only{} --build-as {}{}",
                    setup_cmd, ccgo_bin, arch, link_type, option_args
                )
            }
            BuildTarget::Ohos => {
                let arch = arch_arg(&self.ctx.options, Some("armeabi-v7a,arm64-v8a,x86_64"));
                format!(
                    "{} && {} build ohos --native-only{} --build-as {}{}",
                    setup_cmd, ccgo_bin, arch, link_type, option_args
                )
            }
            BuildTarget::Openwrt => {
                let arch = arch_arg(&self.ctx.options, Some("mipsel_24kc,aarch64"));
                format!(
                    "{} && {} build openwrt --build-as {}{}{}",
                    setup_cmd, ccgo_bin, link_type, arch, option_args
                )
            }
            _ => {
                let platform = self.ctx.options.target.to_string().to_lowercase();
                let toolchain = toolchain_arg(&self.ctx.options);
                let arch = arch_arg(&self.ctx.options, None);
                format!(
                    "{} && {} build {} --build-as {}{}{}{}",
                    setup_cmd, ccgo_bin, platform, link_type, toolchain, arch, option_args
                )
            }
        }
    }

    /// Execute full Docker build workflow
    pub fn execute(&self) -> Result<BuildResult> {
        let start = Instant::now();

        // 1. Check Docker
        self.check_docker()?;

        // 2. Build/pull image
        // FIXME: Temporarily disable prebuilt images to test new code
        self.build_image(false)?;

        // 3. Run build
        self.run_build()?;

        let duration = start.elapsed();

        // 4. Collect build results from target directory
        self.collect_build_results(duration.as_secs_f64())
    }

    /// Collect build results from target directory after Docker build
    fn collect_build_results(&self, duration_secs: f64) -> Result<BuildResult> {
        let platform = self.ctx.options.target.to_string().to_lowercase();

        // The container receives the same mode/profile/sanitizer flags, so the
        // host-computed output directory is authoritative. Fall back to the old
        // `target/<platform>/` layout for older pre-installed ccgo versions.
        let old_scan_dir = self.ctx.project_root.join("target").join(&platform);
        let scan_dir = if self.ctx.output_dir.exists() {
            self.ctx.output_dir.clone()
        } else {
            old_scan_dir
        };

        let artifacts = scan_docker_artifacts(&scan_dir, &platform, &self.ctx.options)?;

        let sdk_archive = artifacts.sdk.ok_or_else(|| {
            anyhow::anyhow!(
                "Docker build completed but SDK archive not found in {}",
                scan_dir.display()
            )
        })?;

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: artifacts.symbols,
            aar_archive: artifacts.aar,
            duration_secs,
            architectures: artifacts.architectures,
        })
    }
}

/// Artifacts produced by a Docker build and copied back to the host.
#[derive(Default)]
struct DockerArtifacts {
    sdk: Option<PathBuf>,
    symbols: Option<PathBuf>,
    aar: Option<PathBuf>,
    architectures: Vec<String>,
}

/// Scan the host output directory for the archives produced in the container.
fn scan_docker_artifacts(
    scan_dir: &Path,
    platform: &str,
    options: &BuildOptions,
) -> Result<DockerArtifacts> {
    let mut artifacts = DockerArtifacts::default();
    if !scan_dir.exists() {
        return Ok(artifacts);
    }

    for entry in std::fs::read_dir(scan_dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("");

        if file_name.contains("_SDK-")
            && file_name.ends_with(".zip")
            && !file_name.contains("-SYMBOLS.zip")
        {
            artifacts.sdk = Some(path);
            artifacts.architectures = default_architectures(platform, options);
        } else if file_name.contains("-SYMBOLS.zip") {
            artifacts.symbols = Some(path);
        } else if file_name.ends_with(".aar") && platform == "android" {
            artifacts.aar = Some(path);
        }
    }

    Ok(artifacts)
}

/// Architectures reported by a Docker build when the output carries no metadata.
fn default_architectures(platform: &str, options: &BuildOptions) -> Vec<String> {
    if !options.architectures.is_empty() {
        return options.architectures.clone();
    }

    let arches: &[&str] = match platform {
        "android" => &["armeabi-v7a", "arm64-v8a", "x86_64"],
        "openwrt" => &["mipsel_24kc", "aarch64"],
        "windows" => &["x86_64", "aarch64"],
        "linux" => &["x86_64"],
        "macos" => &["x86_64", "arm64"],
        "ios" => &["arm64", "armv7", "x86_64"],
        _ => &[],
    };

    arches.iter().map(|arch| (*arch).to_string()).collect()
}

/// The toolchain passed to the in-container ccgo. The `all` image installs both
/// MinGW and the xwin-based MSVC toolchain, so the host selection is forwarded
/// unchanged.
fn toolchain_arg(options: &BuildOptions) -> String {
    if options.target != BuildTarget::Windows {
        return String::new();
    }
    format!(" --toolchain {}", options.toolchain)
}

/// Explicit architectures, or the platform's default set.
fn arch_arg(options: &BuildOptions, default: Option<&str>) -> String {
    if !options.architectures.is_empty() {
        format!(" --arch {}", options.architectures.join(","))
    } else {
        default
            .map(|arch| format!(" --arch {arch}"))
            .unwrap_or_default()
    }
}

/// Flags shared by every in-container `ccgo build` invocation.
fn option_args(options: &BuildOptions) -> String {
    let mut args: Vec<String> = Vec::new();
    if options.release {
        args.push("--release".to_string());
    }
    if let Some(profile) = &options.profile_name {
        args.push(format!("--profile {profile}"));
    }
    if let Some(sanitizer) = &options.sanitizer {
        args.push(
            match sanitizer {
                crate::builder::sanitizer::SanitizerKind::Address => "--asan",
                crate::builder::sanitizer::SanitizerKind::Thread => "--tsan",
            }
            .to_string(),
        );
    }
    append_feature_args(&mut args, options);
    if matches!(options.target, BuildTarget::Android | BuildTarget::Ohos) {
        if let Some(stl) = &options.stl {
            args.push(format!("--stl {stl}"));
        }
    }
    if let Some(jobs) = options.jobs {
        args.push(format!("--jobs {jobs}"));
    }
    if options.native_only && !matches!(options.target, BuildTarget::Android | BuildTarget::Ohos) {
        args.push("--native-only".to_string());
    }
    if options.no_fetch {
        args.push("--no-fetch".to_string());
    }
    if options.verbose {
        args.push("--verbose".to_string());
    }
    if options.ide_project {
        args.push("--ide-project".to_string());
    }
    if let Some(cache) = &options.cache {
        if cache != "auto" {
            args.push(format!("--cache {cache}"));
        }
    }
    append_linkage_args(&mut args, options);
    if args.is_empty() {
        String::new()
    } else {
        format!(" {}", args.join(" "))
    }
}

fn append_feature_args(args: &mut Vec<String>, options: &BuildOptions) {
    if !options.features.is_empty() {
        args.push(format!("--features {}", options.features.join(",")));
    }
    if !options.use_default_features {
        args.push("--no-default-features".to_string());
    }
    if options.all_features {
        args.push("--all-features".to_string());
    }
    if !options.variants.is_empty() {
        args.push(format!("--variant {}", options.variants.join(",")));
    }
}

fn append_linkage_args(args: &mut Vec<String>, options: &BuildOptions) {
    if let Some(value) = &options.linkage_default {
        args.push(format!("--linkage {value}"));
    }
    let mut overrides: Vec<_> = options.linkage_overrides.iter().collect();
    overrides.sort_by_key(|(name, _)| *name);
    for (name, value) in overrides {
        args.push(format!("--linkage {name}={value}"));
    }
    if let Some(value) = &options.linkage_on_shared_default {
        args.push(format!("--linkage-on-shared {value}"));
    }
    let mut shared: Vec<_> = options.linkage_on_shared_overrides.iter().collect();
    shared.sort_by_key(|(name, _)| *name);
    for (name, value) in shared {
        args.push(format!("--linkage-on-shared {name}={value}"));
    }
    if let Some(value) = &options.linkage_on_static_default {
        args.push(format!("--linkage-on-static {value}"));
    }
    let mut r#static: Vec<_> = options.linkage_on_static_overrides.iter().collect();
    r#static.sort_by_key(|(name, _)| *name);
    for (name, value) in r#static {
        args.push(format!("--linkage-on-static {name}={value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docker_options_forward_release_and_build_mode() {
        let mut options = BuildOptions {
            target: BuildTarget::Windows,
            release: true,
            ..Default::default()
        };
        assert!(option_args(&options).contains("--release"));

        options.release = false;
        assert!(!option_args(&options).contains("--release"));

        options.profile_name = Some("ci".into());
        options.sanitizer = Some(crate::builder::sanitizer::SanitizerKind::Address);
        options.features = vec!["networking".into()];
        options.variants = vec!["oversea".into()];
        options.jobs = Some(4);
        let args = option_args(&options);
        assert!(args.contains("--profile ci"));
        assert!(args.contains("--asan"));
        assert!(args.contains("--features networking"));
        assert!(args.contains("--variant oversea"));
        assert!(args.contains("--jobs 4"));
    }

    #[test]
    fn docker_forwards_toolchain_selection_unchanged() {
        let mut options = BuildOptions {
            target: BuildTarget::Windows,
            ..Default::default()
        };
        assert_eq!(toolchain_arg(&options), " --toolchain all");

        options.toolchain = WindowsToolchain::Mingw;
        assert_eq!(toolchain_arg(&options), " --toolchain mingw");

        options.toolchain = WindowsToolchain::Msvc;
        assert_eq!(toolchain_arg(&options), " --toolchain msvc");

        options.target = BuildTarget::Linux;
        assert_eq!(toolchain_arg(&options), "");
    }

    #[test]
    fn windows_toolchain_selects_matching_docker_image() {
        let mut options = BuildOptions {
            target: BuildTarget::Windows,
            ..Default::default()
        };
        let all = PlatformDockerConfig::for_platform(&options).unwrap();
        assert_eq!(all.image_name, "ccgo-builder-windows-all");
        assert_eq!(all.dockerfile, "Dockerfile.windows-all");

        options.toolchain = WindowsToolchain::Mingw;
        let mingw = PlatformDockerConfig::for_platform(&options).unwrap();
        assert_eq!(mingw.image_name, "ccgo-builder-windows-mingw");

        options.toolchain = WindowsToolchain::Msvc;
        let msvc = PlatformDockerConfig::for_platform(&options).unwrap();
        assert_eq!(msvc.image_name, "ccgo-builder-windows-msvc");
    }

    #[test]
    fn docker_arch_uses_explicit_values_or_platform_defaults() {
        let mut options = BuildOptions {
            target: BuildTarget::Android,
            architectures: vec!["arm64-v8a".into()],
            ..Default::default()
        };
        assert_eq!(
            arch_arg(&options, Some("arm64-v8a,x86_64")),
            " --arch arm64-v8a"
        );

        options.architectures.clear();
        assert_eq!(
            arch_arg(&options, Some("arm64-v8a,x86_64")),
            " --arch arm64-v8a,x86_64"
        );
        assert_eq!(arch_arg(&options, None), "");
    }

    #[test]
    fn docker_options_forward_linkage_and_ide_flags() {
        use crate::config::Linkage;
        let mut options = BuildOptions {
            target: BuildTarget::Linux,
            ide_project: true,
            ..Default::default()
        };
        options.linkage_default = Some(Linkage::SharedExternal);
        options
            .linkage_overrides
            .insert("leaf".into(), Linkage::StaticEmbedded);
        options.no_fetch = true;
        options.verbose = true;
        let args = option_args(&options);
        assert!(args.contains("--ide-project"));
        assert!(args.contains("--no-fetch"));
        assert!(args.contains("--verbose"));
        assert!(args.contains("--linkage shared-external"));
        assert!(args.contains("--linkage leaf=static-embedded"));
    }

    #[test]
    fn docker_options_forward_android_stl_only_for_android_ohos() {
        let mut options = BuildOptions {
            target: BuildTarget::Android,
            stl: Some("c++_static".into()),
            ..Default::default()
        };
        assert!(option_args(&options).contains("--stl c++_static"));

        options.target = BuildTarget::Windows;
        assert!(!option_args(&options).contains("--stl"));
    }
}
