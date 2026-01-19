# Linux 平台

使用 CCGO 为 Linux 构建 C++ 库的完整指南。

## 概述

CCGO 提供全面的 Linux 支持：
- **多编译器**：GCC 和 Clang
- **多架构**：x86_64、ARM64 (aarch64)、ARMv7
- **输出格式**：静态库 (.a)、共享库 (.so)
- **构建方式**：本地构建或 Docker（跨平台）
- **IDE 支持**：CodeLite 项目生成
- **发行版兼容**：Ubuntu、Debian、CentOS、Fedora、Alpine
- **C 库变体**：支持 glibc 和 musl

## 前置条件

### 方式 1：本地构建（需要 Linux）

**Ubuntu/Debian:**

```bash
# 安装 GCC 工具链
sudo apt update
sudo apt install -y build-essential cmake pkg-config

# 安装 Clang（可选）
sudo apt install -y clang

# 验证安装
gcc --version
g++ --version
cmake --version
```

**CentOS/Fedora/RHEL:**

```bash
# 安装 GCC 工具链
sudo yum groupinstall -y "Development Tools"
sudo yum install -y cmake

# 或在 Fedora 上
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y cmake

# 安装 Clang（可选）
sudo yum install -y clang
# 或在 Fedora 上
sudo dnf install -y clang

# 验证
gcc --version
g++ --version
cmake --version
```

**Alpine Linux:**

```bash
# 安装 GCC 工具链（musl libc）
apk add --no-cache build-base cmake

# 安装 Clang（可选）
apk add --no-cache clang

# 验证
gcc --version
g++ --version
cmake --version
```

### 方式 2：Docker 构建（任意操作系统）

在 macOS 或 Windows 上使用 Docker 构建 Linux 库。

**所需条件：**
- 已安装并运行 Docker Desktop
- 3GB+ 磁盘空间用于 Docker 镜像

**优势：**
- 可在任何操作系统上构建
- 一致的构建环境
- 无需 Linux 依赖
- 支持多个 Linux 发行版

**限制：**
- 无法运行/测试应用程序（除非使用 Docker shell）
- 首次下载较大（~800MB 镜像）

详见 [Docker 构建](#docker-构建) 部分。

## 快速开始

### 基础构建

```bash
# 使用默认编译器为 x86_64 构建
ccgo build linux

# 使用 Docker 构建（从任意操作系统交叉编译）
ccgo build linux --docker

# 指定编译器
ccgo build linux --compiler gcc       # GCC（默认）
ccgo build linux --compiler clang     # Clang
ccgo build linux --compiler auto      # 两个编译器都构建

# 指定架构
ccgo build linux --arch x86_64        # 64位 Intel/AMD（默认）
ccgo build linux --arch arm64         # 64位 ARM (aarch64)
ccgo build linux --arch armv7         # 32位 ARM

# 构建类型
ccgo build linux --build-type debug    # Debug 构建
ccgo build linux --build-type release  # Release 构建（默认）

# 链接类型
ccgo build linux --link-type static    # 仅静态库
ccgo build linux --link-type shared    # 仅共享库
ccgo build linux --link-type both      # 两种类型（默认）
```

### 生成 CodeLite 项目

```bash
# 生成 CodeLite 工作区
ccgo build linux --ide-project

# 在 CodeLite 中打开
codelite cmake_build/linux/MyLib.workspace
```

## 输出结构

### 默认输出 (`target/linux/`)

```
target/linux/
├── MyLib_Linux_SDK-1.0.0.zip           # 主包
│   ├── lib/
│   │   ├── static/
│   │   │   ├── gcc/
│   │   │   │   └── libmylib.a          # GCC 静态库
│   │   │   └── clang/
│   │   │       └── libmylib.a          # Clang 静态库
│   │   └── shared/
│   │       ├── gcc/
│   │       │   ├── libmylib.so         # GCC 共享库
│   │       │   └── libmylib.so.1.0.0   # 版本化库
│   │       └── clang/
│   │           ├── libmylib.so
│   │           └── libmylib.so.1.0.0
│   ├── include/
│   │   └── mylib/                      # 头文件
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                 # 构建元数据
│
└── MyLib_Linux_SDK-1.0.0-SYMBOLS.zip   # 调试符号
    └── symbols/
        ├── gcc/
        │   └── libmylib.so.debug       # GCC 调试符号
        └── clang/
            └── libmylib.so.debug       # Clang 调试符号
```

### 库类型

**静态库 (.a):**
- 目标文件的归档
- 编译时链接
- 可执行文件较大
- 无运行时依赖
- 包含所有符号

**共享库 (.so):**
- 运行时动态链接
- 可执行文件较小
- 进程间共享
- 版本化（libmylib.so.1.0.0）
- 兼容性符号链接：
  - `libmylib.so` → `libmylib.so.1` → `libmylib.so.1.0.0`

### 构建元数据

`build_info.json` 包含：

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My Linux library"
  },
  "build": {
    "platform": "linux",
    "architectures": ["x86_64"],
    "compilers": ["gcc", "clang"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "gcc_version": "11.4.0",
    "clang_version": "14.0.0",
    "libc": "glibc-2.35"
  },
  "outputs": {
    "libraries": {
      "gcc": {
        "static": "lib/static/gcc/libmylib.a",
        "shared": "lib/shared/gcc/libmylib.so"
      },
      "clang": {
        "static": "lib/static/clang/libmylib.a",
        "shared": "lib/shared/clang/libmylib.so"
      }
    },
    "headers": "include/mylib/",
    "symbols": {
      "gcc": "symbols/gcc/libmylib.so.debug",
      "clang": "symbols/clang/libmylib.so.debug"
    }
  }
}
```

## GCC vs Clang

### GCC (GNU 编译器集合)

**优点：**
- 大多数 Linux 发行版的默认编译器
- 优秀的优化
- 广泛的架构支持
- 更好的 C++20/C++23 支持（最新版本）
- 更大的社区

**缺点：**
- 编译速度比 Clang 慢
- 错误消息不如 Clang 友好
- 有时生成的二进制文件更大

**何时使用：**
- 标准 Linux 开发
- 最大兼容性
- 最新的 C++ 标准
- 最佳优化

### Clang (LLVM)

**优点：**
- 更快的编译速度
- 更好的错误消息和警告
- 优秀的静态分析
- 更适合开发
- 模块化架构

**缺点：**
- 可能生成稍慢的代码
- 不太常见作为默认编译器
- 某些架构优化较少

**何时使用：**
- 开发和调试
- 需要更好的诊断信息
- 使用 LLVM 生态系统
- 需要静态分析

## 在 C++ 中使用库

### 链接静态库

**CMakeLists.txt:**

```cmake
# 查找库
find_library(MYLIB_LIBRARY
    NAMES mylib libmylib.a
    PATHS "/path/to/lib/static/gcc"
)

# 链接到目标
target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "/path/to/include")
```

**手动编译：**

```bash
# 使用 GCC
g++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/static/gcc -lmylib

# 使用 Clang
clang++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/static/clang -lmylib
```

### 链接共享库

**CMakeLists.txt:**

```cmake
# 查找共享库
find_library(MYLIB_LIBRARY
    NAMES mylib
    PATHS "/path/to/lib/shared/gcc"
)

target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "/path/to/include")

# 设置 RPATH 以便运行时找到库
set_target_properties(myapp PROPERTIES
    BUILD_RPATH "/path/to/lib/shared/gcc"
    INSTALL_RPATH "$ORIGIN:$ORIGIN/../lib"
)
```

**手动编译：**

```bash
# 使用 GCC
g++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/shared/gcc -lmylib \
    -Wl,-rpath,/path/to/lib/shared/gcc

# 使用 LD_LIBRARY_PATH 运行
LD_LIBRARY_PATH=/path/to/lib/shared/gcc ./myapp
```

**在代码中使用：**

```cpp
#include <mylib/mylib.h>

int main() {
    mylib::MyClass obj;
    obj.do_work();
    return 0;
}
```

## Docker 构建

在任何操作系统上使用 Docker 构建 Linux 库：

### 前置条件

```bash
# 安装 Docker Desktop
# 下载地址：https://www.docker.com/products/docker-desktop/

# 验证 Docker 运行
docker ps
```

### 使用 Docker 构建

```bash
# 首次构建会下载预构建镜像（~800MB）
ccgo build linux --docker

# 后续构建很快
ccgo build linux --docker --arch x86_64

# 所有标准选项都可用
ccgo build linux --docker --compiler gcc --link-type static
```

### 工作原理

1. CCGO 使用 Docker Hub 的预构建 `ccgo-builder-linux` 镜像
2. 项目目录挂载到容器中
3. 使用 Ubuntu 22.04 + GCC/Clang 构建
4. 输出写入主机文件系统

### 限制

- **无法运行**：Docker 中没有 X11 显示
- **无法测试**：GUI 应用程序无法工作
- 使用 `docker exec -it <container> bash` 运行 CLI 应用

## 发行版兼容性

### glibc vs musl

**glibc (GNU C 库):**
- 大多数发行版的标准
- 更好的性能
- 更广泛的兼容性
- 二进制文件更大

**musl:**
- 用于 Alpine Linux
- 更小更简单
- 静态链接友好
- 严格符合 POSIX

### ABI 兼容性

在较旧的发行版上构建的库通常可在较新的发行版上工作：

```
# 在 Ubuntu 18.04 上构建（glibc 2.27）
# 可在 Ubuntu 20.04、22.04、24.04 上工作

# 在 Ubuntu 22.04 上构建（glibc 2.35）
# 可能无法在 Ubuntu 18.04、20.04 上工作
```

**最佳实践**：在您需要支持的最旧发行版上构建。

### 版本化共享库

CCGO 自动创建版本化的共享库：

```bash
# 创建的文件
libmylib.so.1.0.0         # 带完整版本的实际库
libmylib.so.1             # SONAME 符号链接
libmylib.so               # 开发符号链接

# 检查 SONAME
objdump -p libmylib.so.1.0.0 | grep SONAME
# 输出：SONAME      libmylib.so.1
```

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

[linux]
compiler = "gcc"               # gcc、clang 或 auto
libc = "glibc"                 # glibc 或 musl
position_independent_code = true  # 共享库的 PIC
strip_symbols = false          # 剥离调试符号
```

### CMake 变量

为 Linux 构建时：

```cmake
${PLATFORM}                    # "linux"
${ARCHITECTURE}                # "x86_64"、"arm64" 或 "armv7"
${BUILD_TYPE}                  # "Debug" 或 "Release"
${LINK_TYPE}                   # "static"、"shared" 或 "both"
${COMPILER}                    # "gcc" 或 "clang"
${LINUX_LIBC}                  # "glibc" 或 "musl"
```

### 条件编译

```cpp
// 平台检测
#ifdef __linux__
    // Linux 特定代码
    #include <unistd.h>

    #ifdef __x86_64__
        // x86_64 特定代码
    #elif defined(__aarch64__)
        // ARM64 特定代码
    #elif defined(__arm__)
        // ARMv7 特定代码
    #endif
#endif

// 编译器检测
#ifdef __GNUC__
    // GCC 或 Clang
    #define MYLIB_API __attribute__((visibility("default")))

    #ifdef __clang__
        // Clang 特定代码
    #else
        // GCC 特定代码
    #endif
#endif

// glibc 检测
#ifdef __GLIBC__
    // glibc 特定代码
    #include <gnu/libc-version.h>
#endif

// 使用
class MYLIB_API MyClass {
public:
    void do_work();
};
```

### 符号可见性

控制共享库中导出的符号：

```cpp
// mylib_export.h
#ifdef __linux__
    #ifdef MYLIB_EXPORTS
        #define MYLIB_API __attribute__((visibility("default")))
    #else
        #define MYLIB_API
    #endif
    #define MYLIB_LOCAL __attribute__((visibility("hidden")))
#else
    #define MYLIB_API
    #define MYLIB_LOCAL
#endif

// 公共 API
class MYLIB_API PublicClass {
public:
    void public_method();
};

// 内部实现（不导出）
class MYLIB_LOCAL InternalClass {
public:
    void internal_method();
};
```

## 最佳实践

### 1. 控制符号可见性

隐藏内部符号以减小库大小并提高加载速度：

```cmake
# CMakeLists.txt
set(CMAKE_CXX_VISIBILITY_PRESET hidden)
set(CMAKE_VISIBILITY_INLINES_HIDDEN YES)
```

```cpp
// 显式导出公共 API
class __attribute__((visibility("default"))) MyPublicClass { ... };
```

### 2. 使用 RPATH 进行分发

设置 RPATH 以便应用程序可以找到共享库：

```cmake
# 将库安装到相对于二进制文件的 lib/ 目录
set_target_properties(myapp PROPERTIES
    INSTALL_RPATH "$ORIGIN:$ORIGIN/../lib"
)
```

目录结构：
```
myapp/
├── bin/
│   └── myapp              # 可执行文件
└── lib/
    └── libmylib.so        # 库
```

### 3. 版本化共享库

为共享库遵循语义版本控制：

```toml
[package]
version = "1.2.3"          # 创建 libmylib.so.1.2.3
```

### 4. 静态链接以简化部署

为了更简单的分发而无需依赖：

```bash
# 仅构建静态库
ccgo build linux --link-type static

# 所有代码都嵌入在可执行文件中
g++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/static -lmylib
```

### 5. 在目标发行版上测试

始终在实际目标发行版上测试：
- Ubuntu 20.04、22.04、24.04
- Debian 11、12
- CentOS 7、8、9
- Fedora（最新版本）
- Alpine（用于 musl）

### 6. 使用位置无关代码

始终为共享库启用 PIC：

```toml
[linux]
position_independent_code = true
```

### 7. 剥离发布二进制文件

减小发布构建的库大小：

```bash
# 剥离调试符号
strip libmylib.so

# 或在 CCGO.toml 中配置
[linux]
strip_symbols = true
```

## 故障排除

### 找不到编译器

```
Error: g++ not found
```

**解决方案：**

```bash
# Ubuntu/Debian
sudo apt install -y build-essential

# CentOS/Fedora
sudo yum groupinstall -y "Development Tools"

# 验证
which g++
g++ --version
```

### 运行时找不到库

```
error while loading shared libraries: libmylib.so: cannot open shared object file
```

**解决方案：**

1. **添加到 LD_LIBRARY_PATH:**
```bash
export LD_LIBRARY_PATH=/path/to/lib:$LD_LIBRARY_PATH
./myapp
```

2. **安装到系统目录：**
```bash
sudo cp libmylib.so /usr/local/lib/
sudo ldconfig
```

3. **使用 RPATH（推荐）：**
```cmake
set_target_properties(myapp PROPERTIES
    INSTALL_RPATH "$ORIGIN/../lib"
)
```

4. **检查库路径：**
```bash
ldd myapp
# 显示：libmylib.so => not found

# 修复后：
ldd myapp
# 显示：libmylib.so => /path/to/lib/libmylib.so
```

### 找不到符号

```
undefined reference to 'mylib::MyClass::do_work()'
```

**解决方案：**

1. **检查符号是否存在：**
```bash
nm -C libmylib.so | grep do_work
# 应显示：00001234 T mylib::MyClass::do_work()
```

2. **验证库已链接：**
```bash
ldd myapp | grep mylib
# 应显示 libmylib.so
```

3. **检查符号可见性：**
```cpp
// 确保符号已导出
class __attribute__((visibility("default"))) MyClass { ... };
```

### 版本不匹配

```
version `GLIBC_2.35' not found
```

**解决方案：**

在较旧的发行版上构建或使用静态链接：

```bash
# 检查所需的 glibc 版本
objdump -T libmylib.so | grep GLIBC

# 检查系统 glibc 版本
ldd --version

# 在较旧的系统上构建或使用较旧基础镜像的 Docker
```

### CMake 配置失败

```
Could not find a package configuration file provided by "MyLib"
```

**解决方案：**

确保 CMake 可以找到库：

```cmake
# 设置 CMAKE_PREFIX_PATH
set(CMAKE_PREFIX_PATH "/path/to/MyLib/lib/cmake")
find_package(MyLib REQUIRED)

# 或设置为环境变量
export CMAKE_PREFIX_PATH=/path/to/MyLib/lib/cmake
```

## 性能提示

### 1. 使用链接时优化

```toml
[build]
cxxflags = ["-flto"]       # 启用 LTO
ldflags = ["-flto"]
```

### 2. 启用优化

```toml
[build]
cxxflags = [
    "-O3",                 # 最大优化
    "-march=native",       # 使用 CPU 特定指令
    "-mtune=native"
]
```

### 3. 配置文件引导优化

```bash
# 1. 使用 profiling 构建
CXXFLAGS="-fprofile-generate" ccgo build linux

# 2. 使用典型工作负载运行
./benchmark

# 3. 使用 profile 数据重新构建
CXXFLAGS="-fprofile-use" ccgo build linux
```

### 4. 静态链接以提高性能

由于更好的优化，静态链接可能更快：

```bash
ccgo build linux --link-type static
```

### 5. 禁用异常（如果不需要）

```toml
[build]
cxxflags = ["-fno-exceptions"]
```

## 打包和分发

### 系统包集成

**Debian/Ubuntu (.deb):**

```bash
# 安装 checkinstall
sudo apt install checkinstall

# 创建 .deb 包
cd target/linux
sudo checkinstall --pkgname=mylib --pkgversion=1.0.0 \
    --provides=mylib make install
```

**基于 RPM (.rpm):**

```bash
# 创建 RPM 包
rpmbuild -ba mylib.spec
```

**AppImage（便携式）:**

```bash
# 捆绑应用程序及其依赖项
appimagetool myapp.AppDir myapp.AppImage
```

### Snap 包

```yaml
# snapcraft.yaml
name: mylib
version: '1.0.0'
summary: My Linux library
description: Complete C++ library for Linux

parts:
  mylib:
    plugin: cmake
    source: .
```

```bash
# 构建 snap
snapcraft
```

### Flatpak

```json
{
  "app-id": "com.example.mylib",
  "runtime": "org.freedesktop.Platform",
  "sdk": "org.freedesktop.Sdk",
  "command": "myapp"
}
```

```bash
# 构建 flatpak
flatpak-builder build-dir com.example.mylib.json
```

## 迁移指南

### 从 Makefile

**之前：**
```makefile
CC = gcc
CFLAGS = -O2 -Wall
TARGET = libmylib.so

$(TARGET): mylib.o
    $(CC) -shared -o $(TARGET) mylib.o
```

**之后：**

1. 创建 CCGO 项目：
```bash
ccgo new mylib
```

2. 将源文件复制到 `src/`

3. 配置 CCGO.toml：
```toml
[linux]
compiler = "gcc"
```

4. 构建：
```bash
ccgo build linux
```

### 从 CMake

**CMakeLists.txt:**
```cmake
project(mylib)
add_library(mylib SHARED src/mylib.cpp)
target_include_directories(mylib PUBLIC include)
```

**CCGO.toml:**
```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "shared"
```

然后：`ccgo build linux`

### 从 Autotools

**之前：**
```bash
./configure
make
make install
```

**之后：**

1. 从 `src/` 目录提取源文件
2. 使用 `ccgo new` 创建 CCGO 项目
3. 将源文件复制到新项目结构
4. 在 CCGO.toml 中配置依赖项
5. 使用 `ccgo build linux` 构建

## 高级主题

### 交叉编译

为不同架构构建：

```bash
# 在 x86_64 上为 ARM64 构建
ccgo build linux --arch arm64 --docker

# 为 ARMv7 构建
ccgo build linux --arch armv7 --docker
```

### 静态 musl 构建

用于真正可移植的静态二进制文件：

```bash
# 使用 Alpine Linux Docker 镜像
ccgo build linux --docker --libc musl --link-type static
```

### 清理器（Sanitizers）

启用地址清理器进行调试：

```toml
[build]
cxxflags = ["-fsanitize=address", "-fno-omit-frame-pointer"]
ldflags = ["-fsanitize=address"]
```

### 覆盖率分析

```bash
# 使用覆盖率构建
CXXFLAGS="-fprofile-arcs -ftest-coverage" ccgo build linux

# 运行测试
ccgo test

# 生成报告
gcov src/mylib.cpp
lcov --capture --directory . --output-file coverage.info
genhtml coverage.info --output-directory coverage_html
```

## 另请参阅

- [构建系统](../features/build-system.md)
- [依赖管理](../features/dependency-management.md)
- [Docker 构建](../features/docker-builds.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [平台概述](index.md)
