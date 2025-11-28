# CCGO Dependencies Guide

This document describes how to use CCGO's dependency management system, including installing, configuring, and using third-party libraries.

## Table of Contents

- [Quick Start](#quick-start)
- [CCGO.toml Configuration](#ccgotoml-configuration)
- [Installing Dependencies](#installing-dependencies)
- [CMake Integration](#cmake-integration)
- [Link Type Support](#link-type-support)
- [Packaging SDK](#packaging-sdk)

## Quick Start

### 1. Configure Dependencies

Declare dependencies in your project's `CCGO.toml` file:

```toml
[project]
name = "myproject"
version = "1.0.0"

[dependencies]
# Download from remote URL
libfoo = { version = "1.0.0", source = "https://example.com/LIBFOO_SDK-1.0.0.zip" }

# Use local path
libbar = { path = "../libbar/target/package/LIBBAR_SDK-1.0.0" }
```

### 2. Install Dependencies

```bash
# Install all dependencies
ccgo install

# Install specific dependency
ccgo install libfoo

# Force reinstall
ccgo install --force
```

### 3. Use in CMake

```cmake
# In CMakeLists.txt
include(${CCGO_CMAKE_DIR}/FindCCGODependencies.cmake)
find_ccgo_dependencies()

# Link dependencies to target
ccgo_link_dependency(myapp libfoo)
```

### 4. Build Project

```bash
# Normal build
ccgo build android
ccgo build ios
```

## CCGO.toml Configuration

### Basic Format

```toml
[dependencies]
# library_name = { configuration_options }
```

### Configuration Options

#### 1. Remote URL Dependencies

```toml
[dependencies]
libfoo = {
    version = "1.0.0",
    source = "https://example.com/LIBFOO_SDK-1.0.0.zip"
}
```

Supported formats:
- `.zip` - ZIP archive
- `.tar.gz` - Gzip-compressed tar archive
- `.tgz` - Gzip-compressed tar archive (shorthand)

#### 2. Local Path Dependencies

```toml
[dependencies]
# Relative path (relative to project root)
libbar = { path = "../libbar/target/package/LIBBAR_SDK-1.0.0" }

# Absolute path
libbaz = { path = "/absolute/path/to/LIBBAZ_SDK-1.0.0" }
```

#### 3. Local Archive Files

```toml
[dependencies]
libqux = { source = "../archives/LIBQUX_SDK-1.0.0.tar.gz" }
```

### Platform-Specific Dependencies

Configure different dependencies for different platforms:

```toml
# Common dependencies (all platforms)
[dependencies]
common_lib = { version = "1.0.0", source = "https://example.com/common.zip" }

# Android-specific dependencies
[dependencies.android]
android_lib = { version = "1.0.0", source = "https://example.com/android.zip" }

# iOS-specific dependencies
[dependencies.ios]
ios_lib = { version = "1.0.0", source = "https://example.com/ios.zip" }

# macOS-specific dependencies
[dependencies.macos]
macos_lib = { version = "1.0.0", source = "https://example.com/macos.zip" }

# tvOS-specific dependencies
[dependencies.tvos]
tvos_lib = { version = "1.0.0", source = "https://example.com/tvos.zip" }

# watchOS-specific dependencies
[dependencies.watchos]
watchos_lib = { version = "1.0.0", source = "https://example.com/watchos.zip" }

# Windows-specific dependencies
[dependencies.windows]
windows_lib = { version = "1.0.0", source = "https://example.com/windows.zip" }

# Linux-specific dependencies
[dependencies.linux]
linux_lib = { version = "1.0.0", source = "https://example.com/linux.zip" }

# OpenHarmony-specific dependencies
[dependencies.ohos]
ohos_lib = { version = "1.0.0", source = "https://example.com/ohos.zip" }
```

## Installing Dependencies

### Basic Commands

```bash
# Install all dependencies
ccgo install

# Install specific dependency
ccgo install libfoo

# Force reinstall
ccgo install --force

# Clean cache before install
ccgo install --clean-cache
```

### Platform-Specific Installation

```bash
# Install only Android dependencies
ccgo install --platform android

# Install only iOS dependencies
ccgo install --platform ios
```

### Custom Cache Directory

```bash
# Use custom cache directory
ccgo install --cache-dir /tmp/ccgo-cache
```

### Installation Directory Structure

Directory structure after dependencies are installed:

```
myproject/
├── third_party/                    # Dependency installation directory
│   ├── libfoo/                     # Library name
│   │   ├── include/               # Header files
│   │   ├── lib/                   # Library files
│   │   │   ├── android/
│   │   │   │   ├── static/       # Static libraries
│   │   │   │   │   ├── arm64-v8a/
│   │   │   │   │   ├── armeabi-v7a/
│   │   │   │   │   └── x86_64/
│   │   │   │   └── shared/       # Shared libraries
│   │   │   │       ├── arm64-v8a/
│   │   │   │       ├── armeabi-v7a/
│   │   │   │       └── x86_64/
│   │   │   ├── ios/
│   │   │   │   ├── static/
│   │   │   │   └── shared/
│   │   │   └── ...
│   │   └── ccgo-package.json      # Package metadata
│   └── libbar/
│       └── ...
└── .ccgo/
    └── cache/                      # Download cache
        └── abc123_libfoo.zip
```

## CMake Integration

### Basic Usage

Include FindCCGODependencies in `CMakeLists.txt`:

```cmake
cmake_minimum_required(VERSION 3.10)
project(MyProject)

# Include CCGO dependency finder
include(${CCGO_CMAKE_DIR}/FindCCGODependencies.cmake)

# Find all installed dependencies
find_ccgo_dependencies()

# Create target
add_executable(myapp src/main.cpp)

# Link dependencies
if(CCGO_DEPENDENCY_LIBFOO_FOUND)
    ccgo_link_dependency(myapp libfoo)
endif()
```

### Available CMake Variables

After finding dependencies, the following variables are set (using libfoo as example):

```cmake
CCGO_DEPENDENCIES_FOUND                     # Whether any dependencies were found
CCGO_DEPENDENCY_LIBFOO_FOUND                # Whether libfoo was found
CCGO_DEPENDENCY_LIBFOO_INCLUDE_DIRS         # libfoo's include directories
CCGO_DEPENDENCY_LIBFOO_LIBRARIES            # libfoo's library files
CCGO_DEPENDENCY_LIBFOO_STATIC_LIBRARIES     # libfoo's static libraries
CCGO_DEPENDENCY_LIBFOO_SHARED_LIBRARIES     # libfoo's shared libraries
```

### Manual Dependency Linking

```cmake
# Link manually without helper function
if(CCGO_DEPENDENCY_LIBFOO_FOUND)
    target_include_directories(myapp PRIVATE
        ${CCGO_DEPENDENCY_LIBFOO_INCLUDE_DIRS}
    )
    target_link_libraries(myapp PRIVATE
        ${CCGO_DEPENDENCY_LIBFOO_LIBRARIES}
    )
endif()
```

### Controlling Link Type

```cmake
# Set before find_ccgo_dependencies()
set(CCGO_DEPENDENCY_LINK_TYPE "static")   # Use static libraries
# set(CCGO_DEPENDENCY_LINK_TYPE "shared")  # Use shared libraries

find_ccgo_dependencies()
```

### Platform-Specific Dependencies

```cmake
# Android platform
if(ANDROID)
    if(CCGO_DEPENDENCY_LIBANDROID_FOUND)
        ccgo_link_dependency(myapp libandroid)
    endif()
endif()

# iOS platform
if(IOS)
    if(CCGO_DEPENDENCY_LIBIOS_FOUND)
        ccgo_link_dependency(myapp libios)
    endif()
endif()

# macOS platform
if(CMAKE_SYSTEM_NAME STREQUAL "Darwin" AND NOT IOS)
    if(CCGO_DEPENDENCY_LIBMACOS_FOUND)
        ccgo_link_dependency(myapp libmacos)
    endif()
endif()
```

## Link Type Support

CCGO supports building and using both static and shared library types.

### Specifying Link Type During Build

All platform build scripts support the `link_type` parameter:

```python
# In build_config.py
def main():
    # Build static library (default)
    build_platform(link_type='static')

    # Build shared library
    build_platform(link_type='shared')

    # Build both types
    build_platform(link_type='both')
```

### Platform Support

| Platform | Static (.a/.lib) | Shared (.so/.dll/.dylib) |
|------|------------------|-------------------------|
| Android | ✅ | ✅ |
| iOS | ✅ | ✅ |
| macOS | ✅ | ✅ |
| tvOS | ✅ | ✅ |
| watchOS | ✅ | ✅ |
| Windows | ✅ | ✅ |
| Linux | ✅ | ✅ |
| OHOS | ✅ | ✅ |

### Output Directory Structure

Output directory structure after build:

```
cmake_build/
└── <Platform>/
    └── <Platform>.out/
        ├── static/                 # Static library output
        │   ├── <arch>/            # Architecture directory (Android/OHOS/Windows)
        │   │   └── lib*.a         # or *.lib
        │   └── *.framework        # Apple platforms
        └── shared/                # Shared library output
            ├── <arch>/
            │   └── lib*.so        # or *.dll
            └── *.framework
```

## Packaging SDK

### Generating SDK Package

```bash
# Package all platforms
ccgo package

# Package specific platforms
ccgo package --platforms android,ios,macos

# Specify version
ccgo package --version 1.0.0

# Include documentation
ccgo package --include-docs

# Clean output directory
ccgo package --clean --output ./release
```

### SDK Package Structure

Generated SDK package structure:

```
MYPROJECT_SDK-1.0.0/
├── include/                       # Public header files
│   └── myproject/
│       └── *.h
├── lib/                           # Platform library files
│   ├── android/
│   │   ├── static/
│   │   │   ├── arm64-v8a/
│   │   │   │   └── libmyproject.a
│   │   │   ├── armeabi-v7a/
│   │   │   └── x86_64/
│   │   └── shared/
│   │       ├── arm64-v8a/
│   │       │   └── libmyproject.so
│   │       ├── armeabi-v7a/
│   │       └── x86_64/
│   ├── ios/
│   │   ├── static/
│   │   │   └── myproject.xcframework/
│   │   └── shared/
│   │       └── myproject.xcframework/
│   ├── macos/
│   ├── tvos/
│   ├── watchos/
│   ├── windows/
│   ├── linux/
│   └── ohos/
├── ccgo-package.json              # Package metadata
└── README.md                      # Package documentation
```

### ccgo-package.json Format

```json
{
  "name": "myproject",
  "version": "1.0.0",
  "generated": "2025-11-25T10:30:00",
  "platforms": {
    "android": {
      "link_types": {
        "static": {
          "architectures": {
            "arm64-v8a": {
              "libraries": [
                {
                  "name": "libmyproject.a",
                  "size": 123456,
                  "path": "lib/android/static/arm64-v8a/libmyproject.a"
                }
              ]
            }
          }
        },
        "shared": { ... }
      }
    },
    "ios": { ... }
  }
}
```

### Using SDK Package as Dependency

Generated SDK packages can be used as dependencies in other projects:

```toml
# In another project's CCGO.toml
[dependencies]
myproject = {
    version = "1.0.0",
    path = "../myproject/target/package/MYPROJECT_SDK-1.0.0"
}
```

## Complete Example

### 1. Create Project and Configure Dependencies

```bash
# Create new project
ccgo new myapp

# Edit CCGO.toml
cd myapp
```

```toml
# CCGO.toml
[project]
name = "myapp"
version = "1.0.0"

[dependencies]
curl = { version = "8.0.0", source = "https://example.com/CURL_SDK-8.0.0.zip" }
openssl = { path = "../openssl/sdk" }
```

### 2. Install Dependencies

```bash
ccgo install
```

Output:
```
================================================================================
CCGO Install - Install Project Dependencies
================================================================================

Project directory: /path/to/myapp

📖 Reading dependencies from CCGO.toml...

Found 2 dependency(ies) to install:
  - curl
  - openssl

================================================================================
Installing Dependencies
================================================================================

📦 Installing curl...
   Source type: remote_url
   Source: https://example.com/CURL_SDK-8.0.0.zip
   📥 Downloading from https://example.com/CURL_SDK-8.0.0.zip...
   Progress: 100%
   ✓ Downloaded to .ccgo/cache/abc123_CURL_SDK-8.0.0.zip
   📦 Extracting CURL_SDK-8.0.0.zip...
   ✓ Extracted to .ccgo/temp/curl
   ✓ Installed to third_party/curl

📦 Installing openssl...
   Source type: local_dir
   Source: /path/to/openssl/sdk
   📂 Copying from local directory...
   ✓ Installed to third_party/openssl

================================================================================
Installation Summary
================================================================================

✓ Successfully installed: 2
```

### 3. Use in CMake

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.10)
project(myapp)

# Include CCGO dependencies
include(${CCGO_CMAKE_DIR}/FindCCGODependencies.cmake)
find_ccgo_dependencies()

# Create application
add_executable(myapp src/main.cpp)

# Link dependencies
ccgo_link_dependency(myapp curl)
ccgo_link_dependency(myapp openssl)
```

### 4. Build

```bash
# Android
ccgo build android --arch arm64-v8a,armeabi-v7a

# iOS
ccgo build ios

# macOS
ccgo build macos
```

### 5. Package SDK

```bash
ccgo package --version 1.0.0 --include-docs
```

## Troubleshooting

### Issue: Dependency Not Found

```
ERROR: CCGO.toml not found in project directory
```

**Solution:** Ensure you run commands from the project root directory and that CCGO.toml exists.

### Issue: Download Failed

```
✗ Download failed: HTTP Error 404
```

**Solution:** Check if the dependency's source URL is correct and network is accessible.

### Issue: CMake Cannot Find Dependency

```
WARNING: Library directory not found for libfoo
```

**Solution:**
1. Ensure you ran `ccgo install`
2. Check if `third_party/libfoo` directory exists
3. Check if platform-specific library files exist

### Issue: Link Type Mismatch

**Solution:** Set the correct link type in CMake:

```cmake
set(CCGO_DEPENDENCY_LINK_TYPE "static")  # or "shared"
find_ccgo_dependencies()
```

## Best Practices

1. **Version Management**: Explicitly specify version numbers in CCGO.toml
2. **Cache Management**: Regularly clean `.ccgo/cache` directory
3. **Platform Dependencies**: Only configure dependencies for needed platforms
4. **Path Usage**: Use relative paths during development, URLs for production
5. **Link Type**: Choose static or shared based on requirements
6. **Dependency Updates**: Use `--force` to force update dependencies

## Reference

- [CCGO.toml.example](build_scripts/CCGO.toml.example) - Complete configuration example
- [CMakeLists.txt.dependencies.example](build_scripts/cmake/CMakeLists.txt.dependencies.example) - CMake usage example
- [FindCCGODependencies.cmake](build_scripts/cmake/FindCCGODependencies.cmake) - CMake module source code
