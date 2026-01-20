# CMake Integration

Complete reference for CMake integration in CCGO projects, including build system architecture, customization, and best practices.

## Overview

CCGO uses CMake as the underlying build system for all C++ cross-platform builds:

- **Modular CMake** - Separate templates for source, tests, benchmarks, and dependencies
- **Platform Abstraction** - Unified build interface across all platforms
- **Toolchain Support** - Pre-configured toolchains for cross-compilation
- **Dependency Management** - Automatic integration of third-party libraries
- **Build Customization** - Extensive configuration options
- **IDE Integration** - Generate projects for Visual Studio, Xcode, CodeLite

## CMake Structure

### CCGO CMake Directory

CCGO centralizes all CMake configuration in the package installation:

```
ccgo/build_scripts/cmake/
├── CMakeLists.txt.dependencies.example  # Dependency configuration
├── CMakeConfig.cmake                    # Global configuration
├── CMakeExtraFlags.cmake                # Compiler flags
├── CMakeFunctions.cmake                 # Helper functions
├── CMakeUtils.cmake                     # Utility functions
├── FindCCGODependencies.cmake           # Dependency finder
├── CCGODependencies.cmake               # Dependency resolver
├── ios.toolchain.cmake                  # iOS cross-compilation
├── tvos.toolchain.cmake                 # tvOS cross-compilation
├── watchos.toolchain.cmake              # watchOS cross-compilation
├── windows-msvc.toolchain.cmake         # Windows MSVC toolchain
└── template/                            # CMake templates
    ├── Root.CMakeLists.txt.in           # Root CMakeLists
    ├── Src.CMakeLists.txt.in            # Source CMakeLists
    ├── Src.SubDir.CMakeLists.txt.in     # Subdirectory CMakeLists
    ├── Tests.CMakeLists.txt.in          # Test CMakeLists
    ├── Benches.CMakeLists.txt.in        # Benchmark CMakeLists
    ├── ThirdParty.CMakeLists.txt.in     # Third-party CMakeLists
    ├── External.CMakeLists.txt.in       # External project CMakeLists
    └── External.Download.txt.in         # Download script
```

### Project CMake Structure

Generated projects reference CCGO's CMake files:

```
my-project/
├── CMakeLists.txt                       # Root CMake configuration
├── src/
│   └── CMakeLists.txt                   # Source build configuration
├── tests/
│   └── CMakeLists.txt                   # Test build configuration
├── benches/
│   └── CMakeLists.txt                   # Benchmark build configuration
└── cmake_build/                         # Build output
    ├── android/
    ├── ios/
    ├── macos/
    ├── windows/
    └── linux/
```

## Root CMakeLists.txt

### Basic Structure

```cmake
cmake_minimum_required(VERSION 3.20)

# Project definition
project(MyLib
    VERSION 1.0.0
    DESCRIPTION "My C++ Library"
    LANGUAGES CXX
)

# C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# CCGO CMake directory (set by ccgo build)
if(NOT DEFINED CCGO_CMAKE_DIR)
    message(FATAL_ERROR "CCGO_CMAKE_DIR must be set")
endif()

# Include CCGO utilities
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CMakeConfig.cmake)

# Platform detection
ccgo_detect_platform()

# Build configuration
option(BUILD_SHARED_LIBS "Build shared libraries" ON)
option(BUILD_TESTS "Build tests" OFF)
option(BUILD_BENCHES "Build benchmarks" OFF)

# Subdirectories
add_subdirectory(src)

if(BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(BUILD_BENCHES)
    add_subdirectory(benches)
endif()
```

### Version Injection

```cmake
# Version configuration
set(PROJECT_VERSION_MAJOR 1)
set(PROJECT_VERSION_MINOR 0)
set(PROJECT_VERSION_PATCH 0)
set(PROJECT_VERSION "${PROJECT_VERSION_MAJOR}.${PROJECT_VERSION_MINOR}.${PROJECT_VERSION_PATCH}")

# Git information (injected by CCGO)
if(DEFINED GIT_SHA)
    set(PROJECT_GIT_SHA ${GIT_SHA})
else()
    set(PROJECT_GIT_SHA "unknown")
endif()

if(DEFINED GIT_BRANCH)
    set(PROJECT_GIT_BRANCH ${GIT_BRANCH})
else()
    set(PROJECT_GIT_BRANCH "unknown")
endif()

# Generate version header
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/include/${PROJECT_NAME}/version.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/include/${PROJECT_NAME}/version.h"
    @ONLY
)
```

## Source CMakeLists.txt

### Library Definition

```cmake
# src/CMakeLists.txt

# Source files
set(SOURCES
    mylib.cpp
    utils.cpp
    network.cpp
)

# Public headers
set(PUBLIC_HEADERS
    ${CMAKE_SOURCE_DIR}/include/mylib/mylib.h
    ${CMAKE_SOURCE_DIR}/include/mylib/utils.h
    ${CMAKE_SOURCE_DIR}/include/mylib/network.h
)

# Private headers
set(PRIVATE_HEADERS
    internal/config.h
    internal/helpers.h
)

# Create library
add_library(${PROJECT_NAME}
    ${SOURCES}
    ${PUBLIC_HEADERS}
    ${PRIVATE_HEADERS}
)

# Include directories
target_include_directories(${PROJECT_NAME}
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_CURRENT_BINARY_DIR}
)

# Compiler definitions
target_compile_definitions(${PROJECT_NAME}
    PRIVATE
        MYLIB_VERSION="${PROJECT_VERSION}"
        $<$<CONFIG:Debug>:MYLIB_DEBUG>
)

# Compiler options
target_compile_options(${PROJECT_NAME}
    PRIVATE
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
        $<$<NOT:$<CXX_COMPILER_ID:MSVC>>:-Wall -Wextra -Wpedantic>
)

# Link libraries
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        # Public dependencies visible to consumers
    PRIVATE
        # Private dependencies hidden from consumers
        Threads::Threads
)

# Platform-specific configuration
ccgo_configure_platform_target(${PROJECT_NAME})

# Export symbols for shared library
if(BUILD_SHARED_LIBS)
    target_compile_definitions(${PROJECT_NAME}
        PRIVATE MYLIB_BUILDING_DLL
        INTERFACE MYLIB_USING_DLL
    )
endif()

# Install rules
install(TARGETS ${PROJECT_NAME}
    EXPORT ${PROJECT_NAME}Targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
    INCLUDES DESTINATION include
)

install(DIRECTORY ${CMAKE_SOURCE_DIR}/include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.h"
)
```

### Subdirectory Organization

```cmake
# src/CMakeLists.txt

# Core library
add_subdirectory(core)

# Platform-specific modules
if(CCGO_PLATFORM STREQUAL "android")
    add_subdirectory(jni)
elseif(CCGO_PLATFORM MATCHES "ios|macos")
    add_subdirectory(objc)
elseif(CCGO_PLATFORM STREQUAL "windows")
    add_subdirectory(win32)
endif()

# Optional features
option(ENABLE_NETWORKING "Enable networking module" ON)
if(ENABLE_NETWORKING)
    add_subdirectory(network)
endif()
```

## Tests CMakeLists.txt

### Test Configuration

```cmake
# tests/CMakeLists.txt

# Find test framework
find_package(GTest REQUIRED)

# Test sources
set(TEST_SOURCES
    test_main.cpp
    test_calculator.cpp
    test_network.cpp
)

# Create test executable
add_executable(${PROJECT_NAME}_tests ${TEST_SOURCES})

# Link test framework and library
target_link_libraries(${PROJECT_NAME}_tests
    PRIVATE
        ${PROJECT_NAME}
        GTest::gtest
        GTest::gtest_main
)

# Include directories
target_include_directories(${PROJECT_NAME}_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}/include
        ${CMAKE_CURRENT_SOURCE_DIR}
)

# Discover tests
include(GoogleTest)
gtest_discover_tests(${PROJECT_NAME}_tests)

# Platform-specific test configuration
ccgo_configure_platform_tests(${PROJECT_NAME}_tests)
```

## Benchmarks CMakeLists.txt

### Benchmark Configuration

```cmake
# benches/CMakeLists.txt

# Find benchmark framework
find_package(benchmark REQUIRED)

# Benchmark sources
set(BENCH_SOURCES
    bench_main.cpp
    bench_calculator.cpp
    bench_network.cpp
)

# Create benchmark executable
add_executable(${PROJECT_NAME}_benches ${BENCH_SOURCES})

# Link benchmark framework and library
target_link_libraries(${PROJECT_NAME}_benches
    PRIVATE
        ${PROJECT_NAME}
        benchmark::benchmark
        benchmark::benchmark_main
)

# Include directories
target_include_directories(${PROJECT_NAME}_benches
    PRIVATE
        ${CMAKE_SOURCE_DIR}/include
        ${CMAKE_CURRENT_SOURCE_DIR}
)

# Platform-specific benchmark configuration
ccgo_configure_platform_benches(${PROJECT_NAME}_benches)
```

## Platform-Specific Configuration

### Android

```cmake
if(ANDROID)
    # Android API level
    set(ANDROID_PLATFORM android-${ANDROID_API_LEVEL})

    # Architecture-specific flags
    if(ANDROID_ABI STREQUAL "armeabi-v7a")
        target_compile_options(${PROJECT_NAME} PRIVATE
            -mfpu=neon
            -mfloat-abi=softfp
        )
    elseif(ANDROID_ABI STREQUAL "arm64-v8a")
        target_compile_options(${PROJECT_NAME} PRIVATE
            -march=armv8-a
        )
    endif()

    # Link Android libraries
    target_link_libraries(${PROJECT_NAME}
        PUBLIC
            android
            log
    )

    # Strip symbols in release builds
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        set_target_properties(${PROJECT_NAME} PROPERTIES
            LINK_FLAGS "-Wl,--strip-all"
        )
    endif()
endif()
```

### iOS/macOS

```cmake
if(APPLE)
    # Framework configuration
    if(IOS OR TVOS OR WATCHOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            FRAMEWORK TRUE
            FRAMEWORK_VERSION A
            MACOSX_FRAMEWORK_IDENTIFIER com.example.${PROJECT_NAME}
            PUBLIC_HEADER "${PUBLIC_HEADERS}"
        )
    endif()

    # Deployment target
    if(IOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            XCODE_ATTRIBUTE_IPHONEOS_DEPLOYMENT_TARGET "12.0"
        )
    elseif(MACOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            XCODE_ATTRIBUTE_MACOSX_DEPLOYMENT_TARGET "10.14"
        )
    endif()

    # Code signing (iOS only)
    if(IOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            XCODE_ATTRIBUTE_CODE_SIGN_IDENTITY "iPhone Developer"
            XCODE_ATTRIBUTE_DEVELOPMENT_TEAM "${DEVELOPMENT_TEAM_ID}"
        )
    endif()

    # Link Apple frameworks
    target_link_libraries(${PROJECT_NAME}
        PUBLIC
            "-framework Foundation"
            "-framework CoreFoundation"
    )
endif()
```

### Windows

```cmake
if(WIN32)
    # MSVC-specific configuration
    if(MSVC)
        # Runtime library
        set_property(TARGET ${PROJECT_NAME} PROPERTY
            MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL"
        )

        # Warning level
        target_compile_options(${PROJECT_NAME} PRIVATE
            /W4
            /WX  # Treat warnings as errors
        )

        # Export all symbols for DLL
        if(BUILD_SHARED_LIBS)
            set_target_properties(${PROJECT_NAME} PROPERTIES
                WINDOWS_EXPORT_ALL_SYMBOLS ON
            )
        endif()
    endif()

    # MinGW-specific configuration
    if(MINGW)
        target_compile_options(${PROJECT_NAME} PRIVATE
            -Wall -Wextra -Wpedantic
        )

        # Static linking of MinGW runtime
        target_link_options(${PROJECT_NAME} PRIVATE
            -static-libgcc
            -static-libstdc++
        )
    endif()

    # Link Windows libraries
    target_link_libraries(${PROJECT_NAME}
        PUBLIC
            ws2_32
            bcrypt
    )
endif()
```

### Linux

```cmake
if(UNIX AND NOT APPLE)
    # Position-independent code
    set_target_properties(${PROJECT_NAME} PROPERTIES
        POSITION_INDEPENDENT_CODE ON
    )

    # RPATH configuration
    set_target_properties(${PROJECT_NAME} PROPERTIES
        BUILD_RPATH_USE_ORIGIN ON
        INSTALL_RPATH "$ORIGIN"
    )

    # Link Linux libraries
    target_link_libraries(${PROJECT_NAME}
        PUBLIC
            pthread
            dl
    )

    # Strip symbols in release builds
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${CMAKE_STRIP} $<TARGET_FILE:${PROJECT_NAME}>
        )
    endif()
endif()
```

## Dependency Management

### Find Package

```cmake
# Find required dependencies
find_package(OpenSSL 1.1.1 REQUIRED)
find_package(ZLIB REQUIRED)
find_package(Protobuf REQUIRED)

# Link dependencies
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        OpenSSL::SSL
        OpenSSL::Crypto
    PRIVATE
        ZLIB::ZLIB
        protobuf::libprotobuf
)
```

### FetchContent

```cmake
include(FetchContent)

# Fetch nlohmann/json
FetchContent_Declare(
    nlohmann_json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG v3.11.2
)
FetchContent_MakeAvailable(nlohmann_json)

# Link fetched dependency
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        nlohmann_json::nlohmann_json
)
```

### ExternalProject

```cmake
include(ExternalProject)

# Build external project
ExternalProject_Add(
    boost
    URL https://boostorg.jfrog.io/artifactory/main/release/1.80.0/source/boost_1_80_0.tar.gz
    PREFIX ${CMAKE_BINARY_DIR}/external/boost
    CONFIGURE_COMMAND ./bootstrap.sh
    BUILD_COMMAND ./b2
    INSTALL_COMMAND ""
    BUILD_IN_SOURCE 1
)

# Add dependency
add_dependencies(${PROJECT_NAME} boost)

# Include external project headers
target_include_directories(${PROJECT_NAME}
    PRIVATE
        ${CMAKE_BINARY_DIR}/external/boost/src/boost
)
```

### Conan Integration

```cmake
# Include Conan CMake integration
include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

# Link Conan dependencies
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        CONAN_PKG::openssl
        CONAN_PKG::zlib
)
```

## CCGO Helper Functions

### ccgo_detect_platform()

Detects the target platform:

```cmake
ccgo_detect_platform()

# Available variables after detection:
# - CCGO_PLATFORM: android, ios, macos, windows, linux, ohos
# - CCGO_PLATFORM_ANDROID
# - CCGO_PLATFORM_IOS
# - CCGO_PLATFORM_MACOS
# - CCGO_PLATFORM_WINDOWS
# - CCGO_PLATFORM_LINUX
# - CCGO_PLATFORM_OHOS
```

### ccgo_configure_platform_target()

Configures target for the detected platform:

```cmake
ccgo_configure_platform_target(${PROJECT_NAME})

# Applies platform-specific:
# - Compiler flags
# - Linker flags
# - Architecture settings
# - Build type configuration
```

### ccgo_add_library()

Creates a library with CCGO conventions:

```cmake
ccgo_add_library(${PROJECT_NAME}
    SOURCES ${SOURCES}
    PUBLIC_HEADERS ${PUBLIC_HEADERS}
    PRIVATE_HEADERS ${PRIVATE_HEADERS}
    LINK_LIBRARIES ${DEPENDENCIES}
)
```

### ccgo_configure_version()

Configures version information:

```cmake
ccgo_configure_version(
    PROJECT_NAME ${PROJECT_NAME}
    VERSION_MAJOR 1
    VERSION_MINOR 0
    VERSION_PATCH 0
    GIT_SHA ${GIT_SHA}
    GIT_BRANCH ${GIT_BRANCH}
)
```

## Build Customization

### Compiler Flags

```cmake
# Global compiler flags
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    add_compile_options(
        -Wall
        -Wextra
        -Wpedantic
        -Werror
        $<$<CONFIG:Debug>:-O0 -g3>
        $<$<CONFIG:Release>:-O3 -DNDEBUG>
    )
elseif(MSVC)
    add_compile_options(
        /W4
        /WX
        $<$<CONFIG:Debug>:/Od /Zi>
        $<$<CONFIG:Release>:/O2 /DNDEBUG>
    )
endif()

# Target-specific flags
target_compile_options(${PROJECT_NAME} PRIVATE
    -fvisibility=hidden
    -ffunction-sections
    -fdata-sections
)
```

### Linker Flags

```cmake
# Remove unused sections
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_link_options(${PROJECT_NAME} PRIVATE
        -Wl,--gc-sections
    )
endif()

# Link-time optimization
if(CMAKE_BUILD_TYPE STREQUAL "Release")
    include(CheckIPOSupported)
    check_ipo_supported(RESULT ipo_supported)
    if(ipo_supported)
        set_property(TARGET ${PROJECT_NAME} PROPERTY
            INTERPROCEDURAL_OPTIMIZATION TRUE
        )
    endif()
endif()
```

### Build Types

```cmake
# Custom build type
set(CMAKE_BUILD_TYPE "RelWithDebInfo" CACHE STRING
    "Build type (Debug, Release, RelWithDebInfo, MinSizeRel)"
)

# Per-configuration settings
set(CMAKE_CXX_FLAGS_DEBUG "-O0 -g3 -DDEBUG")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")
set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "-O2 -g -DNDEBUG")
set(CMAKE_CXX_FLAGS_MINSIZEREL "-Os -DNDEBUG")
```

## IDE Project Generation

### Xcode

```bash
# Generate Xcode project
ccgo build ios --ide-project

# Or manually
cmake -G Xcode \
    -DCMAKE_TOOLCHAIN_FILE=${CCGO_CMAKE_DIR}/ios.toolchain.cmake \
    -DPLATFORM=OS64 \
    ..
```

### Visual Studio

```bash
# Generate Visual Studio project
ccgo build windows --ide-project

# Or manually
cmake -G "Visual Studio 17 2022" \
    -A x64 \
    ..
```

### CodeLite

```bash
# Generate CodeLite project
ccgo build linux --ide-project

# Or manually
cmake -G "CodeLite - Unix Makefiles" ..
```

## Best Practices

### 1. Modern CMake

```cmake
# Good: Use target-based commands
target_include_directories(${PROJECT_NAME} PUBLIC include/)
target_link_libraries(${PROJECT_NAME} PUBLIC OpenSSL::SSL)

# Bad: Use directory-based commands
include_directories(include/)
link_libraries(ssl)
```

### 2. Generator Expressions

```cmake
# Platform-specific flags
target_compile_options(${PROJECT_NAME} PRIVATE
    $<$<PLATFORM_ID:Windows>:/W4>
    $<$<PLATFORM_ID:Linux>:-Wall>
)

# Build type-specific definitions
target_compile_definitions(${PROJECT_NAME} PRIVATE
    $<$<CONFIG:Debug>:DEBUG_BUILD>
    $<$<CONFIG:Release>:RELEASE_BUILD>
)
```

### 3. Interface Libraries

```cmake
# Create interface library for header-only library
add_library(header_only INTERFACE)
target_include_directories(header_only INTERFACE include/)
target_compile_features(header_only INTERFACE cxx_std_17)

# Use interface library
target_link_libraries(${PROJECT_NAME} PUBLIC header_only)
```

### 4. Export Configuration

```cmake
# Export targets
install(EXPORT ${PROJECT_NAME}Targets
    FILE ${PROJECT_NAME}Targets.cmake
    NAMESPACE ${PROJECT_NAME}::
    DESTINATION lib/cmake/${PROJECT_NAME}
)

# Generate config file
include(CMakePackageConfigHelpers)
configure_package_config_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/Config.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake
    INSTALL_DESTINATION lib/cmake/${PROJECT_NAME}
)

# Install config files
install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake
    DESTINATION lib/cmake/${PROJECT_NAME}
)
```

## Troubleshooting

### CMake Cache Issues

```bash
# Clear CMake cache
rm -rf cmake_build/
ccgo build android  # Regenerate

# Or manually
rm CMakeCache.txt
cmake ..
```

### Toolchain Not Found

```bash
# Verify CCGO_CMAKE_DIR is set
echo $CCGO_CMAKE_DIR

# Manually set toolchain
cmake -DCMAKE_TOOLCHAIN_FILE=/path/to/toolchain.cmake ..
```

### Missing Dependencies

```cmake
# Add dependency search paths
list(APPEND CMAKE_PREFIX_PATH
    /usr/local
    /opt/homebrew
    ${CMAKE_SOURCE_DIR}/third_party
)
```

## Resources

### CMake Documentation

- [CMake Official Documentation](https://cmake.org/documentation/)
- [Modern CMake](https://cliutils.gitlab.io/modern-cmake/)
- [Effective CMake](https://www.youtube.com/watch?v=bsXLMQ6WgIk)

### CCGO Documentation

- [CLI Reference](cli.md)
- [CCGO.toml Reference](ccgo-toml.md)
- [Build System](../features/build-system.md)
- [Platform Guides](../platforms/index.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [CCGO.toml Reference](ccgo-toml.md)
- [Build System Overview](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Platform-Specific Guides](../platforms/index.md)
