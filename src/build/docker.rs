//! Docker-based cross-platform build support
//!
//! Enables building all platform libraries on any OS using Docker containers
//! with the appropriate toolchains.
//!
//! Dockerfiles are embedded in the binary at compile time and extracted to
//! ~/.ccgo/dockers/ at runtime when needed.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use super::{BuildContext, BuildResult};
use crate::commands::build::BuildTarget;

// Embed Dockerfiles at compile time
const DOCKERFILE_LINUX: &str = include_str!("../../dockers/Dockerfile.linux");
const DOCKERFILE_ANDROID: &str = include_str!("../../dockers/Dockerfile.android");
const DOCKERFILE_OHOS: &str = include_str!("../../dockers/Dockerfile.ohos");
const DOCKERFILE_APPLE: &str = include_str!("../../dockers/Dockerfile.apple");
const DOCKERFILE_WINDOWS_MINGW: &str = include_str!("../../dockers/Dockerfile.windows-mingw");
const DOCKERFILE_WINDOWS_MSVC: &str = include_str!("../../dockers/Dockerfile.windows-msvc");

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
    pub fn for_platform(platform: &BuildTarget) -> Option<Self> {
        match platform {
            BuildTarget::Linux => Some(Self {
                dockerfile: "Dockerfile.linux",
                dockerfile_content: DOCKERFILE_LINUX,
                image_name: "ccgo-builder-linux",
                remote_image: format!("{}/ccgo-builder-linux:latest", GHCR_REPO),
                size_estimate: "~800MB",
            }),
            BuildTarget::Windows => Some(Self {
                dockerfile: "Dockerfile.windows-mingw",
                dockerfile_content: DOCKERFILE_WINDOWS_MINGW,
                image_name: "ccgo-builder-windows-mingw",
                remote_image: format!("{}/ccgo-builder-windows-mingw:latest", GHCR_REPO),
                size_estimate: "~1.2GB",
            }),
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
        let config = PlatformDockerConfig::for_platform(&ctx.options.target)
            .ok_or_else(|| anyhow::anyhow!("Platform {:?} does not support Docker builds", ctx.options.target))?;

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
        fs::create_dir_all(&docker_dir)
            .with_context(|| format!("Failed to create Docker cache directory: {}", docker_dir.display()))?;

        // Write the embedded Dockerfile to the cache directory
        let dockerfile_path = docker_dir.join(config.dockerfile);
        fs::write(&dockerfile_path, config.dockerfile_content)
            .with_context(|| format!("Failed to write Dockerfile: {}", dockerfile_path.display()))?;

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
            eprintln!("  Reason: {}", String::from_utf8_lossy(&output.stderr).trim());
            return Ok(false);
        }

        // Tag the remote image with local name
        let status = Command::new("docker")
            .args(["tag", &self.config.remote_image, &self.config.image_name])
            .status()
            .context("Failed to tag Docker image")?;

        if !status.success() {
            bail!("Failed to tag Docker image");
        }

        eprintln!("✓ Successfully pulled prebuilt image: {}", self.config.remote_image);
        eprintln!("✓ Tagged as: {}", self.config.image_name);
        Ok(true)
    }

    /// Build Docker image if not exists
    pub fn build_image(&self, use_prebuilt: bool) -> Result<()> {
        eprintln!("\n=== Preparing Docker image: {} ===", self.config.image_name);
        eprintln!("Platform: {}", self.ctx.options.target);

        // Check if image already exists locally
        let output = Command::new("docker")
            .args(["images", "-q", self.config.image_name])
            .output()
            .context("Failed to check Docker images")?;

        if !String::from_utf8_lossy(&output.stdout).trim().is_empty() {
            eprintln!("✓ Image {} already exists locally (using cached image)", self.config.image_name);
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

    /// Run build inside Docker container
    pub fn run_build(&self) -> Result<()> {
        eprintln!("\n=== Running {} build in Docker container ===", self.ctx.options.target);
        eprintln!("Project directory: {}", self.ctx.project_root.display());

        // Clean target/{platform} directory before Docker build
        let target_platform_dir = self.ctx.output_dir.parent().unwrap();
        if target_platform_dir.exists() {
            std::fs::remove_dir_all(target_platform_dir)
                .context("Failed to clean target directory")?;
            eprintln!("Cleaned up: {}", target_platform_dir.display());
        }

        // Build Docker run command
        let mut cmd = Command::new("docker");
        cmd.arg("run")
            .arg("--rm")
            .arg("--entrypoint=");  // Clear the image's default entrypoint

        // Mount project directory
        cmd.arg("-v").arg(format!("{}:/workspace", self.ctx.project_root.display()));

        // Mount .git directory if found
        if let Some(git_dir) = self.find_git_root() {
            cmd.arg("-v").arg(format!("{}:/workspace/.git:ro", git_dir.display()));
            eprintln!("Git repository: {} (mounted .git to container)", git_dir.parent().unwrap().display());
        } else {
            eprintln!("⚠ No git repository found (git info will be 'unknown')");
        }

        // Set working directory
        cmd.arg("-w").arg("/workspace");

        // Determine ccgo installation method:
        // Default mode: Install ccgo from crates.io (for normal users)
        // --dev mode: Build from local source or download pre-built binary (for developers)
        let mut use_local_source = false;

        if self.ctx.options.dev {
            // Dev mode: Try to find and mount local ccgo source
            let ccgo_src_path = std::env::var("CCGO_SRC_PATH")
                .map(PathBuf::from)
                .or_else(|_| {
                    // Try to find ccgo source relative to current executable
                    if let Ok(exe) = std::env::current_exe() {
                        // Go up from target/debug/ccgo or target/release/ccgo to project root
                        if let Some(root) = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()) {
                            if root.join("Cargo.toml").exists() {
                                return Ok(root.to_path_buf());
                            }
                        }
                    }
                    Err(())
                });

            if let Ok(path) = ccgo_src_path {
                if path.join("Cargo.toml").exists() {
                    cmd.arg("-v").arg(format!("{}:/ccgo-src:ro", path.canonicalize().unwrap_or(path.clone()).display()));
                    eprintln!("Using --dev mode: mounting local ccgo source from {}", path.display());

                    // Mount cargo cache to speed up repeated builds
                    let cargo_cache_dir = Self::get_docker_cache_dir()
                        .map(|d| d.parent().unwrap().join("cargo-cache"))
                        .unwrap_or_else(|_| PathBuf::from("/tmp/ccgo-cargo-cache"));
                    if let Err(e) = fs::create_dir_all(&cargo_cache_dir) {
                        eprintln!("⚠ Could not create cargo cache directory: {}", e);
                    } else {
                        cmd.arg("-v").arg(format!("{}:/usr/local/cargo/registry", cargo_cache_dir.display()));
                        eprintln!("Using cargo cache: {}", cargo_cache_dir.display());
                    }
                    use_local_source = true;
                }
            }

            if !use_local_source {
                eprintln!("Using --dev mode: will download pre-built ccgo from GitHub releases");
            }
        } else {
            eprintln!("Using pre-installed ccgo from Docker image...");
        }

        // Image name
        cmd.arg(self.config.image_name);

        // Build command to run in container
        let platform = self.ctx.options.target.to_string().to_lowercase();
        let link_type = self.ctx.options.link_type.to_string();

        // Determine how to get ccgo binary
        // Default: Use pre-installed ccgo from Docker image (fastest)
        // --dev with local source: Build from source
        // --dev without local source: Download pre-built from GitHub releases
        let (setup_cmd, ccgo_bin) = if use_local_source {
            // Build ccgo from local source using cargo
            let cargo_build_cmd = "CARGO_TARGET_DIR=/tmp/ccgo-build cargo build --release --manifest-path /ccgo-src/Cargo.toml".to_string();
            (cargo_build_cmd, "/tmp/ccgo-build/release/ccgo".to_string())
        } else if self.ctx.options.dev {
            // Download pre-built ccgo from GitHub releases
            let download_cmd = format!(
                "echo 'Downloading pre-built ccgo from GitHub releases...' && \
                 curl -fsSL https://github.com/zhlinh/ccgo/releases/latest/download/ccgo-x86_64-unknown-linux-gnu.tar.gz -o /tmp/ccgo.tar.gz && \
                 tar xzf /tmp/ccgo.tar.gz -C /tmp && \
                 chmod +x /tmp/ccgo-x86_64-unknown-linux-gnu/ccgo || \
                 (echo 'ERROR: Failed to download ccgo from GitHub releases.' && \
                  echo 'No release found. Try without --dev flag to use pre-installed ccgo' && \
                  exit 1)"
            );
            (download_cmd, "/tmp/ccgo-x86_64-unknown-linux-gnu/ccgo".to_string())
        } else {
            // Default: Use pre-installed ccgo from Docker image
            // Fall back to pip install if not available, with helpful error message
            let setup_cmd = format!(
                "command -v ccgo >/dev/null 2>&1 || \
                 (command -v pip3 >/dev/null 2>&1 && pip3 install -q ccgo) || \
                 (echo 'ERROR: ccgo not found and pip3 not available.' && \
                  echo 'Your Docker image may be outdated. Please rebuild it:' && \
                  echo '  docker rmi {}' && \
                  echo 'Then run your build command again.' && \
                  exit 1)",
                self.config.image_name
            );
            (setup_cmd, "ccgo".to_string())
        };

        // Build toolchain argument for Windows builds
        let toolchain_arg = if self.ctx.options.target == BuildTarget::Windows {
            format!(" --toolchain {}", self.ctx.options.toolchain)
        } else {
            String::new()
        };

        let build_cmd = if self.ctx.options.target == BuildTarget::Android {
            format!(
                "{} && \
                 {} build android --native-only \
                 --arch armeabi-v7a,arm64-v8a,x86_64 --link-type {}",
                setup_cmd, ccgo_bin, link_type
            )
        } else if self.ctx.options.target == BuildTarget::Ohos {
            format!(
                "{} && \
                 {} build ohos --native-only \
                 --arch armeabi-v7a,arm64-v8a,x86_64 --link-type {}",
                setup_cmd, ccgo_bin, link_type
            )
        } else {
            format!(
                "{} && \
                 {} build {} --link-type {}{}",
                setup_cmd, ccgo_bin, platform, link_type, toolchain_arg
            )
        };

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

        // Python ccgo inside Docker may output to:
        // - New structure: target/{release|debug}/{platform}/
        // - Old structure: target/{platform}/
        // We scan both to ensure compatibility
        let release_subdir = if self.ctx.options.release { "release" } else { "debug" };
        let new_scan_dir = self.ctx.project_root.join("target").join(release_subdir).join(&platform);
        let old_scan_dir = self.ctx.project_root.join("target").join(&platform);

        // Scan output directory for generated archives
        let mut sdk_archive: Option<PathBuf> = None;
        let mut symbols_archive: Option<PathBuf> = None;
        let mut aar_archive: Option<PathBuf> = None;
        let mut architectures: Vec<String> = Vec::new();

        // Try new structure first, fall back to old structure
        let scan_dir = if new_scan_dir.exists() {
            new_scan_dir
        } else {
            old_scan_dir
        };

        if scan_dir.exists() {
            for entry in std::fs::read_dir(&scan_dir)? {
                let entry = entry?;
                let path = entry.path();

                if !path.is_file() {
                    continue;
                }

                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");

                // Look for SDK archive: {lib}_{platform}_SDK-{version}.zip (exclude -SYMBOLS.zip)
                if file_name.contains("_SDK-") && file_name.ends_with(".zip") && !file_name.contains("-SYMBOLS.zip") {
                    sdk_archive = Some(path.clone());

                    // Extract architectures from archive name if present
                    // Format: {lib}_{platform}_SDK-{version}.zip
                    // For multi-arch: might contain arch info in path
                    if platform == "android" {
                        architectures = vec![
                            "armeabi-v7a".to_string(),
                            "arm64-v8a".to_string(),
                            "x86_64".to_string(),
                        ];
                    } else if platform == "linux" || platform == "windows" {
                        architectures = vec!["x86_64".to_string()];
                    } else if platform == "macos" {
                        architectures = vec!["x86_64".to_string(), "arm64".to_string()];
                    } else if platform == "ios" {
                        architectures = vec![
                            "arm64".to_string(),
                            "armv7".to_string(),
                            "x86_64".to_string(), // simulator
                        ];
                    }
                }

                // Look for symbols archive: {lib}_{platform}_SDK-{version}-SYMBOLS.zip
                if file_name.contains("-SYMBOLS.zip") {
                    symbols_archive = Some(path.clone());
                }

                // Look for AAR archive (Android): {lib}-{version}.aar
                if file_name.ends_with(".aar") && platform == "android" {
                    aar_archive = Some(path.clone());
                }
            }
        }

        let sdk_archive = sdk_archive
            .ok_or_else(|| anyhow::anyhow!(
                "Docker build completed but SDK archive not found in {}",
                scan_dir.display()
            ))?;

        Ok(BuildResult {
            sdk_archive,
            symbols_archive,
            aar_archive,
            duration_secs,
            architectures,
        })
    }
}
