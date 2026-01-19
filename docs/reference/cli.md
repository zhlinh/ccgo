# CLI Reference

Complete command-line reference for CCGO.

## Global Options

These options are available for all commands:

```bash
ccgo [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |
| `--verbose` | Enable verbose output |
| `--quiet` | Suppress non-error output |
| `--color <WHEN>` | Control color output: auto, always, never |

## Commands Overview

| Command | Description |
|---------|-------------|
| [new](#new) | Create a new project |
| [init](#init) | Initialize CCGO in existing project |
| [build](#build) | Build for target platform |
| [test](#test) | Run tests |
| [bench](#bench) | Run benchmarks |
| [doc](#doc) | Generate documentation |
| [install](#install) | Install dependencies |
| [update](#update) | Update dependencies |
| [vendor](#vendor) | Vendor dependencies locally |
| [add](#add) | Add a dependency |
| [remove](#remove) | Remove a dependency |
| [publish](#publish) | Publish library |
| [check](#check) | Check platform requirements |
| [clean](#clean) | Clean build artifacts |
| [tag](#tag) | Create version tag |
| [package](#package) | Package source distribution |
| [run](#run) | Build and run example/binary |
| [ci](#ci) | Run CI build pipeline |

---

## new

Create a new CCGO project from template.

```bash
ccgo new [OPTIONS] <NAME>
```

### Arguments

- `<NAME>` - Project name (required)

### Options

| Option | Description |
|--------|-------------|
| `--template <URL>` | Custom Copier template URL |
| `--defaults` | Use default values without prompting |
| `--vcs-ref <REF>` | Template git reference (branch/tag) |

### Examples

```bash
# Create new project with interactive prompts
ccgo new myproject

# Create with defaults
ccgo new myproject --defaults

# Use custom template
ccgo new myproject --template https://github.com/user/template

# Use specific template version
ccgo new myproject --vcs-ref v2.0.0
```

---

## init

Initialize CCGO in an existing project.

```bash
ccgo init [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--template <URL>` | Custom Copier template URL |
| `--defaults` | Use default values without prompting |
| `--force` | Overwrite existing files |

### Examples

```bash
# Initialize in current directory
ccgo init

# Force overwrite existing files
ccgo init --force
```

---

## build

Build the project for specified platform.

```bash
ccgo build [OPTIONS] [PLATFORM]
```

### Arguments

- `[PLATFORM]` - Target platform: android, ios, macos, windows, linux, ohos, watchos, tvos, kmp

### Options

| Option | Description |
|--------|-------------|
| `--arch <ARCH>` | Target architecture(s), comma-separated |
| `--link-type <TYPE>` | Link type: static, shared, both (default: both) |
| `--toolchain <TOOL>` | Toolchain: msvc, mingw, auto (Windows only) |
| `--docker` | Build using Docker |
| `--ide-project` | Generate IDE project files |
| `--release` | Build in release mode |
| `--debug` | Build in debug mode (default) |
| `--clean` | Clean before build |

### Platform-Specific Architectures

**Android**: armeabi-v7a, arm64-v8a, x86, x86_64
**iOS**: armv7, arm64, x86_64 (sim), arm64 (sim)
**macOS**: x86_64, arm64
**Windows**: x86, x86_64
**Linux**: x86_64, aarch64
**OpenHarmony**: armeabi-v7a, arm64-v8a, x86_64

### Examples

```bash
# Build for current platform
ccgo build

# Build Android with multiple architectures
ccgo build android --arch arm64-v8a,armeabi-v7a

# Build iOS in release mode
ccgo build ios --release

# Build Windows with MSVC using Docker
ccgo build windows --toolchain msvc --docker

# Build and generate Xcode project
ccgo build ios --ide-project

# Clean build
ccgo build linux --clean
```

---

## test

Run project tests.

```bash
ccgo test [OPTIONS] [PLATFORM]
```

### Arguments

- `[PLATFORM]` - Platform to test on (default: current)

### Options

| Option | Description |
|--------|-------------|
| `--filter <PATTERN>` | Run tests matching pattern |
| `--verbose` | Verbose test output |
| `--no-fail-fast` | Continue running tests after failure |

### Examples

```bash
# Run all tests
ccgo test

# Run specific test
ccgo test --filter test_name

# Run tests verbosely
ccgo test --verbose
```

---

## bench

Run project benchmarks.

```bash
ccgo bench [OPTIONS] [PLATFORM]
```

### Arguments

- `[PLATFORM]` - Platform to benchmark on (default: current)

### Options

| Option | Description |
|--------|-------------|
| `--filter <PATTERN>` | Run benchmarks matching pattern |
| `--baseline <NAME>` | Save/compare against baseline |

### Examples

```bash
# Run all benchmarks
ccgo bench

# Run specific benchmark
ccgo bench --filter bench_name

# Save baseline
ccgo bench --baseline main

# Compare against baseline
ccgo bench --baseline main
```

---

## doc

Generate project documentation.

```bash
ccgo doc [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--open` | Open documentation in browser |
| `--no-deps` | Don't include dependencies |
| `--format <FMT>` | Output format: html, markdown |

### Examples

```bash
# Generate and open documentation
ccgo doc --open

# Generate markdown docs
ccgo doc --format markdown
```

---

## install

Install project dependencies.

```bash
ccgo install [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--locked` | Use exact versions from CCGO.lock |
| `--offline` | Use vendored dependencies only |

### Examples

```bash
# Install dependencies
ccgo install

# Install with locked versions
ccgo install --locked

# Offline install
ccgo install --offline
```

---

## update

Update dependencies to latest compatible versions.

```bash
ccgo update [OPTIONS] [DEPENDENCY]
```

### Arguments

- `[DEPENDENCY]` - Specific dependency to update (optional)

### Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would be updated |

### Examples

```bash
# Update all dependencies
ccgo update

# Update specific dependency
ccgo update spdlog

# Dry run
ccgo update --dry-run
```

---

## vendor

Copy all dependencies to vendor/ directory.

```bash
ccgo vendor [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--no-delete` | Don't delete existing vendor directory |

### Examples

```bash
# Vendor dependencies
ccgo vendor
```

---

## add

Add a dependency to CCGO.toml.

```bash
ccgo add [OPTIONS] <DEPENDENCY>
```

### Arguments

- `<DEPENDENCY>` - Dependency specification

### Dependency Formats

```bash
# Git repository
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# Local path
ccgo add mylib --path ../mylib

# Registry (future)
ccgo add fmt@10.1.1
```

### Options

| Option | Description |
|--------|-------------|
| `--git <URL>` | Git repository URL |
| `--tag <TAG>` | Git tag |
| `--branch <BRANCH>` | Git branch |
| `--rev <REV>` | Git revision |
| `--path <PATH>` | Local path |

### Examples

```bash
# Add from Git with tag
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# Add from local path
ccgo add mylib --path ../mylib
```

---

## remove

Remove a dependency from CCGO.toml.

```bash
ccgo remove <DEPENDENCY>
```

### Arguments

- `<DEPENDENCY>` - Dependency name

### Examples

```bash
# Remove dependency
ccgo remove spdlog
```

---

## publish

Publish library to registry.

```bash
ccgo publish [OPTIONS] <PLATFORM>
```

### Arguments

- `<PLATFORM>` - Platform to publish: android, ios, macos, apple, ohos, conan, kmp

### Options

| Option | Description |
|--------|-------------|
| `--registry <TYPE>` | Registry: local, official, private |
| `--url <URL>` | Custom registry URL (private only) |
| `--skip-build` | Use existing build artifacts |
| `--manager <MGR>` | Package manager (apple): cocoapods, spm, all |
| `--push` | Push git tags (SPM only) |
| `--remote-name <NAME>` | Git remote name (default: origin) |

### Examples

```bash
# Publish Android to Maven Local
ccgo publish android --registry local

# Publish to Maven Central
ccgo publish android --registry official

# Publish iOS to CocoaPods
ccgo publish apple --manager cocoapods

# Publish to SPM with git push
ccgo publish apple --manager spm --push

# Publish to private Maven
ccgo publish android --registry private --url https://maven.example.com
```

---

## check

Check if platform requirements are met.

```bash
ccgo check [OPTIONS] [PLATFORM]
```

### Arguments

- `[PLATFORM]` - Platform to check (default: all)

### Options

| Option | Description |
|--------|-------------|
| `--verbose` | Show detailed check results |

### Examples

```bash
# Check all platforms
ccgo check

# Check specific platform
ccgo check android --verbose
```

---

## clean

Clean build artifacts.

```bash
ccgo clean [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--dry-run` | Show what would be deleted |
| `-y, --yes` | Skip confirmation prompt |

### Examples

```bash
# Preview what will be deleted
ccgo clean --dry-run

# Clean without confirmation
ccgo clean -y
```

---

## tag

Create a version tag from CCGO.toml.

```bash
ccgo tag [OPTIONS] [VERSION]
```

### Arguments

- `[VERSION]` - Version tag (default: from CCGO.toml)

### Options

| Option | Description |
|--------|-------------|
| `-m, --message <MSG>` | Tag message |
| `--push` | Push tag to remote |

### Examples

```bash
# Create tag from CCGO.toml version
ccgo tag

# Create custom version tag
ccgo tag v2.0.0 -m "Release 2.0.0"

# Create and push tag
ccgo tag --push
```

---

## package

Package source for distribution.

```bash
ccgo package [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--format <FMT>` | Package format: tar.gz, zip (default: tar.gz) |
| `--output <PATH>` | Output path |

### Examples

```bash
# Create source package
ccgo package

# Create ZIP package
ccgo package --format zip

# Custom output path
ccgo package --output /path/to/output
```

---

## run

Build and run example or binary.

```bash
ccgo run [OPTIONS] <TARGET>
```

### Arguments

- `<TARGET>` - Example or binary name

### Options

| Option | Description |
|--------|-------------|
| `--example` | Run as example (default if TARGET in examples/) |
| `--bin` | Run as binary |
| `--release` | Build in release mode |
| `-- <ARGS>` | Arguments to pass to the program |

### Examples

```bash
# Run example
ccgo run my_example

# Run with arguments
ccgo run my_example -- --arg1 value1

# Run in release mode
ccgo run my_example --release
```

---

## ci

Run CI build pipeline.

```bash
ccgo ci [OPTIONS]
```

Reads `CI_BUILD_*` environment variables to determine what to build.

### Environment Variables

- `CI_BUILD_ANDROID` - Build Android if set
- `CI_BUILD_IOS` - Build iOS if set
- `CI_BUILD_MACOS` - Build macOS if set
- `CI_BUILD_WINDOWS` - Build Windows if set
- `CI_BUILD_LINUX` - Build Linux if set
- `CI_BUILD_OHOS` - Build OpenHarmony if set

### Examples

```bash
# Run CI builds based on environment
export CI_BUILD_ANDROID=1
export CI_BUILD_IOS=1
ccgo ci
```

---

## Environment Variables

CCGO respects the following environment variables:

| Variable | Description |
|----------|-------------|
| `CCGO_HOME` | CCGO home directory (default: ~/.ccgo) |
| `ANDROID_HOME` | Android SDK path |
| `ANDROID_NDK` | Android NDK path |
| `OHOS_SDK_HOME` | OpenHarmony SDK path |
| `CCGO_CMAKE_DIR` | Custom CMake scripts directory |
| `NO_COLOR` | Disable colored output |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Build error |
| 4 | Dependency error |
| 101 | User cancelled operation |

---

## Getting Help

```bash
# Get help for any command
ccgo <command> --help

# Examples
ccgo build --help
ccgo publish --help
```

For more information, see:
- [CCGO.toml Reference](ccgo-toml.md)
- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
