# Platform Support

CCGO provides comprehensive cross-platform build support for C++ projects. This section covers platform-specific guides, requirements, and best practices.

## Supported Platforms

### Mobile Platforms

| Platform | Architectures | Output Formats | Status |
|----------|--------------|----------------|--------|
| [Android](android.md) | arm64-v8a, armeabi-v7a, x86, x86_64 | AAR, .so, .a | ✅ Stable |
| [iOS](ios.md) | armv7, arm64, simulator (x86_64, arm64) | Framework, XCFramework | ✅ Stable |
| [OpenHarmony](openharmony.md) | arm64-v8a, armeabi-v7a, x86_64 | HAR, .so, .a | ✅ Stable |

### Desktop Platforms

| Platform | Architectures | Output Formats | Status |
|----------|--------------|----------------|--------|
| [macOS](macos.md) | x86_64, arm64 (Apple Silicon) | Framework, XCFramework, dylib | ✅ Stable |
| [Windows](windows.md) | x86, x86_64 | DLL, LIB (MSVC/MinGW) | ✅ Stable |
| [Linux](linux.md) | x86_64, aarch64 | .so, .a | ✅ Stable |

### TV and Wearable Platforms

| Platform | Architectures | Output Formats | Status |
|----------|--------------|----------------|--------|
| watchOS | armv7k, arm64_32, simulator | Framework, XCFramework | ✅ Stable |
| tvOS | arm64, simulator (x86_64, arm64) | Framework, XCFramework | ✅ Stable |

### Multi-Platform

| Platform | Description | Status |
|----------|-------------|--------|
| Kotlin Multiplatform | KMP library with native C++ | 🚧 Coming Soon |

## Quick Start

### Basic Build

```bash
# Build for your current platform
ccgo build

# Build for specific platform
ccgo build android --arch arm64-v8a
ccgo build ios
ccgo build windows --toolchain msvc
```

### Docker-Based Builds

Build any platform on any host OS:

```bash
# Build Linux libraries on macOS/Windows
ccgo build linux --docker

# Build Windows libraries on Linux/macOS
ccgo build windows --docker

# Build macOS/iOS libraries on Linux/Windows (experimental)
ccgo build macos --docker
```

## Platform Selection Guide

### For Mobile Apps

- **Android**: Use AAR for easy integration with Android Studio/Gradle
- **iOS**: Use XCFramework for both device and simulator support
- **OpenHarmony**: Use HAR for DevEco Studio integration

### For Desktop Apps

- **Windows**: MSVC for Visual Studio projects, MinGW for GCC compatibility
- **macOS**: Framework for Xcode projects, dylib for general use
- **Linux**: Shared libraries (.so) for most applications

### For Multi-Platform

- **Kotlin Multiplatform**: Unified API across Android, iOS, macOS, Linux, Windows

## Build Options

### Architecture Selection

```bash
# Single architecture
ccgo build android --arch arm64-v8a

# Multiple architectures
ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64

# All architectures (default)
ccgo build android
```

### Link Type

```bash
# Static libraries only
ccgo build --link-type static

# Shared libraries only
ccgo build --link-type shared

# Both (default)
ccgo build --link-type both
```

### Toolchain Selection (Windows)

```bash
# MSVC (default on Windows)
ccgo build windows --toolchain msvc

# MinGW
ccgo build windows --toolchain mingw

# Both
ccgo build windows --toolchain auto
```

## Platform Requirements

### Development Prerequisites

| Platform | Requirements |
|----------|-------------|
| Android | Android SDK/NDK or Docker |
| iOS | macOS with Xcode or Docker (experimental) |
| macOS | macOS with Xcode or Docker (experimental) |
| Windows | Visual Studio or MinGW or Docker |
| Linux | GCC/Clang or Docker |
| OpenHarmony | OpenHarmony SDK or Docker |
| watchOS/tvOS | macOS with Xcode |

### Docker Requirements

All platforms can be built using Docker with zero local toolchain setup:

- Install [Docker Desktop](https://www.docker.com/products/docker-desktop)
- Run `ccgo build <platform> --docker`
- First build downloads pre-built images (~2-10 minutes)
- Subsequent builds use cached images (instant startup)

## Platform-Specific Guides

- [Android Development](android.md) - AAR packaging, JNI, Gradle integration
- [iOS Development](ios.md) - Framework/XCFramework, Swift interop
- [macOS Development](macos.md) - Universal binaries, code signing
- [Windows Development](windows.md) - MSVC vs MinGW, DLL export
- [Linux Development](linux.md) - System libraries, packaging
- [OpenHarmony Development](openharmony.md) - HAR packaging, ArkTS interop

## Common Tasks

### Publishing

```bash
# Publish to Maven (Android/OpenHarmony)
ccgo publish android --registry official

# Publish to CocoaPods (iOS/macOS)
ccgo publish apple --manager cocoapods

# Publish to Swift Package Manager
ccgo publish apple --manager spm --push

# Publish to Conan (All platforms)
ccgo publish conan --registry official
```

### IDE Projects

```bash
# Generate Android Studio project
ccgo build android --ide-project

# Generate Xcode project
ccgo build ios --ide-project

# Generate Visual Studio project
ccgo build windows --ide-project --toolchain msvc
```

### Checking Platform Support

```bash
# Check if platform requirements are met
ccgo check android
ccgo check ios --verbose

# Check all platforms
ccgo check --all
```

## Platform-Specific Configuration

Each platform can be configured in `CCGO.toml`:

```toml
[android]
min_sdk_version = 21
target_sdk_version = 33
ndk_version = "25.2.9519653"

[ios]
min_deployment_target = "12.0"
enable_bitcode = false

[windows]
msvc_runtime = "dynamic"  # or "static"
```

See [CCGO.toml Reference](../reference/ccgo-toml.md) for complete options.

## Troubleshooting

### Build Failures

1. Check platform requirements: `ccgo check <platform>`
2. Try Docker build: `ccgo build <platform> --docker`
3. Enable verbose logging: `ccgo build <platform> --verbose`

### Docker Issues

1. Ensure Docker is running: `docker ps`
2. Clear Docker cache: `docker system prune`
3. Re-pull images: `docker pull ccgo-builder-<platform>`

### Platform-Specific Issues

See individual platform guides for detailed troubleshooting.

## Next Steps

- Choose your target platform guide above
- Review [Build System](../features/build-system.md) documentation
- Explore [Publishing Options](../features/publishing.md)
- Check [Docker Builds](../features/docker-builds.md) for universal compilation
