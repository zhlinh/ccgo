# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [3.0.5] - 2025-01-04

### Fixed

- Pass toolchain argument for Windows builds in Docker

### Changed

- Use consistent badge style for PyPI version in documentation

## [3.0.4] - 2025-01-03

### Added

- ELF parsing support for better binary analysis
- Unified include directory handling across all platforms

### Fixed

- Prioritize `out/` directory for merged libraries
- Rewrite builders for improved reliability

## [3.0.3] - 2025-01-02

### Added

- Docker support for OHOS platform builds
- Pre-installed ccgo from PyPI in Docker images for faster builds

### Fixed

- Linux SDK archive structure and content

## [3.0.2] - 2025-01-01

### Added

- Release script for automated version bumps
- Lock file for dependency management

### Changed

- Updated version dependencies

## [3.0.1] - 2024-12-31

### Added

- crates.io version badge to README header
- Release automation workflows for CI/CD
- Docker image publishing workflows

### Fixed

- Dual registry publishing support (PyPI and crates.io)
- Unreachable code warnings in Rust implementation

## [3.0.0] - 2024-12-30

### Added

- **Rust CLI rewrite** - Complete rewrite of CCGO CLI in Rust for better performance
- Docker-based cross-platform builds (Linux, Windows, macOS, iOS, watchOS, tvOS, Android)
- Embedded Dockerfiles in binary at compile time
- Pre-built binaries for multiple platforms (Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64)

### Changed

- **BREAKING**: CMake build output structure changed to `cmake_build/{release|debug}/<platform>`
- Improved AAR packaging - only add AAR to `haars/android/` in SDK archive

### Fixed

- Clean up unused imports and mark intentionally unused code

## Installation

**Python Package (PyPI):**
```bash
pip install ccgo==3.0.5
# or
pip install --upgrade ccgo
```

**Rust Binary (crates.io):**
```bash
cargo install ccgo
```

**Pre-built Binaries:**
Download platform-specific binaries from [GitHub Releases](https://github.com/zhlinh/ccgo/releases).

## Quick Start

```bash
# Create a new C++ cross-platform project
ccgo new my-project

# Build for different platforms
cd my-project/<project_name>
ccgo build android
ccgo build ios
ccgo build macos
ccgo build linux
ccgo build windows

# Cross-platform builds using Docker
ccgo build linux --docker
ccgo build windows --docker
ccgo build macos --docker
ccgo build ios --docker
```

## Available Commands

- `ccgo new` - Create a new C++ cross-platform project
- `ccgo init` - Initialize CCGO in an existing project
- `ccgo build` - Build for specific platforms (android, ios, macos, linux, windows, ohos, kmp)
- `ccgo test` - Run tests
- `ccgo bench` - Run benchmarks
- `ccgo doc` - Generate documentation
- `ccgo publish` - Publish to registries (Maven, OHPM, CocoaPods, SPM, Conan)
- `ccgo check` - Check platform dependencies
- `ccgo clean` - Clean build artifacts
- `ccgo tag` - Create version tag
- `ccgo package` - Package source for distribution
- `ccgo install` - Install dependencies
- `ccgo ci` - CI build orchestration

## Supported Platforms

- **Android** - AAR packages via Gradle (armeabi-v7a, arm64-v8a, x86, x86_64)
- **iOS** - Frameworks/XCFrameworks via Xcode (arm64, x86_64 simulator)
- **macOS** - Frameworks/XCFrameworks (arm64, x86_64)
- **Linux** - Static/shared libraries (x86_64, aarch64)
- **Windows** - Static/shared libraries via MSVC or MinGW (x86_64)
- **OpenHarmony (OHOS)** - HAR packages via Hvigor (armeabi-v7a, arm64-v8a, x86_64)
- **watchOS** - Frameworks/XCFrameworks (arm64, simulator)
- **tvOS** - Frameworks/XCFrameworks (arm64, simulator)

## Publishing Targets

- **Maven Central** - Android/KMP libraries
- **Maven Local** - Local development
- **OHPM** - OpenHarmony packages
- **CocoaPods** - iOS/macOS frameworks
- **Swift Package Manager (SPM)** - iOS/macOS frameworks
- **Conan** - C++ package manager
- **GitHub Pages** - Documentation

## Links

- [GitHub Repository](https://github.com/zhlinh/ccgo)
- [PyPI Package](https://pypi.org/project/ccgo/)
- [crates.io Package](https://crates.io/crates/ccgo)
- [Documentation](https://github.com/zhlinh/ccgo/tree/main/docs)
