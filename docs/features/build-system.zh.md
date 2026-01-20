# 构建系统

CCGO 跨平台构建系统的综合指南。

## 概述

CCGO 提供统一的构建系统：

- **多平台支持**：为 Android、iOS、macOS、Windows、Linux、OpenHarmony、watchOS、tvOS 构建
- **架构灵活性**：每个平台支持单个或多个架构
- **构建类型控制**：Debug 和 Release 构建
- **链接类型选项**：静态、动态或两种库类型
- **工具链选择**：平台特定的工具链选择（例如 MSVC vs MinGW）
- **Docker 集成**：无需本地工具链的通用交叉编译
- **增量构建**：使用 CMake 缓存快速重建
- **统一输出格式**：跨平台一致的 ZIP 存档结构

## 构建架构

### 高层流程

```
用户命令 (ccgo build)
    ↓
CLI 解析器 (cli.py)
    ↓
构建命令 (commands/build.py)
    ↓
平台构建脚本 (build_scripts/build_<platform>.py)
    ↓
CMake 配置 (build_scripts/cmake/)
    ↓
原生工具链 (NDK/Xcode/MSVC/GCC/等)
    ↓
归档和打包 (带元数据的 ZIP)
```

### 关键组件

**1. CLI 层 (`ccgo/cli.py`、`ccgo/commands/build.py`)**
- 解析用户命令和选项
- 验证平台和架构组合
- 分发到平台特定的构建脚本

**2. 构建脚本 (`ccgo/build_scripts/build_*.py`)**
- 平台特定的构建逻辑
- 使用正确的工具链文件调用 CMake
- 产物收集和打包
- 集中在 ccgo 包中（不复制到项目）

**3. CMake 配置 (`ccgo/build_scripts/cmake/`)**
- CMake 实用函数和模板
- 平台特定的工具链文件
- 构建类型配置（debug/release）
- 依赖解析

**4. 构建配置（项目中的 `build_config.py`）**
- 项目特定的构建设置
- 在 `ccgo new` 期间从模板生成
- 由构建脚本加载

## 平台抽象

### 通用构建接口

所有平台构建脚本实现通用接口：

```python
# build_scripts/build_<platform>.py

def configure_cmake(project_dir, build_dir, config):
    """使用平台特定设置配置 CMake"""
    pass

def build_libraries(build_dir, config):
    """构建静态和/或动态库"""
    pass

def collect_artifacts(build_dir, output_dir, config):
    """收集构建产物"""
    pass

def package_artifacts(output_dir, config):
    """将产物打包到 ZIP 存档"""
    pass
```

### 平台特定构建脚本

| 平台 | 脚本 | 工具链 | 输出格式 |
|------|------|--------|----------|
| Android | `build_android.py` | NDK | .so、.a、AAR |
| iOS | `build_ios.py` | Xcode | Framework、XCFramework |
| macOS | `build_macos.py` | Xcode | Framework、XCFramework、dylib |
| Windows | `build_windows.py` | MSVC/MinGW | .dll、.lib/.a |
| Linux | `build_linux.py` | GCC/Clang | .so、.a |
| OpenHarmony | `build_ohos.py` | OHOS SDK | .so、.a、HAR |
| watchOS | `build_watchos.py` | Xcode | Framework、XCFramework |
| tvOS | `build_tvos.py` | Xcode | Framework、XCFramework |

## CMake 集成

### CMake 目录结构

```
ccgo/build_scripts/cmake/
├── CMakeUtils.cmake          # 实用函数
├── CMakeFunctions.cmake      # 构建辅助函数
├── FindCCGODependencies.cmake # 依赖解析
├── ios.toolchain.cmake       # iOS 交叉编译
├── tvos.toolchain.cmake      # tvOS 交叉编译
├── watchos.toolchain.cmake   # watchOS 交叉编译
├── windows-msvc.toolchain.cmake  # Windows MSVC
└── template/                 # CMakeLists.txt 模板
    ├── Root.CMakeLists.txt.in
    ├── Src.CMakeLists.txt.in
    ├── Tests.CMakeLists.txt.in
    └── ...
```

### CMake 配置变量

CCGO 将这些变量传递给 CMake：

```cmake
# 平台信息
${CCGO_CMAKE_DIR}          # CCGO cmake 工具路径
${PLATFORM}                # 目标平台（android、ios 等）
${ARCHITECTURE}            # 目标架构（arm64-v8a、x86_64 等）

# 构建配置
${BUILD_TYPE}              # Debug 或 Release
${LINK_TYPE}               # static、shared 或 both
${CPP_STANDARD}            # C++ 标准（11、14、17、20、23）

# 项目信息
${PROJECT_NAME}            # 来自 CCGO.toml
${PROJECT_VERSION}         # 来自 CCGO.toml
${PROJECT_NAMESPACE}       # C++ 命名空间

# 平台特定（Android）
${ANDROID_ABI}             # Android 架构
${ANDROID_PLATFORM}        # Android API 级别
${ANDROID_NDK}             # NDK 路径
${ANDROID_STL}             # STL 类型

# 平台特定（Apple）
${CMAKE_OSX_DEPLOYMENT_TARGET}      # 最低 OS 版本
${CMAKE_OSX_ARCHITECTURES}          # 架构列表
```

### 项目中的 CMake 使用

**项目 CMakeLists.txt：**

```cmake
cmake_minimum_required(VERSION 3.18)
project(mylib VERSION 1.0.0)

# 包含 CCGO 工具
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)

# 使用 CCGO 函数
ccgo_setup_project()

# 定义库
ccgo_add_library(${PROJECT_NAME}
    SOURCES
        src/mylib.cpp
        src/utils.cpp
    HEADERS
        include/mylib/mylib.h
        include/mylib/utils.h
    PUBLIC_HEADERS
        include/mylib/mylib.h
)

# 链接依赖
ccgo_link_dependencies(${PROJECT_NAME}
    PUBLIC spdlog fmt
)
```

## 构建配置

### CCGO.toml 构建部分

```toml
[build]
cpp_standard = "17"               # C++ 标准
cmake_minimum_version = "3.18"    # 最低 CMake 版本
compile_flags = ["-Wall", "-Wextra"]  # 额外的编译器标志
link_flags = ["-flto"]            # 额外的链接器标志

[build.definitions]
DEBUG_MODE = "1"                  # 预处理器定义
APP_VERSION = "\"1.0.0\""

[build]
include_dirs = ["third_party/include"]  # 额外的包含目录
link_dirs = ["third_party/lib"]         # 额外的库目录
system_libs = ["pthread", "dl"]         # 要链接的系统库
```

### build_config.py

在项目根目录生成，包含运行时构建配置：

```python
# build_config.py

PROJECT_NAME = "mylib"
PROJECT_VERSION = "1.0.0"
CPP_STANDARD = "17"

# 平台特定配置
ANDROID_CONFIG = {
    "min_sdk_version": 21,
    "target_sdk_version": 33,
    "ndk_version": "25.2.9519653",
    "stl": "c++_static",
    "architectures": ["arm64-v8a", "armeabi-v7a", "x86_64"]
}

IOS_CONFIG = {
    "min_deployment_target": "12.0",
    "enable_bitcode": False,
    "architectures": ["arm64"]
}

# ... 更多平台配置
```

## 构建过程

### 逐步构建流程

**1. 解析命令**
```bash
ccgo build android --arch arm64-v8a --release
```
- 平台：android
- 架构：arm64-v8a
- 构建类型：release

**2. 加载配置**
- 读取 CCGO.toml
- 加载 build_config.py
- 验证平台/架构组合

**3. 配置 CMake**
```bash
cmake -S <source_dir> -B <build_dir> \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_TOOLCHAIN_FILE=<ndk>/build/cmake/android.toolchain.cmake \
    -DANDROID_ABI=arm64-v8a \
    -DANDROID_PLATFORM=android-21 \
    -DCCGO_CMAKE_DIR=<ccgo>/build_scripts/cmake
```

**4. 构建库**
```bash
cmake --build <build_dir> --config Release --target all
```

**5. 收集产物**
- 复制库（.so、.a、.dll 等）
- 复制头文件
- 复制平台特定包（AAR、Framework 等）
- 生成构建元数据（build_info.json）

**6. 打包**
- 创建统一的 ZIP 存档结构
- 如果是 debug 构建则创建符号 ZIP
- 计算校验和

### 构建目录

```
project/
├── cmake_build/              # CMake 构建目录
│   ├── android/
│   │   ├── arm64-v8a/       # 每个架构的构建
│   │   │   ├── debug/
│   │   │   └── release/
│   │   └── armeabi-v7a/
│   ├── ios/
│   │   └── ...
│   └── ...
└── target/                   # 最终构建输出
    ├── android/
    │   ├── MYLIB_ANDROID_SDK-1.0.0.zip
    │   ├── MYLIB_ANDROID_SDK-1.0.0-SYMBOLS.zip
    │   └── build_info.json
    ├── ios/
    └── ...
```

## 输出产物

### 统一存档结构

所有平台使用一致的结构：

```
{PROJECT}_{PLATFORM}_SDK-{version}.zip
├── lib/
│   ├── static/              # 静态库
│   │   └── {arch}/          # 每个架构（移动平台）
│   │       └── lib{name}.a
│   └── shared/              # 动态库
│       └── {arch}/
│           └── lib{name}.so
├── frameworks/              # 仅 Apple 平台
│   ├── static/
│   │   └── {Name}.xcframework
│   └── shared/
│       └── {Name}.xcframework
├── haars/                   # 仅 Android/OHOS
│   └── {name}-release.aar
├── include/                 # 公共头文件
│   └── {project}/
│       ├── {header}.h
│       └── version.h
└── build_info.json          # 构建元数据
```

### 构建元数据（build_info.json）

```json
{
  "project": "mylib",
  "version": "1.0.0",
  "platform": "android",
  "architectures": ["arm64-v8a", "armeabi-v7a"],
  "build_type": "release",
  "link_types": ["static", "shared"],
  "timestamp": "2025-01-19T10:30:00Z",
  "git": {
    "commit": "a1b2c3d",
    "branch": "main",
    "tag": "v1.0.0"
  },
  "toolchain": {
    "name": "Android NDK",
    "version": "25.2.9519653",
    "compiler": "clang 14.0.7"
  },
  "dependencies": {
    "spdlog": "1.12.0",
    "fmt": "10.1.1"
  },
  "checksums": {
    "sha256": "..."
  }
}
```

## 构建类型

### Debug 构建

**特点：**
- 包含调试符号
- 无优化（-O0）
- 启用断言
- 更大的二进制大小
- 更容易调试

**使用：**
```bash
ccgo build <platform> --debug
```

**CMake 标志：**
```cmake
-DCMAKE_BUILD_TYPE=Debug
-DCMAKE_CXX_FLAGS_DEBUG="-g -O0"
```

### Release 构建

**特点：**
- 符号剥离（单独的 SYMBOLS.zip）
- 完全优化（-O3 或等效）
- 禁用断言
- 更小的二进制大小
- 更好的性能

**使用：**
```bash
ccgo build <platform> --release
```

**CMake 标志：**
```cmake
-DCMAKE_BUILD_TYPE=Release
-DCMAKE_CXX_FLAGS_RELEASE="-O3 -DNDEBUG"
```

## 链接类型

### 静态库

**特点：**
- 代码嵌入最终二进制文件
- 无运行时依赖
- 更大的二进制大小
- 单文件部署

**使用：**
```bash
ccgo build <platform> --link-type static
```

**输出：** `.a`（Unix）、`.lib`（Windows）

### 动态库

**特点：**
- 代码在单独的库文件中
- 需要运行时依赖
- 更小的二进制大小
- 应用间代码共享

**使用：**
```bash
ccgo build <platform> --link-type shared
```

**输出：** `.so`（Unix/Android）、`.dylib`（macOS）、`.dll`（Windows）

### 两者都有（默认）

构建静态和动态库：

```bash
ccgo build <platform> --link-type both
```

## 工具链选择

### Windows：MSVC vs MinGW

**MSVC（Microsoft Visual C++）：**
```bash
ccgo build windows --toolchain msvc
```
- 原生 Windows 工具链
- 最佳 Visual Studio 集成
- 与 Windows SDK ABI 兼容

**MinGW（Minimalist GNU for Windows）：**
```bash
ccgo build windows --toolchain mingw
```
- 基于 GCC 的工具链
- 更好的交叉编译支持
- 兼容 Docker 构建

**Auto（两者）：**
```bash
ccgo build windows --toolchain auto  # 默认
```

### 平台工具链

| 平台 | 默认工具链 | 备选方案 |
|------|-----------|---------|
| Android | NDK（Clang）| - |
| iOS | Xcode（Clang）| - |
| macOS | Xcode（Clang）| - |
| Windows | MSVC | MinGW |
| Linux | GCC | Clang |
| OpenHarmony | OHOS SDK | - |

## Docker 构建

### 概述

Docker 构建实现通用交叉编译：
- **零本地设置**：无需安装 SDK/NDK/工具链
- **一致的环境**：所有机器上相同的构建环境
- **隔离构建**：与本地安装无冲突
- **预构建镜像**：快速启动（从 Docker Hub 拉取镜像）

### 使用

```bash
# 使用 Docker 构建任何平台
ccgo build android --docker
ccgo build ios --docker
ccgo build windows --docker
ccgo build linux --docker
```

### Docker 镜像

| 平台 | 镜像名称 | 大小 | 包含 |
|------|---------|------|------|
| Android | `ccgo-builder-android` | ~3.5GB | SDK、NDK、CMake |
| iOS/macOS/watchOS/tvOS | `ccgo-builder-apple` | ~2.5GB | OSXCross、SDK |
| Windows | `ccgo-builder-windows` | ~1.2GB | MinGW-w64、CMake |
| Linux | `ccgo-builder-linux` | ~800MB | GCC、Clang、CMake |

### Docker 构建流程

```
ccgo build <platform> --docker
    ↓
检查 Docker 是否运行
    ↓
拉取/使用缓存的 Docker 镜像
    ↓
将项目目录挂载为卷
    ↓
在容器内运行构建
    ↓
将产物写入主机文件系统
```

## 增量构建

### CMake 缓存

CCGO 使用 CMake 的内置缓存：
- CMake 缓存存储在 `cmake_build/<platform>/<arch>/<build_type>/`
- 仅重新编译更改的源文件
- 自动检测头文件更改

**强制完全重建：**
```bash
ccgo build <platform> --clean
```

### 依赖缓存

- 依赖构建一次，为增量构建缓存
- 缓存失效时：
  - CCGO.toml 中的依赖版本更改
  - CCGO.lock 已更新
  - CMake 配置更改

**清除依赖缓存：**
```bash
rm -rf cmake_build/
ccgo build <platform>
```

### 构建性能

**首次构建：** 10-30 分钟（编译所有依赖）
**增量构建：** 10-60 秒（仅更改的文件）

**优化提示：**
1. 开发期间使用 `--arch` 限制架构
2. 使用 `--link-type` 仅构建所需的库类型
3. 为编译器缓存启用 `ccache`（未来功能）
4. 使用预构建依赖（未来功能）

## IDE 项目生成

### 生成 IDE 项目

```bash
# Android Studio 项目
ccgo build android --ide-project

# Xcode 项目
ccgo build ios --ide-project

# Visual Studio 项目
ccgo build windows --ide-project --toolchain msvc

# CodeLite 项目（Linux）
ccgo build linux --ide-project
```

### IDE 集成

**Android Studio：**
- 生成 `.iml` 文件
- Gradle 同步支持
- 使用 LLDB 的原生调试

**Xcode：**
- 生成 `.xcodeproj`
- 集成调试
- 代码签名支持

**Visual Studio：**
- 生成 `.sln` 和 `.vcxproj`
- IntelliSense 支持
- MSVC 调试器集成

## 自定义构建步骤

### 预构建钩子

**添加自定义预构建脚本：**

```python
# build_config.py

def pre_build_hook(platform, arch, build_type):
    """在构建开始前调用"""
    print(f"Pre-build: {platform} {arch} {build_type}")
    # 自定义逻辑
```

### 后构建钩子

```python
# build_config.py

def post_build_hook(platform, arch, build_type, output_dir):
    """在构建完成后调用"""
    print(f"Post-build: artifacts in {output_dir}")
    # 自定义产物处理
```

### 自定义 CMake

**扩展 CMakeLists.txt：**

```cmake
# CMakeLists.txt

# 自定义源生成
add_custom_command(
    OUTPUT ${CMAKE_CURRENT_BINARY_DIR}/generated.cpp
    COMMAND python3 ${CMAKE_CURRENT_SOURCE_DIR}/codegen.py
    DEPENDS codegen.py
)

# 添加生成的源
target_sources(${PROJECT_NAME} PRIVATE
    ${CMAKE_CURRENT_BINARY_DIR}/generated.cpp
)
```

## 故障排除

### 常见构建问题

#### CMake 配置失败

```
Error: CMake configuration failed
```

**解决方案：**
1. 检查 CMake 版本：`cmake --version`（需要 3.18+）
2. 验证工具链安装
3. 使用详细模式运行：`ccgo build <platform> --verbose`
4. 检查 `cmake_build/<platform>/CMakeError.log`

#### 未找到编译器

```
Error: Could not find compiler
```

**解决方案：**
1. 安装所需工具链
2. 设置环境变量（ANDROID_NDK 等）
3. 使用 Docker 构建：`ccgo build <platform> --docker`

#### 链接错误

```
Error: undefined reference to 'symbol'
```

**解决方案：**
1. 检查所有源文件是否在 CMakeLists.txt 中
2. 验证依赖版本匹配
3. 检查 C++ 标准一致性
4. 启用详细链接：将 `-Wl,--verbose` 添加到 link_flags

#### 内存不足

```
Error: c++: fatal error: Killed signal terminated program cc1plus
```

**解决方案：**
1. 构建更少的架构：`--arch arm64-v8a`
2. 构建单一链接类型：`--link-type static`
3. 增加 Docker 内存：Docker Desktop → 偏好设置 → 资源
4. 在 Linux 上使用交换空间

### 性能问题

#### 构建缓慢

**诊断：**
```bash
ccgo build <platform> --verbose  # 查看时间信息
```

**优化：**
1. 开发期间限制架构
2. 使用增量构建（除非必要否则不要 `--clean`）
3. 启用并行构建（CMake 自动）
4. 为构建目录使用 SSD

#### 磁盘空间问题

**检查大小：**
```bash
du -sh cmake_build/
du -sh target/
```

**清理：**
```bash
ccgo clean          # 删除所有构建产物
ccgo clean --yes    # 跳过确认
```

## 最佳实践

### 1. 版本控制

**提交：**
- CCGO.toml
- CMakeLists.txt
- build_config.py
- CCGO.lock（如果使用锁定依赖）

**不提交：**
- cmake_build/
- target/
- *.pyc

**.gitignore：**
```gitignore
cmake_build/
target/
__pycache__/
*.pyc
.DS_Store
```

### 2. CI/CD 集成

**GitHub Actions 示例：**

```yaml
name: Build All Platforms

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        platform: [android, ios, linux, windows, macos]

    steps:
      - uses: actions/checkout@v3

      - name: Install CCGO
        run: pip install ccgo

      - name: Build ${{ matrix.platform }}
        run: ccgo build ${{ matrix.platform }} --docker --release

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.platform }}-libs
          path: target/${{ matrix.platform }}/*.zip
```

### 3. 构建配置

**开发：**
```toml
[build]
cpp_standard = "17"
compile_flags = ["-Wall", "-Wextra", "-Werror"]  # 严格警告
```

**生产：**
```toml
[build]
cpp_standard = "17"
compile_flags = ["-O3", "-DNDEBUG"]              # 优化
link_flags = ["-flto"]                           # 链接时优化
```

### 4. 依赖管理

**固定依赖以确保可重现性：**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }
```

**使用 CCGO.lock：**
```bash
ccgo install --locked  # 从 CCGO.lock 安装精确版本
```

## 高级主题

### 多模块构建

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

**构建顺序：**
1. CCGO 自动确定构建顺序
2. lib1 首先构建
3. lib2 以 lib1 作为依赖构建

### 交叉编译

**示例：在 Linux 上构建 macOS 库：**
```bash
# 使用 OSXCross 的 Docker
ccgo build macos --docker
```

**示例：在 macOS 上构建 Windows 库：**
```bash
# 使用 MinGW 的 Docker
ccgo build windows --docker --toolchain mingw
```

### 自定义工具链

**添加自定义工具链文件：**

```cmake
# my-toolchain.cmake
set(CMAKE_SYSTEM_NAME Linux)
set(CMAKE_C_COMPILER /path/to/custom-gcc)
set(CMAKE_CXX_COMPILER /path/to/custom-g++)
```

**在构建中使用：**
```python
# build_config.py
CUSTOM_TOOLCHAIN = "/path/to/my-toolchain.cmake"
```

## 另请参阅

- [CLI 参考](../reference/cli.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
- [平台指南](../platforms/index.zh.md)
- [依赖管理](dependency-management.zh.md)
- [Docker 构建](docker-builds.zh.md)
- [发布](publishing.zh.md)
