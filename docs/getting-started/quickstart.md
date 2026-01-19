# Quick Start

Get started with CCGO in 5 minutes! This guide will walk you through creating your first cross-platform C++ project.

## Create a New Project

```bash
# Create a new project named "hello"
ccgo new hello

# Navigate to the project directory
cd hello/hello
```

The generated project structure:

```
hello/
└── hello/           # Main project directory
    ├── CCGO.toml    # Project configuration
    ├── CMakeLists.txt
    ├── include/     # Public headers
    │   └── hello/
    │       └── hello.h
    ├── src/         # Source files
    │   └── hello.cpp
    ├── tests/       # Unit tests
    │   └── test_hello.cpp
    ├── benches/     # Benchmarks
    │   └── bench_hello.cpp
    └── examples/    # Example programs
        └── example_hello.cpp
```

## Build for Your Platform

=== "Native Build"
    ```bash
    # Build for your current platform
    ccgo build
    ```

=== "Android"
    ```bash
    # Build for Android (multiple architectures)
    ccgo build android --arch arm64-v8a,armeabi-v7a
    ```

=== "iOS"
    ```bash
    # Build for iOS (requires macOS)
    ccgo build ios
    ```

=== "Docker Build"
    ```bash
    # Build for any platform using Docker (works on any OS)
    ccgo build linux --docker
    ccgo build windows --docker
    ccgo build macos --docker
    ```

## Run Tests

```bash
# Run unit tests
ccgo test

# Run benchmarks
ccgo bench
```

## Add a Dependency

Edit `CCGO.toml`:

```toml
[dependencies]
# From Git repository
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# From local path
# mylib = { path = "../mylib" }

# From registry (coming soon)
# fmt = "10.1.1"
```

Install dependencies:

```bash
ccgo install
```

Use the dependency in your code (`src/hello.cpp`):

```cpp
#include <spdlog/spdlog.h>

void greet(const std::string& name) {
    spdlog::info("Hello, {}!", name);
}
```

## Publish Your Library

=== "Maven Local"
    ```bash
    # Build and publish to Maven Local
    ccgo publish android --registry local
    ```

=== "CocoaPods"
    ```bash
    # Build and publish to CocoaPods
    ccgo publish apple --manager cocoapods
    ```

=== "Swift Package Manager"
    ```bash
    # Build and publish to SPM
    ccgo publish apple --manager spm --push
    ```

## Configure Your Project

Edit `CCGO.toml` to customize your project:

```toml
[package]
name = "hello"
version = "1.0.0"
description = "A cross-platform C++ library"
authors = ["Your Name <you@example.com>"]
license = "MIT"

[library]
type = "both"  # "static", "shared", or "both"
namespace = "hello"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[build]
cpp_standard = 17
cmake_minimum_version = "3.20"

[android]
min_sdk_version = 21
target_sdk_version = 33

[ios]
min_deployment_target = "12.0"
```

## Next Steps

- [Configuration Guide](configuration.md) - Learn about all CCGO.toml options
- [Platforms](../platforms/index.md) - Platform-specific build guides
- [Features](../features/build-system.md) - Explore CCGO features
- [CLI Reference](../reference/cli.md) - Complete command reference

## Common Commands

```bash
# Project creation
ccgo new <name>          # Create new project
ccgo init                # Initialize CCGO in existing project

# Building
ccgo build <platform>    # Build for specific platform
ccgo build --docker      # Build using Docker
ccgo clean               # Clean build artifacts

# Testing
ccgo test                # Run tests
ccgo bench               # Run benchmarks

# Dependency management
ccgo install             # Install dependencies
ccgo install --locked    # Use exact versions from lockfile
ccgo vendor              # Vendor dependencies locally

# Publishing
ccgo publish <platform> --registry <type>  # Publish library
ccgo tag                 # Create version tag

# Utilities
ccgo check <platform>    # Check platform requirements
ccgo doc --open          # Generate and open documentation
```

## Troubleshooting

### Build Fails

```bash
# Check platform requirements
ccgo check android

# Try Docker build if local toolchain has issues
ccgo build android --docker
```

### Dependency Issues

```bash
# Remove lock file and reinstall
rm CCGO.lock
ccgo install

# Vendor dependencies for offline builds
ccgo vendor
```

### Need Help?

- Check [Documentation](https://ccgo.readthedocs.io)
- Browse [Examples](https://github.com/zhlinh/ccgo-now)
- Ask in [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- Report bugs in [GitHub Issues](https://github.com/zhlinh/ccgo/issues)
