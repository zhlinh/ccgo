# Android 开发

使用 CCGO 为 Android 构建 C++ 库的完整指南。

## 概述

CCGO 提供全面的 Android 支持：
- **多架构构建**：arm64-v8a、armeabi-v7a、x86、x86_64
- **AAR 打包**：即用型 Android Archive 格式
- **Gradle 集成**：与 Android Studio 项目无缝集成
- **JNI 支持**：自动 JNI 包装器生成
- **Maven 发布**：发布到 Maven Local、Central 或私有仓库
- **Docker 构建**：无需本地 Android SDK/NDK 安装即可构建

## 前置条件

### 选项 1：本地开发

**必需：**
- Android SDK（API 级别 21+）
- Android NDK（r21+，推荐：r25+）
- CMake（3.18+）
- Python（3.8+）

**安装：**

```bash
# 安装 Android Studio（包含 SDK）
# 从 https://developer.android.com/studio 下载

# 设置环境变量
export ANDROID_HOME=$HOME/Android/Sdk
export ANDROID_NDK=$ANDROID_HOME/ndk/25.2.9519653

# 验证安装
ccgo check android --verbose
```

### 选项 2：基于 Docker 的开发

**必需：**
- Docker Desktop

```bash
# 使用 Docker 构建（无需本地 SDK/NDK）
ccgo build android --docker
```

首次构建下载预构建镜像（约 3.5GB，5-10 分钟）。后续构建使用缓存镜像。

## 快速开始

### 创建新项目

```bash
# 创建新的 Android 兼容项目
ccgo new my-android-lib
cd my-android-lib/my-android-lib

# 为 Android 构建
ccgo build android
```

### 构建单个架构

```bash
# 仅为 arm64-v8a 构建
ccgo build android --arch arm64-v8a
```

### 构建多个架构

```bash
# 为 arm64-v8a 和 armeabi-v7a 构建
ccgo build android --arch arm64-v8a,armeabi-v7a

# 构建所有架构（默认）
ccgo build android
```

### 构建选项

```bash
# Release 构建（优化）
ccgo build android --release

# Debug 构建（带符号）
ccgo build android --debug

# 清理构建
ccgo build android --clean

# Docker 构建
ccgo build android --docker

# 链接类型控制
ccgo build android --link-type static   # 仅静态库
ccgo build android --link-type shared   # 仅动态库
ccgo build android --link-type both     # 两者都有（默认）
```

## 输出结构

构建后，在 `target/android/` 中查找产物：

```
target/android/
├── MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip      # 主包
├── MY-ANDROID-LIB_ANDROID_SDK-1.0.0-SYMBOLS.zip  # 调试符号
└── build_info.json                            # 构建元数据
```

### 主包结构

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

### 符号包结构

```
MY-ANDROID-LIB_ANDROID_SDK-1.0.0-SYMBOLS.zip
└── obj/
    ├── armeabi-v7a/
    │   └── libmy-android-lib.so  # 未剥离的带调试符号
    ├── arm64-v8a/
    │   └── libmy-android-lib.so
    ├── x86/
    │   └── libmy-android-lib.so
    └── x86_64/
        └── libmy-android-lib.so
```

## 配置

### CCGO.toml

配置 Android 特定设置：

```toml
[package]
name = "my-android-lib"
version = "1.0.0"

[library]
type = "both"  # 构建静态和动态库

[android]
min_sdk_version = 21          # Android 5.0 (Lollipop)
target_sdk_version = 33       # Android 13
ndk_version = "25.2.9519653"  # 特定 NDK 版本
stl = "c++_static"            # STL 类型：c++_static 或 c++_shared
architectures = ["arm64-v8a", "armeabi-v7a", "x86_64"]  # 可选：限制架构

[build]
cpp_standard = "17"
compile_flags = ["-Wall", "-Wextra"]
```

### Android 配置选项

| 选项 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `min_sdk_version` | 整数 | 最低 Android API 级别 | `21` |
| `target_sdk_version` | 整数 | 目标 Android API 级别 | `33` |
| `ndk_version` | 字符串 | 特定 NDK 版本 | 最新已安装 |
| `stl` | 字符串 | STL 类型：`c++_static`、`c++_shared` | `c++_static` |
| `architectures` | 数组 | 要构建的目标 ABI | 所有支持的 |

### 支持的架构

| ABI | 架构 | 描述 |
|-----|------|------|
| `arm64-v8a` | ARM 64 位 | 现代 Android 设备（推荐）|
| `armeabi-v7a` | ARM 32 位 | 旧版 Android 设备 |
| `x86_64` | Intel 64 位 | 模拟器、平板、Chrome OS |
| `x86` | Intel 32 位 | 旧版模拟器 |

**推荐：** 为生产应用构建 `arm64-v8a` 和 `armeabi-v7a`。

## AAR 集成

### 在 Android 项目中使用 AAR

**1. 将 AAR 复制到项目：**

```bash
# 从构建输出复制 AAR
cp target/android/MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip .
unzip MY-ANDROID-LIB_ANDROID_SDK-1.0.0.zip
cp haars/my-android-lib-release.aar android-app/libs/
```

**2. 配置 app/build.gradle.kts：**

```kotlin
android {
    // ...
}

dependencies {
    implementation(fileTree(mapOf("dir" to "libs", "include" to listOf("*.aar"))))
    // 或
    implementation(files("libs/my-android-lib-release.aar"))
}
```

**3. 在 Java/Kotlin 中使用：**

```kotlin
class MainActivity : AppCompatActivity() {
    companion object {
        init {
            System.loadLibrary("my-android-lib")
        }
    }

    // 声明原生方法
    external fun nativeMethod(): String

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 调用原生方法
        val result = nativeMethod()
        Log.d("Native", "Result: $result")
    }
}
```

## JNI 集成

### 自动 JNI 包装器

CCGO 可以自动生成 JNI 包装器：

**C++ 头文件（include/my-android-lib/my-android-lib.h）：**

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

**生成的 JNI 包装器（自动生成）：**

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

### 手动 JNI 实现

**创建 src/jni/my_jni.cpp：**

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

**Java/Kotlin 端：**

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

## Gradle 集成

### 使用 CCGO Gradle 插件

CCGO 提供 Gradle 约定插件用于标准化 Android 构建。

**settings.gradle.kts：**

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

**app/build.gradle.kts：**

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
    projectPath.set(file("../"))  // CCGO 项目根路径
    buildType.set("release")       // 或 "debug"
    architectures.set(listOf("arm64-v8a", "armeabi-v7a"))
}
```

## 发布

### Maven Local（开发）

```bash
# 发布到 Maven Local 进行测试
ccgo publish android --registry local

# 位置：~/.m2/repository/com/example/my-android-lib/1.0.0/
```

### Maven Central（生产）

**1. 配置凭据：**

创建 `~/.gradle/gradle.properties`：

```properties
mavenCentralUsername=your-username
mavenCentralPassword=your-password
signing.keyId=12345678
signing.password=your-key-password
signing.secretKeyRingFile=/Users/you/.gnupg/secring.gpg
```

**2. 发布：**

```bash
ccgo publish android --registry official
```

### 私有 Maven 仓库

```bash
ccgo publish android --registry private \
    --url https://maven.example.com/releases
```

### 使用已发布的库

**app/build.gradle.kts：**

```kotlin
dependencies {
    implementation("com.example:my-android-lib:1.0.0")
}
```

## 高级主题

### 多模块项目

**项目结构：**

```
my-project/
├── CCGO.toml
├── lib1/
│   ├── CCGO.toml
│   └── src/
└── lib2/
    ├── CCGO.toml（依赖于 lib1）
    └── src/
```

**lib2/CCGO.toml：**

```toml
[dependencies]
lib1 = { path = "../lib1" }
```

### 自定义 CMake 配置

**CMakeLists.txt：**

```cmake
cmake_minimum_required(VERSION 3.18)

# CCGO 自动提供：
# - ${CCGO_CMAKE_DIR}：CCGO cmake 工具路径
# - ${ANDROID_ABI}：当前正在构建的架构
# - ${ANDROID_PLATFORM}：Android API 级别

include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# 自定义 Android 特定配置
if(ANDROID)
    # 添加 Android 特定编译器标志
    add_compile_options(-fPIC)

    # 链接 Android 库
    find_library(LOG_LIB log)
    find_library(ANDROID_LIB android)

    target_link_libraries(${PROJECT_NAME}
        ${LOG_LIB}
        ${ANDROID_LIB}
    )
endif()
```

### Proguard 规则

**创建 proguard-rules.pro：**

```proguard
# 保留原生方法
-keepclasseswithmembernames class * {
    native <methods>;
}

# 保留 JNI 导出方法
-keep class com.example.mylib.** { *; }
```

### 应用大小优化

**1. 剥离不需要的符号（在 release 构建中自动）：**

```bash
ccgo build android --release
```

**2. 仅使用必需的架构：**

```toml
[android]
architectures = ["arm64-v8a"]  # 如果不需要，放弃 32 位支持
```

**3. 启用链接时优化：**

```toml
[build]
link_flags = ["-flto"]
```

**4. 按 ABI 拆分 APK：**

**app/build.gradle.kts：**

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

## 故障排除

### 常见问题

#### 未找到 NDK

```
Error: Android NDK not found
```

**解决方案：**

```bash
# 通过 Android Studio 安装 NDK：Tools → SDK Manager → SDK Tools → NDK

# 或手动设置
export ANDROID_NDK=$ANDROID_HOME/ndk/25.2.9519653

# 验证
ccgo check android --verbose
```

#### 架构不匹配

```
Error: UnsatisfiedLinkError: dlopen failed: library "libmy-android-lib.so" not found
```

**解决方案：**

确保 AAR 包含设备/模拟器使用的架构：

```bash
# 检查 AAR 内容
unzip -l my-android-lib-release.aar | grep "\.so$"

# 使用正确的架构重新构建
ccgo build android --arch arm64-v8a
```

#### C++ 标准不匹配

```
Error: undefined reference to std::__cxx11::...
```

**解决方案：**

确保依赖之间的 C++ 标准一致：

```toml
[build]
cpp_standard = "17"  # 与依赖匹配

[android]
stl = "c++_static"  # 或 c++_shared
```

#### 缺少符号

```
Error: undefined reference to 'my_function'
```

**解决方案：**

检查所有源文件是否已编译：

```bash
# 启用详细构建
ccgo build android --verbose

# 检查 CMakeLists.txt 是否包含所有源文件
```

### Docker 构建问题

#### Docker 未运行

```
Error: Cannot connect to the Docker daemon
```

**解决方案：**

```bash
# 启动 Docker Desktop
open -a Docker  # macOS

# 验证
docker ps
```

#### 镜像拉取失败

```
Error: failed to pull image ccgo-builder-android
```

**解决方案：**

```bash
# 使用手动拉取重试
docker pull ccgogroup/ccgo-builder-android:latest

# 或使用本地构建
cd ccgo/dockers/
docker build -t ccgo-builder-android -f Dockerfile.android .
```

### 性能问题

#### 首次构建缓慢

**正常：** 首次构建编译所有依赖（约 10-30 分钟）。

**优化：**

```bash
# 使用预构建依赖（未来功能）
ccgo install --prebuilt

# 启用 ccache
export USE_CCACHE=1
export CCACHE_DIR=$HOME/.ccache
```

#### 增量构建不工作

```bash
# 清理 CMake 缓存
rm -rf cmake_build/android/

# 重新构建
ccgo build android
```

## 最佳实践

### 1. 版本管理

```toml
[package]
version = "1.0.0"  # 发布前更新
```

```bash
# 创建 git 标签
ccgo tag v1.0.0 --push
```

### 2. 架构选择

```toml
[android]
# 生产：arm64-v8a + armeabi-v7a（覆盖 99%+ 设备）
architectures = ["arm64-v8a", "armeabi-v7a"]

# 开发：仅 arm64-v8a（更快的构建）
# architectures = ["arm64-v8a"]
```

### 3. STL 选择

```toml
[android]
# 首选 c++_static（无运行时依赖）
stl = "c++_static"

# 仅在多个原生库共享 STL 时使用 c++_shared
# stl = "c++_shared"
```

### 4. 依赖管理

```toml
[dependencies]
# 固定到特定版本以确保可重现性
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 使用 CCGO.lock 进行精确的依赖解析
```

### 5. 测试

```bash
# 本地构建和测试
ccgo build android --arch arm64-v8a
ccgo test android

# 发布前在示例应用中测试 AAR
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

## 示例

### 完整项目

参见 [ccgo-now](https://github.com/zhlinh/ccgo-now) 获取完整的 Android 项目示例。

### 最小示例

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

**构建：**

```bash
ccgo build android --arch arm64-v8a
```

**在 Android 中使用：**

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

## 资源

### 官方文档

- [Android NDK 文档](https://developer.android.com/ndk)
- [JNI 规范](https://docs.oracle.com/javase/8/docs/technotes/guides/jni/)
- [CMake Android 指南](https://developer.android.com/ndk/guides/cmake)

### CCGO 文档

- [CLI 参考](../reference/cli.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
- [发布指南](../features/publishing.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [构建系统概述](../features/build-system.zh.md)
- [依赖管理](../features/dependency-management.zh.md)
- [iOS 开发](ios.zh.md)
- [OpenHarmony 开发](openharmony.zh.md)
