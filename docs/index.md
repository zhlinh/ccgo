# CCGO

[![PyPI](https://img.shields.io/pypi/v/ccgo.svg)](https://pypi.org/project/ccgo/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://readthedocs.org/projects/ccgo/badge/?version=latest)](https://ccgo.readthedocs.io/)

A modern C++ cross-platform build system and project generator that simplifies building native libraries for Android, iOS, macOS, Windows, Linux, OpenHarmony, and Kotlin Multiplatform.

## Features

- **Universal Cross-Platform**: Build for 8+ platforms from a single codebase
- **Zero Configuration**: Works out-of-box with sensible defaults
- **Docker-Based Builds**: Build any platform on any OS without local toolchain setup
- **Unified Publishing**: Publish to Maven, CocoaPods, SPM, OHPM, and Conan with one command
- **Smart Dependency Management**: Git, path, and registry-based dependencies with lockfile support
- **Template-Driven**: Generate new projects with modern C++ best practices
- **Git Integration**: Automatic versioning and commit management
- **CMake Integration**: Leverage CMake's power with simplified configuration

## Supported Platforms

| Platform | Architectures | Output Formats |
|----------|--------------|----------------|
| **Android** | armeabi-v7a, arm64-v8a, x86, x86_64 | AAR, Static/Shared libs |
| **iOS** | armv7, arm64, x86_64, arm64-simulator | Framework, XCFramework |
| **macOS** | x86_64, arm64 (Apple Silicon) | Framework, XCFramework |
| **Windows** | x86, x86_64 | DLL, Static libs (MSVC/MinGW) |
| **Linux** | x86_64, aarch64 | Shared/Static libs |
| **OpenHarmony** | armeabi-v7a, arm64-v8a, x86_64 | HAR packages |
| **watchOS** | armv7k, arm64_32, x86_64 | Framework, XCFramework |
| **tvOS** | arm64, x86_64 | Framework, XCFramework |

## Quick Links

- [Installation](getting-started/installation.md) - Get CCGO installed
- [Quick Start](getting-started/quickstart.md) - Create your first project in 5 minutes
- [Configuration](getting-started/configuration.md) - Configure CCGO for your project
- [Platforms](platforms/index.md) - Platform-specific build guides
- [CLI Reference](reference/cli.md) - Complete command-line reference

## Example Usage

```bash
# Create a new project
ccgo new myproject

# Build for Android
cd myproject/myproject
ccgo build android --arch arm64-v8a,armeabi-v7a

# Build for iOS (requires macOS)
ccgo build ios

# Build for Windows using Docker (works on any OS)
ccgo build windows --docker

# Run tests
ccgo test

# Publish to Maven Local
ccgo publish android --registry local
```

## Why CCGO?

1. **Simplicity**: One tool, one config file (CCGO.toml), all platforms
2. **Speed**: Parallel builds, incremental compilation, Docker caching
3. **Flexibility**: Support both Python-based and CMake-only workflows
4. **Modern**: Built with Rust for reliability and performance
5. **Universal**: Docker builds enable any-to-any cross-compilation
6. **Team-Friendly**: Lockfile ensures reproducible builds across team members

## Architecture

CCGO consists of four main components:

- **ccgo**: Python/Rust CLI tool that orchestrates builds and manages projects
- **ccgo-template**: Copier-based template for generating new C++ projects
- **ccgo-gradle-plugins**: Gradle convention plugins for Android/KMP builds
- **ccgo-now**: Example project demonstrating CCGO capabilities

## Community

- [GitHub Issues](https://github.com/zhlinh/ccgo/issues) - Bug reports and feature requests
- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions) - Questions and community support
- [Roadmap](development/roadmap.md) - See what's coming next

## License

CCGO is licensed under the MIT License. See [LICENSE](https://github.com/zhlinh/ccgo/blob/main/LICENSE) for details.
