# CLI 参考

CCGO 的完整命令行参考。

## 全局选项

这些选项适用于所有命令：

```bash
ccgo [全局选项] <命令> [命令选项]
```

| 选项 | 描述 |
|------|------|
| `-h, --help` | 打印帮助信息 |
| `-V, --version` | 打印版本信息 |
| `--verbose` | 启用详细输出 |
| `--quiet` | 抑制非错误输出 |
| `--color <WHEN>` | 控制彩色输出：auto, always, never |

## 命令概览

| 命令 | 描述 |
|------|------|
| [new](#new) | 创建新项目 |
| [init](#init) | 在现有项目中初始化 CCGO |
| [build](#build) | 为目标平台构建 |
| [test](#test) | 运行测试 |
| [bench](#bench) | 运行基准测试 |
| [doc](#doc) | 生成文档 |
| [install](#install) | 安装依赖 |
| [update](#update) | 更新依赖 |
| [vendor](#vendor) | 本地 vendor 依赖 |
| [add](#add) | 添加依赖 |
| [remove](#remove) | 删除依赖 |
| [publish](#publish) | 发布库 |
| [check](#check) | 检查平台要求 |
| [clean](#clean) | 清理构建产物 |
| [tag](#tag) | 创建版本标签 |
| [package](#package) | 打包源码分发 |
| [run](#run) | 构建并运行示例/二进制 |
| [ci](#ci) | 运行 CI 构建流水线 |

---

## new

从模板创建新的 CCGO 项目。

```bash
ccgo new [选项] <名称>
```

### 参数

- `<名称>` - 项目名称（必填）

### 选项

| 选项 | 描述 |
|------|------|
| `--template <URL>` | 自定义 Copier 模板 URL |
| `--defaults` | 使用默认值，不提示 |
| `--vcs-ref <REF>` | 模板 git 引用（分支/标签）|

### 示例

```bash
# 使用交互式提示创建新项目
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
ccgo init [选项]
```

### 选项

| 选项 | 描述 |
|------|------|
| `--template <URL>` | 自定义 Copier 模板 URL |
| `--defaults` | 使用默认值，不提示 |
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

为指定平台构建项目。

```bash
ccgo build [选项] [平台]
```

### 参数

- `[平台]` - 目标平台：android, ios, macos, windows, linux, ohos, watchos, tvos, kmp

### 选项

| 选项 | 描述 |
|------|------|
| `--arch <ARCH>` | 目标架构，逗号分隔 |
| `--link-type <TYPE>` | 链接类型：static, shared, both（默认：both）|
| `--toolchain <TOOL>` | 工具链：msvc, mingw, auto（仅 Windows）|
| `--docker` | 使用 Docker 构建 |
| `--ide-project` | 生成 IDE 项目文件 |
| `--release` | 以 release 模式构建 |
| `--debug` | 以 debug 模式构建（默认）|
| `--clean` | 构建前清理 |

### 平台特定架构

**Android**: armeabi-v7a, arm64-v8a, x86, x86_64
**iOS**: armv7, arm64, x86_64 (sim), arm64 (sim)
**macOS**: x86_64, arm64
**Windows**: x86, x86_64
**Linux**: x86_64, aarch64
**OpenHarmony**: armeabi-v7a, arm64-v8a, x86_64

### 示例

```bash
# 为当前平台构建
ccgo build

# 构建多架构 Android
ccgo build android --arch arm64-v8a,armeabi-v7a

# 以 release 模式构建 iOS
ccgo build ios --release

# 使用 Docker 构建 Windows（MSVC）
ccgo build windows --toolchain msvc --docker

# 构建并生成 Xcode 项目
ccgo build ios --ide-project

# 清理构建
ccgo build linux --clean
```

---

## test

运行项目测试。

```bash
ccgo test [选项] [平台]
```

### 参数

- `[平台]` - 测试平台（默认：当前）

### 选项

| 选项 | 描述 |
|------|------|
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
ccgo bench [选项] [平台]
```

### 参数

- `[平台]` - 基准测试平台（默认：当前）

### 选项

| 选项 | 描述 |
|------|------|
| `--filter <PATTERN>` | 运行匹配模式的基准测试 |
| `--baseline <NAME>` | 保存/对比基线 |

### 示例

```bash
# 运行所有基准测试
ccgo bench

# 运行特定基准测试
ccgo bench --filter bench_name

# 保存基线
ccgo bench --baseline main

# 对比基线
ccgo bench --baseline main
```

---

## doc

生成项目文档。

```bash
ccgo doc [选项]
```

### 选项

| 选项 | 描述 |
|------|------|
| `--open` | 在浏览器中打开文档 |
| `--no-deps` | 不包括依赖 |
| `--format <FMT>` | 输出格式：html, markdown |

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
ccgo install [选项]
```

### 选项

| 选项 | 描述 |
|------|------|
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
ccgo update [选项] [依赖]
```

### 参数

- `[依赖]` - 要更新的特定依赖（可选）

### 选项

| 选项 | 描述 |
|------|------|
| `--dry-run` | 显示将要更新的内容 |

### 示例

```bash
# 更新所有依赖
ccgo update

# 更新特定依赖
ccgo update spdlog

# 试运行
ccgo update --dry-run
```

---

## vendor

将所有依赖复制到 vendor/ 目录。

```bash
ccgo vendor [选项]
```

### 选项

| 选项 | 描述 |
|------|------|
| `--no-delete` | 不删除现有 vendor 目录 |

### 示例

```bash
# Vendor 依赖
ccgo vendor
```

---

## add

向 CCGO.toml 添加依赖。

```bash
ccgo add [选项] <依赖>
```

### 参数

- `<依赖>` - 依赖规范

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
|------|------|
| `--git <URL>` | Git 仓库 URL |
| `--tag <TAG>` | Git 标签 |
| `--branch <BRANCH>` | Git 分支 |
| `--rev <REV>` | Git 修订版 |
| `--path <PATH>` | 本地路径 |

### 示例

```bash
# 从 Git 添加（带标签）
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# 从本地路径添加
ccgo add mylib --path ../mylib
```

---

## remove

从 CCGO.toml 删除依赖。

```bash
ccgo remove <依赖>
```

### 参数

- `<依赖>` - 依赖名称

### 示例

```bash
# 删除依赖
ccgo remove spdlog
```

---

## publish

将库发布到注册表。

```bash
ccgo publish [选项] <平台>
```

### 参数

- `<平台>` - 发布平台：android, ios, macos, apple, ohos, conan, kmp

### 选项

| 选项 | 描述 |
|------|------|
| `--registry <TYPE>` | 注册表：local, official, private |
| `--url <URL>` | 自定义注册表 URL（仅 private）|
| `--skip-build` | 使用现有构建产物 |
| `--manager <MGR>` | 包管理器（apple）：cocoapods, spm, all |
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

检查是否满足平台要求。

```bash
ccgo check [选项] [平台]
```

### 参数

- `[平台]` - 要检查的平台（默认：全部）

### 选项

| 选项 | 描述 |
|------|------|
| `--verbose` | 显示详细检查结果 |

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
ccgo clean [选项]
```

### 选项

| 选项 | 描述 |
|------|------|
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
ccgo tag [选项] [版本]
```

### 参数

- `[版本]` - 版本标签（默认：来自 CCGO.toml）

### 选项

| 选项 | 描述 |
|------|------|
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

打包源码以供分发。

```bash
ccgo package [选项]
```

### 选项

| 选项 | 描述 |
|------|------|
| `--format <FMT>` | 包格式：tar.gz, zip（默认：tar.gz）|
| `--output <PATH>` | 输出路径 |

### 示例

```bash
# 创建源码包
ccgo package

# 创建 ZIP 包
ccgo package --format zip

# 自定义输出路径
ccgo package --output /path/to/output
```

---

## run

构建并运行示例或二进制。

```bash
ccgo run [选项] <目标>
```

### 参数

- `<目标>` - 示例或二进制名称

### 选项

| 选项 | 描述 |
|------|------|
| `--example` | 作为示例运行（如果 TARGET 在 examples/ 中，则为默认）|
| `--bin` | 作为二进制运行 |
| `--release` | 以 release 模式构建 |
| `-- <ARGS>` | 传递给程序的参数 |

### 示例

```bash
# 运行示例
ccgo run my_example

# 带参数运行
ccgo run my_example -- --arg1 value1

# 以 release 模式运行
ccgo run my_example --release
```

---

## ci

运行 CI 构建流水线。

```bash
ccgo ci [选项]
```

读取 `CI_BUILD_*` 环境变量以确定构建什么。

### 环境变量

- `CI_BUILD_ANDROID` - 如果设置，构建 Android
- `CI_BUILD_IOS` - 如果设置，构建 iOS
- `CI_BUILD_MACOS` - 如果设置，构建 macOS
- `CI_BUILD_WINDOWS` - 如果设置，构建 Windows
- `CI_BUILD_LINUX` - 如果设置，构建 Linux
- `CI_BUILD_OHOS` - 如果设置，构建 OpenHarmony

### 示例

```bash
# 基于环境运行 CI 构建
export CI_BUILD_ANDROID=1
export CI_BUILD_IOS=1
ccgo ci
```

---

## 环境变量

CCGO 遵守以下环境变量：

| 变量 | 描述 |
|------|------|
| `CCGO_HOME` | CCGO 主目录（默认：~/.ccgo）|
| `ANDROID_HOME` | Android SDK 路径 |
| `ANDROID_NDK` | Android NDK 路径 |
| `OHOS_SDK_HOME` | OpenHarmony SDK 路径 |
| `CCGO_CMAKE_DIR` | 自定义 CMake 脚本目录 |
| `NO_COLOR` | 禁用彩色输出 |

---

## 退出代码

| 代码 | 含义 |
|------|------|
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
ccgo <命令> --help

# 示例
ccgo build --help
ccgo publish --help
```

更多信息，请参阅：
- [CCGO.toml 参考](ccgo-toml.md)
- [构建系统](../features/build-system.md)
- [依赖管理](../features/dependency-management.md)
