# Kotlin 多平台

使用 CCGO 构建包含原生 C++ 代码的 Kotlin 多平台（KMP）库的完整指南。

## 概览

CCGO 实现了 C++ 库与 Kotlin 多平台项目的无缝集成，使您能够：

- **跨平台共享 C++ 代码** - 在所有 KMP 平台（Android、iOS、macOS、Linux、Windows）间共享
- **统一构建系统** - 使用单个命令为所有平台构建原生库
- **类型安全绑定** - 为 C++ API 生成 Kotlin expect/actual 声明
- **原生性能** - 直接 JNI/Objective-C 互操作，无额外开销
- **Gradle 集成** - 在 KMP Gradle 构建中获得一流支持

**支持的 KMP 目标平台：**
- Android（ARM64、ARMv7、x86_64、x86）
- iOS（arm64、模拟器）
- macOS（x86_64、arm64）
- Linux（x86_64）
- Windows（x86_64）

## 前置要求

### 必需工具

```bash
# 安装 CCGO
pip install ccgo

# 验证安装
ccgo --version
```

### 平台 SDK

| 平台 | 要求 |
|------|------|
| Android | Android SDK、NDK 21+ |
| iOS | 带 Xcode 的 macOS |
| macOS | 带 Xcode 的 macOS |
| Linux | GCC 或 Clang |
| Windows | Visual Studio 或 MinGW |

## 快速开始

### 创建新的 KMP 项目

```bash
# 创建带 C++ 支持的新 KMP 项目
ccgo new my-kmp-lib

# 进入项目目录
cd my-kmp-lib

# 为所有平台构建
ccgo build kmp
```

### 项目结构

```
my-kmp-lib/
├── CCGO.toml                    # CCGO 配置
├── build.gradle.kts             # 根 Gradle 构建
├── settings.gradle.kts
├── src/
│   ├── commonMain/
│   │   └── kotlin/
│   │       └── com/example/
│   │           └── MyLib.kt     # Kotlin expect 声明
│   ├── androidMain/
│   │   └── kotlin/
│   │       └── com/example/
│   │           └── MyLib.android.kt  # Android actual（JNI）
│   ├── iosMain/
│   │   └── kotlin/
│   │       └── com/example/
│   │           └── MyLib.ios.kt      # iOS actual（Objective-C）
│   └── nativeMain/             # 桌面平台
│       └── kotlin/
│           └── com/example/
│               └── MyLib.native.kt   # cinterop 绑定
├── cpp/
│   ├── include/
│   │   └── mylib/
│   │       └── mylib.h         # C++ 公共头文件
│   ├── src/
│   │   └── mylib.cpp           # C++ 实现
│   ├── jni/                    # Android 的 JNI 包装器
│   │   └── mylib_jni.cpp
│   └── objc/                   # iOS 的 Objective-C 包装器
│       ├── MyLibWrapper.h
│       └── MyLibWrapper.mm
└── target/                     # 构建输出
    ├── android/
    ├── ios/
    ├── macos/
    ├── linux/
    └── windows/
```

## 配置

### CCGO.toml

```toml
[package]
name = "mylib"
version = "1.0.0"
type = "kmp"

[kmp]
# Kotlin 包名
package_name = "com.example.mylib"

# KMP 目标平台
targets = ["android", "ios", "macos", "linux", "windows"]

# Android 配置
[kmp.android]
min_sdk = 21
target_sdk = 33
namespace = "com.example.mylib"

# iOS 配置
[kmp.ios]
min_deployment_target = "12.0"
framework_name = "MyLib"

# macOS 配置
[kmp.macos]
min_deployment_target = "10.14"

[dependencies]
# C++ 依赖
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
    // Android 目标
    androidTarget {
        compilations.all {
            kotlinOptions {
                jvmTarget = "1.8"
            }
        }
    }

    // iOS 目标
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

    // Windows（通过 MinGW）
    mingwX64()

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation("org.jetbrains.kotlin:kotlin-stdlib:1.9.20")
            }
        }

        val androidMain by getting {
            dependencies {
                // JNI 绑定自动包含
            }
        }

        val iosMain by getting {
            dependencies {
                // Framework 绑定自动包含
            }
        }

        val nativeMain by getting {
            dependencies {
                // 桌面平台的 cinterop 绑定
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

    // 链接 CCGO 构建的原生库
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("${projectDir}/target/android")
        }
    }
}
```

## 构建 KMP 库

### 构建所有平台

```bash
# 为所有 KMP 目标构建原生代码
ccgo build kmp

# 输出结构：
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

`ccgo build kmp` 只构建当前宿主机上 Kotlin/Native 能产出的目标。项目存在
`kmp/gradlew` 时，`ccgo build all` 会在所有原生平台构建完成后才构建 KMP，
并直接复用这些平台产出的原生库，不再重复编译。需要其他宿主机家族的 KMP
产物时，请到对应平台上执行 `ccgo build kmp`。

### 使用其他机器生成的 C/C++ SDK ZIP

在对应机器上构建原生 SDK，把 ZIP 复制到当前项目，在 `CCGO.toml` 中显式指定：

```toml
[kmp]
targets = ["android", "desktop", "ios", "macos"]

[kmp.native_sdks]
android = "prebuilt/DEMO_ANDROID_SDK-1.2.3.zip"
ios = "prebuilt/DEMO_IOS_SDK-1.2.3.zip"
macos = "prebuilt/DEMO_MACOS_SDK-1.2.3.zip"
windows = "prebuilt/DEMO_WINDOWS_SDK-1.2.3.zip"
linux = "prebuilt/DEMO_LINUX_SDK-1.2.3.zip"
```

```bash
ccgo build kmp --release --variant ANO
```

路径相对项目根目录解析。请明确选用所需模式、features 和 variant 的 SDK；
输出的 `--variant` 仅控制 KMP 归档命名，不会自动选择或转换输入 SDK。
导入时校验项目名、版本、平台；ZIP 解压到独立临时目录，保留到 Gradle 完成后清理。
已有 SDK 的平台跳过 C/C++ 编译，其他选中平台保持原来的构建流程。

需要同时更新 ccgo CLI 和 `ccgo-gradle-plugins` 的 KMP native convention 插件。
旧插件如果忽略 SDK 输入，会在打包前报错。插件按以下方式使用导入的文件：

- Android：将 `lib/android/shared/<abi>/*.so` 作为 AAR 的 JNI 库。
- iOS/macOS：依据平台、架构、设备/模拟器选择静态 XCFramework 切片，结合 SDK 头文件生成 cinterop。
- Linux：使用 `lib/linux/static/<arch>/lib<project>.a` 和 SDK 头文件生成 cinterop。
- Desktop JVM：把 Windows/Linux/macOS 动态库放进 JAR 的 `native/<platform>/<arch>/...`，保留工具链子目录。

Kotlin/Native 声明的每个架构都需要对应静态库或切片，仅有动态库的 SDK 不能用于这些 cinterop 目标。
Desktop JAR 仍需要项目自己的代码提取并加载原生资源；标准插件的 Windows 路线是 Desktop JVM，
MSVC SDK 不会转换成 MinGW 的 Kotlin/Native klib。

自定义 Gradle 项目可以读取 `-PccgoNativeSdk.<platform>` 指定的解压根目录，
接入头文件、静态链接、JNI 和资源；确认所有输入已接入后，写入
`-PccgoNativeSdkReceipt` 指定的文件来确认支持。不能只写确认文件而忽略 SDK。

`ccgo build all --auto-docker` 会先构建原生平台、最后构建 KMP，但 Docker 不会扩展
KMP 的宿主目标选择，也不能在 Linux/Windows 上提供 macOS/Xcode。
已有 C/C++ SDK 也不能替代 Kotlin/Native 工具链；剩余 klib 仍需到对应宿主机上编译。

### 构建特定平台

```bash
# 仅 Android
ccgo build kmp --target android

# 仅 iOS
ccgo build kmp --target ios

# 桌面平台
ccgo build kmp --target macos
ccgo build kmp --target linux
ccgo build kmp --target windows
```

### Gradle 集成

```bash
# 构建 Kotlin 代码和原生库
./gradlew build

# 发布到 Maven Local
./gradlew publishToMavenLocal

# 创建 iOS XCFramework
./gradlew linkReleaseFrameworkIos
```

## 原生互操作

### Android（JNI）

**C++ 头文件：**
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

**JNI 包装器：**
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

**Kotlin Expect/Actual：**
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

### iOS（Objective-C++）

**Objective-C++ 包装器：**
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

**Kotlin Actual：**
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

### 桌面（cinterop）

**def 文件：**
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

**Kotlin Actual：**
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

## 测试

### 单元测试

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

### 运行测试

```bash
# 所有平台
./gradlew allTests

# Android
./gradlew testDebugUnitTest

# iOS 模拟器
./gradlew iosSimulatorArm64Test

# 桌面
./gradlew macosX64Test
./gradlew linuxX64Test
./gradlew mingwX64Test
```

## 发布

### Maven Central

```bash
# 构建 release KMP SDK、签名并发布所有平台
ccgo publish kmp --registry official
```

使用与 Android 相同的 Central Portal token 和 PGP 签名变量（`MAVEN_CENTRAL_USERNAME` / `MAVEN_CENTRAL_PASSWORD` 和 `SIGNING_IN_MEMORY_KEY` / `SIGNING_IN_MEMORY_KEY_PASSWORD`，或 gpg）。`ccgo publish` 不会调用 Gradle；`ccgo build kmp` 内部仍用 Gradle 编译 Kotlin。

### 预构建 SDK（不依赖 Gradle）

如果已经有 CCGO KMP SDK ZIP，本地和私有 Maven 仓库可以直接发布：

```bash
ccgo publish kmp --path ./STDCOMM_KMP_SDK-0.0.5.zip --registry private \
  --remote https://maven.example.com/releases
```

ccgo 会按目标上传包内的 `-android`、`-desktop`、native 产物及其 POM、校验文件，不需要 `kmp/` 工程或 `gradlew`。改用 `--registry official` 会把签名后的 bundle 上传到 Central Portal。

### 使用库

```kotlin
// 在另一个 KMP 项目中
dependencies {
    implementation("com.example:mylib:1.0.0")
}
```

## 常见问题

### JNI 方法未找到

**问题：**
```
java.lang.UnsatisfiedLinkError: No implementation found for ...
```

**解决方案：**
```bash
# 验证库已构建
ls target/android/arm64-v8a/libmylib.so

# 检查 JNI 方法签名
javap -s com.example.mylib.Calculator

# 验证方法命名匹配
# Java: Java_com_example_mylib_Calculator_add
```

### iOS Framework 未找到

**问题：**
```
Module 'MyLib' not found
```

**解决方案：**
```bash
# 重新构建 iOS framework
ccgo build kmp --target ios

# 验证 framework 结构
ls -la target/ios/MyLib.framework/

# 添加 framework 到 Xcode 搜索路径
```

### 桌面库加载失败

**问题：**
```
java.lang.UnsatisfiedLinkError: Can't load library
```

**解决方案：**
```bash
# Linux：设置 LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/path/to/target/linux:$LD_LIBRARY_PATH

# macOS：设置 DYLD_LIBRARY_PATH
export DYLD_LIBRARY_PATH=/path/to/target/macos:$DYLD_LIBRARY_PATH

# Windows：添加到 PATH
set PATH=%PATH%;C:\path\to\target\windows
```

## 最佳实践

### 1. API 设计

- 保持 C++ API 简单且与 C 兼容以便更容易绑定
- 尽可能使用原始类型（int、double、const char*）
- 避免在公共头文件中使用 C++ 模板和复杂类型
- 提供清晰的错误处理（返回码、异常）

### 2. 内存管理

```cpp
// 好：清晰的所有权
char* create_string() {
    char* str = (char*)malloc(100);
    strcpy(str, "Hello");
    return str;  // 调用者必须释放
}

void free_string(char* str) {
    free(str);
}

// 更好：内部使用智能指针
std::string get_string() {
    return "Hello";
}
```

### 3. 线程安全

```cpp
// 使共享状态线程安全
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

### 4. 平台特定代码

```cpp
#ifdef __ANDROID__
    // Android 特定代码
#elif defined(__APPLE__)
    #include <TargetConditionals.h>
    #if TARGET_OS_IOS
        // iOS 特定代码
    #elif TARGET_OS_OSX
        // macOS 特定代码
    #endif
#elif defined(__linux__)
    // Linux 特定代码
#elif defined(_WIN32)
    // Windows 特定代码
#endif
```

## 性能优化

### 1. 最小化 JNI 调用

```kotlin
// 差：多次 JNI 调用
fun processArray(data: IntArray): IntArray {
    return data.map { Calculator.add(it, 1) }.toIntArray()
}

// 好：单次 JNI 调用
external fun processArray(data: IntArray): IntArray  // C++ 处理整个数组
```

### 2. 使用直接 ByteBuffer

```kotlin
// 大数据传输高效
val buffer = ByteBuffer.allocateDirect(1024 * 1024)
processData(buffer)  // 零拷贝 JNI 访问
```

### 3. 缓存 JNI 引用

```cpp
// 缓存类和方法 ID
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

## 示例

### 完整项目

参见 [ccgo-kmp-example](https://github.com/zhlinh/ccgo-kmp-example) 获取完整的 KMP 项目。

### 最小示例

CCGO 模板中可用：
```bash
ccgo new my-kmp --template kmp-minimal
```

## 资源

### 官方文档

- [Kotlin 多平台](https://kotlinlang.org/docs/multiplatform.html)
- [Kotlin/Native 互操作](https://kotlinlang.org/docs/native-c-interop.html)
- [JNI 规范](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/)

### CCGO 文档

- [CLI 参考](../reference/cli.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
- [Android 平台](android.zh.md)
- [iOS 平台](ios.zh.md)
- [发布指南](../features/publishing.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [构建系统概述](../features/build-system.zh.md)
- [依赖管理](../features/dependency-management.zh.md)
- [发布到 Maven Central](../features/publishing.zh.md)
- [Android 开发](android.zh.md)
- [iOS 开发](ios.zh.md)
