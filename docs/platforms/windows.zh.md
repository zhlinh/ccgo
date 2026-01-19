# Windows 平台

使用 CCGO 为 Windows 构建 C++ 库的完整指南。

## 概述

CCGO 提供全面的 Windows 支持：
- **多工具链**：MSVC（Visual Studio）和 MinGW（GCC）
- **多架构**：x86、x64、ARM64
- **输出格式**：静态库（.lib）、动态库（.dll）
- **构建方式**：本地构建（Visual Studio/MinGW）或 Docker（跨平台）
- **IDE 支持**：Visual Studio 项目生成
- **子系统**：控制台和窗口子系统
- **运行时库**：静态和动态 CRT 链接

## 前置条件

### 方式一：本地构建（需要 Windows）

**使用 MSVC：**
- Windows 10+（64位）
- Visual Studio 2019+ 及 C++ 工作负载
- CMake 3.20+

**安装方法：**

```powershell
# 从 visualstudio.microsoft.com 安装 Visual Studio
# 选择"使用 C++ 的桌面开发"工作负载

# 安装 CMake
# 从 cmake.org 下载或使用 chocolatey
choco install cmake

# 验证安装
cmake --version
cl.exe
```

**使用 MinGW：**
- Windows 10+（64位）
- MinGW-w64 或 MSYS2
- CMake 3.20+

**安装方法：**

```powershell
# 从 msys2.org 安装 MSYS2
# 然后安装 MinGW-w64
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-cmake

# 添加到 PATH
# C:\msys64\mingw64\bin

# 验证
gcc --version
g++ --version
```

### 方式二：Docker 构建（任何操作系统）

在 Linux 或 macOS 上使用 Docker 和 MinGW 构建 Windows 库。

**必需：**
- 已安装并运行 Docker Desktop
- 5GB+ 磁盘空间用于 Docker 镜像

**优势：**
- 在任何操作系统上构建
- 无需 Windows 许可证
- 一致的构建环境
- MinGW-w64 交叉编译

**限制：**
- 仅 MinGW（无 MSVC）
- 无法运行/测试 Windows 应用
- 初始下载较大（约 1.2GB 镜像）

详见 [Docker 构建](#docker-构建)部分。

## 快速开始

### 基本构建

```bash
# 使用默认工具链为 x64 构建
ccgo build windows

# 使用 Docker 构建（MinGW 交叉编译）
ccgo build windows --docker

# 指定工具链
ccgo build windows --toolchain msvc      # MSVC（仅 Windows）
ccgo build windows --toolchain mingw     # MinGW
ccgo build windows --toolchain auto      # 两个工具链（默认）

# 指定架构
ccgo build windows --arch x86            # 32位
ccgo build windows --arch x64            # 64位（默认）
ccgo build windows --arch arm64          # ARM64

# 构建类型
ccgo build windows --build-type debug    # Debug 构建
ccgo build windows --build-type release  # Release 构建（默认）

# 链接类型
ccgo build windows --link-type static    # 仅静态库
ccgo build windows --link-type shared    # 仅 DLL
ccgo build windows --link-type both      # 两种类型（默认）
```

### 生成 Visual Studio 项目

```bash
# 生成 Visual Studio 解决方案
ccgo build windows --ide-project

# 在 Visual Studio 中打开
start cmake_build/windows/msvc/MyLib.sln
```

## 输出结构

### 默认输出 (`target/windows/`)

```
target/windows/
├── MyLib_Windows_SDK-1.0.0.zip          # 主包
│   ├── lib/
│   │   ├── static/
│   │   │   ├── msvc/
│   │   │   │   └── mylib.lib            # MSVC 静态库
│   │   │   └── mingw/
│   │   │       └── libmylib.a           # MinGW 静态库
│   │   └── shared/
│   │       ├── msvc/
│   │       │   ├── mylib.dll            # MSVC DLL
│   │       │   └── mylib.lib            # 导入库
│   │       └── mingw/
│   │           ├── libmylib.dll         # MinGW DLL
│   │           └── libmylib.dll.a       # 导入库
│   ├── bin/                             # DLLs（用于运行时）
│   │   ├── msvc/
│   │   │   └── mylib.dll
│   │   └── mingw/
│   │       └── libmylib.dll
│   ├── include/
│   │   └── mylib/                       # 头文件
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                  # 构建元数据
│
└── MyLib_Windows_SDK-1.0.0-SYMBOLS.zip  # 调试符号
    └── symbols/
        ├── msvc/
        │   └── mylib.pdb                # MSVC 调试符号
        └── mingw/
            └── libmylib.dll.debug       # MinGW 调试符号
```

### 库类型

**静态库：**
- MSVC：`.lib` 文件
- MinGW：`.a` 文件
- 编译时链接
- 可执行文件更大
- 无运行时依赖

**动态库（DLL）：**
- MSVC：`.dll` + `.lib`（导入库）
- MinGW：`.dll` + `.dll.a`（导入库）
- 运行时加载
- 可执行文件更小
- 需要 DLL 在运行时存在

### 构建元数据

`build_info.json` 包含：

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My Windows library"
  },
  "build": {
    "platform": "windows",
    "architectures": ["x64"],
    "toolchains": ["msvc", "mingw"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "msvc_version": "19.38",
    "mingw_version": "13.2.0"
  },
  "outputs": {
    "libraries": {
      "msvc": {
        "static": "lib/static/msvc/mylib.lib",
        "shared": "lib/shared/msvc/mylib.dll"
      },
      "mingw": {
        "static": "lib/static/mingw/libmylib.a",
        "shared": "lib/shared/mingw/libmylib.dll"
      }
    },
    "headers": "include/mylib/",
    "symbols": {
      "msvc": "symbols/msvc/mylib.pdb",
      "mingw": "symbols/mingw/libmylib.dll.debug"
    }
  }
}
```

## MSVC vs MinGW

### MSVC（Microsoft Visual C++）

**优点：**
- 微软官方编译器
- 最佳 Windows 集成
- 使用 Visual Studio 的出色调试
- 更好的 Windows 优化
- 与 Windows SDK 兼容

**缺点：**
- 仅 Windows
- 需要 Visual Studio 安装
- 工具链更大

**何时使用：**
- Windows 特定开发
- 需要 Visual Studio 集成
- 最大化 Windows 性能
- 使用 Windows SDK API

### MinGW（Minimalist GNU for Windows）

**优点：**
- 基于 GCC（跨平台兼容）
- 可以从 Linux/macOS 交叉编译
- 工具链更小
- 开源
- 与 Unix 工具兼容

**缺点：**
- 某些 Windows API 未完全支持
- 可能比 MSVC 慢
- 较少 Windows 特定优化

**何时使用：**
- 跨平台开发
- 在非 Windows 系统上构建
- 需要 GCC 兼容性
- 不需要高级 Windows API

## 在 C++ 中使用库

### 链接静态库

**CMakeLists.txt（MSVC）：**

```cmake
# 查找库
find_library(MYLIB_LIBRARY
    NAMES mylib
    PATHS "path/to/lib/static/msvc"
)

# 链接到目标
target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "path/to/include")
```

**CMakeLists.txt（MinGW）：**

```cmake
find_library(MYLIB_LIBRARY
    NAMES mylib libmylib.a
    PATHS "path/to/lib/static/mingw"
)

target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "path/to/include")
```

### 链接动态库

**CMakeLists.txt：**

```cmake
# 链接导入库
target_link_libraries(myapp PRIVATE "path/to/lib/shared/msvc/mylib.lib")

# 复制 DLL 到输出目录
add_custom_command(TARGET myapp POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "path/to/bin/msvc/mylib.dll"
        $<TARGET_FILE_DIR:myapp>
)
```

**在代码中使用：**

```cpp
#include <mylib/mylib.h>

int main() {
    // DLL 函数自动解析
    mylib::MyClass obj;
    obj.do_work();
    return 0;
}
```

## Docker 构建

在任何操作系统上使用 Docker 和 MinGW 构建 Windows 库：

### 前置条件

```bash
# 安装 Docker Desktop
# 下载地址：https://www.docker.com/products/docker-desktop/

# 验证 Docker 正在运行
docker ps
```

### 使用 Docker 构建

```bash
# 首次构建下载预构建镜像（约 1.2GB）
ccgo build windows --docker

# 后续构建很快
ccgo build windows --docker --arch x64

# 所有标准选项都可用（仅 MinGW）
ccgo build windows --docker --link-type static
```

### 工作原理

1. CCGO 使用 Docker Hub 的预构建 `ccgo-builder-windows` 镜像
2. 项目目录挂载到容器中
3. 使用 MinGW-w64 交叉编译器构建
4. 输出写入主机文件系统

### 限制

- **仅 MinGW**：无法在 Docker 中使用 MSVC 构建
- **无法运行**：Docker 中没有 Windows 运行时
- **无法测试**：无法执行 Windows 二进制文件

## 平台配置

### CCGO.toml 设置

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"                  # static、shared 或 both

[build]
cpp_standard = "17"            # C++ 标准

[windows]
subsystem = "console"          # console 或 windows
runtime_library = "MD"         # MT、MD、MTd、MDd（仅 MSVC）
windows_sdk_version = "10.0"   # Windows SDK 版本
```

### CMake 变量

为 Windows 构建时：

```cmake
${PLATFORM}                    # "windows"
${ARCHITECTURE}                # "x86"、"x64" 或 "arm64"
${BUILD_TYPE}                  # "Debug" 或 "Release"
${LINK_TYPE}                   # "static"、"shared" 或 "both"
${TOOLCHAIN}                   # "msvc" 或 "mingw"
${WINDOWS_SUBSYSTEM}           # "console" 或 "windows"
${MSVC_RUNTIME_LIBRARY}        # "MD"、"MT" 等（仅 MSVC）
```

### 条件编译

```cpp
// 平台检测
#ifdef _WIN32
    // Windows 特定代码
    #include <windows.h>

    #ifdef _WIN64
        // 64位 Windows
    #else
        // 32位 Windows
    #endif
#endif

// 编译器检测
#ifdef _MSC_VER
    // MSVC 特定代码
    #pragma warning(disable: 4996)
#elif defined(__MINGW32__) || defined(__MINGW64__)
    // MinGW 特定代码
#endif

// DLL 导出/导入
#ifdef MYLIB_EXPORTS
    #define MYLIB_API __declspec(dllexport)
#else
    #define MYLIB_API __declspec(dllimport)
#endif

// 用法
class MYLIB_API MyClass {
public:
    void do_work();
};
```

## 最佳实践

### 1. 支持两个工具链

使用 MSVC 和 MinGW 构建：

```bash
# 构建两者（默认）
ccgo build windows --toolchain auto
```

### 2. 使用正确的 DLL 导出

始终使用 `__declspec(dllexport/dllimport)`：

```cpp
// mylib_export.h
#ifdef _WIN32
    #ifdef MYLIB_EXPORTS
        #define MYLIB_API __declspec(dllexport)
    #else
        #define MYLIB_API __declspec(dllimport)
    #endif
#else
    #define MYLIB_API
#endif
```

### 3. 处理运行时库

选择正确的 CRT 链接：

```toml
[windows]
runtime_library = "MD"  # 动态 CRT（推荐）
# runtime_library = "MT"  # 静态 CRT（更大，无依赖）
```

### 4. 在分发中包含 DLL

始终将 DLL 与二进制文件一起包含：

```
distribution/
├── myapp.exe
├── mylib.dll            # 您的 DLL
└── vcruntime140.dll     # MSVC 运行时（如果需要）
```

### 5. 在目标 Windows 上测试

始终在实际 Windows 系统上测试：
- 不同的 Windows 版本（10、11）
- 不同的架构（x86、x64）
- 有和没有 Visual Studio 安装的情况

## 故障排除

### 未找到 MSVC

```
Error: Could not find MSVC compiler
```

**解决方案：**

```powershell
# 安装带 C++ 工作负载的 Visual Studio
# 或安装生成工具

# 验证
where cl.exe

# 如果需要，添加到 PATH
$env:PATH += ";C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.38.33130\bin\Hostx64\x64"
```

### 未找到 MinGW

```
Error: Could not find MinGW compiler
```

**解决方案：**

```bash
# 安装 MSYS2/MinGW
# 添加到 PATH
export PATH="/c/msys64/mingw64/bin:$PATH"

# 验证
gcc --version
g++ --version
```

### 未找到 DLL

```
Error: The code execution cannot proceed because mylib.dll was not found
```

**解决方案：**

1. 将 DLL 复制到可执行文件目录
2. 将 DLL 目录添加到 PATH：
```powershell
$env:PATH += ";C:\path\to\dlls"
```

3. 使用延迟加载（MSVC）：
```cmake
target_link_options(myapp PRIVATE "/DELAYLOAD:mylib.dll")
```

### 未找到符号

```
Error: unresolved external symbol
```

**解决方案：**

1. 检查 DLL 导出：
```powershell
dumpbin /EXPORTS mylib.dll
```

2. 验证 __declspec(dllexport)：
```cpp
class __declspec(dllexport) MyClass { ... };
```

3. 使用 .def 文件导出：
```
LIBRARY mylib
EXPORTS
    MyFunction
    MyClass
```

## 性能提示

### 1. 使用链接时优化

```toml
[build]
cxxflags = ["/GL"]           # MSVC
ldflags = ["/LTCG"]          # MSVC
# cxxflags = ["-flto"]       # MinGW
```

### 2. 启用优化

```toml
[build]
cxxflags = [
    "/O2",                   # MSVC：优化速度
    "/arch:AVX2"             # 使用 AVX2 指令
]
```

### 3. 独立部署的静态 CRT

用于无需 Visual C++ 可再发行组件的部署：

```toml
[windows]
runtime_library = "MT"       # 静态 CRT
```

## 迁移指南

### 从 Visual Studio 项目

**之前：**
```
MyLib.vcxproj
MyLib.sln
```

**之后：**

1. 创建 CCGO 项目：
```bash
ccgo new mylib
```

2. 复制源文件到 `src/`

3. 配置 CCGO.toml：
```toml
[windows]
subsystem = "console"
runtime_library = "MD"
```

4. 构建：
```bash
ccgo build windows
```

### 从 CMake

**CMakeLists.txt：**
```cmake
project(mylib)
add_library(mylib src/mylib.cpp)
```

**CCGO.toml：**
```toml
[package]
name = "mylib"
version = "1.0.0"
```

然后：`ccgo build windows`

## 另请参阅

- [构建系统](../features/build-system.md)
- [依赖管理](../features/dependency-management.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [平台概览](index.md)
