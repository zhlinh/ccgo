# Git 简写与版本发现

> v3.1.1 新增

CCGO 借鉴 Swift Package Manager 的方式，支持简化的 Git 依赖语法和自动版本发现。

## Git URL 简写

无需写完整的 Git URL，可以直接使用简写：

```bash
# GitHub 简写
ccgo add github:fmtlib/fmt
ccgo add gh:nlohmann/json           # 'gh' 是别名

# GitLab 简写
ccgo add gitlab:user/repo
ccgo add gl:user/repo               # 'gl' 是别名

# Bitbucket 简写
ccgo add bitbucket:user/repo
ccgo add bb:user/repo               # 'bb' 是别名

# Gitee 简写
ccgo add gitee:user/repo

# 裸 owner/repo（默认 GitHub）
ccgo add fmtlib/fmt
```

### 携带版本标签

可以在简写中直接指定版本：

```bash
ccgo add github:fmtlib/fmt@v10.1.1
ccgo add gh:nlohmann/json@v3.11.0
```

## 自动版本发现

使用 `--latest` 自动发现并使用最新的 Git tag：

```bash
# 查找并使用最新的稳定版本
ccgo add github:fmtlib/fmt --latest

# 包含预发布版本
ccgo add github:fmtlib/fmt --latest --prerelease
```

### 工作原理

1. CCGO 执行 `git ls-remote --tags` 拉取所有 tag
2. 将 tag 解析为语义化版本
3. 按版本排序（最高优先）
4. 选择最新的稳定（非预发布）版本
5. 加上 `--prerelease` 时，预发布版本也会纳入候选

## 示例

### 添加依赖

```bash
# 以下命令完全等价：
ccgo add github:fmtlib/fmt@v10.1.1
ccgo add gh:fmtlib/fmt@v10.1.1
ccgo add fmtlib/fmt@v10.1.1
ccgo add fmt --git https://github.com/fmtlib/fmt.git --tag v10.1.1

# 自动发现最新版本
ccgo add github:gabime/spdlog --latest
```

### 生成的 CCGO.toml

```toml
[[dependencies]]
name = "fmt"
version = "0.0.0"
git = "https://github.com/fmtlib/fmt.git"
branch = "v10.1.1"

[[dependencies]]
name = "spdlog"
version = "0.0.0"
git = "https://github.com/gabime/spdlog.git"
branch = "v1.12.0"
```

## --git 选项中的简写

`--git` 选项也可使用简写：

```bash
# 这些命令等价：
ccgo add fmt --git github:fmtlib/fmt
ccgo add fmt --git gh:fmtlib/fmt
ccgo add fmt --git https://github.com/fmtlib/fmt.git
```

## 支持的 Provider

| Provider | 前缀 | 别名 | 基础 URL |
|----------|--------|-------|----------|
| GitHub | `github:` | `gh:` | https://github.com |
| GitLab | `gitlab:` | `gl:` | https://gitlab.com |
| Bitbucket | `bitbucket:` | `bb:` | https://bitbucket.org |
| Gitee | `gitee:` | - | https://gitee.com |

## SSH URL

CCGO 可以为私有仓库生成 SSH URL：

```rust
// 在你的代码中
let spec = expand_git_shorthand("github:company/private-lib")?;
let ssh_url = spec.ssh_url(); // git@github.com:company/private-lib.git
```

## 包注册表集成

> v3.2.0 新增

CCGO 现在支持包注册表以简化依赖管理。

### 简化的依赖语法

可在 CCGO.toml 中使用 table 风格的语法：

```toml
# 通过注册表使用简化版本语法
[dependencies]
fmt = "^10.1"
spdlog = "1.12.0"

# 或带更多选项
[dependencies.json]
version = "^3.11"
features = ["ordered_map"]
registry = "company-internal"  # 使用指定注册表
```

### 注册表命令

```bash
# 添加自定义注册表
ccgo registry add company https://github.com/company/package-index.git

# 列出已配置的注册表
ccgo registry list
ccgo registry list --details

# 更新注册表索引
ccgo registry update           # 更新所有注册表
ccgo registry update company   # 更新指定注册表

# 显示注册表信息
ccgo registry info ccgo-packages

# 搜索包
ccgo registry search json
ccgo registry search json --registry company --limit 10
```

### 私有注册表

在 CCGO.toml 中配置私有注册表：

```toml
[registries]
company = "https://github.com/company/package-index.git"
private = "git@github.com:company/private-index.git"
```

### 默认注册表

CCGO 使用 `ccgo-packages` 作为默认注册表，托管于：
`https://github.com/ArcticLampyrid/ccgo-packages.git`

### 注册表解析的工作原理

1. CCGO 检查依赖是否指定了注册表
2. 在该注册表的索引中查找包
3. 从索引解析 Git URL 与版本
4. 若未在任何注册表中找到，则回退到 Git URL

## 另请参阅

- [注册表参考](registry.zh.md)
- [依赖管理](dependency-management.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [路线图](../development/roadmap.zh.md)
