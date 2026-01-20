# Kotlin Multiplatform

Complete guide to building Kotlin Multiplatform (KMP) libraries with native C++ code using CCGO.

## Overview

CCGO enables seamless integration of C++ libraries into Kotlin Multiplatform projects, allowing you to:

- **Share C++ code** across all KMP platforms (Android, iOS, macOS, Linux, Windows)
- **Unified build system** - Build native libraries for all platforms with a single command
- **Type-safe bindings** - Generate Kotlin expect/actual declarations for C++ APIs
- **Native performance** - Direct JNI/Objective-C interop without overhead
- **Gradle integration** - First-class support in KMP Gradle builds

**Supported KMP targets:**
- Android (ARM64, ARMv7, x86_64, x86)
- iOS (arm64, simulator)
- macOS (x86_64, arm64)
- Linux (x86_64)
- Windows (x86_64)

## Prerequisites

### Required Tools

```bash
# Install CCGO
pip install ccgo

# Verify installation
ccgo --version
```

### Platform SDKs

| Platform | Requirements |
|----------|-------------|
| Android | Android SDK, NDK 21+ |
| iOS | macOS with Xcode |
| macOS | macOS with Xcode |
| Linux | GCC or Clang |
| Windows | Visual Studio or MinGW |

## Quick Start

### Create New KMP Project

```bash
# Create new KMP project with C++ support
ccgo new my-kmp-lib

# Navigate to project
cd my-kmp-lib

# Build for all platforms
ccgo build kmp
```

### Project Structure

```
my-kmp-lib/
├── CCGO.toml                    # CCGO configuration
├── build.gradle.kts             # Root Gradle build
├── settings.gradle.kts
├── src/
│   ├── commonMain/
│   │   └── kotlin/
│   │       └── com/example/
│   │           └── MyLib.kt     # Kotlin expect declarations
│   ├── androidMain/
│   │   └── kotlin/
│   │       └── com/example/
│   │           └── MyLib.android.kt  # Android actual (JNI)
│   ├── iosMain/
│   │   └── kotlin/
│   │       └── com/example/
│   │           └── MyLib.ios.kt      # iOS actual (Objective-C)
│   └── nativeMain/             # Desktop platforms
│       └── kotlin/
│           └── com/example/
│               └── MyLib.native.kt   # cinterop bindings
├── cpp/
│   ├── include/
│   │   └── mylib/
│   │       └── mylib.h         # C++ public headers
│   ├── src/
│   │   └── mylib.cpp           # C++ implementation
│   ├── jni/                    # JNI wrappers for Android
│   │   └── mylib_jni.cpp
│   └── objc/                   # Objective-C wrappers for iOS
│       ├── MyLibWrapper.h
│       └── MyLibWrapper.mm
└── target/                     # Build outputs
    ├── android/
    ├── ios/
    ├── macos/
    ├── linux/
    └── windows/
```

## Configuration

### CCGO.toml

```toml
[package]
name = "mylib"
version = "1.0.0"
type = "kmp"

[kmp]
# Kotlin package name
package_name = "com.example.mylib"

# KMP targets
targets = ["android", "ios", "macos", "linux", "windows"]

# Android configuration
[kmp.android]
min_sdk = 21
target_sdk = 33
namespace = "com.example.mylib"

# iOS configuration
[kmp.ios]
min_deployment_target = "12.0"
framework_name = "MyLib"

# macOS configuration
[kmp.macos]
min_deployment_target = "10.14"

[dependencies]
# C++ dependencies
cpp = [
    { name = "openssl", version = "1.1.1" }
]
```

### build.gradle.kts

```kotlin
plugins {
    kotlin("multiplatform") version "1.9.20"
    id("com.android.library") version "8.1.0"
}

kotlin {
    // Android target
    androidTarget {
        compilations.all {
            kotlinOptions {
                jvmTarget = "1.8"
            }
        }
    }

    // iOS targets
    listOf(
        iosX64(),
        iosArm64(),
        iosSimulatorArm64()
    ).forEach {
        it.binaries.framework {
            baseName = "MyLib"
        }
    }

    // macOS
    macosX64()
    macosArm64()

    // Linux
    linuxX64()

    // Windows (via MinGW)
    mingwX64()

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation("org.jetbrains.kotlin:kotlin-stdlib:1.9.20")
            }
        }

        val androidMain by getting {
            dependencies {
                // JNI bindings automatically included
            }
        }

        val iosMain by getting {
            dependencies {
                // Framework bindings automatically included
            }
        }

        val nativeMain by getting {
            dependencies {
                // cinterop bindings for desktop platforms
            }
        }
    }
}

android {
    namespace = "com.example.mylib"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
    }

    // Link native libraries built by CCGO
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("${projectDir}/target/android")
        }
    }
}
```

## Building KMP Libraries

### Build All Platforms

```bash
# Build native code for all KMP targets
ccgo build kmp

# Output structure:
# target/
# ├── android/
# │   ├── armeabi-v7a/libmylib.so
# │   ├── arm64-v8a/libmylib.so
# │   └── x86_64/libmylib.so
# ├── ios/
# │   └── MyLib.framework
# ├── macos/
# │   └── libmylib.dylib
# ├── linux/
# │   └── libmylib.so
# └── windows/
#     └── mylib.dll
```

### Build Specific Platform

```bash
# Android only
ccgo build kmp --target android

# iOS only
ccgo build kmp --target ios

# Desktop platforms
ccgo build kmp --target macos
ccgo build kmp --target linux
ccgo build kmp --target windows
```

### Gradle Integration

```bash
# Build Kotlin code and native libraries
./gradlew build

# Publish to Maven Local
./gradlew publishToMavenLocal

# Create iOS XCFramework
./gradlew linkReleaseFrameworkIos
```

## Native Interop

### Android (JNI)

**C++ Header:**
```cpp
// include/mylib/mylib.h
#pragma once
#include <string>

namespace mylib {
    class Calculator {
    public:
        static int add(int a, int b);
        static std::string greet(const std::string& name);
    };
}
```

**JNI Wrapper:**
```cpp
// cpp/jni/mylib_jni.cpp
#include <jni.h>
#include "mylib/mylib.h"

extern "C" {

JNIEXPORT jint JNICALL
Java_com_example_mylib_Calculator_add(JNIEnv* env, jclass clazz,
                                       jint a, jint b) {
    return mylib::Calculator::add(a, b);
}

JNIEXPORT jstring JNICALL
Java_com_example_mylib_Calculator_greet(JNIEnv* env, jclass clazz,
                                         jstring name) {
    const char* cName = env->GetStringUTFChars(name, nullptr);
    std::string result = mylib::Calculator::greet(cName);
    env->ReleaseStringUTFChars(name, cName);
    return env->NewStringUTF(result.c_str());
}

} // extern "C"
```

**Kotlin Expect/Actual:**
```kotlin
// commonMain/kotlin/Calculator.kt
expect object Calculator {
    fun add(a: Int, b: Int): Int
    fun greet(name: String): String
}

// androidMain/kotlin/Calculator.android.kt
actual object Calculator {
    init {
        System.loadLibrary("mylib")
    }

    actual external fun add(a: Int, b: Int): Int
    actual external fun greet(name: String): String
}
```

### iOS (Objective-C++)

**Objective-C++ Wrapper:**
```objc
// cpp/objc/MyLibWrapper.h
#import <Foundation/Foundation.h>

@interface MyLibCalculator : NSObject
+ (NSInteger)add:(NSInteger)a b:(NSInteger)b;
+ (NSString*)greet:(NSString*)name;
@end

// cpp/objc/MyLibWrapper.mm
#import "MyLibWrapper.h"
#include "mylib/mylib.h"

@implementation MyLibCalculator

+ (NSInteger)add:(NSInteger)a b:(NSInteger)b {
    return mylib::Calculator::add((int)a, (int)b);
}

+ (NSString*)greet:(NSString*)name {
    std::string result = mylib::Calculator::greet([name UTF8String]);
    return [NSString stringWithUTF8String:result.c_str()];
}

@end
```

**Kotlin Actual:**
```kotlin
// iosMain/kotlin/Calculator.ios.kt
import platform.Foundation.*
import kotlinx.cinterop.*

actual object Calculator {
    actual fun add(a: Int, b: Int): Int {
        return MyLibCalculator.add(a.toLong(), b.toLong()).toInt()
    }

    actual fun greet(name: String): String {
        return MyLibCalculator.greet(name) ?: ""
    }
}
```

### Desktop (cinterop)

**def File:**
```
# nativeInterop/cinterop/mylib.def
headers = mylib.h
headerFilter = mylib/*
package = com.example.mylib.native

compilerOpts.linux = -I/usr/include
compilerOpts.macos = -I/usr/local/include
linkerOpts.linux = -L/usr/lib -lmylib
linkerOpts.macos = -L/usr/local/lib -lmylib
```

**Kotlin Actual:**
```kotlin
// nativeMain/kotlin/Calculator.native.kt
import com.example.mylib.native.*
import kotlinx.cinterop.*

actual object Calculator {
    actual fun add(a: Int, b: Int): Int {
        return mylib_add(a, b)
    }

    actual fun greet(name: String): String {
        return mylib_greet(name)?.toKString() ?: ""
    }
}
```

## Testing

### Unit Tests

```kotlin
// commonTest/kotlin/CalculatorTest.kt
import kotlin.test.Test
import kotlin.test.assertEquals

class CalculatorTest {
    @Test
    fun testAdd() {
        assertEquals(5, Calculator.add(2, 3))
    }

    @Test
    fun testGreet() {
        assertEquals("Hello, World!", Calculator.greet("World"))
    }
}
```

### Run Tests

```bash
# All platforms
./gradlew allTests

# Android
./gradlew testDebugUnitTest

# iOS simulator
./gradlew iosSimulatorArm64Test

# Desktop
./gradlew macosX64Test
./gradlew linuxX64Test
./gradlew mingwX64Test
```

## Publishing

### Maven Central

```bash
# Publish all platforms
ccgo publish kmp --registry official

# Or use Gradle
./gradlew publishAllPublicationsToMavenCentral
```

### Consuming the Library

```kotlin
// In another KMP project
dependencies {
    implementation("com.example:mylib:1.0.0")
}
```

## Common Issues

### JNI Method Not Found

**Problem:**
```
java.lang.UnsatisfiedLinkError: No implementation found for ...
```

**Solution:**
```bash
# Verify library is built
ls target/android/arm64-v8a/libmylib.so

# Check JNI method signature
javap -s com.example.mylib.Calculator

# Verify method naming matches
# Java: Java_com_example_mylib_Calculator_add
```

### iOS Framework Not Found

**Problem:**
```
Module 'MyLib' not found
```

**Solution:**
```bash
# Rebuild iOS framework
ccgo build kmp --target ios

# Verify framework structure
ls -la target/ios/MyLib.framework/

# Add framework to Xcode search paths
```

### Desktop Library Loading Failed

**Problem:**
```
java.lang.UnsatisfiedLinkError: Can't load library
```

**Solution:**
```bash
# Linux: Set LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/path/to/target/linux:$LD_LIBRARY_PATH

# macOS: Set DYLD_LIBRARY_PATH
export DYLD_LIBRARY_PATH=/path/to/target/macos:$DYLD_LIBRARY_PATH

# Windows: Add to PATH
set PATH=%PATH%;C:\path\to\target\windows
```

## Best Practices

### 1. API Design

- Keep C++ API simple and C-compatible for easier bindings
- Use primitive types when possible (int, double, const char*)
- Avoid C++ templates and complex types in public headers
- Provide clear error handling (return codes, exceptions)

### 2. Memory Management

```cpp
// Good: Clear ownership
char* create_string() {
    char* str = (char*)malloc(100);
    strcpy(str, "Hello");
    return str;  // Caller must free
}

void free_string(char* str) {
    free(str);
}

// Better: Use smart pointers internally
std::string get_string() {
    return "Hello";
}
```

### 3. Thread Safety

```cpp
// Make shared state thread-safe
class ThreadSafeCounter {
    std::mutex mutex;
    int count = 0;

public:
    int increment() {
        std::lock_guard<std::mutex> lock(mutex);
        return ++count;
    }
};
```

### 4. Platform-Specific Code

```cpp
#ifdef __ANDROID__
    // Android-specific code
#elif defined(__APPLE__)
    #include <TargetConditionals.h>
    #if TARGET_OS_IOS
        // iOS-specific code
    #elif TARGET_OS_OSX
        // macOS-specific code
    #endif
#elif defined(__linux__)
    // Linux-specific code
#elif defined(_WIN32)
    // Windows-specific code
#endif
```

## Performance Optimization

### 1. Minimize JNI Calls

```kotlin
// Bad: Multiple JNI calls
fun processArray(data: IntArray): IntArray {
    return data.map { Calculator.add(it, 1) }.toIntArray()
}

// Good: Single JNI call
external fun processArray(data: IntArray): IntArray  // C++ processes entire array
```

### 2. Use Direct ByteBuffers

```kotlin
// Efficient for large data transfer
val buffer = ByteBuffer.allocateDirect(1024 * 1024)
processData(buffer)  // Zero-copy JNI access
```

### 3. Cache JNI References

```cpp
// Cache class and method IDs
static jclass calculatorClass = nullptr;
static jmethodID addMethod = nullptr;

JNIEXPORT jint JNI_OnLoad(JavaVM* vm, void* reserved) {
    JNIEnv* env;
    vm->GetEnv((void**)&env, JNI_VERSION_1_6);

    calculatorClass = (jclass)env->NewGlobalRef(
        env->FindClass("com/example/mylib/Calculator"));
    addMethod = env->GetMethodID(calculatorClass, "add", "(II)I");

    return JNI_VERSION_1_6;
}
```

## Examples

### Complete Project

See [ccgo-kmp-example](https://github.com/zhlinh/ccgo-kmp-example) for a complete KMP project.

### Minimal Example

Available in CCGO templates:
```bash
ccgo new my-kmp --template kmp-minimal
```

## Resources

### Official Documentation

- [Kotlin Multiplatform](https://kotlinlang.org/docs/multiplatform.html)
- [Kotlin/Native Interop](https://kotlinlang.org/docs/native-c-interop.html)
- [JNI Specification](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/)

### CCGO Documentation

- [CLI Reference](../reference/cli.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Android Platform](android.md)
- [iOS Platform](ios.md)
- [Publishing Guide](../features/publishing.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [Build System Overview](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Publishing to Maven Central](../features/publishing.md)
- [Android Development](android.md)
- [iOS Development](ios.md)
