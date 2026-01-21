# CMake Integration Guide

> Version: v3.1.0 | Updated: 2026-01-21

This guide explains how to integrate CCGO's CMake modules into your C++ cross-platform project for dependency management, platform-specific builds, and code organization.

## Table of Contents

1. [Overview](#overview)
2. [CMake Module Files](#cmake-module-files)
3. [Basic Setup](#basic-setup)
4. [Dependency Management](#dependency-management)
5. [Source Organization](#source-organization)
6. [Platform-Specific Code](#platform-specific-code)
7. [Build Configuration](#build-configuration)
8. [Complete Examples](#complete-examples)
9. [Best Practices](#best-practices)

---

## Overview

CCGO provides a set of CMake modules that simplify cross-platform C++ development:

- **CCGODependencies.cmake**: Manages dependencies from CCGO.toml
- **CMakeUtils.cmake**: Utility functions for project setup
- **CMakeFunctions.cmake**: Helper functions for source organization
- **CMakeConfig.cmake**: Project-wide configuration
- **CMakeExtraFlags.cmake**: Compiler flags and optimizations
- **Platform toolchains**: ios.toolchain.cmake, windows-msvc.toolchain.cmake, etc.

All CMake modules are centralized in the CCGO package and accessed via the `CCGO_CMAKE_DIR` variable.

---

## CMake Module Files

### Module Locations

CMake modules are installed with CCGO and referenced via:

```cmake
# CCGO_CMAKE_DIR is automatically set by CCGO build system
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
```

### Key Modules

| Module | Purpose |
|--------|---------|
| `CCGODependencies.cmake` | Dependency integration functions |
| `CMakeUtils.cmake` | Project setup and configuration |
| `CMakeFunctions.cmake` | Source file collection utilities |
| `CMakeConfig.cmake` | Global project settings |
| `CMakeExtraFlags.cmake` | Compiler optimization flags |
| `ios.toolchain.cmake` | iOS cross-compilation toolchain |
| `windows-msvc.toolchain.cmake` | Windows MSVC toolchain |

---

## Basic Setup

### Minimal CMakeLists.txt

```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# Include CCGO utilities
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# Create library
add_library(myproject STATIC
    src/main.cpp
    src/utils.cpp
)

# Set C++ standard
target_compile_features(myproject PUBLIC cxx_std_17)
```

### With CCGO Dependencies

```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# Include CCGO modules
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# Create library
add_library(myproject STATIC
    src/main.cpp
)

# Add CCGO dependencies (from CCGO.toml)
ccgo_add_dependencies(myproject)

target_compile_features(myproject PUBLIC cxx_std_17)
```

---

## Dependency Management

CCGO automatically manages dependencies from `CCGO.toml` and makes them available via CMake variables.

### Available Variables

| Variable | Description |
|----------|-------------|
| `CCGO_DEP_PATHS` | Semicolon-separated list of dependency paths |
| `CCGO_DEP_INCLUDE_DIRS` | Semicolon-separated list of include directories |
| `CCGO_CMAKE_DIR` | Path to CCGO CMake modules |

### Function: ccgo_add_dependencies()

Automatically adds dependency include directories to your target.

**Signature:**
```cmake
ccgo_add_dependencies(<target_name>)
```

**Example:**
```cmake
add_library(mylib STATIC src/main.cpp)

# Adds all CCGO dependency include directories
ccgo_add_dependencies(mylib)
```

This is equivalent to:
```cmake
target_include_directories(mylib PRIVATE
    ${CCGO_DEP_INCLUDE_DIRS}
)
```

### Function: ccgo_link_dependency()

Links a specific library from a CCGO dependency.

**Signature:**
```cmake
ccgo_link_dependency(<target_name> <dependency_name> <library_name>)
```

**Parameters:**
- `target_name`: Your CMake target
- `dependency_name`: Dependency name from CCGO.toml
- `library_name`: Library file name (without prefix/extension)

**Example:**
```cmake
add_library(mylib STATIC src/main.cpp)

# Link fmt library from fmt dependency
ccgo_link_dependency(mylib fmt fmt)

# Link spdlog library from spdlog dependency
ccgo_link_dependency(mylib spdlog spdlog)
```

The function searches for libraries in common locations:
- `<dep_path>/lib/`
- `<dep_path>/build/lib/`
- `<dep_path>/cmake_build/lib/`
- `<dep_path>/`

Supports different naming conventions:
- `libfmt.a`, `libfmt.so`, `libfmt.dylib` (Unix)
- `fmt.lib` (Windows)

### Function: ccgo_add_subdirectory()

Adds a CCGO dependency as a subdirectory (if it has CMakeLists.txt).

**Signature:**
```cmake
ccgo_add_subdirectory(<dependency_name>)
```

**Example:**
```cmake
# Add fmt dependency as subdirectory
ccgo_add_subdirectory(fmt)

# Now you can use fmt targets
add_library(mylib STATIC src/main.cpp)
target_link_libraries(mylib PRIVATE fmt::fmt)
```

### Function: ccgo_print_dependencies()

Prints debug information about available dependencies.

**Signature:**
```cmake
ccgo_print_dependencies()
```

**Example Output:**
```
=== CCGO Dependencies ===
Include directories:
  - /project/third_party/fmt/include
  - /project/third_party/spdlog/include
Dependency paths:
  - /project/third_party/fmt
  - /project/third_party/spdlog
========================
```

### Complete Dependency Example

**CCGO.toml:**
```toml
[package]
name = "myproject"
version = "1.0.0"

[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"

[[dependencies]]
name = "spdlog"
version = "^1.12"
git = "https://github.com/gabime/spdlog.git"
```

**CMakeLists.txt:**
```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# Print dependency info (debugging)
ccgo_print_dependencies()

# Create library
add_library(myproject STATIC
    src/main.cpp
    src/logger.cpp
)

# Method 1: Add all dependencies (includes only)
ccgo_add_dependencies(myproject)

# Method 2: Add specific dependencies as subdirectories
ccgo_add_subdirectory(fmt)
ccgo_add_subdirectory(spdlog)
target_link_libraries(myproject PRIVATE fmt::fmt spdlog::spdlog)

# Method 3: Link specific libraries manually
# ccgo_link_dependency(myproject fmt fmt)
# ccgo_link_dependency(myproject spdlog spdlog)

target_compile_features(myproject PUBLIC cxx_std_17)
```

---

## Source Organization

CCGO provides functions to automatically collect source files from directory structures.

### Function: add_sub_layer_sources_recursively()

Recursively collects all source files from a directory tree.

**Signature:**
```cmake
add_sub_layer_sources_recursively(<output_variable> <source_directory>)
```

**Supported Extensions:**
- `.cc`, `.c`, `.cpp` (C/C++ source)
- `.mm`, `.m` (Objective-C/C++)

**Example:**
```cmake
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)

# Collect all sources from src/ directory
set(MY_SOURCES "")
add_sub_layer_sources_recursively(MY_SOURCES ${CMAKE_SOURCE_DIR}/src)

# Create library with collected sources
add_library(mylib STATIC ${MY_SOURCES})
```

**Directory Structure:**
```
src/
├── main.cpp
├── utils/
│   ├── string_utils.cpp
│   └── file_utils.cpp
├── api/
│   ├── android/
│   │   └── jni_wrapper.cpp
│   └── ios/
│       └── swift_bridge.mm
└── core/
    └── engine.cpp
```

All files will be collected, and platform-specific directories (`android/`, `ios/`) are filtered automatically based on the build platform.

### Function: add_subdirectories_recursively()

Collects valid subdirectories for a given platform.

**Signature:**
```cmake
add_subdirectories_recursively(<output_variable> <root_directory>)
```

**Platform-Specific Directories:**
- `android/`, `jni/`: Only included when building for Android
- `ohos/`, `napi/`: Only included when building for OpenHarmony
- `ios/`: Only included when building for iOS
- `macos/`, `osx/`: Only included when building for macOS
- `oni/`, `apple/`: Only included when building for any Apple platform
- `windows/`, `win/`: Only included when building for Windows (MSVC)
- `linux/`: Only included when building for Linux

**Example:**
```cmake
set(SUBDIRS "")
add_subdirectories_recursively(SUBDIRS ${CMAKE_SOURCE_DIR}/src)

message(STATUS "Platform-specific subdirectories: ${SUBDIRS}")
```

### Macro: exclude_unittest_files()

Excludes unit test files from the build when tests are disabled.

**Signature:**
```cmake
exclude_unittest_files(<source_list_variable>)
```

**Excluded Patterns:**
- `*_unittest.cc`
- `*_test.cc`
- `*_mock.cc`

**Example:**
```cmake
file(GLOB MY_SOURCES src/*.cc)

# Exclude test files if GOOGLETEST_SUPPORT is OFF
exclude_unittest_files(MY_SOURCES)

add_library(mylib STATIC ${MY_SOURCES})
```

---

## Platform-Specific Code

### Platform Detection Variables

CCGO sets standard CMake variables for platform detection:

| Variable | Platform |
|----------|----------|
| `ANDROID` | Android |
| `APPLE` | macOS or iOS |
| `IOS` | iOS specifically |
| `OHOS` | OpenHarmony |
| `MSVC` | Windows (MSVC) |
| `UNIX` | Unix-like (Linux, macOS) |

### Conditional Compilation

```cmake
if(ANDROID)
    target_sources(mylib PRIVATE src/android/jni_impl.cpp)
elseif(APPLE AND IOS)
    target_sources(mylib PRIVATE src/ios/swift_bridge.mm)
elseif(APPLE)
    target_sources(mylib PRIVATE src/macos/cocoa_impl.mm)
elseif(MSVC)
    target_sources(mylib PRIVATE src/windows/win32_impl.cpp)
elseif(UNIX)
    target_sources(mylib PRIVATE src/linux/posix_impl.cpp)
endif()
```

### Platform-Specific Include Directories

```cmake
# Include platform API directories
include_directories(${CMAKE_SOURCE_DIR}/include/${PROJECT_NAME}/api/ios/)
include_directories(${CMAKE_SOURCE_DIR}/include/${PROJECT_NAME}/api/macos/)
include_directories(${CMAKE_SOURCE_DIR}/include/${PROJECT_NAME}/api/apple/)
```

This is automatically done by CMakeUtils.cmake when included.

---

## Build Configuration

### Common Configuration Options

```cmake
# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Export compile commands (for IDE integration)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Symbol visibility (default or hidden)
set(CMAKE_CXX_VISIBILITY_PRESET default)
set(CMAKE_C_VISIBILITY_PRESET default)

# Build types
set(CMAKE_CONFIGURATION_TYPES "Debug;Release" CACHE STRING "" FORCE)
```

### CCGO-Specific Options

```cmake
# Enable install rules
option(CCGO_ENABLE_INSTALL "Enable install rule" ON)

# Use system includes (suppress warnings)
option(CCGO_USE_SYSTEM_INCLUDES "Use SYSTEM for includes" OFF)

# Tag prefix for logging
set(CCGO_TAG_PREFIX "MyProject")

# Git revision
execute_process(
    COMMAND git rev-parse --short HEAD
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    OUTPUT_VARIABLE CCGO_REVISION
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
add_definitions(-DCCGO_REVISION="${CCGO_REVISION}")
```

### Third-Party Library Options

```cmake
# Enable third-party libraries
option(GOOGLETEST_SUPPORT "Use GoogleTest for unit tests" OFF)
option(BENCHMARK_SUPPORT "Use GoogleBenchmark for benchmarks" OFF)
option(RAPIDJSON_SUPPORT "Use RapidJSON for JSON support" ON)
```

---

## Complete Examples

### Example 1: Simple Library

```cmake
cmake_minimum_required(VERSION 3.18)
project(SimpleLib VERSION 1.0.0 LANGUAGES CXX)

# Include CCGO utilities
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# Collect sources
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)

# Create library
add_library(simplelib STATIC ${SOURCES})

target_compile_features(simplelib PUBLIC cxx_std_17)
target_include_directories(simplelib
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
)
```

### Example 2: Library with Dependencies

```cmake
cmake_minimum_required(VERSION 3.18)
project(AdvancedLib VERSION 1.0.0 LANGUAGES CXX)

# Include CCGO modules
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# Debug: Print dependencies
ccgo_print_dependencies()

# Collect sources
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)
exclude_unittest_files(SOURCES)

# Create library
add_library(advancedlib STATIC ${SOURCES})

# Add CCGO dependencies
ccgo_add_dependencies(advancedlib)

# Add specific dependencies as subdirectories
ccgo_add_subdirectory(fmt)
ccgo_add_subdirectory(spdlog)

# Link libraries
target_link_libraries(advancedlib
    PRIVATE
        fmt::fmt
        spdlog::spdlog
)

target_compile_features(advancedlib PUBLIC cxx_std_17)
```

### Example 3: Platform-Specific Build

```cmake
cmake_minimum_required(VERSION 3.18)
project(CrossPlatformLib VERSION 1.0.0)

include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)

# Collect common sources
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)

# Create library
add_library(crossplatformlib STATIC ${SOURCES})

# Platform-specific configurations
if(ANDROID)
    target_compile_definitions(crossplatformlib PRIVATE PLATFORM_ANDROID)
    target_link_libraries(crossplatformlib PRIVATE log android)
elseif(APPLE AND IOS)
    target_compile_definitions(crossplatformlib PRIVATE PLATFORM_IOS)
    target_link_libraries(crossplatformlib PRIVATE
        "-framework Foundation"
        "-framework UIKit"
    )
elseif(APPLE)
    target_compile_definitions(crossplatformlib PRIVATE PLATFORM_MACOS)
    target_link_libraries(crossplatformlib PRIVATE
        "-framework Foundation"
        "-framework Cocoa"
    )
elseif(MSVC)
    target_compile_definitions(crossplatformlib PRIVATE PLATFORM_WINDOWS)
elseif(UNIX)
    target_compile_definitions(crossplatformlib PRIVATE PLATFORM_LINUX)
    target_link_libraries(crossplatformlib PRIVATE pthread dl)
endif()

target_compile_features(crossplatformlib PUBLIC cxx_std_17)
```

### Example 4: Library with Tests

```cmake
cmake_minimum_required(VERSION 3.18)
project(LibWithTests VERSION 1.0.0)

include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# Main library
set(LIB_SOURCES "")
add_sub_layer_sources_recursively(LIB_SOURCES ${CMAKE_SOURCE_DIR}/src)
exclude_unittest_files(LIB_SOURCES)

add_library(mylib STATIC ${LIB_SOURCES})
ccgo_add_dependencies(mylib)
target_compile_features(mylib PUBLIC cxx_std_17)

# Tests (if enabled)
option(GOOGLETEST_SUPPORT "Build tests" OFF)

if(GOOGLETEST_SUPPORT)
    ccgo_add_subdirectory(googletest)

    file(GLOB TEST_SOURCES tests/*_test.cpp tests/*_unittest.cpp)

    add_executable(mylib_tests ${TEST_SOURCES})
    target_link_libraries(mylib_tests
        PRIVATE
            mylib
            gtest_main
            gtest
    )

    enable_testing()
    add_test(NAME mylib_tests COMMAND mylib_tests)
endif()
```

---

## Best Practices

### 1. Always Use CCGO_CMAKE_DIR

```cmake
# ✅ Good: Use variable
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# ❌ Bad: Hardcode path
include(/usr/local/lib/ccgo/cmake/CMakeUtils.cmake)
```

### 2. Leverage Automatic Source Collection

```cmake
# ✅ Good: Use helper function
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)
add_library(mylib STATIC ${SOURCES})

# ❌ Bad: Manual file listing
add_library(mylib STATIC
    src/a.cpp
    src/b.cpp
    src/c.cpp
    # ... hundreds of files
)
```

### 3. Use Platform-Specific Directories

```
src/
├── core/           # Common code
├── android/        # Android-only (auto-filtered)
├── ios/            # iOS-only (auto-filtered)
├── macos/          # macOS-only (auto-filtered)
└── windows/        # Windows-only (auto-filtered)
```

### 4. Properly Handle Dependencies

```cmake
# Option 1: Include directories only (lightweight)
ccgo_add_dependencies(mylib)

# Option 2: Add as subdirectory (full integration)
ccgo_add_subdirectory(fmt)
target_link_libraries(mylib PRIVATE fmt::fmt)

# Option 3: Link specific library (fine control)
ccgo_link_dependency(mylib fmt fmt)
```

### 5. Use Modern CMake Targets

```cmake
# ✅ Good: Target-based
target_include_directories(mylib PUBLIC ${CMAKE_SOURCE_DIR}/include)
target_compile_features(mylib PUBLIC cxx_std_17)
target_link_libraries(mylib PRIVATE fmt::fmt)

# ❌ Bad: Directory-based
include_directories(${CMAKE_SOURCE_DIR}/include)
set(CMAKE_CXX_STANDARD 17)
link_libraries(fmt)
```

### 6. Organize CMakeLists.txt

```cmake
# 1. CMake version and project
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# 2. Include CCGO modules
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# 3. Options and configuration
option(BUILD_TESTS "Build tests" OFF)

# 4. Collect sources
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)

# 5. Define targets
add_library(myproject STATIC ${SOURCES})

# 6. Configure targets
target_compile_features(myproject PUBLIC cxx_std_17)
target_include_directories(myproject PUBLIC ...)
ccgo_add_dependencies(myproject)

# 7. Platform-specific settings
if(ANDROID)
    # Android-specific
endif()

# 8. Tests (if enabled)
if(BUILD_TESTS)
    add_subdirectory(tests)
endif()
```

---

## Troubleshooting

### CCGO_CMAKE_DIR Not Defined

**Problem:** `CCGO_CMAKE_DIR` variable is not set.

**Solution:** This variable is automatically set by CCGO build system. Make sure you're building with:
```bash
ccgo build <platform>
```

Don't invoke CMake directly unless you manually set:
```bash
cmake -DCCGO_CMAKE_DIR=/path/to/ccgo/cmake ...
```

### Dependencies Not Found

**Problem:** `ccgo_add_dependencies()` does nothing or warnings about missing dependencies.

**Solution:**
1. Run `ccgo install` to fetch dependencies from CCGO.toml
2. Verify `CCGO_DEP_PATHS` and `CCGO_DEP_INCLUDE_DIRS` are set:
   ```cmake
   ccgo_print_dependencies()
   ```

### Platform-Specific Code Not Included

**Problem:** Platform-specific directories like `src/android/` are not included in the build.

**Solution:** Ensure you're using `add_sub_layer_sources_recursively()` which automatically filters based on platform:
```cmake
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)
```

### Circular Dependencies

**Problem:** CMake reports circular dependencies when using `ccgo_add_subdirectory()`.

**Solution:** Some dependencies may have circular references. Use `ccgo_link_dependency()` instead:
```cmake
# Instead of:
# ccgo_add_subdirectory(problematic_dep)

# Use:
ccgo_link_dependency(mylib problematic_dep lib_name)
```

---

## See Also

- [CCGO.toml Configuration Reference](../reference/config.md)
- [CLI Reference](../reference/cli.md)
- [Platform Guides](../platforms/index.md)
- [Dependency Management](../features/dependency-management.md)
