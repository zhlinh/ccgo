# OpenHarmony 平台

使用 CCGO 为 OpenHarmony (OHOS) 构建 C++ 库的完整指南。

## 概述

CCGO 提供全面的 OpenHarmony 支持：
- **多架构**：ARMv7、ARM64、x86_64
- **输出格式**：HAR 包、静态库 (.a)、共享库 (.so)
- **构建方式**：本地构建 (DevEco Studio) 或 Docker（跨平台）
- **IDE 支持**：DevEco Studio 项目生成
- **发布**：OHPM 仓库集成
- **ArkTS 集成**：原生模块接口 (NAPI) 支持
- **API 兼容性**：OpenHarmony 3.2+

## 前置条件

### 方式 1：本地构建（需要 OpenHarmony SDK）

**安装 DevEco Studio:**

1. 从 [OpenHarmony 开发者门户](https://developer.harmonyos.com/cn/develop/deveco-studio) 下载
2. 通过 DevEco Studio 安装 OpenHarmony SDK
3. 配置 SDK 路径

**所需工具：**

```bash
# 安装 Node.js（用于 OHPM）
# 从 nodejs.org 下载

# 安装 OHPM (OpenHarmony 包管理器)
npm install -g @ohos/hpm-cli

# 验证安装
node --version
ohpm --version
```

**环境设置：**

```bash
# 设置 OHOS SDK 路径
export OHOS_SDK_HOME="/path/to/ohos-sdk"
export PATH="$OHOS_SDK_HOME/native/build-tools/cmake/bin:$PATH"

# 验证
cmake --version
```

### 方式 2：Docker 构建（任意操作系统）

在任何操作系统上使用 Docker 构建 OpenHarmony 库。

**所需条件：**
- 已安装并运行 Docker Desktop
- 4GB+ 磁盘空间用于 Docker 镜像

**优势：**
- 可在任何操作系统上构建
- 无需 DevEco Studio
- 一致的构建环境
- 预配置的 OHOS SDK

**限制：**
- 无法在 OpenHarmony 设备上运行
- 无法使用 DevEco Studio 集成
- 首次下载较大（~1.5GB 镜像）

详见 [Docker 构建](#docker-构建) 部分。

## 快速开始

### 基础构建

```bash
# 为默认架构构建 (arm64-v8a)
ccgo build ohos

# 使用 Docker 构建（从任意操作系统交叉编译）
ccgo build ohos --docker

# 指定架构
ccgo build ohos --arch armeabi-v7a      # 32位 ARM
ccgo build ohos --arch arm64-v8a        # 64位 ARM（默认）
ccgo build ohos --arch x86_64           # x86_64 模拟器

# 构建多个架构
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64

# 构建类型
ccgo build ohos --build-type debug      # Debug 构建
ccgo build ohos --build-type release    # Release 构建（默认）

# 链接类型
ccgo build ohos --link-type static      # 仅静态库
ccgo build ohos --link-type shared      # 仅共享库
ccgo build ohos --link-type both        # 两种类型（默认）

# 生成 HAR 包
ccgo build ohos --har                   # 创建 .har 包
```

### 生成 DevEco Studio 项目

```bash
# 生成 DevEco Studio 项目
ccgo build ohos --ide-project

# 在 DevEco Studio 中打开
# 文件 -> 打开 -> cmake_build/ohos/
```

## 输出结构

### 默认输出 (`target/ohos/`)

```
target/ohos/
├── MyLib_OHOS_SDK-1.0.0.zip            # 主包
│   ├── lib/
│   │   ├── static/
│   │   │   ├── armeabi-v7a/
│   │   │   │   └── libmylib.a          # 32位 ARM 静态库
│   │   │   ├── arm64-v8a/
│   │   │   │   └── libmylib.a          # 64位 ARM 静态库
│   │   │   └── x86_64/
│   │   │       └── libmylib.a          # x86_64 静态库
│   │   └── shared/
│   │       ├── armeabi-v7a/
│   │       │   └── libmylib.so         # 32位 ARM 共享库
│   │       ├── arm64-v8a/
│   │       │   └── libmylib.so         # 64位 ARM 共享库
│   │       └── x86_64/
│   │           └── libmylib.so         # x86_64 共享库
│   ├── haars/
│   │   └── mylib-1.0.0.har             # HAR 包
│   ├── include/
│   │   └── mylib/                      # 头文件
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                 # 构建元数据
│
└── MyLib_OHOS_SDK-1.0.0-SYMBOLS.zip    # 调试符号
    └── obj/
        ├── armeabi-v7a/
        │   └── libmylib.so             # 未剥离的库
        ├── arm64-v8a/
        │   └── libmylib.so
        └── x86_64/
            └── libmylib.so
```

### HAR 包

HAR (Harmony Archive) 是 OpenHarmony 的库包格式：

**结构：**
```
mylib-1.0.0.har
├── libs/
│   ├── armeabi-v7a/
│   │   └── libmylib.so
│   ├── arm64-v8a/
│   │   └── libmylib.so
│   └── x86_64/
│       └── libmylib.so
├── include/
│   └── mylib/
│       ├── mylib.h
│       └── version.h
├── oh-package.json5               # 包元数据
└── module.json5                   # 模块配置
```

**oh-package.json5:**
```json5
{
  "name": "mylib",
  "version": "1.0.0",
  "description": "My OpenHarmony library",
  "main": "index.ets",
  "author": "Your Name",
  "license": "MIT",
  "dependencies": {},
  "devDependencies": {}
}
```

### 构建元数据

`build_info.json` 包含：

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My OpenHarmony library"
  },
  "build": {
    "platform": "ohos",
    "architectures": ["armeabi-v7a", "arm64-v8a", "x86_64"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "ohos_sdk_version": "10",
    "api_version": "10"
  },
  "outputs": {
    "libraries": {
      "static": {
        "armeabi-v7a": "lib/static/armeabi-v7a/libmylib.a",
        "arm64-v8a": "lib/static/arm64-v8a/libmylib.a",
        "x86_64": "lib/static/x86_64/libmylib.a"
      },
      "shared": {
        "armeabi-v7a": "lib/shared/armeabi-v7a/libmylib.so",
        "arm64-v8a": "lib/shared/arm64-v8a/libmylib.so",
        "x86_64": "lib/shared/x86_64/libmylib.so"
      }
    },
    "har": "haars/mylib-1.0.0.har",
    "headers": "include/mylib/"
  }
}
```

## 在 OpenHarmony 中使用库

### 在 ArkTS/eTS 应用中

**1. 添加 HAR 依赖：**

```json5
// oh-package.json5
{
  "dependencies": {
    "mylib": "file:../mylib-1.0.0.har"
  }
}
```

**2. 创建原生模块包装器：**

```cpp
// src/native/mylib_napi.cpp
#include <napi/native_api.h>
#include <mylib/mylib.h>

static napi_value DoWork(napi_env env, napi_callback_info info) {
    mylib::MyClass obj;
    obj.do_work();

    napi_value result;
    napi_create_int32(env, 0, &result);
    return result;
}

EXTERN_C_START
static napi_value Init(napi_env env, napi_value exports) {
    napi_property_descriptor desc[] = {
        { "doWork", nullptr, DoWork, nullptr, nullptr, nullptr, napi_default, nullptr }
    };
    napi_define_properties(env, exports, sizeof(desc) / sizeof(desc[0]), desc);
    return exports;
}
EXTERN_C_END

static napi_module myLibModule = {
    .nm_version = 1,
    .nm_flags = 0,
    .nm_filename = nullptr,
    .nm_register_func = Init,
    .nm_modname = "mylib",
    .nm_priv = nullptr,
    .reserved = { 0 },
};

extern "C" __attribute__((constructor)) void RegisterMyLibModule() {
    napi_module_register(&myLibModule);
}
```

**3. 在 ArkTS 中使用：**

```typescript
// src/main/ets/pages/Index.ets
import mylib from 'libmylib.so';

@Entry
@Component
struct Index {
  build() {
    Button('Do Work')
      .onClick(() => {
        mylib.doWork();
      })
  }
}
```

### 在 C++ 应用中

**CMakeLists.txt:**

```cmake
# 链接静态库
target_link_libraries(myapp PRIVATE
    ${CMAKE_SOURCE_DIR}/libs/${OHOS_ARCH}/libmylib.a
)

target_include_directories(myapp PRIVATE
    ${CMAKE_SOURCE_DIR}/include
)
```

**直接使用：**

```cpp
#include <mylib/mylib.h>

int main() {
    mylib::MyClass obj;
    obj.do_work();
    return 0;
}
```

## Docker 构建

在任何操作系统上使用 Docker 构建 OpenHarmony 库：

### 前置条件

```bash
# 安装 Docker Desktop
# 下载地址：https://www.docker.com/products/docker-desktop/

# 验证 Docker 运行
docker ps
```

### 使用 Docker 构建

```bash
# 首次构建会下载预构建镜像（~1.5GB）
ccgo build ohos --docker

# 后续构建很快
ccgo build ohos --docker --arch arm64-v8a

# 所有标准选项都可用
ccgo build ohos --docker --arch armeabi-v7a,arm64-v8a --har
```

### 工作原理

1. CCGO 使用 Docker Hub 的预构建 `ccgo-builder-ohos` 镜像
2. 项目目录挂载到容器中
3. 使用 OHOS SDK 和工具链构建
4. 输出写入主机文件系统

### 限制

- **无法运行**：Docker 中没有 OpenHarmony 运行时
- **无法测试**：无法访问设备或模拟器
- **无 DevEco Studio**：无法生成 IDE 项目

## 发布到 OHPM

OHPM (OpenHarmony Package Manager) 是官方包仓库。

### 设置

```bash
# 登录 OHPM
ohpm login

# 配置仓库（如果使用私有仓库）
ohpm config set registry https://your-registry.com
```

### 发布

```bash
# 发布 HAR 到官方 OHPM 仓库
ccgo publish ohos --registry official

# 发布到私有仓库
ccgo publish ohos --registry private --url https://your-registry.com

# 跳过构建并发布现有 HAR
ccgo publish ohos --skip-build
```

### 包配置

确保 `CCGO.toml` 有正确的元数据：

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My OpenHarmony library"
authors = ["Your Name <your.email@example.com>"]
license = "MIT"
homepage = "https://github.com/yourusername/mylib"
repository = "https://github.com/yourusername/mylib"

[ohos]
min_api_version = 9
target_api_version = 10
```

## 平台配置

### CCGO.toml 设置

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"                      # static、shared 或 both

[build]
cpp_standard = "17"                # C++ 标准

[ohos]
min_api_version = 9                # 最低 API 版本
target_api_version = 10            # 目标 API 版本
compile_sdk_version = 10           # 编译 SDK 版本
ndk_version = "4.0.0"              # NDK 版本
```

### CMake 变量

为 OpenHarmony 构建时：

```cmake
${PLATFORM}                        # "ohos"
${OHOS_ARCH}                       # "armeabi-v7a"、"arm64-v8a" 或 "x86_64"
${BUILD_TYPE}                      # "Debug" 或 "Release"
${LINK_TYPE}                       # "static"、"shared" 或 "both"
${OHOS_API_VERSION}                # 目标 API 版本
${OHOS_SDK_HOME}                   # OHOS SDK 路径
```

### 条件编译

```cpp
// 平台检测
#ifdef __OHOS__
    // OpenHarmony 特定代码
    #include <hilog/log.h>

    #ifdef __aarch64__
        // ARM64 特定代码
    #elif defined(__arm__)
        // ARMv7 特定代码
    #elif defined(__x86_64__)
        // x86_64 特定代码
    #endif
#endif

// API 版本检测
#if __OHOS_API_VERSION__ >= 10
    // API 10+ 特性
#else
    // 旧版 API 的回退
#endif

// 日志记录
#ifdef __OHOS__
    #define LOG_TAG "MyLib"
    #define LOG_INFO(fmt, ...) \
        OH_LOG_INFO(LOG_APP, fmt, ##__VA_ARGS__)
#else
    #define LOG_INFO(fmt, ...) \
        printf(fmt "\n", ##__VA_ARGS__)
#endif
```

## 最佳实践

### 1. 使用 HAR 包

将库打包为 HAR 以便轻松分发：

```bash
# 始终生成 HAR
ccgo build ohos --har
```

### 2. 支持多个架构

为所有常见架构构建：

```bash
# 构建所有架构
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64
```

### 3. 实现 NAPI 包装器

为 C++ 代码提供 ArkTS/eTS 绑定：

```cpp
// 始终使用 NAPI 包装原生代码
static napi_value ExportFunction(napi_env env, napi_callback_info info) {
    // 实现
}
```

### 4. 使用 HiLog 进行日志记录

使用 OpenHarmony 的日志系统：

```cpp
#include <hilog/log.h>

#define LOG_DOMAIN 0x0001
#define LOG_TAG "MyLib"

void log_message() {
    OH_LOG_INFO(LOG_APP, "Message from MyLib");
}
```

### 5. 处理 API 版本控制

在运行时检查 API 版本：

```cpp
#include <parameter/system_parameter.h>

int get_api_version() {
    char value[32];
    int ret = GetParameter("const.ohos.apiversion", "", value, sizeof(value));
    if (ret > 0) {
        return atoi(value);
    }
    return 0;
}
```

### 6. 在真实设备上测试

始终在物理 OpenHarmony 设备上测试：
- 不同的 API 版本（9、10、11+）
- 不同的架构（ARM32、ARM64）
- 不同的 OEM 和设备类型

### 7. 最小化库大小

减小 HAR 大小以便更快下载：

```toml
[build]
cxxflags = ["-Os", "-flto"]        # 优化大小
strip_symbols = true               # 剥离调试符号
```

## 故障排除

### 找不到 OHOS SDK

```
Error: OHOS SDK not found
```

**解决方案：**

```bash
# 设置 OHOS SDK 路径
export OHOS_SDK_HOME="/path/to/ohos-sdk"

# 或在 CCGO.toml 中
[ohos]
sdk_path = "/path/to/ohos-sdk"

# 验证
ls $OHOS_SDK_HOME/native
```

### HAR 导入失败

```
Error: Failed to import HAR package
```

**解决方案：**

1. **检查 oh-package.json5：**
```json5
{
  "dependencies": {
    "mylib": "file:../path/to/mylib-1.0.0.har"  // 使用正确的路径
  }
}
```

2. **验证 HAR 结构：**
```bash
unzip -l mylib-1.0.0.har
# 应包含：libs/、include/、oh-package.json5
```

3. **重新安装依赖：**
```bash
ohpm install
```

### 找不到 NAPI 符号

```
Error: Cannot find module 'libmylib.so'
```

**解决方案：**

1. **检查库是否在 HAR 中：**
```bash
unzip -l mylib-1.0.0.har | grep libmylib.so
```

2. **验证模块名称匹配：**
```cpp
// 在 NAPI 代码中
.nm_modname = "mylib",  // 必须匹配导入名称
```

3. **检查 build.gradle：**
```groovy
externalNativeBuild {
    cmake {
        targets "mylib"  // 模块名称
    }
}
```

### API 版本不匹配

```
Error: Minimum API version not met
```

**解决方案：**

更新设备或降低最低 API 版本：

```toml
[ohos]
min_api_version = 9  # 降低以支持更多设备
```

### 不支持的架构

```
Error: No native library found for architecture
```

**解决方案：**

为缺失的架构构建：

```bash
# 构建所有架构
ccgo build ohos --arch armeabi-v7a,arm64-v8a,x86_64
```

## 性能提示

### 1. 使用 ARM NEON 指令

为 ARM 启用 NEON 优化：

```toml
[build]
cxxflags = ["-mfpu=neon", "-mfloat-abi=softfp"]  # ARMv7
# ARM64 默认启用 NEON
```

### 2. 链接时优化

```toml
[build]
cxxflags = ["-flto"]
ldflags = ["-flto"]
```

### 3. 优化大小

OpenHarmony 设备通常存储有限：

```toml
[build]
cxxflags = ["-Os", "-ffunction-sections", "-fdata-sections"]
ldflags = ["-Wl,--gc-sections"]
```

### 4. 静态链接以提高性能

静态库可能更快：

```bash
ccgo build ohos --link-type static
```

### 5. 在设备上进行性能分析

使用 HiPerf 进行性能分析：

```bash
# 在设备上
hiperfcmd start -o perf.data
# 运行应用
hiperfcmd stop
hiperfcmd report -i perf.data
```

## 迁移指南

### 从原生 C++ 模块

**之前：**
```cpp
// 独立 C++ 模块
namespace mylib {
    void do_work();
}
```

**之后：**

1. 创建 CCGO 项目：
```bash
ccgo new mylib
```

2. 添加 NAPI 包装器：
```cpp
// 添加 NAPI 绑定
static napi_value DoWork(napi_env env, napi_callback_info info) {
    mylib::do_work();
    return nullptr;
}
```

3. 构建 HAR：
```bash
ccgo build ohos --har
```

### 从 Android NDK 库

与 Android 有许多相似之处：

**差异：**
- 使用 HAR 而不是 AAR
- 使用 OHPM 而不是 Maven
- 使用 NAPI 而不是 JNI
- 不同的构建系统（Hvigor vs Gradle）

**迁移步骤：**

1. 将 C++ 代码复制到 CCGO 项目
2. 更新构建配置：
```toml
[ohos]
min_api_version = 9
```

3. 用 NAPI 替换 JNI：
```cpp
// JNI
JNIEXPORT jint JNICALL Java_com_example_MyLib_doWork(JNIEnv* env, jobject obj)

// NAPI
static napi_value DoWork(napi_env env, napi_callback_info info)
```

4. 构建：
```bash
ccgo build ohos --har
```

## 高级主题

### 多模块 HAR

创建包含多个模块的 HAR：

```
mylib/
├── core/              # 核心模块
│   ├── src/
│   └── include/
├── ui/                # UI 模块
│   ├── src/
│   └── include/
└── CCGO.toml
```

```toml
[package]
name = "mylib"

[[modules]]
name = "core"
path = "core"

[[modules]]
name = "ui"
path = "ui"
dependencies = ["core"]
```

### 嵌入资源

在 HAR 中包含资源：

```
mylib-1.0.0.har
├── libs/
├── include/
├── resources/
│   ├── base/
│   │   └── element/
│   └── rawfile/
│       └── data.bin
└── oh-package.json5
```

### 代码签名

为分发签名 HAR：

```bash
# 生成密钥
hapsigner generate-keypair -keyAlias mylib -keyAlg RSA

# 签名 HAR
hapsigner sign-app -mode localSign \
    -keyAlias mylib \
    -signAlg SHA256withRSA \
    -inputFile mylib-1.0.0.har \
    -outputFile mylib-1.0.0-signed.har
```

### 混淆

保护 C++ 代码：

```toml
[build]
cxxflags = ["-fvisibility=hidden", "-ffunction-sections"]
strip_symbols = true
```

## 另请参阅

- [构建系统](../features/build-system.md)
- [依赖管理](../features/dependency-management.md)
- [发布管理](../features/publishing.md)
- [Docker 构建](../features/docker-builds.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [平台概述](index.md)
