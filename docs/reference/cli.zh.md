# CLI 命令参考

所有 CCGO 命令的完整命令行参考。

## 概述

CCGO 为 C++ 跨平台开发提供了全面的 CLI 工具，具有快速启动时间和零 Python 依赖（Rust 实现）。

## 全局选项

适用于所有命令：

```bash
ccgo [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS]
```

| 选项 | 描述 |
|--------|-------------|
| `-h, --help` | 打印帮助信息 |
| `-V, --version` | 打印版本信息 |
| `-v, --verbose` | 启用详细输出 |
| `--no-color` | 禁用彩色终端输出 |

## 命令概览

| 分类 | 命令 | 描述 |
|----------|---------|-------------|
| **项目** | [new](#new) | 创建新项目 |
| | [init](#init) | 在现有项目中初始化 CCGO |
| **构建** | [build](#build) | 为目标平台构建 |
| | [run](#run) | 构建并运行示例/二进制文件 |
| | [test](#test) | 运行 GoogleTest 单元测试 |
| | [bench](#bench) | 运行 Google Benchmark 基准测试 |
| | [doc](#doc) | 生成 Doxygen 文档 |
| **依赖** | [install](#install) | 从 CCGO.toml 安装依赖 |
| | [add](#add) | 添加依赖 |
| | [remove](#remove) | 移除依赖 |
| | [update](#update) | 更新依赖到最新兼容版本 |
| | [vendor](#vendor) | 将依赖复制到 vendor/ 目录以供离线构建 |
| | [tree](#tree) | 显示依赖树 |
| **发现** | [search](#search) | 搜索包 |
| | [collection](#collection) | 管理包集合 |
| **发布** | [publish](#publish) | 发布到包管理器 |
| | [package](#package) | 打包 SDK 以供分发 |
| **维护** | [check](#check) | 检查平台要求 |
| | [clean](#clean) | 清理构建产物 |
| | [tag](#tag) | 创建版本标签 |

---

## new

从模板创建新的 CCGO 项目。

```bash
ccgo new [OPTIONS] <NAME>
```

### 参数

- `<NAME>` - 项目名称（必需）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--template <URL>` | 自定义 Copier 模板 URL |
| `--defaults` | 使用默认值，不进行交互提示 |
| `--vcs-ref <REF>` | 模板 git 引用（分支/标签）|

### 示例

```bash
# 使用交互提示创建新项目
ccgo new myproject

# 使用默认值创建
ccgo new myproject --defaults

# 使用自定义模板
ccgo new myproject --template https://github.com/user/template

# 使用特定模板版本
ccgo new myproject --vcs-ref v2.0.0
```

---

## init

在现有项目中初始化 CCGO。

```bash
ccgo init [OPTIONS]
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--template <URL>` | 自定义 Copier 模板 URL |
| `--defaults` | 使用默认值，不进行交互提示 |
| `--force` | 覆盖现有文件 |

### 示例

```bash
# 在当前目录初始化
ccgo init

# 强制覆盖现有文件
ccgo init --force
```

---

## build

为特定平台构建 C++ 库。

```bash
ccgo build [OPTIONS] <TARGET>
```

### 参数

- `<TARGET>` - 构建目标（必需）

**可用目标：**

| 目标 | 描述 |
|--------|-------------|
| `all` | 所有支持的平台 |
| `apple` | 所有苹果平台（iOS、macOS、watchOS、tvOS）|
| `android` | Android（基于 NDK）|
| `ios` | iOS（arm64、模拟器）|
| `macos` | macOS（x86_64、arm64 通用二进制）|
| `windows` | Windows（MSVC/MinGW）|
| `linux` | Linux（GCC/Clang）|
| `ohos` | OpenHarmony（基于 Hvigor）|
| `watchos` | Apple Watch |
| `tvos` | Apple TV |
| `kmp` | Kotlin 多平台 |
| `conan` | Conan 包 |

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--arch <ARCH>` | 目标架构（逗号分隔）|
| `--link-type <TYPE>` | `static`、`shared` 或 `both`（默认：`both`）|
| `--docker` | 使用 Docker 容器构建 |
| `--auto-docker` | 当本地构建不可用时自动检测并使用 Docker |
| `-j, --jobs <N>` | 并行构建作业数 |
| `--ide-project` | 生成 IDE 项目文件（Xcode、Visual Studio 等）|
| `--release` | 以发布模式构建（默认：调试）|
| `--native-only` | 仅构建本地库，跳过 AAR/HAR 打包 |
| `--toolchain <TOOL>` | Windows 工具链：`msvc`、`mingw`、`auto`（默认：`auto`）|
| `--dev` | 开发模式（在 Docker 中使用 GitHub 上的预构建 ccgo）|
| `-F, --features <FEATURES>` | 启用特性（逗号分隔）|
| `--no-default-features` | 禁用默认特性 |
| `--all-features` | 启用所有可用特性 |

### 平台特定架构

**Android：**
- `armeabi-v7a` - ARM 32 位
- `arm64-v8a` - ARM 64 位（默认）
- `x86` - Intel 32 位（模拟器）
- `x86_64` - Intel 64 位（模拟器）

**iOS：**
- `arm64` - iPhone/iPad（默认）
- `simulator` - 模拟器（自动检测主机架构）

**macOS：**
- `x86_64` - Intel Mac
- `arm64` - Apple Silicon
- 默认创建通用二进制（两种架构）

**Windows：**
- `x86_64` - 64 位（默认）

**Linux：**
- `x86_64` - 64 位（默认）
- `aarch64` - ARM 64 位

**OpenHarmony：**
- `armeabi-v7a` - ARM 32 位
- `arm64-v8a` - ARM 64 位（默认）
- `x86_64` - Intel 64 位

### 特性系统

使用特性控制条件编译：

```bash
# 启用特定特性
ccgo build android --features networking,ssl

# 禁用默认特性并仅启用特定特性
ccgo build android --features minimal --no-default-features

# 启用所有特性
ccgo build android --all-features
```

特性在 `CCGO.toml` 中定义：

```toml
[features]
default = ["networking"]
networking = []
ssl = ["networking"]
advanced = ["networking", "ssl"]
```

### Docker 构建

使用 Docker 从任何操作系统构建任何平台：

```bash
# 显式使用 Docker
ccgo build linux --docker
ccgo build windows --docker
ccgo build android --docker

# 在需要时自动检测（例如，从 macOS 构建 Linux）
ccgo build linux --auto-docker
```

**优势：**
- 通用交叉编译
- 无需本地工具链安装
- 一致的构建环境
- Docker Hub 上的预构建镜像

### 示例

```bash
# 使用特定架构构建 Android
ccgo build android --arch arm64-v8a,armeabi-v7a

# 以发布模式构建 iOS
ccgo build ios --release

# 为所有苹果平台构建
ccgo build apple --release

# 仅使用 MSVC 工具链构建 Windows
ccgo build windows --toolchain msvc

# 使用 Docker 构建 Linux（从 macOS/Windows）
ccgo build linux --docker

# 使用并行作业构建
ccgo build android -j 8

# 构建并生成 Xcode 项目
ccgo build ios --ide-project

# 使用特定特性构建
ccgo build android --features networking,ssl --release

# 构建所有平台
ccgo build all --release
```

### 构建输出

构建输出到 `target/<platform>/`，采用统一的归档结构：

```
target/android/
└── mylib_Android_SDK-1.0.0.zip
    ├── lib/
    │   ├── static/arm64-v8a/libmylib.a
    │   └── shared/arm64-v8a/libmylib.so
    ├── haars/mylib.aar
    ├── include/mylib/
    └── build_info.json
```

---

## test

运行项目测试。

```bash
ccgo test [OPTIONS] [PLATFORM]
```

### 参数

- `[PLATFORM]` - 测试平台（默认：当前）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--filter <PATTERN>` | 运行匹配模式的测试 |
| `--verbose` | 详细测试输出 |
| `--no-fail-fast` | 失败后继续运行测试 |

### 示例

```bash
# 运行所有测试
ccgo test

# 运行特定测试
ccgo test --filter test_name

# 详细运行测试
ccgo test --verbose
```

---

## bench

运行项目基准测试。

```bash
ccgo bench [OPTIONS] [PLATFORM]
```

### 参数

- `[PLATFORM]` - 基准测试平台（默认：当前）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--filter <PATTERN>` | 运行匹配模式的基准测试 |
| `--baseline <NAME>` | 保存/比较基线 |

### 示例

```bash
# 运行所有基准测试
ccgo bench

# 运行特定基准测试
ccgo bench --filter bench_name

# 保存基线
ccgo bench --baseline main

# 与基线比较
ccgo bench --baseline main
```

---

## doc

生成项目文档。

```bash
ccgo doc [OPTIONS]
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--open` | 在浏览器中打开文档 |
| `--no-deps` | 不包含依赖 |
| `--format <FMT>` | 输出格式：html、markdown |

### 示例

```bash
# 生成并打开文档
ccgo doc --open

# 生成 markdown 文档
ccgo doc --format markdown
```

---

## install

安装项目依赖。

```bash
ccgo install [OPTIONS]
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--locked` | 使用 CCGO.lock 中的精确版本 |
| `--offline` | 仅使用 vendored 依赖 |

### 示例

```bash
# 安装依赖
ccgo install

# 使用锁定版本安装
ccgo install --locked

# 离线安装
ccgo install --offline
```

---

## update

将依赖更新到最新兼容版本。

```bash
ccgo update [OPTIONS] [DEPENDENCY]
```

### 参数

- `[DEPENDENCY]` - 要更新的特定依赖（可选）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--dry-run` | 显示将要更新的内容 |

### 示例

```bash
# 更新所有依赖
ccgo update

# 更新特定依赖
ccgo update spdlog

# 预演
ccgo update --dry-run
```

---

## vendor

将所有依赖复制到 vendor/ 目录以供离线构建。

```bash
ccgo vendor [OPTIONS]
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--no-delete` | 不删除现有 vendor 目录 |

### 示例

```bash
# Vendor 所有依赖
ccgo vendor

# 保留现有 vendor 目录
ccgo vendor --no-delete
```

---

## tree

显示项目依赖树及各种可视化选项。

```bash
ccgo tree [OPTIONS] [PACKAGE]
```

### 参数

- `[PACKAGE]` - 显示特定包的依赖（可选）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `-d, --depth <DEPTH>` | 显示的最大深度（默认：无限制）|
| `--no-dedupe` | 不去重复的依赖 |
| `-l, --locked` | 从锁文件显示依赖版本 |
| `-f, --format <FORMAT>` | 输出格式：text、json、dot（默认：text）|
| `--duplicates` | 仅显示重复的依赖 |
| `-i, --invert <PACKAGE>` | 显示依赖此包的包 |
| `--conflicts` | 高亮版本冲突 |

### 输出格式

**Text**（默认）：
- 使用框绘字符的树状可视化
- 显示依赖层次结构
- 用 `(*)` 标记重复项

**JSON**：
- 包含版本和源信息的结构化数据
- 包含冲突检测结果
- 适合程序化处理

**DOT**（Graphviz）：
- 图形可视化格式
- 以红色高亮冲突
- 可使用 `dot` 命令渲染

### 示例

```bash
# 显示完整依赖树
ccgo tree

# 限制深度为 2 级
ccgo tree --depth 2

# 显示特定包的依赖
ccgo tree spdlog

# 显示所有出现（不去重）
ccgo tree --no-dedupe

# 输出为 JSON
ccgo tree --format json

# 生成 Graphviz 图表
ccgo tree --format dot > deps.dot
dot -Tpng deps.dot -o deps.png

# 显示反向依赖
ccgo tree --invert fmt

# 高亮版本冲突
ccgo tree --conflicts

# 仅显示重复的依赖
ccgo tree --duplicates

# 使用 CCGO.toml.lock 中的锁定版本
ccgo tree --locked
```

---

## search

在已订阅的集合中搜索包。

```bash
ccgo search <QUERY> [OPTIONS]
```

### 参数

- `<QUERY>` - 搜索关键词或模式（必需）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `-c, --collection <NAME>` | 仅在特定集合中搜索 |
| `-d, --details` | 显示详细的包信息 |
| `--limit <N>` | 限制结果数量（默认：20）|

### 示例

```bash
# 在所有集合中搜索包
ccgo search json

# 在特定集合中搜索
ccgo search logging --collection official

# 显示详细信息
ccgo search crypto --details

# 限制结果为 50
ccgo search lib --limit 50

# 组合选项
ccgo search network --collection community --details --limit 10
```

### 搜索结果

结果包括：
- 包名和版本
- 摘要描述
- 源集合
- 仓库 URL（使用 --details）
- 支持的平台（使用 --details）
- 许可证信息（使用 --details）
- 关键词（使用 --details）

---

## collection

管理用于发现的包集合。

```bash
ccgo collection <SUBCOMMAND> [OPTIONS]
```

### 子命令

| 子命令 | 描述 |
|------------|-------------|
| `add <URL>` | 添加新集合 |
| `list` | 列出所有已订阅的集合 |
| `remove <NAME>` | 移除集合 |
| `refresh [NAME]` | 刷新集合 |

### collection add

添加新的包集合。

```bash
ccgo collection add <URL>
```

**支持的 URL 方案：**
- `file://` - 本地文件路径
- `http://` - HTTP URL
- `https://` - HTTPS URL

**参数：**
- `<URL>` - 集合 URL（必需）

**示例：**
```bash
# 添加官方集合
ccgo collection add https://ccgo.dev/collections/official.json

# 添加社区集合
ccgo collection add https://ccgo.dev/collections/community.json

# 添加本地集合
ccgo collection add file:///path/to/my-collection.json

# 从 HTTP 添加
ccgo collection add http://example.com/packages.json
```

### collection list

列出所有已订阅的集合。

```bash
ccgo collection list [OPTIONS]
```

**选项：**

| 选项 | 描述 |
|--------|-------------|
| `-d, --details` | 显示详细信息 |

**示例：**
```bash
# 列出集合
ccgo collection list

# 显示详细信息
ccgo collection list --details
```

### collection remove

移除已订阅的集合。

```bash
ccgo collection remove <NAME_OR_URL>
```

**参数：**
- `<NAME_OR_URL>` - 集合名称或 URL（必需）

**示例：**
```bash
# 按名称移除
ccgo collection remove official

# 按 URL 移除
ccgo collection remove https://ccgo.dev/collections/community.json
```

### collection refresh

刷新集合数据。

```bash
ccgo collection refresh [NAME]
```

**参数：**
- `[NAME]` - 要刷新的集合名称（可选，默认：全部）

**示例：**
```bash
# 刷新所有集合
ccgo collection refresh

# 刷新特定集合
ccgo collection refresh official
```

---

## add

向 CCGO.toml 添加依赖。

```bash
ccgo add [OPTIONS] <DEPENDENCY>
```

### 参数

- `<DEPENDENCY>` - 依赖规范

### 依赖格式

```bash
# Git 仓库
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# 本地路径
ccgo add mylib --path ../mylib

# 注册表（未来）
ccgo add fmt@10.1.1
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--git <URL>` | Git 仓库 URL |
| `--tag <TAG>` | Git 标签 |
| `--branch <BRANCH>` | Git 分支 |
| `--rev <REV>` | Git 修订版本 |
| `--path <PATH>` | 本地路径 |

### 示例

```bash
# 从 Git 使用标签添加
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# 从本地路径添加
ccgo add mylib --path ../mylib
```

---

## remove

从 CCGO.toml 移除依赖。

```bash
ccgo remove <DEPENDENCY>
```

### 参数

- `<DEPENDENCY>` - 依赖名称

### 示例

```bash
# 移除依赖
ccgo remove spdlog
```

---

## publish

发布库到注册表。

```bash
ccgo publish [OPTIONS] <PLATFORM>
```

### 参数

- `<PLATFORM>` - 要发布的平台：android、ios、macos、apple、ohos、conan、kmp

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--registry <TYPE>` | 注册表：local、official、private |
| `--url <URL>` | 自定义注册表 URL（仅 private）|
| `--skip-build` | 使用现有构建产物 |
| `--manager <MGR>` | 包管理器（apple）：cocoapods、spm、all |
| `--push` | 推送 git 标签（仅 SPM）|
| `--remote-name <NAME>` | Git 远程名称（默认：origin）|

### 示例

```bash
# 发布 Android 到 Maven Local
ccgo publish android --registry local

# 发布到 Maven Central
ccgo publish android --registry official

# 发布 iOS 到 CocoaPods
ccgo publish apple --manager cocoapods

# 发布到 SPM 并推送 git
ccgo publish apple --manager spm --push

# 发布到私有 Maven
ccgo publish android --registry private --url https://maven.example.com
```

---

## check

检查平台要求是否满足。

```bash
ccgo check [OPTIONS] [PLATFORM]
```

### 参数

- `[PLATFORM]` - 要检查的平台（默认：全部）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--verbose` | 显示详细的检查结果 |

### 示例

```bash
# 检查所有平台
ccgo check

# 检查特定平台
ccgo check android --verbose
```

---

## clean

清理构建产物。

```bash
ccgo clean [OPTIONS]
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--dry-run` | 显示将要删除的内容 |
| `-y, --yes` | 跳过确认提示 |

### 示例

```bash
# 预览将要删除的内容
ccgo clean --dry-run

# 无确认清理
ccgo clean -y
```

---

## tag

从 CCGO.toml 创建版本标签。

```bash
ccgo tag [OPTIONS] [VERSION]
```

### 参数

- `[VERSION]` - 版本标签（默认：从 CCGO.toml）

### 选项

| 选项 | 描述 |
|--------|-------------|
| `-m, --message <MSG>` | 标签消息 |
| `--push` | 推送标签到远程 |

### 示例

```bash
# 从 CCGO.toml 版本创建标签
ccgo tag

# 创建自定义版本标签
ccgo tag v2.0.0 -m "Release 2.0.0"

# 创建并推送标签
ccgo tag --push
```

---

## package

打包源代码以供分发。

```bash
ccgo package [OPTIONS]
```

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--format <FMT>` | 包格式：tar.gz、zip（默认：tar.gz）|
| `--output <PATH>` | 输出路径 |

### 示例

```bash
# 创建源代码包
ccgo package

# 创建 ZIP 包
ccgo package --format zip

# 自定义输出路径
ccgo package --output /path/to/output
```

---

## run

构建并运行示例或二进制文件。

```bash
ccgo run [OPTIONS] <TARGET>
```

### 参数

- `<TARGET>` - 示例或二进制文件名称

### 选项

| 选项 | 描述 |
|--------|-------------|
| `--example` | 作为示例运行（如果 TARGET 在 examples/ 中则为默认）|
| `--bin` | 作为二进制文件运行 |
| `--release` | 以发布模式构建 |
| `-- <ARGS>` | 传递给程序的参数 |

### 示例

```bash
# 运行示例
ccgo run my_example

# 使用参数运行
ccgo run my_example -- --arg1 value1

# 以发布模式运行
ccgo run my_example --release
```

---

## 环境变量

CCGO 支持以下环境变量：

| 变量 | 描述 |
|----------|-------------|
| `CCGO_HOME` | CCGO 主目录（默认：~/.ccgo）|
| `ANDROID_HOME` | Android SDK 路径 |
| `ANDROID_NDK` | Android NDK 路径 |
| `OHOS_SDK_HOME` | OpenHarmony SDK 路径 |
| `CCGO_CMAKE_DIR` | 自定义 CMake 脚本目录 |
| `NO_COLOR` | 禁用彩色输出 |

---

## 退出代码

| 代码 | 含义 |
|------|---------|
| 0 | 成功 |
| 1 | 一般错误 |
| 2 | 配置错误 |
| 3 | 构建错误 |
| 4 | 依赖错误 |
| 101 | 用户取消操作 |

---

## 获取帮助

```bash
# 获取任何命令的帮助
ccgo <command> --help

# 示例
ccgo build --help
ccgo publish --help
```

更多信息，请参阅：
- [CCGO.toml 参考](ccgo-toml.zh.md)
- [构建系统](../features/build-system.zh.md)
- [依赖管理](../features/dependency-management.zh.md)
