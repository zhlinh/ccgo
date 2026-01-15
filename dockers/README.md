# Docker Build Support

This directory contains Dockerfiles for cross-platform builds using Docker containers.

## Supported Platforms

- **Linux** (`Dockerfile.linux`) - Ubuntu + GCC/Clang
- **Windows MinGW** (`Dockerfile.windows-mingw`) - Ubuntu + MinGW-w64 cross-compiler
- **Windows MSVC** (`Dockerfile.windows-msvc`) - Windows Server Core + Visual Studio Build Tools
- **Apple Platforms** (`Dockerfile.apple`) - Ubuntu + OSXCross for macOS/iOS/tvOS/watchOS
- **Android** (`Dockerfile.android`) - Ubuntu + Android SDK/NDK

## Usage

Build any platform from any OS using Docker:

```bash
# Build for Linux (from macOS/Windows)
ccgo build linux --docker

# Build for Windows (from macOS/Linux)
ccgo build windows --docker

# Build for macOS (from Linux/Windows)
ccgo build macos --docker

# Build for iOS (from Linux/Windows)
ccgo build ios --docker

# Build for Android (from macOS/Windows/Linux)
ccgo build android --docker
```

## CCGO Version in Docker Images

**IMPORTANT**: All Docker images use the **Rust-based ccgo** from PyPI (3.x+ versions).

The `pip3 install ccgo` command installs a **pre-built Rust binary** packaged as a Python wheel. This is NOT the old Python-based ccgo - PyPI ccgo 3.x+ contains only Rust binaries with zero Python runtime dependency.

- **No Python runtime overhead**: The binary runs natively
- **Full Rust feature support**: Including MSVC toolchain, modern CMake integration, etc.
- **Fast installation**: Pre-compiled binaries download in seconds

## Prebuilt Images

Prebuilt Docker images are available from GitHub Container Registry (GHCR):
- `ghcr.io/zhlinh/ccgo-builder-linux:latest`
- `ghcr.io/zhlinh/ccgo-builder-windows-mingw:latest`
- `ghcr.io/zhlinh/ccgo-builder-windows-msvc:latest`
- `ghcr.io/zhlinh/ccgo-builder-apple:latest`
- `ghcr.io/zhlinh/ccgo-builder-android:latest`

The first Docker build will pull these prebuilt images (3-20x faster than building from Dockerfile).

## Image Sizes

- Linux: ~800MB
- Windows MinGW: ~1.2GB
- Apple (macOS/iOS/tvOS/watchOS): ~2.5GB
- Android: ~3.5GB

## Requirements

- Docker Desktop installed and running
- Internet connection (for first-time image pull)

## How It Works

1. **Check Docker** - Verifies Docker is installed and daemon is running
2. **Pull/Build Image** - Pulls prebuilt image from GHCR, or builds from Dockerfile if unavailable
3. **Mount Project** - Mounts project directory and .git into container
4. **Run Build** - Installs ccgo in container and runs platform-specific build
5. **Output** - Build artifacts are written to host's `target/` directory

## Environment Variables

- `CCGO_DOCKER_DIR` - Override Dockerfiles directory location

## CMake Toolchain Files

Some Docker images require CMake toolchain files for cross-compilation.

### Automated Toolchain File Management

CMake toolchain files are automatically managed through the following process:

1. **Source of Truth**: `cmake/windows-msvc.toolchain.cmake` (single file to maintain)
2. **Compile-time Embedding**: File is embedded into Rust binary via `include_str!` macro
3. **Runtime Extraction**: Automatically extracted to `~/.ccgo/dockers/cmake/` when building Docker images
4. **Docker Build**: Copied from cache to Docker image at `/opt/ccgo/windows-msvc.toolchain.cmake`

**Benefits**:
- ✅ Only one file to maintain (in `cmake/` directory)
- ✅ Toolchain files always stay in sync with the binary version
- ✅ No manual copying or syncing required
- ✅ Toolchain files are versioned with the code

**Implementation**: See `src/build/docker.rs` lines 28 and 177-186

**Verification**:
```bash
# Source file
md5sum cmake/windows-msvc.toolchain.cmake

# Extracted file (after running ccgo build --docker)
md5sum ~/.ccgo/dockers/cmake/windows-msvc.toolchain.cmake

# File in Docker image
docker run --rm ccgo-builder-windows-msvc "md5sum /opt/ccgo/windows-msvc.toolchain.cmake"

# All three should have the same MD5: 22fe278c82d8efaaafefe101b583799f
```

## Notes

- Docker builds use the same `ccgo build` commands internally
- Git repository is mounted read-only for version info
- All build artifacts are written to the host filesystem
- Containers are removed after build completes (`--rm` flag)
