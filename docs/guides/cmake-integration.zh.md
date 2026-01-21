# CMake 集成指南

> 版本：v3.1.0 | 更新时间：2026-01-21

本指南说明如何将 CCGO 的 CMake 模块集成到您的 C++ 跨平台项目中，以实现依赖管理、平台特定构建和代码组织。

## 目录

1. [概述](#概述)
2. [CMake 模块文件](#cmake-模块文件)
3. [基本设置](#基本设置)
4. [依赖管理](#依赖管理)
5. [源代码组织](#源代码组织)
6. [平台特定代码](#平台特定代码)
7. [构建配置](#构建配置)
8. [完整示例](#完整示例)
9. [最佳实践](#最佳实践)

---

## 概述

CCGO 提供一组 CMake 模块，简化跨平台 C++ 开发：

- **CCGODependencies.cmake**: 管理来自 CCGO.toml 的依赖
- **CMakeUtils.cmake**: 项目设置的实用函数
- **CMakeFunctions.cmake**: 源代码组织的辅助函数
- **CMakeConfig.cmake**: 项目范围的配置
- **CMakeExtraFlags.cmake**: 编译器标志和优化
- **平台工具链**: ios.toolchain.cmake、windows-msvc.toolchain.cmake 等

所有 CMake 模块都集中在 CCGO 包中，并通过 `CCGO_CMAKE_DIR` 变量访问。

---

## CMake 模块文件

### 模块位置

CMake 模块随 CCGO 一起安装，通过以下方式引用：

```cmake
# CCGO_CMAKE_DIR 由 CCGO 构建系统自动设置
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
```

### 关键模块

| 模块 | 用途 |
|------|------|
| `CCGODependencies.cmake` | 依赖集成函数 |
| `CMakeUtils.cmake` | 项目设置和配置 |
| `CMakeFunctions.cmake` | 源文件收集实用工具 |
| `CMakeConfig.cmake` | 全局项目设置 |
| `CMakeExtraFlags.cmake` | 编译器优化标志 |
| `ios.toolchain.cmake` | iOS 交叉编译工具链 |
| `windows-msvc.toolchain.cmake` | Windows MSVC 工具链 |

---

## 基本设置

### 最小 CMakeLists.txt

```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# 包含 CCGO 实用工具
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# 创建库
add_library(myproject STATIC
    src/main.cpp
    src/utils.cpp
)

# 设置 C++ 标准
target_compile_features(myproject PUBLIC cxx_std_17)
```

### 带 CCGO 依赖

```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# 包含 CCGO 模块
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# 创建库
add_library(myproject STATIC
    src/main.cpp
)

# 添加 CCGO 依赖（来自 CCGO.toml）
ccgo_add_dependencies(myproject)

target_compile_features(myproject PUBLIC cxx_std_17)
```

---

## 依赖管理

CCGO 自动管理来自 `CCGO.toml` 的依赖，并通过 CMake 变量使其可用。

### 可用变量

| 变量 | 描述 |
|------|------|
| `CCGO_DEP_PATHS` | 分号分隔的依赖路径列表 |
| `CCGO_DEP_INCLUDE_DIRS` | 分号分隔的包含目录列表 |
| `CCGO_CMAKE_DIR` | CCGO CMake 模块路径 |

### 函数: ccgo_add_dependencies()

自动将依赖包含目录添加到目标。

**签名:**
```cmake
ccgo_add_dependencies(<target_name>)
```

**示例:**
```cmake
add_library(mylib STATIC src/main.cpp)

# 添加所有 CCGO 依赖包含目录
ccgo_add_dependencies(mylib)
```

这等同于:
```cmake
target_include_directories(mylib PRIVATE
    ${CCGO_DEP_INCLUDE_DIRS}
)
```

### 函数: ccgo_link_dependency()

链接来自 CCGO 依赖的特定库。

**签名:**
```cmake
ccgo_link_dependency(<target_name> <dependency_name> <library_name>)
```

**参数:**
- `target_name`: 您的 CMake 目标
- `dependency_name`: 来自 CCGO.toml 的依赖名称
- `library_name`: 库文件名（不含前缀/扩展名）

**示例:**
```cmake
add_library(mylib STATIC src/main.cpp)

# 从 fmt 依赖链接 fmt 库
ccgo_link_dependency(mylib fmt fmt)

# 从 spdlog 依赖链接 spdlog 库
ccgo_link_dependency(mylib spdlog spdlog)
```

该函数在常见位置搜索库：
- `<dep_path>/lib/`
- `<dep_path>/build/lib/`
- `<dep_path>/cmake_build/lib/`
- `<dep_path>/`

支持不同的命名约定：
- `libfmt.a`、`libfmt.so`、`libfmt.dylib`（Unix）
- `fmt.lib`（Windows）

### 函数: ccgo_add_subdirectory()

将 CCGO 依赖添加为子目录（如果有 CMakeLists.txt）。

**签名:**
```cmake
ccgo_add_subdirectory(<dependency_name>)
```

**示例:**
```cmake
# 将 fmt 依赖添加为子目录
ccgo_add_subdirectory(fmt)

# 现在可以使用 fmt 目标
add_library(mylib STATIC src/main.cpp)
target_link_libraries(mylib PRIVATE fmt::fmt)
```

### 函数: ccgo_print_dependencies()

打印有关可用依赖的调试信息。

**签名:**
```cmake
ccgo_print_dependencies()
```

**示例输出:**
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

### 完整依赖示例

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

# 打印依赖信息（调试）
ccgo_print_dependencies()

# 创建库
add_library(myproject STATIC
    src/main.cpp
    src/logger.cpp
)

# 方法 1: 添加所有依赖（仅包含目录）
ccgo_add_dependencies(myproject)

# 方法 2: 将特定依赖添加为子目录
ccgo_add_subdirectory(fmt)
ccgo_add_subdirectory(spdlog)
target_link_libraries(myproject PRIVATE fmt::fmt spdlog::spdlog)

# 方法 3: 手动链接特定库
# ccgo_link_dependency(myproject fmt fmt)
# ccgo_link_dependency(myproject spdlog spdlog)

target_compile_features(myproject PUBLIC cxx_std_17)
```

---

## 源代码组织

CCGO 提供函数自动从目录结构收集源文件。

### 函数: add_sub_layer_sources_recursively()

递归收集目录树中的所有源文件。

**签名:**
```cmake
add_sub_layer_sources_recursively(<output_variable> <source_directory>)
```

**支持的扩展名:**
- `.cc`、`.c`、`.cpp`（C/C++ 源代码）
- `.mm`、`.m`（Objective-C/C++）

**示例:**
```cmake
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)

# 从 src/ 目录收集所有源代码
set(MY_SOURCES "")
add_sub_layer_sources_recursively(MY_SOURCES ${CMAKE_SOURCE_DIR}/src)

# 使用收集的源代码创建库
add_library(mylib STATIC ${MY_SOURCES})
```

**目录结构:**
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

所有文件都将被收集，平台特定目录（`android/`、`ios/`）会根据构建平台自动过滤。

### 函数: add_subdirectories_recursively()

为给定平台收集有效的子目录。

**签名:**
```cmake
add_subdirectories_recursively(<output_variable> <root_directory>)
```

**平台特定目录:**
- `android/`、`jni/`: 仅在为 Android 构建时包含
- `ohos/`、`napi/`: 仅在为 OpenHarmony 构建时包含
- `ios/`: 仅在为 iOS 构建时包含
- `macos/`、`osx/`: 仅在为 macOS 构建时包含
- `oni/`、`apple/`: 仅在为任何 Apple 平台构建时包含
- `windows/`、`win/`: 仅在为 Windows (MSVC) 构建时包含
- `linux/`: 仅在为 Linux 构建时包含

**示例:**
```cmake
set(SUBDIRS "")
add_subdirectories_recursively(SUBDIRS ${CMAKE_SOURCE_DIR}/src)

message(STATUS "平台特定子目录: ${SUBDIRS}")
```

### 宏: exclude_unittest_files()

在禁用测试时从构建中排除单元测试文件。

**签名:**
```cmake
exclude_unittest_files(<source_list_variable>)
```

**排除模式:**
- `*_unittest.cc`
- `*_test.cc`
- `*_mock.cc`

**示例:**
```cmake
file(GLOB MY_SOURCES src/*.cc)

# 如果 GOOGLETEST_SUPPORT 为 OFF 则排除测试文件
exclude_unittest_files(MY_SOURCES)

add_library(mylib STATIC ${MY_SOURCES})
```

---

## 平台特定代码

### 平台检测变量

CCGO 设置标准 CMake 变量用于平台检测：

| 变量 | 平台 |
|------|------|
| `ANDROID` | Android |
| `APPLE` | macOS 或 iOS |
| `IOS` | iOS 特指 |
| `OHOS` | OpenHarmony |
| `MSVC` | Windows (MSVC) |
| `UNIX` | 类 Unix（Linux、macOS）|

### 条件编译

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

### 平台特定包含目录

```cmake
# 包含平台 API 目录
include_directories(${CMAKE_SOURCE_DIR}/include/${PROJECT_NAME}/api/ios/)
include_directories(${CMAKE_SOURCE_DIR}/include/${PROJECT_NAME}/api/macos/)
include_directories(${CMAKE_SOURCE_DIR}/include/${PROJECT_NAME}/api/apple/)
```

这在包含 CMakeUtils.cmake 时会自动完成。

---

## 构建配置

### 常见配置选项

```cmake
# 设置 C++ 标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 导出编译命令（用于 IDE 集成）
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# 符号可见性（default 或 hidden）
set(CMAKE_CXX_VISIBILITY_PRESET default)
set(CMAKE_C_VISIBILITY_PRESET default)

# 构建类型
set(CMAKE_CONFIGURATION_TYPES "Debug;Release" CACHE STRING "" FORCE)
```

### CCGO 特定选项

```cmake
# 启用安装规则
option(CCGO_ENABLE_INSTALL "Enable install rule" ON)

# 使用系统包含（抑制警告）
option(CCGO_USE_SYSTEM_INCLUDES "Use SYSTEM for includes" OFF)

# 日志标签前缀
set(CCGO_TAG_PREFIX "MyProject")

# Git 修订版本
execute_process(
    COMMAND git rev-parse --short HEAD
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
    OUTPUT_VARIABLE CCGO_REVISION
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
add_definitions(-DCCGO_REVISION="${CCGO_REVISION}")
```

### 第三方库选项

```cmake
# 启用第三方库
option(GOOGLETEST_SUPPORT "Use GoogleTest for unit tests" OFF)
option(BENCHMARK_SUPPORT "Use GoogleBenchmark for benchmarks" OFF)
option(RAPIDJSON_SUPPORT "Use RapidJSON for JSON support" ON)
```

---

## 完整示例

### 示例 1: 简单库

```cmake
cmake_minimum_required(VERSION 3.18)
project(SimpleLib VERSION 1.0.0 LANGUAGES CXX)

# 包含 CCGO 实用工具
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# 收集源代码
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)

# 创建库
add_library(simplelib STATIC ${SOURCES})

target_compile_features(simplelib PUBLIC cxx_std_17)
target_include_directories(simplelib
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
)
```

### 示例 2: 带依赖的库

```cmake
cmake_minimum_required(VERSION 3.18)
project(AdvancedLib VERSION 1.0.0 LANGUAGES CXX)

# 包含 CCGO 模块
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# 调试: 打印依赖
ccgo_print_dependencies()

# 收集源代码
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)
exclude_unittest_files(SOURCES)

# 创建库
add_library(advancedlib STATIC ${SOURCES})

# 添加 CCGO 依赖
ccgo_add_dependencies(advancedlib)

# 将特定依赖添加为子目录
ccgo_add_subdirectory(fmt)
ccgo_add_subdirectory(spdlog)

# 链接库
target_link_libraries(advancedlib
    PRIVATE
        fmt::fmt
        spdlog::spdlog
)

target_compile_features(advancedlib PUBLIC cxx_std_17)
```

### 示例 3: 平台特定构建

```cmake
cmake_minimum_required(VERSION 3.18)
project(CrossPlatformLib VERSION 1.0.0)

include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)

# 收集通用源代码
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)

# 创建库
add_library(crossplatformlib STATIC ${SOURCES})

# 平台特定配置
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

### 示例 4: 带测试的库

```cmake
cmake_minimum_required(VERSION 3.18)
project(LibWithTests VERSION 1.0.0)

include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# 主库
set(LIB_SOURCES "")
add_sub_layer_sources_recursively(LIB_SOURCES ${CMAKE_SOURCE_DIR}/src)
exclude_unittest_files(LIB_SOURCES)

add_library(mylib STATIC ${LIB_SOURCES})
ccgo_add_dependencies(mylib)
target_compile_features(mylib PUBLIC cxx_std_17)

# 测试（如果启用）
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

## 最佳实践

### 1. 始终使用 CCGO_CMAKE_DIR

```cmake
# ✅ 好: 使用变量
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# ❌ 坏: 硬编码路径
include(/usr/local/lib/ccgo/cmake/CMakeUtils.cmake)
```

### 2. 利用自动源代码收集

```cmake
# ✅ 好: 使用辅助函数
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)
add_library(mylib STATIC ${SOURCES})

# ❌ 坏: 手动列出文件
add_library(mylib STATIC
    src/a.cpp
    src/b.cpp
    src/c.cpp
    # ... 数百个文件
)
```

### 3. 使用平台特定目录

```
src/
├── core/           # 通用代码
├── android/        # 仅 Android（自动过滤）
├── ios/            # 仅 iOS（自动过滤）
├── macos/          # 仅 macOS（自动过滤）
└── windows/        # 仅 Windows（自动过滤）
```

### 4. 正确处理依赖

```cmake
# 选项 1: 仅包含目录（轻量级）
ccgo_add_dependencies(mylib)

# 选项 2: 添加为子目录（完全集成）
ccgo_add_subdirectory(fmt)
target_link_libraries(mylib PRIVATE fmt::fmt)

# 选项 3: 手动链接特定库（精细控制）
ccgo_link_dependency(mylib fmt fmt)
```

### 5. 使用现代 CMake 目标

```cmake
# ✅ 好: 基于目标
target_include_directories(mylib PUBLIC ${CMAKE_SOURCE_DIR}/include)
target_compile_features(mylib PUBLIC cxx_std_17)
target_link_libraries(mylib PRIVATE fmt::fmt)

# ❌ 坏: 基于目录
include_directories(${CMAKE_SOURCE_DIR}/include)
set(CMAKE_CXX_STANDARD 17)
link_libraries(fmt)
```

### 6. 组织 CMakeLists.txt

```cmake
# 1. CMake 版本和项目
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# 2. 包含 CCGO 模块
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# 3. 选项和配置
option(BUILD_TESTS "Build tests" OFF)

# 4. 收集源代码
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)

# 5. 定义目标
add_library(myproject STATIC ${SOURCES})

# 6. 配置目标
target_compile_features(myproject PUBLIC cxx_std_17)
target_include_directories(myproject PUBLIC ...)
ccgo_add_dependencies(myproject)

# 7. 平台特定设置
if(ANDROID)
    # Android 特定
endif()

# 8. 测试（如果启用）
if(BUILD_TESTS)
    add_subdirectory(tests)
endif()
```

---

## 故障排除

### CCGO_CMAKE_DIR 未定义

**问题:** `CCGO_CMAKE_DIR` 变量未设置。

**解决方案:** 该变量由 CCGO 构建系统自动设置。确保您使用以下命令构建：
```bash
ccgo build <platform>
```

不要直接调用 CMake，除非手动设置：
```bash
cmake -DCCGO_CMAKE_DIR=/path/to/ccgo/cmake ...
```

### 找不到依赖

**问题:** `ccgo_add_dependencies()` 不起作用或有关缺少依赖的警告。

**解决方案:**
1. 运行 `ccgo install` 从 CCGO.toml 获取依赖
2. 验证 `CCGO_DEP_PATHS` 和 `CCGO_DEP_INCLUDE_DIRS` 已设置：
   ```cmake
   ccgo_print_dependencies()
   ```

### 未包含平台特定代码

**问题:** 像 `src/android/` 这样的平台特定目录未包含在构建中。

**解决方案:** 确保使用 `add_sub_layer_sources_recursively()`，它会根据平台自动过滤：
```cmake
set(SOURCES "")
add_sub_layer_sources_recursively(SOURCES ${CMAKE_SOURCE_DIR}/src)
```

### 循环依赖

**问题:** 使用 `ccgo_add_subdirectory()` 时 CMake 报告循环依赖。

**解决方案:** 某些依赖可能有循环引用。改用 `ccgo_link_dependency()`：
```cmake
# 而不是:
# ccgo_add_subdirectory(problematic_dep)

# 使用:
ccgo_link_dependency(mylib problematic_dep lib_name)
```

---

## 另请参阅

- [CCGO.toml 配置参考](../reference/config.zh.md)
- [CLI 参考](../reference/cli.zh.md)
- [平台指南](../platforms/index.zh.md)
- [依赖管理](../features/dependency-management.zh.md)
