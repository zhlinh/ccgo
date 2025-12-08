#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# build_docker.py
# ccgo
#
# Copyright 2024 zhlinh and ccgo Project Authors. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Software.

"""
Docker-based cross-platform build support.

This script enables building all platform libraries on any OS using Docker containers
with the appropriate toolchains.

Features:
- Build Linux libraries using Ubuntu + GCC/Clang
- Build Windows libraries using Ubuntu + MinGW-w64
- Build Apple platforms (macOS/iOS/watchOS/tvOS) using Ubuntu + OSXCross
- Build Android libraries using Ubuntu + Android SDK/NDK
- Automatic Docker image building and caching
- Volume mounting for source code and build artifacts
- Build artifact extraction from containers

Requirements:
- Docker Desktop installed and running
- Sufficient disk space for Docker images (~8GB total for all platforms)

Usage:
    python3 build_docker.py <platform> <project_dir> [build_args...]

    platform:    'linux', 'windows', 'macos', 'ios', 'watchos', 'tvos', 'android'
    project_dir: Path to project directory containing CCGO.toml
    build_args:  Optional arguments to pass to build script

Examples:
    # Build Linux library
    python3 build_docker.py linux /path/to/project

    # Build Windows library with MinGW
    python3 build_docker.py windows /path/to/project

    # Build macOS library (using OSXCross)
    python3 build_docker.py macos /path/to/project

    # Build iOS library
    python3 build_docker.py ios /path/to/project

    # Build Android library
    python3 build_docker.py android /path/to/project
"""

import os
import sys
import subprocess
import shutil
from pathlib import Path


# GitHub Container Registry (GHCR) organization/username for prebuilt images
# Change this to your GHCR username when publishing
GHCR_REPO = "ghcr.io/zhlinh"  # e.g., "ghcr.io/yourname"

# Platform configuration mapping
PLATFORM_CONFIG = {
    "linux": {
        "dockerfile": "Dockerfile.linux",
        "image_name": "ccgo-builder-linux",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-linux:latest",
        "build_script": "build_linux.py",
        "build_mode": "1",
        "size_estimate": "~800MB"
    },
    "windows": {
        "dockerfile": "Dockerfile.windows-mingw",
        "image_name": "ccgo-builder-windows-mingw",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-windows-mingw:latest",
        "build_script": "build_windows.py",
        "build_mode": "1",
        "size_estimate": "~1.2GB"
    },
    "windows-msvc": {
        "dockerfile": "Dockerfile.windows-msvc",
        "image_name": "ccgo-builder-windows-msvc",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-windows-msvc:latest",
        "build_script": "build_windows.py",
        "build_mode": "1",
        "size_estimate": "~1.5GB",
        "build_args": "--toolchain=msvc"
    },
    "macos": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_macos.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "ios": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_ios.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "watchos": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_watchos.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "tvos": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_tvos.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "android": {
        "dockerfile": "Dockerfile.android",
        "image_name": "ccgo-builder-android",
        "remote_image": f"{GHCR_REPO}/ccgo-builder-android:latest",
        "build_script": "build_android.py",
        "build_mode": "1",
        "native_only": True,  # For Android, we build native libs only
        "size_estimate": "~3.5GB"
    }
}


class DockerBuilder:
    """Docker-based cross-platform builder."""

    def __init__(self, platform: str, project_dir: str, dev_mode: bool = False, toolchain: str = "auto", link_type: str = "both"):
        """
        Initialize Docker builder.

        Args:
            platform: Target platform (linux, windows, macos, ios, watchos, tvos, android)
            project_dir: Absolute path to project directory
            dev_mode: Use local ccgo source instead of PyPI (for development)
            toolchain: For Windows, specify toolchain: msvc, gnu/mingw, or auto
            link_type: Library link type: static, shared, or both (default: both)
        """
        self.platform = platform.lower()
        self.project_dir = Path(project_dir).resolve()
        self.toolchain = toolchain.lower() if toolchain else "auto"
        self.link_type = link_type.lower() if link_type else "both"
        self.docker_dir = Path(__file__).parent.resolve()

        # Auto-detect dev mode if not explicitly specified
        if not dev_mode:
            # Check if local ccgo source is available
            ccgo_source = self._find_ccgo_source()
            if ccgo_source:
                print(f"ℹ Auto-detected local ccgo source at {ccgo_source}")
                print(f"  Using development mode (local source in Docker)")
                print(f"  To disable, use production PyPI version (requires v2.1.1+ with cmake files)")
                dev_mode = True

        self.dev_mode = dev_mode

        # Handle Windows toolchain selection
        if self.platform == "windows" and self.toolchain != "auto":
            if self.toolchain in ["msvc"]:
                # Use MSVC Docker configuration
                self.platform = "windows-msvc"
                print(f"ℹ Using MSVC toolchain for Windows build")
            elif self.toolchain in ["gnu", "mingw"]:
                # Use default MinGW configuration
                print(f"ℹ Using MinGW toolchain for Windows build")
            else:
                print(f"⚠ Unknown toolchain '{self.toolchain}', using default MinGW")

        # Validate platform
        if self.platform not in PLATFORM_CONFIG:
            # Filter out internal platform variants for error message
            supported = [p for p in PLATFORM_CONFIG.keys() if not p.startswith("windows-") or p == "windows"]
            supported_str = ", ".join(supported)
            raise ValueError(
                f"Unsupported platform: {platform}\n"
                f"Supported platforms: {supported_str}"
            )

        # Get platform configuration
        self.config = PLATFORM_CONFIG[self.platform]
        self.image_name = self.config["image_name"]
        self.dockerfile = self.config["dockerfile"]

        # Validate project directory
        if not self.project_dir.exists():
            raise FileNotFoundError(f"Project directory not found: {project_dir}")

        config_file = self.project_dir / "CCGO.toml"
        if not config_file.exists():
            raise FileNotFoundError(f"CCGO.toml not found in: {project_dir}")

    def check_docker(self):
        """Check if Docker is installed and running."""
        print("Checking Docker installation...")
        try:
            result = subprocess.run(
                ["docker", "--version"],
                capture_output=True,
                text=True,
                check=True
            )
            print(f"✓ {result.stdout.strip()}")
        except (subprocess.CalledProcessError, FileNotFoundError) as e:
            print("=" * 60)
            print("ERROR: Docker is not installed or not in PATH")
            print("=" * 60)
            print("Please install Docker Desktop from: https://www.docker.com/products/docker-desktop")
            print(f"Details: {e}")
            sys.exit(1)

        # Check if Docker daemon is running by trying to connect
        try:
            result = subprocess.run(
                ["docker", "info"],
                capture_output=True,
                text=True,
                timeout=30  # 30 second timeout for daemon connection
            )
            if result.returncode != 0:
                raise subprocess.CalledProcessError(result.returncode, "docker info", result.stderr)
            print("✓ Docker daemon is running")
        except subprocess.TimeoutExpired:
            print("=" * 60)
            print("ERROR: Docker daemon connection timed out")
            print("=" * 60)
            print("Docker appears to be starting up or unresponsive.")
            print("Please wait for Docker Desktop to fully start and try again.")
            sys.exit(1)
        except subprocess.CalledProcessError as e:
            print("=" * 60)
            print("ERROR: Docker daemon is not running")
            print("=" * 60)
            print("Docker CLI is installed, but the Docker service/daemon is not running.")
            print("")
            print("To fix this:")
            print("  1. Start Docker Desktop application")
            print("  2. Wait for Docker to fully initialize (check the whale icon in system tray)")
            print("  3. Run this command again")
            print("")
            if e.stderr:
                print(f"Details: {e.stderr.strip()}")
            sys.exit(1)

    def pull_prebuilt_image(self):
        """Try to pull prebuilt image from Docker Hub."""
        remote_image = self.config.get("remote_image")
        if not remote_image:
            return False

        print(f"\n=== Checking for prebuilt image on Docker Hub ===")
        print(f"Image: {remote_image}")
        print(f"Size: {self.config['size_estimate']}")

        try:
            # Try to pull the prebuilt image
            print("Pulling prebuilt image from Docker Hub...")
            print("(This is much faster than building from scratch)")
            result = subprocess.run(
                ["docker", "pull", remote_image],
                capture_output=True,
                text=True,
                timeout=600  # 10 minute timeout
            )

            if result.returncode == 0:
                # Tag the remote image with local name
                subprocess.run(
                    ["docker", "tag", remote_image, self.image_name],
                    check=True
                )
                print(f"✓ Successfully pulled prebuilt image: {remote_image}")
                print(f"✓ Tagged as: {self.image_name}")
                return True
            else:
                print(f"⚠ Could not pull prebuilt image from GHCR")
                print(f"  Reason: {result.stderr}")
                return False

        except subprocess.TimeoutExpired:
            print("⚠ Docker pull timeout (network too slow)")
            return False
        except Exception as e:
            print(f"⚠ Failed to pull prebuilt image: {e}")
            return False

    def build_image(self, use_prebuilt=True):
        """Build Docker image if not exists or outdated."""
        print(f"\n=== Preparing Docker image: {self.image_name} ===")
        print(f"Platform: {self.platform}")

        # Check if image already exists locally
        result = subprocess.run(
            ["docker", "images", "-q", self.image_name],
            capture_output=True,
            text=True
        )

        if result.stdout.strip():
            print(f"✓ Image {self.image_name} already exists locally (using cached image)")
            print(f"  To rebuild, run: docker rmi {self.image_name}")
            return

        # Try to pull prebuilt image first
        if use_prebuilt:
            if self.pull_prebuilt_image():
                return  # Successfully pulled prebuilt image

            print("\n⚠ Prebuilt image not available, building from Dockerfile...")
            print("  (This will take 5-30 minutes depending on platform)")

        # Build from Dockerfile
        print(f"\n=== Building Docker image from Dockerfile ===")
        print(f"Dockerfile: {self.dockerfile}")
        print(f"Estimated size: {self.config['size_estimate']}")
        print("Building... (grab a coffee ☕)")

        dockerfile_path = self.docker_dir / self.dockerfile
        if not dockerfile_path.exists():
            raise FileNotFoundError(f"Dockerfile not found: {dockerfile_path}")

        cmd = [
            "docker", "build",
            "-f", str(dockerfile_path),
            "-t", self.image_name,
            str(self.docker_dir)
        ]

        # Enable BuildKit for faster builds
        env = os.environ.copy()
        env["DOCKER_BUILDKIT"] = "1"

        subprocess.run(cmd, check=True, env=env)
        print(f"✓ Image {self.image_name} built successfully")

    def _get_ccgo_version(self):
        """
        Get the currently installed ccgo version.

        Returns:
            Version string (e.g., "0.1.0") or None if not found
        """
        try:
            # Try to get version from pip show
            result = subprocess.run(
                ["pip3", "show", "ccgo"],
                capture_output=True,
                text=True,
                timeout=5
            )
            if result.returncode == 0:
                for line in result.stdout.split('\n'):
                    if line.startswith('Version:'):
                        version = line.split(':', 1)[1].strip()
                        return version
        except Exception as e:
            print(f"⚠ Could not determine ccgo version: {e}")

        return None

    def _find_ccgo_source(self):
        """
        Find local ccgo source directory (repo root with setup.py or pyproject.toml).

        This is used in development mode to mount local ccgo source.

        Returns:
            Path to ccgo repo root, or None if not found
        """
        # Start from docker_dir and search upward for setup.py or pyproject.toml
        # docker_dir is .../ccgo/ccgo/dockers
        # We need to find .../ccgo (repo root with setup.py or pyproject.toml)
        current = self.docker_dir
        for _ in range(5):  # Search up to 5 levels
            parent = current.parent
            setup_py = parent / "setup.py"
            pyproject_toml = parent / "pyproject.toml"
            if setup_py.exists() or pyproject_toml.exists():
                # Verify it's the ccgo repo by checking for ccgo package
                ccgo_dir = parent / "ccgo"
                if ccgo_dir.exists() and ccgo_dir.is_dir():
                    return parent
            current = parent

        return None

    def _find_git_root(self):
        """
        Find git repository root by searching upward from project directory.

        The .git directory may be in the project directory itself or in a parent
        directory. This method searches upward to find the git root.

        Returns:
            Path to .git directory, or None if not found
        """
        current = self.project_dir
        for _ in range(10):  # Search up to 10 levels
            git_dir = current / ".git"
            if git_dir.exists():
                # .git can be a directory (normal repo) or a file (worktree/submodule)
                if git_dir.is_dir():
                    # Verify it's a valid git directory by checking for HEAD file
                    head_file = git_dir / "HEAD"
                    if head_file.exists():
                        return git_dir
                    # Empty .git directory (possibly created by Docker), skip it
                elif git_dir.is_file():
                    # Handle git worktree or submodule
                    # The file contains: "gitdir: /path/to/actual/.git/worktrees/xxx"
                    try:
                        with open(git_dir, 'r') as f:
                            content = f.read().strip()
                            if content.startswith('gitdir:'):
                                # Valid git worktree/submodule reference
                                return git_dir
                    except Exception:
                        pass
            # Move to parent directory
            parent = current.parent
            if parent == current:
                # Reached filesystem root
                break
            current = parent

        return None

    def run_build(self, build_args: list = None):
        """
        Run build inside Docker container using ccgo command.

        Args:
            build_args: Additional arguments to pass to build command
        """
        print(f"\n=== Running {self.platform} build in Docker container ===")
        print(f"Project directory: {self.project_dir}")

        # Clean target/{platform} directory before Docker build to avoid stale artifacts
        target_platform_dir = Path(self.project_dir) / "target" / self.platform
        if target_platform_dir.exists():
            shutil.rmtree(target_platform_dir)
            print(f"Cleaned up: {target_platform_dir}")

        # Determine installation method based on dev_mode
        docker_volumes = [
            "-v", f"{self.project_dir}:/workspace",  # Always mount project directory
        ]

        # Mount .git directory if found (for git info in build_info.json)
        # The .git may be in parent directory, so we search upward
        git_dir = self._find_git_root()
        if git_dir:
            # Mount .git to /workspace/.git so git commands work in container
            docker_volumes.extend(["-v", f"{git_dir}:/workspace/.git:ro"])
            print(f"Git repository: {git_dir.parent} (mounted .git to container)")
        else:
            print(f"⚠ No git repository found (git info will be 'unknown')")

        if self.dev_mode:
            # Development mode: Use local ccgo source
            ccgo_source = self._find_ccgo_source()
            if ccgo_source:
                print(f"Development mode: Using local ccgo source from {ccgo_source}")
                docker_volumes.extend(["-v", f"{ccgo_source}:/ccgo"])
                # Use non-editable install to avoid PEP 660 compatibility issues in containers
                # Upgrade pip first to support --break-system-packages if needed (for newer Ubuntu)
                install_cmd = "pip3 install -q --upgrade pip && pip3 install -q /ccgo"
            else:
                print("⚠ Development mode requested but ccgo source not found!")
                print("  Falling back to PyPI installation")
                self.dev_mode = False  # Fallback

        if not self.dev_mode:
            # Production mode: Install from PyPI with version matching
            # Upgrade pip first to ensure compatibility
            ccgo_version = self._get_ccgo_version()
            if ccgo_version:
                print(f"CCGO version: {ccgo_version} (will install same version in container)")
                install_cmd = f"pip3 install -q --upgrade pip && pip3 install -q ccgo=={ccgo_version}"
            else:
                print(f"CCGO installation: Latest from PyPI (version not detected)")
                install_cmd = f"pip3 install -q --upgrade pip && pip3 install -q ccgo"

        # Construct ccgo build command
        # Use python3 -m ccgo.main instead of ccgo command to avoid PATH issues in Docker
        link_type_arg = f" --link-type {self.link_type}" if self.link_type else ""

        # Map Docker platform to ccgo build target
        # windows-msvc and windows-mingw both map to "windows" build target
        build_target = self.platform
        if self.platform.startswith("windows-"):
            build_target = "windows"

        if self.platform == "android":
            # Android build in Docker: native-only with archive (no Gradle in Docker)
            ccgo_cmd = f"python3 -m ccgo.main build android --native-only --archive --arch armeabi-v7a,arm64-v8a,x86_64 --no-docker{link_type_arg}"
        else:
            # Standard build command
            # IMPORTANT: Add --no-docker to prevent recursive Docker calls inside container
            ccgo_cmd = f"python3 -m ccgo.main build {build_target} --no-docker{link_type_arg}"

        if build_args:
            ccgo_cmd += " " + " ".join(build_args)

        # Multi-stage build command:
        # 1. Install ccgo (from local source or PyPI)
        # 2. Run ccgo build command
        build_cmd = f"{install_cmd} && {ccgo_cmd}"

        # Docker run command with volume mounts
        docker_cmd = [
            "docker", "run",
            "--rm",  # Remove container after execution
        ]

        # Add environment variables for Windows toolchain selection
        if self.platform.startswith("windows"):
            if "msvc" in self.platform:
                docker_cmd.extend(["-e", "CCGO_WINDOWS_TOOLCHAIN=msvc"])
            elif self.toolchain != "auto":
                docker_cmd.extend(["-e", f"CCGO_WINDOWS_TOOLCHAIN={self.toolchain}"])

        docker_cmd.extend(docker_volumes)
        docker_cmd.extend([
            "-w", "/workspace",  # Set working directory
            self.image_name,
            build_cmd
        ])

        print(f"\nDocker command:")
        print(f"  docker run --rm \\")
        for i in range(0, len(docker_volumes), 2):
            print(f"    {docker_volumes[i]} {docker_volumes[i+1]} \\")
        print(f"    -w /workspace \\")
        print(f"    {self.image_name} \\")
        print(f"    '{build_cmd}'")
        print("-" * 60)

        # Run build in container
        try:
            result = subprocess.run(docker_cmd, capture_output=True, text=True)

            # Print output
            if result.stdout:
                print(result.stdout)
            if result.stderr:
                print(result.stderr, file=sys.stderr)

            print("-" * 60)

            if result.returncode != 0:
                print(f"ERROR: Build failed with exit code {result.returncode}")
                # Check if Docker daemon issue
                if "Cannot connect to the Docker daemon" in (result.stderr or ""):
                    print("")
                    print("=" * 60)
                    print("Docker daemon is not running!")
                    print("=" * 60)
                    print("Please start Docker Desktop and try again.")
                elif "docker: Error response from daemon" in (result.stderr or ""):
                    print("")
                    print("Docker container failed to start. Check the error above.")
                sys.exit(result.returncode)

            # Verify build artifacts exist
            target_output = self.project_dir / "target" / self.platform
            if not target_output.exists() or not any(target_output.iterdir()):
                print(f"⚠️  WARNING: Build reported success but no artifacts found in {target_output}")
                print("   This may indicate the build was skipped or failed silently.")
                # Don't exit with error here, just warn - artifacts might be in different location

            print(f"✓ {self.platform.capitalize()} build completed successfully")

        except subprocess.CalledProcessError as e:
            print("-" * 60)
            print(f"ERROR: Build failed with exit code {e.returncode}")
            sys.exit(e.returncode)
        except Exception as e:
            print("-" * 60)
            print(f"ERROR: Unexpected error during build: {e}")
            sys.exit(1)

    def print_results(self):
        """Print build results location."""
        print(f"\n=== Build Results ===")

        # Normalize platform name for Windows variants
        # Windows-msvc and windows should both use "Windows" for cmake_build
        platform_name = self.platform
        if platform_name.startswith("windows"):
            cmake_platform = "Windows"
            target_platform = "windows"
        else:
            cmake_platform = platform_name.capitalize()
            target_platform = platform_name

        build_output = self.project_dir / "cmake_build" / cmake_platform
        if build_output.exists():
            print(f"Build artifacts: {build_output}")
        else:
            print(f"WARNING: Build output directory not found: {build_output}")

        # Check target directory (new unified location for all platforms)
        target_output = self.project_dir / "target" / target_platform
        if target_output.exists():
            print(f"Packaged artifacts: {target_output}")
            # List files in target directory
            for item in sorted(target_output.iterdir()):
                if item.is_file():
                    size = item.stat().st_size / (1024 * 1024)
                    print(f"  {item.name} ({size:.2f} MB)")
                elif item.is_dir():
                    print(f"  {item.name}/")


def print_usage():
    """Print usage information."""
    print("Usage: python3 build_docker.py <platform> <project_dir> [options] [build_args...]")
    print("\nSupported platforms:")
    # Filter out internal platform variants for display
    for platform, config in PLATFORM_CONFIG.items():
        if not platform.startswith("windows-"):
            print(f"  {platform:12} - {config['size_estimate']:10} Docker image")
    print("\nOptions:")
    print("  --dev              Use local ccgo source (for development, not published to PyPI)")
    print("  --toolchain=<type> Windows toolchain: msvc, gnu/mingw, or auto (default: auto)")
    print("  --link-type=<type> Library link type: static, shared, or both (default: both)")
    print("\nExamples:")
    print("  python3 build_docker.py linux /path/to/project")
    print("  python3 build_docker.py linux /path/to/project --link-type=static")
    print("  python3 build_docker.py windows /path/to/project --dev")
    print("  python3 build_docker.py windows /path/to/project --toolchain=msvc")
    print("  python3 build_docker.py windows /path/to/project --toolchain=mingw")
    print("  python3 build_docker.py macos /path/to/project")
    print("  python3 build_docker.py ios /path/to/project")
    print("  python3 build_docker.py android /path/to/project")


def main():
    """Main entry point for Docker-based builds."""
    if len(sys.argv) < 3:
        print_usage()
        sys.exit(1)

    platform = sys.argv[1]
    project_dir = sys.argv[2]

    # Parse --dev flag, --toolchain flag, --link-type flag and remaining build args
    dev_mode = False
    toolchain = "auto"
    link_type = "both"
    build_args = []

    for arg in sys.argv[3:]:
        if arg == "--dev":
            dev_mode = True
        elif arg.startswith("--toolchain="):
            toolchain = arg.split("=", 1)[1]
        elif arg.startswith("--link-type="):
            link_type = arg.split("=", 1)[1]
        else:
            build_args.append(arg)

    if not build_args:
        build_args = None

    try:
        # Special handling for Windows with auto toolchain: build both MinGW and MSVC
        if platform.lower() == "windows" and toolchain == "auto":
            print("\n" + "=" * 60)
            print("Windows build with auto toolchain: Building both MinGW and MSVC")
            print("=" * 60)

            # Check Docker once
            temp_builder = DockerBuilder(platform, project_dir, dev_mode=dev_mode, toolchain="mingw", link_type=link_type)
            temp_builder.check_docker()

            # Build MinGW first (without archive - we'll archive at the end on host)
            print("\n" + "=" * 60)
            print("Step 1/2: Building with MinGW toolchain")
            print("=" * 60)
            mingw_builder = DockerBuilder(platform, project_dir, dev_mode=dev_mode, toolchain="mingw", link_type=link_type)
            mingw_builder.build_image()
            # Add --no-archive flag for MinGW build
            mingw_args = list(build_args) if build_args else []
            mingw_args.append("--no-archive")
            mingw_builder.run_build(mingw_args)

            # Build MSVC second (also without archive - we'll archive on host)
            print("\n" + "=" * 60)
            print("Step 2/2: Building with MSVC toolchain")
            print("=" * 60)
            msvc_builder = DockerBuilder(platform, project_dir, dev_mode=dev_mode, toolchain="msvc", link_type=link_type)
            msvc_builder.build_image()
            # MSVC build without archive
            msvc_args = list(build_args) if build_args else []
            msvc_args.append("--no-archive")
            msvc_builder.run_build(msvc_args)

            # Archive on host machine (after both Docker builds are complete)
            # This ensures we can see all build outputs from both toolchains
            print("\n" + "=" * 60)
            print("Archiving both toolchains on host...")
            print("=" * 60)
            try:
                # Add the ccgo build_scripts directory to Python path
                ccgo_build_scripts = Path(__file__).parent.parent / "build_scripts"
                if str(ccgo_build_scripts) not in sys.path:
                    sys.path.insert(0, str(ccgo_build_scripts))

                # Change to project directory and import build_windows
                original_cwd = os.getcwd()
                os.chdir(project_dir)

                # Import and call archive function
                from build_windows import archive_windows_project, print_build_results
                archive_windows_project(link_type=link_type, toolchain='auto')
                print_build_results(link_type=link_type)

                os.chdir(original_cwd)
            except Exception as e:
                print(f"WARNING: Archive on host failed: {e}")
                import traceback
                traceback.print_exc()

            print("\n" + "=" * 60)
            print(f"✓ Docker build for {platform} completed successfully!")
            print("  - MinGW libraries: lib/static/mingw/, lib/shared/mingw/")
            print("  - MSVC libraries:  lib/static/msvc/, lib/shared/msvc/")
            print("=" * 60)
        else:
            # Standard single-toolchain build
            builder = DockerBuilder(platform, project_dir, dev_mode=dev_mode, toolchain=toolchain, link_type=link_type)
            builder.check_docker()
            builder.build_image()
            builder.run_build(build_args)
            builder.print_results()

            print("\n" + "=" * 60)
            print(f"✓ Docker build for {platform} completed successfully!")
            print("=" * 60)

    except Exception as e:
        print(f"\nERROR: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
