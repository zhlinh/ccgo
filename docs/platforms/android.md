# Android Development

Complete guide for building C++ libraries for Android with CCGO.

## Overview

CCGO provides comprehensive Android support with:
- **Multi-architecture builds**: arm64-v8a, armeabi-v7a, x86, x86_64
- **AAR packaging**: Ready-to-use Android Archive format
- **Gradle integration**: Seamless integration with Android Studio projects
- **JNI support**: Automatic JNI wrapper generation
- **Maven publishing**: Publish to Maven Local, Central, or private repositories
- **Docker builds**: Build without local Android SDK/NDK installation

## Prerequisites

### Option 1: Local Development

**Required:**
- Android SDK (API level 21+)
- Android NDK (r21+, recommended: r25+)
- CMake (3.18+)
- Python (3.8+)

**Installation:**

```bash
# Install Android Studio (includes SDK)
# Download from https://developer.android.com/studio

# Set environment variables
export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK=$ANDROID_HOME/ndk/25.2.9519653

# Verify installation
ccgo check android --verbose
```

### Option 2: Docker-Based Development

**Required:**
- Docker Desktop

```bash
# Build with Docker (no local SDK/NDK needed)
ccgo build android --docker
```

First build downloads prebuilt image (~3.5GB, 5-10 minutes). Subsequent builds use cached image.

## Quick Start

### Create New Project

```bash
# Create new Android-compatible project
ccgo new my-android-lib
cd my-android-lib/my-android-lib

# Build for Android
ccgo build android
```

### Build Single Architecture

```bash
# Build for arm64-v8a only
ccgo build android --arch arm64-v8a
```

### Build Multiple Architectures

```bash
# Build for arm64-v8a and armeabi-v7a
ccgo build android --arch arm64-v8a,armeabi-v7a

# Build all architectures (default)
ccgo build android
```

### Build Options

```bash
# Release build (optimized)
ccgo build android --release

# Debug build (with symbols)
ccgo build android --debug

# Clean build
ccgo build android --clean

# Docker build
ccgo build android --docker

# Link type control
ccgo build android --link-type static   # Static library only
ccgo build android --link-type shared   # Shared library only
ccgo build android --link-type both     # Both (default)
```

## Output Structure

After building, find artifacts in `target/android/`:

```
target/android/
├── MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip      # Main package
├── MY-ANDROID-LIB_ANDROID_SDK-1.0.0-SYMBOLS.zip  # Debug symbols
└── build_info.json                            # Build metadata
```

### Main Package Structure

```
MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip
├── lib/
│   ├── static/
│   │   ├── armeabi-v7a/
│   │   │   └── libmy-android-lib.a
│   │   ├── arm64-v8a/
│   │   │   └── libmy-android-lib.a
│   │   ├── x86/
│   │   │   └── libmy-android-lib.a
│   │   └── x86_64/
│   │       └── libmy-android-lib.a
│   └── shared/
│       ├── armeabi-v7a/
│       │   └── libmy-android-lib.so
│       ├── arm64-v8a/
│       │   └── libmy-android-lib.so
│       ├── x86/
│       │   └── libmy-android-lib.so
│       └── x86_64/
│           └── libmy-android-lib.so
├── haars/
│   └── my-android-lib-release.aar
├── include/
│   └── my-android-lib/
│       ├── my-android-lib.h
│       └── version.h
└── build_info.json
```

### Symbols Package Structure

```
MY-ANDROID-LIB_ANDROID_SDK-1.0.0-SYMBOLS.zip
└── obj/
    ├── armeabi-v7a/
    │   └── libmy-android-lib.so  # Unstripped with debug symbols
    ├── arm64-v8a/
    │   └── libmy-android-lib.so
    ├── x86/
    │   └── libmy-android-lib.so
    └── x86_64/
        └── libmy-android-lib.so
```

## Configuration

### CCGO.toml

Configure Android-specific settings:

```toml
[package]
name = "my-android-lib"
version = "1.0.0"

[library]
type = "both"  # Build both static and shared libraries

[android]
min_sdk_version = 21          # Android 5.0 (Lollipop)
target_sdk_version = 33       # Android 13
ndk_version = "25.2.9519653"  # Specific NDK version
stl = "c++_static"            # STL type: c++_static or c++_shared
architectures = ["arm64-v8a", "armeabi-v7a", "x86_64"]  # Optional: limit architectures

[build]
cpp_standard = "17"
compile_flags = ["-Wall", "-Wextra"]
```

### Android Configuration Options

| Option | Type | Description | Default |
|--------|------|-------------|---------|
| `min_sdk_version` | integer | Minimum Android API level | `21` |
| `target_sdk_version` | integer | Target Android API level | `33` |
| `ndk_version` | string | Specific NDK version | Latest installed |
| `stl` | string | STL type: `c++_static`, `c++_shared` | `c++_static` |
| `architectures` | array | Target ABIs to build | All supported |

### Supported Architectures

| ABI | Architecture | Description |
|-----|-------------|-------------|
| `arm64-v8a` | ARM 64-bit | Modern Android devices (recommended) |
| `armeabi-v7a` | ARM 32-bit | Legacy Android devices |
| `x86_64` | Intel 64-bit | Emulators, tablets, Chrome OS |
| `x86` | Intel 32-bit | Legacy emulators |

**Recommendation:** Build `arm64-v8a` and `armeabi-v7a` for production apps.

## AAR Integration

### Using AAR in Android Project

**1. Copy AAR to project:**

```bash
# Copy AAR from build output
cp target/android/MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip .
unzip MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip
cp haars/my-android-lib-release.aar android-app/libs/
```

**2. Configure app/build.gradle.kts:**

```kotlin
android {
    // ...
}

dependencies {
    implementation(fileTree(mapOf("dir" to "libs", "include" to listOf("*.aar"))))
    // or
    implementation(files("libs/my-android-lib-release.aar"))
}
```

**3. Use in Java/Kotlin:**

```kotlin
class MainActivity : AppCompatActivity() {
    companion object {
        init {
            System.loadLibrary("my-android-lib")
        }
    }

    // Declare native methods
    external fun nativeMethod(): String

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Call native method
        val result = nativeMethod()
        Log.d("Native", "Result: $result")
    }
}
```

## JNI Integration

### Automatic JNI Wrapper

CCGO can generate JNI wrappers automatically:

**C++ Header (include/my-android-lib/my-android-lib.h):**

```cpp
#pragma once

#include <string>

namespace my_android_lib {

class MyLib {
public:
    static std::string get_version();
    static int calculate(int a, int b);
};

} // namespace my_android_lib
```

**Generated JNI Wrapper (auto-generated):**

```cpp
#include <jni.h>
#include "my-android-lib/my-android-lib.h"

extern "C" {

JNIEXPORT jstring JNICALL
Java_com_example_mylib_MyLib_getVersion(JNIEnv* env, jclass) {
    std::string version = my_android_lib::MyLib::get_version();
    return env->NewStringUTF(version.c_str());
}

JNIEXPORT jint JNICALL
Java_com_example_mylib_MyLib_calculate(JNIEnv*, jclass, jint a, jint b) {
    return my_android_lib::MyLib::calculate(a, b);
}

} // extern "C"
```

### Manual JNI Implementation

**Create src/jni/my_jni.cpp:**

```cpp
#include <jni.h>
#include <string>
#include "my-android-lib/my-android-lib.h"

extern "C" {

JNIEXPORT jstring JNICALL
Java_com_example_MyNativeLib_stringFromJNI(JNIEnv* env, jobject /* this */) {
    std::string hello = my_android_lib::MyLib::get_version();
    return env->NewStringUTF(hello.c_str());
}

JNIEXPORT jint JNICALL
Java_com_example_MyNativeLib_add(JNIEnv*, jobject, jint a, jint b) {
    return my_android_lib::MyLib::calculate(a, b);
}

} // extern "C"
```

**Java/Kotlin side:**

```kotlin
package com.example

class MyNativeLib {
    external fun stringFromJNI(): String
    external fun add(a: Int, b: Int): Int

    companion object {
        init {
            System.loadLibrary("my-android-lib")
        }
    }
}
```

## Gradle Integration

### Using CCGO Gradle Plugins

CCGO provides Gradle convention plugins for standardized Android builds.

**settings.gradle.kts:**

```kotlin
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}
```

**app/build.gradle.kts:**

```kotlin
plugins {
    id("com.android.library")
    id("com.mojeter.ccgo.gradle.android.library")
    id("com.mojeter.ccgo.gradle.android.library.native")
}

android {
    namespace = "com.example.mylib"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
    }

    ndkVersion = "25.2.9519653"
}

ccgoNative {
    projectPath.set(file("../"))  // Path to CCGO project root
    buildType.set("release")       // or "debug"
    architectures.set(listOf("arm64-v8a", "armeabi-v7a"))
}
```

## Publishing

### Maven Local (Development)

```bash
# Publish to Maven Local for testing
ccgo publish android --registry local

# Location: ~/.m2/repository/com/example/my-android-lib/1.0.0/
```

### Maven Central (Production)

**1. Configure credentials:**

Create `~/.gradle/gradle.properties`:

```properties
mavenCentralUsername=your-username
mavenCentralPassword=your-password
signing.keyId=12345678
signing.password=your-key-password
signing.secretKeyRingFile=/Users/you/.gnupg/secring.gpg
```

**2. Publish:**

```bash
ccgo publish android --registry official
```

### Private Maven Repository

```bash
ccgo publish android --registry private \
    --url https://maven.example.com/releases
```

### Using Published Library

**app/build.gradle.kts:**

```kotlin
dependencies {
    implementation("com.example:my-android-lib:1.0.0")
}
```

## Advanced Topics

### Multi-Module Projects

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

**lib2/CCGO.toml:**

```toml
[dependencies]
lib1 = { path = "../lib1" }
```

### Custom CMake Configuration

**CMakeLists.txt:**

```cmake
cmake_minimum_required(VERSION 3.18)

# CCGO automatically provides:
# - ${CCGO_CMAKE_DIR}: Path to CCGO cmake utilities
# - ${ANDROID_ABI}: Current architecture being built
# - ${ANDROID_PLATFORM}: Android API level

include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# Custom Android-specific configuration
if(ANDROID)
    # Add Android-specific compiler flags
    add_compile_options(-fPIC)

    # Link against Android libraries
    find_library(LOG_LIB log)
    find_library(ANDROID_LIB android)

    target_link_libraries(${PROJECT_NAME}
        ${LOG_LIB}
        ${ANDROID_LIB}
    )
endif()
```

### Proguard Rules

**Create proguard-rules.pro:**

```proguard
# Keep native methods
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep JNI exported methods
-keep class com.example.mylib.** { *; }
```

### App Size Optimization

**1. Strip unneeded symbols (automatic in release builds):**

```bash
ccgo build android --release
```

**2. Use only required architectures:**

```toml
[android]
architectures = ["arm64-v8a"]  # Drop 32-bit support if not needed
```

**3. Enable link-time optimization:**

```toml
[build]
link_flags = ["-flto"]
```

**4. Split APKs by ABI:**

**app/build.gradle.kts:**

```kotlin
android {
    splits {
        abi {
            isEnable = true
            reset()
            include("arm64-v8a", "armeabi-v7a")
            isUniversalApk = false
        }
    }
}
```

## Troubleshooting

### Common Issues

#### NDK Not Found

```
Error: Android NDK not found
```

**Solution:**

```bash
# Install NDK via Android Studio: Tools → SDK Manager → SDK Tools → NDK

# Or set manually
export ANDROID_NDK=$ANDROID_HOME/ndk/25.2.9519653

# Verify
ccgo check android --verbose
```

#### Architecture Mismatch

```
Error: UnsatisfiedLinkError: dlopen failed: library "libmy-android-lib.so" not found
```

**Solution:**

Ensure AAR contains the architecture your device/emulator uses:

```bash
# Check AAR contents
unzip -l my-android-lib-release.aar | grep "\.so$"

# Rebuild with correct architecture
ccgo build android --arch arm64-v8a
```

#### C++ Standard Mismatch

```
Error: undefined reference to std::__cxx11::...
```

**Solution:**

Ensure consistent C++ standard across dependencies:

```toml
[build]
cpp_standard = "17"  # Match with dependencies

[android]
stl = "c++_static"  # Or c++_shared
```

#### Missing Symbols

```
Error: undefined reference to 'my_function'
```

**Solution:**

Check that all source files are compiled:

```bash
# Enable verbose build
ccgo build android --verbose

# Check CMakeLists.txt includes all sources
```

### Docker Build Issues

#### Docker Not Running

```
Error: Cannot connect to the Docker daemon
```

**Solution:**

```bash
# Start Docker Desktop
open -a Docker  # macOS

# Verify
docker ps
```

#### Image Pull Failure

```
Error: failed to pull image ccgo-builder-android
```

**Solution:**

```bash
# Retry with manual pull
docker pull ccgogroup/ccgo-builder-android:latest

# Or use local build
cd ccgo/dockers/
docker build -t ccgo-builder-android -f Dockerfile.android .
```

### Performance Issues

#### Slow First Build

**Normal:** First build compiles all dependencies (~10-30 minutes).

**Optimization:**

```bash
# Use prebuilt dependencies (future feature)
ccgo install --prebuilt

# Enable ccache
export USE_CCACHE=1
export CCACHE_DIR=$HOME/.ccache
```

#### Incremental Build Not Working

```bash
# Clean CMake cache
rm -rf cmake_build/android/

# Rebuild
ccgo build android
```

## Best Practices

### 1. Version Management

```toml
[package]
version = "1.0.0"  # Update before release
```

```bash
# Create git tag
ccgo tag v1.0.0 --push
```

### 2. Architecture Selection

```toml
[android]
# Production: arm64-v8a + armeabi-v7a (covers 99%+ devices)
architectures = ["arm64-v8a", "armeabi-v7a"]

# Development: arm64-v8a only (faster builds)
# architectures = ["arm64-v8a"]
```

### 3. STL Choice

```toml
[android]
# Prefer c++_static (no runtime dependency)
stl = "c++_static"

# Use c++_shared only if multiple native libraries share STL
# stl = "c++_shared"
```

### 4. Dependency Management

```toml
[dependencies]
# Pin to specific versions for reproducibility
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Use CCGO.lock for exact dependency resolution
```

### 5. Testing

```bash
# Build and test locally
ccgo build android --arch arm64-v8a
ccgo test android

# Test AAR in sample app before publishing
```

### 6. CI/CD

```yaml
# .github/workflows/android.yml
name: Android Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Android
        run: |
          pip install ccgo
          ccgo build android --docker

      - name: Test
        run: ccgo test android --docker

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: android-libs
          path: target/android/*.zip
```

## Examples

### Complete Project

See [ccgo-now](https://github.com/zhlinh/ccgo-now) for a complete Android project example.

### Minimal Example

```cpp
// include/mylib/mylib.h
#pragma once
#include <string>

namespace mylib {
    std::string get_greeting();
    int add(int a, int b);
}

// src/mylib.cpp
#include "mylib/mylib.h"

namespace mylib {
    std::string get_greeting() {
        return "Hello from C++!";
    }

    int add(int a, int b) {
        return a + b;
    }
}
```

**Build:**

```bash
ccgo build android --arch arm64-v8a
```

**Use in Android:**

```kotlin
class MainActivity : AppCompatActivity() {
    init {
        System.loadLibrary("mylib")
    }

    external fun getGreeting(): String
    external fun add(a: Int, b: Int): Int

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val greeting = getGreeting()
        val result = add(2, 3)

        Log.d("MyLib", "$greeting, 2+3=$result")
    }
}
```

## Resources

### Official Documentation

- [Android NDK Documentation](https://developer.android.com/ndk)
- [JNI Specification](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/)
- [CMake Android Guide](https://developer.android.com/ndk/guides/cmake)

### CCGO Documentation

- [CLI Reference](../reference/cli.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Publishing Guide](../features/publishing.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [Build System Overview](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [iOS Development](ios.md)
- [OpenHarmony Development](openharmony.md)
