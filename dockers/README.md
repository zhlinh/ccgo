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

## Prebuilt Images

Prebuilt Docker images are available from GitHub Container Registry (GHCR):
- `ghcr.io/zhlinh/ccgo-builder-linux:latest`
- `ghcr.io/zhlinh/ccgo-builder-windows-mingw:latest`
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

## Notes

- Docker builds use the same `ccgo build` commands internally
- Git repository is mounted read-only for version info
- All build artifacts are written to the host filesystem
- Containers are removed after build completes (`--rm` flag)
