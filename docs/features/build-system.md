# Build System

Comprehensive guide to CCGO's cross-platform build system.

## Overview

CCGO provides a unified build system that:

- **Multi-platform support**: Build for Android, iOS, macOS, Windows, Linux, OpenHarmony, watchOS, tvOS
- **Architecture flexibility**: Single or multiple architectures per platform
- **Build type control**: Debug and release builds
- **Link type options**: Static, shared, or both library types
- **Toolchain selection**: Platform-specific toolchain choices (e.g., MSVC vs MinGW)
- **Docker integration**: Universal cross-compilation without local toolchains
- **Incremental builds**: Fast rebuilds with CMake caching
- **Unified output format**: Consistent ZIP archive structure across platforms

## Build Architecture

### High-Level Flow

```
User Command (ccgo build)
    ↓
CLI Parser (cli.py)
    ↓
Build Command (commands/build.py)
    ↓
Platform Build Script (build_scripts/build_<platform>.py)
    ↓
CMake Configuration (build_scripts/cmake/)
    ↓
Native Toolchain (NDK/Xcode/MSVC/GCC/etc.)
    ↓
Archive & Package (ZIP with metadata)
```

### Key Components

**1. CLI Layer (`ccgo/cli.py`, `ccgo/commands/build.py`)**
- Parses user commands and options
- Validates platform and architecture combinations
- Dispatches to platform-specific build scripts

**2. Build Scripts (`ccgo/build_scripts/build_*.py`)**
- Platform-specific build logic
- CMake invocation with correct toolchain files
- Artifact collection and packaging
- Centralized in ccgo package (not copied to projects)

**3. CMake Configuration (`ccgo/build_scripts/cmake/`)**
- CMake utility functions and templates
- Platform-specific toolchain files
- Build type configuration (debug/release)
- Dependency resolution

**4. Build Configuration (`build_config.py` in project)**
- Project-specific build settings
- Generated from template during `ccgo new`
- Loaded by build scripts

## Platform Abstraction

### Common Build Interface

All platform build scripts implement a common interface:

```python
# build_scripts/build_<platform>.py

def configure_cmake(project_dir, build_dir, config):
    """Configure CMake with platform-specific settings"""
    pass

def build_libraries(build_dir, config):
    """Build static and/or shared libraries"""
    pass

def collect_artifacts(build_dir, output_dir, config):
    """Collect build artifacts"""
    pass

def package_artifacts(output_dir, config):
    """Package artifacts into ZIP archive"""
    pass
```

### Platform-Specific Build Scripts

| Platform | Script | Toolchain | Output Formats |
|----------|--------|-----------|----------------|
| Android | `build_android.py` | NDK | .so, .a, AAR |
| iOS | `build_ios.py` | Xcode | Framework, XCFramework |
| macOS | `build_macos.py` | Xcode | Framework, XCFramework, dylib |
| Windows | `build_windows.py` | MSVC/MinGW | .dll, .lib/.a |
| Linux | `build_linux.py` | GCC/Clang | .so, .a |
| OpenHarmony | `build_ohos.py` | OHOS SDK | .so, .a, HAR |
| watchOS | `build_watchos.py` | Xcode | Framework, XCFramework |
| tvOS | `build_tvos.py` | Xcode | Framework, XCFramework |

## CMake Integration

### CMake Directory Structure

```
ccgo/build_scripts/cmake/
├── CMakeUtils.cmake          # Utility functions
├── CMakeFunctions.cmake      # Build helper functions
├── FindCCGODependencies.cmake # Dependency resolution
├── ios.toolchain.cmake       # iOS cross-compilation
├── tvos.toolchain.cmake      # tvOS cross-compilation
├── watchos.toolchain.cmake   # watchOS cross-compilation
├── windows-msvc.toolchain.cmake  # Windows MSVC
└── template/                 # CMakeLists.txt templates
    ├── Root.CMakeLists.txt.in
    ├── Src.CMakeLists.txt.in
    ├── Tests.CMakeLists.txt.in
    └── ...
```

### CMake Configuration Variables

CCGO passes these variables to CMake:

```cmake
# Platform information
${CCGO_CMAKE_DIR}          # Path to CCGO cmake utilities
${PLATFORM}                # Target platform (android, ios, etc.)
${ARCHITECTURE}            # Target architecture (arm64-v8a, x86_64, etc.)

# Build configuration
${BUILD_TYPE}              # Debug or Release
${LINK_TYPE}               # static, shared, or both
${CPP_STANDARD}            # C++ standard (11, 14, 17, 20, 23)

# Project information
${PROJECT_NAME}            # From CCGO.toml
${PROJECT_VERSION}         # From CCGO.toml
${PROJECT_NAMESPACE}       # C++ namespace

# Platform-specific (Android)
${ANDROID_ABI}             # Android architecture
${ANDROID_PLATFORM}        # Android API level
${ANDROID_NDK}             # NDK path
${ANDROID_STL}             # STL type

# Platform-specific (Apple)
${CMAKE_OSX_DEPLOYMENT_TARGET}      # Minimum OS version
${CMAKE_OSX_ARCHITECTURES}          # Architecture list
```

### CMake Usage in Projects

**Project CMakeLists.txt:**

```cmake
cmake_minimum_required(VERSION 3.18)
project(mylib VERSION 1.0.0)

# Include CCGO utilities
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# Use CCGO functions
ccgo_setup_project()

# Define library
ccgo_add_library(${PROJECT_NAME}
    SOURCES
        src/mylib.cpp
        src/utils.cpp
    HEADERS
        include/mylib/mylib.h
        include/mylib/utils.h
    PUBLIC_HEADERS
        include/mylib/mylib.h
)

# Link dependencies
ccgo_link_dependencies(${PROJECT_NAME}
    PUBLIC spdlog fmt
)
```

## Build Configuration

### CCGO.toml Build Section

```toml
[build]
cpp_standard = "17"               # C++ standard
cmake_minimum_version = "3.18"    # Minimum CMake version
compile_flags = ["-Wall", "-Wextra"]  # Additional compiler flags
link_flags = ["-flto"]            # Additional linker flags

[build.definitions]
DEBUG_MODE = "1"                  # Preprocessor definitions
APP_VERSION = "\"1.0.0\""

[build]
include_dirs = ["third_party/include"]  # Additional include directories
link_dirs = ["third_party/lib"]         # Additional library directories
system_libs = ["pthread", "dl"]         # System libraries to link
```

### build_config.py

Generated in project root, contains runtime build configuration:

```python
# build_config.py

PROJECT_NAME = "mylib"
PROJECT_VERSION = "1.0.0"
CPP_STANDARD = "17"

# Platform-specific configuration
ANDROID_CONFIG = {
    "min_sdk_version": 21,
    "target_sdk_version": 33,
    "ndk_version": "25.2.9519653",
    "stl": "c++_static",
    "architectures": ["arm64-v8a", "armeabi-v7a", "x86_64"]
}

IOS_CONFIG = {
    "min_deployment_target": "12.0",
    "enable_bitcode": False,
    "architectures": ["arm64"]
}

# ... more platform configs
```

## Build Process

### Step-by-Step Build Flow

**1. Parse Command**
```bash
ccgo build android --arch arm64-v8a --release
```
- Platform: android
- Architecture: arm64-v8a
- Build type: release

**2. Load Configuration**
- Read CCGO.toml
- Load build_config.py
- Validate platform/architecture combination

**3. Configure CMake**
```bash
cmake -S <source_dir> -B <build_dir> \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_TOOLCHAIN_FILE=<ndk>/build/cmake/android.toolchain.cmake \
    -DANDROID_ABI=arm64-v8a \
    -DANDROID_PLATFORM=android-21 \
    -DCCGO_CMAKE_DIR=<ccgo>/build_scripts/cmake
```

**4. Build Libraries**
```bash
cmake --build <build_dir> --config Release --target all
```

**5. Collect Artifacts**
- Copy libraries (.so, .a, .dll, etc.)
- Copy headers
- Copy platform-specific packages (AAR, Framework, etc.)
- Generate build metadata (build_info.json)

**6. Package**
- Create unified ZIP archive structure
- Create symbols ZIP if debug build
- Calculate checksums

### Build Directories

```
project/
├── cmake_build/              # CMake build directories
│   ├── android/
│   │   ├── arm64-v8a/       # Per-architecture builds
│   │   │   ├── debug/
│   │   │   └── release/
│   │   └── armeabi-v7a/
│   ├── ios/
│   │   └── ...
│   └── ...
└── target/                   # Final build outputs
    ├── android/
    │   ├── MYLIB_ANDROID_SDK-1.0.0.zip
    │   ├── MYLIB_ANDROID_SDK-1.0.0-SYMBOLS.zip
    │   └── build_info.json
    ├── ios/
    └── ...
```

## Output Artifacts

### Unified Archive Structure

All platforms use a consistent structure:

```
{PROJECT}_{PLATFORM}_SDK-{version}.zip
├── lib/
│   ├── static/              # Static libraries
│   │   └── {arch}/          # Per-architecture (mobile)
│   │       └── lib{name}.a
│   └── shared/              # Shared libraries
│       └── {arch}/
│           └── lib{name}.so
├── frameworks/              # Apple platforms only
│   ├── static/
│   │   └── {Name}.xcframework
│   └── shared/
│       └── {Name}.xcframework
├── haars/                   # Android/OHOS only
│   └── {name}-release.aar
├── include/                 # Public headers
│   └── {project}/
│       ├── {header}.h
│       └── version.h
└── build_info.json          # Build metadata
```

### Build Metadata (build_info.json)

```json
{
  "project": "mylib",
  "version": "1.0.0",
  "platform": "android",
  "architectures": ["arm64-v8a", "armeabi-v7a"],
  "build_type": "release",
  "link_types": ["static", "shared"],
  "timestamp": "2025-01-19T10:30:00Z",
  "git": {
    "commit": "a1b2c3d",
    "branch": "main",
    "tag": "v1.0.0"
  },
  "toolchain": {
    "name": "Android NDK",
    "version": "25.2.9519653",
    "compiler": "clang 14.0.7"
  },
  "dependencies": {
    "spdlog": "1.12.0",
    "fmt": "10.1.1"
  },
  "checksums": {
    "sha256": "..."
  }
}
```

## Build Types

### Debug Build

**Characteristics:**
- Debug symbols included
- No optimization (-O0)
- Assertions enabled
- Larger binary size
- Easier debugging

**Usage:**
```bash
ccgo build <platform> --debug
```

**CMake flags:**
```cmake
-DCMAKE_BUILD_TYPE=Debug
-DCMAKE_CXX_FLAGS_DEBUG="-g -O0"
```

### Release Build

**Characteristics:**
- Symbols stripped (separate SYMBOLS.zip)
- Full optimization (-O3 or equivalent)
- Assertions disabled
- Smaller binary size
- Better performance

**Usage:**
```bash
ccgo build <platform> --release
```

**CMake flags:**
```cmake
-DCMAKE_BUILD_TYPE=Release
-DCMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG"
```

## Link Types

### Static Libraries

**Characteristics:**
- Code embedded in final binary
- No runtime dependency
- Larger binary size
- Single file deployment

**Usage:**
```bash
ccgo build <platform> --link-type static
```

**Output:** `.a` (Unix), `.lib` (Windows)

### Shared Libraries

**Characteristics:**
- Code in separate library file
- Runtime dependency required
- Smaller binary size
- Code sharing between apps

**Usage:**
```bash
ccgo build <platform> --link-type shared
```

**Output:** `.so` (Unix/Android), `.dylib` (macOS), `.dll` (Windows)

### Both (Default)

Build both static and shared libraries:

```bash
ccgo build <platform> --link-type both
```

## Toolchain Selection

### Windows: MSVC vs MinGW

**MSVC (Microsoft Visual C++):**
```bash
ccgo build windows --toolchain msvc
```
- Native Windows toolchain
- Best Visual Studio integration
- ABI compatible with Windows SDK

**MinGW (Minimalist GNU for Windows):**
```bash
ccgo build windows --toolchain mingw
```
- GCC-based toolchain
- Better cross-compilation support
- Compatible with Docker builds

**Auto (Both):**
```bash
ccgo build windows --toolchain auto  # default
```

### Platform Toolchains

| Platform | Default Toolchain | Alternatives |
|----------|-------------------|--------------|
| Android | NDK (Clang) | - |
| iOS | Xcode (Clang) | - |
| macOS | Xcode (Clang) | - |
| Windows | MSVC | MinGW |
| Linux | GCC | Clang |
| OpenHarmony | OHOS SDK | - |

## Docker Builds

### Overview

Docker builds enable universal cross-compilation:
- **Zero local setup**: No SDK/NDK/toolchain installation required
- **Consistent environment**: Same build environment across all machines
- **Isolated builds**: No conflicts with local installations
- **Pre-built images**: Fast startup (images pulled from Docker Hub)

### Usage

```bash
# Build any platform with Docker
ccgo build android --docker
ccgo build ios --docker
ccgo build windows --docker
ccgo build linux --docker
```

### Docker Images

| Platform | Image Name | Size | Contains |
|----------|-----------|------|----------|
| Android | `ccgo-builder-android` | ~3.5GB | SDK, NDK, CMake |
| iOS/macOS/watchOS/tvOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, SDKs |
| Windows | `ccgo-builder-windows` | ~1.2GB | MinGW-w64, CMake |
| Linux | `ccgo-builder-linux` | ~800MB | GCC, Clang, CMake |

### Docker Build Flow

```
ccgo build <platform> --docker
    ↓
Check Docker is running
    ↓
Pull/use cached Docker image
    ↓
Mount project directory as volume
    ↓
Run build inside container
    ↓
Write artifacts to host filesystem
```

## Incremental Builds

### CMake Caching

CCGO uses CMake's built-in caching:
- CMake cache stored in `cmake_build/<platform>/<arch>/<build_type>/`
- Only recompiles changed source files
- Detects header changes automatically

**Force full rebuild:**
```bash
ccgo build <platform> --clean
```

### Dependency Caching

- Dependencies built once, cached for incremental builds
- Cache invalidated when:
  - Dependency version changes in CCGO.toml
  - CCGO.lock is updated
  - CMake configuration changes

**Clear dependency cache:**
```bash
rm -rf cmake_build/
ccgo build <platform>
```

### Build Performance

**First build:** 10-30 minutes (compiles all dependencies)
**Incremental build:** 10-60 seconds (only changed files)

**Optimization tips:**
1. Use `--arch` to limit architectures during development
2. Use `--link-type` to build only needed library types
3. Enable `ccache` for compiler caching (future feature)
4. Use prebuilt dependencies (future feature)

## IDE Project Generation

### Generate IDE Projects

```bash
# Android Studio project
ccgo build android --ide-project

# Xcode project
ccgo build ios --ide-project

# Visual Studio project
ccgo build windows --ide-project --toolchain msvc

# CodeLite project (Linux)
ccgo build linux --ide-project
```

### IDE Integration

**Android Studio:**
- Generates `.iml` files
- Gradle sync support
- Native debugging with LLDB

**Xcode:**
- Generates `.xcodeproj`
- Integrated debugging
- Code signing support

**Visual Studio:**
- Generates `.sln` and `.vcxproj`
- IntelliSense support
- MSVC debugger integration

## Custom Build Steps

### Pre-Build Hooks

**Add custom pre-build script:**

```python
# build_config.py

def pre_build_hook(platform, arch, build_type):
    """Called before build starts"""
    print(f"Pre-build: {platform} {arch} {build_type}")
    # Custom logic here
```

### Post-Build Hooks

```python
# build_config.py

def post_build_hook(platform, arch, build_type, output_dir):
    """Called after build completes"""
    print(f"Post-build: artifacts in {output_dir}")
    # Custom artifact processing
```

### Custom CMake

**Extend CMakeLists.txt:**

```cmake
# CMakeLists.txt

# Custom source generation
add_custom_command(
    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/generated.cpp
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/codegen.py
    DEPENDS codegen.py
)

# Add generated source
target_sources(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_BINARY_DIR}/generated.cpp
)
```

## Troubleshooting

### Common Build Issues

#### CMake Configuration Failed

```
Error: CMake configuration failed
```

**Solutions:**
1. Check CMake version: `cmake --version` (need 3.18+)
2. Verify toolchain installation
3. Run with verbose: `ccgo build <platform> --verbose`
4. Check `cmake_build/<platform>/CMakeError.log`

#### Compiler Not Found

```
Error: Could not find compiler
```

**Solutions:**
1. Install required toolchain
2. Set environment variables (ANDROID_NDK, etc.)
3. Use Docker build: `ccgo build <platform> --docker`

#### Link Errors

```
Error: undefined reference to 'symbol'
```

**Solutions:**
1. Check all source files are in CMakeLists.txt
2. Verify dependency versions match
3. Check C++ standard consistency
4. Enable verbose linking: add `-Wl,--verbose` to link_flags

#### Out of Memory

```
Error: c++: fatal error: Killed signal terminated program cc1plus
```

**Solutions:**
1. Build fewer architectures: `--arch arm64-v8a`
2. Build single link type: `--link-type static`
3. Increase Docker memory: Docker Desktop → Preferences → Resources
4. Use swap space on Linux

### Performance Issues

#### Slow Builds

**Diagnosis:**
```bash
ccgo build <platform> --verbose  # See timing information
```

**Optimizations:**
1. Limit architectures during development
2. Use incremental builds (don't `--clean` unless necessary)
3. Enable parallel builds (automatic with CMake)
4. Use SSD for build directory

#### Disk Space Issues

**Check sizes:**
```bash
du -sh cmake_build/
du -sh target/
```

**Clean up:**
```bash
ccgo clean          # Remove all build artifacts
ccgo clean --yes    # Skip confirmation
```

## Best Practices

### 1. Version Control

**Do commit:**
- CCGO.toml
- CMakeLists.txt
- build_config.py
- CCGO.lock (if using locked dependencies)

**Don't commit:**
- cmake_build/
- target/
- *.pyc

**.gitignore:**
```gitignore
cmake_build/
target/
__pycache__/
*.pyc
.DS_Store
```

### 2. CI/CD Integration

**GitHub Actions example:**

```yaml
name: Build All Platforms

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        platform: [android, ios, linux, windows, macos]

    steps:
      - uses: actions/checkout@v3

      - name: Install CCGO
        run: pip install ccgo

      - name: Build ${{ matrix.platform }}
        run: ccgo build ${{ matrix.platform }} --docker --release

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.platform }}-libs
          path: target/${{ matrix.platform }}/*.zip
```

### 3. Build Configuration

**Development:**
```toml
[build]
cpp_standard = "17"
compile_flags = ["-Wall", "-Wextra", "-Werror"]  # Strict warnings
```

**Production:**
```toml
[build]
cpp_standard = "17"
compile_flags = ["-O3", "-DNDEBUG"]              # Optimized
link_flags = ["-flto"]                           # Link-time optimization
```

### 4. Dependency Management

**Pin dependencies for reproducibility:**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }
```

**Use CCGO.lock:**
```bash
ccgo install --locked  # Install exact versions from CCGO.lock
```

## Advanced Topics

### Multi-Module Builds

**Project structure:**
```
my-project/
├── CCGO.toml
├── lib1/
│   ├── CCGO.toml
│   └── src/
└── lib2/
    ├── CCGO.toml (depends on lib1)
    └── src/
```

**Build order:**
1. CCGO automatically determines build order
2. lib1 built first
3. lib2 built with lib1 as dependency

### Cross-Compilation

**Example: Build macOS library on Linux:**
```bash
# Using Docker with OSXCross
ccgo build macos --docker
```

**Example: Build Windows library on macOS:**
```bash
# Using Docker with MinGW
ccgo build windows --docker --toolchain mingw
```

### Custom Toolchains

**Add custom toolchain file:**

```cmake
# my-toolchain.cmake
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_C_COMPILER /path/to/custom-gcc)
set(CMAKE_CXX_COMPILER /path/to/custom-g++)
```

**Use in build:**
```python
# build_config.py
CUSTOM_TOOLCHAIN = "/path/to/my-toolchain.cmake"
```

## See Also

- [CLI Reference](../reference/cli.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Platform Guides](../platforms/index.md)
- [Dependency Management](dependency-management.md)
- [Docker Builds](docker-builds.md)
- [Publishing](publishing.md)
