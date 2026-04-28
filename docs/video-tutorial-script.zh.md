# CCGO 视频教程脚本

用于演示 CCGO 能力和工作流的完整视频教程脚本。

---

## 视频 1：CCGO 介绍（5-7 分钟）

### 场景 1：开场（30 秒）

**[Screen: CCGO logo animation]**

**Narrator:**
"欢迎来到 CCGO——面向跨平台开发的现代 C++ 构建系统。无论你是为移动端、桌面端，还是嵌入式系统构建项目，CCGO 都能让从开发到分发的整个流程变得简单。"

**[Screen: Split screen showing different platforms - Android, iOS, macOS, Windows, Linux, OpenHarmony]**

### 场景 2：问题（1 分钟）

**[Screen: Traditional C++ development workflow with multiple tools]**

**Narrator:**
"传统的 C++ 跨平台开发非常复杂。每个平台都需要不同的构建系统——Android 用 Gradle，iOS 用 Xcode，Windows 用 Visual Studio，Linux 用 CMake。每一种都有自己的配置格式、构建命令和打包方式。"

**[Screen: Show frustration - multiple terminal windows, config files, SDKs]**

"管理依赖？更复杂。安装 SDK？要花几个小时。更别提向不同的包管理器发布产物了。"

### 场景 3：解决方案（1 分钟）

**[Screen: CCGO workflow - single tool, multiple platforms]**

**Narrator:**
"CCGO 改变了这一切。一个工具，一份配置文件，覆盖所有平台。C++ 代码只写一次，剩下的都交给 CCGO——构建、打包、向任意平台发布。"

**[Screen: Show CCGO commands]**

```bash
ccgo build android    # 为 Android 构建
ccgo build ios        # 为 iOS 构建
ccgo build macos      # 为 macOS 构建
ccgo build windows    # 为 Windows 构建
ccgo build linux      # 为 Linux 构建
```

### 场景 4：核心特性（2 分钟）

**[Screen: Feature highlights with icons]**

**Narrator:**
"CCGO 的特别之处在于："

**[Icon 1: Multi-platform]**
"**全平台支持**——基于一份代码，同时构建 Android、iOS、macOS、Windows、Linux 和 OpenHarmony。"

**[Icon 2: Docker]**
"**Docker 集成**——在任意操作系统上为任意平台构建。无需拥有一台 Mac，也能构建 iOS 库。"

**[Icon 3: Package managers]**
"**统一发布**——使用一致的命令发布到 Maven Central、CocoaPods、Swift Package Manager、OHPM 和 Conan。"

**[Icon 4: CMake]**
"**CMake 集成**——兼容你已有的 CMake 项目，同时叠加平台特定的增强能力。"

**[Icon 5: Templates]**
"**项目模板**——几秒钟生成可投入生产的项目结构。"

### 场景 5：行动召唤（30 秒）

**[Screen: Installation command]**

**Narrator:**
"准备简化你的 C++ 工作流了吗？现在就安装 CCGO："

```bash
pip install ccgo
```

**[Screen: Documentation link]**

"访问 ccgo.dev 查看完整的文档、教程和示例。让我们一起构建出色的产品！"

**[Screen: CCGO logo with tagline: "Build Once, Deploy Everywhere"]**

---

## 视频 2：快速上手教程（8-10 分钟）

### 场景 1：安装（1 分钟）

**[Screen: Terminal]**

**Narrator:**
"我们开始吧。首先用 pip 安装 CCGO："

```bash
$ pip install ccgo
$ ccgo --version
CCGO version 0.1.0
```

"就是这样，CCGO 装好了。现在我们来创建第一个跨平台的 C++ 库。"

### 场景 2：创建新项目（2 分钟）

**[Screen: Terminal]**

**Narrator:**
"使用 `ccgo new` 命令创建新项目："

```bash
$ ccgo new mylib
```

**[Screen: Interactive prompts]**

"CCGO 会就项目信息问你几个问题："

```
📦 Project name: mylib
👤 Author: John Doe
📧 Email: john@example.com
📄 License: MIT
📝 Description: My awesome C++ library
```

**[Screen: Generated project structure]**

"就这样，一个完整的项目结构就生成好了："

```
mylib/
├── CCGO.toml              # 项目配置
├── CMakeLists.txt         # 构建配置
├── src/                   # 源代码
│   └── mylib.cpp
├── include/               # 公开头文件
│   └── mylib/
│       └── mylib.h
├── tests/                 # 单元测试
│   └── test_mylib.cpp
└── examples/              # 示例程序
    └── example.cpp
```

### 场景 3：编写代码（1.5 分钟）

**[Screen: Code editor showing include/mylib/mylib.h]**

**Narrator:**
"我们给库加点功能。打开头文件："

```cpp
// include/mylib/mylib.h
#pragma once

#include <string>

namespace mylib {

class Calculator {
public:
    int add(int a, int b);
    int multiply(int a, int b);
    std::string get_version();
};

} // namespace mylib
```

**[Screen: Code editor showing src/mylib.cpp]**

"接下来是实现："

```cpp
// src/mylib.cpp
#include "mylib/mylib.h"

namespace mylib {

int Calculator::add(int a, int b) {
    return a + b;
}

int Calculator::multiply(int a, int b) {
    return a * b;
}

std::string Calculator::get_version() {
    return "1.0.0";
}

} // namespace mylib
```

### 场景 4：为多平台构建（3 分钟）

**[Screen: Terminal]**

**Narrator:**
"接下来是见证奇迹的时刻——为多个平台构建。先来 Android："

```bash
$ ccgo build android
Building for Android...
Architectures: armeabi-v7a, arm64-v8a, x86_64
✓ Build complete: target/android/mylib-android-1.0.0.aar
```

**[Screen: Show generated AAR file]**

"CCGO 自动为多个 Android 架构构建，并将所有产物打包成一个 AAR 文件。"

**[Screen: Terminal]**

"接下来是 iOS："

```bash
$ ccgo build ios
Building for iOS...
✓ Framework: target/ios/MyLib.framework
✓ XCFramework: target/ios/MyLib.xcframework
```

**[Screen: Show XCFramework structure]**

"CCGO 同时生成传统的 framework 和现代的 XCFramework，能在真机和模拟器上同时使用。"

**[Screen: Terminal]**

"那 Windows 呢？即使你在 macOS 或 Linux 上，CCGO 也能借助 Docker 帮你搞定："

```bash
$ ccgo build windows --docker
Pulling Docker image...
Building for Windows...
✓ Static library: target/windows/lib/static/mylib.lib
✓ DLL: target/windows/lib/shared/mylib.dll
```

### 场景 5：测试（1 分钟）

**[Screen: Terminal]**

**Narrator:**
"我们运行测试，确认一切正常："

```bash
$ ccgo test
Running tests...
[==========] Running 3 tests
[----------] 2 tests from Calculator
[ RUN      ] Calculator.Add
[       OK ] Calculator.Add (0 ms)
[ RUN      ] Calculator.Multiply
[       OK ] Calculator.Multiply (0 ms)
[==========] 3 tests ran. (1 ms total)
[  PASSED  ] 3 tests.
```

### 场景 6：小结（30 秒）

**[Screen: Summary of created files]**

**Narrator:**
"短短几分钟，我们就创建了一个跨平台的 C++ 库，为 Android、iOS 和 Windows 完成了构建，还通过测试验证了它能正常工作。CCGO 处理了所有平台相关的细节，让你专注于写出色的代码。"

---

## 视频 3：进阶特性（10-12 分钟）

### 场景 1：依赖管理（2 分钟）

**[Screen: CCGO.toml file]**

**Narrator:**
"CCGO 让依赖管理变得很简单。只需要把依赖加到 CCGO.toml 文件里："

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
nlohmann_json = { git = "https://github.com/nlohmann/json.git", tag = "v3.11.2" }
```

**[Screen: Terminal]**

"CCGO 会自动下载并构建依赖："

```bash
$ ccgo build android
Fetching dependencies...
  ✓ spdlog v1.12.0
  ✓ nlohmann_json v3.11.2
Building for Android...
```

**[Screen: Code using dependencies]**

```cpp
#include <spdlog/spdlog.h>
#include <nlohmann/json.hpp>

void log_config() {
    nlohmann::json config = {
        {"name", "mylib"},
        {"version", "1.0.0"}
    };
    spdlog::info("Config: {}", config.dump());
}
```

### 场景 2：Docker 构建（2.5 分钟）

**[Screen: Diagram showing Docker workflow]**

**Narrator:**
"CCGO 最强大的功能之一就是 Docker 集成。在任意操作系统上，都能为任意平台构建。"

**[Screen: macOS terminal]**

"在 macOS 上，但要为 Linux 构建？"

```bash
$ ccgo build linux --docker
Pulling ccgo-builder-linux image...
Building in Docker container...
✓ Build complete
```

**[Screen: Windows terminal]**

"在 Windows 上，但需要 iOS 库？"

```bash
$ ccgo build ios --docker
Pulling ccgo-builder-apple image...
Building in Docker container...
✓ XCFramework created
```

**[Screen: Show Docker advantages]**

**Narrator:**
"Docker 构建特别适合："
- "CI/CD 流水线——任何环境下都能得到一致的构建结果"
- "团队开发——所有人都使用相同的工具链"
- "交叉编译——无需安装 SDK 就能构建任意平台"

### 场景 3：发布（2.5 分钟）

**[Screen: Publishing workflow diagram]**

**Narrator:**
"库构建完成后，发布同样简单。CCGO 用统一的接口支持所有主流的包管理器。"

**[Screen: Terminal - Android/Maven]**

"将 Android 库发布到 Maven Central："

```bash
$ ccgo publish android --registry official
Building AAR...
Signing with PGP...
Uploading to Maven Central...
✓ Published: com.example:mylib:1.0.0
```

**[Screen: Terminal - iOS/CocoaPods]**

"发布到 CocoaPods："

```bash
$ ccgo publish apple --manager cocoapods
Validating podspec...
Pushing to CocoaPods Trunk...
✓ Published: MyLib 1.0.0
```

**[Screen: Terminal - Multiple platforms]**

"或者一次发布到多个平台："

```bash
$ ccgo publish android --registry official
$ ccgo publish apple --manager all --push
$ ccgo publish ohos --registry official
```

### 场景 4：配置详解（2 分钟）

**[Screen: CCGO.toml with annotations]**

**Narrator:**
"CCGO.toml 是项目的控制中心。我们来看看几个关键段落："

```toml
# 项目基础元信息
[package]
name = "mylib"
version = "1.0.0"
description = "My awesome library"

# 库类型
[library]
type = "both"  # static、shared 或 both

# 构建设置
[build]
cpp_standard = "17"
cxxflags = ["-O3", "-Wall"]

# 平台特定设置
[android]
min_sdk_version = 21
target_sdk_version = 33

[ios]
deployment_target = "12.0"

[windows]
runtime_library = "MD"  # 动态 CRT
```

**[Screen: Highlight different sections]**

"每个段落控制构建的不同方面，既给你细粒度的控制能力，又保持了整体的简洁。"

### 场景 5：IDE 集成（1.5 分钟）

**[Screen: Terminal]**

**Narrator:**
"CCGO 还能为平台特定的开发生成 IDE 工程："

**[Screen: Android Studio project]**

```bash
$ ccgo build android --ide-project
Generating Android Studio project...
✓ Project: cmake_build/android/
```

**[Screen: Xcode project]**

```bash
$ ccgo build ios --ide-project
Generating Xcode project...
✓ Project: cmake_build/ios/MyLib.xcodeproj
```

**[Screen: Visual Studio project]**

```bash
$ ccgo build windows --ide-project
Generating Visual Studio solution...
✓ Solution: cmake_build/windows/MyLib.sln
```

### 场景 6：小结（30 秒）

**[Screen: Feature summary]**

**Narrator:**
"CCGO 的进阶特性——依赖管理、Docker 构建、统一发布、IDE 集成——共同打造了顺滑的跨平台开发体验。"

---

## 视频 4：实战案例（15-20 分钟）

### 场景 1：项目概述（2 分钟）

**[Screen: Project visualization]**

**Narrator:**
"我们来做一个真实的 C++ 库——一个能在移动端和桌面端运行的图像处理库。"

**[Screen: Feature list]**

"这个库要："
- "调整尺寸和裁剪图像"
- "应用滤镜（灰度、模糊、锐化）"
- "支持多种图像格式"
- "同时提供 C++ 和原生平台的 API"

### 场景 2：项目搭建（2 分钟）

**[Screen: Terminal]**

```bash
$ ccgo new imagelib
$ cd imagelib
```

**[Screen: Add dependencies to CCGO.toml]**

```toml
[dependencies]
stb = { git = "https://github.com/nothings/stb.git", branch = "master" }
```

**[Screen: Project structure]**

```
imagelib/
├── src/
│   ├── image.cpp          # 图像核心类
│   ├── filters.cpp        # 滤镜实现
│   └── platform/          # 平台特定代码
│       ├── android_jni.cpp
│       ├── ios_objc.mm
│       └── windows_dll.cpp
├── include/
│   └── imagelib/
│       ├── image.h
│       └── filters.h
└── tests/
    ├── test_image.cpp
    └── test_filters.cpp
```

### 场景 3：实现（4 分钟）

**[Screen: Code editor - image.h]**

**Narrator:**
"我们来实现核心的 Image 类："

```cpp
// include/imagelib/image.h
#pragma once

#include <vector>
#include <string>

namespace imagelib {

class Image {
public:
    Image(int width, int height);
    ~Image();

    // 加载和保存
    bool load(const std::string& path);
    bool save(const std::string& path);

    // 基本操作
    void resize(int new_width, int new_height);
    void crop(int x, int y, int width, int height);

    // Getter
    int width() const { return width_; }
    int height() const { return height_; }
    const unsigned char* data() const { return data_.data(); }

private:
    int width_;
    int height_;
    std::vector<unsigned char> data_;
};

} // namespace imagelib
```

**[Screen: Code editor - filters.h]**

```cpp
// include/imagelib/filters.h
#pragma once

#include "image.h"

namespace imagelib {

class Filters {
public:
    static void grayscale(Image& image);
    static void blur(Image& image, int radius);
    static void sharpen(Image& image);
};

} // namespace imagelib
```

**[Screen: Implementation walkthrough - showing key parts of image.cpp and filters.cpp]**

### 场景 4：平台特定绑定（4 分钟）

**[Screen: Android JNI binding]**

**Narrator:**
"Android 端我们写 JNI 绑定："

```cpp
// src/platform/android_jni.cpp
#include <jni.h>
#include "imagelib/image.h"

extern "C" {

JNIEXPORT jlong JNICALL
Java_com_example_imagelib_Image_nativeCreate(
    JNIEnv* env, jclass clazz, jint width, jint height) {
    auto* image = new imagelib::Image(width, height);
    return reinterpret_cast<jlong>(image);
}

JNIEXPORT void JNICALL
Java_com_example_imagelib_Image_nativeResize(
    JNIEnv* env, jclass clazz, jlong handle, jint width, jint height) {
    auto* image = reinterpret_cast<imagelib::Image*>(handle);
    image->resize(width, height);
}

} // extern "C"
```

**[Screen: iOS Objective-C++ wrapper]**

**Narrator:**
"iOS 端我们写一层 Objective-C++ 包装，方便和 Swift 互操作："

```objc
// src/platform/ios_objc.mm
#import "ImageLib.h"
#include "imagelib/image.h"

@implementation ImageLib {
    std::unique_ptr<imagelib::Image> _image;
}

- (instancetype)initWithWidth:(NSInteger)width height:(NSInteger)height {
    if (self = [super init]) {
        _image = std::make_unique<imagelib::Image>(width, height);
    }
    return self;
}

- (void)resizeToWidth:(NSInteger)width height:(NSInteger)height {
    _image->resize(width, height);
}

@end
```

**[Screen: Windows DLL exports]**

```cpp
// src/platform/windows_dll.cpp
#ifdef _WIN32
    #define IMAGELIB_API __declspec(dllexport)
#else
    #define IMAGELIB_API
#endif

extern "C" {

IMAGELIB_API void* imagelib_create(int width, int height) {
    return new imagelib::Image(width, height);
}

IMAGELIB_API void imagelib_resize(void* handle, int width, int height) {
    auto* image = static_cast<imagelib::Image*>(handle);
    image->resize(width, height);
}

} // extern "C"
```

### 场景 5：构建与测试（3 分钟）

**[Screen: Terminal - building for all platforms]**

**Narrator:**
"现在我们为所有平台构建："

```bash
# 移动平台
$ ccgo build android
✓ AAR: imagelib-android-1.0.0.aar

$ ccgo build ios
✓ XCFramework: ImageLib.xcframework

# 桌面平台
$ ccgo build macos
✓ Framework: ImageLib.framework

$ ccgo build windows --docker
✓ DLL: imagelib.dll

$ ccgo build linux --docker
✓ Shared library: libimagelib.so
```

**[Screen: Running tests]**

```bash
$ ccgo test
Running imagelib tests...
[==========] 15 tests from 3 test suites
[  PASSED  ] 15 tests
```

### 场景 6：发布（2 分钟）

**[Screen: Terminal - publishing]**

**Narrator:**
"最后一步，发布我们的库："

```bash
# Android - Maven Central
$ ccgo publish android --registry official
✓ Published to Maven Central

# iOS - CocoaPods 和 SPM
$ ccgo publish apple --manager all --push
✓ Published to CocoaPods
✓ Tagged for Swift Package Manager

# OpenHarmony - OHPM
$ ccgo publish ohos --registry official
✓ Published to OHPM

# 跨平台 - Conan
$ ccgo publish conan --registry official
✓ Published to Conan Center
```

### 场景 7：集成示例（2 分钟）

**[Screen: Android usage example]**

**Narrator:**
"现在开发者可以非常方便地使用我们的库。在 Android 上："

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.example:imagelib:1.0.0")
}

// 使用
val image = Image(800, 600)
image.load("/sdcard/photo.jpg")
Filters.grayscale(image)
image.save("/sdcard/photo_gray.jpg")
```

**[Screen: iOS usage example]**

"在 iOS 上用 Swift："

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/example/imagelib", from: "1.0.0")
]

// 使用
let image = ImageLib(width: 800, height: 600)
image.load("photo.jpg")
image.grayscale()
image.save("photo_gray.jpg")
```

### 场景 8：小结（1 分钟）

**[Screen: Summary screen]**

**Narrator:**
"在这一节里，我们用 CCGO 构建了一个完整的跨平台图像处理库。我们："

- "搭建了一个功能完整的 C++ 库"
- "为 Android、iOS 和 Windows 添加了平台绑定"
- "用一个工具完成了所有平台的构建"
- "做了完整的测试"
- "发布到了多个包管理器"

"CCGO 处理掉了所有平台的复杂性，让我们能专注于把软件做好。"

---

## 视频 5：技巧与最佳实践（8-10 分钟）

### 场景 1：项目组织（2 分钟）

**[Screen: Well-organized project structure]**

**Narrator:**
"我们来聊聊最佳实践。先说项目组织："

**[Screen: Directory structure]**

```
mylib/
├── include/          # 仅放公开 API
│   └── mylib/
│       └── *.h
├── src/             # 实现
│   ├── core/        # 核心功能
│   ├── utils/       # 工具
│   └── platform/    # 平台特定代码
├── tests/           # 单元测试
├── benches/         # 基准测试
├── examples/        # 用法示例
└── docs/            # 文档
```

"公开的 API 放 include/，内部代码放 src/，平台相关的代码单独隔离。"

### 场景 2：版本管理（1.5 分钟）

**[Screen: CCGO.toml version]**

**Narrator:**
"始终使用语义化版本："

```toml
[package]
version = "1.2.3"  # MAJOR.MINOR.PATCH
```

**[Screen: Git workflow]**

"为发布打标签："

```bash
$ ccgo tag
Created tag: v1.2.3

$ git push origin v1.2.3
```

### 场景 3：CI/CD 集成（2 分钟）

**[Screen: GitHub Actions workflow]**

**Narrator:**
"用 CI/CD 自动化你的构建："

```yaml
# .github/workflows/build.yml
name: Build
on: [push, pull_request]

jobs:
  build:
    strategy:
      matrix:
        platform: [android, ios, macos, windows, linux]
    steps:
      - uses: actions/checkout@v3
      - run: pip install ccgo
      - run: ccgo build ${{ matrix.platform }} --docker
      - run: ccgo test
```

"每次提交都会构建并测试所有平台。"

### 场景 4：Docker 最佳实践（1.5 分钟）

**[Screen: Docker tips]**

**Narrator:**
"使用 Docker 构建时的几点建议："

**[Tip 1]** "缓存镜像，加快构建——构建之间不要删除"
**[Tip 2]** "分配充足资源——4 核以上 CPU、8GB 以上内存"
**[Tip 3]** "CI/CD 用 Docker，本地开发用原生构建"
**[Tip 4]** "每月更新镜像，使用最新的工具链"

### 场景 5：性能优化（1.5 分钟）

**[Screen: Optimization techniques]**

**Narrator:**
"优化你的构建："

```toml
[build]
# 链接时优化
cxxflags = ["-flto"]
ldflags = ["-flto"]

# 平台特定优化
[android]
cxxflags = ["-O3", "-ffast-math"]

[ios]
cxxflags = ["-O3", "-fvectorize"]
```

**[Screen: Build performance comparison]**

"这些优化能让二进制体积减少 30%，性能提升 20%。"

### 场景 6：常见问题排查（1.5 分钟）

**[Screen: Common errors and solutions]**

**Narrator:**
"几个常见问题和解决办法："

**[Issue 1: Dependency not found]**
```bash
Error: Could not find dependency 'spdlog'

# 解决方案：检查 git URL 和 tag
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

**[Issue 2: Build failed]**
```bash
Error: Build failed for arm64-v8a

# 解决方案：检查 CMakeLists.txt 和构建日志
$ ccgo build android --verbose
```

**[Issue 3: Docker permission denied]**
```bash
Error: permission denied (Linux)

# 解决方案：把用户加入 docker 组
$ sudo usermod -aG docker $USER
```

### 场景 7：小结（30 秒）

**[Screen: Best practices checklist]**

**Narrator:**
"按照这些最佳实践，CCGO 的跨平台开发会更顺畅："

- "✓ 合理组织代码"
- "✓ 使用语义化版本"
- "✓ 用 CI/CD 自动化"
- "✓ 借助 Docker 保持一致性"
- "✓ 进行性能优化"
- "✓ 在所有目标平台上测试"

---

## 结尾

**[Screen: CCGO logo]**

**Narrator:**
"感谢观看！CCGO 简化了跨平台 C++ 开发，让你可以专注于把软件做好。访问 ccgo.dev 获取文档、示例和社区支持。祝你写代码愉快！"

**[Screen: Links]**
- 文档：ccgo.dev/docs
- GitHub：github.com/ccgo-org/ccgo
- Discord：discord.gg/ccgo
- Twitter：@ccgodev

---

## 制作说明

### 视频风格
- **基调**：专业但不失亲切
- **节奏**：适中——给观众跟上的时间
- **画面**：终端录屏、代码编辑器和示意图混合呈现
- **音乐**：轻柔的背景音乐，无人声歌词

### 屏幕录制
- **分辨率**：至少 1920x1080
- **字号**：足够移动端观看（终端最少 16pt）
- **光标**：高亮显示光标移动
- **打字速度**：适中，命令后留出停顿

### 后期制作
- **字幕**：添加无障碍闭路字幕
- **章节**：YouTube 章节方便导航
- **时间戳**：写入视频简介
- **代码片段**：写入视频简介

### 配套材料
- **示例代码**：包含全部示例的 GitHub 仓库
- **速查表**：常用命令的 PDF 速查表
- **练习**：引导式教程
- **FAQ**：常见问题与解答

---

## 目标受众

- **主要受众**：刚接触跨平台开发的 C++ 开发者
- **次要受众**：希望用 C++ 写性能关键代码的移动开发者
- **第三类受众**：希望简化跨平台构建流程的团队

## 行动召唤

每个视频结尾都应该：
1. 给出文档链接
2. 鼓励试用 CCGO
3. 加入社区（Discord/GitHub）
4. 点赞订阅获取更多教程
