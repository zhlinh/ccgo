# CLI Command Reference

Complete command-line reference for all CCGO commands.

## Overview

CCGO provides a comprehensive CLI for C++ cross-platform development with fast startup times and zero Python dependencies (Rust implementation).

## Global Options

Available for all commands:

```bash
ccgo [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

| Option | Description |
|--------|-------------|
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |
| `-v, --verbose` | Enable verbose output |
| `--no-color` | Disable colored terminal output |

## Commands Overview

| Category | Command | Description |
|----------|---------|-------------|
| **Project** | [new](#new) | Create a new project |
| | [init](#init) | Initialize CCGO in existing project |
| **Build** | [build](#build) | Build for target platform |
| | [run](#run) | Build and run example/binary |
| | [test](#test) | Run GoogleTest unit tests |
| | [bench](#bench) | Run Google Benchmark benchmarks |
| | [doc](#doc) | Generate Doxygen documentation |
| **Dependencies** | [install](#install) | Install dependencies from CCGO.toml |
| | [add](#add) | Add a dependency |
| | [remove](#remove) | Remove a dependency |
| | [update](#update) | Update dependencies to latest versions |
| | [vendor](#vendor) | Vendor dependencies for offline builds |
| | [tree](#tree) | Display dependency tree |
| **Discovery** | [search](#search) | Search for packages |
| | [collection](#collection) | Manage package collections |
| **Publishing** | [publish](#publish) | Publish to package managers |
| | [package](#package) | Package SDK for distribution |
| **Maintenance** | [check](#check) | Check platform requirements |
| | [clean](#clean) | Clean build artifacts |
| | [tag](#tag) | Create version tag |

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

Build C++ library for specific platform(s).

```bash
ccgo build [OPTIONS] <TARGET>
```

### Arguments

- `<TARGET>` - Build target (required)

**Available Targets:**

| Target | Description |
|--------|-------------|
| `all` | All supported platforms |
| `apple` | All Apple platforms (iOS, macOS, watchOS, tvOS) |
| `android` | Android (NDK-based) |
| `ios` | iOS (arm64, simulator) |
| `macos` | macOS (x86_64, arm64 universal) |
| `windows` | Windows (MSVC/MinGW) |
| `linux` | Linux (GCC/Clang) |
| `ohos` | OpenHarmony (Hvigor-based) |
| `watchos` | Apple Watch |
| `tvos` | Apple TV |
| `kmp` | Kotlin Multiplatform |
| `conan` | Conan package |

### Options

| Option | Description |
|--------|-------------|
| `--arch <ARCH>` | Target architectures (comma-separated) |
| `--link-type <TYPE>` | `static`, `shared`, or `both` (default: `both`) |
| `--docker` | Build using Docker container |
| `--auto-docker` | Auto-detect and use Docker when native build not possible |
| `-j, --jobs <N>` | Number of parallel build jobs |
| `--ide-project` | Generate IDE project files (Xcode, Visual Studio, etc.) |
| `--release` | Build in release mode (default: debug) |
| `--native-only` | Build native libraries only, skip AAR/HAR packaging |
| `--toolchain <TOOL>` | Windows toolchain: `msvc`, `mingw`, `auto` (default: `auto`) |
| `--dev` | Development mode (use pre-built ccgo from GitHub in Docker) |
| `-F, --features <FEATURES>` | Enable features (comma-separated) |
| `--no-default-features` | Disable default features |
| `--all-features` | Enable all available features |

### Platform-Specific Architectures

**Android:**
- `armeabi-v7a` - ARM 32-bit
- `arm64-v8a` - ARM 64-bit (default)
- `x86` - Intel 32-bit (emulator)
- `x86_64` - Intel 64-bit (emulator)

**iOS:**
- `arm64` - iPhone/iPad (default)
- `simulator` - Simulator (auto-detects host arch)

**macOS:**
- `x86_64` - Intel Macs
- `arm64` - Apple Silicon
- Universal binary created by default (both architectures)

**Windows:**
- `x86_64` - 64-bit (default)

**Linux:**
- `x86_64` - 64-bit (default)
- `aarch64` - ARM 64-bit

**OpenHarmony:**
- `armeabi-v7a` - ARM 32-bit
- `arm64-v8a` - ARM 64-bit (default)
- `x86_64` - Intel 64-bit

### Features System

Control conditional compilation with features:

```bash
# Enable specific features
ccgo build android --features networking,ssl

# Disable default features and enable only specific ones
ccgo build android --features minimal --no-default-features

# Enable all features
ccgo build android --all-features
```

Features are defined in `CCGO.toml`:

```toml
[features]
default = ["networking"]
networking = []
ssl = ["networking"]
advanced = ["networking", "ssl"]
```

### Docker Builds

Build any platform from any OS using Docker:

```bash
# Explicit Docker usage
ccgo build linux --docker
ccgo build windows --docker
ccgo build android --docker

# Auto-detect when needed (e.g., building Linux from macOS)
ccgo build linux --auto-docker
```

**Benefits:**
- Universal cross-compilation
- No local toolchain installation required
- Consistent build environment
- Pre-built images from Docker Hub

### Examples

```bash
# Build Android with specific architectures
ccgo build android --arch arm64-v8a,armeabi-v7a

# Build iOS in release mode
ccgo build ios --release

# Build for all Apple platforms
ccgo build apple --release

# Build Windows with MSVC toolchain only
ccgo build windows --toolchain msvc

# Build Linux using Docker (from macOS/Windows)
ccgo build linux --docker

# Build with parallel jobs
ccgo build android -j 8

# Build and generate Xcode project
ccgo build ios --ide-project

# Build with specific features
ccgo build android --features networking,ssl --release

# Build all platforms
ccgo build all --release
```

### Build Output

Builds are output to `target/<platform>/` with unified archive structure:

```
target/android/
└── mylib_Android_SDK-1.0.0.zip
    ├── lib/
    │   ├── static/arm64-v8a/libmylib.a
    │   └── shared/arm64-v8a/libmylib.so
    ├── haars/mylib.aar
    ├── include/mylib/
    └── build_info.json
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

Copy all dependencies to vendor/ directory for offline builds.

```bash
ccgo vendor [OPTIONS]
```

### Options

| Option | Description |
|--------|-------------|
| `--no-delete` | Don't delete existing vendor directory |

### Examples

```bash
# Vendor all dependencies
ccgo vendor

# Preserve existing vendor directory
ccgo vendor --no-delete
```

---

## tree

Display project dependency tree with various visualization options.

```bash
ccgo tree [OPTIONS] [PACKAGE]
```

### Arguments

- `[PACKAGE]` - Show dependencies of a specific package (optional)

### Options

| Option | Description |
|--------|-------------|
| `-d, --depth <DEPTH>` | Maximum depth to display (default: unlimited) |
| `--no-dedupe` | Don't deduplicate repeated dependencies |
| `-l, --locked` | Show dependency versions from lock file |
| `-f, --format <FORMAT>` | Output format: text, json, dot (default: text) |
| `--duplicates` | Show only duplicate dependencies |
| `-i, --invert <PACKAGE>` | Show packages that depend on this package |
| `--conflicts` | Highlight version conflicts |

### Output Formats

**Text** (default):
- Tree-style visualization with box-drawing characters
- Shows dependency hierarchy
- Marks duplicates with `(*)`

**JSON**:
- Structured data with version and source information
- Includes conflict detection results
- Suitable for programmatic processing

**DOT** (Graphviz):
- Graph visualization format
- Highlights conflicts in red
- Can be rendered with `dot` command

### Examples

```bash
# Show full dependency tree
ccgo tree

# Limit depth to 2 levels
ccgo tree --depth 2

# Show specific package dependencies
ccgo tree spdlog

# Show all occurrences (no deduplication)
ccgo tree --no-dedupe

# Output as JSON
ccgo tree --format json

# Generate Graphviz diagram
ccgo tree --format dot > deps.dot
dot -Tpng deps.dot -o deps.png

# Show reverse dependencies
ccgo tree --invert fmt

# Highlight version conflicts
ccgo tree --conflicts

# Show only duplicate dependencies
ccgo tree --duplicates

# Use locked versions from CCGO.toml.lock
ccgo tree --locked
```

---

## search

Search for packages in subscribed collections.

```bash
ccgo search <QUERY> [OPTIONS]
```

### Arguments

- `<QUERY>` - Search keyword or pattern (required)

### Options

| Option | Description |
|--------|-------------|
| `-c, --collection <NAME>` | Search only in specific collection |
| `-d, --details` | Show detailed package information |
| `--limit <N>` | Limit number of results (default: 20) |

### Examples

```bash
# Search all collections for packages
ccgo search json

# Search in specific collection
ccgo search logging --collection official

# Show detailed information
ccgo search crypto --details

# Limit results to 50
ccgo search lib --limit 50

# Combined options
ccgo search network --collection community --details --limit 10
```

### Search Results

Results include:
- Package name and version
- Summary description
- Source collection
- Repository URL (with --details)
- Supported platforms (with --details)
- License information (with --details)
- Keywords (with --details)

---

## collection

Manage package collections for discovery.

```bash
ccgo collection <SUBCOMMAND> [OPTIONS]
```

### Subcommands

| Subcommand | Description |
|------------|-------------|
| `add <URL>` | Add a new collection |
| `list` | List all subscribed collections |
| `remove <NAME>` | Remove a collection |
| `refresh [NAME]` | Refresh collection(s) |

### collection add

Add a new package collection.

```bash
ccgo collection add <URL>
```

**Supported URL schemes:**
- `file://` - Local file path
- `http://` - HTTP URL
- `https://` - HTTPS URL

**Arguments:**
- `<URL>` - Collection URL (required)

**Examples:**
```bash
# Add official collection
ccgo collection add https://ccgo.dev/collections/official.json

# Add community collection
ccgo collection add https://ccgo.dev/collections/community.json

# Add local collection
ccgo collection add file:///path/to/my-collection.json

# Add from HTTP
ccgo collection add http://example.com/packages.json
```

### collection list

List all subscribed collections.

```bash
ccgo collection list [OPTIONS]
```

**Options:**

| Option | Description |
|--------|-------------|
| `-d, --details` | Show detailed information |

**Examples:**
```bash
# List collections
ccgo collection list

# Show detailed information
ccgo collection list --details
```

### collection remove

Remove a subscribed collection.

```bash
ccgo collection remove <NAME_OR_URL>
```

**Arguments:**
- `<NAME_OR_URL>` - Collection name or URL (required)

**Examples:**
```bash
# Remove by name
ccgo collection remove official

# Remove by URL
ccgo collection remove https://ccgo.dev/collections/community.json
```

### collection refresh

Refresh collection data.

```bash
ccgo collection refresh [NAME]
```

**Arguments:**
- `[NAME]` - Collection name to refresh (optional, default: all)

**Examples:**
```bash
# Refresh all collections
ccgo collection refresh

# Refresh specific collection
ccgo collection refresh official
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
