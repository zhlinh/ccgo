# CCGO Video Tutorial Script

Complete video tutorial script for demonstrating CCGO's capabilities and workflows.

---

## Video 1: Introduction to CCGO (5-7 minutes)

### Scene 1: Opening (30 seconds)

**[Screen: CCGO logo animation]**

**Narrator:**
"Welcome to CCGO - the modern C++ build system for cross-platform development. Whether you're building for mobile, desktop, or embedded systems, CCGO simplifies the entire process from development to distribution."

**[Screen: Split screen showing different platforms - Android, iOS, macOS, Windows, Linux, OpenHarmony]**

### Scene 2: The Problem (1 minute)

**[Screen: Traditional C++ development workflow with multiple tools]**

**Narrator:**
"Traditional C++ cross-platform development is complex. You need different build systems for each platform - Gradle for Android, Xcode for iOS, Visual Studio for Windows, CMake for Linux. Each has its own configuration format, build commands, and packaging methods."

**[Screen: Show frustration - multiple terminal windows, config files, SDKs]**

"Managing dependencies? Even more complicated. Installing SDKs? Hours of setup time. And don't get me started on publishing to different package managers."

### Scene 3: The Solution (1 minute)

**[Screen: CCGO workflow - single tool, multiple platforms]**

**Narrator:**
"CCGO changes everything. One tool, one configuration file, all platforms. Write your C++ code once, and CCGO handles the rest - building, packaging, and publishing to any platform."

**[Screen: Show CCGO commands]**

```bash
ccgo build android    # Build for Android
ccgo build ios        # Build for iOS
ccgo build macos      # Build for macOS
ccgo build windows    # Build for Windows
ccgo build linux      # Build for Linux
```

### Scene 4: Key Features (2 minutes)

**[Screen: Feature highlights with icons]**

**Narrator:**
"Here's what makes CCGO special:"

**[Icon 1: Multi-platform]**
"**Universal Platform Support** - Build for Android, iOS, macOS, Windows, Linux, and OpenHarmony from a single codebase."

**[Icon 2: Docker]**
"**Docker Integration** - Build for any platform from any operating system. No need to own a Mac to build iOS libraries."

**[Icon 3: Package managers]**
"**Unified Publishing** - Publish to Maven Central, CocoaPods, Swift Package Manager, OHPM, and Conan with consistent commands."

**[Icon 4: CMake]**
"**CMake Integration** - Works with your existing CMake projects while adding platform-specific enhancements."

**[Icon 5: Templates]**
"**Project Templates** - Generate production-ready project structures in seconds."

### Scene 5: Call to Action (30 seconds)

**[Screen: Installation command]**

**Narrator:**
"Ready to simplify your C++ workflow? Install CCGO today:"

```bash
pip install ccgo
```

**[Screen: Documentation link]**

"Visit our documentation at ccgo.dev for comprehensive guides, tutorials, and examples. Let's build something amazing together!"

**[Screen: CCGO logo with tagline: "Build Once, Deploy Everywhere"]**

---

## Video 2: Quick Start Tutorial (8-10 minutes)

### Scene 1: Installation (1 minute)

**[Screen: Terminal]**

**Narrator:**
"Let's get started with CCGO. First, install it using pip:"

```bash
$ pip install ccgo
$ ccgo --version
CCGO version 0.1.0
```

"That's it! CCGO is installed. Now let's create our first cross-platform C++ library."

### Scene 2: Creating a New Project (2 minutes)

**[Screen: Terminal]**

**Narrator:**
"Use the `ccgo new` command to create a new project:"

```bash
$ ccgo new mylib
```

**[Screen: Interactive prompts]**

"CCGO will ask you a few questions about your project:"

```
📦 Project name: mylib
👤 Author: John Doe
📧 Email: john@example.com
📄 License: MIT
📝 Description: My awesome C++ library
```

**[Screen: Generated project structure]**

"And just like that, you have a complete project structure:"

```
mylib/
├── CCGO.toml              # Project configuration
├── CMakeLists.txt         # Build configuration
├── src/                   # Source code
│   └── mylib.cpp
├── include/               # Public headers
│   └── mylib/
│       └── mylib.h
├── tests/                 # Unit tests
│   └── test_mylib.cpp
└── examples/              # Example applications
    └── example.cpp
```

### Scene 3: Writing Code (1.5 minutes)

**[Screen: Code editor showing include/mylib/mylib.h]**

**Narrator:**
"Let's add some functionality to our library. Open the header file:"

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

"Now the implementation:"

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

### Scene 4: Building for Multiple Platforms (3 minutes)

**[Screen: Terminal]**

**Narrator:**
"Now comes the magic - building for multiple platforms. Let's start with Android:"

```bash
$ ccgo build android
Building for Android...
Architectures: armeabi-v7a, arm64-v8a, x86_64
✓ Build complete: target/android/mylib-android-1.0.0.aar
```

**[Screen: Show generated AAR file]**

"CCGO automatically builds for multiple Android architectures and packages everything into an AAR file."

**[Screen: Terminal]**

"Now iOS:"

```bash
$ ccgo build ios
Building for iOS...
✓ Framework: target/ios/MyLib.framework
✓ XCFramework: target/ios/MyLib.xcframework
```

**[Screen: Show XCFramework structure]**

"CCGO creates both traditional frameworks and modern XCFrameworks that work on devices and simulators."

**[Screen: Terminal]**

"What about Windows? Even if you're on macOS or Linux, CCGO has you covered with Docker:"

```bash
$ ccgo build windows --docker
Pulling Docker image...
Building for Windows...
✓ Static library: target/windows/lib/static/mylib.lib
✓ DLL: target/windows/lib/shared/mylib.dll
```

### Scene 5: Testing (1 minute)

**[Screen: Terminal]**

**Narrator:**
"Let's make sure everything works by running tests:"

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

### Scene 6: Wrap Up (30 seconds)

**[Screen: Summary of created files]**

**Narrator:**
"In just a few minutes, we've created a cross-platform C++ library, built it for Android, iOS, and Windows, and verified it works with tests. CCGO handles all the platform-specific details, letting you focus on writing great code."

---

## Video 3: Advanced Features (10-12 minutes)

### Scene 1: Dependency Management (2 minutes)

**[Screen: CCGO.toml file]**

**Narrator:**
"CCGO makes dependency management simple. Just add dependencies to your CCGO.toml file:"

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
nlohmann_json = { git = "https://github.com/nlohmann/json.git", tag = "v3.11.2" }
```

**[Screen: Terminal]**

"CCGO automatically downloads and builds dependencies:"

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

### Scene 2: Docker Builds (2.5 minutes)

**[Screen: Diagram showing Docker workflow]**

**Narrator:**
"One of CCGO's most powerful features is Docker integration. Build for any platform from any operating system."

**[Screen: macOS terminal]**

"Running macOS but need to build for Linux?"

```bash
$ ccgo build linux --docker
Pulling ccgo-builder-linux image...
Building in Docker container...
✓ Build complete
```

**[Screen: Windows terminal]**

"On Windows but need iOS libraries?"

```bash
$ ccgo build ios --docker
Pulling ccgo-builder-apple image...
Building in Docker container...
✓ XCFramework created
```

**[Screen: Show Docker advantages]**

**Narrator:**
"Docker builds are perfect for:"
- "CI/CD pipelines - consistent builds everywhere"
- "Team development - everyone uses the same toolchain"
- "Cross-compilation - build any platform without installing SDKs"

### Scene 3: Publishing (2.5 minutes)

**[Screen: Publishing workflow diagram]**

**Narrator:**
"Once your library is built, publishing is just as easy. CCGO supports all major package managers with a unified interface."

**[Screen: Terminal - Android/Maven]**

"Publishing an Android library to Maven Central:"

```bash
$ ccgo publish android --registry official
Building AAR...
Signing with PGP...
Uploading to Maven Central...
✓ Published: com.example:mylib:1.0.0
```

**[Screen: Terminal - iOS/CocoaPods]**

"Publishing to CocoaPods:"

```bash
$ ccgo publish apple --manager cocoapods
Validating podspec...
Pushing to CocoaPods Trunk...
✓ Published: MyLib 1.0.0
```

**[Screen: Terminal - Multiple platforms]**

"Or publish to multiple platforms at once:"

```bash
$ ccgo publish android --registry official
$ ccgo publish apple --manager all --push
$ ccgo publish ohos --registry official
```

### Scene 4: Configuration Deep Dive (2 minutes)

**[Screen: CCGO.toml with annotations]**

**Narrator:**
"CCGO.toml is your project's control center. Let's explore the key sections:"

```toml
# Basic project metadata
[package]
name = "mylib"
version = "1.0.0"
description = "My awesome library"

# Library type
[library]
type = "both"  # static, shared, or both

# Build settings
[build]
cpp_standard = "17"
cxxflags = ["-O3", "-Wall"]

# Platform-specific settings
[android]
min_sdk_version = 21
target_sdk_version = 33

[ios]
deployment_target = "12.0"

[windows]
runtime_library = "MD"  # Dynamic CRT
```

**[Screen: Highlight different sections]**

"Each section controls specific aspects of your build, giving you fine-grained control while maintaining simplicity."

### Scene 5: IDE Integration (1.5 minutes)

**[Screen: Terminal]**

**Narrator:**
"CCGO can generate IDE projects for platform-specific development:"

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

### Scene 6: Wrap Up (30 seconds)

**[Screen: Feature summary]**

**Narrator:**
"CCGO's advanced features - dependency management, Docker builds, unified publishing, and IDE integration - work together to create a seamless cross-platform development experience."

---

## Video 4: Real-World Example (15-20 minutes)

### Scene 1: Project Overview (2 minutes)

**[Screen: Project visualization]**

**Narrator:**
"Let's build a real-world C++ library - a image processing library that works on mobile and desktop platforms."

**[Screen: Feature list]**

"Our library will:"
- "Resize and crop images"
- "Apply filters (grayscale, blur, sharpen)"
- "Support multiple image formats"
- "Provide both C++ and platform-native APIs"

### Scene 2: Project Setup (2 minutes)

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
│   ├── image.cpp          # Core image class
│   ├── filters.cpp        # Filter implementations
│   └── platform/          # Platform-specific code
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

### Scene 3: Implementation (4 minutes)

**[Screen: Code editor - image.h]**

**Narrator:**
"Let's implement the core image class:"

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

    // Loading and saving
    bool load(const std::string& path);
    bool save(const std::string& path);

    // Basic operations
    void resize(int new_width, int new_height);
    void crop(int x, int y, int width, int height);

    // Getters
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

### Scene 4: Platform-Specific Bindings (4 minutes)

**[Screen: Android JNI binding]**

**Narrator:**
"For Android, we create JNI bindings:"

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
"For iOS, we create an Objective-C++ wrapper for Swift interop:"

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

### Scene 5: Building and Testing (3 minutes)

**[Screen: Terminal - building for all platforms]**

**Narrator:**
"Now let's build for all platforms:"

```bash
# Mobile platforms
$ ccgo build android
✓ AAR: imagelib-android-1.0.0.aar

$ ccgo build ios
✓ XCFramework: ImageLib.xcframework

# Desktop platforms
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

### Scene 6: Publishing (2 minutes)

**[Screen: Terminal - publishing]**

**Narrator:**
"Finally, let's publish our library:"

```bash
# Android - Maven Central
$ ccgo publish android --registry official
✓ Published to Maven Central

# iOS - CocoaPods and SPM
$ ccgo publish apple --manager all --push
✓ Published to CocoaPods
✓ Tagged for Swift Package Manager

# OpenHarmony - OHPM
$ ccgo publish ohos --registry official
✓ Published to OHPM

# Cross-platform - Conan
$ ccgo publish conan --registry official
✓ Published to Conan Center
```

### Scene 7: Integration Examples (2 minutes)

**[Screen: Android usage example]**

**Narrator:**
"Now developers can easily use our library. In Android:"

```kotlin
// build.gradle.kts
dependencies {
    implementation("com.example:imagelib:1.0.0")
}

// Usage
val image = Image(800, 600)
image.load("/sdcard/photo.jpg")
Filters.grayscale(image)
image.save("/sdcard/photo_gray.jpg")
```

**[Screen: iOS usage example]**

"In iOS with Swift:"

```swift
// Package.swift
dependencies: [
    .package(url: "https://github.com/example/imagelib", from: "1.0.0")
]

// Usage
let image = ImageLib(width: 800, height: 600)
image.load("photo.jpg")
image.grayscale()
image.save("photo_gray.jpg")
```

### Scene 8: Wrap Up (1 minute)

**[Screen: Summary screen]**

**Narrator:**
"In this tutorial, we built a complete cross-platform image processing library with CCGO. We:"

- "Created a full-featured C++ library"
- "Added platform-specific bindings for Android, iOS, and Windows"
- "Built for all platforms with a single tool"
- "Tested thoroughly"
- "Published to multiple package managers"

"CCGO handled all the platform complexities, letting us focus on building great software."

---

## Video 5: Tips and Best Practices (8-10 minutes)

### Scene 1: Project Organization (2 minutes)

**[Screen: Well-organized project structure]**

**Narrator:**
"Let's talk about best practices. First, project organization:"

**[Screen: Directory structure]**

```
mylib/
├── include/          # Public API only
│   └── mylib/
│       └── *.h
├── src/             # Implementation
│   ├── core/        # Core functionality
│   ├── utils/       # Utilities
│   └── platform/    # Platform-specific code
├── tests/           # Unit tests
├── benches/         # Benchmarks
├── examples/        # Usage examples
└── docs/            # Documentation
```

"Keep your public API in include/, internal code in src/, and platform-specific code isolated."

### Scene 2: Version Management (1.5 minutes)

**[Screen: CCGO.toml version]**

**Narrator:**
"Always use semantic versioning:"

```toml
[package]
version = "1.2.3"  # MAJOR.MINOR.PATCH
```

**[Screen: Git workflow]**

"Create tags for releases:"

```bash
$ ccgo tag
Created tag: v1.2.3

$ git push origin v1.2.3
```

### Scene 3: CI/CD Integration (2 minutes)

**[Screen: GitHub Actions workflow]**

**Narrator:**
"Automate your builds with CI/CD:"

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

"This builds and tests all platforms on every commit."

### Scene 4: Docker Best Practices (1.5 minutes)

**[Screen: Docker tips]**

**Narrator:**
"When using Docker builds:"

**[Tip 1]** "Cache images for faster builds - don't delete them between builds"
**[Tip 2]** "Allocate enough resources - 4+ CPUs, 8GB+ RAM"
**[Tip 3]** "Use Docker for CI/CD, native builds for local development"
**[Tip 4]** "Update images monthly for latest toolchains"

### Scene 5: Performance Optimization (1.5 minutes)

**[Screen: Optimization techniques]**

**Narrator:**
"Optimize your builds:"

```toml
[build]
# Link-time optimization
cxxflags = ["-flto"]
ldflags = ["-flto"]

# Platform-specific optimizations
[android]
cxxflags = ["-O3", "-ffast-math"]

[ios]
cxxflags = ["-O3", "-fvectorize"]
```

**[Screen: Build performance comparison]**

"These optimizations can reduce binary size by 30% and improve performance by 20%."

### Scene 6: Troubleshooting Common Issues (1.5 minutes)

**[Screen: Common errors and solutions]**

**Narrator:**
"Common issues and solutions:"

**[Issue 1: Dependency not found]**
```bash
Error: Could not find dependency 'spdlog'

# Solution: Check git URL and tag
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

**[Issue 2: Build failed]**
```bash
Error: Build failed for arm64-v8a

# Solution: Check CMakeLists.txt and build logs
$ ccgo build android --verbose
```

**[Issue 3: Docker permission denied]**
```bash
Error: permission denied (Linux)

# Solution: Add user to docker group
$ sudo usermod -aG docker $USER
```

### Scene 7: Wrap Up (30 seconds)

**[Screen: Best practices checklist]**

**Narrator:**
"Follow these best practices for smooth cross-platform development with CCGO:"

- "✓ Organize code logically"
- "✓ Use semantic versioning"
- "✓ Automate with CI/CD"
- "✓ Leverage Docker for consistency"
- "✓ Optimize for performance"
- "✓ Test on all target platforms"

---

## Closing

**[Screen: CCGO logo]**

**Narrator:**
"Thank you for watching! CCGO simplifies cross-platform C++ development, letting you focus on building great software. Visit ccgo.dev for documentation, examples, and community support. Happy coding!"

**[Screen: Links]**
- Documentation: ccgo.dev/docs
- GitHub: github.com/ccgo-org/ccgo
- Discord: discord.gg/ccgo
- Twitter: @ccgodev

---

## Production Notes

### Video Style
- **Tone**: Professional but approachable
- **Pace**: Moderate - allow time for viewers to follow along
- **Visuals**: Mix of terminal recordings, code editors, and diagrams
- **Music**: Subtle background music, no lyrics

### Screen Recording
- **Resolution**: 1920x1080 minimum
- **Font size**: Large enough for mobile viewing (16pt minimum in terminal)
- **Cursor**: Highlight cursor movements
- **Typing speed**: Moderate, with pauses after commands

### Post-Production
- **Captions**: Add closed captions for accessibility
- **Chapters**: YouTube chapters for easy navigation
- **Timestamps**: In video description
- **Code snippets**: Include in video description

### Supplementary Materials
- **Sample code**: GitHub repository with all examples
- **Cheat sheet**: PDF with common commands
- **Practice exercises**: Guided tutorials
- **FAQ**: Common questions and answers

---

## Target Audience

- **Primary**: C++ developers new to cross-platform development
- **Secondary**: Mobile developers wanting to use C++ for performance-critical code
- **Tertiary**: Teams looking to streamline their cross-platform build process

## Call to Action

Each video should end with:
1. Link to documentation
2. Encourage trying CCGO
3. Join community (Discord/GitHub)
4. Like/subscribe for more tutorials
