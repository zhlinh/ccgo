# Migration from Conan to CCGO

> Version: v3.0.10 | Updated: 2026-01-21

## Overview

This guide helps you migrate C++ projects from [Conan](https://conan.io/) to CCGO. Both are C++ package managers, but CCGO provides a more integrated cross-platform build system optimized for mobile and embedded platforms.

### Why Migrate to CCGO?

| Feature | Conan | CCGO |
|---------|-------|------|
| **Cross-Platform Builds** | Manual per-platform setup | Single command for 8+ platforms |
| **Mobile Focus** | Android/iOS require complex setup | First-class Android, iOS, OpenHarmony support |
| **Configuration** | conanfile.py/txt + CMake | Unified CCGO.toml |
| **Publishing** | Conan Center, custom servers | Maven, CocoaPods, SPM, OHPM, Conan |
| **Docker Builds** | Manual setup | Built-in universal cross-compilation |
| **Dependency Locking** | conan.lock | CCGO.lock |
| **Gradual Migration** | N/A | Can use Conan packages as CCGO deps |

### Migration Effort

**Typical small project**: 1-2 hours
**Medium project with 10+ deps**: 4-8 hours
**Large project with custom Conan recipes**: 1-2 days

---

## Quick Comparison

### Conan vs CCGO Concepts

| Conan Concept | CCGO Equivalent | Notes |
|---------------|-----------------|-------|
| `conanfile.txt` | `CCGO.toml` | TOML format for dependencies |
| `conanfile.py` | `CCGO.toml` + CMakeLists.txt | Build logic moves to CMake |
| `conan install` | `ccgo install` | Install dependencies |
| `conan create` | `ccgo build` | Build package |
| `conan upload` | `ccgo publish` | Publish to registry |
| Conan Center | CCGO Registry (planned) | Currently uses Git deps |
| `requires` | `[dependencies]` | Dependency declarations |
| `tool_requires` | N/A | Use system tools or Docker |
| Profile | Platform flags | `--arch`, `--toolchain`, etc. |
| Generator | CMake integration | Direct CMake module |
| Package recipe | CCGO.toml + CMakeLists.txt | Simpler declarative format |

---

## Migration Paths

### Path 1: Simple Dependencies Only (conanfile.txt)

**Best for**: Projects that only consume packages, no custom recipes

**Steps**:
1. Convert `conanfile.txt` → `CCGO.toml`
2. Update CMakeLists.txt includes
3. Test builds

**Time**: 30 minutes - 2 hours

---

### Path 2: Custom Package Recipe (conanfile.py)

**Best for**: Projects with custom Conan packages

**Steps**:
1. Extract dependency list → `CCGO.toml` `[dependencies]`
2. Convert build logic → `CMakeLists.txt`
3. Update publishing config → `CCGO.toml` `[publish]`
4. Test builds and publishing

**Time**: 2-8 hours

---

### Path 3: Hybrid Approach (Gradual Migration)

**Best for**: Large projects, minimal disruption

**Steps**:
1. Keep Conan for most deps
2. Use CCGO for cross-platform builds
3. Gradually replace Conan deps with CCGO/Git deps
4. Remove Conan when ready

**Time**: Spread over weeks/months

---

## Step-by-Step Migration

### Step 1: Analyze Current Conan Setup

#### Identify Dependency Sources

```bash
# List all Conan dependencies
conan info . --only requires

# Check which deps are from Conan Center
conan search <package> --remote=conancenter

# Identify custom recipes (local or private server)
conan search --remote=all
```

**Categorize dependencies**:
- **Conan Center**: May have Git alternatives
- **Custom recipes**: Need to port or keep as Conan
- **System libraries**: Can use system or build from source

---

### Step 2: Create CCGO.toml

#### From conanfile.txt

**Before (conanfile.txt)**:
```ini
[requires]
fmt/10.1.1
spdlog/1.12.0
boost/1.82.0

[generators]
cmake_find_package
cmake_paths

[options]
fmt:shared=False
boost:shared=False
```

**After (CCGO.toml)**:
```toml
[package]
name = "myproject"
version = "1.0.0"

[dependencies]
# Option 1: Use Conan packages (hybrid mode)
fmt = { version = "10.1.1", source = "conan" }
spdlog = { version = "1.12.0", source = "conan" }

# Option 2: Use Git repositories
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }
spdlog = { git = "https://github.com/gabime/spdlog", tag = "v1.12.0" }

# Option 3: Mix both
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }
boost = { version = "1.82.0", source = "conan" }  # Keep Conan for complex deps

[build]
cmake_version = "3.22.1"
```

**Advantages of Git dependencies**:
- ✅ No Conan server needed
- ✅ Lock exact commit/tag
- ✅ Build from source with your exact flags
- ❌ Slower first build (no binary cache)

---

#### From conanfile.py

**Before (conanfile.py)**:
```python
from conan import ConanFile
from conan.tools.cmake import CMake, cmake_layout

class MyProjectConan(ConanFile):
    name = "myproject"
    version = "1.0.0"
    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False]}
    default_options = {"shared": False}
    exports_sources = "CMakeLists.txt", "src/*", "include/*"

    def requirements(self):
        self.requires("fmt/10.1.1")
        if self.options.shared:
            self.requires("spdlog/1.12.0")

    def layout(self):
        cmake_layout(self)

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        cmake = CMake(self)
        cmake.install()
```

**After (CCGO.toml)**:
```toml
[package]
name = "myproject"
version = "1.0.0"
description = "My C++ project"
license = "MIT"

[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

[dependencies.spdlog]
git = "https://github.com/gabime/spdlog"
tag = "v1.12.0"
optional = true

[features]
default = []
with-logging = ["spdlog"]  # Conditional dependency

[build]
cmake_version = "3.22.1"
link_type = "static"  # or "shared"

[android]
min_sdk = 21
compile_sdk = 34
default_archs = ["armeabi-v7a", "arm64-v8a", "x86_64"]

[ios]
deployment_target = "13.0"

[publish.maven]
group_id = "com.example"
artifact_id = "myproject"
```

**Note**: Build logic (CMake configuration/build/install) moves to your `CMakeLists.txt` - see Step 3.

---

### Step 3: Update CMake Integration

#### Conan CMake Integration

**Before (with Conan)**:
```cmake
cmake_minimum_required(VERSION 3.15)
project(MyProject)

# Conan integration (multiple approaches)

# Option 1: cmake-conan
include(${CMAKE_BINARY_DIR}/conan.cmake)
conan_cmake_configure(REQUIRES fmt/10.1.1
                      GENERATORS cmake_find_package)
conan_cmake_install(...)

# Option 2: CMakeDeps + CMakeToolchain generators
find_package(fmt REQUIRED)
find_package(spdlog REQUIRED)

add_executable(myapp main.cpp)
target_link_libraries(myapp fmt::fmt spdlog::spdlog)
```

---

#### CCGO CMake Integration

**After (with CCGO)**:
```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# CCGO integration
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# Collect sources (CCGO helper)
add_sub_layer_sources_recursively(MYAPP_SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/src)

# Create library/executable
add_library(myproject STATIC ${MYAPP_SOURCES})

# Add all dependency include paths
ccgo_add_dependencies(myproject)

# Link specific dependencies (CCGO will find them in installed deps)
ccgo_link_dependency(myproject fmt fmt)
ccgo_link_dependency(myproject spdlog spdlog)

# Or use CMake's find_package if dependency provides it
find_package(fmt REQUIRED)
find_package(spdlog REQUIRED)
target_link_libraries(myproject PRIVATE fmt::fmt spdlog::spdlog)
```

**Key Differences**:
- `${CCGO_CMAKE_DIR}` is set by ccgo build system
- `ccgo_add_dependencies()` adds all dependency include paths
- `ccgo_link_dependency()` finds and links specific libraries
- Simpler, less boilerplate than Conan

---

### Step 4: Install Dependencies

```bash
# Conan
conan install . --output-folder=build --build=missing

# CCGO
ccgo install
# Reads CCGO.toml, installs to .ccgo/deps/
# Generates CCGO.lock for version locking
```

**CCGO Dependency Installation**:
- Git deps: Cloned to `.ccgo/deps/<name>`
- Conan deps (hybrid mode): Uses `conan install` internally
- Lockfile: `CCGO.lock` ensures reproducible builds

---

### Step 5: Update Build Commands

#### Platform Builds

**Conan**:
```bash
# Android (complex setup)
conan install . --profile=android-armv8 --build=missing
conan build .

# iOS (complex setup)
conan install . --profile=ios-armv8 --build=missing
conan build .
```

**CCGO**:
```bash
# Android
ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64

# iOS
ccgo build ios

# macOS
ccgo build macos

# All platforms with Docker (any host OS!)
ccgo build android --docker
ccgo build ios --docker
ccgo build windows --docker
```

**CCGO Advantages**:
- ✅ Single command per platform
- ✅ Automatic toolchain setup
- ✅ Docker-based universal cross-compilation
- ✅ Parallel architecture builds

---

### Step 6: Update Publishing

#### Publishing to Maven (Android)

**Conan**:
```bash
# Custom conanfile.py deploy() method
conan create . --profile=android-armv8
conan upload myproject/1.0.0 --remote=myremote
```

**CCGO**:
```bash
# Configure once in CCGO.toml
[publish.maven]
group_id = "com.example"
artifact_id = "myproject"

# Publish to Maven Central
ccgo publish android --registry official

# Publish to custom Maven
ccgo publish android --registry private --url https://maven.example.com
```

---

#### Publishing to CocoaPods (iOS)

**Conan**: Not directly supported, requires custom scripts

**CCGO**:
```bash
# Configure once in CCGO.toml
[publish.cocoapods]
summary = "My C++ library"
homepage = "https://github.com/user/myproject"

# Publish
ccgo publish apple --manager cocoapods
```

---

## Common Migration Scenarios

### Scenario 1: Header-Only Library (e.g., {fmt})

**Conan conanfile.py**:
```python
class FmtConan(ConanFile):
    name = "fmt"
    version = "10.1.1"
    # Header-only, no build()

    def package(self):
        self.copy("*.h", dst="include", src="include")
```

**CCGO CCGO.toml**:
```toml
[package]
name = "fmt-wrapper"
version = "10.1.1"

[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

# CMakeLists.txt handles include path setup
```

**CMakeLists.txt**:
```cmake
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

add_library(fmt INTERFACE)
ccgo_add_dependencies(fmt)  # Adds fmt include paths

# Or use CMake's find_package
add_subdirectory(.ccgo/deps/fmt)
target_link_libraries(myapp PRIVATE fmt::fmt-header-only)
```

---

### Scenario 2: Library with Build Options

**Conan conanfile.py**:
```python
class MyLibConan(ConanFile):
    options = {
        "shared": [True, False],
        "with_ssl": [True, False]
    }
    default_options = {
        "shared": False,
        "with_ssl": True
    }

    def requirements(self):
        if self.options.with_ssl:
            self.requires("openssl/3.1.0")
```

**CCGO CCGO.toml**:
```toml
[package]
name = "mylib"

[dependencies]
openssl = { git = "https://github.com/openssl/openssl", tag = "openssl-3.1.0", optional = true }

[features]
default = ["ssl"]
ssl = ["openssl"]

[build]
link_type = "static"  # or "shared"
```

**Build with/without SSL**:
```bash
# With SSL (default)
ccgo build android

# Without SSL
ccgo build android --no-default-features
```

---

### Scenario 3: Multi-Package Workspace

**Conan** (separate conanfile.py per package):
```
myworkspace/
├── core/conanfile.py      # Depends on nothing
├── utils/conanfile.py     # Depends on core
└── app/conanfile.py       # Depends on utils
```

**CCGO** (single CCGO.toml at root):
```toml
[workspace]
members = ["core", "utils", "app"]
resolver = "2"

# Workspace-level dependency (inherited by all members)
[workspace.dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }
```

**core/CCGO.toml**:
```toml
[package]
name = "core"
version = "1.0.0"

[dependencies]
fmt = { workspace = true }  # Inherit from workspace
```

**utils/CCGO.toml**:
```toml
[package]
name = "utils"
version = "1.0.0"

[dependencies]
core = { path = "../core" }  # Local dependency
fmt = { workspace = true }
```

---

### Scenario 4: Private Conan Server

**Conan**:
```bash
# Configure remote
conan remote add mycompany https://conan.example.com
conan remote login mycompany admin -p password

# Use packages
conan install . --remote=mycompany
```

**CCGO Hybrid Mode** (keep Conan for private deps):
```toml
[dependencies]
# Public packages: use Git
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

# Private packages: use Conan
internal-lib = { version = "2.3.0", source = "conan", remote = "mycompany" }
```

**Or Migrate to Git** (recommended):
```toml
[dependencies]
# Host private packages on Git (GitHub/GitLab/Bitbucket)
internal-lib = { git = "https://github.com/mycompany/internal-lib", tag = "v2.3.0" }
```

---

## Dependency Mapping

### Popular Conan Packages → CCGO Equivalents

| Conan Package | CCGO Recommended Approach |
|---------------|---------------------------|
| `fmt` | `{ git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }` |
| `spdlog` | `{ git = "https://github.com/gabime/spdlog", tag = "v1.12.0" }` |
| `catch2` | `{ git = "https://github.com/catchorg/Catch2", tag = "v3.4.0" }` |
| `gtest` | `{ git = "https://github.com/google/googletest", tag = "v1.14.0" }` |
| `nlohmann_json` | `{ git = "https://github.com/nlohmann/json", tag = "v3.11.2" }` |
| `boost` | Keep Conan or use system package (too complex for Git) |
| `openssl` | Keep Conan or use system package (security updates) |
| `protobuf` | `{ git = "https://github.com/protocolbuffers/protobuf", tag = "v23.4" }` |
| `grpc` | Keep Conan (complex build) or use system package |
| `sqlite3` | `{ git = "https://github.com/sqlite/sqlite", tag = "version-3.42.0" }` |

---

## Troubleshooting

### Issue: Missing Conan Package in CCGO

**Problem**: Package available in Conan Center, but no Git repository

**Solutions**:
1. **Hybrid Mode**: Keep using Conan for that package
   ```toml
   [dependencies]
   mypackage = { version = "1.0.0", source = "conan" }
   ```

2. **Find Git Source**: Search GitHub/GitLab for upstream
   ```bash
   # Example: protobuf is on GitHub
   https://github.com/protocolbuffers/protobuf
   ```

3. **Vendor the Code**: Copy source into your project
   ```toml
   [dependencies]
   myvendored = { path = "third_party/myvendored" }
   ```

---

### Issue: Complex Build Recipe in conanfile.py

**Problem**: Custom build logic in `build()` method

**Solution**: Move logic to `CMakeLists.txt`

**Conan conanfile.py**:
```python
def build(self):
    cmake = CMake(self)
    cmake.definitions["CUSTOM_OPTION"] = "ON"
    cmake.definitions["BUILD_SHARED"] = self.options.shared
    cmake.configure(source_folder="subfolder")
    cmake.build(target="mylib")
```

**CCGO CMakeLists.txt**:
```cmake
cmake_minimum_required(VERSION 3.18)

# Set options directly
option(CUSTOM_OPTION "Custom option" ON)
option(BUILD_SHARED_LIBS "Build shared" OFF)

# Configure subfolder
add_subdirectory(subfolder)

# Build target
add_library(mylib ...)
```

---

### Issue: Conan Generators Not Available

**Problem**: `cmake_find_package` generator missing in CCGO

**Solution**: Use CCGO's CMake integration

**Before**:
```cmake
find_package(fmt REQUIRED)  # Generated by Conan
```

**After**:
```cmake
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)
ccgo_add_dependencies(myapp)
ccgo_link_dependency(myapp fmt fmt)
```

---

### Issue: Profile Configuration

**Problem**: Conan profiles specify compiler, settings

**Conan ~/.conan/profiles/android-armv8**:
```ini
[settings]
os=Android
os.api_level=21
arch=armv8
compiler=clang
compiler.version=14
compiler.libcxx=c++_shared
build_type=Release
```

**CCGO**: Platform settings in CCGO.toml, architecture in command

**CCGO.toml**:
```toml
[android]
min_sdk = 21
compile_sdk = 34
stl = "c++_shared"
```

**Command**:
```bash
ccgo build android --arch arm64-v8a --config release
```

---

## Best Practices

### 1. Gradual Migration Strategy

**✅ Do**: Start hybrid (Conan + CCGO)
```toml
[dependencies]
# Simple deps: migrate to Git
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

# Complex deps: keep Conan temporarily
boost = { version = "1.82.0", source = "conan" }
```

**✅ Do**: Migrate package by package over sprints

---

### 2. Lock Dependencies

**✅ Do**: Commit CCGO.lock to version control
```bash
ccgo install       # Generates CCGO.lock
git add CCGO.lock  # Lock exact versions
git commit -m "Lock dependency versions"
```

---

### 3. Test Cross-Platform Builds Early

**✅ Do**: Verify all target platforms after migration
```bash
ccgo build android --arch arm64-v8a
ccgo build ios
ccgo build macos
ccgo build windows --docker  # Test on macOS/Linux
```

---

### 4. Document Custom Patches

**✅ Do**: Document any Conan package patches in migration notes
```toml
[dependencies]
mypackage = { git = "https://github.com/user/mypackage-fork", branch = "custom-fixes" }
# Custom fork with Android NDK r25 fixes
```

---

### 5. Use Docker for Reproducibility

**✅ Do**: Use Docker builds for CI/CD
```yaml
# .github/workflows/build.yml
- name: Build for all platforms
  run: |
    ccgo build android --docker --arch arm64-v8a,armeabi-v7a,x86_64
    ccgo build ios --docker
    ccgo build macos --docker
```

---

## Migration Checklist

Use this checklist to track your migration progress:

### Pre-Migration

- [ ] Audit all Conan dependencies
- [ ] Identify custom Conan recipes
- [ ] Check platform-specific build configs
- [ ] Document current build workflow
- [ ] Set up test environment

### Migration

- [ ] Create `CCGO.toml` from `conanfile.txt`/`conanfile.py`
- [ ] Convert dependency declarations
- [ ] Update `CMakeLists.txt` includes
- [ ] Test dependency installation (`ccgo install`)
- [ ] Verify builds on all platforms
- [ ] Test publishing workflow (if applicable)
- [ ] Update CI/CD pipelines
- [ ] Update developer documentation

### Post-Migration

- [ ] Remove Conan files (`conanfile.*`, `conan.lock`)
- [ ] Remove Conan-related CI/CD steps
- [ ] Archive Conan configuration for reference
- [ ] Train team on CCGO workflow
- [ ] Monitor build performance
- [ ] Gather team feedback

---

## Performance Comparison

### Build Times (Example Project: 10 dependencies)

| Operation | Conan | CCGO | Notes |
|-----------|-------|------|-------|
| Install deps (first) | 5-10 min | 3-7 min | CCGO builds from source |
| Install deps (cached) | 30s | 10s | CCGO uses Git cache |
| Android build | 2 min | 90s | CCGO parallel archs |
| iOS build | 3 min | 2 min | CCGO optimized toolchain |
| Cross-platform (4 platforms) | 15 min | 6 min | CCGO Docker parallel |

*Times vary by project size and hardware*

---

## Additional Resources

- [Conan Official Docs](https://docs.conan.io/2/)
- [CCGO CLI Reference](../reference/cli.md)
- [CCGO.toml Configuration](../reference/config.md)
- [CMake Integration Guide](cmake-integration.md)
- [CCGO Dependency Management](../guides/dependencies.md)

**Community Support**:
- [CCGO GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [CCGO Discord](https://discord.gg/ccgo) (coming soon)

---

## FAQ

### Q: Can I keep using Conan alongside CCGO?

**A**: Yes! CCGO supports hybrid mode:
```toml
[dependencies]
conan-package = { version = "1.0.0", source = "conan" }
git-package = { git = "https://github.com/user/package" }
```

---

### Q: What about binary packages? Conan has prebuilt binaries.

**A**: CCGO currently builds from source for maximum flexibility. Benefits:
- ✅ Exact compiler/flag control
- ✅ No ABI compatibility issues
- ✅ Security (build yourself)
- ❌ Slower first build (mitigated by caching)

Future: Binary cache support planned for CCGO Registry.

---

### Q: Can I publish to Conan Center from CCGO?

**A**: Not directly. To publish to Conan Center:
1. Keep a `conanfile.py` for publishing
2. Use CCGO for development builds
3. Export to Conan: `conan create . --profile=...`

Or migrate consumers to CCGO gradually.

---

### Q: How do I handle Conan options in CCGO?

**A**: Use CCGO features:

**Conan**:
```python
options = {"shared": [True, False], "with_ssl": [True, False]}
```

**CCGO**:
```toml
[features]
shared = []
ssl = ["openssl"]
```

```bash
ccgo build --features shared,ssl
```

---

### Q: What about `tool_requires` (build tools)?

**A**: CCGO uses system tools or Docker:

**Conan**:
```python
tool_requires = ["cmake/3.24.0", "ninja/1.11.1"]
```

**CCGO**:
```bash
# Install tools via system package manager
brew install cmake ninja  # macOS
apt install cmake ninja-build  # Ubuntu

# Or use Docker (tools included)
ccgo build --docker
```

---

## Summary

Migrating from Conan to CCGO simplifies cross-platform C++ development:

**Key Benefits**:
1. ✅ **Unified Configuration**: Single `CCGO.toml` vs multiple files
2. ✅ **Mobile-First**: Android, iOS, OpenHarmony out-of-the-box
3. ✅ **Simpler CMake**: Less boilerplate, clearer dependencies
4. ✅ **Docker Integration**: Universal cross-compilation
5. ✅ **Modern Publishing**: Maven, CocoaPods, SPM support

**Migration Effort**: Typically 1-8 hours for most projects

**Recommendation**: Start with hybrid mode, migrate incrementally

---

**Sources**:
- [GitHub - conan-io/conan: Conan - The open-source C and C++ package manager](https://github.com/conan-io/conan)
- [Conan 2 - C and C++ Package Manager Documentation](https://docs.conan.io/2/)
- [GitHub - conan-io/cmake-conan: CMake wrapper for conan C and C++ package manager](https://github.com/conan-io/cmake-conan)

---

*This guide is part of the CCGO documentation. For questions or improvements, open an issue on [GitHub](https://github.com/zhlinh/ccgo/issues).*
