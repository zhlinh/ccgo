# CCGO.toml Reference

Complete configuration reference for CCGO projects.

## Overview

`CCGO.toml` is the manifest file that defines your project's metadata, dependencies, build settings, and platform-specific configurations. It is written in [TOML](https://toml.io/) format.

## File Location

The `CCGO.toml` file must be located at the root of your project directory.

## Basic Structure

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[build]
cpp_standard = "17"

[android]
min_sdk_version = 21
```

---

## [package]

Defines package metadata.

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Package name (lowercase, alphanumeric, hyphens) |
| `version` | string | Semantic version (e.g., "1.0.0") |

### Optional Fields

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `authors` | array of strings | Package authors | `[]` |
| `description` | string | Short description | `""` |
| `license` | string | License identifier (e.g., "MIT", "Apache-2.0") | `""` |
| `repository` | string | Source repository URL | `""` |
| `homepage` | string | Project homepage URL | `""` |
| `documentation` | string | Documentation URL | `""` |
| `keywords` | array of strings | Search keywords | `[]` |
| `categories` | array of strings | Package categories | `[]` |
| `readme` | string | Path to README file | `"README.md"` |

### Example

```toml
[package]
name = "awesome-cpp-lib"
version = "2.1.3"
authors = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
description = "An awesome C++ library"
license = "MIT"
repository = "https://github.com/user/awesome-cpp-lib"
homepage = "https://awesome-cpp-lib.dev"
keywords = ["networking", "async", "performance"]
categories = ["network-programming", "asynchronous"]
```

---

## [library]

Defines library build configuration.

### Fields

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `type` | string | Library type: `"static"`, `"shared"`, `"both"` | `"both"` |
| `namespace` | string | C++ namespace for library | Package name |
| `output_name` | string | Custom library output name | Package name |
| `crate_type` | array of strings | Output types (for compatibility) | `["staticlib", "cdylib"]` |

### Example

```toml
[library]
type = "both"              # Build both static and shared libraries
namespace = "mylib"        # C++ namespace
output_name = "mylib-cpp"  # Output files: libmylib-cpp.a, libmylib-cpp.so
```

---

## [dependencies]

Defines project dependencies.

### Dependency Sources

#### Git Repository

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", branch = "master" }
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280" }
```

**Fields:**
- `git` (required): Git repository URL
- `tag`: Git tag (mutually exclusive with `branch` and `rev`)
- `branch`: Git branch (mutually exclusive with `tag` and `rev`)
- `rev`: Git commit hash (mutually exclusive with `tag` and `branch`)

#### Local Path

```toml
[dependencies]
mylib = { path = "../mylib" }
utils = { path = "./libs/utils" }
```

**Fields:**
- `path` (required): Relative or absolute path to dependency

#### Registry (Future)

```toml
[dependencies]
fmt = "10.1.1"              # Exact version
spdlog = "^1.12.0"          # Compatible version (>=1.12.0, <2.0.0)
boost = "~1.80"             # Minor version (>=1.80.0, <1.81.0)
```

### Version Requirements

| Syntax | Meaning | Example |
|--------|---------|---------|
| `"1.2.3"` | Exact version | `"1.2.3"` matches only 1.2.3 |
| `"^1.2.3"` | Compatible version | `"^1.2.3"` matches >=1.2.3, <2.0.0 |
| `"~1.2.3"` | Minor version | `"~1.2.3"` matches >=1.2.3, <1.3.0 |
| `">=1.2.3"` | Greater or equal | `">=1.2.3"` matches >=1.2.3 |
| `">1.2.3"` | Greater than | `">1.2.3"` matches >1.2.3 |
| `"<=1.2.3"` | Less or equal | `"<=1.2.3"` matches <=1.2.3 |
| `"<1.2.3"` | Less than | `"<1.2.3"` matches <1.2.3 |

### Optional Dependencies

```toml
[dependencies]
required = { git = "https://github.com/user/required.git", tag = "v1.0.0" }

[dependencies.optional]
networking = { git = "https://github.com/user/network.git", tag = "v2.0.0" }
database = { git = "https://github.com/user/db.git", tag = "v3.0.0" }
```

Enable optional dependencies with features:

```toml
[features]
network = ["networking"]
db = ["database"]
full = ["network", "db"]
```

### Platform-Specific Dependencies

```toml
[dependencies]
common = { git = "https://github.com/user/common.git", tag = "v1.0.0" }

[target.'cfg(target_os = "android")'.dependencies]
android-specific = { path = "./android-lib" }

[target.'cfg(target_os = "ios")'.dependencies]
ios-specific = { path = "./ios-lib" }
```

---

## [build]

Defines build configuration.

### Fields

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `cpp_standard` | string | C++ standard: "11", "14", "17", "20", "23" | `"17"` |
| `cmake_minimum_version` | string | Minimum CMake version | `"3.18"` |
| `compile_flags` | array of strings | Additional compiler flags | `[]` |
| `link_flags` | array of strings | Additional linker flags | `[]` |
| `definitions` | table | Preprocessor definitions | `{}` |
| `include_dirs` | array of strings | Additional include directories | `[]` |
| `link_dirs` | array of strings | Additional library search paths | `[]` |
| `system_libs` | array of strings | System libraries to link | `[]` |

### Example

```toml
[build]
cpp_standard = "20"
cmake_minimum_version = "3.20"
compile_flags = ["-Wall", "-Wextra", "-Werror"]
link_flags = ["-flto"]

[build.definitions]
DEBUG_MODE = "1"
APP_VERSION = "\"1.0.0\""
ENABLE_LOGGING = true

[build]
include_dirs = ["third_party/include"]
link_dirs = ["third_party/lib"]
system_libs = ["pthread", "dl"]
```

---

## Platform Configuration

### [android]

Android-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `min_sdk_version` | integer | Minimum Android API level | `21` |
| `target_sdk_version` | integer | Target Android API level | `33` |
| `ndk_version` | string | NDK version | Latest |
| `stl` | string | STL type: "c++_static", "c++_shared" | `"c++_static"` |
| `architectures` | array of strings | Target architectures | All |

```toml
[android]
min_sdk_version = 21
target_sdk_version = 33
ndk_version = "25.2.9519653"
stl = "c++_static"
architectures = ["arm64-v8a", "armeabi-v7a"]
```

### [ios]

iOS-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `min_deployment_target` | string | Minimum iOS version | `"12.0"` |
| `enable_bitcode` | boolean | Enable bitcode | `false` |
| `architectures` | array of strings | Target architectures | All |

```toml
[ios]
min_deployment_target = "13.0"
enable_bitcode = false
architectures = ["arm64"]
```

### [macos]

macOS-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `min_deployment_target` | string | Minimum macOS version | `"10.15"` |
| `architectures` | array of strings | Target architectures | `["x86_64", "arm64"]` |

```toml
[macos]
min_deployment_target = "11.0"
architectures = ["arm64", "x86_64"]  # Universal binary
```

### [windows]

Windows-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `toolchain` | string | Toolchain: "msvc", "mingw", "auto" | `"auto"` |
| `msvc_runtime` | string | MSVC runtime: "static", "dynamic" | `"dynamic"` |
| `architectures` | array of strings | Target architectures | `["x86_64"]` |

```toml
[windows]
toolchain = "msvc"
msvc_runtime = "static"
architectures = ["x86_64", "x86"]
```

### [linux]

Linux-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `architectures` | array of strings | Target architectures | `["x86_64"]` |
| `system_deps` | array of strings | System dependencies | `[]` |

```toml
[linux]
architectures = ["x86_64", "aarch64"]
system_deps = ["libssl-dev", "libcurl4-openssl-dev"]
```

### [ohos]

OpenHarmony-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `min_api_version` | integer | Minimum API version | `9` |
| `target_api_version` | integer | Target API version | `10` |
| `architectures` | array of strings | Target architectures | All |

```toml
[ohos]
min_api_version = 9
target_api_version = 10
architectures = ["arm64-v8a", "armeabi-v7a"]
```

### [watchos]

watchOS-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `min_deployment_target` | string | Minimum watchOS version | `"5.0"` |
| `architectures` | array of strings | Target architectures | All |

```toml
[watchos]
min_deployment_target = "6.0"
architectures = ["armv7k", "arm64_32"]
```

### [tvos]

tvOS-specific configuration.

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `min_deployment_target` | string | Minimum tvOS version | `"12.0"` |
| `architectures` | array of strings | Target architectures | All |

```toml
[tvos]
min_deployment_target = "13.0"
architectures = ["arm64"]
```

---

## [features]

Defines conditional compilation features.

```toml
[features]
default = ["feature1"]          # Default features
feature1 = []                   # Simple feature
feature2 = ["dependency1"]      # Enables optional dependency
full = ["feature1", "feature2"] # Composite feature
```

### Example

```toml
[dependencies.optional]
networking = { git = "https://github.com/user/network.git", tag = "v1.0.0" }
logging = { git = "https://github.com/user/logging.git", tag = "v2.0.0" }

[features]
default = ["basic"]
basic = []
network = ["networking"]
debug = ["logging"]
full = ["basic", "network", "debug"]
```

Enable features during build:

```bash
ccgo build --features network,debug
ccgo build --all-features
ccgo build --no-default-features
```

---

## [examples]

Defines example programs.

```toml
[[examples]]
name = "basic"
path = "examples/basic.cpp"

[[examples]]
name = "advanced"
path = "examples/advanced.cpp"
required_features = ["network"]
```

Build and run examples:

```bash
ccgo run basic
ccgo run advanced --features network
```

---

## [bins]

Defines binary targets.

```toml
[[bins]]
name = "mytool"
path = "src/bin/mytool.cpp"

[[bins]]
name = "myapp"
path = "src/bin/myapp.cpp"
required_features = ["full"]
```

Build and run binaries:

```bash
ccgo run mytool --bin
ccgo build --bin myapp --features full
```

---

## Complete Example

```toml
[package]
name = "mylib"
version = "1.0.0"
authors = ["Developer <dev@example.com>"]
description = "A cross-platform C++ library"
license = "MIT"
repository = "https://github.com/user/mylib"
homepage = "https://mylib.dev"
keywords = ["cpp", "cross-platform"]
categories = ["library"]

[library]
type = "both"
namespace = "mylib"
output_name = "mylib"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[dependencies.optional]
networking = { git = "https://github.com/user/network.git", tag = "v1.0.0" }

[build]
cpp_standard = "17"
cmake_minimum_version = "3.18"
compile_flags = ["-Wall", "-Wextra"]

[build.definitions]
APP_VERSION = "\"1.0.0\""

[features]
default = ["basic"]
basic = []
network = ["networking"]
full = ["basic", "network"]

[android]
min_sdk_version = 21
target_sdk_version = 33
architectures = ["arm64-v8a", "armeabi-v7a"]

[ios]
min_deployment_target = "12.0"
enable_bitcode = false

[windows]
toolchain = "auto"
msvc_runtime = "dynamic"

[[examples]]
name = "basic"
path = "examples/basic.cpp"

[[bins]]
name = "mytool"
path = "src/bin/mytool.cpp"
```

---

## Schema Validation

CCGO validates `CCGO.toml` on every command. Common errors:

### Invalid TOML Syntax

```toml
# ERROR: Missing closing quote
name = "mylib

# ERROR: Invalid key format
my-key = value
```

### Missing Required Fields

```toml
# ERROR: Missing 'name' field
[package]
version = "1.0.0"
```

### Invalid Version Format

```toml
[package]
name = "mylib"
version = "1.0"        # ERROR: Must be semantic version (1.0.0)
```

### Conflicting Git References

```toml
[dependencies]
# ERROR: Cannot specify both 'tag' and 'branch'
lib = { git = "https://...", tag = "v1.0.0", branch = "main" }
```

---

## Environment Variable Expansion

CCGO supports environment variable expansion in string values:

```toml
[package]
version = "${VERSION:-1.0.0}"  # Default to "1.0.0" if VERSION not set

[dependencies]
mylib = { path = "${MYLIB_PATH:-../mylib}" }
```

Syntax:
- `${VAR}`: Expand variable (error if not set)
- `${VAR:-default}`: Expand with default value
- `$$`: Literal `$` character

---

## Best Practices

### Versioning

- Use semantic versioning (MAJOR.MINOR.PATCH)
- Update version before tagging: `ccgo tag`
- Keep CHANGELOG.md synchronized with versions

### Dependencies

- Pin dependencies to specific tags/revisions for reproducibility
- Use `CCGO.lock` for exact dependency resolution
- Document dependency requirements in README

### Platform Configuration

- Set reasonable minimum versions for maximum compatibility
- Test on minimum supported platform versions
- Document platform-specific requirements

### Features

- Use features for optional functionality
- Keep default features minimal
- Document features in README

### Build Settings

- Match cpp_standard with dependencies
- Use warning flags (`-Wall -Wextra`) in development
- Avoid platform-specific flags in main config

---

## Migration Guide

### From CMakeLists.txt

```cmake
# CMakeLists.txt
project(mylib VERSION 1.0.0)
set(CMAKE_CXX_STANDARD 17)
find_package(spdlog REQUIRED)
```

Becomes:

```toml
# CCGO.toml
[package]
name = "mylib"
version = "1.0.0"

[build]
cpp_standard = "17"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

### From Conan

```ini
# conanfile.txt
[requires]
spdlog/1.12.0
fmt/10.1.1

[options]
spdlog:shared=False
```

Becomes:

```toml
# CCGO.toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[library]
type = "static"
```

---

## See Also

- [CLI Reference](cli.md)
- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Publishing](../features/publishing.md)
