# CCGO Dependencies Guide

本文档介绍如何使用CCGO的依赖管理系统，包括安装、配置和使用第三方库。

## 目录

- [快速开始](#快速开始)
- [CCGO.toml配置](#ccgotoml配置)
- [安装依赖](#安装依赖)
- [CMake集成](#cmake集成)
- [Link Type支持](#link-type支持)
- [打包SDK](#打包sdk)

## 快速开始

### 1. 配置依赖

在项目的`CCGO.toml`文件中声明依赖：

```toml
[project]
name = "myproject"
version = "1.0.0"

[dependencies]
# 从远程URL下载
libfoo = { version = "1.0.0", source = "https://example.com/libfoo_SDK-1.0.0.zip" }

# 使用本地路径
libbar = { path = "../libbar/sdk_package/libbar_SDK-1.0.0" }
```

### 2. 安装依赖

```bash
# 安装所有依赖
ccgo install

# 安装特定依赖
ccgo install libfoo

# 强制重新安装
ccgo install --force
```

### 3. 在CMake中使用

```cmake
# 在CMakeLists.txt中
include(${CCGO_CMAKE_DIR}/FindCCGODependencies.cmake)
find_ccgo_dependencies()

# 链接依赖到目标
ccgo_link_dependency(myapp libfoo)
```

### 4. 构建项目

```bash
# 正常构建
ccgo build android
ccgo build ios
```

## CCGO.toml配置

### 基本格式

```toml
[dependencies]
# 库名 = { 配置选项 }
```

### 配置选项

#### 1. 远程URL依赖

```toml
[dependencies]
libfoo = {
    version = "1.0.0",
    source = "https://example.com/libfoo_SDK-1.0.0.zip"
}
```

支持的格式：
- `.zip` - ZIP压缩包
- `.tar.gz` - Gzip压缩的tar包
- `.tgz` - Gzip压缩的tar包（简写）

#### 2. 本地路径依赖

```toml
[dependencies]
# 相对路径（相对于项目根目录）
libbar = { path = "../libbar/sdk_package/libbar_SDK-1.0.0" }

# 绝对路径
libbaz = { path = "/absolute/path/to/libbaz_SDK-1.0.0" }
```

#### 3. 本地归档文件

```toml
[dependencies]
libqux = { source = "../archives/libqux_SDK-1.0.0.tar.gz" }
```

### 平台特定依赖

为不同平台配置不同的依赖：

```toml
# 通用依赖（所有平台）
[dependencies]
common_lib = { version = "1.0.0", source = "https://example.com/common.zip" }

# Android专用依赖
[dependencies.android]
android_lib = { version = "1.0.0", source = "https://example.com/android.zip" }

# iOS专用依赖
[dependencies.ios]
ios_lib = { version = "1.0.0", source = "https://example.com/ios.zip" }

# macOS专用依赖
[dependencies.macos]
macos_lib = { version = "1.0.0", source = "https://example.com/macos.zip" }

# tvOS专用依赖
[dependencies.tvos]
tvos_lib = { version = "1.0.0", source = "https://example.com/tvos.zip" }

# watchOS专用依赖
[dependencies.watchos]
watchos_lib = { version = "1.0.0", source = "https://example.com/watchos.zip" }

# Windows专用依赖
[dependencies.windows]
windows_lib = { version = "1.0.0", source = "https://example.com/windows.zip" }

# Linux专用依赖
[dependencies.linux]
linux_lib = { version = "1.0.0", source = "https://example.com/linux.zip" }

# OpenHarmony专用依赖
[dependencies.ohos]
ohos_lib = { version = "1.0.0", source = "https://example.com/ohos.zip" }
```

## 安装依赖

### 基本命令

```bash
# 安装所有依赖
ccgo install

# 安装特定依赖
ccgo install libfoo

# 强制重新安装
ccgo install --force

# 清理缓存后安装
ccgo install --clean-cache
```

### 平台特定安装

```bash
# 只安装Android依赖
ccgo install --platform android

# 只安装iOS依赖
ccgo install --platform ios
```

### 自定义缓存目录

```bash
# 使用自定义缓存目录
ccgo install --cache-dir /tmp/ccgo-cache
```

### 安装目录结构

依赖安装后的目录结构：

```
myproject/
├── third_party/                    # 依赖安装目录
│   ├── libfoo/                     # 库名
│   │   ├── include/               # 头文件
│   │   ├── lib/                   # 库文件
│   │   │   ├── android/
│   │   │   │   ├── static/       # 静态库
│   │   │   │   │   ├── arm64-v8a/
│   │   │   │   │   ├── armeabi-v7a/
│   │   │   │   │   └── x86_64/
│   │   │   │   └── shared/       # 动态库
│   │   │   │       ├── arm64-v8a/
│   │   │   │       ├── armeabi-v7a/
│   │   │   │       └── x86_64/
│   │   │   ├── ios/
│   │   │   │   ├── static/
│   │   │   │   └── shared/
│   │   │   └── ...
│   │   └── ccgo-package.json      # 包元数据
│   └── libbar/
│       └── ...
└── .ccgo/
    └── cache/                      # 下载缓存
        └── abc123_libfoo.zip
```

## CMake集成

### 基本用法

在`CMakeLists.txt`中引入FindCCGODependencies：

```cmake
cmake_minimum_required(VERSION 3.10)
project(MyProject)

# 引入CCGO依赖查找器
include(${CCGO_CMAKE_DIR}/FindCCGODependencies.cmake)

# 查找所有已安装的依赖
find_ccgo_dependencies()

# 创建目标
add_executable(myapp src/main.cpp)

# 链接依赖
if(CCGO_DEPENDENCY_LIBFOO_FOUND)
    ccgo_link_dependency(myapp libfoo)
endif()
```

### 可用的CMake变量

查找依赖后，会设置以下变量（以libfoo为例）：

```cmake
CCGO_DEPENDENCIES_FOUND                     # 是否找到任何依赖
CCGO_DEPENDENCY_LIBFOO_FOUND                # 是否找到libfoo
CCGO_DEPENDENCY_LIBFOO_INCLUDE_DIRS         # libfoo的include目录
CCGO_DEPENDENCY_LIBFOO_LIBRARIES            # libfoo的库文件
CCGO_DEPENDENCY_LIBFOO_STATIC_LIBRARIES     # libfoo的静态库
CCGO_DEPENDENCY_LIBFOO_SHARED_LIBRARIES     # libfoo的动态库
```

### 手动链接依赖

```cmake
# 不使用helper函数，手动链接
if(CCGO_DEPENDENCY_LIBFOO_FOUND)
    target_include_directories(myapp PRIVATE
        ${CCGO_DEPENDENCY_LIBFOO_INCLUDE_DIRS}
    )
    target_link_libraries(myapp PRIVATE
        ${CCGO_DEPENDENCY_LIBFOO_LIBRARIES}
    )
endif()
```

### 控制Link Type

```cmake
# 在find_ccgo_dependencies()之前设置
set(CCGO_DEPENDENCY_LINK_TYPE "static")   # 使用静态库
# set(CCGO_DEPENDENCY_LINK_TYPE "shared")  # 使用动态库

find_ccgo_dependencies()
```

### 平台特定依赖

```cmake
# Android平台
if(ANDROID)
    if(CCGO_DEPENDENCY_LIBANDROID_FOUND)
        ccgo_link_dependency(myapp libandroid)
    endif()
endif()

# iOS平台
if(IOS)
    if(CCGO_DEPENDENCY_LIBIOS_FOUND)
        ccgo_link_dependency(myapp libios)
    endif()
endif()

# macOS平台
if(CMAKE_SYSTEM_NAME STREQUAL "Darwin" AND NOT IOS)
    if(CCGO_DEPENDENCY_LIBMACOS_FOUND)
        ccgo_link_dependency(myapp libmacos)
    endif()
endif()
```

## Link Type支持

CCGO支持构建和使用static（静态）和shared（动态）两种类型的库。

### 构建时指定Link Type

所有平台的构建脚本都支持`link_type`参数：

```python
# build_config.py中
def main():
    # 构建静态库（默认）
    build_platform(link_type='static')

    # 构建动态库
    build_platform(link_type='shared')

    # 同时构建两种类型
    build_platform(link_type='both')
```

### 平台支持情况

| 平台 | Static (.a/.lib) | Shared (.so/.dll/.dylib) |
|------|------------------|-------------------------|
| Android | ✅ | ✅ |
| iOS | ✅ | ✅ |
| macOS | ✅ | ✅ |
| tvOS | ✅ | ✅ |
| watchOS | ✅ | ✅ |
| Windows | ✅ | ✅ |
| Linux | ✅ | ✅ |
| OHOS | ✅ | ✅ |

### 输出目录结构

构建后的输出目录结构：

```
cmake_build/
└── <Platform>/
    └── <Platform>.out/
        ├── static/                 # 静态库输出
        │   ├── <arch>/            # 架构目录（Android/OHOS/Windows）
        │   │   └── lib*.a         # 或 *.lib
        │   └── *.framework        # Apple平台
        └── shared/                # 动态库输出
            ├── <arch>/
            │   └── lib*.so        # 或 *.dll
            └── *.framework
```

## 打包SDK

### 生成SDK包

```bash
# 打包所有平台
ccgo package

# 打包特定平台
ccgo package --platforms android,ios,macos

# 指定版本
ccgo package --version 1.0.0

# 包含文档
ccgo package --include-docs

# 清理输出目录
ccgo package --clean --output ./release
```

### SDK包结构

生成的SDK包结构：

```
myproject_SDK-1.0.0/
├── include/                       # 公共头文件
│   └── myproject/
│       └── *.h
├── lib/                           # 平台库文件
│   ├── android/
│   │   ├── static/
│   │   │   ├── arm64-v8a/
│   │   │   │   └── libmyproject.a
│   │   │   ├── armeabi-v7a/
│   │   │   └── x86_64/
│   │   └── shared/
│   │       ├── arm64-v8a/
│   │       │   └── libmyproject.so
│   │       ├── armeabi-v7a/
│   │       └── x86_64/
│   ├── ios/
│   │   ├── static/
│   │   │   └── myproject.xcframework/
│   │   └── shared/
│   │       └── myproject.xcframework/
│   ├── macos/
│   ├── tvos/
│   ├── watchos/
│   ├── windows/
│   ├── linux/
│   └── ohos/
├── ccgo-package.json              # 包元数据
└── README.md                      # 包说明
```

### ccgo-package.json格式

```json
{
  "name": "myproject",
  "version": "1.0.0",
  "generated": "2025-11-25T10:30:00",
  "platforms": {
    "android": {
      "link_types": {
        "static": {
          "architectures": {
            "arm64-v8a": {
              "libraries": [
                {
                  "name": "libmyproject.a",
                  "size": 123456,
                  "path": "lib/android/static/arm64-v8a/libmyproject.a"
                }
              ]
            }
          }
        },
        "shared": { ... }
      }
    },
    "ios": { ... }
  }
}
```

### 使用SDK包作为依赖

生成的SDK包可以被其他项目作为依赖使用：

```toml
# 在另一个项目的CCGO.toml中
[dependencies]
myproject = {
    version = "1.0.0",
    path = "../myproject/sdk_package/myproject_SDK-1.0.0"
}
```

## 完整示例

### 1. 创建项目并配置依赖

```bash
# 创建新项目
ccgo new myapp

# 编辑CCGO.toml
cd myapp
```

```toml
# CCGO.toml
[project]
name = "myapp"
version = "1.0.0"

[dependencies]
curl = { version = "8.0.0", source = "https://example.com/curl_SDK-8.0.0.zip" }
openssl = { path = "../openssl/sdk" }
```

### 2. 安装依赖

```bash
ccgo install
```

输出：
```
================================================================================
CCGO Install - Install Project Dependencies
================================================================================

Project directory: /path/to/myapp

📖 Reading dependencies from CCGO.toml...

Found 2 dependency(ies) to install:
  - curl
  - openssl

================================================================================
Installing Dependencies
================================================================================

📦 Installing curl...
   Source type: remote_url
   Source: https://example.com/curl_SDK-8.0.0.zip
   📥 Downloading from https://example.com/curl_SDK-8.0.0.zip...
   Progress: 100%
   ✓ Downloaded to .ccgo/cache/abc123_curl_SDK-8.0.0.zip
   📦 Extracting curl_SDK-8.0.0.zip...
   ✓ Extracted to .ccgo/temp/curl
   ✓ Installed to third_party/curl

📦 Installing openssl...
   Source type: local_dir
   Source: /path/to/openssl/sdk
   📂 Copying from local directory...
   ✓ Installed to third_party/openssl

================================================================================
Installation Summary
================================================================================

✓ Successfully installed: 2
```

### 3. 在CMake中使用

```cmake
# CMakeLists.txt
cmake_minimum_required(VERSION 3.10)
project(myapp)

# 引入CCGO依赖
include(${CCGO_CMAKE_DIR}/FindCCGODependencies.cmake)
find_ccgo_dependencies()

# 创建应用
add_executable(myapp src/main.cpp)

# 链接依赖
ccgo_link_dependency(myapp curl)
ccgo_link_dependency(myapp openssl)
```

### 4. 构建

```bash
# Android
ccgo build android --arch arm64-v8a,armeabi-v7a

# iOS
ccgo build ios

# macOS
ccgo build macos
```

### 5. 打包SDK

```bash
ccgo package --version 1.0.0 --include-docs
```

## 故障排除

### 问题：依赖未找到

```
ERROR: CCGO.toml not found in project directory
```

**解决方案：** 确保在项目根目录执行命令，且存在CCGO.toml文件。

### 问题：下载失败

```
✗ Download failed: HTTP Error 404
```

**解决方案：** 检查依赖的source URL是否正确，网络是否可访问。

### 问题：CMake找不到依赖

```
WARNING: Library directory not found for libfoo
```

**解决方案：**
1. 确保运行了`ccgo install`
2. 检查`third_party/libfoo`目录是否存在
3. 检查是否有对应平台的库文件

### 问题：Link Type不匹配

**解决方案：** 在CMake中设置正确的link type：

```cmake
set(CCGO_DEPENDENCY_LINK_TYPE "static")  # 或 "shared"
find_ccgo_dependencies()
```

## 最佳实践

1. **版本管理**：在CCGO.toml中明确指定版本号
2. **缓存管理**：定期清理`.ccgo/cache`目录
3. **平台依赖**：只为需要的平台配置依赖
4. **路径使用**：开发时使用相对路径，生产环境使用URL
5. **Link Type**：根据需求选择static或shared
6. **依赖更新**：使用`--force`强制更新依赖

## 参考资料

- [CCGO.toml.example](build_scripts/CCGO.toml.example) - 完整配置示例
- [CMakeLists.txt.dependencies.example](build_scripts/cmake/CMakeLists.txt.dependencies.example) - CMake使用示例
- [FindCCGODependencies.cmake](build_scripts/cmake/FindCCGODependencies.cmake) - CMake模块源码
