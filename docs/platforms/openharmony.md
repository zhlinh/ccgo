# OpenHarmony Platform

Complete guide to building C++ libraries for OpenHarmony (OHOS) with CCGO.

## Overview

CCGO provides comprehensive OpenHarmony support with:
- **Multiple architectures**: ARMv7, ARM64, x86_64
- **Output formats**: HAR packages, Static libraries (.a), Shared libraries (.so)
- **Build methods**: Local (DevEco Studio) or Docker (cross-platform)
- **IDE support**: DevEco Studio project generation
- **Publishing**: OHPM registry integration
- **ArkTS integration**: Native module interface (NAPI) support
- **API compatibility**: OpenHarmony 3.2+

## Prerequisites

### Option 1: Local Build (OpenHarmony SDK Required)

**Install DevEco Studio:**

1. Download from [OpenHarmony Developer Portal](https://developer.harmonyos.com/cn/develop/deveco-studio)
2. Install OpenHarmony SDK through DevEco Studio
3. Configure SDK path

**Required tools:**

```bash
# Install Node.js (for OHPM)
# Download from nodejs.org

# Install OHPM (OpenHarmony Package Manager)
npm install -g @ohos/hpm-cli

# Verify installation
node --version
ohpm --version
```

**Environment setup:**

```bash
# Set OHOS SDK path
export OHOS_SDK_HOME="/path/to/ohos-sdk"
export PATH="$OHOS_SDK_HOME/native/build-tools/cmake/bin:$PATH"

# Verify
cmake --version
```

### Option 2: Docker Build (Any OS)

Build OpenHarmony libraries on any OS using Docker.

**Required:**
- Docker Desktop installed and running
- 4GB+ disk space for Docker image

**Advantages:**
- Build on any operating system
- No DevEco Studio required
- Consistent build environment
- Pre-configured OHOS SDK

**Limitations:**
- Cannot run on OpenHarmony devices
- Cannot use DevEco Studio integration
- Larger initial download (~1.5GB image)

See [Docker Builds](#docker-builds) section for details.

## Quick Start

### Basic Build

```bash
# Build for default architecture (arm64-v8a)
ccgo build ohos

# Build with Docker (cross-compile from any OS)
ccgo build ohos --docker

# Specify architecture
ccgo build ohos --arch armeabi-v7a      # 32-bit ARM
ccgo build ohos --arch arm64-v8a        # 64-bit ARM (default)
ccgo build ohos --arch x86_64           # x86_64 emulator

# Build multiple architectures
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64

# Build types
ccgo build ohos --build-type debug      # Debug build
ccgo build ohos --build-type release    # Release build (default)

# Link types
ccgo build ohos --link-type static      # Static library only
ccgo build ohos --link-type shared      # Shared library only
ccgo build ohos --link-type both        # Both types (default)

# Generate HAR package
ccgo build ohos --har                   # Creates .har package
```

### Generate DevEco Studio Project

```bash
# Generate DevEco Studio project
ccgo build ohos --ide-project

# Open in DevEco Studio
# File -> Open -> cmake_build/ohos/
```

## Output Structure

### Default Output (`target/ohos/`)

```
target/ohos/
├── MyLib_OHOS_SDK-1.0.0.zip            # Main package
│   ├── lib/
│   │   ├── static/
│   │   │   ├── armeabi-v7a/
│   │   │   │   └── libmylib.a          # 32-bit ARM static
│   │   │   ├── arm64-v8a/
│   │   │   │   └── libmylib.a          # 64-bit ARM static
│   │   │   └── x86_64/
│   │   │       └── libmylib.a          # x86_64 static
│   │   └── shared/
│   │       ├── armeabi-v7a/
│   │       │   └── libmylib.so         # 32-bit ARM shared
│   │       ├── arm64-v8a/
│   │       │   └── libmylib.so         # 64-bit ARM shared
│   │       └── x86_64/
│   │           └── libmylib.so         # x86_64 shared
│   ├── haars/
│   │   └── mylib-1.0.0.har             # HAR package
│   ├── include/
│   │   └── mylib/                      # Header files
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                 # Build metadata
│
└── MyLib_OHOS_SDK-1.0.0-SYMBOLS.zip    # Debug symbols
    └── obj/
        ├── armeabi-v7a/
        │   └── libmylib.so             # Unstripped library
        ├── arm64-v8a/
        │   └── libmylib.so
        └── x86_64/
            └── libmylib.so
```

### HAR Package

HAR (Harmony Archive) is OpenHarmony's library package format:

**Structure:**
```
mylib-1.0.0.har
├── libs/
│   ├── armeabi-v7a/
│   │   └── libmylib.so
│   ├── arm64-v8a/
│   │   └── libmylib.so
│   └── x86_64/
│       └── libmylib.so
├── include/
│   └── mylib/
│       ├── mylib.h
│       └── version.h
├── oh-package.json5               # Package metadata
└── module.json5                   # Module configuration
```

**oh-package.json5:**
```json5
{
  "name": "mylib",
  "version": "1.0.0",
  "description": "My OpenHarmony library",
  "main": "index.ets",
  "author": "Your Name",
  "license": "MIT",
  "dependencies": {},
  "devDependencies": {}
}
```

### Build Metadata

`build_info.json` contains:

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My OpenHarmony library"
  },
  "build": {
    "platform": "ohos",
    "architectures": ["armeabi-v7a", "arm64-v8a", "x86_64"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "ohos_sdk_version": "10",
    "api_version": "10"
  },
  "outputs": {
    "libraries": {
      "static": {
        "armeabi-v7a": "lib/static/armeabi-v7a/libmylib.a",
        "arm64-v8a": "lib/static/arm64-v8a/libmylib.a",
        "x86_64": "lib/static/x86_64/libmylib.a"
      },
      "shared": {
        "armeabi-v7a": "lib/shared/armeabi-v7a/libmylib.so",
        "arm64-v8a": "lib/shared/arm64-v8a/libmylib.so",
        "x86_64": "lib/shared/x86_64/libmylib.so"
      }
    },
    "har": "haars/mylib-1.0.0.har",
    "headers": "include/mylib/"
  }
}
```

## Using Libraries in OpenHarmony

### In ArkTS/eTS Application

**1. Add HAR dependency:**

```json5
// oh-package.json5
{
  "dependencies": {
    "mylib": "file:../mylib-1.0.0.har"
  }
}
```

**2. Create native module wrapper:**

```cpp
// src/native/mylib_napi.cpp
#include <napi/native_api.h>
#include <mylib/mylib.h>

static napi_value DoWork(napi_env env, napi_callback_info info) {
    mylib::MyClass obj;
    obj.do_work();

    napi_value result;
    napi_create_int32(env, 0, &result);
    return result;
}

EXTERN_C_START
static napi_value Init(napi_env env, napi_value exports) {
    napi_property_descriptor desc[] = {
        { "doWork", nullptr, DoWork, nullptr, nullptr, nullptr, napi_default, nullptr }
    };
    napi_define_properties(env, exports, sizeof(desc) / sizeof(desc[0]), desc);
    return exports;
}
EXTERN_C_END

static napi_module myLibModule = {
    .nm_version = 1,
    .nm_flags = 0,
    .nm_filename = nullptr,
    .nm_register_func = Init,
    .nm_modname = "mylib",
    .nm_priv = nullptr,
    .reserved = { 0 },
};

extern "C" __attribute__((constructor)) void RegisterMyLibModule() {
    napi_module_register(&myLibModule);
}
```

**3. Use in ArkTS:**

```typescript
// src/main/ets/pages/Index.ets
import mylib from 'libmylib.so';

@Entry
@Component
struct Index {
  build() {
    Button('Do Work')
      .onClick(() => {
        mylib.doWork();
      })
  }
}
```

### In C++ Application

**CMakeLists.txt:**

```cmake
# Link static library
target_link_libraries(myapp PRIVATE
    ${CMAKE_SOURCE_DIR}/libs/${OHOS_ARCH}/libmylib.a
)

target_include_directories(myapp PRIVATE
    ${CMAKE_SOURCE_DIR}/include
)
```

**Direct usage:**

```cpp
#include <mylib/mylib.h>

int main() {
    mylib::MyClass obj;
    obj.do_work();
    return 0;
}
```

## Docker Builds

Build OpenHarmony libraries on any OS using Docker:

### Prerequisites

```bash
# Install Docker Desktop
# Download from: https://www.docker.com/products/docker-desktop/

# Verify Docker is running
docker ps
```

### Build with Docker

```bash
# First build downloads prebuilt image (~1.5GB)
ccgo build ohos --docker

# Subsequent builds are fast
ccgo build ohos --docker --arch arm64-v8a

# All standard options work
ccgo build ohos --docker --arch armeabi-v7a,arm64-v8a --har
```

### How It Works

1. CCGO uses prebuilt `ccgo-builder-ohos` image from Docker Hub
2. Project directory mounted into container
3. Build runs with OHOS SDK and toolchain
4. Output written to host filesystem

### Limitations

- **Cannot run**: No OpenHarmony runtime in Docker
- **Cannot test**: No device or emulator access
- **No DevEco Studio**: Cannot generate IDE projects

## Publishing to OHPM

OHPM (OpenHarmony Package Manager) is the official package registry.

### Setup

```bash
# Login to OHPM
ohpm login

# Configure registry (if using private registry)
ohpm config set registry https://your-registry.com
```

### Publish

```bash
# Publish HAR to official OHPM registry
ccgo publish ohos --registry official

# Publish to private registry
ccgo publish ohos --registry private --url https://your-registry.com

# Skip build and publish existing HAR
ccgo publish ohos --skip-build
```

### Package Configuration

Ensure `CCGO.toml` has correct metadata:

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My OpenHarmony library"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"
homepage = "https://github.com/yourusername/mylib"
repository = "https://github.com/yourusername/mylib"

[ohos]
min_api_version = 9
target_api_version = 10
```

## Platform Configuration

### CCGO.toml Settings

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"                      # static, shared, or both

[build]
cpp_standard = "17"                # C++ standard

[ohos]
min_api_version = 9                # Minimum API version
target_api_version = 10            # Target API version
compile_sdk_version = 10           # Compile SDK version
ndk_version = "4.0.0"              # NDK version
```

### CMake Variables

When building for OpenHarmony:

```cmake
${PLATFORM}                        # "ohos"
${OHOS_ARCH}                       # "armeabi-v7a", "arm64-v8a", or "x86_64"
${BUILD_TYPE}                      # "Debug" or "Release"
${LINK_TYPE}                       # "static", "shared", or "both"
${OHOS_API_VERSION}                # Target API version
${OHOS_SDK_HOME}                   # OHOS SDK path
```

### Conditional Compilation

```cpp
// Platform detection
#ifdef __OHOS__
    // OpenHarmony-specific code
    #include <hilog/log.h>

    #ifdef __aarch64__
        // ARM64-specific code
    #elif defined(__arm__)
        // ARMv7-specific code
    #elif defined(__x86_64__)
        // x86_64-specific code
    #endif
#endif

// API version detection
#if __OHOS_API_VERSION__ >= 10
    // API 10+ features
#else
    // Fallback for older APIs
#endif

// Logging
#ifdef __OHOS__
    #define LOG_TAG "MyLib"
    #define LOG_INFO(fmt, ...) \
        OH_LOG_INFO(LOG_APP, fmt, ##__VA_ARGS__)
#else
    #define LOG_INFO(fmt, ...) \
        printf(fmt "\n", ##__VA_ARGS__)
#endif
```

## Best Practices

### 1. Use HAR Packages

Package libraries as HAR for easy distribution:

```bash
# Always generate HAR
ccgo build ohos --har
```

### 2. Support Multiple Architectures

Build for all common architectures:

```bash
# Build all architectures
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64
```

### 3. Implement NAPI Wrappers

Provide ArkTS/eTS bindings for C++ code:

```cpp
// Always wrap native code with NAPI
static napi_value ExportFunction(napi_env env, napi_callback_info info) {
    // Implementation
}
```

### 4. Use HiLog for Logging

Use OpenHarmony's logging system:

```cpp
#include <hilog/log.h>

#define LOG_DOMAIN 0x0001
#define LOG_TAG "MyLib"

void log_message() {
    OH_LOG_INFO(LOG_APP, "Message from MyLib");
}
```

### 5. Handle API Versioning

Check API version at runtime:

```cpp
#include <parameter/system_parameter.h>

int get_api_version() {
    char value[32];
    int ret = GetParameter("const.ohos.apiversion", "", value, sizeof(value));
    if (ret > 0) {
        return atoi(value);
    }
    return 0;
}
```

### 6. Test on Real Devices

Always test on physical OpenHarmony devices:
- Different API versions (9, 10, 11+)
- Different architectures (ARM32, ARM64)
- Different OEMs and device types

### 7. Minimize Library Size

Reduce HAR size for faster downloads:

```toml
[build]
cxxflags = ["-Os", "-flto"]        # Optimize for size
strip_symbols = true               # Strip debug symbols
```

## Troubleshooting

### OHOS SDK Not Found

```
Error: OHOS SDK not found
```

**Solution:**

```bash
# Set OHOS SDK path
export OHOS_SDK_HOME="/path/to/ohos-sdk"

# Or in CCGO.toml
[ohos]
sdk_path = "/path/to/ohos-sdk"

# Verify
ls $OHOS_SDK_HOME/native
```

### HAR Import Failed

```
Error: Failed to import HAR package
```

**Solutions:**

1. **Check oh-package.json5:**
```json5
{
  "dependencies": {
    "mylib": "file:../path/to/mylib-1.0.0.har"  // Use correct path
  }
}
```

2. **Verify HAR structure:**
```bash
unzip -l mylib-1.0.0.har
# Should contain: libs/, include/, oh-package.json5
```

3. **Reinstall dependencies:**
```bash
ohpm install
```

### NAPI Symbol Not Found

```
Error: Cannot find module 'libmylib.so'
```

**Solutions:**

1. **Check library is in HAR:**
```bash
unzip -l mylib-1.0.0.har | grep libmylib.so
```

2. **Verify module name matches:**
```cpp
// In NAPI code
.nm_modname = "mylib",  // Must match import name
```

3. **Check build.gradle:**
```groovy
externalNativeBuild {
    cmake {
        targets "mylib"  // Module name
    }
}
```

### API Version Mismatch

```
Error: Minimum API version not met
```

**Solution:**

Update device or lower minimum API version:

```toml
[ohos]
min_api_version = 9  # Lower to support more devices
```

### Architecture Not Supported

```
Error: No native library found for architecture
```

**Solution:**

Build for missing architecture:

```bash
# Build all architectures
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64
```

## Performance Tips

### 1. Use ARM NEON Instructions

Enable NEON optimizations for ARM:

```toml
[build]
cxxflags = ["-mfpu=neon", "-mfloat-abi=softfp"]  # ARMv7
# ARM64 has NEON enabled by default
```

### 2. Link-Time Optimization

```toml
[build]
cxxflags = ["-flto"]
ldflags = ["-flto"]
```

### 3. Optimize for Size

OpenHarmony devices often have limited storage:

```toml
[build]
cxxflags = ["-Os", "-ffunction-sections", "-fdata-sections"]
ldflags = ["-Wl,--gc-sections"]
```

### 4. Static Linking for Performance

Static libraries can be faster:

```bash
ccgo build ohos --link-type static
```

### 5. Profile on Device

Use HiPerf for profiling:

```bash
# On device
hiperfcmd start -o perf.data
# Run app
hiperfcmd stop
hiperfcmd report -i perf.data
```

## Migration Guides

### From Native C++ Module

**Before:**
```cpp
// Standalone C++ module
namespace mylib {
    void do_work();
}
```

**After:**

1. Create CCGO project:
```bash
ccgo new mylib
```

2. Add NAPI wrapper:
```cpp
// Add NAPI bindings
static napi_value DoWork(napi_env env, napi_callback_info info) {
    mylib::do_work();
    return nullptr;
}
```

3. Build HAR:
```bash
ccgo build ohos --har
```

### From Android NDK Library

Many similarities with Android:

**Differences:**
- Use HAR instead of AAR
- Use OHPM instead of Maven
- Use NAPI instead of JNI
- Different build system (Hvigor vs Gradle)

**Migration steps:**

1. Copy C++ code to CCGO project
2. Update build configuration:
```toml
[ohos]
min_api_version = 9
```

3. Replace JNI with NAPI:
```cpp
// JNI
JNIEXPORT jint JNICALL Java_com_example_MyLib_doWork(JNIEnv* env, jobject obj)

// NAPI
static napi_value DoWork(napi_env env, napi_callback_info info)
```

4. Build:
```bash
ccgo build ohos --har
```

## Advanced Topics

### Multi-Module HAR

Create HAR with multiple modules:

```
mylib/
├── core/              # Core module
│   ├── src/
│   └── include/
├── ui/                # UI module
│   ├── src/
│   └── include/
└── CCGO.toml
```

```toml
[package]
name = "mylib"

[[modules]]
name = "core"
path = "core"

[[modules]]
name = "ui"
path = "ui"
dependencies = ["core"]
```

### Embedding Resources

Include resources in HAR:

```
mylib-1.0.0.har
├── libs/
├── include/
├── resources/
│   ├── base/
│   │   └── element/
│   └── rawfile/
│       └── data.bin
└── oh-package.json5
```

### Code Signing

Sign HAR for distribution:

```bash
# Generate key
hapsigner generate-keypair -keyAlias mylib -keyAlg RSA

# Sign HAR
hapsigner sign-app -mode localSign \
    -keyAlias mylib \
    -signAlg SHA256withRSA \
    -inputFile mylib-1.0.0.har \
    -outputFile mylib-1.0.0-signed.har
```

### Obfuscation

Protect C++ code:

```toml
[build]
cxxflags = ["-fvisibility=hidden", "-ffunction-sections"]
strip_symbols = true
```

## See Also

- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Publishing Management](../features/publishing.md)
- [Docker Builds](../features/docker-builds.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Platforms Overview](index.md)
