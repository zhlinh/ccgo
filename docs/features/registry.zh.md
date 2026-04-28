# 包注册表

> v3.2.0 新增

CCGO 支持包注册表 —— 一种基于 Git 的轻量级包索引，无需中心服务器即可实现简化的依赖管理。

## 概述

借鉴 Swift Package Manager 的方式，CCGO 使用 Git 仓库作为包索引。这种设计：

- 无需服务器维护
- 复用既有 Git 基础设施
- 天然支持私有包
- 缓存后即可离线使用

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
ccgo-packages/
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
  "name": "ccgo-packages",
  "description": "Official CCGO package index",
  "version": "1.0.0",
  "package_count": 42,
  "updated_at": "2026-01-24T12:00:00Z",
  "homepage": "https://github.com/ArcticLampyrid/ccgo-packages"
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
      "git_tag": "10.2.1",
      "checksum": "sha256:...",
      "yanked": false
    },
    {
      "version": "10.1.1",
      "git_tag": "10.1.1",
      "checksum": "sha256:...",
      "yanked": false
    }
  ]
}
```

## 配置

### 默认注册表

CCGO 自带一个已配置的默认注册表：

```toml
# 隐式默认 —— 无需配置
# 默认：https://github.com/ArcticLampyrid/ccgo-packages.git
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

  ✓ ccgo-packages (default)
    URL: https://github.com/ArcticLampyrid/ccgo-packages.git

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
ccgo registry info ccgo-packages
```

输出：
```
================================================================================
CCGO Registry - Registry Information
================================================================================

Registry: ccgo-packages
  URL: https://github.com/ArcticLampyrid/ccgo-packages.git
  Cached: true

Index Metadata:
  Name: CCGO Packages
  Description: Official CCGO package index
  Version: 1.0.0
  Packages: 42
  Last Updated: 2026-01-24T12:00:00Z
  Homepage: https://github.com/ArcticLampyrid/ccgo-packages
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
├── ccgo-packages/           # 已克隆的索引仓库
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

从注册表解析包时：

1. 在指定（或默认）注册表中查找包
2. 过滤匹配版本约束的版本
3. 排除已 yanked 的版本
4. 选择最高匹配版本
5. 用 `git_tag` 克隆该仓库

## 发布到索引

使用 `ccgo publish index` 将你的包加入索引仓库：

```bash
# 发布到自定义索引
ccgo publish index --index-repo https://github.com/user/my-packages.git

# 自定义名称并推送
ccgo publish index \
  --index-repo https://github.com/company/packages.git \
  --index-name company \
  --index-push

# 自定义提交消息
ccgo publish index \
  --index-repo git@github.com:user/packages.git \
  --index-message "Add mylib v2.0.0"

# 为每个版本生成 SHA-256 校验和
ccgo publish index \
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
- [依赖管理](dependency-management.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
