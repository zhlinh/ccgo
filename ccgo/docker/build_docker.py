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


# Docker Hub organization/username for prebuilt images
# Change this to your Docker Hub username when publishing
DOCKER_HUB_REPO = "ccgo"  # e.g., "yourname/ccgo" or just "yourname" if image is "yourname/ccgo-builder-linux"

# Platform configuration mapping
PLATFORM_CONFIG = {
    "linux": {
        "dockerfile": "Dockerfile.linux",
        "image_name": "ccgo-builder-linux",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-linux:latest",
        "build_script": "build_linux.py",
        "build_mode": "1",
        "size_estimate": "~800MB"
    },
    "windows": {
        "dockerfile": "Dockerfile.windows",
        "image_name": "ccgo-builder-windows",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-windows:latest",
        "build_script": "build_windows.py",
        "build_mode": "1",
        "size_estimate": "~1.2GB"
    },
    "macos": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_macos.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "ios": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_ios.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "watchos": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_watchos.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "tvos": {
        "dockerfile": "Dockerfile.apple",
        "image_name": "ccgo-builder-apple",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-apple:latest",
        "build_script": "build_tvos.py",
        "build_mode": "1",
        "size_estimate": "~2.5GB"
    },
    "android": {
        "dockerfile": "Dockerfile.android",
        "image_name": "ccgo-builder-android",
        "remote_image": f"{DOCKER_HUB_REPO}/ccgo-builder-android:latest",
        "build_script": "build_android.py",
        "build_mode": "1",
        "native_only": True,  # For Android, we build native libs only
        "size_estimate": "~3.5GB"
    }
}


class DockerBuilder:
    """Docker-based cross-platform builder."""

    def __init__(self, platform: str, project_dir: str, dev_mode: bool = False):
        """
        Initialize Docker builder.

        Args:
            platform: Target platform (linux, windows, macos, ios, watchos, tvos, android)
            project_dir: Absolute path to project directory
            dev_mode: Use local ccgo source instead of PyPI (for development)
        """
        self.platform = platform.lower()
        self.project_dir = Path(project_dir).resolve()
        self.docker_dir = Path(__file__).parent.resolve()
        self.dev_mode = dev_mode

        # Validate platform
        if self.platform not in PLATFORM_CONFIG:
            supported = ", ".join(PLATFORM_CONFIG.keys())
            raise ValueError(
                f"Unsupported platform: {platform}\n"
                f"Supported platforms: {supported}"
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
        except (subprocess.CalledProcessError, FileNotFoundError):
            print("ERROR: Docker is not installed or not running")
            print("Please install Docker Desktop from: https://www.docker.com/products/docker-desktop")
            sys.exit(1)

        # Check if Docker daemon is running
        try:
            subprocess.run(
                ["docker", "ps"],
                capture_output=True,
                check=True
            )
            print("✓ Docker daemon is running")
        except subprocess.CalledProcessError:
            print("ERROR: Docker daemon is not running")
            print("Please start Docker Desktop and try again")
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
                print(f"⚠ Could not pull prebuilt image from Docker Hub")
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
        Find local ccgo source directory (repo root with setup.py).

        This is used in development mode to mount local ccgo source.

        Returns:
            Path to ccgo repo root, or None if not found
        """
        # Start from docker_dir and search upward for setup.py
        # docker_dir is .../ccgo/ccgo/docker
        # We need to find .../ccgo (repo root with setup.py)
        current = self.docker_dir
        for _ in range(5):  # Search up to 5 levels
            parent = current.parent
            setup_py = parent / "setup.py"
            if setup_py.exists():
                # Verify it's the ccgo repo by checking for ccgo package
                ccgo_dir = parent / "ccgo"
                if ccgo_dir.exists() and ccgo_dir.is_dir():
                    return parent
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

        # Determine installation method based on dev_mode
        docker_volumes = [
            "-v", f"{self.project_dir}:/workspace",  # Always mount project directory
        ]

        if self.dev_mode:
            # Development mode: Use local ccgo source
            ccgo_source = self._find_ccgo_source()
            if ccgo_source:
                print(f"Development mode: Using local ccgo source from {ccgo_source}")
                docker_volumes.extend(["-v", f"{ccgo_source}:/ccgo"])
                install_cmd = "pip3 install -q -e /ccgo"
            else:
                print("⚠ Development mode requested but ccgo source not found!")
                print("  Falling back to PyPI installation")
                self.dev_mode = False  # Fallback

        if not self.dev_mode:
            # Production mode: Install from PyPI with version matching
            ccgo_version = self._get_ccgo_version()
            if ccgo_version:
                print(f"CCGO version: {ccgo_version} (will install same version in container)")
                install_cmd = f"pip3 install -q ccgo=={ccgo_version}"
            else:
                print(f"CCGO installation: Latest from PyPI (version not detected)")
                install_cmd = f"pip3 install -q ccgo"

        # Construct ccgo build command
        if self.platform == "android" and self.config.get("native_only"):
            # Android native-only build
            ccgo_cmd = f"ccgo build android --native-only --arch armeabi-v7a,arm64-v8a,x86_64"
        else:
            # Standard build command
            ccgo_cmd = f"ccgo build {self.platform}"

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
            subprocess.run(docker_cmd, check=True)
            print("-" * 60)
            print(f"✓ {self.platform.capitalize()} build completed successfully")
        except subprocess.CalledProcessError as e:
            print("-" * 60)
            print(f"ERROR: Build failed with exit code {e.returncode}")
            sys.exit(e.returncode)

    def print_results(self):
        """Print build results location."""
        print(f"\n=== Build Results ===")
        build_output = self.project_dir / "cmake_build" / self.platform.capitalize()
        if build_output.exists():
            print(f"Build artifacts: {build_output}")
        else:
            print(f"WARNING: Build output directory not found: {build_output}")

        bin_output = self.project_dir / "bin" / self.platform
        if bin_output.exists():
            print(f"Packaged artifacts: {bin_output}")
            # List files in bin directory
            for item in sorted(bin_output.iterdir()):
                if item.is_file():
                    size = item.stat().st_size / (1024 * 1024)
                    print(f"  {item.name} ({size:.2f} MB)")
                elif item.is_dir():
                    print(f"  {item.name}/")


def print_usage():
    """Print usage information."""
    print("Usage: python3 build_docker.py <platform> <project_dir> [--dev] [build_args...]")
    print("\nSupported platforms:")
    for platform, config in PLATFORM_CONFIG.items():
        print(f"  {platform:12} - {config['size_estimate']:10} Docker image")
    print("\nOptions:")
    print("  --dev        Use local ccgo source (for development, not published to PyPI)")
    print("\nExamples:")
    print("  python3 build_docker.py linux /path/to/project")
    print("  python3 build_docker.py windows /path/to/project --dev")
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

    # Parse --dev flag and remaining build args
    dev_mode = False
    build_args = []

    for arg in sys.argv[3:]:
        if arg == "--dev":
            dev_mode = True
        else:
            build_args.append(arg)

    if not build_args:
        build_args = None

    try:
        builder = DockerBuilder(platform, project_dir, dev_mode=dev_mode)
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
