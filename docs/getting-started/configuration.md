# Configuration Guide

Complete guide to configuring CCGO projects through CCGO.toml and other configuration files.

## Overview

CCGO uses a file-based configuration system with:

- **CCGO.toml**: Main project configuration
- **build_config.py**: Build-specific settings (generated)
- **CMakeLists.txt**: CMake integration
- **.ccgoignore**: Files to exclude from operations
- **Environment variables**: Runtime configuration

## CCGO.toml

### File Location

```
myproject/
├── CCGO.toml          # Main configuration
├── src/
├── include/
└── tests/
```

### Basic Structure

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My cross-platform C++ library"
authors = ["Your Name <you@example.com>"]
license = "MIT"
homepage = "https://github.com/myuser/mylib"
repository = "https://github.com/myuser/mylib"

[library]
type = "both"                    # static, shared, or both

[build]
cpp_standard = "17"              # C++ standard version

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[android]
min_sdk_version = 21

[ios]
deployment_target = "12.0"
```

## Package Configuration

### Required Fields

```toml
[package]
name = "mylib"                   # REQUIRED: Project name (lowercase, no spaces)
version = "1.0.0"                # REQUIRED: Semantic version
```

### Optional Metadata

```toml
[package]
description = "A powerful C++ library for..."
authors = [
    "John Doe <john@example.com>",
    "Jane Smith <jane@example.com>"
]
license = "MIT"                  # License identifier
license_file = "LICENSE"         # Path to license file
readme = "README.md"             # Path to README
homepage = "https://mylib.dev"
repository = "https://github.com/user/mylib"
documentation = "https://docs.mylib.dev"
keywords = ["networking", "async", "performance"]
categories = ["network", "concurrency"]
```

### Version Field

Version must follow [Semantic Versioning](https://semver.org/):

```toml
[package]
version = "1.0.0"                # Major.Minor.Patch
# version = "1.0.0-alpha"        # Pre-release
# version = "1.0.0-beta.1"       # Pre-release with number
# version = "1.0.0+build.123"    # Build metadata
```

**Rules:**
- MAJOR: Breaking changes
- MINOR: New features (backward compatible)
- PATCH: Bug fixes
- Pre-release: Optional `-alpha`, `-beta`, `-rc.N`
- Build metadata: Optional `+build.N`

## Library Configuration

### Library Type

```toml
[library]
type = "static"                  # Static library only
# type = "shared"                # Shared library only
# type = "both"                  # Both static and shared (default)
```

**Static library:**
- Compiled into executable
- Larger executable size
- Faster startup
- No runtime dependencies

**Shared library:**
- Loaded at runtime
- Smaller executable
- Can be updated independently
- Requires library at runtime

**Both:**
- Builds both types
- Users choose at link time

### Library Naming

```toml
[library]
name = "mylib"                   # Override library name (optional)
# Default: uses package.name
```

**Generated files:**
- Static: `libmylib.a` (Unix) / `mylib.lib` (Windows MSVC)
- Shared: `libmylib.so` (Linux) / `libmylib.dylib` (macOS) / `mylib.dll` (Windows)

## Build Configuration

### C++ Standard

```toml
[build]
cpp_standard = "17"              # C++17 (recommended)
# cpp_standard = "11"            # C++11
# cpp_standard = "14"            # C++14
# cpp_standard = "20"            # C++20
# cpp_standard = "23"            # C++23
```

**Support by platform:**
- C++11: All platforms
- C++14: All platforms
- C++17: All platforms (recommended)
- C++20: Modern compilers only
- C++23: Cutting-edge compilers

### Build Types

```toml
[build]
default_build_type = "release"   # Default: release
# default_build_type = "debug"   # For development
```

**Debug:**
- No optimization
- Debug symbols
- Assertions enabled
- Larger binary size

**Release:**
- Full optimization
- Stripped symbols (separate file)
- Assertions disabled
- Smaller binary size

### Compiler Flags

```toml
[build]
cflags = ["-Wall", "-Wextra"]                    # C flags
cxxflags = ["-Wall", "-Wextra", "-pedantic"]     # C++ flags
ldflags = ["-Wl,-rpath,$ORIGIN"]                 # Linker flags
```

**Common flags:**

```toml
[build]
# Warnings
cxxflags = [
    "-Wall",                     # All warnings
    "-Wextra",                   # Extra warnings
    "-Werror",                   # Treat warnings as errors
    "-pedantic"                  # Strict ISO C++
]

# Optimization
cxxflags = [
    "-O3",                       # Maximum optimization
    "-march=native",             # CPU-specific optimization
    "-flto"                      # Link-time optimization
]

# Security
cxxflags = [
    "-fstack-protector-strong",  # Stack protection
    "-D_FORTIFY_SOURCE=2",       # Buffer overflow detection
    "-fPIC"                      # Position independent code
]
```

### Defines

```toml
[build]
defines = [
    "USE_FEATURE_X",             # Simple define
    "MAX_CONNECTIONS=100",       # Define with value
    "DEBUG_LOGGING"              # Debug-only define
]
```

**Platform-specific defines:**

```toml
[build.android]
defines = ["ANDROID_PLATFORM"]

[build.ios]
defines = ["IOS_PLATFORM"]

[build.windows]
defines = ["WINDOWS_PLATFORM", "_WIN32_WINNT=0x0601"]
```

### Include Directories

```toml
[build]
include_dirs = [
    "include",                   # Public headers
    "src/internal",              # Private headers
    "third_party/lib/include"    # Third-party includes
]
```

### Source Files

By default, CCGO compiles all `.cpp/.cc/.cxx` files in `src/`. Override:

```toml
[build]
sources = [
    "src/**/*.cpp",              # All .cpp in src/
    "src/core/*.cc",             # Specific directory
    "src/platform/linux/*.cpp"   # Platform-specific
]

exclude = [
    "src/experimental/**",       # Exclude directory
    "src/**/*_test.cpp"          # Exclude test files
]
```

## Dependency Configuration

### Git Dependencies

```toml
[dependencies]
# Tag (recommended for stability)
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Branch (for latest features)
fmt = { git = "https://github.com/fmtlib/fmt.git", branch = "master" }

# Commit hash (for exact reproducibility)
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280a" }
```

### Path Dependencies

```toml
[dependencies]
# Relative path
myutils = { path = "../myutils" }

# Absolute path
common = { path = "/opt/libs/common" }

# Workspace dependency
core = { path = "./libs/core" }
```

### Optional Dependencies

```toml
[dependencies]
# Required dependencies
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Optional dependencies
[dependencies.optional]
networking = { git = "https://github.com/user/networking.git", tag = "v1.0.0" }
database = { git = "https://github.com/user/database.git", tag = "v2.0.0" }
```

Enable with features:

```toml
[features]
default = ["basic"]
basic = []
network = ["networking"]        # Enables networking dependency
db = ["database"]               # Enables database dependency
full = ["basic", "network", "db"]
```

Build with features:

```bash
ccgo build android --features network,db
```

### Platform-Specific Dependencies

```toml
[dependencies]
common = { git = "https://github.com/user/common.git", tag = "v1.0.0" }

# Android-only
[target.'cfg(target_os = "android")'.dependencies]
android-log = { git = "https://github.com/user/android-log.git", tag = "v1.0.0" }

# iOS-only
[target.'cfg(target_os = "ios")'.dependencies]
ios-utils = { path = "./ios-utils" }

# Windows-only
[target.'cfg(target_os = "windows")'.dependencies]
win32-api = { git = "https://github.com/user/win32-api.git", tag = "v1.0.0" }
```

## Platform-Specific Configuration

### Android

```toml
[android]
min_sdk_version = 21             # Minimum API level
target_sdk_version = 34          # Target API level
ndk_version = "26.1.10909125"    # NDK version (optional)
stl = "c++_shared"               # STL type: c++_static, c++_shared
package_name = "com.example.mylib"  # Java package name
```

### iOS

```toml
[ios]
deployment_target = "12.0"       # Minimum iOS version
enable_bitcode = false           # Bitcode support (deprecated)
enable_arc = true                # Automatic Reference Counting
frameworks = [                   # System frameworks
    "Foundation",
    "UIKit",
    "CoreGraphics"
]
```

### macOS

```toml
[macos]
deployment_target = "10.15"      # Minimum macOS version
enable_hardened_runtime = true   # Hardened runtime
frameworks = [                   # System frameworks
    "Foundation",
    "AppKit"
]
```

### Windows

```toml
[windows]
subsystem = "console"            # Subsystem: console, windows
runtime_library = "MD"           # Runtime: MT, MD, MTd, MDd
windows_sdk_version = "10.0"     # Windows SDK version
```

### Linux

```toml
[linux]
min_glibc_version = "2.17"       # Minimum glibc version
link_pthread = true              # Link pthread
link_dl = true                   # Link libdl
link_rt = true                   # Link librt
```

### OpenHarmony

```toml
[ohos]
api_version = 9                  # API version
package_name = "com.example.mylib"  # Package name
```

## Features Configuration

### Defining Features

```toml
[features]
# Default features (enabled automatically)
default = ["std"]

# Feature with no dependencies
std = []

# Feature that enables dependencies
network = ["cpp-httplib", "openssl"]

# Feature that enables other features
full = ["std", "network", "database"]

# Feature combinations
web = ["network", "json"]
```

### Using Features

**In code:**

```cpp
#ifdef CCGO_FEATURE_NETWORK
    // Network code
    #include <httplib.h>
#endif

#ifdef CCGO_FEATURE_DATABASE
    // Database code
    #include <sqlite3.h>
#endif
```

**At build time:**

```bash
# Build with specific features
ccgo build android --features network

# Build with multiple features
ccgo build android --features network,database

# Build with all features
ccgo build android --all-features

# Build without default features
ccgo build android --no-default-features

# Combine flags
ccgo build android --no-default-features --features network
```

## Test Configuration

### Test Settings

```toml
[test]
# Test framework (default: catch2)
framework = "catch2"             # catch2, gtest, or custom

# Test sources
sources = [
    "tests/**/*.cpp"
]

# Test dependencies
[test.dependencies]
catch2 = { git = "https://github.com/catchorg/Catch2.git", tag = "v3.4.0" }
```

### Running Tests

```bash
# Run all tests
ccgo test

# Run specific test
ccgo test --filter "MyTest"

# Run with verbose output
ccgo test --verbose
```

## Benchmark Configuration

### Benchmark Settings

```toml
[bench]
# Benchmark framework
framework = "google-benchmark"   # google-benchmark or custom

# Benchmark sources
sources = [
    "benches/**/*.cpp"
]

# Benchmark dependencies
[bench.dependencies]
benchmark = { git = "https://github.com/google/benchmark.git", tag = "v1.8.3" }
```

### Running Benchmarks

```bash
# Run all benchmarks
ccgo bench

# Run specific benchmark
ccgo bench --filter "MyBenchmark"

# With iterations
ccgo bench --iterations 1000
```

## Documentation Configuration

### Documentation Settings

```toml
[doc]
# Documentation generator
generator = "doxygen"            # doxygen or custom

# Source directories
source_dirs = [
    "include",
    "src",
    "docs"
]

# Output directory
output_dir = "target/doc"
```

### Generating Documentation

```bash
# Generate documentation
ccgo doc

# Generate and open in browser
ccgo doc --open
```

## Publishing Configuration

### Package Metadata

```toml
[package]
name = "mylib"
version = "1.0.0"
authors = ["Your Name <you@example.com>"]
license = "MIT"
description = "My cross-platform library"
homepage = "https://github.com/user/mylib"
repository = "https://github.com/user/mylib"
documentation = "https://docs.mylib.dev"
```

### Publishing Settings

```toml
[publish]
# Registries
registry = "default"             # Registry name

# Maven (Android)
[publish.maven]
group_id = "com.example"
artifact_id = "mylib"

# CocoaPods (Apple)
[publish.cocoapods]
pod_name = "MyLib"
swift_version = "5.0"

# OHPM (OpenHarmony)
[publish.ohpm]
package_name = "@example/mylib"
```

## build_config.py

Generated file with build-specific configuration:

```python
# build_config.py (auto-generated)

# Project information
PROJECT_NAME = "mylib"
PROJECT_VERSION = "1.0.0"

# Build settings
BUILD_TYPE = "release"
CPP_STANDARD = "17"
LIBRARY_TYPE = "both"

# Platforms
ANDROID_MIN_SDK = 21
IOS_DEPLOYMENT_TARGET = "12.0"

# Custom settings
CUSTOM_DEFINES = []
CUSTOM_FLAGS = []
```

**Usage in build scripts:**

```python
from build_config import PROJECT_NAME, PROJECT_VERSION

print(f"Building {PROJECT_NAME} v{PROJECT_VERSION}")
```

## .ccgoignore

Exclude files from CCGO operations:

```
# .ccgoignore

# Build directories
cmake_build/
target/
bin/

# IDE files
.vscode/
.idea/
*.swp

# OS files
.DS_Store
Thumbs.db

# Dependencies
vendor/
node_modules/

# Generated files
*.pyc
__pycache__/
```

**Syntax:**
- `#` for comments
- `*` wildcard for files
- `**` wildcard for directories
- `/` to match from root
- `!` to negate (include)

**Example:**

```
# Ignore all .log files
*.log

# But include important.log
!important.log

# Ignore build/ in root only
/build/

# Ignore all build/ directories
**/build/
```

## Environment Variables

### Build-Time Variables

```bash
# Build type
export BUILD_TYPE=debug          # debug or release

# Verbosity
export CCGO_VERBOSE=1            # Verbose output

# Parallel builds
export CMAKE_BUILD_PARALLEL_LEVEL=8  # Use 8 cores

# Architecture
export ANDROID_ARCH=arm64-v8a    # Android architecture
```

### Platform-Specific Variables

**Android:**

```bash
export ANDROID_HOME=/path/to/android-sdk
export ANDROID_NDK_HOME=/path/to/ndk
export ANDROID_MIN_SDK=21
```

**iOS:**

```bash
export CODE_SIGN_IDENTITY="Apple Development"
export DEVELOPMENT_TEAM="TEAM123456"
export IOS_DEPLOYMENT_TARGET="12.0"
```

**Windows:**

```bash
export MSVC_VERSION=2022         # Visual Studio version
export WINDOWS_SDK_VERSION=10.0.22621.0
```

### Docker Variables

```bash
# Docker build
export USE_DOCKER=1              # Enable Docker builds

# Docker image
export DOCKER_IMAGE=ccgo-builder-linux:latest
```

## Configuration Validation

### Check Configuration

```bash
# Validate CCGO.toml
ccgo check

# Validate for specific platform
ccgo check android

# Verbose validation
ccgo check --verbose
```

### Common Issues

**Invalid version format:**

```
Error: Invalid version '1.0' in CCGO.toml
```

**Solution:** Use semantic versioning (e.g., `1.0.0`)

**Missing required field:**

```
Error: Missing required field 'name' in [package]
```

**Solution:** Add required field to CCGO.toml

**Invalid dependency:**

```
Error: Invalid dependency format for 'spdlog'
```

**Solution:** Check dependency syntax

## Configuration Templates

### Minimal Configuration

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "static"
```

### Standard Configuration

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My library"
authors = ["Your Name <you@example.com>"]
license = "MIT"

[library]
type = "both"

[build]
cpp_standard = "17"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[android]
min_sdk_version = 21

[ios]
deployment_target = "12.0"
```

### Advanced Configuration

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "Advanced C++ library"
authors = ["Team <team@example.com>"]
license = "MIT"
homepage = "https://mylib.dev"
repository = "https://github.com/user/mylib"

[library]
type = "both"

[build]
cpp_standard = "20"
cxxflags = ["-Wall", "-Wextra", "-Werror"]
defines = ["USE_ADVANCED_FEATURES"]

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[dependencies.optional]
networking = { git = "https://github.com/user/networking.git", tag = "v1.0.0" }

[features]
default = ["basic"]
basic = []
network = ["networking"]
full = ["basic", "network"]

[android]
min_sdk_version = 21
target_sdk_version = 34
package_name = "com.example.mylib"

[ios]
deployment_target = "12.0"
frameworks = ["Foundation", "UIKit"]

[test]
framework = "catch2"

[test.dependencies]
catch2 = { git = "https://github.com/catchorg/Catch2.git", tag = "v3.4.0" }
```

## Best Practices

### 1. Use Semantic Versioning

```toml
[package]
version = "1.0.0"                # Good: Major.Minor.Patch
# version = "1.0"                # Bad: Missing patch
# version = "v1.0.0"             # Bad: Don't include 'v' prefix
```

### 2. Pin Dependencies

```toml
[dependencies]
# Good: Specific tag
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Bad: Tracking branch
# spdlog = { git = "https://github.com/gabime/spdlog.git", branch = "master" }
```

### 3. Organize Sections

```toml
# Good: Logical order
[package]
[library]
[build]
[dependencies]
[android]
[ios]

# Bad: Random order
[ios]
[package]
[dependencies]
[build]
```

### 4. Comment Your Config

```toml
[build]
# Enable all warnings
cxxflags = ["-Wall", "-Wextra"]

# Platform-specific optimization
defines = [
    "USE_SSE=1",                 # Enable SSE instructions
    "MAX_THREADS=8"              # Limit thread pool size
]
```

### 5. Keep It Simple

Only configure what you need:

```toml
# Good: Only necessary config
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "static"

# Bad: Unnecessary config
# [build]
# cpp_standard = "17"            # Default is 17
# [library]
# name = "mylib"                 # Same as package.name
```

## Migration Guide

### From CMakeLists.txt

**CMakeLists.txt:**

```cmake
project(mylib VERSION 1.0.0)
set(CMAKE_CXX_STANDARD 17)
add_library(mylib src/mylib.cpp)
```

**CCGO.toml:**

```toml
[package]
name = "mylib"
version = "1.0.0"

[build]
cpp_standard = "17"
```

### From Conan

**conanfile.txt:**

```ini
[requires]
spdlog/1.12.0
fmt/10.1.1

[options]
spdlog:shared=False
```

**CCGO.toml:**

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[library]
type = "static"
```

## See Also

- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Project Structure](project-structure.md)
- [Dependency Management](../features/dependency-management.md)
- [Build System](../features/build-system.md)
