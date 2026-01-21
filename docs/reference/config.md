# CCGO.toml Configuration Reference

> Version: v3.1.0 | Updated: 2026-01-21

This document provides a complete reference for the `CCGO.toml` configuration file, which controls all aspects of your C++ cross-platform project in CCGO.

## Table of Contents

1. [Overview](#overview)
2. [File Structure](#file-structure)
3. [Package Section](#package-section)
4. [Workspace Section](#workspace-section)
5. [Dependencies](#dependencies)
6. [Features](#features)
7. [Build Configuration](#build-configuration)
8. [Platform Configurations](#platform-configurations)
9. [Binary and Example Targets](#binary-and-example-targets)
10. [Publishing Configuration](#publishing-configuration)
11. [Complete Examples](#complete-examples)

---

## Overview

CCGO uses a TOML-based configuration file similar to Cargo.toml for Rust projects. The `CCGO.toml` file should be placed in your project's root directory.

### Basic Requirements

Every `CCGO.toml` must contain **at least one** of the following:
- `[package]` section - for a single package/library
- `[workspace]` section - for managing multiple related packages

A configuration can have both sections (workspace root that is also a package).

---

## File Structure

### Minimal Package Configuration

```toml
[package]
name = "mylib"
version = "1.0.0"
```

### Minimal Workspace Configuration

```toml
[workspace]
members = ["core", "utils"]
```

---

## Package Section

The `[package]` section defines metadata for a single package.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | **Yes** | Package name (must be valid C++ identifier) |
| `version` | string | **Yes** | Semantic version (e.g., "1.0.0") |
| `description` | string | No | Brief description of the package |
| `authors` | array[string] | No | List of authors |
| `license` | string | No | SPDX license identifier (e.g., "MIT", "Apache-2.0") |
| `repository` | string | No | Git repository URL |

### Example

```toml
[package]
name = "mylib"
version = "1.2.3"
description = "My awesome C++ library"
authors = ["John Doe <john@example.com>", "Jane Smith"]
license = "MIT"
repository = "https://github.com/user/mylib"
```

### Legacy Alias

For backward compatibility, `[project]` is accepted as an alias for `[package]`:

```toml
[project]  # Treated the same as [package]
name = "mylib"
version = "1.0.0"
```

---

## Workspace Section

The `[workspace]` section enables managing multiple related packages together.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `members` | array[string] | **Yes** | Workspace member paths (supports glob patterns) |
| `exclude` | array[string] | No | Paths to exclude from membership |
| `resolver` | string | No | Dependency resolver version ("1" or "2", default: "1") |
| `default_members` | array[string] | No | Default members for workspace commands |

### Workspace Dependencies

Workspace-level dependencies can be defined and inherited by members:

```toml
[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
```

### Member Path Patterns

The `members` array supports glob patterns:

```toml
[workspace]
members = [
    "core",              # Exact path
    "utils",             # Exact path
    "examples/*",        # All direct subdirectories
    "plugins/**"         # All subdirectories recursively
]
exclude = ["examples/deprecated"]
```

### Resolver Versions

- **"1"** (default): Legacy resolver
- **"2"**: New resolver with better feature unification and conflict resolution

### Complete Workspace Example

```toml
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/old"]
resolver = "2"
default_members = ["core", "utils"]

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]

[[workspace.dependencies]]
name = "spdlog"
version = "^1.12"
```

### Workspace Member Inheritance

Members can inherit dependencies from the workspace:

**Workspace root (CCGO.toml):**
```toml
[workspace]
members = ["core"]

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]
```

**Member (core/CCGO.toml):**
```toml
[package]
name = "mylib-core"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true           # Inherit from workspace
features = ["extra"]       # Add extra features (merged with workspace features)
```

After resolution, the member's fmt dependency will have:
- `version = "^10.0"` (from workspace)
- `git = "..."` (from workspace)
- `features = ["std", "extra"]` (merged)

---

## Dependencies

Dependencies are defined as an array of tables using `[[dependencies]]`.

### Dependency Sources

CCGO supports multiple dependency sources:

#### 1. Version-Based Dependencies (Future)

```toml
[[dependencies]]
name = "fmt"
version = "^10.0"  # Semantic version requirement
```

#### 2. Git Dependencies

```toml
[[dependencies]]
name = "spdlog"
version = "^1.12"
git = "https://github.com/gabime/spdlog.git"
branch = "v1.x"    # Optional: specific branch
```

```toml
[[dependencies]]
name = "json"
version = "^3.11"
git = "https://github.com/nlohmann/json.git"
tag = "v3.11.2"    # Optional: specific tag
```

```toml
[[dependencies]]
name = "pinned"
version = "1.0.0"
git = "https://github.com/user/pinned.git"
rev = "abc123"     # Optional: specific commit hash
```

#### 3. Path Dependencies

```toml
[[dependencies]]
name = "local-utils"
version = "1.0.0"
path = "../utils"  # Relative or absolute path
```

### Dependency Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | **Yes** | Dependency name |
| `version` | string | Conditional | Version requirement (required unless `workspace = true`) |
| `git` | string | No | Git repository URL |
| `branch` | string | No | Git branch name |
| `tag` | string | No | Git tag |
| `rev` | string | No | Git revision (commit hash) |
| `path` | string | No | Local file path |
| `optional` | boolean | No | Whether dependency is optional (default: false) |
| `features` | array[string] | No | Features to enable for this dependency |
| `default_features` | boolean | No | Whether to enable default features (default: true) |
| `workspace` | boolean | No | Inherit from workspace dependencies (default: false) |

### Version Requirements

CCGO supports semantic versioning ranges:

| Syntax | Meaning | Example |
|--------|---------|---------|
| `^1.2.3` | Compatible with 1.2.3 (>=1.2.3, <2.0.0) | `^10.0` |
| `~1.2.3` | Reasonably close to 1.2.3 (>=1.2.3, <1.3.0) | `~1.12.0` |
| `>=1.2.3` | Greater than or equal | `>=1.0,<2.0` |
| `1.2.*` | Wildcard versions | `1.*` |
| `1.2.3` | Exact version | `10.2.1` |

### Optional Dependencies

Optional dependencies are only included when enabled by a feature:

```toml
[[dependencies]]
name = "http-client"
version = "^1.0"
optional = true  # Only included when feature enables it

[features]
networking = ["http-client"]  # Feature that enables optional dependency
```

### Dependency Features

Enable specific features of a dependency:

```toml
[[dependencies]]
name = "serde"
version = "^1.0"
features = ["derive", "std"]
default_features = false  # Disable default features
```

### Workspace Dependency Inheritance

Members can inherit dependencies from the workspace:

```toml
[[dependencies]]
name = "fmt"
workspace = true              # Required: inherit from workspace
features = ["extra-feature"]  # Optional: add additional features
```

### Complete Dependencies Example

```toml
# Regular dependency from git
[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
tag = "10.2.1"

# Local path dependency
[[dependencies]]
name = "utils"
version = "1.0.0"
path = "../shared/utils"

# Optional dependency for a feature
[[dependencies]]
name = "openssl"
version = "^1.1"
optional = true

# Dependency with features
[[dependencies]]
name = "spdlog"
version = "^1.12"
features = ["std", "fmt_external"]
default_features = false

# Workspace-inherited dependency
[[dependencies]]
name = "googletest"
workspace = true
```

---

## Features

The `[features]` section defines conditional compilation and optional dependencies.

### Default Features

```toml
[features]
default = ["std"]  # Features enabled by default
```

### Feature Definitions

Features can depend on:
1. Other features
2. Optional dependency names
3. Dependency feature syntax (`dep/feature`)

```toml
[features]
default = ["std"]
std = []                           # Empty feature (just a flag)
networking = ["http-client"]       # Enables optional dependency
advanced = ["networking", "async"] # Depends on other features
full = ["networking", "advanced"]  # Transitive dependencies
derive = ["serde/derive"]          # Enables feature in a dependency
```

### Feature Resolution

Features are resolved recursively:

```toml
[features]
default = ["std"]
std = []
networking = ["http-client"]
advanced = ["networking"]  # Also enables "http-client"
full = ["advanced"]        # Enables "advanced", "networking", and "http-client"
```

### Using Features

#### Enable features during build:

```bash
ccgo build android --features networking,advanced
```

#### Disable default features:

```bash
ccgo build ios --no-default-features
```

#### Enable specific features without defaults:

```bash
ccgo build linux --no-default-features --features networking
```

### Complete Features Example

```toml
[features]
default = ["std"]
std = []
networking = ["http-client", "tls"]
async = ["async-runtime"]
full = ["networking", "async", "logging"]
logging = ["spdlog-dep"]

# Optional dependencies enabled by features
[[dependencies]]
name = "http-client"
version = "^1.0"
optional = true

[[dependencies]]
name = "async-runtime"
version = "^2.0"
optional = true

[[dependencies]]
name = "spdlog-dep"
version = "^1.12"
optional = true

# Dependency feature syntax
[[dependencies]]
name = "serde"
version = "^1.0"

# Feature can enable serde's derive feature
[features]
derive = ["serde/derive"]
```

---

## Build Configuration

The `[build]` section configures build system behavior.

### Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `parallel` | boolean | No | Enable parallel builds (default: false) |
| `jobs` | integer | No | Number of parallel jobs |
| `symbol_visibility` | boolean | No | Symbol visibility (default: false for hidden) |
| `submodule_deps` | table | No | Submodule internal dependencies for shared linking |

### Example

```toml
[build]
parallel = true
jobs = 4
symbol_visibility = false  # Hidden by default

# For projects with multiple submodules/components
[build.submodule_deps]
api = ["base"]           # API depends on base
feature = ["base", "core"]  # Feature depends on base and core
```

The `submodule_deps` maps to the `CCGO_CONFIG_DEPS_MAP` CMake variable for shared library linking.

---

## Platform Configurations

Platform-specific configurations are defined under `[platforms.<platform>]`.

### Supported Platforms

- `android`
- `ios`
- `macos`
- `windows`
- `linux`
- `ohos` (OpenHarmony)

### Android Configuration

```toml
[platforms.android]
min_sdk = 21                        # Minimum SDK version
architectures = [                    # Target architectures
    "armeabi-v7a",
    "arm64-v8a",
    "x86_64"
]
```

### iOS Configuration

```toml
[platforms.ios]
min_version = "13.0"  # Minimum iOS deployment target
```

### macOS Configuration

```toml
[platforms.macos]
min_version = "10.15"  # Minimum macOS deployment target
```

### Windows Configuration

```toml
[platforms.windows]
toolchain = "auto"  # auto, msvc, or mingw
```

Toolchain options:
- `"auto"`: Build with both MSVC and MinGW (default)
- `"msvc"`: MSVC only
- `"mingw"`: MinGW only

### Linux Configuration

```toml
[platforms.linux]
architectures = ["x86_64", "aarch64"]
```

### OpenHarmony (OHOS) Configuration

```toml
[platforms.ohos]
min_api = 9                          # Minimum API level
architectures = [
    "armeabi-v7a",
    "arm64-v8a"
]
```

### Complete Platform Example

```toml
[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a", "x86_64"]

[platforms.ios]
min_version = "13.0"

[platforms.macos]
min_version = "10.15"

[platforms.windows]
toolchain = "auto"

[platforms.linux]
architectures = ["x86_64"]

[platforms.ohos]
min_api = 9
architectures = ["arm64-v8a"]
```

---

## Binary and Example Targets

### Binary Targets

Define executable binaries with `[[bin]]`:

```toml
[[bin]]
name = "my-cli"              # Binary name
path = "src/bin/cli.cpp"     # Path to main source file

[[bin]]
name = "my-server"
path = "src/bin/server.cpp"
```

Run binaries with:
```bash
ccgo run my-cli
ccgo run my-server -- --help
```

### Example Targets

Define example programs with `[[example]]`:

```toml
[[example]]
name = "basic-usage"
path = "examples/basic.cpp"  # Optional: defaults to examples/{name}.cpp

[[example]]
name = "advanced"
# path defaults to examples/advanced.cpp or examples/advanced/main.cpp
```

Run examples with:
```bash
ccgo run --example basic-usage
ccgo run --example advanced
```

---

## Publishing Configuration

Publishing configurations are currently in the legacy `[project]` format from Python implementation. This section will be updated when the Rust implementation adds publishing support.

### Maven Publishing (Android/KMP)

```toml
[publish.android.maven]
group_id = "com.example.mylib"
artifact_id = "mylib"  # Optional, defaults to package name
channel_desc = ""       # e.g., "beta", "release"

dependencies = [
    { group = "com.example", artifact = "dep1", version = "1.0.0" },
    { group = "com.example", artifact = "dep2", version = "2.0.0" }
]
```

### Apple Publishing (CocoaPods/SPM)

```toml
[publish.apple]
pod_name = "MyLib"
platforms = ["ios", "macos"]
min_ios_version = "13.0"
min_macos_version = "10.15"
summary = "My library description"

[publish.apple.cocoapods]
enabled = true
repo = "trunk"  # or "private" with spec_repo URL
license = "MIT"
homepage = "https://github.com/user/mylib"
static_framework = true

[publish.apple.spm]
enabled = true
git_url = "https://github.com/user/mylib"
use_local_path = false
```

### OHOS Publishing (OHPM)

```toml
[publish.ohos.ohpm]
registry = "official"  # official, private, or local
dependencies = []
```

---

## Complete Examples

### Single Package Library

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My C++ library"
authors = ["Developer <dev@example.com>"]
license = "MIT"
repository = "https://github.com/user/mylib"

[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"

[[dependencies]]
name = "spdlog"
version = "^1.12"
optional = true

[features]
default = ["std"]
std = []
logging = ["spdlog"]

[build]
parallel = true
jobs = 4
symbol_visibility = false

[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a", "x86_64"]

[platforms.ios]
min_version = "13.0"
```

### Workspace with Multiple Packages

**Workspace root (CCGO.toml):**

```toml
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/old"]
resolver = "2"

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]

[[workspace.dependencies]]
name = "googletest"
version = "^1.14"
git = "https://github.com/google/googletest.git"

# Optional: workspace root can also be a package
[package]
name = "myproject"
version = "1.0.0"
```

**Member package (core/CCGO.toml):**

```toml
[package]
name = "myproject-core"
version = "1.0.0"
description = "Core library"

[[dependencies]]
name = "fmt"
workspace = true  # Inherit from workspace

[[dependencies]]
name = "spdlog"
version = "^1.12"
git = "https://github.com/gabime/spdlog.git"
```

**Member package (utils/CCGO.toml):**

```toml
[package]
name = "myproject-utils"
version = "1.0.0"
description = "Utility library"

[[dependencies]]
name = "fmt"
workspace = true
features = ["chrono"]  # Add extra features to workspace dependency

[[dependencies]]
name = "myproject-core"
path = "../core"  # Depend on another workspace member
```

### Library with Binaries and Examples

```toml
[package]
name = "advanced-lib"
version = "2.0.0"
description = "Advanced C++ library with CLI tools"
license = "Apache-2.0"

[[dependencies]]
name = "fmt"
version = "^10.0"

[[dependencies]]
name = "argparse"
version = "^2.9"
optional = true

[features]
default = []
cli = ["argparse"]

[[bin]]
name = "mytool"
path = "src/bin/tool.cpp"

[[bin]]
name = "converter"
path = "src/bin/converter.cpp"

[[example]]
name = "basic"
# Defaults to examples/basic.cpp

[[example]]
name = "advanced"
path = "examples/advanced/main.cpp"

[build]
parallel = true

[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a"]

[platforms.linux]
architectures = ["x86_64", "aarch64"]
```

---

## Version Migration Notes

### From Python CLI (v3.0) to Rust CLI (v3.1+)

Key changes in CCGO.toml format:

1. **Section names:**
   - `[project]` is now `[package]` (but `[project]` still works as alias)

2. **New features:**
   - `[workspace]` section for multi-package projects
   - `[features]` section for conditional compilation
   - `[[bin]]` and `[[example]]` sections for executables
   - `workspace = true` for dependency inheritance
   - `resolver` field in workspace config

3. **Dependencies:**
   - Changed from dictionary format to array of tables (`[[dependencies]]`)
   - Added support for workspace dependency inheritance
   - Added `optional`, `features`, `default_features` fields

4. **Platform configurations:**
   - Moved to `[platforms.<name>]` sections
   - Simplified field names (e.g., `min_sdk` instead of Android-specific names)

5. **Build configuration:**
   - New `[build]` section
   - Added `parallel`, `jobs`, `submodule_deps` fields

---

## Best Practices

1. **Version Control**: Always commit `CCGO.toml` to version control
2. **Semantic Versioning**: Use proper semver (MAJOR.MINOR.PATCH)
3. **Package Names**: Use lowercase names without spaces (kebab-case recommended)
4. **Workspace Organization**:
   - Use workspaces for related packages
   - Define shared dependencies in workspace root
   - Use `resolver = "2"` for better dependency resolution
5. **Features**:
   - Keep `default` features minimal
   - Use features to make dependencies optional
   - Document features in README
6. **Platform Support**: Only configure platforms you actually support
7. **Dependencies**:
   - Pin versions in production (`tag` or `rev` for git dependencies)
   - Use `^` version ranges for flexibility in development
   - Use lockfile (`CCGO.toml.lock`) for reproducible builds

---

## Validation

CCGO validates `CCGO.toml` when parsing. Common errors:

| Error | Cause | Solution |
|-------|-------|----------|
| "must contain either [package] or [workspace]" | Missing both sections | Add at least one section |
| "Invalid version requirement" | Bad semver syntax | Fix version string (e.g., "^1.0.0") |
| "Unknown feature" | Requesting undefined feature | Check `[features]` section |
| "workspace dependency not found" | `workspace = true` but not in workspace deps | Add to `[[workspace.dependencies]]` |
| "Failed to parse CCGO.toml" | TOML syntax error | Check TOML syntax |

---

## See Also

- [CLI Reference](cli.md) - CCGO command-line interface
- [Quick Start](../getting-started/quickstart.md) - Quick start guide
- [Project Structure](../getting-started/project-structure.md) - Project organization
- [Dependency Management](../features/dependency-management.md) - Managing dependencies
