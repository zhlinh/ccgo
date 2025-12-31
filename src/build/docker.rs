//! Docker-based cross-platform build support
//!
//! Enables building all platform libraries on any OS using Docker containers
//! with the appropriate toolchains.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use super::{BuildContext, BuildResult};
use crate::commands::build::BuildTarget;

/// GitHub Container Registry organization for prebuilt images
const GHCR_REPO: &str = "ghcr.io/zhlinh";

/// Platform Docker configuration
pub struct PlatformDockerConfig {
    /// Dockerfile name
    pub dockerfile: &'static str,
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
                image_name: "ccgo-builder-linux",
                remote_image: format!("{}/ccgo-builder-linux:latest", GHCR_REPO),
                size_estimate: "~800MB",
            }),
            BuildTarget::Windows => Some(Self {
                dockerfile: "Dockerfile.windows-mingw",
                image_name: "ccgo-builder-windows-mingw",
                remote_image: format!("{}/ccgo-builder-windows-mingw:latest", GHCR_REPO),
                size_estimate: "~1.2GB",
            }),
            BuildTarget::Macos => Some(Self {
                dockerfile: "Dockerfile.apple",
                image_name: "ccgo-builder-apple",
                remote_image: format!("{}/ccgo-builder-apple:latest", GHCR_REPO),
                size_estimate: "~2.5GB",
            }),
            BuildTarget::Ios | BuildTarget::Tvos | BuildTarget::Watchos => Some(Self {
                dockerfile: "Dockerfile.apple",
                image_name: "ccgo-builder-apple",
                remote_image: format!("{}/ccgo-builder-apple:latest", GHCR_REPO),
                size_estimate: "~2.5GB",
            }),
            BuildTarget::Android => Some(Self {
                dockerfile: "Dockerfile.android",
                image_name: "ccgo-builder-android",
                remote_image: format!("{}/ccgo-builder-android:latest", GHCR_REPO),
                size_estimate: "~3.5GB",
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
    /// Path to Dockerfiles directory
    docker_dir: PathBuf,
}

impl DockerBuilder {
    /// Create a new Docker builder
    pub fn new(ctx: BuildContext) -> Result<Self> {
        let config = PlatformDockerConfig::for_platform(&ctx.options.target)
            .ok_or_else(|| anyhow::anyhow!("Platform {:?} does not support Docker builds", ctx.options.target))?;

        // Find Dockerfiles directory
        let docker_dir = Self::find_docker_dir()?;

        Ok(Self {
            config,
            ctx,
            docker_dir,
        })
    }

    /// Find the Dockerfiles directory
    fn find_docker_dir() -> Result<PathBuf> {
        // Try to find Dockerfiles in the following locations:
        // 1. Environment variable CCGO_DOCKER_DIR (override)
        // 2. Relative to ccgo-rs binary in development (ccgo-rs/target/debug -> ccgo-rs/dockers)
        // 3. Relative to ccgo-rs binary when installed (~/.cargo/bin -> look for ccgo-rs repo)
        // 4. Via git repository root (find ccgo-rs/dockers)

        // 1. Check environment variable (allows user override)
        if let Ok(dir) = std::env::var("CCGO_DOCKER_DIR") {
            let path = PathBuf::from(dir);
            if path.exists() {
                return Ok(path);
            }
        }

        // 2. Check relative to current executable (development mode)
        // ccgo-rs/target/debug/ccgo -> ccgo-rs/dockers
        // ccgo-rs/target/release/ccgo -> ccgo-rs/dockers
        if let Ok(exe) = std::env::current_exe() {
            if let Some(ccgo_rs_root) = exe.parent()
                .and_then(|p| p.parent())  // target/debug -> target
                .and_then(|p| p.parent())  // target -> ccgo-rs
            {
                let docker_dir = ccgo_rs_root.join("dockers");
                if docker_dir.exists() {
                    return Ok(docker_dir);
                }
            }
        }

        // 3. Try to find ccgo-rs repository via git (works when binary is installed)
        // This searches for ccgo-rs git repository starting from current directory
        if let Ok(output) = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .output()
        {
            if output.status.success() {
                let git_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let git_root_path = PathBuf::from(&git_root);

                // Check if this is ccgo-rs repository itself
                let docker_dir = git_root_path.join("dockers");
                if docker_dir.exists() {
                    return Ok(docker_dir);
                }

                // Check if ccgo-rs is a subdirectory (mono-repo structure)
                // ccgo-group/ccgo-rs/dockers
                if let Some(parent) = git_root_path.parent() {
                    let docker_dir = parent.join("ccgo-rs/dockers");
                    if docker_dir.exists() {
                        return Ok(docker_dir);
                    }
                }
            }
        }

        // 4. Search for ccgo-rs repository in common development locations
        // Check if we're in ccgo-group or ccgo-rs directory structure
        if let Ok(current_dir) = std::env::current_dir() {
            let mut search_dir = current_dir.clone();
            for _ in 0..5 {  // Search up to 5 levels
                // Check ccgo-rs/dockers in current level
                let docker_dir = search_dir.join("ccgo-rs/dockers");
                if docker_dir.exists() {
                    return Ok(docker_dir);
                }

                // Check dockers in current level (if we're in ccgo-rs itself)
                let docker_dir = search_dir.join("dockers");
                if docker_dir.exists() && search_dir.ends_with("ccgo-rs") {
                    return Ok(docker_dir);
                }

                // Move up one directory
                if let Some(parent) = search_dir.parent() {
                    search_dir = parent.to_path_buf();
                } else {
                    break;
                }
            }
        }

        bail!(
            "Could not find Dockerfiles directory.\n\n\
             Searched for ccgo-rs/dockers/ in:\n  \
             1. CCGO_DOCKER_DIR environment variable\n  \
             2. Relative to ccgo binary (development mode)\n  \
             3. Git repository root\n  \
             4. Parent directories\n\n\
             To fix:\n  \
             • Set CCGO_DOCKER_DIR environment variable to the dockers directory\n  \
             • Or run from within a project in the ccgo-rs repository"
        )
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

        // Image name
        cmd.arg(self.config.image_name);

        // Build command to run in container
        // Install Python ccgo from PyPI and run build (CCGO.toml has backward compatibility keys)
        let platform = self.ctx.options.target.to_string().to_lowercase();
        let link_type = self.ctx.options.link_type.to_string();

        let build_cmd = if self.ctx.options.target == BuildTarget::Android {
            format!(
                "pip3 install -q --upgrade pip && \
                 pip3 install -q ccgo && \
                 python3 -m ccgo.main build android --native-only --archive \
                 --arch armeabi-v7a,arm64-v8a,x86_64 --no-docker --link-type {}",
                link_type
            )
        } else {
            format!(
                "pip3 install -q --upgrade pip && \
                 pip3 install -q ccgo && \
                 python3 -m ccgo.main build {} --no-docker --link-type {}",
                platform, link_type
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
        self.build_image(true)?;

        // 3. Run build
        self.run_build()?;

        let duration = start.elapsed();

        // 4. Collect build results from target directory
        self.collect_build_results(duration.as_secs_f64())
    }

    /// Collect build results from target directory after Docker build
    fn collect_build_results(&self, duration_secs: f64) -> Result<BuildResult> {
        let platform = self.ctx.options.target.to_string().to_lowercase();

        // Python ccgo inside Docker outputs to target/{platform}/ (not target/debug/{platform}/)
        let scan_dir = self.ctx.project_root.join("target").join(&platform);

        // Scan output directory for generated archives
        let mut sdk_archive: Option<PathBuf> = None;
        let mut symbols_archive: Option<PathBuf> = None;
        let mut aar_archive: Option<PathBuf> = None;
        let mut architectures: Vec<String> = Vec::new();

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

                // Look for SDK archive: {lib}_{platform}_SDK-{version}.zip
                if file_name.contains("_SDK-") && file_name.ends_with(".zip") {
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
