# 配置指南

通过 CCGO.toml 和其他配置文件配置 CCGO 项目的完整指南。

## 概述

CCGO 使用基于文件的配置系统：

- **CCGO.toml**：主项目配置
- **build_config.py**：特定于构建的设置（自动生成）
- **CMakeLists.txt**：CMake 集成
- **.ccgoignore**：要从操作中排除的文件
- **环境变量**：运行时配置

## CCGO.toml

### 文件位置

```
myproject/
├── CCGO.toml          # 主配置
├── src/
├── include/
└── tests/
```

### 基本结构

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My cross-platform C++ library"
authors = ["Your Name <you@example.com>"]
license = "MIT"
homepage = "https://github.com/myuser/mylib"
repository = "https://github.com/myuser/mylib"

[library]
type = "both"                    # static、shared 或 both

[build]
cpp_standard = "17"              # C++ 标准版本

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[android]
min_sdk_version = 21

[ios]
deployment_target = "12.0"
```

## 包配置

### 必需字段

```toml
[package]
name = "mylib"                   # 必需：项目名称（小写，无空格）
version = "1.0.0"                # 必需：语义化版本
```

### 可选元数据

```toml
[package]
description = "A powerful C++ library for..."
authors = [
    "John Doe <john@example.com>",
    "Jane Smith <jane@example.com>"
]
license = "MIT"                  # 许可证标识符
license_file = "LICENSE"         # 许可证文件路径
readme = "README.md"             # README 路径
homepage = "https://mylib.dev"
repository = "https://github.com/user/mylib"
documentation = "https://docs.mylib.dev"
keywords = ["networking", "async", "performance"]
categories = ["network", "concurrency"]
```

### 版本字段

版本必须遵循[语义化版本](https://semver.org/)：

```toml
[package]
version = "1.0.0"                # 主版本.次版本.修订版本
# version = "1.0.0-alpha"        # 预发布版本
# version = "1.0.0-beta.1"       # 带编号的预发布版本
# version = "1.0.0+build.123"    # 构建元数据
```

**规则：**
- MAJOR（主版本）：破坏性变更
- MINOR（次版本）：新功能（向后兼容）
- PATCH（修订版本）：错误修复
- 预发布：可选的 `-alpha`、`-beta`、`-rc.N`
- 构建元数据：可选的 `+build.N`

## 库配置

### 库类型

```toml
[library]
type = "static"                  # 仅静态库
# type = "shared"                # 仅动态库
# type = "both"                  # 静态和动态都构建（默认）
```

**静态库：**
- 编译到可执行文件中
- 可执行文件体积更大
- 启动更快
- 无运行时依赖

**动态库：**
- 运行时加载
- 可执行文件更小
- 可独立更新
- 需要库在运行时存在

**Both（两者）：**
- 构建两种类型
- 用户在链接时选择

### 库命名

```toml
[library]
name = "mylib"                   # 覆盖库名称（可选）
# 默认：使用 package.name
```

**生成的文件：**
- 静态：`libmylib.a`（Unix）/ `mylib.lib`（Windows MSVC）
- 动态：`libmylib.so`（Linux）/ `libmylib.dylib`（macOS）/ `mylib.dll`（Windows）

## 构建配置

### C++ 标准

```toml
[build]
cpp_standard = "17"              # C++17（推荐）
# cpp_standard = "11"            # C++11
# cpp_standard = "14"            # C++14
# cpp_standard = "20"            # C++20
# cpp_standard = "23"            # C++23
```

**各平台支持：**
- C++11：所有平台
- C++14：所有平台
- C++17：所有平台（推荐）
- C++20：仅现代编译器
- C++23：仅前沿编译器

### 构建类型

```toml
[build]
default_build_type = "release"   # 默认：release
# default_build_type = "debug"   # 用于开发
```

**Debug（调试）：**
- 无优化
- 调试符号
- 启用断言
- 二进制体积更大

**Release（发布）：**
- 完全优化
- 剥离符号（单独文件）
- 禁用断言
- 二进制体积更小

### 编译器标志

```toml
[build]
cflags = ["-Wall", "-Wextra"]                    # C 标志
cxxflags = ["-Wall", "-Wextra", "-pedantic"]     # C++ 标志
ldflags = ["-Wl,-rpath,$ORIGIN"]                 # 链接器标志
```

**常用标志：**

```toml
[build]
# 警告
cxxflags = [
    "-Wall",                     # 所有警告
    "-Wextra",                   # 额外警告
    "-Werror",                   # 将警告视为错误
    "-pedantic"                  # 严格 ISO C++
]

# 优化
cxxflags = [
    "-O3",                       # 最大优化
    "-march=native",             # CPU 特定优化
    "-flto"                      # 链接时优化
]

# 安全
cxxflags = [
    "-fstack-protector-strong",  # 栈保护
    "-D_FORTIFY_SOURCE=2",       # 缓冲区溢出检测
    "-fPIC"                      # 位置无关代码
]
```

### 宏定义

```toml
[build]
defines = [
    "USE_FEATURE_X",             # 简单定义
    "MAX_CONNECTIONS=100",       # 带值的定义
    "DEBUG_LOGGING"              # 仅调试定义
]
```

**平台特定定义：**

```toml
[build.android]
defines = ["ANDROID_PLATFORM"]

[build.ios]
defines = ["IOS_PLATFORM"]

[build.windows]
defines = ["WINDOWS_PLATFORM", "_WIN32_WINNT=0x0601"]
```

### 包含目录

```toml
[build]
include_dirs = [
    "include",                   # 公共头文件
    "src/internal",              # 私有头文件
    "third_party/lib/include"    # 第三方包含
]
```

### 源文件

默认情况下，CCGO 编译 `src/` 中的所有 `.cpp/.cc/.cxx` 文件。覆盖：

```toml
[build]
sources = [
    "src/**/*.cpp",              # src/ 中的所有 .cpp
    "src/core/*.cc",             # 特定目录
    "src/platform/linux/*.cpp"   # 平台特定
]

exclude = [
    "src/experimental/**",       # 排除目录
    "src/**/*_test.cpp"          # 排除测试文件
]
```

## 依赖配置

### Git 依赖

```toml
[dependencies]
# 标签（推荐用于稳定性）
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 分支（用于最新功能）
fmt = { git = "https://github.com/fmtlib/fmt.git", branch = "master" }

# 提交哈希（用于精确可重现性）
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280a" }
```

### 路径依赖

```toml
[dependencies]
# 相对路径
myutils = { path = "../myutils" }

# 绝对路径
common = { path = "/opt/libs/common" }

# 工作区依赖
core = { path = "./libs/core" }
```

### 可选依赖

```toml
[dependencies]
# 必需的依赖项
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 可选依赖项
[dependencies.optional]
networking = { git = "https://github.com/user/networking.git", tag = "v1.0.0" }
database = { git = "https://github.com/user/database.git", tag = "v2.0.0" }
```

通过特性启用：

```toml
[features]
default = ["basic"]
basic = []
network = ["networking"]        # 启用 networking 依赖项
db = ["database"]               # 启用 database 依赖项
full = ["basic", "network", "db"]
```

使用特性构建：

```bash
ccgo build android --features network,db
```

### 平台特定依赖

```toml
[dependencies]
common = { git = "https://github.com/user/common.git", tag = "v1.0.0" }

# 仅 Android
[target.'cfg(target_os = "android")'.dependencies]
android-log = { git = "https://github.com/user/android-log.git", tag = "v1.0.0" }

# 仅 iOS
[target.'cfg(target_os = "ios")'.dependencies]
ios-utils = { path = "./ios-utils" }

# 仅 Windows
[target.'cfg(target_os = "windows")'.dependencies]
win32-api = { git = "https://github.com/user/win32-api.git", tag = "v1.0.0" }
```

## 平台特定配置

### Android

```toml
[android]
min_sdk_version = 21             # 最低 API 级别
target_sdk_version = 34          # 目标 API 级别
ndk_version = "26.1.10909125"    # NDK 版本（可选）
stl = "c++_shared"               # STL 类型：c++_static、c++_shared
package_name = "com.example.mylib"  # Java 包名
```

### iOS

```toml
[ios]
deployment_target = "12.0"       # 最低 iOS 版本
enable_bitcode = false           # Bitcode 支持（已弃用）
enable_arc = true                # 自动引用计数
frameworks = [                   # 系统框架
    "Foundation",
    "UIKit",
    "CoreGraphics"
]
```

### macOS

```toml
[macos]
deployment_target = "10.15"      # 最低 macOS 版本
enable_hardened_runtime = true   # 加固运行时
frameworks = [                   # 系统框架
    "Foundation",
    "AppKit"
]
```

### Windows

```toml
[windows]
subsystem = "console"            # 子系统：console、windows
runtime_library = "MD"           # 运行时：MT、MD、MTd、MDd
windows_sdk_version = "10.0"     # Windows SDK 版本
```

### Linux

```toml
[linux]
min_glibc_version = "2.17"       # 最低 glibc 版本
link_pthread = true              # 链接 pthread
link_dl = true                   # 链接 libdl
link_rt = true                   # 链接 librt
```

### OpenHarmony

```toml
[ohos]
api_version = 9                  # API 版本
package_name = "com.example.mylib"  # 包名
```

## 特性配置

### 定义特性

```toml
[features]
# 默认特性（自动启用）
default = ["std"]

# 无依赖的特性
std = []

# 启用依赖项的特性
network = ["cpp-httplib", "openssl"]

# 启用其他特性的特性
full = ["std", "network", "database"]

# 特性组合
web = ["network", "json"]
```

### 使用特性

**在代码中：**

```cpp
#ifdef CCGO_FEATURE_NETWORK
    // 网络代码
    #include <httplib.h>
#endif

#ifdef CCGO_FEATURE_DATABASE
    // 数据库代码
    #include <sqlite3.h>
#endif
```

**在构建时：**

```bash
# 使用特定特性构建
ccgo build android --features network

# 使用多个特性构建
ccgo build android --features network,database

# 使用所有特性构建
ccgo build android --all-features

# 不使用默认特性构建
ccgo build android --no-default-features

# 组合标志
ccgo build android --no-default-features --features network
```

## 测试配置

### 测试设置

```toml
[test]
# 测试框架（默认：catch2）
framework = "catch2"             # catch2、gtest 或自定义

# 测试源
sources = [
    "tests/**/*.cpp"
]

# 测试依赖项
[test.dependencies]
catch2 = { git = "https://github.com/catchorg/Catch2.git", tag = "v3.4.0" }
```

### 运行测试

```bash
# 运行所有测试
ccgo test

# 运行特定测试
ccgo test --filter "MyTest"

# 使用详细输出运行
ccgo test --verbose
```

## 基准测试配置

### 基准测试设置

```toml
[bench]
# 基准测试框架
framework = "google-benchmark"   # google-benchmark 或自定义

# 基准测试源
sources = [
    "benches/**/*.cpp"
]

# 基准测试依赖项
[bench.dependencies]
benchmark = { git = "https://github.com/google/benchmark.git", tag = "v1.8.3" }
```

### 运行基准测试

```bash
# 运行所有基准测试
ccgo bench

# 运行特定基准测试
ccgo bench --filter "MyBenchmark"

# 指定迭代次数
ccgo bench --iterations 1000
```

## 文档配置

### 文档设置

```toml
[doc]
# 文档生成器
generator = "doxygen"            # doxygen 或自定义

# 源目录
source_dirs = [
    "include",
    "src",
    "docs"
]

# 输出目录
output_dir = "target/doc"
```

### 生成文档

```bash
# 生成文档
ccgo doc

# 生成并在浏览器中打开
ccgo doc --open
```

## 发布配置

### 包元数据

```toml
[package]
name = "mylib"
version = "1.0.0"
authors = ["Your Name <you@example.com>"]
license = "MIT"
description = "My cross-platform library"
homepage = "https://github.com/user/mylib"
repository = "https://github.com/user/mylib"
documentation = "https://docs.mylib.dev"
```

### 发布设置

```toml
[publish]
# 仓库
registry = "default"             # 仓库名称

# Maven（Android）
[publish.maven]
group_id = "com.example"
artifact_id = "mylib"

# CocoaPods（Apple）
[publish.cocoapods]
pod_name = "MyLib"
swift_version = "5.0"

# OHPM（OpenHarmony）
[publish.ohpm]
package_name = "@example/mylib"
```

## build_config.py

生成的包含构建特定配置的文件：

```python
# build_config.py（自动生成）

# 项目信息
PROJECT_NAME = "mylib"
PROJECT_VERSION = "1.0.0"

# 构建设置
BUILD_TYPE = "release"
CPP_STANDARD = "17"
LIBRARY_TYPE = "both"

# 平台
ANDROID_MIN_SDK = 21
IOS_DEPLOYMENT_TARGET = "12.0"

# 自定义设置
CUSTOM_DEFINES = []
CUSTOM_FLAGS = []
```

**在构建脚本中使用：**

```python
from build_config import PROJECT_NAME, PROJECT_VERSION

print(f"Building {PROJECT_NAME} v{PROJECT_VERSION}")
```

## .ccgoignore

从 CCGO 操作中排除文件：

```
# .ccgoignore

# 构建目录
cmake_build/
target/
bin/

# IDE 文件
.vscode/
.idea/
*.swp

# 操作系统文件
.DS_Store
Thumbs.db

# 依赖项
vendor/
node_modules/

# 生成的文件
*.pyc
__pycache__/
```

**语法：**
- `#` 表示注释
- `*` 通配符匹配文件
- `**` 通配符匹配目录
- `/` 从根目录匹配
- `!` 取反（包含）

**示例：**

```
# 忽略所有 .log 文件
*.log

# 但包含 important.log
!important.log

# 仅忽略根目录中的 build/
/build/

# 忽略所有 build/ 目录
**/build/
```

## 环境变量

### 构建时变量

```bash
# 构建类型
export BUILD_TYPE=debug          # debug 或 release

# 详细程度
export CCGO_VERBOSE=1            # 详细输出

# 并行构建
export CMAKE_BUILD_PARALLEL_LEVEL=8  # 使用 8 个核心

# 架构
export ANDROID_ARCH=arm64-v8a    # Android 架构
```

### 平台特定变量

**Android：**

```bash
export ANDROID_HOME=/path/to/android-sdk
export ANDROID_NDK_HOME=/path/to/ndk
export ANDROID_MIN_SDK=21
```

**iOS：**

```bash
export CODE_SIGN_IDENTITY="Apple Development"
export DEVELOPMENT_TEAM="TEAM123456"
export IOS_DEPLOYMENT_TARGET="12.0"
```

**Windows：**

```bash
export MSVC_VERSION=2022         # Visual Studio 版本
export WINDOWS_SDK_VERSION=10.0.22621.0
```

### Docker 变量

```bash
# Docker 构建
export USE_DOCKER=1              # 启用 Docker 构建

# Docker 镜像
export DOCKER_IMAGE=ccgo-builder-linux:latest
```

## 配置验证

### 检查配置

```bash
# 验证 CCGO.toml
ccgo check

# 验证特定平台
ccgo check android

# 详细验证
ccgo check --verbose
```

### 常见问题

**无效的版本格式：**

```
Error: Invalid version '1.0' in CCGO.toml
```

**解决方案：** 使用语义化版本（例如 `1.0.0`）

**缺少必需字段：**

```
Error: Missing required field 'name' in [package]
```

**解决方案：** 将必需字段添加到 CCGO.toml

**无效的依赖项：**

```
Error: Invalid dependency format for 'spdlog'
```

**解决方案：** 检查依赖项语法

## 配置模板

### 最小配置

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "static"
```

### 标准配置

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My library"
authors = ["Your Name <you@example.com>"]
license = "MIT"

[library]
type = "both"

[build]
cpp_standard = "17"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[android]
min_sdk_version = 21

[ios]
deployment_target = "12.0"
```

### 高级配置

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "Advanced C++ library"
authors = ["Team <team@example.com>"]
license = "MIT"
homepage = "https://mylib.dev"
repository = "https://github.com/user/mylib"

[library]
type = "both"

[build]
cpp_standard = "20"
cxxflags = ["-Wall", "-Wextra", "-Werror"]
defines = ["USE_ADVANCED_FEATURES"]

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[dependencies.optional]
networking = { git = "https://github.com/user/networking.git", tag = "v1.0.0" }

[features]
default = ["basic"]
basic = []
network = ["networking"]
full = ["basic", "network"]

[android]
min_sdk_version = 21
target_sdk_version = 34
package_name = "com.example.mylib"

[ios]
deployment_target = "12.0"
frameworks = ["Foundation", "UIKit"]

[test]
framework = "catch2"

[test.dependencies]
catch2 = { git = "https://github.com/catchorg/Catch2.git", tag = "v3.4.0" }
```

## 最佳实践

### 1. 使用语义化版本

```toml
[package]
version = "1.0.0"                # 好：主版本.次版本.修订版本
# version = "1.0"                # 差：缺少修订版本
# version = "v1.0.0"             # 差：不要包含 'v' 前缀
```

### 2. 固定依赖项

```toml
[dependencies]
# 好：特定标签
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 差：跟踪分支
# spdlog = { git = "https://github.com/gabime/spdlog.git", branch = "master" }
```

### 3. 组织章节

```toml
# 好：逻辑顺序
[package]
[library]
[build]
[dependencies]
[android]
[ios]

# 差：随机顺序
[ios]
[package]
[dependencies]
[build]
```

### 4. 为配置添加注释

```toml
[build]
# 启用所有警告
cxxflags = ["-Wall", "-Wextra"]

# 平台特定优化
defines = [
    "USE_SSE=1",                 # 启用 SSE 指令
    "MAX_THREADS=8"              # 限制线程池大小
]
```

### 5. 保持简单

只配置您需要的：

```toml
# 好：仅必要的配置
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "static"

# 差：不必要的配置
# [build]
# cpp_standard = "17"            # 默认为 17
# [library]
# name = "mylib"                 # 与 package.name 相同
```

## 迁移指南

### 从 CMakeLists.txt

**CMakeLists.txt：**

```cmake
project(mylib VERSION 1.0.0)
set(CMAKE_CXX_STANDARD 17)
add_library(mylib src/mylib.cpp)
```

**CCGO.toml：**

```toml
[package]
name = "mylib"
version = "1.0.0"

[build]
cpp_standard = "17"
```

### 从 Conan

**conanfile.txt：**

```ini
[requires]
spdlog/1.12.0
fmt/10.1.1

[options]
spdlog:shared=False
```

**CCGO.toml：**

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[library]
type = "static"
```

## 另请参阅

- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [项目结构](project-structure.md)
- [依赖管理](../features/dependency-management.md)
- [构建系统](../features/build-system.md)
