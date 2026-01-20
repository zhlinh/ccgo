# 项目结构

CCGO 项目组织和目录结构的完整指南。

## 概述

CCGO 项目遵循标准化结构：

- **源代码**：`src/` 用于实现，`include/` 用于头文件
- **测试**：`tests/` 用于单元测试
- **基准测试**：`benches/` 用于性能测试
- **文档**：`docs/` 用于 Doxygen/Sphinx 文档
- **配置**：`CCGO.toml` 用于项目设置
- **构建输出**：`target/` 用于编译后的二进制文件

## 标准项目布局

```
myproject/
├── CCGO.toml                    # 项目配置
├── CMakeLists.txt               # CMake 配置（生成）
├── build_config.py              # 特定于构建的设置（生成）
├── .ccgoignore                  # 要从 CCGO 操作中排除的文件
├── README.md                    # 项目文档
├── LICENSE                      # 许可证文件
├── .gitignore                   # Git 忽略规则
│
├── include/                     # 公共头文件
│   └── myproject/
│       ├── myproject.h          # 主头文件
│       ├── version.h            # 版本信息（生成）
│       └── feature.h            # 特性专用头文件
│
├── src/                         # 实现文件
│   ├── myproject.cpp            # 主实现
│   ├── feature.cpp              # 特性实现
│   └── internal/                # 私有实现
│       └── utils.cpp
│
├── tests/                       # 单元测试
│   ├── test_main.cpp            # 测试运行器
│   ├── test_myproject.cpp       # 主模块测试
│   └── test_feature.cpp         # 特性测试
│
├── benches/                     # 基准测试
│   ├── bench_main.cpp           # 基准测试运行器
│   └── bench_performance.cpp    # 性能基准测试
│
├── docs/                        # 文档
│   ├── Doxyfile                 # Doxygen 配置
│   ├── api/                     # API 文档
│   └── guides/                  # 用户指南
│
├── examples/                    # 示例应用（可选）
│   ├── basic/
│   │   └── main.cpp
│   └── advanced/
│       └── main.cpp
│
├── cmake_build/                 # CMake 构建目录（生成）
│   ├── android/
│   ├── ios/
│   ├── linux/
│   └── windows/
│
└── target/                      # 构建输出（生成）
    ├── android/
    ├── ios/
    ├── linux/
    └── windows/
```

## 目录详情

### 根文件

#### CCGO.toml

**用途：** 主项目配置文件。

**内容：**
```toml
[package]
name = "myproject"
version = "1.0.0"
description = "My C++ project"

[library]
type = "both"

[build]
cpp_standard = "17"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

**参见：** [配置指南](configuration.md)

#### CMakeLists.txt

**用途：** CMake 配置（由 CCGO 自动生成）。

**内容：**
```cmake
cmake_minimum_required(VERSION 3.20)
project(myproject VERSION 1.0.0)

# CCGO cmake 实用程序
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# 添加子目录
add_subdirectory(src)
add_subdirectory(tests)
add_subdirectory(benches)
```

**注意：** 不要手动编辑 - 每次构建时重新生成。

#### build_config.py

**用途：** 特定于构建的设置（自动生成）。

**内容：**
```python
PROJECT_NAME = "myproject"
PROJECT_VERSION = "1.0.0"
BUILD_TYPE = "release"
CPP_STANDARD = "17"
```

**用法：** 由构建脚本内部使用。

#### .ccgoignore

**用途：** 从 CCGO 操作中排除文件。

**内容：**
```
# 构建目录
cmake_build/
target/
bin/

# IDE 文件
.vscode/
.idea/

# 生成的文件
*.pyc
```

**参见：** [配置指南 - .ccgoignore](configuration.md#ccgoignore)

### include/

**用途：** 库用户将包含的公共头文件。

**结构：**
```
include/
└── myproject/               # 命名空间目录（必需）
    ├── myproject.h          # 主头文件
    ├── version.h            # 版本信息（生成）
    ├── config.h             # 配置（生成）
    ├── feature_a.h          # 特性头文件
    ├── feature_b.h
    └── internal/            # 内部头文件（不供公开使用）
        └── impl.h
```

**指南：**

1. **命名空间目录：** 始终使用与项目名称匹配的子目录
   ```cpp
   // 好
   #include <myproject/myproject.h>

   // 差 - 没有命名空间
   #include <myproject.h>
   ```

2. **头文件保护：** 使用 `#pragma once` 或传统保护
   ```cpp
   #pragma once

   namespace myproject {
   // ...
   }
   ```

3. **版本头文件：** 自动生成版本信息
   ```cpp
   // version.h（生成）
   #pragma once

   #define MYPROJECT_VERSION_MAJOR 1
   #define MYPROJECT_VERSION_MINOR 0
   #define MYPROJECT_VERSION_PATCH 0
   #define MYPROJECT_VERSION "1.0.0"
   ```

4. **最小化包含：** 只包含必要的内容
   ```cpp
   // 好 - 尽可能前向声明
   class FeatureA;

   // 差 - 如果不需要则包含
   #include "feature_a.h"
   ```

### src/

**用途：** 实现文件（.cpp、.cc、.cxx）。

**结构：**
```
src/
├── myproject.cpp            # 主实现
├── feature_a.cpp            # 特性实现
├── feature_b.cpp
├── internal/                # 私有实现
│   ├── utils.cpp
│   └── platform/            # 平台特定代码
│       ├── android.cpp
│       ├── ios.cpp
│       └── linux.cpp
└── CMakeLists.txt           # 由 CCGO 生成
```

**指南：**

1. **每个文件一个类：** 保持文件专注
   ```
   src/
   ├── feature_a.cpp         # FeatureA 实现
   └── feature_b.cpp         # FeatureB 实现
   ```

2. **私有头文件：** 使用 internal/ 存放私有头文件
   ```cpp
   // src/internal/utils.h
   #pragma once
   // 不供公开使用的内部实用程序
   ```

3. **平台特定代码：** 使用子目录或条件编译
   ```cpp
   // 方式 1：单独文件
   #ifdef __ANDROID__
   #include "platform/android.cpp"
   #elif defined(__APPLE__)
   #include "platform/ios.cpp"
   #endif

   // 方式 2：条件块
   void platform_init() {
   #ifdef __ANDROID__
       // Android 特定
   #elif defined(__APPLE__)
       // iOS 特定
   #endif
   }
   ```

### tests/

**用途：** 使用 Catch2 或 Google Test 的单元测试。

**结构：**
```
tests/
├── test_main.cpp            # 测试运行器
├── test_myproject.cpp       # 主模块测试
├── test_feature_a.cpp       # 特性专用测试
├── test_feature_b.cpp
└── CMakeLists.txt           # 由 CCGO 生成
```

**测试文件示例：**

```cpp
// test_myproject.cpp
#include <catch2/catch_test_macros.hpp>
#include <myproject/myproject.h>

TEST_CASE("Basic functionality", "[myproject]") {
    myproject::MyClass obj;
    REQUIRE(obj.get_value() == 42);
}

TEST_CASE("Feature A", "[feature_a]") {
    myproject::FeatureA feature;
    REQUIRE(feature.is_enabled());
}
```

**测试运行器：**

```cpp
// test_main.cpp
#define CATCH_CONFIG_MAIN
#include <catch2/catch_test_macros.hpp>
```

**运行测试：**

```bash
ccgo test                    # 运行所有测试
ccgo test --filter "Basic"   # 运行特定测试
```

### benches/

**用途：** 使用 Google Benchmark 的性能基准测试。

**结构：**
```
benches/
├── bench_main.cpp           # 基准测试运行器
├── bench_performance.cpp    # 性能基准测试
├── bench_feature_a.cpp      # 特性专用基准测试
└── CMakeLists.txt           # 由 CCGO 生成
```

**基准测试示例：**

```cpp
// bench_performance.cpp
#include <benchmark/benchmark.h>
#include <myproject/myproject.h>

static void BM_MyFunction(benchmark::State& state) {
    myproject::MyClass obj;
    for (auto _ : state) {
        obj.do_work();
    }
}
BENCHMARK(BM_MyFunction);

BENCHMARK_MAIN();
```

**运行基准测试：**

```bash
ccgo bench                   # 运行所有基准测试
ccgo bench --filter "MyFunc" # 运行特定基准测试
```

### docs/

**用途：** 文档源文件。

**结构：**
```
docs/
├── Doxyfile                 # Doxygen 配置
├── README.md                # 文档概览
├── api/                     # API 文档
│   └── reference.md
├── guides/                  # 用户指南
│   ├── getting-started.md
│   └── advanced.md
└── images/                  # 文档图片
    └── architecture.png
```

**生成文档：**

```bash
ccgo doc                     # 生成文档
ccgo doc --open              # 生成并在浏览器中打开
```

### examples/

**用途：** 展示如何使用您的库的示例应用。

**结构：**
```
examples/
├── basic/                   # 简单示例
│   ├── main.cpp
│   └── CMakeLists.txt
├── advanced/                # 高级用法
│   ├── main.cpp
│   └── CMakeLists.txt
└── README.md                # 示例文档
```

**示例应用：**

```cpp
// examples/basic/main.cpp
#include <myproject/myproject.h>
#include <iostream>

int main() {
    myproject::MyClass obj;
    std::cout << "Value: " << obj.get_value() << std::endl;
    return 0;
}
```

**构建示例：**

```cmake
# examples/basic/CMakeLists.txt
cmake_minimum_required(VERSION 3.20)
project(basic_example)

find_package(myproject REQUIRED)

add_executable(basic main.cpp)
target_link_libraries(basic myproject::myproject)
```

### cmake_build/

**用途：** CMake 构建工件（生成，不提交到 git）。

**结构：**
```
cmake_build/
├── android/                 # Android 构建文件
│   ├── armeabi-v7a/
│   ├── arm64-v8a/
│   └── x86_64/
├── ios/                     # iOS 构建文件
│   ├── arm64/
│   └── x86_64/
├── linux/                   # Linux 构建文件
├── windows/                 # Windows 构建文件
└── macos/                   # macOS 构建文件
```

**注意：** 添加到 `.gitignore`：

```
cmake_build/
```

### target/

**用途：** 最终构建输出（生成）。

**结构：**
```
target/
├── android/                             # Android 输出
│   └── MyProject_Android_SDK-1.0.0.zip
├── ios/                                 # iOS 输出
│   └── MyProject_iOS_SDK-1.0.0.zip
├── linux/                               # Linux 输出
│   └── MyProject_Linux_SDK-1.0.0.zip
├── windows/                             # Windows 输出
│   └── MyProject_Windows_SDK-1.0.0.zip
└── doc/                                 # 文档
    └── html/
```

**注意：** 添加到 `.gitignore`：

```
target/
```

## 文件命名约定

### 头文件

```
include/myproject/
├── myproject.h              # 主头文件（小写，匹配项目）
├── feature_a.h              # 特性头文件（小写，下划线）
├── my_class.h               # 类头文件（小写，下划线）
└── constants.h              # 实用程序头文件
```

### 源文件

```
src/
├── myproject.cpp            # 匹配头文件名
├── feature_a.cpp
├── my_class.cpp
└── internal/
    └── utils.cpp            # 私有实用程序
```

### 测试文件

```
tests/
├── test_myproject.cpp       # 前缀 "test_"
├── test_feature_a.cpp
└── test_my_class.cpp
```

### 基准测试文件

```
benches/
├── bench_performance.cpp    # 前缀 "bench_"
├── bench_feature_a.cpp
└── bench_algorithms.cpp
```

## 多模块项目

对于具有多个库的项目：

```
workspace/
├── CCGO.toml                # 工作区配置（可选）
├── lib_core/                # 核心库
│   ├── CCGO.toml
│   ├── include/
│   └── src/
├── lib_utils/               # 实用程序库
│   ├── CCGO.toml
│   ├── include/
│   └── src/
└── app/                     # 应用
    ├── CCGO.toml
    ├── src/
    └── main.cpp
```

**工作区 CCGO.toml：**

```toml
[workspace]
members = [
    "lib_core",
    "lib_utils",
    "app"
]
```

**模块依赖项：**

```toml
# app/CCGO.toml
[dependencies]
lib_core = { path = "../lib_core" }
lib_utils = { path = "../lib_utils" }
```

## 平台特定文件

### Android

```
project/
├── android/                 # Android 特定文件（可选）
│   ├── gradle.properties
│   └── proguard-rules.pro
└── CCGO.toml
    [android]
    min_sdk_version = 21
```

### iOS

```
project/
├── ios/                     # iOS 特定文件（可选）
│   ├── Info.plist
│   └── Entitlements.plist
└── CCGO.toml
    [ios]
    deployment_target = "12.0"
```

### Windows

```
project/
├── windows/                 # Windows 特定文件（可选）
│   ├── resource.rc
│   └── app.manifest
└── CCGO.toml
    [windows]
    subsystem = "console"
```

## 最佳实践

### 1. 一致的命名

使用一致的命名约定：

```
# 好 - 一致的小写带下划线
include/mylib/my_feature.h
src/my_feature.cpp
tests/test_my_feature.cpp

# 差 - 不一致的大小写
include/mylib/MyFeature.h
src/my_feature.cpp
tests/TestMyFeature.cpp
```

### 2. 按特性组织

将相关文件分组：

```
src/
├── networking/              # 网络特性
│   ├── client.cpp
│   ├── server.cpp
│   └── protocol.cpp
└── database/                # 数据库特性
    ├── connection.cpp
    └── query.cpp
```

### 3. 分离公共和私有

保持 API 表面干净：

```
include/
└── mylib/
    ├── public_api.h         # 仅公共 API
    └── internal/            # 内部细节
        └── impl.h
```

### 4. 文档靠近代码

将文档保持在源代码附近：

```
src/
├── feature_a/
│   ├── feature_a.cpp
│   ├── feature_a.h
│   └── README.md            # 特性文档
```

### 5. 示例即测试

示例应该可以编译和运行：

```
examples/
└── basic/
    ├── main.cpp             # 可工作的示例
    ├── CMakeLists.txt       # 构建配置
    └── README.md            # 如何运行
```

## .gitignore 模板

CCGO 项目推荐的 `.gitignore`：

```gitignore
# 构建目录
cmake_build/
target/
bin/
build/

# IDE 文件
.vscode/
.idea/
*.swp
*.swo
*~

# 操作系统文件
.DS_Store
Thumbs.db
desktop.ini

# Python
__pycache__/
*.pyc
*.pyo
*.egg-info/

# 生成的文件
*.autosave
*.user
*.log

# 依赖项
vendor/
```

## 从现有项目迁移

### 从 CMake 项目

1. **创建 CCGO.toml：**
   ```bash
   ccgo init
   ```

2. **复制源文件：**
   ```bash
   mkdir -p include/mylib src
   cp src/*.h include/mylib/
   cp src/*.cpp src/
   ```

3. **配置 CCGO.toml：**
   ```toml
   [package]
   name = "mylib"
   version = "1.0.0"

   [build]
   cpp_standard = "17"
   ```

4. **构建：**
   ```bash
   ccgo build linux
   ```

### 从纯头文件库

1. **将头文件保留在 include/ 中：**
   ```
   include/
   └── mylib/
       ├── mylib.h
       └── impl.h
   ```

2. **配置为纯头文件：**
   ```toml
   [library]
   type = "header-only"
   ```

### 从多个 CMakeLists.txt

1. **合并到 CCGO 结构：**
   ```
   旧：
   project/
   ├── CMakeLists.txt
   ├── module1/
   │   └── CMakeLists.txt
   └── module2/
       └── CMakeLists.txt

   新：
   project/
   ├── CCGO.toml
   ├── module1/
   │   ├── CCGO.toml
   │   └── src/
   └── module2/
       ├── CCGO.toml
       └── src/
   ```

2. **使用工作区：**
   ```toml
   # 根 CCGO.toml
   [workspace]
   members = ["module1", "module2"]
   ```

## 常见模式

### 带示例的库

```
mylib/
├── CCGO.toml
├── include/
├── src/
├── tests/
└── examples/
    ├── basic/
    ├── advanced/
    └── integration/
```

### 带工具的库

```
mylib/
├── CCGO.toml
├── include/
├── src/
├── tests/
└── tools/
    ├── converter/
    └── analyzer/
```

### 带插件的库

```
mylib/
├── CCGO.toml
├── include/
├── src/
├── plugins/
│   ├── plugin_a/
│   └── plugin_b/
└── tests/
```

## 另请参阅

- [安装指南](installation.zh.md)
- [配置指南](configuration.zh.md)
- [构建系统](../features/build-system.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
