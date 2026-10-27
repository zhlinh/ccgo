# 包注册表

> v3.2.0 新增

CCGO 支持包注册表 —— 一种基于 Git 的轻量级包索引，无需中心服务器即可实现简化的依赖管理。

## 概述

借鉴 Swift Package Manager 的方式，CCGO 使用 Git 仓库作为包索引。这种设计：

- 无需服务器维护
- 复用既有 Git 基础设施
- 天然支持私有包
- 缓存后即可离线使用

## 两层模型

CCGO 把"有哪些包可用"和"包的字节存放在哪里"拆成两层：

**第一层 —— 发现（索引仓库）。** 一个 Git 仓库，里面的 JSON 文件列出每个
已发布的包及其版本。可在网页上浏览、用 `git log` 审计、用
`ccgo registry search` 查询。发布方每发一次新版就跑一次
`ccgo publish package` 往索引里追加新的 `VersionEntry`。索引就是 Git 里的
文本，所以代码评审、分支保护、签名提交都自然适用。

**第二层 —— 解析（产物归档）。** 每个 `VersionEntry` 可以记录一个
`archive_url`，指向打包好的构建产物（zip 或 tar.gz）—— 放在 CDN、
artifactory 或 release 页面 —— 还可以带上 SHA-256 `checksum`。当消费方的
`ccgo fetch` 通过索引解析一个仅写了 `version` 的依赖时，ccgo 会直接下载
那个归档、校验 checksum，再解压到 `.ccgo/deps/<name>/`。不传 git 历史，
clone 后无需再构建，消费侧不再编译源码。

消费方的 `CCGO.toml` 不再写 URL —— 只写 `version = "1.0.0"` 加可选的
`registry = "name"` 选择器。剩下的事由索引和 `[registries]` 配置一起完成。

## 配置

在项目的 `CCGO.toml` 中声明一个或多个注册表。map 的键是注册表名称，
值是索引仓库的 Git URL。

```toml
[registries]
mna = "git@git.example.com:org/ccgo-index.git"
public = "https://github.com/example-org/ccgo-index.git"

[[dependencies]]
name = "stdcomm"
version = "25.2.9519653"
registry = "mna"           # 显式选择器

[[dependencies]]
name = "fmt"
version = "10.2.1"
# 不写 registry = ... —— 这个包和版本必须只命中一个注册表
```

`[[dependencies]].registry` 字段是可选的。设置后，查找会被锁定到那一个
注册表（且如果名字不在 `[registries]` 里会报错）。不设置时，ccgo 检查
所有配置的注册表；多个来源同时命中会报错，需要显式指定 registry。
指定注册表后若版本未命中，会直接失败，不再回退到本地包缓存。

## 发布方 CI 工作流

`ccgo publish package` 是 **append-only、单版本一次** 的(对标 CocoaPods
`pod repo push`)。每次调用只往 index 里追加一条 `VersionEntry`,重
复发布同一版本会被拒绝。用 `--index-version` 和/或 `--index-tag` 指定
要发布的版本:

```bash
ccgo build all --release
ccgo package --release        # 产出 NAME_CCGO_PACKAGE-VERSION.zip
# 上传 zip 到你的 CDN/artifactory(你的脚本)
ccgo publish package \
  --index-repo git@example.com:org/index.git \
  --index-name org-index \
  --index-version 25.2.9519653 \
  --archive-url-template "https://artifacts.example.com/{name}/{name}_CCGO_PACKAGE-{version}.zip" \
  --checksum \
  --index-push
```

只传 `--index-tag v1.0.0` 时,version 会自动剥前缀(`v`/`V`)推导出
`1.0.0`。只传 `--index-version 1.0.0` 时,tag 默认补成 `v1.0.0`。对
不遵循 `v<version>` 约定的 tag(比如 `release-v1.0.0`,或 monorepo
前缀 `stdcomm-v1.0.0`),两个 flag 都显式传:

```bash
ccgo publish package ... --index-version 1.0.0 --index-tag stdcomm-v1.0.0
```

发布前 ccgo 会跑 `git rev-parse --verify <tag>` 验证 tag 真实存在。
新 `VersionEntry` 被追加进 `<index>/<sharded>/<name>.json` 已有的
`versions` 数组,然后按版本号字符串降序排序。

`{name}`、`{version}`、`{tag}` 占位符替换进 `--archive-url-template`。
`--checksum` 和 template 同时给的话,SHA-256 哈希的是本地
`target/release/package/<NAME>_CCGO_PACKAGE-<version>.zip` —— 跟消费方
fetch 时下到的 CDN 字节流是同一份。

完整的标志参考见下面的[发布到索引](#发布到索引)。

## 注册表索引格式

注册表是一个包含 JSON 文件的 Git 仓库，这些 JSON 文件描述可用的包。

为获得最佳 Git 性能，沿用 Rust 的 crates.io-index 命名约定：

| 名称长度 | 路径模式 | 示例 |
|-------------|--------------|---------|
| 1 字符 | `1/{name}.json` | `a` → `1/a.json` |
| 2 字符 | `2/{name}.json` | `cc` → `2/cc.json` |
| 3 字符 | `3/{first}/{name}.json` | `fmt` → `3/f/fmt.json` |
| 4+ 字符 | `{[0:2]}/{[2:4]}/{name}.json` | `spdlog` → `sp/dl/spdlog.json` |

```
ccgo-index/
├── index.json              # 注册表元信息
├── 1/
│   └── a.json              # 1 字符的包
├── 2/
│   └── cc.json             # 2 字符的包
├── 3/
│   └── f/
│       └── fmt.json        # 3 字符的包
├── sp/
│   └── dl/
│       └── spdlog.json     # 4+ 字符的包
└── nl/
    └── oh/
        └── nlohmann-json.json
```

这种目录结构：
- 避免单个目录文件过多（GitHub 限制约每目录 1000 个文件）
- 提升 Git 性能（大目录会拖慢 clone/pull）
- 均匀分布包，减少合并冲突

### index.json

```json
{
  "name": "ccgo-index",
  "description": "Official CCGO package index",
  "version": "1.0.0",
  "package_count": 42,
  "updated_at": "2026-01-24T12:00:00Z",
  "homepage": "https://github.com/zhlinh/ccgo-index"
}
```

### 包条目（例如 fmt.json）

```json
{
  "name": "fmt",
  "description": "A modern formatting library",
  "repository": "https://github.com/fmtlib/fmt.git",
  "license": "MIT",
  "platforms": ["android", "ios", "macos", "windows", "linux", "ohos"],
  "keywords": ["formatting", "string", "printf"],
  "versions": [
    {
      "version": "10.2.1",
      "tag": "v10.2.1",
      "checksum": "sha256:...",
      "archive_url": "https://artifacts.example.com/fmt/fmt_CCGO_PACKAGE-10.2.1.zip",
      "archive_format": "zip",
      "yanked": false
    },
    {
      "version": "10.1.1",
      "tag": "v10.1.1",
      "checksum": "sha256:...",
      "yanked": false
    }
  ]
}
```

`archive_url` 与 `archive_format` 是可选的；不带它们的条目仍然合法，
只会被注册表解析路径跳过（消费方那一版需要在 CCGO.toml 里显式声明
`git`/`zip` 来源）。当 `archive_url` 存在但没有 `archive_format` 时，
默认按 `"zip"` 处理；同时也支持 `"tar.gz"`。

## 配置

### 默认注册表

CCGO 自带一个已配置的默认注册表：

```toml
# 隐式默认 —— 无需配置
# 默认：git@github.com:zhlinh/ccgo-index.git
```

### 自定义注册表

在 `CCGO.toml` 中添加自定义注册表：

```toml
[registries]
company = "https://github.com/company/package-index.git"
private = "git@github.com:company/private-packages.git"
local = "file:///path/to/local/registry"
```

## 使用注册表

### 简化的依赖

启用注册表后可使用简化的依赖语法：

```toml
# 不必再写：
[[dependencies]]
name = "fmt"
version = "0.0.0"
git = "https://github.com/fmtlib/fmt.git"
branch = "10.2.1"

# 改为：
[dependencies]
fmt = "^10.2"
```

### 指定注册表

为某个依赖指定具体的注册表：

```toml
[dependencies.internal-lib]
version = "^1.0"
registry = "company"

# 或行内：
[dependencies]
public-lib = "^2.0"  # 使用默认注册表
```

## CLI 命令

### ccgo registry add

添加新注册表：

```bash
ccgo registry add <name> <url>

# 示例：
ccgo registry add company https://github.com/company/packages.git
ccgo registry add private git@github.com:company/private.git
```

### ccgo registry list

列出已配置的注册表：

```bash
ccgo registry list
ccgo registry list --details  # 显示包数量与更新时间
```

输出：
```
================================================================================
CCGO Registry - Configured Registries
================================================================================

Registries:

  ✓ ccgo-index (default)
    URL: git@github.com:zhlinh/ccgo-index.git

  ✓ company
    URL: https://github.com/company/packages.git

💡 Update registries with: ccgo registry update
```

### ccgo registry update

更新注册表索引：

```bash
ccgo registry update          # 更新所有注册表
ccgo registry update company  # 更新指定注册表
```

### ccgo registry remove

移除注册表：

```bash
ccgo registry remove company
```

注意：不能移除默认注册表。

### ccgo registry info

显示注册表详情：

```bash
ccgo registry info ccgo-index
```

输出：
```
================================================================================
CCGO Registry - Registry Information
================================================================================

Registry: ccgo-index
  URL: git@github.com:zhlinh/ccgo-index.git
  Cached: true

Index Metadata:
  Name: CCGO Packages
  Description: Official CCGO package index
  Version: 1.0.0
  Packages: 42
  Last Updated: 2026-01-24T12:00:00Z
  Homepage: https://github.com/zhlinh/ccgo-index
```

### ccgo registry search

搜索包：

```bash
ccgo registry search json
ccgo registry search json --registry company
ccgo registry search json --limit 5
```

## 增强的 search 命令

`ccgo search` 命令现在会同时搜索注册表与集合：

```bash
ccgo search json                    # 搜索所有源
ccgo search json --registry company # 搜索指定注册表
ccgo search json --registries-only  # 跳过集合
ccgo search json --collections-only # 跳过注册表
ccgo search json --details          # 显示详细信息
```

## 缓存位置

注册表索引在本地缓存：

```
~/.ccgo/registries/
├── ccgo-index/           # 已克隆的索引仓库
│   ├── index.json
│   └── ...
└── company/
    ├── index.json
    └── ...
```

## 创建注册表

要创建自己的包注册表：

1. 创建一个 Git 仓库
2. 添加包含注册表元信息的 `index.json`
3. 按单字母目录结构添加包 JSON 文件
4. 提交并推送

### 包 JSON Schema

```json
{
  "name": "string (required)",
  "description": "string (required)",
  "repository": "string (required, Git URL)",
  "license": "string (optional)",
  "platforms": ["array", "of", "platforms"],
  "keywords": ["array", "of", "keywords"],
  "versions": [
    {
      "version": "semver string (required)",
      "git_tag": "string (required)",
      "checksum": "sha256:... (optional)",
      "yanked": "boolean (default: false)"
    }
  ]
}
```

## 版本解析

`ccgo fetch` 通过 `[registries]` 解析仅写了 `version` 的依赖时：

1. 检查所有注册表（或 `[[dependencies]].registry`
   指定的那一个）。
2. 对每个注册表，确认其索引已克隆到 `~/.ccgo/registries/<name>/`
   （已存在则 pull）。
3. 在该索引中查找包对应的 sharded JSON 条目。
4. 在条目的 `versions[]` 中筛出未 yanked、版本号精确匹配的项。
5. 要求命中来源唯一。多个注册表同时命中时必须显式指定 `registry`，
   不使用隐式注册表优先级。
6. 若条目带有 `archive_url`，下载该归档；存在 `checksum` 时校验 SHA-256，
   然后解压至 `.ccgo/deps/<name>/`。
7. lockfile 记录 `source = "registry+<index-url>"` 和下载内容的 SHA-256，
   即使索引未提供可选的 checksum 也会计算。Git 源码回退则记录完整 commit。

`ccgo fetch --locked` 校验配置中的注册表身份，使用锁中的精确版本（允许
已 yank 的版本），并按锁中的 checksum 或 Git commit 获取内容。它不会改写
`CCGO.lock`，包括时间戳。即使已有安装也会重新检查注册表和内容，因此当前
锁定获取仍需能够访问注册表及制品或源码仓库。索引 checksum 改变会报错；
移除这个可选字段也不能绕过锁中的校验。下载地址可以改变，但内容必须一致。
注册表归档当前只支持 ZIP。

旧式 `registry+<name>@<version>` 锁文件在显式选择来源、或只配置一个注册表时
兼容读取。缺少 checksum 或完整 commit 的旧锁需要去掉 `--locked`，执行
`ccgo fetch --force` 重新生成。缓存统一遵循 `CCGO_HOME`；已有缓存的 Git origin
与配置地址不符时会报错，避免复用同名注册表的旧来源。

注意：当前迭代不解析 `^1.0`、`~2.1` 这类 semver 区间。`version = "x.y.z"`
是按字符串与 `VersionEntry.version` 精确匹配的。区间支持是后续计划。

## 发布到索引

使用 `ccgo publish package` 将你的包加入索引仓库：

```bash
# 发布到自定义索引
ccgo publish package --index-repo https://github.com/user/my-packages.git

# 自定义名称并推送
ccgo publish package \
  --index-repo https://github.com/company/packages.git \
  --index-name company \
  --index-push

# 自定义提交消息
ccgo publish package \
  --index-repo git@github.com:user/packages.git \
  --index-message "Add mylib v2.0.0"

# 为每个版本生成 SHA-256 校验和
ccgo publish package \
  --index-repo https://github.com/user/packages.git \
  --checksum \
  --index-push
```

### 它做了什么

1. 从 `CCGO.toml` 读取包元信息
2. 从 Git tag 发现版本（例如 `v1.0.0`、`1.0.0`）
3. 在正确的目录结构下生成 JSON 文件
4. 克隆 / 更新索引仓库
5. 提交变更（可选推送）

### 选项

| 选项 | 说明 |
|--------|-------------|
| `--index-repo <url>` | 索引仓库 URL（必填）|
| `--index-name <name>` | 注册表名称（默认 custom-index）|
| `--index-push` | 提交后推送变更到远端 |
| `--index-message <msg>` | 自定义提交消息 |
| `--checksum` | 通过 git archive 生成 SHA-256 校验和 |
| `--archive-url-template <T>` | 烤进每个 `VersionEntry.archive_url` 的 URL 模板。占位符：`{name}`、`{version}`、`{tag}`。|
| `--archive-format <fmt>` | 写入 `VersionEntry.archive_format` 的格式提示（默认 `zip`，也支持 `tar.gz`）。仅在设置了 `--archive-url-template` 时生效。|

### 示例输出

```
=== Publishing to Package Index ===

📦 Package: mylib
📝 Description: My awesome library
🔗 Repository: https://github.com/user/mylib.git

🔍 Discovering versions from Git tags...
   Found 3 version(s):
   - 2.0.0
   - 1.1.0
   - 1.0.0

📂 Index repository: https://github.com/user/my-packages.git
📥 Cloning index repository...
✅ Written: my/li/mylib.json
📊 Index metadata updated: 5 package(s)
✅ Committed: Update mylib to 2.0.0

✅ Package index updated successfully!

📋 To use this package:
   1. Add registry: ccgo registry add custom-index https://github.com/user/my-packages.git
   2. Add dependency: [dependencies]
      mylib = "^2.0.0"
```

## 最佳实践

1. **使用 semver**：用语义化版本为包打 tag
2. **不要删除版本**：改为标记 `yanked`
3. **添加校验和**：启用完整性校验
4. **保持索引精简**：只收录稳定的、已发布的版本
5. **定期更新**：用 `ccgo registry update` 保持本地缓存新鲜

## 另请参阅

- [Git 简写](git-shorthand.zh.md)
- [依赖管理](dependency-management.zh.md)
- [迁移到注册表方案](../guides/migrating-to-registry.zh.md)
- [依赖解析](../dependency-resolution.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
