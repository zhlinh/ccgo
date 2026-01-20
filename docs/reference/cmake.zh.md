# CMake 集成

CCGO 项目中 CMake 集成的完整参考，包括构建系统架构、自定义和最佳实践。

## 概览

CCGO 使用 CMake 作为所有 C++ 跨平台构建的底层构建系统：

- **模块化 CMake** - 源代码、测试、基准测试和依赖的独立模板
- **平台抽象** - 所有平台统一的构建接口
- **工具链支持** - 预配置的交叉编译工具链
- **依赖管理** - 自动集成第三方库
- **构建自定义** - 广泛的配置选项
- **IDE 集成** - 生成 Visual Studio、Xcode、CodeLite 项目

## CMake 结构

### CCGO CMake 目录

CCGO 将所有 CMake 配置集中在包安装中：

```
ccgo/build_scripts/cmake/
├── CMakeLists.txt.dependencies.example  # 依赖配置
├── CMakeConfig.cmake                    # 全局配置
├── CMakeExtraFlags.cmake                # 编译器标志
├── CMakeFunctions.cmake                 # 辅助函数
├── CMakeUtils.cmake                     # 实用函数
├── FindCCGODependencies.cmake           # 依赖查找器
├── CCGODependencies.cmake               # 依赖解析器
├── ios.toolchain.cmake                  # iOS 交叉编译
├── tvos.toolchain.cmake                 # tvOS 交叉编译
├── watchos.toolchain.cmake              # watchOS 交叉编译
├── windows-msvc.toolchain.cmake         # Windows MSVC 工具链
└── template/                            # CMake 模板
    ├── Root.CMakeLists.txt.in           # 根 CMakeLists
    ├── Src.CMakeLists.txt.in            # 源代码 CMakeLists
    ├── Src.SubDir.CMakeLists.txt.in     # 子目录 CMakeLists
    ├── Tests.CMakeLists.txt.in          # 测试 CMakeLists
    ├── Benches.CMakeLists.txt.in        # 基准测试 CMakeLists
    ├── ThirdParty.CMakeLists.txt.in     # 第三方 CMakeLists
    ├── External.CMakeLists.txt.in       # 外部项目 CMakeLists
    └── External.Download.txt.in         # 下载脚本
```

### 项目 CMake 结构

生成的项目引用 CCGO 的 CMake 文件：

```
my-project/
├── CMakeLists.txt                       # 根 CMake 配置
├── src/
│   └── CMakeLists.txt                   # 源代码构建配置
├── tests/
│   └── CMakeLists.txt                   # 测试构建配置
├── benches/
│   └── CMakeLists.txt                   # 基准测试构建配置
└── cmake_build/                         # 构建输出
    ├── android/
    ├── ios/
    ├── macos/
    ├── windows/
    └── linux/
```

## 根 CMakeLists.txt

### 基本结构

```cmake
cmake_minimum_required(VERSION 3.20)

# 项目定义
project(MyLib
    VERSION 1.0.0
    DESCRIPTION "My C++ Library"
    LANGUAGES CXX
)

# C++ 标准
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# CCGO CMake 目录（由 ccgo build 设置）
if(NOT DEFINED CCGO_CMAKE_DIR)
    message(FATAL_ERROR "CCGO_CMAKE_DIR must be set")
endif()

# 包含 CCGO 实用工具
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)
include(${CCGO_CMAKE_DIR}/CMakeConfig.cmake)

# 平台检测
ccgo_detect_platform()

# 构建配置
option(BUILD_SHARED_LIBS "Build shared libraries" ON)
option(BUILD_TESTS "Build tests" OFF)
option(BUILD_BENCHES "Build benchmarks" OFF)

# 子目录
add_subdirectory(src)

if(BUILD_TESTS)
    enable_testing()
    add_subdirectory(tests)
endif()

if(BUILD_BENCHES)
    add_subdirectory(benches)
endif()
```

### 版本注入

```cmake
# 版本配置
set(PROJECT_VERSION_MAJOR 1)
set(PROJECT_VERSION_MINOR 0)
set(PROJECT_VERSION_PATCH 0)
set(PROJECT_VERSION "${PROJECT_VERSION_MAJOR}.${PROJECT_VERSION_MINOR}.${PROJECT_VERSION_PATCH}")

# Git 信息（由 CCGO 注入）
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

# 生成版本头文件
configure_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/include/${PROJECT_NAME}/version.h.in"
    "${CMAKE_CURRENT_BINARY_DIR}/include/${PROJECT_NAME}/version.h"
    @ONLY
)
```

## 源代码 CMakeLists.txt

### 库定义

```cmake
# src/CMakeLists.txt

# 源文件
set(SOURCES
    mylib.cpp
    utils.cpp
    network.cpp
)

# 公共头文件
set(PUBLIC_HEADERS
    ${CMAKE_SOURCE_DIR}/include/mylib/mylib.h
    ${CMAKE_SOURCE_DIR}/include/mylib/utils.h
    ${CMAKE_SOURCE_DIR}/include/mylib/network.h
)

# 私有头文件
set(PRIVATE_HEADERS
    internal/config.h
    internal/helpers.h
)

# 创建库
add_library(${PROJECT_NAME}
    ${SOURCES}
    ${PUBLIC_HEADERS}
    ${PRIVATE_HEADERS}
)

# 包含目录
target_include_directories(${PROJECT_NAME}
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}
        ${CMAKE_CURRENT_BINARY_DIR}
)

# 编译器定义
target_compile_definitions(${PROJECT_NAME}
    PRIVATE
        MYLIB_VERSION="${PROJECT_VERSION}"
        $<$<CONFIG:Debug>:MYLIB_DEBUG>
)

# 编译器选项
target_compile_options(${PROJECT_NAME}
    PRIVATE
        $<$<CXX_COMPILER_ID:MSVC>:/W4>
        $<$<NOT:$<CXX_COMPILER_ID:MSVC>>:-Wall -Wextra -Wpedantic>
)

# 链接库
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        # 消费者可见的公共依赖
    PRIVATE
        # 消费者不可见的私有依赖
        Threads::Threads
)

# 平台特定配置
ccgo_configure_platform_target(${PROJECT_NAME})

# 为共享库导出符号
if(BUILD_SHARED_LIBS)
    target_compile_definitions(${PROJECT_NAME}
        PRIVATE MYLIB_BUILDING_DLL
        INTERFACE MYLIB_USING_DLL
    )
endif()

# 安装规则
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

### 子目录组织

```cmake
# src/CMakeLists.txt

# 核心库
add_subdirectory(core)

# 平台特定模块
if(CCGO_PLATFORM STREQUAL "android")
    add_subdirectory(jni)
elseif(CCGO_PLATFORM MATCHES "ios|macos")
    add_subdirectory(objc)
elseif(CCGO_PLATFORM STREQUAL "windows")
    add_subdirectory(win32)
endif()

# 可选功能
option(ENABLE_NETWORKING "Enable networking module" ON)
if(ENABLE_NETWORKING)
    add_subdirectory(network)
endif()
```

## 测试 CMakeLists.txt

### 测试配置

```cmake
# tests/CMakeLists.txt

# 查找测试框架
find_package(GTest REQUIRED)

# 测试源文件
set(TEST_SOURCES
    test_main.cpp
    test_calculator.cpp
    test_network.cpp
)

# 创建测试可执行文件
add_executable(${PROJECT_NAME}_tests ${TEST_SOURCES})

# 链接测试框架和库
target_link_libraries(${PROJECT_NAME}_tests
    PRIVATE
        ${PROJECT_NAME}
        GTest::gtest
        GTest::gtest_main
)

# 包含目录
target_include_directories(${PROJECT_NAME}_tests
    PRIVATE
        ${CMAKE_SOURCE_DIR}/include
        ${CMAKE_CURRENT_SOURCE_DIR}
)

# 发现测试
include(GoogleTest)
gtest_discover_tests(${PROJECT_NAME}_tests)

# 平台特定测试配置
ccgo_configure_platform_tests(${PROJECT_NAME}_tests)
```

## 基准测试 CMakeLists.txt

### 基准测试配置

```cmake
# benches/CMakeLists.txt

# 查找基准测试框架
find_package(benchmark REQUIRED)

# 基准测试源文件
set(BENCH_SOURCES
    bench_main.cpp
    bench_calculator.cpp
    bench_network.cpp
)

# 创建基准测试可执行文件
add_executable(${PROJECT_NAME}_benches ${BENCH_SOURCES})

# 链接基准测试框架和库
target_link_libraries(${PROJECT_NAME}_benches
    PRIVATE
        ${PROJECT_NAME}
        benchmark::benchmark
        benchmark::benchmark_main
)

# 包含目录
target_include_directories(${PROJECT_NAME}_benches
    PRIVATE
        ${CMAKE_SOURCE_DIR}/include
        ${CMAKE_CURRENT_SOURCE_DIR}
)

# 平台特定基准测试配置
ccgo_configure_platform_benches(${PROJECT_NAME}_benches)
```

## 平台特定配置

### Android

```cmake
if(ANDROID)
    # Android API 级别
    set(ANDROID_PLATFORM android-${ANDROID_API_LEVEL})

    # 架构特定标志
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

    # 链接 Android 库
    target_link_libraries(${PROJECT_NAME}
        PUBLIC
            android
            log
    )

    # 在发布版本中剥离符号
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
    # Framework 配置
    if(IOS OR TVOS OR WATCHOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            FRAMEWORK TRUE
            FRAMEWORK_VERSION A
            MACOSX_FRAMEWORK_IDENTIFIER com.example.${PROJECT_NAME}
            PUBLIC_HEADER "${PUBLIC_HEADERS}"
        )
    endif()

    # 部署目标
    if(IOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            XCODE_ATTRIBUTE_IPHONEOS_DEPLOYMENT_TARGET "12.0"
        )
    elseif(MACOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            XCODE_ATTRIBUTE_MACOSX_DEPLOYMENT_TARGET "10.14"
        )
    endif()

    # 代码签名（仅 iOS）
    if(IOS)
        set_target_properties(${PROJECT_NAME} PROPERTIES
            XCODE_ATTRIBUTE_CODE_SIGN_IDENTITY "iPhone Developer"
            XCODE_ATTRIBUTE_DEVELOPMENT_TEAM "${DEVELOPMENT_TEAM_ID}"
        )
    endif()

    # 链接 Apple 框架
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
    # MSVC 特定配置
    if(MSVC)
        # 运行时库
        set_property(TARGET ${PROJECT_NAME} PROPERTY
            MSVC_RUNTIME_LIBRARY "MultiThreaded$<$<CONFIG:Debug>:Debug>DLL"
        )

        # 警告级别
        target_compile_options(${PROJECT_NAME} PRIVATE
            /W4
            /WX  # 将警告视为错误
        )

        # 为 DLL 导出所有符号
        if(BUILD_SHARED_LIBS)
            set_target_properties(${PROJECT_NAME} PROPERTIES
                WINDOWS_EXPORT_ALL_SYMBOLS ON
            )
        endif()
    endif()

    # MinGW 特定配置
    if(MINGW)
        target_compile_options(${PROJECT_NAME} PRIVATE
            -Wall -Wextra -Wpedantic
        )

        # MinGW 运行时的静态链接
        target_link_options(${PROJECT_NAME} PRIVATE
            -static-libgcc
            -static-libstdc++
        )
    endif()

    # 链接 Windows 库
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
    # 位置无关代码
    set_target_properties(${PROJECT_NAME} PROPERTIES
        POSITION_INDEPENDENT_CODE ON
    )

    # RPATH 配置
    set_target_properties(${PROJECT_NAME} PROPERTIES
        BUILD_RPATH_USE_ORIGIN ON
        INSTALL_RPATH "$ORIGIN"
    )

    # 链接 Linux 库
    target_link_libraries(${PROJECT_NAME}
        PUBLIC
            pthread
            dl
    )

    # 在发布版本中剥离符号
    if(CMAKE_BUILD_TYPE STREQUAL "Release")
        add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
            COMMAND ${CMAKE_STRIP} $<TARGET_FILE:${PROJECT_NAME}>
        )
    endif()
endif()
```

## 依赖管理

### 查找包

```cmake
# 查找必需的依赖
find_package(OpenSSL 1.1.1 REQUIRED)
find_package(ZLIB REQUIRED)
find_package(Protobuf REQUIRED)

# 链接依赖
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

# 获取 nlohmann/json
FetchContent_Declare(
    nlohmann_json
    GIT_REPOSITORY https://github.com/nlohmann/json.git
    GIT_TAG v3.11.2
)
FetchContent_MakeAvailable(nlohmann_json)

# 链接获取的依赖
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        nlohmann_json::nlohmann_json
)
```

### ExternalProject

```cmake
include(ExternalProject)

# 构建外部项目
ExternalProject_Add(
    boost
    URL https://boostorg.jfrog.io/artifactory/main/release/1.80.0/source/boost_1_80_0.tar.gz
    PREFIX ${CMAKE_BINARY_DIR}/external/boost
    CONFIGURE_COMMAND ./bootstrap.sh
    BUILD_COMMAND ./b2
    INSTALL_COMMAND ""
    BUILD_IN_SOURCE 1
)

# 添加依赖
add_dependencies(${PROJECT_NAME} boost)

# 包含外部项目头文件
target_include_directories(${PROJECT_NAME}
    PRIVATE
        ${CMAKE_BINARY_DIR}/external/boost/src/boost
)
```

### Conan 集成

```cmake
# 包含 Conan CMake 集成
include(${CMAKE_BINARY_DIR}/conanbuildinfo.cmake)
conan_basic_setup(TARGETS)

# 链接 Conan 依赖
target_link_libraries(${PROJECT_NAME}
    PUBLIC
        CONAN_PKG::openssl
        CONAN_PKG::zlib
)
```

## CCGO 辅助函数

### ccgo_detect_platform()

检测目标平台：

```cmake
ccgo_detect_platform()

# 检测后可用的变量：
# - CCGO_PLATFORM: android, ios, macos, windows, linux, ohos
# - CCGO_PLATFORM_ANDROID
# - CCGO_PLATFORM_IOS
# - CCGO_PLATFORM_MACOS
# - CCGO_PLATFORM_WINDOWS
# - CCGO_PLATFORM_LINUX
# - CCGO_PLATFORM_OHOS
```

### ccgo_configure_platform_target()

为检测到的平台配置目标：

```cmake
ccgo_configure_platform_target(${PROJECT_NAME})

# 应用平台特定的：
# - 编译器标志
# - 链接器标志
# - 架构设置
# - 构建类型配置
```

### ccgo_add_library()

使用 CCGO 约定创建库：

```cmake
ccgo_add_library(${PROJECT_NAME}
    SOURCES ${SOURCES}
    PUBLIC_HEADERS ${PUBLIC_HEADERS}
    PRIVATE_HEADERS ${PRIVATE_HEADERS}
    LINK_LIBRARIES ${DEPENDENCIES}
)
```

### ccgo_configure_version()

配置版本信息：

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

## 构建自定义

### 编译器标志

```cmake
# 全局编译器标志
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

# 目标特定标志
target_compile_options(${PROJECT_NAME} PRIVATE
    -fvisibility=hidden
    -ffunction-sections
    -fdata-sections
)
```

### 链接器标志

```cmake
# 删除未使用的段
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang|GNU")
    target_link_options(${PROJECT_NAME} PRIVATE
        -Wl,--gc-sections
    )
endif()

# 链接时优化
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

### 构建类型

```cmake
# 自定义构建类型
set(CMAKE_BUILD_TYPE "RelWithDebInfo" CACHE STRING
    "Build type (Debug, Release, RelWithDebInfo, MinSizeRel)"
)

# 每个配置的设置
set(CMAKE_CXX_FLAGS_DEBUG "-O0 -g3 -DDEBUG")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")
set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "-O2 -g -DNDEBUG")
set(CMAKE_CXX_FLAGS_MINSIZEREL "-Os -DNDEBUG")
```

## IDE 项目生成

### Xcode

```bash
# 生成 Xcode 项目
ccgo build ios --ide-project

# 或手动
cmake -G Xcode \
    -DCMAKE_TOOLCHAIN_FILE=${CCGO_CMAKE_DIR}/ios.toolchain.cmake \
    -DPLATFORM=OS64 \
    ..
```

### Visual Studio

```bash
# 生成 Visual Studio 项目
ccgo build windows --ide-project

# 或手动
cmake -G "Visual Studio 17 2022" \
    -A x64 \
    ..
```

### CodeLite

```bash
# 生成 CodeLite 项目
ccgo build linux --ide-project

# 或手动
cmake -G "CodeLite - Unix Makefiles" ..
```

## 最佳实践

### 1. 现代 CMake

```cmake
# 好：使用基于目标的命令
target_include_directories(${PROJECT_NAME} PUBLIC include/)
target_link_libraries(${PROJECT_NAME} PUBLIC OpenSSL::SSL)

# 坏：使用基于目录的命令
include_directories(include/)
link_libraries(ssl)
```

### 2. 生成器表达式

```cmake
# 平台特定标志
target_compile_options(${PROJECT_NAME} PRIVATE
    $<$<PLATFORM_ID:Windows>:/W4>
    $<$<PLATFORM_ID:Linux>:-Wall>
)

# 构建类型特定定义
target_compile_definitions(${PROJECT_NAME} PRIVATE
    $<$<CONFIG:Debug>:DEBUG_BUILD>
    $<$<CONFIG:Release>:RELEASE_BUILD>
)
```

### 3. 接口库

```cmake
# 为仅头文件库创建接口库
add_library(header_only INTERFACE)
target_include_directories(header_only INTERFACE include/)
target_compile_features(header_only INTERFACE cxx_std_17)

# 使用接口库
target_link_libraries(${PROJECT_NAME} PUBLIC header_only)
```

### 4. 导出配置

```cmake
# 导出目标
install(EXPORT ${PROJECT_NAME}Targets
    FILE ${PROJECT_NAME}Targets.cmake
    NAMESPACE ${PROJECT_NAME}::
    DESTINATION lib/cmake/${PROJECT_NAME}
)

# 生成配置文件
include(CMakePackageConfigHelpers)
configure_package_config_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/Config.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake
    INSTALL_DESTINATION lib/cmake/${PROJECT_NAME}
)

# 安装配置文件
install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/${PROJECT_NAME}Config.cmake
    DESTINATION lib/cmake/${PROJECT_NAME}
)
```

## 故障排除

### CMake 缓存问题

```bash
# 清除 CMake 缓存
rm -rf cmake_build/
ccgo build android  # 重新生成

# 或手动
rm CMakeCache.txt
cmake ..
```

### 找不到工具链

```bash
# 验证 CCGO_CMAKE_DIR 已设置
echo $CCGO_CMAKE_DIR

# 手动设置工具链
cmake -DCMAKE_TOOLCHAIN_FILE=/path/to/toolchain.cmake ..
```

### 缺少依赖

```cmake
# 添加依赖搜索路径
list(APPEND CMAKE_PREFIX_PATH
    /usr/local
    /opt/homebrew
    ${CMAKE_SOURCE_DIR}/third_party
)
```

## 资源

### CMake 文档

- [CMake 官方文档](https://cmake.org/documentation/)
- [Modern CMake](https://cliutils.gitlab.io/modern-cmake/)
- [Effective CMake](https://www.youtube.com/watch?v=bsXLMQ6WgIk)

### CCGO 文档

- [CLI 参考](cli.zh.md)
- [CCGO.toml 参考](ccgo-toml.zh.md)
- [构建系统](../features/build-system.zh.md)
- [平台指南](../platforms/index.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [CCGO.toml 参考](ccgo-toml.zh.md)
- [构建系统概览](../features/build-system.zh.md)
- [依赖管理](../features/dependency-management.zh.md)
- [平台特定指南](../platforms/index.zh.md)
