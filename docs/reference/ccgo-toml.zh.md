# CCGO.toml 参考

CCGO 项目的完整配置参考。

## 概述

`CCGO.toml` 是定义项目元数据、依赖、构建设置和平台特定配置的清单文件。它使用 [TOML](https://toml.io/) 格式编写。

## 文件位置

`CCGO.toml` 文件必须位于项目目录的根目录。

## 基本结构

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[build]
cpp_standard = "17"

[android]
min_sdk_version = 21
```

---

## [package]

定义包元数据。

### 必填字段

| 字段 | 类型 | 描述 |
|------|------|------|
| `name` | 字符串 | 包名称（小写、字母数字、连字符）|
| `version` | 字符串 | 语义化版本（例如 "1.0.0"）|

### 可选字段

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `authors` | 字符串数组 | 包作者 | `[]` |
| `description` | 字符串 | 简短描述 | `""` |
| `license` | 字符串 | 许可证标识符（如 "MIT"、"Apache-2.0"）| `""` |
| `repository` | 字符串 | 源代码仓库 URL | `""` |
| `homepage` | 字符串 | 项目主页 URL | `""` |
| `documentation` | 字符串 | 文档 URL | `""` |
| `keywords` | 字符串数组 | 搜索关键词 | `[]` |
| `categories` | 字符串数组 | 包分类 | `[]` |
| `readme` | 字符串 | README 文件路径 | `"README.md"` |

### 示例

```toml
[package]
name = "awesome-cpp-lib"
version = "2.1.3"
authors = ["Alice <alice@example.com>", "Bob <bob@example.com>"]
description = "一个很棒的 C++ 库"
license = "MIT"
repository = "https://github.com/user/awesome-cpp-lib"
homepage = "https://awesome-cpp-lib.dev"
keywords = ["networking", "async", "performance"]
categories = ["network-programming", "asynchronous"]
```

---

## [library]

定义库构建配置。

### 字段

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `type` | 字符串 | 库类型：`"static"`、`"shared"`、`"both"` | `"both"` |
| `namespace` | 字符串 | 库的 C++ 命名空间 | 包名称 |
| `output_name` | 字符串 | 自定义库输出名称 | 包名称 |
| `crate_type` | 字符串数组 | 输出类型（兼容性）| `["staticlib", "cdylib"]` |

### 示例

```toml
[library]
type = "both"              # 构建静态和动态库
namespace = "mylib"        # C++ 命名空间
output_name = "mylib-cpp"  # 输出文件：libmylib-cpp.a, libmylib-cpp.so
```

---

## [dependencies]

定义项目依赖。

### 依赖源

#### Git 仓库

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", branch = "master" }
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280" }
```

**字段：**
- `git`（必填）：Git 仓库 URL
- `tag`：Git 标签（与 `branch` 和 `rev` 互斥）
- `branch`：Git 分支（与 `tag` 和 `rev` 互斥）
- `rev`：Git 提交哈希（与 `tag` 和 `branch` 互斥）

#### 本地路径

```toml
[dependencies]
mylib = { path = "../mylib" }
utils = { path = "./libs/utils" }
```

**字段：**
- `path`（必填）：依赖的相对或绝对路径

#### 注册表（未来）

```toml
[dependencies]
fmt = "10.1.1"              # 精确版本
spdlog = "^1.12.0"          # 兼容版本（>=1.12.0, <2.0.0）
boost = "~1.80"             # 次要版本（>=1.80.0, <1.81.0）
```

### 版本要求

| 语法 | 含义 | 示例 |
|------|------|------|
| `"1.2.3"` | 精确版本 | `"1.2.3"` 仅匹配 1.2.3 |
| `"^1.2.3"` | 兼容版本 | `"^1.2.3"` 匹配 >=1.2.3, <2.0.0 |
| `"~1.2.3"` | 次要版本 | `"~1.2.3"` 匹配 >=1.2.3, <1.3.0 |
| `">=1.2.3"` | 大于或等于 | `">=1.2.3"` 匹配 >=1.2.3 |
| `">1.2.3"` | 大于 | `">1.2.3"` 匹配 >1.2.3 |
| `"<=1.2.3"` | 小于或等于 | `"<=1.2.3"` 匹配 <=1.2.3 |
| `"<1.2.3"` | 小于 | `"<1.2.3"` 匹配 <1.2.3 |

### 可选依赖

```toml
[dependencies]
required = { git = "https://github.com/user/required.git", tag = "v1.0.0" }

[dependencies.optional]
networking = { git = "https://github.com/user/network.git", tag = "v2.0.0" }
database = { git = "https://github.com/user/db.git", tag = "v3.0.0" }
```

通过特性启用可选依赖：

```toml
[features]
network = ["networking"]
db = ["database"]
full = ["network", "db"]
```

### 平台特定依赖

```toml
[dependencies]
common = { git = "https://github.com/user/common.git", tag = "v1.0.0" }

[target.'cfg(target_os = "android")'.dependencies]
android-specific = { path = "./android-lib" }

[target.'cfg(target_os = "ios")'.dependencies]
ios-specific = { path = "./ios-lib" }
```

---

## [build]

定义构建配置。

### 字段

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cpp_standard` | 字符串 | C++ 标准："11"、"14"、"17"、"20"、"23" | `"17"` |
| `cmake_minimum_version` | 字符串 | 最低 CMake 版本 | `"3.18"` |
| `compile_flags` | 字符串数组 | 额外的编译器标志 | `[]` |
| `link_flags` | 字符串数组 | 额外的链接器标志 | `[]` |
| `definitions` | 表 | 预处理器定义 | `{}` |
| `include_dirs` | 字符串数组 | 额外的包含目录 | `[]` |
| `link_dirs` | 字符串数组 | 额外的库搜索路径 | `[]` |
| `system_libs` | 字符串数组 | 要链接的系统库 | `[]` |
| `cmake_file` | 字符串 | configure 时包含的 cmake 脚本路径（相对于项目根目录）。省略则自动发现 `CCGO.cmake`；设为 `""` 可完全禁用 cmake 文件包含。各平台可通过 `[platforms.X.build] cmake_file` 覆盖。 | 自动发现 |

### 示例

```toml
[build]
cpp_standard = "20"
cmake_minimum_version = "3.20"
compile_flags = ["-Wall", "-Wextra", "-Werror"]
link_flags = ["-flto"]

[build.definitions]
DEBUG_MODE = "1"
APP_VERSION = "\"1.0.0\""
ENABLE_LOGGING = true

[build]
include_dirs = ["third_party/include"]
link_dirs = ["third_party/lib"]
system_libs = ["pthread", "dl"]
```

### [build.cmake]

在 configure 阶段注入的额外 CMake 参数和编译器标志。这些标志追加在所有内部 CCGO 标志**之后**，因此可以覆盖或扩展默认值。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `arguments` | 字符串数组 | 直接传递给 cmake configure 的原始参数 | `[]` |
| `c_flags` | 字符串数组 | 追加到 `CMAKE_C_FLAGS` 的标志 | `[]` |
| `cpp_flags` | 字符串数组 | 追加到 `CMAKE_CXX_FLAGS` 的标志 | `[]` |

```toml
[build.cmake]
arguments = ["-DCMAKE_VERBOSE_MAKEFILE=ON", "-DENABLE_FEATURE=1"]
c_flags   = ["-D__STDC_FORMAT_MACROS"]
cpp_flags = ["-fexceptions", "-frtti"]
```

平台专属的 cmake 覆盖可在 `[platforms.<name>.build.cmake]` 下设置（见各平台章节）。全局与平台特定的列表会拼接；平台值追加在全局值之后。

---

## 平台配置

### [platforms.android]

Android 特定配置。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `min_sdk` | 整数 | 最低 Android API 级别 | `21` |
| `architectures` | 字符串数组 | 目标架构 | 全部 |
| `default_dep_linkage` | 字符串 | 默认依赖链接方式 | — |
| `dep_linkage_on_shared` | 字符串 | 消费者构建动态库时的依赖链接方式 | — |
| `dep_linkage_on_static` | 字符串 | 消费者构建静态库时的依赖链接方式 | — |

**链接方式取值：** `"shared-external"` | `"static-embedded"` | `"static-external"`

```toml
[platforms.android]
min_sdk        = 21
architectures  = ["arm64-v8a", "armeabi-v7a"]
default_dep_linkage = "static-embedded"
```

#### [platforms.android.build]

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cmake_file` | 字符串 | Android 专用 cmake 脚本（相对于项目根目录）。`""` 可禁用 Android 的 cmake 文件包含。 | — |

#### [platforms.android.build.cmake]

仅应用于 Android 构建的 CMake 标志，追加在 `[build.cmake]` 之后。

```toml
[platforms.android.build.cmake]
arguments = ["-DANDROID_ARM_NEON=TRUE", "-DANDROID_TOOLCHAIN=clang"]
cpp_flags = ["-fexceptions", "-frtti"]
```

### [platforms.ios]

iOS 特定配置。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `min_version` | 字符串 | 最低 iOS 部署目标 | `"12.0"` |
| `architectures` | 字符串数组 | 目标架构 | 全部 |
| `default_dep_linkage` | 字符串 | 默认依赖链接方式 | — |
| `dep_linkage_on_shared` | 字符串 | 消费者构建动态库时的依赖链接方式 | — |
| `dep_linkage_on_static` | 字符串 | 消费者构建静态库时的依赖链接方式 | — |

```toml
[platforms.ios]
min_version   = "13.0"
architectures = ["arm64"]
```

#### [platforms.ios.build]

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cmake_file` | 字符串 | iOS 专用 cmake 脚本。`""` 可禁用 iOS 的 cmake 文件包含。 | — |

#### [platforms.ios.build.cmake]

```toml
[platforms.ios.build.cmake]
arguments = ["-DIOS_SPECIFIC=1"]
cpp_flags = []
```

### [platforms.macos]

macOS 特定配置。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `min_version` | 字符串 | 最低 macOS 部署目标 | `"10.15"` |
| `architectures` | 字符串数组 | 目标架构 | `["x86_64", "arm64"]` |
| `default_dep_linkage` | 字符串 | 默认依赖链接方式 | — |
| `dep_linkage_on_shared` | 字符串 | 消费者构建动态库时的依赖链接方式 | — |
| `dep_linkage_on_static` | 字符串 | 消费者构建静态库时的依赖链接方式 | — |

```toml
[platforms.macos]
min_version   = "11.0"
architectures = ["arm64", "x86_64"]  # 通用二进制
```

#### [platforms.macos.build]

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cmake_file` | 字符串 | macOS 专用 cmake 脚本。`""` 可禁用 macOS 的 cmake 文件包含。 | — |

#### [platforms.macos.build.cmake]

```toml
[platforms.macos.build.cmake]
arguments = []
cpp_flags = []
```

### [platforms.windows]

Windows 特定配置。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `architectures` | 字符串数组 | 目标架构 | `["x86_64"]` |
| `default_dep_linkage` | 字符串 | 默认依赖链接方式 | — |
| `dep_linkage_on_shared` | 字符串 | 消费者构建动态库时的依赖链接方式 | — |
| `dep_linkage_on_static` | 字符串 | 消费者构建静态库时的依赖链接方式 | — |

```toml
[platforms.windows]
architectures = ["x86_64", "x86"]
```

#### [platforms.windows.build]

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cmake_file` | 字符串 | Windows 专用 cmake 脚本。`""` 可禁用 Windows 的 cmake 文件包含。 | — |

#### [platforms.windows.build.cmake]

```toml
[platforms.windows.build.cmake]
arguments = ["-DCMAKE_WINDOWS_EXPORT_ALL_SYMBOLS=ON"]
cpp_flags = []
```

### [platforms.linux]

Linux 特定配置。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `architectures` | 字符串数组 | 目标架构 | `["x86_64"]` |
| `default_dep_linkage` | 字符串 | 默认依赖链接方式 | — |
| `dep_linkage_on_shared` | 字符串 | 消费者构建动态库时的依赖链接方式 | — |
| `dep_linkage_on_static` | 字符串 | 消费者构建静态库时的依赖链接方式 | — |

```toml
[platforms.linux]
architectures = ["x86_64", "aarch64"]
```

#### [platforms.linux.build]

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cmake_file` | 字符串 | Linux 专用 cmake 脚本。`""` 可禁用 Linux 的 cmake 文件包含。 | — |

#### [platforms.linux.build.cmake]

```toml
[platforms.linux.build.cmake]
arguments = []
cpp_flags = []
```

### [platforms.ohos]

OpenHarmony 特定配置。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `min_api` | 整数 | 最低 OpenHarmony API 级别 | `9` |
| `architectures` | 字符串数组 | 目标架构 | 全部 |
| `default_dep_linkage` | 字符串 | 默认依赖链接方式 | — |
| `dep_linkage_on_shared` | 字符串 | 消费者构建动态库时的依赖链接方式 | — |
| `dep_linkage_on_static` | 字符串 | 消费者构建静态库时的依赖链接方式 | — |

```toml
[platforms.ohos]
min_api       = 9
architectures = ["arm64-v8a", "armeabi-v7a"]
```

#### [platforms.ohos.build]

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `cmake_file` | 字符串 | OpenHarmony 专用 cmake 脚本。`""` 可禁用 OHOS 的 cmake 文件包含。 | — |

#### [platforms.ohos.build.cmake]

```toml
[platforms.ohos.build.cmake]
arguments = []
cpp_flags = []
```

---

## [features]

定义条件编译特性。

```toml
[features]
default = ["feature1"]          # 默认特性
feature1 = []                   # 简单特性
feature2 = ["dependency1"]      # 启用可选依赖
full = ["feature1", "feature2"] # 复合特性
```

### 示例

```toml
[dependencies.optional]
networking = { git = "https://github.com/user/network.git", tag = "v1.0.0" }
logging = { git = "https://github.com/user/logging.git", tag = "v2.0.0" }

[features]
default = ["basic"]
basic = []
network = ["networking"]
debug = ["logging"]
full = ["basic", "network", "debug"]
```

构建时启用特性：

```bash
ccgo build --features network,debug
ccgo build --all-features
ccgo build --no-default-features
```

---

## [examples]

定义示例程序。

```toml
[[examples]]
name = "basic"
path = "examples/basic.cpp"

[[examples]]
name = "advanced"
path = "examples/advanced.cpp"
required_features = ["network"]
```

构建和运行示例：

```bash
ccgo run basic
ccgo run advanced --features network
```

---

## [bins]

定义二进制目标。

```toml
[[bins]]
name = "mytool"
path = "src/bin/mytool.cpp"

[[bins]]
name = "myapp"
path = "src/bin/myapp.cpp"
required_features = ["full"]
```

构建和运行二进制：

```bash
ccgo run mytool --bin
ccgo build --bin myapp --features full
```

---

## [profile.\<name\>]

命名构建 profile 存储可复用的配置片段，通过 `--profile <name>` 选择。

```bash
ccgo build android --profile sanitize
```

**内置 profile**（无需声明）：

| 名称 | 效果 |
|------|------|
| `debug` | `release = false` |
| `release` | `release = true` |

### 顶层字段

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `inherits` | 字符串 | 要继承的父 profile 名称（单继承） | — |
| `name` | 字符串 | 覆盖输出包名 | 包名 |
| `release` | 布尔值 | `true` = release 构建，`false` = debug 构建 | — |
| `link_type` | 字符串 | `"static"` \| `"shared"` \| `"both"` | — |
| `jobs` | 整数 | 并行构建任务数 | 自动 |

### [profile.\<name\>.cmake]

此 profile 激活时应用于所有平台的额外 CMake 标志。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `merge` | 字符串 | `"replace"` — 丢弃继承的标志；`"extend"` — 追加在继承标志之后 | `"replace"` |
| `arguments` | 字符串数组 | 直接传递给 cmake configure 的原始参数 | `[]` |
| `c_flags` | 字符串数组 | 追加到 `CMAKE_C_FLAGS` 的标志 | `[]` |
| `cpp_flags` | 字符串数组 | 追加到 `CMAKE_CXX_FLAGS` 的标志 | `[]` |

### [profile.\<name\>.features]

此 profile 激活时启用的 features。

| 字段 | 类型 | 描述 | 默认值 |
|------|------|------|--------|
| `merge` | 字符串 | `"replace"` 或 `"extend"` | `"replace"` |
| `list` | 字符串数组 | feature 名称列表 | `[]` |

### [profile.\<name\>.dep_linkage]

此 profile 激活时的依赖链接策略。

| 字段 | 类型 | 描述 |
|------|------|------|
| `default` | 字符串 | 适用于所有构建类型的链接方式 |
| `on_shared` | 字符串 | 消费者构建动态库时的覆盖值 |
| `on_static` | 字符串 | 消费者构建静态库时的覆盖值 |

**链接方式取值：** `"shared-external"` | `"static-embedded"` | `"static-external"`

### 平台级覆盖

在 profile 内部针对特定平台覆盖 cmake 标志或依赖链接方式。

```toml
[profile.<name>.platforms.<platform>.build.cmake]
merge     = "extend"
arguments = []
c_flags   = []
cpp_flags = []

[profile.<name>.platforms.<platform>.build.dep_linkage]
default   = "static-embedded"
on_shared = "shared-external"
on_static = "static-embedded"
```

支持的 `<platform>` 取值：`android`、`ios`、`macos`、`windows`、`linux`、`ohos`

### 继承与合并顺序

Profile 通过 `inherits` 支持单继承。支持继承链；循环引用会被检测并报告错误。

优先级顺序（后者覆盖前者）：
1. 全局 `CCGO.toml` 配置
2. 继承链（从最古老的祖先到最近的父 profile）
3. 当前激活 profile 的自身设置
4. CLI 标志（`--release`、`--link-type`、`--features`、`--jobs`）

CMake 标志通过四个层次累积：

```
[build.cmake]
  + [platforms.<p>.build.cmake]
  + [profile.<name>.cmake]
  + [profile.<name>.platforms.<p>.build.cmake]
```

### 示例

```toml
[profile.sanitize]
inherits  = "debug"
link_type = "both"

[profile.sanitize.cmake]
merge     = "extend"
c_flags   = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
cpp_flags = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]

[profile.sanitize.features]
merge = "extend"
list  = ["debug-logging"]

[profile.sanitize.platforms.android.build.cmake]
merge     = "extend"
cpp_flags = ["-fsanitize=address"]

[profile.fat-static]
inherits  = "release"
link_type = "static"

[profile.fat-static.dep_linkage]
default = "static-embedded"
```

---

## 完整示例

```toml
[package]
name = "mylib"
version = "1.0.0"
authors = ["Developer <dev@example.com>"]
description = "一个跨平台 C++ 库"
license = "MIT"
repository = "https://github.com/user/mylib"
homepage = "https://mylib.dev"
keywords = ["cpp", "cross-platform"]
categories = ["library"]

[library]
type = "both"
namespace = "mylib"
output_name = "mylib"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[dependencies.optional]
networking = { git = "https://github.com/user/network.git", tag = "v1.0.0" }

[build]
cpp_standard = "17"
cmake_minimum_version = "3.18"
compile_flags = ["-Wall", "-Wextra"]

[build.definitions]
APP_VERSION = "\"1.0.0\""

[features]
default = ["basic"]
basic = []
network = ["networking"]
full = ["basic", "network"]

[android]
min_sdk_version = 21
target_sdk_version = 33
architectures = ["arm64-v8a", "armeabi-v7a"]

[ios]
min_deployment_target = "12.0"
enable_bitcode = false

[windows]
toolchain = "auto"
msvc_runtime = "dynamic"

[[examples]]
name = "basic"
path = "examples/basic.cpp"

[[bins]]
name = "mytool"
path = "src/bin/mytool.cpp"
```

---

## 模式验证

CCGO 在每个命令上验证 `CCGO.toml`。常见错误：

### 无效的 TOML 语法

```toml
# 错误：缺少结束引号
name = "mylib

# 错误：无效的键格式
my-key = value
```

### 缺少必填字段

```toml
# 错误：缺少 'name' 字段
[package]
version = "1.0.0"
```

### 无效的版本格式

```toml
[package]
name = "mylib"
version = "1.0"        # 错误：必须是语义化版本（1.0.0）
```

### 冲突的 Git 引用

```toml
[dependencies]
# 错误：不能同时指定 'tag' 和 'branch'
lib = { git = "https://...", tag = "v1.0.0", branch = "main" }
```

---

## 环境变量扩展

CCGO 支持在字符串值中扩展环境变量：

```toml
[package]
version = "${VERSION:-1.0.0}"  # 如果未设置 VERSION，默认为 "1.0.0"

[dependencies]
mylib = { path = "${MYLIB_PATH:-../mylib}" }
```

语法：
- `${VAR}`：扩展变量（如果未设置则报错）
- `${VAR:-default}`：使用默认值扩展
- `$$`：字面 `$` 字符

---

## 最佳实践

### 版本控制

- 使用语义化版本（主版本.次版本.修订号）
- 在标记前更新版本：`ccgo tag`
- 保持 CHANGELOG.md 与版本同步

### 依赖管理

- 将依赖固定到特定标签/修订版本以确保可重现性
- 使用 `CCGO.lock` 进行精确的依赖解析
- 在 README 中记录依赖要求

### 平台配置

- 设置合理的最低版本以获得最大兼容性
- 在支持的最低平台版本上测试
- 记录平台特定要求

### 特性

- 为可选功能使用特性
- 保持默认特性最小化
- 在 README 中记录特性

### 构建设置

- 使 cpp_standard 与依赖匹配
- 在开发中使用警告标志（`-Wall -Wextra`）
- 避免在主配置中使用平台特定标志

---

## 迁移指南

### 从 CMakeLists.txt

```cmake
# CMakeLists.txt
project(mylib VERSION 1.0.0)
set(CMAKE_CXX_STANDARD 17)
find_package(spdlog REQUIRED)
```

变为：

```toml
# CCGO.toml
[package]
name = "mylib"
version = "1.0.0"

[build]
cpp_standard = "17"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

### 从 Conan

```ini
# conanfile.txt
[requires]
spdlog/1.12.0
fmt/10.1.1

[options]
spdlog:shared=False
```

变为：

```toml
# CCGO.toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[library]
type = "static"
```

---

## 另请参阅

- [CLI 参考](cli.zh.md)
- [构建系统](../features/build-system.zh.md)
- [构建 Profile](../features/build-profiles.zh.md)
- [依赖管理](../features/dependency-management.zh.md)
- [发布](../features/publishing.zh.md)
- [CCGO.toml.example](../../CCGO.toml.example)
