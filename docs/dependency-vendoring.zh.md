# 依赖 Vendoring

## 概述

CCGO 支持**依赖 vendoring** —— 将项目所有依赖缓存到本地 `vendor/` 目录。这带来：

- ✅ **离线构建** —— vendor 后无需网络访问
- ✅ **可复现构建** —— 精确依赖版本锁定在 vendor 中
- ✅ **更快的 CI/CD** —— 构建期间无需下载依赖
- ✅ **安全性** —— 可审查和审计 vendored 依赖
- ✅ **合规性** —— 满足隔离环境的要求

## 快速开始

```bash
# 1. 先安装依赖（生成 CCGO.lock）
ccgo install

# 2. 将所有依赖 vendor 到 vendor/ 目录
ccgo vendor

# 3. 将 vendor/ 提交到版本控制
git add vendor/
git commit -m "vendor: cache dependencies for offline builds"

# 4. 此后 ccgo install 将使用 vendored 副本
ccgo install
```

## 命令

### `ccgo vendor`

将 `CCGO.lock` 中所有锁定的依赖复制到 `vendor/` 目录。

**前置条件**：
- 必须存在 `CCGO.lock`（先运行 `ccgo install`）
- 必须安装 Git（用于 git 依赖）

**执行流程**：
1. 读取 `CCGO.lock` 获取精确依赖版本
2. 将每个依赖复制到 `vendor/{name}/`
3. 剥离 `.git` 目录以节省空间
4. 生成 `vendor/.vendor.toml` 清单
5. 清理未使用的 vendored 依赖

**示例**：
```bash
$ ccgo vendor

================================================================================
CCGO Vendor - Vendor Dependencies for Offline Builds
================================================================================

Project directory: /path/to/project
Vendor directory: /path/to/project/vendor

📦 Vendoring 3 dependencies...
   ✓ Vendored fmt
   ✓ Vendored json
   ✓ Vendored gtest

================================================================================
Vendor Summary
================================================================================

✓ Vendored: 3

📁 Vendor directory: /path/to/project/vendor

💡 To use vendored dependencies:
   - Dependencies are now in vendor/
   - Commit vendor/ to version control for offline builds
```

### `ccgo vendor --verify`

验证 vendor/ 目录完整性，不修改它。

**检查内容**：
- 所有锁定的依赖都存在于 vendor/ 中
- vendored 源码与 lockfile 匹配
- 没有缺失或过期的依赖

**示例**：
```bash
$ ccgo vendor --verify

🔍 Verifying vendor directory...

✓ Vendor directory is up-to-date
  3 packages verified
```

**错误情况**：
```bash
$ ccgo vendor --verify

🔍 Verifying vendor directory...

⚠️  Vendor directory needs update:
   Missing: fmt
   Outdated: json (git URL changed)

   Run 'ccgo vendor --sync' to fix
Error: Vendor verification failed
```

### `ccgo vendor --sync`

重新 vendor 自上次 vendor 以来发生变化的依赖。

**使用场景**：
- 在 `CCGO.toml` 中更新依赖之后
- 修改 `CCGO.lock` 之后
- 修复验证失败

**示例**：
```bash
$ ccgo vendor --sync

📦 Vendoring 3 dependencies...
   ⏭️  fmt already vendored
   ✓ Vendored json (updated)
   ⏭️  gtest already vendored

✓ Vendored: 1
⏭️  Skipped (already vendored): 2
```

### `ccgo vendor --no-delete`

vendor 依赖时不清理未使用的依赖。

适用场景：
- 调试 vendoring 问题
- 临时测试不同的依赖版本
- 希望保留旧的 vendored 副本

**示例**：
```bash
$ ccgo vendor --no-delete

# 即使 CCGO.lock 中没有，也保留旧的 vendored 依赖
```

### `ccgo vendor --path custom-vendor`

使用自定义目录名替代 `vendor/`。

**示例**：
```bash
$ ccgo vendor --path .deps

# 将依赖 vendor 到 .deps/ 而不是 vendor/
```

## Install 如何使用 Vendor

运行 `ccgo install` 时，它会自动检查 vendored 依赖：

```rust
// 优先级顺序：
1. 检查 vendor/{name}/ 目录
2. 若存在 → 从 vendor 安装（离线模式）
3. 若不存在 → 从 git/path 拉取（在线模式）
```

**示例输出**：
```bash
$ ccgo install

📦 Installing fmt...
   📦 Found in vendor/ directory (offline mode)
   Source: /path/to/project/vendor/fmt
   ✓ Installed from vendor to .ccgo/deps/fmt

📦 Installing json...
   📦 Found in vendor/ directory (offline mode)
   Source: /path/to/project/vendor/json
   ✓ Installed from vendor to .ccgo/deps/json
```

## Vendor 目录结构

```
vendor/
├── .vendor.toml          # Vendor 清单（自动生成）
├── fmt/                  # vendored 依赖
│   ├── include/
│   ├── src/
│   └── CMakeLists.txt
├── json/
│   └── ...
└── gtest/
    └── ...
```

### `.vendor.toml` 格式

```toml
# 由 ccgo vendor 自动生成
# 请勿手动编辑

version = 1

[metadata]
generated_at = "2026-01-21T10:30:00+08:00"
ccgo_version = "3.0.11"
lockfile_hash = "abc123..."

[[package]]
name = "fmt"
version = "10.0.0"
source = "git+https://github.com/fmtlib/fmt"
vendored_at = "2026-01-21T10:30:00+08:00"
checksum = "sha256:def456..."

[[package]]
name = "json"
version = "3.11.2"
source = "git+https://github.com/nlohmann/json"
vendored_at = "2026-01-21T10:30:00+08:00"
checksum = "sha256:ghi789..."
```

## 常见工作流

### 使用 Vendoring 的 CI/CD 流水线

```yaml
# .github/workflows/build.yml
name: Build

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # 依赖已在 vendor/ 中——无需下载！
      - name: Install dependencies
        run: ccgo install

      - name: Build
        run: ccgo build linux
```

**收益**：
- ⚡ 无依赖下载耗时
- 🔒 构建时无网络故障
- 📦 多次运行结果一致

### 隔离（Air-Gapped）环境

适用于无互联网访问的环境：

```bash
# 在联网机器上：
ccgo install          # 下载依赖
ccgo vendor           # 缓存到 vendor/
tar czf project.tar.gz .

# 将 project.tar.gz 传输到隔离机器

# 在隔离机器上：
tar xzf project.tar.gz
cd project/
ccgo install          # 使用 vendored 副本
ccgo build linux      # 离线构建
```

### 更新 vendored 依赖

```bash
# 1. 在 CCGO.toml 中更新依赖版本
vim CCGO.toml

# 2. 更新 lockfile
ccgo install

# 3. 同步 vendor/ 到新版本
ccgo vendor --sync

# 4. 提交变更
git add CCGO.toml CCGO.lock vendor/
git commit -m "deps: update fmt to v10.1.0"
```

## 最佳实践

### ✅ 推荐做法

- **锁定后再 vendor** —— 始终在 `ccgo vendor` 之前运行 `ccgo install`
- **提交 vendor/** —— 将 `vendor/` 纳入版本控制
- **定期验证** —— 在 CI 中运行 `ccgo vendor --verify`
- **撰写文档** —— 在 README 中说明 vendoring 策略
- **审查变更** —— 在 PR 中审查 vendored 依赖的变更

### ❌ 应避免

- **不要手动编辑** —— 永远不要直接修改 `vendor/` 中的文件
- **不要混用模式** —— 不要混用 vendored 与非 vendored 依赖
- **不要忽略 lockfile** —— 始终将 `CCGO.lock` 与 `vendor/` 一起提交
- **不要 vendor 构建产物** —— `.git/`、`target/`、`build/` 已自动排除

## 故障排查

### 问题：找不到 CCGO.lock

**解决方法**：先运行 `ccgo install` 生成 lockfile：

```bash
$ ccgo install    # 生成 CCGO.lock
$ ccgo vendor     # 现在可以工作了
```

### 问题：vendor 验证失败

**解决方法**：重新同步 vendor 目录：

```bash
$ ccgo vendor --verify   # 显示问题所在
$ ccgo vendor --sync     # 修复问题
```

### 问题：vendor 目录过大

**原因**：
- vendoring 包含了较大的测试/文档文件
- `.git` 目录未被剥离

**解决方法**：
```bash
# 剥离 .git 目录（默认）
ccgo vendor --strip-git=true

# 手动清理 vendored 依赖：
rm -rf vendor/*/docs vendor/*/tests vendor/*/examples
```

### 问题：vendor 期间 git clone 失败

**可能原因**：
- 网络问题
- 未安装 Git
- CCGO.toml 中的 git URL 无效

**解决方法**：
```bash
# 检查 git 是否已安装
git --version

# 检查依赖来源
cat CCGO.toml | grep git

# 手动测试 clone
git clone <url>
```

## 配置

### .gitignore

如果**不想**提交 vendor/：

```gitignore
# 不提交 vendored 依赖
vendor/
!vendor/.vendor.toml
```

如果**希望**提交 vendor/（推荐）：

```gitignore
# 提交 vendor/ 以支持离线构建
# （无需任何条目）
```

### CCGO.toml

无需特殊配置。Vendoring 适用于任何依赖：

```toml
[[dependencies]]
name = "fmt"
version = "10.0.0"
git = "https://github.com/fmtlib/fmt"

[[dependencies]]
name = "mylib"
version = "1.0.0"
path = "../mylib"  # path 依赖也会被 vendor
```

## 性能影响

### Vendor 耗时

| 依赖数量    | 首次 Vendor   | 后续（--sync）       |
|-------------|---------------|---------------------|
| 1-5 个      | ~5-10s        | ~1-2s               |
| 5-10 个     | ~10-30s       | ~2-5s               |
| 10+ 个      | ~30-60s       | ~5-10s              |

### Install 耗时（启用 vendor）

| 依赖数量    | 无 Vendor      | 有 Vendor   | 加速比 |
|-------------|----------------|-------------|--------|
| 1-5 个      | ~10-30s        | ~1-2s       | 10x    |
| 5-10 个     | ~30-60s        | ~2-5s       | 10x    |
| 10+ 个      | ~60-120s       | ~5-10s      | 12x    |

**为什么这么快？**
- 无需 git clone 操作
- 无网络延迟
- 仅文件复制/symlink

## 安全考量

### 审计 vendored 依赖

提交前审查 vendored 代码：

```bash
# vendor 依赖
ccgo vendor

# 审查变更
git diff vendor/

# 检查可疑文件
find vendor/ -name "*.so" -o -name "*.dll" -o -name "*.exe"

# 审查后再提交
git add vendor/
git commit -m "vendor: add dependencies"
```

### 校验和验证

`.vendor.toml` 包含 SHA-256 校验和（TODO：实现）：

```toml
[[package]]
name = "fmt"
checksum = "sha256:abc123..."  # 验证完整性
```

验证方法：

```bash
ccgo vendor --verify  # 检查校验和
```

## 另请参阅

- [依赖解析](dependency-resolution.zh.md) —— 传递依赖处理
- [CCGO.toml 参考](reference/ccgo-toml.md) —— 配置文件规范
- [CLI 参考](reference/cli.md) —— 命令行接口

## 变更日志

### v3.0.11 (2026-01-21)

- ✅ 实现依赖 vendoring
- ✅ 新增 `ccgo vendor` 命令
- ✅ `ccgo install` 自动检测 vendor/
- ✅ Vendor 验证与同步
- ✅ 生成 `.vendor.toml` 清单

---

*该特性为企业用户和注重安全的用户提供离线构建与可复现环境的能力。*
