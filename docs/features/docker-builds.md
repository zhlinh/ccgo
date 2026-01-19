# Docker Builds

Complete guide to building C++ libraries using Docker with CCGO for universal cross-platform compilation.

## Overview

CCGO's Docker builds enable **universal cross-platform compilation** - build libraries for any platform from any host operating system without installing platform-specific toolchains.

**Key features:**

- **Build anywhere**: Compile for Linux, Windows, macOS, iOS, watchOS, tvOS, Android on any OS
- **Zero dependencies**: No Xcode, Visual Studio, Android Studio, or SDK installations required
- **Prebuilt images**: Fast setup with images from Docker Hub (3-20x faster than manual builds)
- **Consistent environment**: Same toolchain versions across all developers
- **Isolated builds**: No conflicts with host system tools
- **Reproducible**: Guaranteed identical builds across different machines

## Why Use Docker Builds?

### Without Docker

**Limitations:**
- **Platform locked**: Need macOS for iOS/macOS, Windows for MSVC, Linux for Linux
- **Complex setup**: Install and configure multiple SDKs and toolchains
- **Version conflicts**: Different projects may need different toolchain versions
- **Storage overhead**: SDKs can consume 10-50GB per platform
- **Setup time**: Hours to install and configure all tools

### With Docker

**Benefits:**
- **Platform agnostic**: Build any platform from any OS
- **Quick setup**: Download prebuilt images (2-10 minutes)
- **Smaller footprint**: Images are 800MB-3.5GB vs 10-50GB SDKs
- **Instant switch**: Switch between toolchain versions easily
- **CI/CD ready**: Perfect for automated builds
- **Reproducible**: Same environment everywhere

## Supported Platforms

| Platform | Docker Image | Size | Toolchain | Host OS |
|----------|-------------|------|-----------|---------|
| Linux | `ccgo-builder-linux` | ~800MB | GCC, Clang, CMake | Any |
| Windows | `ccgo-builder-windows` | ~1.2GB | MinGW-w64, CMake | Any |
| macOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | Any |
| iOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | Any |
| watchOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | Any |
| tvOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | Any |
| Android | `ccgo-builder-android` | ~3.5GB | Android NDK, CMake | Any |

**Note**: Apple platforms (macOS, iOS, watchOS, tvOS) share the same image.

## Prerequisites

### Install Docker Desktop

**macOS:**
```bash
# Download from docker.com or use Homebrew
brew install --cask docker

# Start Docker Desktop
open -a Docker
```

**Windows:**
```bash
# Download Docker Desktop from docker.com
# Run installer and restart

# Verify
docker --version
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install docker.io docker-compose
sudo systemctl start docker
sudo systemctl enable docker

# Add user to docker group
sudo usermod -aG docker $USER
# Log out and back in

# Verify
docker --version
```

### System Requirements

- **Disk space**: 5-10GB for all Docker images
- **Memory**: 4GB+ RAM recommended
- **CPU**: Modern multi-core processor recommended
- **Network**: Fast internet for first-time image download

## Quick Start

### Basic Docker Builds

```bash
# Build for Linux from any OS
ccgo build linux --docker

# Build for Windows from macOS or Linux
ccgo build windows --docker

# Build for macOS from Windows or Linux
ccgo build macos --docker

# Build for iOS from Windows or Linux
ccgo build ios --docker

# Build for Android from any OS
ccgo build android --docker
```

### First Build (Image Download)

```bash
# First time: Downloads prebuilt image
$ ccgo build linux --docker

Pulling Docker image ccgo-builder-linux:latest...
latest: Pulling from ccgo/ccgo-builder-linux
Digest: sha256:abc123...
Status: Downloaded newer image for ccgo-builder-linux:latest

Building in Docker container...
[Build output...]
Build complete!
```

**Download times (first build only):**
- Linux: ~2-3 minutes (~800MB)
- Windows: ~3-4 minutes (~1.2GB)
- Apple: ~5-8 minutes (~2.5GB)
- Android: ~8-10 minutes (~3.5GB)

### Subsequent Builds

```bash
# All subsequent builds use cached image (instant startup)
$ ccgo build linux --docker

Using existing Docker image ccgo-builder-linux:latest...
Building in Docker container...
[Build output...]
Build complete!
```

## How Docker Builds Work

### Build Process

1. **Image Selection**: CCGO selects appropriate Docker image for target platform
2. **Image Download**: Pulls prebuilt image from Docker Hub (first time only)
3. **Container Launch**: Starts Docker container with mounted project directory
4. **Build Execution**: Runs build inside container using platform toolchain
5. **Output Collection**: Writes build artifacts to host filesystem
6. **Cleanup**: Container is removed (image remains cached)

### File System Mounting

```
Host                          Docker Container
====                          ================
/project/                 --> /workspace/
  ├── src/                    ├── src/
  ├── include/                ├── include/
  ├── CCGO.toml               ├── CCGO.toml
  └── target/             <-- └── target/
       └── linux/                  └── linux/
```

- **Project directory** mounted read-only
- **Output directory** mounted read-write
- **Build artifacts** written to host filesystem

### Toolchain Environments

**Linux container:**
```bash
# Ubuntu 22.04
gcc 11.4.0
clang 14.0.0
cmake 3.22.1
```

**Windows container:**
```bash
# Ubuntu + MinGW-w64
x86_64-w64-mingw32-gcc 10.0.0
cmake 3.22.1
```

**Apple container:**
```bash
# Ubuntu + OSXCross
clang 14.0.0 (LLVM)
OSXCross targeting macOS 10.13+
cmake 3.22.1
```

**Android container:**
```bash
# Ubuntu + Android NDK
Android NDK r25c
cmake 3.22.1
```

## Docker Image Management

### List Downloaded Images

```bash
# List CCGO Docker images
docker images | grep ccgo-builder

# Output:
# ccgo-builder-linux    latest    abc123    2 weeks ago    800MB
# ccgo-builder-windows  latest    def456    2 weeks ago    1.2GB
# ccgo-builder-apple    latest    ghi789    2 weeks ago    2.5GB
```

### Update Images

```bash
# Update all images to latest version
docker pull ccgo-builder-linux:latest
docker pull ccgo-builder-windows:latest
docker pull ccgo-builder-apple:latest
docker pull ccgo-builder-android:latest
```

### Remove Images

```bash
# Remove specific image
docker rmi ccgo-builder-linux:latest

# Remove all CCGO images
docker rmi $(docker images -q "ccgo-builder-*")

# Reclaim space
docker system prune -a
```

### Disk Space

```bash
# Check Docker disk usage
docker system df

# Output:
# TYPE            TOTAL     ACTIVE    SIZE      RECLAIMABLE
# Images          4         0         7.8GB     7.8GB (100%)
# Containers      0         0         0B        0B
# Local Volumes   0         0         0B        0B
```

## Advanced Usage

### Build Multiple Platforms

```bash
# Build for all mobile platforms
ccgo build android --docker
ccgo build ios --docker

# Build for all desktop platforms
ccgo build linux --docker
ccgo build windows --docker
ccgo build macos --docker
```

### Architecture Selection

```bash
# Android - multiple architectures
ccgo build android --docker --arch armeabi-v7a,arm64-v8a,x86_64

# iOS - build for device and simulator
ccgo build ios --docker --arch arm64,x86_64

# Windows - different architectures
ccgo build windows --docker --arch x86,x64,arm64
```

### Custom Docker Options

```bash
# Pass custom Docker run options
DOCKER_OPTS="--cpus=4 --memory=8g" ccgo build linux --docker

# Mount additional volumes
DOCKER_OPTS="-v /extra/libs:/libs:ro" ccgo build linux --docker
```

### Parallel Builds

```bash
# Build multiple platforms in parallel
ccgo build linux --docker &
ccgo build windows --docker &
ccgo build macos --docker &
wait

echo "All builds complete!"
```

## Platform-Specific Notes

### Linux Docker Builds

**Advantages:**
- Build on macOS or Windows
- Consistent glibc version
- Both GCC and Clang available

**Limitations:**
- Cannot run GUI applications
- Cannot test applications (no X11 display)

**Usage:**
```bash
# Build for x86_64
ccgo build linux --docker --arch x86_64

# Build for ARM64
ccgo build linux --docker --arch arm64

# Build with specific compiler
ccgo build linux --docker --compiler clang
```

### Windows Docker Builds

**Advantages:**
- Build on macOS or Linux
- No Visual Studio required

**Limitations:**
- MinGW only (no MSVC)
- Output may not be compatible with MSVC-built code
- Cannot run Windows applications

**Usage:**
```bash
# Build for x64
ccgo build windows --docker --arch x64

# Build for x86
ccgo build windows --docker --arch x86

# Build both architectures
ccgo build windows --docker --arch x86,x64
```

**MSVC compatibility note:**
- Docker builds use MinGW-w64 (GCC for Windows)
- For MSVC builds, use native Windows build
- MinGW and MSVC have different ABI (not binary compatible)

### Apple Docker Builds

**Advantages:**
- Build macOS/iOS/watchOS/tvOS on Windows or Linux
- No Xcode or macOS required
- Uses OSXCross (LLVM-based)

**Limitations:**
- Cannot code sign applications
- Cannot run or test applications
- Cannot notarize applications
- No Xcode project generation

**Usage:**
```bash
# Build for macOS
ccgo build macos --docker

# Build for iOS
ccgo build ios --docker

# Build for multiple Apple platforms
ccgo build macos --docker
ccgo build ios --docker
ccgo build watchos --docker
ccgo build tvos --docker
```

### Android Docker Builds

**Advantages:**
- Build on any OS
- No Android Studio required
- Consistent NDK version

**Limitations:**
- Cannot run on Android devices
- Cannot generate Android Studio projects
- Cannot run tests

**Usage:**
```bash
# Build for all Android architectures
ccgo build android --docker --arch armeabi-v7a,arm64-v8a,x86,x86_64

# Build AAR package
ccgo build android --docker --aar
```

## Troubleshooting

### Docker Not Found

```
Error: docker command not found
```

**Solution:**
```bash
# Install Docker Desktop and ensure it's running
docker --version

# macOS: Check if Docker Desktop is running
open -a Docker
```

### Permission Denied

```
Error: permission denied while trying to connect to the Docker daemon
```

**Solution (Linux):**
```bash
# Add user to docker group
sudo usermod -aG docker $USER

# Log out and back in
# Or run with sudo (not recommended)
sudo ccgo build linux --docker
```

### Image Pull Failed

```
Error: Error response from daemon: manifest for ccgo-builder-linux:latest not found
```

**Solution:**
```bash
# Check internet connection
ping docker.io

# Try manual pull
docker pull ccgo-builder-linux:latest

# Check Docker Hub status
# https://status.docker.com/
```

### Disk Space Issues

```
Error: no space left on device
```

**Solution:**
```bash
# Check disk space
docker system df

# Clean up unused images
docker system prune -a

# Remove specific images
docker rmi ccgo-builder-linux:latest
```

### Build Artifacts Not Found

```
Error: Build artifacts not found in target/
```

**Solution:**
```bash
# Check Docker mount permissions
ls -la target/

# Ensure target directory exists and is writable
mkdir -p target
chmod 755 target

# Check Docker logs
docker logs $(docker ps -lq)
```

### Slow Builds

```
Builds taking too long inside Docker
```

**Solution:**
```bash
# Allocate more resources to Docker
# Docker Desktop → Preferences → Resources
# Increase: CPUs (4+), Memory (8GB+), Disk space

# Use Docker native rather than virtualization
# (faster on Linux)

# Enable BuildKit for faster image pulls
export DOCKER_BUILDKIT=1
```

## Best Practices

### 1. Use Docker for CI/CD

Perfect for automated builds:

```yaml
# .github/workflows/build.yml
name: Build
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build all platforms
        run: |
          ccgo build linux --docker
          ccgo build windows --docker
          ccgo build macos --docker
```

### 2. Cache Docker Images

Don't delete images between builds:

```bash
# Keep images cached for fast builds
# Only update when needed
docker pull ccgo-builder-linux:latest
```

### 3. Use for Reproducible Builds

Same environment everywhere:

```bash
# Development
ccgo build linux --docker

# CI/CD
ccgo build linux --docker

# Result: Identical binaries
```

### 4. Combine with Native Builds

Use Docker for cross-platform, native for platform-specific:

```bash
# On macOS
ccgo build macos              # Native (code signing, testing)
ccgo build linux --docker     # Docker (cross-compile)
ccgo build windows --docker   # Docker (cross-compile)
```

### 5. Regular Image Updates

```bash
# Monthly: Update images
docker pull ccgo-builder-linux:latest
docker pull ccgo-builder-windows:latest
docker pull ccgo-builder-apple:latest
docker pull ccgo-builder-android:latest
```

### 6. Resource Allocation

```bash
# Allocate sufficient resources
# Docker Desktop → Preferences → Resources:
# - CPUs: 4-8 (more = faster parallel builds)
# - Memory: 8-16GB
# - Disk: 50GB+
```

## Performance Optimization

### Parallel Platform Builds

```bash
#!/bin/bash
# build-all.sh

echo "Building all platforms..."

ccgo build linux --docker &
PID_LINUX=$!

ccgo build windows --docker &
PID_WINDOWS=$!

ccgo build macos --docker &
PID_MACOS=$!

ccgo build android --docker &
PID_ANDROID=$!

wait $PID_LINUX $PID_WINDOWS $PID_MACOS $PID_ANDROID

echo "All builds complete!"
```

### Incremental Builds

Docker builds support incremental compilation:

```bash
# First build (clean)
ccgo build linux --docker

# Subsequent builds (incremental)
# Only changed files are recompiled
ccgo build linux --docker
```

### Resource Tuning

```bash
# Limit Docker resources for single build
DOCKER_OPTS="--cpus=2 --memory=4g" ccgo build linux --docker

# Maximum resources for faster builds
DOCKER_OPTS="--cpus=8 --memory=16g" ccgo build linux --docker
```

## Security Considerations

### 1. Image Verification

```bash
# Verify image signatures (if available)
docker trust inspect ccgo-builder-linux:latest

# Check image digest
docker images --digests | grep ccgo-builder
```

### 2. Network Isolation

```bash
# Build without network access (after image download)
DOCKER_OPTS="--network=none" ccgo build linux --docker
```

### 3. Read-Only Mounts

```bash
# Mount source code read-only (default in CCGO)
DOCKER_OPTS="-v $(pwd):/workspace:ro" ccgo build linux --docker
```

### 4. User Permissions

```bash
# Run as non-root user inside container (CCGO default)
# Output files owned by current user
ls -l target/linux/
```

## Comparison: Native vs Docker

| Aspect | Native Build | Docker Build |
|--------|-------------|-------------|
| **Setup time** | Hours (SDK install) | Minutes (image download) |
| **Disk space** | 10-50GB per platform | 800MB-3.5GB per platform |
| **Build speed** | Faster (native) | Slightly slower (~10%) |
| **Cross-platform** | Limited (need target OS) | Full (any target from any OS) |
| **Reproducibility** | Medium (version drift) | High (fixed environment) |
| **Code signing** | Yes | No (Apple platforms) |
| **Testing** | Yes | Limited/No |
| **CI/CD** | Complex | Simple |
| **Maintenance** | Manual updates | Automatic (pull images) |

**Recommendation:**
- **Development**: Native for target platform (better testing/debugging)
- **CI/CD**: Docker for all platforms (consistency, reproducibility)
- **Cross-compilation**: Docker always (only option without target OS)

## See Also

- [Build System](build-system.md)
- [Linux Platform](../platforms/linux.md)
- [Windows Platform](../platforms/windows.md)
- [macOS Platform](../platforms/macos.md)
- [iOS Platform](../platforms/ios.md)
- [Android Platform](../platforms/android.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
