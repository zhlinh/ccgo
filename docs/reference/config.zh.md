# CCGO.toml 配置参考

> 版本：v3.1.0 | 更新时间：2026-01-21

本文档提供 `CCGO.toml` 配置文件的完整参考，该文件控制 CCGO 中 C++ 跨平台项目的所有方面。

## 目录

1. [概述](#概述)
2. [文件结构](#文件结构)
3. [Package 部分](#package-部分)
4. [Workspace 部分](#workspace-部分)
5. [依赖项](#依赖项)
6. [特性](#特性)
7. [构建配置](#构建配置)
8. [平台配置](#平台配置)
9. [二进制和示例目标](#二进制和示例目标)
10. [发布配置](#发布配置)
11. [完整示例](#完整示例)

---

## 概述

CCGO 使用基于 TOML 的配置文件，类似于 Rust 项目的 Cargo.toml。`CCGO.toml` 文件应放置在项目的根目录中。

### 基本要求

每个 `CCGO.toml` 必须包含以下**至少一项**：
- `[package]` 部分 - 用于单个包/库
- `[workspace]` 部分 - 用于管理多个相关包

配置可以同时包含两个部分（工作区根目录也是一个包）。

---

## 文件结构

### 最小包配置

```toml
[package]
name = "mylib"
version = "1.0.0"
```

### 最小工作区配置

```toml
[workspace]
members = ["core", "utils"]
```

---

## Package 部分

`[package]` 部分定义单个包的元数据。

### 字段

| 字段 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `name` | 字符串 | **是** | 包名称（必须是有效的 C++ 标识符）|
| `version` | 字符串 | **是** | 语义化版本（例如 "1.0.0"）|
| `description` | 字符串 | 否 | 包的简要描述 |
| `authors` | 字符串数组 | 否 | 作者列表 |
| `license` | 字符串 | 否 | SPDX 许可证标识符（例如 "MIT"、"Apache-2.0"）|
| `repository` | 字符串 | 否 | Git 仓库 URL |

### 示例

```toml
[package]
name = "mylib"
version = "1.2.3"
description = "我的超棒 C++ 库"
authors = ["张三 <zhangsan@example.com>", "李四"]
license = "MIT"
repository = "https://github.com/user/mylib"
```

### 旧版别名

为了向后兼容，`[project]` 被接受为 `[package]` 的别名：

```toml
[project]  # 与 [package] 相同
name = "mylib"
version = "1.0.0"
```

---

## Workspace 部分

`[workspace]` 部分使管理多个相关包成为可能。

### 字段

| 字段 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `members` | 字符串数组 | **是** | 工作区成员路径（支持 glob 模式）|
| `exclude` | 字符串数组 | 否 | 要排除的路径 |
| `resolver` | 字符串 | 否 | 依赖解析器版本（"1" 或 "2"，默认："1"）|
| `default_members` | 字符串数组 | 否 | 工作区命令的默认成员 |

### 工作区依赖

可以定义工作区级别的依赖并由成员继承：

```toml
[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
```

### 成员路径模式

`members` 数组支持 glob 模式：

```toml
[workspace]
members = [
    "core",              # 精确路径
    "utils",             # 精确路径
    "examples/*",        # 所有直接子目录
    "plugins/**"         # 所有子目录（递归）
]
exclude = ["examples/deprecated"]
```

### 解析器版本

- **"1"**（默认）：旧版解析器
- **"2"**：新解析器，具有更好的特性统一和冲突解决

### 完整工作区示例

```toml
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/old"]
resolver = "2"
default_members = ["core", "utils"]

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]

[[workspace.dependencies]]
name = "spdlog"
version = "^1.12"
```

### 工作区成员继承

成员可以从工作区继承依赖：

**工作区根目录（CCGO.toml）：**
```toml
[workspace]
members = ["core"]

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]
```

**成员（core/CCGO.toml）：**
```toml
[package]
name = "mylib-core"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true           # 从工作区继承
features = ["extra"]       # 添加额外特性（与工作区特性合并）
```

解析后，成员的 fmt 依赖将具有：
- `version = "^10.0"`（来自工作区）
- `git = "..."`（来自工作区）
- `features = ["std", "extra"]`（已合并）

---

## 依赖项

依赖项使用 `[[dependencies]]` 定义为表数组。

### 依赖来源

CCGO 支持多种依赖来源：

#### 1. 基于版本的依赖（未来）

```toml
[[dependencies]]
name = "fmt"
version = "^10.0"  # 语义化版本要求
```

#### 2. Git 依赖

```toml
[[dependencies]]
name = "spdlog"
version = "^1.12"
git = "https://github.com/gabime/spdlog.git"
branch = "v1.x"    # 可选：特定分支
```

```toml
[[dependencies]]
name = "json"
version = "^3.11"
git = "https://github.com/nlohmann/json.git"
tag = "v3.11.2"    # 可选：特定标签
```

```toml
[[dependencies]]
name = "pinned"
version = "1.0.0"
git = "https://github.com/user/pinned.git"
rev = "abc123"     # 可选：特定提交哈希
```

#### 3. 路径依赖

```toml
[[dependencies]]
name = "local-utils"
version = "1.0.0"
path = "../utils"  # 相对或绝对路径
```

### 依赖字段

| 字段 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `name` | 字符串 | **是** | 依赖名称 |
| `version` | 字符串 | 条件 | 版本要求（除非 `workspace = true` 则必需）|
| `git` | 字符串 | 否 | Git 仓库 URL |
| `branch` | 字符串 | 否 | Git 分支名称 |
| `tag` | 字符串 | 否 | Git 标签 |
| `rev` | 字符串 | 否 | Git 修订版本（提交哈希）|
| `path` | 字符串 | 否 | 本地文件路径 |
| `optional` | 布尔值 | 否 | 依赖是否可选（默认：false）|
| `features` | 字符串数组 | 否 | 为此依赖启用的特性 |
| `default_features` | 布尔值 | 否 | 是否启用默认特性（默认：true）|
| `workspace` | 布尔值 | 否 | 从工作区依赖继承（默认：false）|

### 版本要求

CCGO 支持语义化版本范围：

| 语法 | 含义 | 示例 |
|------|------|------|
| `^1.2.3` | 与 1.2.3 兼容（>=1.2.3, <2.0.0）| `^10.0` |
| `~1.2.3` | 合理接近 1.2.3（>=1.2.3, <1.3.0）| `~1.12.0` |
| `>=1.2.3` | 大于或等于 | `>=1.0,<2.0` |
| `1.2.*` | 通配符版本 | `1.*` |
| `1.2.3` | 精确版本 | `10.2.1` |

### 可选依赖

可选依赖仅在特性启用时包含：

```toml
[[dependencies]]
name = "http-client"
version = "^1.0"
optional = true  # 仅在特性启用时包含

[features]
networking = ["http-client"]  # 启用可选依赖的特性
```

### 依赖特性

为依赖启用特定特性：

```toml
[[dependencies]]
name = "serde"
version = "^1.0"
features = ["derive", "std"]
default_features = false  # 禁用默认特性
```

### 工作区依赖继承

成员可以从工作区继承依赖：

```toml
[[dependencies]]
name = "fmt"
workspace = true              # 必需：从工作区继承
features = ["extra-feature"]  # 可选：添加额外特性
```

### 完整依赖示例

```toml
# 来自 git 的常规依赖
[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
tag = "10.2.1"

# 本地路径依赖
[[dependencies]]
name = "utils"
version = "1.0.0"
path = "../shared/utils"

# 特性的可选依赖
[[dependencies]]
name = "openssl"
version = "^1.1"
optional = true

# 带特性的依赖
[[dependencies]]
name = "spdlog"
version = "^1.12"
features = ["std", "fmt_external"]
default_features = false

# 工作区继承的依赖
[[dependencies]]
name = "googletest"
workspace = true
```

---

## 特性

`[features]` 部分定义条件编译和可选依赖。

### 默认特性

```toml
[features]
default = ["std"]  # 默认启用的特性
```

### 特性定义

特性可以依赖于：
1. 其他特性
2. 可选依赖名称
3. 依赖特性语法（`dep/feature`）

```toml
[features]
default = ["std"]
std = []                           # 空特性（只是一个标志）
networking = ["http-client"]       # 启用可选依赖
advanced = ["networking", "async"] # 依赖于其他特性
full = ["networking", "advanced"]  # 传递依赖
derive = ["serde/derive"]          # 在依赖中启用特性
```

### 特性解析

特性递归解析：

```toml
[features]
default = ["std"]
std = []
networking = ["http-client"]
advanced = ["networking"]  # 也启用 "http-client"
full = ["advanced"]        # 启用 "advanced"、"networking" 和 "http-client"
```

### 使用特性

#### 构建时启用特性：

```bash
ccgo build android --features networking,advanced
```

#### 禁用默认特性：

```bash
ccgo build ios --no-default-features
```

#### 启用特定特性而不启用默认特性：

```bash
ccgo build linux --no-default-features --features networking
```

### 完整特性示例

```toml
[features]
default = ["std"]
std = []
networking = ["http-client", "tls"]
async = ["async-runtime"]
full = ["networking", "async", "logging"]
logging = ["spdlog-dep"]

# 由特性启用的可选依赖
[[dependencies]]
name = "http-client"
version = "^1.0"
optional = true

[[dependencies]]
name = "async-runtime"
version = "^2.0"
optional = true

[[dependencies]]
name = "spdlog-dep"
version = "^1.12"
optional = true

# 依赖特性语法
[[dependencies]]
name = "serde"
version = "^1.0"

# 特性可以启用 serde 的 derive 特性
[features]
derive = ["serde/derive"]
```

---

## 构建配置

`[build]` 部分配置构建系统行为。

### 字段

| 字段 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `parallel` | 布尔值 | 否 | 启用并行构建（默认：false）|
| `jobs` | 整数 | 否 | 并行作业数 |
| `symbol_visibility` | 布尔值 | 否 | 符号可见性（默认：false 为隐藏）|
| `submodule_deps` | 表 | 否 | 用于共享链接的子模块内部依赖 |

### 示例

```toml
[build]
parallel = true
jobs = 4
symbol_visibility = false  # 默认隐藏

# 对于具有多个子模块/组件的项目
[build.submodule_deps]
api = ["base"]           # API 依赖于 base
feature = ["base", "core"]  # Feature 依赖于 base 和 core
```

`submodule_deps` 映射到 `CCGO_CONFIG_DEPS_MAP` CMake 变量用于共享库链接。

---

## 平台配置

平台特定配置在 `[platforms.<platform>]` 下定义。

### 支持的平台

- `android`
- `ios`
- `macos`
- `windows`
- `linux`
- `ohos`（OpenHarmony）

### Android 配置

```toml
[platforms.android]
min_sdk = 21                        # 最小 SDK 版本
architectures = [                    # 目标架构
    "armeabi-v7a",
    "arm64-v8a",
    "x86_64"
]
```

### iOS 配置

```toml
[platforms.ios]
min_version = "13.0"  # 最小 iOS 部署目标
```

### macOS 配置

```toml
[platforms.macos]
min_version = "10.15"  # 最小 macOS 部署目标
```

### Windows 配置

```toml
[platforms.windows]
toolchain = "auto"  # auto、msvc 或 mingw
```

工具链选项：
- `"auto"`：使用 MSVC 和 MinGW 构建（默认）
- `"msvc"`：仅 MSVC
- `"mingw"`：仅 MinGW

### Linux 配置

```toml
[platforms.linux]
architectures = ["x86_64", "aarch64"]
```

### OpenHarmony (OHOS) 配置

```toml
[platforms.ohos]
min_api = 9                          # 最小 API 级别
architectures = [
    "armeabi-v7a",
    "arm64-v8a"
]
```

### 完整平台示例

```toml
[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a", "x86_64"]

[platforms.ios]
min_version = "13.0"

[platforms.macos]
min_version = "10.15"

[platforms.windows]
toolchain = "auto"

[platforms.linux]
architectures = ["x86_64"]

[platforms.ohos]
min_api = 9
architectures = ["arm64-v8a"]
```

---

## 二进制和示例目标

### 二进制目标

使用 `[[bin]]` 定义可执行二进制文件：

```toml
[[bin]]
name = "my-cli"              # 二进制名称
path = "src/bin/cli.cpp"     # 主源文件路径

[[bin]]
name = "my-server"
path = "src/bin/server.cpp"
```

运行二进制文件：
```bash
ccgo run my-cli
ccgo run my-server -- --help
```

### 示例目标

使用 `[[example]]` 定义示例程序：

```toml
[[example]]
name = "basic-usage"
path = "examples/basic.cpp"  # 可选：默认为 examples/{name}.cpp

[[example]]
name = "advanced"
# path 默认为 examples/advanced.cpp 或 examples/advanced/main.cpp
```

运行示例：
```bash
ccgo run --example basic-usage
ccgo run --example advanced
```

---

## 发布配置

发布配置当前采用 Python 实现的旧版 `[project]` 格式。当 Rust 实现添加发布支持时，本部分将更新。

### Maven 发布（Android/KMP）

```toml
[publish.android.maven]
group_id = "com.example.mylib"
artifact_id = "mylib"  # 可选，默认为包名
channel_desc = ""       # 例如 "beta"、"release"

dependencies = [
    { group = "com.example", artifact = "dep1", version = "1.0.0" },
    { group = "com.example", artifact = "dep2", version = "2.0.0" }
]
```

### Apple 发布（CocoaPods/SPM）

```toml
[publish.apple]
pod_name = "MyLib"
platforms = ["ios", "macos"]
min_ios_version = "13.0"
min_macos_version = "10.15"
summary = "我的库描述"

[publish.apple.cocoapods]
enabled = true
repo = "trunk"  # 或 "private" 带 spec_repo URL
license = "MIT"
homepage = "https://github.com/user/mylib"
static_framework = true

[publish.apple.spm]
enabled = true
git_url = "https://github.com/user/mylib"
use_local_path = false
```

### OHOS 发布（OHPM）

```toml
[publish.ohos.ohpm]
registry = "official"  # official、private 或 local
dependencies = []
```

---

## 完整示例

### 单包库

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "我的 C++ 库"
authors = ["开发者 <dev@example.com>"]
license = "MIT"
repository = "https://github.com/user/mylib"

[[dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"

[[dependencies]]
name = "spdlog"
version = "^1.12"
optional = true

[features]
default = ["std"]
std = []
logging = ["spdlog"]

[build]
parallel = true
jobs = 4
symbol_visibility = false

[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a", "x86_64"]

[platforms.ios]
min_version = "13.0"
```

### 多包工作区

**工作区根目录（CCGO.toml）：**

```toml
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/old"]
resolver = "2"

[[workspace.dependencies]]
name = "fmt"
version = "^10.0"
git = "https://github.com/fmtlib/fmt.git"
features = ["std"]

[[workspace.dependencies]]
name = "googletest"
version = "^1.14"
git = "https://github.com/google/googletest.git"

# 可选：工作区根也可以是一个包
[package]
name = "myproject"
version = "1.0.0"
```

**成员包（core/CCGO.toml）：**

```toml
[package]
name = "myproject-core"
version = "1.0.0"
description = "核心库"

[[dependencies]]
name = "fmt"
workspace = true  # 从工作区继承

[[dependencies]]
name = "spdlog"
version = "^1.12"
git = "https://github.com/gabime/spdlog.git"
```

**成员包（utils/CCGO.toml）：**

```toml
[package]
name = "myproject-utils"
version = "1.0.0"
description = "实用工具库"

[[dependencies]]
name = "fmt"
workspace = true
features = ["chrono"]  # 向工作区依赖添加额外特性

[[dependencies]]
name = "myproject-core"
path = "../core"  # 依赖于另一个工作区成员
```

### 带二进制和示例的库

```toml
[package]
name = "advanced-lib"
version = "2.0.0"
description = "带 CLI 工具的高级 C++ 库"
license = "Apache-2.0"

[[dependencies]]
name = "fmt"
version = "^10.0"

[[dependencies]]
name = "argparse"
version = "^2.9"
optional = true

[features]
default = []
cli = ["argparse"]

[[bin]]
name = "mytool"
path = "src/bin/tool.cpp"

[[bin]]
name = "converter"
path = "src/bin/converter.cpp"

[[example]]
name = "basic"
# 默认为 examples/basic.cpp

[[example]]
name = "advanced"
path = "examples/advanced/main.cpp"

[build]
parallel = true

[platforms.android]
min_sdk = 21
architectures = ["arm64-v8a"]

[platforms.linux]
architectures = ["x86_64", "aarch64"]
```

---

## 版本迁移说明

### 从 Python CLI (v3.0) 到 Rust CLI (v3.1+)

CCGO.toml 格式的主要变化：

1. **部分名称：**
   - `[project]` 现在是 `[package]`（但 `[project]` 仍作为别名工作）

2. **新特性：**
   - 用于多包项目的 `[workspace]` 部分
   - 用于条件编译的 `[features]` 部分
   - 用于可执行文件的 `[[bin]]` 和 `[[example]]` 部分
   - 用于依赖继承的 `workspace = true`
   - 工作区配置中的 `resolver` 字段

3. **依赖项：**
   - 从字典格式更改为表数组（`[[dependencies]]`）
   - 添加了对工作区依赖继承的支持
   - 添加了 `optional`、`features`、`default_features` 字段

4. **平台配置：**
   - 移至 `[platforms.<name>]` 部分
   - 简化字段名称（例如 `min_sdk` 而不是 Android 特定名称）

5. **构建配置：**
   - 新增 `[build]` 部分
   - 添加了 `parallel`、`jobs`、`submodule_deps` 字段

---

## 最佳实践

1. **版本控制**：始终将 `CCGO.toml` 提交到版本控制
2. **语义化版本**：使用正确的 semver（MAJOR.MINOR.PATCH）
3. **包名称**：使用不带空格的小写名称（推荐 kebab-case）
4. **工作区组织**：
   - 对相关包使用工作区
   - 在工作区根目录定义共享依赖
   - 使用 `resolver = "2"` 以获得更好的依赖解析
5. **特性**：
   - 保持 `default` 特性最小化
   - 使用特性使依赖可选
   - 在 README 中记录特性
6. **平台支持**：仅配置实际支持的平台
7. **依赖项**：
   - 在生产中固定版本（git 依赖使用 `tag` 或 `rev`）
   - 在开发中使用 `^` 版本范围以获得灵活性
   - 使用锁文件（`CCGO.toml.lock`）进行可重现构建

---

## 验证

CCGO 在解析时验证 `CCGO.toml`。常见错误：

| 错误 | 原因 | 解决方案 |
|------|------|----------|
| "must contain either [package] or [workspace]" | 缺少两个部分 | 至少添加一个部分 |
| "Invalid version requirement" | semver 语法错误 | 修复版本字符串（例如 "^1.0.0"）|
| "Unknown feature" | 请求未定义的特性 | 检查 `[features]` 部分 |
| "workspace dependency not found" | `workspace = true` 但不在工作区依赖中 | 添加到 `[[workspace.dependencies]]` |
| "Failed to parse CCGO.toml" | TOML 语法错误 | 检查 TOML 语法 |

---

## 另请参阅

- [CLI 参考](cli.zh.md) - CCGO 命令行界面
- [快速入门](../getting-started/quickstart.zh.md) - 快速入门指南
- [项目结构](../getting-started/project-structure.zh.md) - 项目组织结构
- [依赖管理](../features/dependency-management.zh.md) - 管理依赖
