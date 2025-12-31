# ccgo

A cross-platform C++ build system designed to simplify and accelerate multi-platform development.

[![PyPI version](https://badge.fury.io/py/ccgo.svg)](https://pypi.org/project/ccgo/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/license/MIT)

## Features

- 🚀 Fast cross-platform C++ builds for Android, iOS, macOS, Windows, Linux, and OpenHarmony (OHOS)
- 📦 Kotlin Multiplatform (KMP) support
- 📦 Conan C/C++ package manager integration
- 🧪 Integrated testing with GoogleTest
- 📊 Benchmarking support with Google Benchmark
- 📚 Documentation generation
- 🛠️ Project scaffolding from templates
- ✅ Environment dependency checking
- 🧹 Smart build artifact cleaning

## Installation

```bash
# Install from PyPI
pip3 install ccgo

# Or install from source in development mode
cd ccgo
pip3 install -e .
```

## Quick Start

```bash
# Create a new C++ library project
ccgo new my-awesome-lib

# Navigate to the project directory
cd my-awesome-lib/<project_relative_path>

# Build for Android
ccgo build android

# Run tests
ccgo test

# Build documentation
ccgo doc --open
```

## Commands Reference

### 1. Project Creation

#### `ccgo new` - Create New Project

Create a new library project in a new directory.

```bash
ccgo new <project-name> [options]
```

**Options:**
- `--template-url <url>` - Custom template repository URL
- `--data <key>=<value>` - Template variables (repeatable)
- `--defaults` - Use default values for all prompts

**Examples:**
```bash
# Create with interactive prompts
ccgo new my-project

# Create with all defaults
ccgo new my-project --defaults

# Use custom template
ccgo new my-project --template-url https://github.com/user/template.git
ccgo new my-project --template-url /path/to/user/template

# Set template variables
ccgo new my-project --data cpy_project_version=2.0.0
```

#### `ccgo init` - Initialize in Current Directory

Initialize a library project in the current directory.

```bash
ccgo init [options]
```

**Options:**
- `--template-url <url>` - Custom template repository URL
- `--data <key>=<value>` - Template variables (repeatable)
- `--defaults` - Use default values for all prompts
- `--force` - Skip confirmation prompt

**Examples:**
```bash
ccgo init
ccgo init --defaults --force
```

### 2. Build Commands

#### `ccgo build` - Build for Platforms

Build your library for specific platforms.

```bash
ccgo build <target> [options]
```

**Targets:**
- `android` - Build for Android (supports `--arch`)
- `ios` - Build for iOS
- `macos` - Build for macOS
- `windows` - Build for Windows
- `linux` - Build for Linux
- `ohos` - Build for OpenHarmony (supports `--arch`)
- `kmp` - Build Kotlin Multiplatform library
- `conan` - Build Conan C/C++ package
- `include` - Build include headers

**Options:**
- `--arch <architectures>` - Comma-separated architecture list (Android/OHOS only)
  - Android: `armeabi-v7a`, `arm64-v8a`, `x86_64`
  - OHOS: `armeabi-v7a`, `arm64-v8a`, `x86_64`
- `--link-type <type>` - Library link type: `static`, `shared`, or `both` (default: `both`)
- `--toolchain <toolchain>` - Windows toolchain: `auto`, `msvc`, or `mingw` (default: `auto`)
- `--ide-project` - Generate IDE project files
- `--docker` - Build using Docker (cross-platform builds)

**Examples:**
```bash
# Build for Android with specific architectures
ccgo build android --arch armeabi-v7a,arm64-v8a

# Build for OHOS with all architectures
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64

# Build for iOS
ccgo build ios

# Build for macOS
ccgo build macos

# Build for Windows
ccgo build windows

# Build for Windows with specific toolchain
ccgo build windows --toolchain msvc
ccgo build windows --toolchain mingw

# Build for Linux
ccgo build linux

# Build static libraries only
ccgo build linux --link-type static

# Build shared libraries only
ccgo build macos --link-type shared

# Build both static and shared libraries (default)
ccgo build ios --link-type both

# Build Kotlin Multiplatform library
ccgo build kmp

# Build Conan C/C++ package
ccgo build conan

# Generate IDE project for Android
ccgo build android --ide-project

# Cross-platform build using Docker
ccgo build linux --docker
ccgo build windows --docker
```

### 3. Testing & Benchmarking

#### `ccgo test` - Run Tests

Build and run GoogleTest-based unit tests.

```bash
ccgo test [options]
```

**Options:**
- `--build-only` - Only build tests without running
- `--run-only` - Only run tests (assumes already built)
- `--filter <pattern>` - GoogleTest filter (e.g., 'MyTest*')
- `--ide-project` - Generate IDE project for tests
- `--gtest-args <args>` - Additional GoogleTest arguments

**Examples:**
```bash
# Build and run all tests
ccgo test

# Only build tests
ccgo test --build-only

# Run specific tests
ccgo test --filter "MyTest*"

# Run tests multiple times
ccgo test --gtest-args "--gtest_repeat=3"

# Generate IDE project for debugging tests
ccgo test --ide-project
```

#### `ccgo bench` - Run Benchmarks

Build and run Google Benchmark-based performance benchmarks.

```bash
ccgo bench [options]
```

**Options:**
- `--build-only` - Only build benchmarks without running
- `--run-only` - Only run benchmarks (assumes already built)
- `--filter <pattern>` - Google Benchmark filter (e.g., 'BM_Sort*')
- `--ide-project` - Generate IDE project for benchmarks
- `--benchmark-args <args>` - Additional Google Benchmark arguments
- `--format <format>` - Output format: `console`, `json`, `csv` (default: console)

**Examples:**
```bash
# Build and run all benchmarks
ccgo bench

# Only build benchmarks
ccgo bench --build-only

# Run specific benchmarks
ccgo bench --filter "BM_Sort*"

# Output results as JSON
ccgo bench --format json

# Output results as CSV
ccgo bench --format csv
```

### 4. Documentation

#### `ccgo doc` - Build Documentation

Generate project documentation (typically using Doxygen).

```bash
ccgo doc [options]
```

**Options:**
- `--open` - Open documentation in browser after building
- `--serve` - Start local web server to view documentation
- `--port <port>` - Port for web server (default: 8000)
- `--clean` - Clean build before generating

**Examples:**
```bash
# Build documentation
ccgo doc

# Build and open in browser
ccgo doc --open

# Build and serve on localhost:8000
ccgo doc --serve

# Serve on custom port
ccgo doc --serve --port 3000

# Clean build
ccgo doc --clean
```

### 5. Publishing

#### `ccgo publish` - Publish Libraries

Publish your library to package repositories.

```bash
ccgo publish <target>
```

**Targets:**
- `android` - Publish to Maven repository
- `ohos` - Publish to OHPM repository
- `kmp` - Publish KMP library to Maven (local or remote)

**Examples:**
```bash
# Publish Android library to Maven
ccgo publish android

# Publish OHOS library to OHPM
ccgo publish ohos

# Publish Kotlin Multiplatform library
ccgo publish kmp
```

### 6. Maintenance Commands

#### `ccgo check` - Check Dependencies

Verify that platform-specific development dependencies are installed.

```bash
ccgo check [target] [options]
```

**Targets:**
- `all` - Check all platforms (default)
- `android` - Check Android development environment
- `ios` - Check iOS development environment
- `macos` - Check macOS development environment
- `windows` - Check Windows development environment
- `linux` - Check Linux development environment
- `ohos` - Check OpenHarmony development environment

**Options:**
- `--verbose` - Show detailed information

**Examples:**
```bash
# Check all platforms
ccgo check

# Check Android environment
ccgo check android

# Check with verbose output
ccgo check ios --verbose
```

#### `ccgo clean` - Clean Build Artifacts

Remove build artifacts and caches.

```bash
ccgo clean [target] [options]
```

**Targets:**
- `all` - Clean all platforms (default)
- `android` - Clean Android build caches
- `ios` - Clean iOS build caches
- `macos` - Clean macOS build caches
- `ohos` - Clean OpenHarmony build caches
- `kmp` - Clean Kotlin Multiplatform build caches
- `examples` - Clean examples build caches

**Options:**
- `--native-only` - Clean only `cmake_build/` (native CMake builds)
- `--dry-run` - Show what would be cleaned without deleting
- `-y, --yes` - Skip confirmation prompts

**Examples:**
```bash
# Clean all (with confirmation)
ccgo clean

# Clean only Android
ccgo clean android

# Preview what will be deleted
ccgo clean --dry-run

# Clean all without confirmation
ccgo clean -y

# Clean only native CMake builds
ccgo clean --native-only
```

### 7. Help

#### `ccgo help` - Show Help

Display comprehensive help information.

```bash
ccgo help

# Or get help for specific command
ccgo <command> --help
```

## Environment Variables

### Android
- `ANDROID_HOME` - Android SDK location
- `ANDROID_NDK_HOME` - Android NDK location
- `JAVA_HOME` - Java Development Kit location

### OpenHarmony (OHOS)
- `OHOS_SDK_HOME` or `HOS_SDK_HOME` - OHOS Native SDK location

### iOS/macOS
- Requires Xcode and command-line tools

## Project Structure

Projects created with ccgo follow this structure:

```
my-project/
├── CCGO.toml                   # CCGO project config
├── CMakeLists.txt              # Root CMake configuration
├── src/                        # Source code
├── include/                    # Public headers
├── docs/                       # docs files
├── tests/                      # GoogleTest unit tests
├── benches/                    # Benchmark tests
├── android/                    # Android-specific files (Gradle)
├── ohos/                       # OHOS-specific files (hvigor)
├── kmp/                        # Kotlin Multiplatform files (Gradle)
```

## Advanced Usage

### Using Custom Templates

You can create projects from custom templates:

```bash
# From GitHub repository
ccgo new my-project --template-url=https://github.com/user/my-template.git

# From local directory
ccgo new my-project --template-url=/path/to/local/template
```

### CI/CD Integration

The generated `build.py` script supports CI/CD workflows with environment variables:

- `CI_IS_RELEASE` - Build as release vs beta
- `CI_BUILD_<PLATFORM>` - Enable/disable platform builds

Example:
```bash
export CI_IS_RELEASE=1
export CI_BUILD_ANDROID=1
export CI_BUILD_IOS=1
python3 build.py
```

### Multi-Architecture Builds

Build for multiple architectures simultaneously:

```bash
# Android: build for 32-bit ARM, 64-bit ARM, and x86_64
ccgo build android --arch armeabi-v7a,arm64-v8a,x86_64

# OHOS: build for all supported architectures
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64
```

## Troubleshooting

### Common Issues

1. **"Command not found" after installation**
   - Ensure `pip3` install directory is in your PATH
   - Try `python3 -m ccgo` instead of `ccgo`

2. **Android build fails**
   - Verify `ANDROID_HOME`, `ANDROID_NDK_HOME`, and `JAVA_HOME` are set
   - Run `ccgo check android --verbose` to diagnose

3. **OHOS build fails**
   - Verify `OHOS_SDK_HOME` or `HOS_SDK_HOME` is set
   - Run `ccgo check ohos --verbose` to diagnose

4. **iOS/macOS build fails**
   - Ensure Xcode and command-line tools are installed
   - Run `xcode-select --install` if needed

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

ccgo is available under the [MIT license](https://opensource.org/license/MIT).
See the LICENSE file for the full license text.

## Links

- [GitHub Repository](https://github.com/zhlinh/ccgo)
- [PyPI Package](https://pypi.org/project/ccgo/)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)