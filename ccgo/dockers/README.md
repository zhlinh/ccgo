# Docker-based Cross-Platform Builds

This directory contains Docker configurations and scripts for cross-platform C++ library builds. This enables building **all platform libraries on any OS** - completely eliminating the need for platform-specific development environments.

## Overview

The Docker build system provides **universal cross-platform compilation**:
- **Linux builds**: Ubuntu 22.04 + GCC/Clang toolchain
- **Windows builds**: Ubuntu 22.04 + MinGW-w64 cross-compiler
- **Apple platforms** (macOS/iOS/watchOS/tvOS): Ubuntu + OSXCross toolchain
- **Android builds**: Ubuntu + Android SDK/NDK + Gradle
- **Consistent environments**: Same build results across different host platforms
- **Zero local dependencies**: No need to install Xcode, Visual Studio, Android Studio, etc.
- **Easy setup**: Only Docker Desktop required

## Quick Start

### Prerequisites

1. **Install Docker Desktop** (~5 minutes)
   - macOS: Download from [Docker Desktop for Mac](https://www.docker.com/products/docker-desktop)
   - Linux: Install via package manager
   - Windows: Download from [Docker Desktop for Windows](https://www.docker.com/products/docker-desktop)

2. **Ensure Docker is running**
   ```bash
   docker --version
   docker ps
   ```

### ⚡ Prebuilt Images

CCGO automatically uses **prebuilt images from Docker Hub**:

- **No manual building required** - images are downloaded automatically
- **3-20x faster** than building from Dockerfile (2-10 min vs 5-30 min)
- **Zero configuration** - works out of the box

First build will download prebuilt images (~2-10 minutes), subsequent builds are instant!

### Build Commands

All platforms can be built on **any OS** (macOS, Windows, Linux) with the same commands:

```bash
# Navigate to your project directory
cd /path/to/your/project

# Build Linux library
ccgo build linux --docker

# Build Windows library
ccgo build windows --docker

# Build macOS library (on any OS!)
ccgo build macos --docker

# Build iOS library
ccgo build ios --docker

# Build watchOS library
ccgo build watchos --docker

# Build tvOS library
ccgo build tvos --docker

# Build Android library (native .so files)
ccgo build android --docker
```

**Example**: You can now build iOS apps on Windows, or Android apps on macOS, all using Docker!

## How It Works

### Architecture

```
┌─────────────┐
│   macOS     │
│  (Host OS)  │
└──────┬──────┘
       │
       │ Docker
       ▼
┌─────────────────────────────────┐
│   Docker Container (Ubuntu)     │
│                                 │
│  ┌──────────────────────────┐  │
│  │  Build Toolchain         │  │
│  │  - GCC/Clang (Linux)     │  │
│  │  - MinGW-w64 (Windows)   │  │
│  │  - CMake, Make, etc.     │  │
│  └──────────────────────────┘  │
│                                 │
│  ┌──────────────────────────┐  │
│  │  Build Process           │  │
│  │  1. Mount source code    │  │
│  │  2. Run build scripts    │  │
│  │  3. Generate artifacts   │  │
│  └──────────────────────────┘  │
└─────────────────────────────────┘
       │
       │ Volume mount
       ▼
┌─────────────────────────────────┐
│   Build Artifacts (Host)        │
│   - cmake_build/                │
│   - target/linux/ or target/windows/  │
└─────────────────────────────────┘
```

### Build Flow

1. **Check Docker**: Verifies Docker is installed and running
2. **Build Image**: Creates Docker image with toolchain (cached after first build)
3. **Mount Volumes**: Mounts project directory and ccgo package into container
4. **Run Build**: Executes platform-specific build script inside container
5. **Extract Artifacts**: Build outputs are written to host filesystem via volume mount

## Docker Images

### Linux Builder (`ccgo-builder-linux`)

- Base: Ubuntu 22.04 LTS
- Toolchain: GCC, G++, Make, CMake
- Output: `.a` static libraries
- Size: ~800MB
- Build time: ~5 minutes

### Windows Builder (`ccgo-builder-windows`)

- Base: Ubuntu 22.04 LTS
- Toolchain: MinGW-w64 (x86_64-w64-mingw32)
- Output: `.lib` static libraries (MinGW-compatible)
- Size: ~1.2GB
- Build time: ~8 minutes

### Apple Platforms Builder (`ccgo-builder-apple`)

- Base: Ubuntu 22.04 LTS
- Toolchain: OSXCross (Clang/LLVM for Apple platforms)
- SDK: macOS 13.3 SDK
- Platforms: macOS, iOS, watchOS, tvOS
- Output: `.a` static libraries, `.framework` bundles
- Size: ~2.5GB
- Build time: ~15-20 minutes (downloads Xcode SDK)

### Android Builder (`ccgo-builder-android`)

- Base: Ubuntu 22.04 LTS
- Toolchain: Android NDK 25.2, Gradle 8.5, OpenJDK 17
- SDK: Android 14 (API 34)
- Architectures: armeabi-v7a, arm64-v8a, x86_64
- Output: `.so` native libraries
- Size: ~3.5GB
- Build time: ~20-25 minutes (downloads Android SDK/NDK)

## Advanced Usage

### Direct Script Invocation

You can also use the Docker build script directly:

```bash
# Build any platform
python3 /path/to/ccgo/dockers/build_docker.py <platform> /path/to/project

# Examples
python3 /path/to/ccgo/dockers/build_docker.py linux /path/to/project
python3 /path/to/ccgo/dockers/build_docker.py windows /path/to/project
python3 /path/to/ccgo/dockers/build_docker.py macos /path/to/project
python3 /path/to/ccgo/dockers/build_docker.py ios /path/to/project
python3 /path/to/ccgo/dockers/build_docker.py android /path/to/project
```

### Rebuild Docker Images

If you need to rebuild the Docker images (e.g., after updating Dockerfiles):

```bash
# Remove specific image
docker rmi ccgo-builder-linux
docker rmi ccgo-builder-windows
docker rmi ccgo-builder-apple
docker rmi ccgo-builder-android

# Next build will recreate images
ccgo build <platform> --docker
```

### View Docker Images

```bash
# List all CCGO Docker images
docker images | grep ccgo-builder

# Check total disk space used
docker system df

# Remove all CCGO images
docker rmi $(docker images -q ccgo-builder*)
```

## Troubleshooting

### Docker not found

**Error**: `Docker is not installed or not running`

**Solution**: Install Docker Desktop and ensure it's running

```bash
# Check Docker installation
docker --version

# Start Docker daemon (macOS)
open -a Docker
```

### Permission denied

**Error**: `permission denied while trying to connect to the Docker daemon`

**Solution**: Ensure your user is in the docker group (Linux) or Docker Desktop is running (macOS/Windows)

```bash
# Linux: Add user to docker group
sudo usermod -aG docker $USER
# Then log out and log back in
```

### Build fails inside container

**Error**: Build script fails during Docker execution

**Solution**: Check build logs and verify project configuration

```bash
# Run with verbose output
ccgo build linux --docker

# Check container logs
docker logs $(docker ps -a | grep ccgo-builder | head -1 | awk '{print $1}')
```

### Volume mount issues

**Error**: Files not accessible in container

**Solution**: Ensure project path is under a Docker-shared directory

- **macOS**: Docker Desktop > Preferences > Resources > File Sharing
- **Windows**: Docker Desktop > Settings > Resources > File Sharing

### Disk space issues

**Error**: `no space left on device`

**Solution**: Clean up Docker resources

```bash
# Remove unused containers
docker container prune

# Remove unused images
docker image prune -a

# Remove all unused data
docker system prune -a --volumes
```

## Comparison: Docker vs Native Builds

| Aspect | Docker Build | Native Build |
|--------|-------------|--------------|
| **Setup** | Docker Desktop only (~4GB) | Platform-specific toolchains |
| **Speed** | Slower (first build: 10-30min) | Faster (incremental: seconds) |
| **Consistency** | Identical across all OS | Varies by environment |
| **Disk Space** | ~8GB for all images | Varies (Xcode: 40GB, VS: 8GB, Android Studio: 12GB) |
| **Cross-platform** | Build any platform on any OS | Need specific OS for each platform |
| **Use Case** | CI/CD, Release builds, No local tools | Active development, Debugging |
| **Maintenance** | Docker updates only | Multiple toolchain updates |

## Limitations

### Windows Docker Build (MinGW)

The Windows Docker build uses MinGW-w64, which produces libraries compatible with GCC/MinGW but may have compatibility issues with MSVC-compiled code:

- **MinGW output**: Works with GCC/MinGW applications
- **MSVC output**: Requires Visual Studio (native Windows build)

If you need MSVC-compatible libraries (for Visual Studio C++ projects), use:
1. Build natively on Windows with Visual Studio
2. Use GitHub Actions with Windows runners
3. Use a Windows VM with Visual Studio

### Apple Platforms Docker Build (OSXCross)

The Apple platforms Docker build uses OSXCross for cross-compilation:

**Advantages:**
- Build macOS/iOS/watchOS/tvOS on any OS
- No need for expensive macOS hardware
- Consistent toolchain across CI/CD

**Limitations:**
- Cannot run/test builds (no macOS runtime)
- Some platform-specific features may not work
- Requires publicly available macOS SDK
- Not officially supported by Apple (use at your own risk)

**Best for:** Library development, CI/CD pipelines, cross-compilation projects

### Android Docker Build

The Android Docker build provides native library (.so) compilation only:

- **What it does**: Compiles C++ code to Android native libraries
- **What it doesn't do**: Full APK/AAR packaging with Gradle (can be done separately)
- **Architectures**: armeabi-v7a, arm64-v8a, x86_64

### Performance

Docker builds may be slower than native builds due to:
- Container startup overhead
- Volume mount I/O overhead
- Limited CPU/memory allocation

For frequent rebuilds during development, consider:
- Using incremental builds
- Building natively when possible
- Using Docker only for release builds

## CI/CD Integration

Docker builds are perfect for CI/CD pipelines. Example GitHub Actions workflow:

```yaml
name: Cross-Platform Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        platform: [linux, windows]

    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.10'

      - name: Install ccgo
        run: pip install ccgo

      - name: Build ${{ matrix.platform }}
        run: ccgo build ${{ matrix.platform }} --docker

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.platform }}-build
          path: bin/${{ matrix.platform }}/
```

## Contributing

To improve Docker build support:

1. **Update Dockerfiles**: Modify `Dockerfile.linux`, `Dockerfile.windows-mingw`, or `Dockerfile.windows-msvc`
2. **Test changes**: Rebuild images and test builds
3. **Document changes**: Update this README
4. **Submit PR**: Contribute back to the project

## See Also

- [CCGO Documentation](../../README.md)
- [Build Command Reference](../commands/build.py)
- [Docker Documentation](https://docs.docker.com/)
- [MinGW-w64 Documentation](https://www.mingw-w64.org/)
