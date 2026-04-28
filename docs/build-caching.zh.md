# 使用 ccache/sccache 进行构建缓存

## 概述

CCGO 通过 ccache 和 sccache 提供**编译器缓存**，借助缓存编译产物可将 C++ 编译速度提升 **30-50%**（甚至更多）。

## 优势

- ✅ **更快的重建** —— 后续构建复用已缓存的编译结果
- ✅ **自动检测** —— 自动检测并使用可用的缓存工具
- ✅ **零配置** —— 安装好缓存工具即可开箱使用
- ✅ **共享缓存** —— 缓存在所有 CCGO 项目间共享
- ✅ **CI/CD 友好** —— 显著加速持续集成构建

## 快速开始

### 安装缓存工具

**方式 1：sccache（推荐）**
```bash
# macOS
brew install sccache

# Linux
cargo install sccache

# Arch Linux
sudo pacman -S sccache

# Debian/Ubuntu
sudo apt install sccache
```

**方式 2：ccache**
```bash
# macOS
brew install ccache

# Linux
sudo apt install ccache  # Debian/Ubuntu
sudo yum install ccache  # CentOS/RHEL
sudo pacman -S ccache    # Arch Linux
```

### 启用缓存构建

`--cache auto` 默认**已启用**：

```bash
# 自动检测并使用可用缓存（默认）
ccgo build linux

# 显式指定缓存工具
ccgo build linux --cache sccache
ccgo build linux --cache ccache

# 禁用缓存
ccgo build linux --cache none
```

## 缓存工具对比

| 特性 | ccache | sccache |
|---------|--------|---------|
| 实现语言 | C | Rust |
| 速度 | 快 | 更快 |
| 云存储 | 不支持 | 支持（S3、Redis、Memcached）|
| 发行情况 | 稳定、成熟 | 现代、活跃维护 |
| 平台支持 | 全平台 | 全平台 |
| 内存占用 | 低 | 中 |
| 推荐 | 不错的选择 | 更好的选择 |

**CCGO 优先级顺序**：sccache > ccache > none

## 使用方式

### 命令行选项

```bash
# 自动检测（默认）—— 优先尝试 sccache，再 ccache
ccgo build <platform> --cache auto

# 强制指定缓存工具
ccgo build <platform> --cache sccache
ccgo build <platform> --cache ccache

# 禁用缓存
ccgo build <platform> --cache none
ccgo build <platform> --cache off
ccgo build <platform> --cache disabled
```

### 构建输出

启用缓存时会看到：

```bash
$ ccgo build linux

Building CCGO Library for linux...
   🚀 Using sccache for compilation caching

Configuring CMake...
-- Build files generated successfully
```

### 缓存统计

**ccache**：
```bash
# 显示缓存统计
ccache -s

# 清零统计（重置计数器）
ccache -z

# 清空缓存
ccache -C
```

**sccache**：
```bash
# 显示缓存统计
sccache --show-stats

# 清零统计（重置计数器）
sccache --zero-stats

# 停止 server（清空内存缓存）
sccache --stop-server
```

## 性能影响

### 第一次构建（冷缓存）

```
耗时：100%（基线）
- 没有缓存产物
- 需要完整编译
```

### 第二次构建（热缓存）

```
耗时：第一次的 20-50%（快 50-80%）
- 重用缓存产物
- 仅重新编译已变更的文件
```

### 示例数据

| 项目规模 | 首次构建 | 缓存构建 | 加速比 |
|-------------|-------------|--------------|---------|
| 小型（5-10 个文件）| 10s | 3s | 3.3x |
| 中型（50-100 个文件）| 60s | 15s | 4x |
| 大型（500+ 个文件）| 300s | 60s | 5x |

**注意**：项目越大、构建越频繁，加速比越高。

## 工作原理

### CMake 集成

CCGO 自动通过 compiler launcher 变量配置 CMake：

```cmake
# 启用缓存时由 CCGO 注入
CMAKE_C_COMPILER_LAUNCHER=/path/to/sccache
CMAKE_CXX_COMPILER_LAUNCHER=/path/to/sccache
```

这会用缓存工具包装你的 C/C++ 编译器（gcc、clang、msvc）。

### 缓存键

编译产物按以下因素缓存：
- 源文件内容
- 编译器标志
- 头文件依赖
- 预处理宏定义
- 编译器版本

任一项变化都会使缓存失效，触发重新编译。

### 缓存位置

**ccache**：
- 默认：`~/.ccache/`（Linux/macOS）、`%LOCALAPPDATA%\ccache`（Windows）
- 配置：`export CCACHE_DIR=/custom/path`

**sccache**：
- 默认：`~/.cache/sccache/`（Linux）、`~/Library/Caches/Mozilla.sccache/`（macOS）
- 配置：`export SCCACHE_DIR=/custom/path`

## 配置

### 环境变量

**ccache**：
```bash
# 设置缓存目录
export CCACHE_DIR=/path/to/cache

# 设置缓存最大容量
export CCACHE_MAXSIZE=5G

# 启用压缩
export CCACHE_COMPRESS=true

# 设置压缩级别（1-9）
export CCACHE_COMPRESSLEVEL=6
```

**sccache**：
```bash
# 设置缓存目录
export SCCACHE_DIR=/path/to/cache

# 设置缓存最大容量
export SCCACHE_CACHE_SIZE="5G"

# 使用 Redis 进行分布式缓存
export SCCACHE_REDIS=redis://localhost:6379

# 使用 AWS S3 进行分布式缓存
export SCCACHE_BUCKET=my-sccache-bucket
export SCCACHE_REGION=us-west-2
```

### CI/CD 配置

**GitHub Actions**：
```yaml
name: Build with Cache

on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      # 安装 sccache
      - name: Install sccache
        run: |
          wget https://github.com/mozilla/sccache/releases/download/v0.5.4/sccache-v0.5.4-x86_64-unknown-linux-musl.tar.gz
          tar xzf sccache-v0.5.4-x86_64-unknown-linux-musl.tar.gz
          sudo mv sccache-v0.5.4-x86_64-unknown-linux-musl/sccache /usr/local/bin/

      # 缓存 sccache 目录
      - name: Cache sccache
        uses: actions/cache@v3
        with:
          path: ~/.cache/sccache
          key: ${{ runner.os }}-sccache-${{ hashFiles('**/CCGO.toml') }}
          restore-keys: |
            ${{ runner.os }}-sccache-

      # 启用缓存的构建
      - name: Build
        run: ccgo build linux

      # 显示缓存统计
      - name: Show cache stats
        run: sccache --show-stats
```

**GitLab CI**：
```yaml
build:
  image: ubuntu:latest
  cache:
    key: $CI_COMMIT_REF_SLUG
    paths:
      - .cache/sccache
  before_script:
    - apt-get update && apt-get install -y sccache
    - export SCCACHE_DIR=$PWD/.cache/sccache
  script:
    - ccgo build linux
    - sccache --show-stats
```

## 故障排除

### 缓存未生效

**检查缓存工具是否已安装**：
```bash
which sccache
which ccache
```

**检查缓存是否被检测到**：
```bash
ccgo build linux --verbose
# 应显示 "Using sccache for compilation caching"
```

**验证 CMake 配置**：
```bash
# 检查 CMake 构建日志中的 COMPILER_LAUNCHER
grep COMPILER_LAUNCHER cmake_build/release/linux/CMakeCache.txt
```

### 缓存未命中率过高

**可能原因**：
- 不同构建间的编译器标志不同
- 基于时间戳的依赖（应改用基于哈希）
- 头文件频繁变化
- 缓存容量过小（用 `CCCACHE_MAXSIZE` 或 `SCCACHE_CACHE_SIZE` 增大）

**解决方法**：
```bash
# 增大缓存容量
export CCACHE_MAXSIZE=10G
export SCCACHE_CACHE_SIZE="10G"

# 启用压缩以容纳更多内容
export CCACHE_COMPRESS=true
```

### 权限错误

**修复缓存目录权限**：
```bash
# ccache
chmod -R u+w ~/.ccache

# sccache
chmod -R u+w ~/.cache/sccache
```

### 缓存损坏

**清空并重建缓存**：
```bash
# ccache
ccache -C  # 清空缓存
ccgo build linux

# sccache
sccache --stop-server  # 停止 server（清空缓存）
rm -rf ~/.cache/sccache  # 删除缓存目录
ccgo build linux
```

## 最佳实践

### DO

✅ **使用 sccache** —— 比 ccache 更快、功能更丰富
✅ **在 CI/CD 中启用** —— 显著加速流水线构建
✅ **监控缓存大小** —— 设置合理上限以避免磁盘空间问题
✅ **共享缓存** —— 团队构建可使用分布式缓存（Redis/S3）
✅ **保持缓存温热** —— 定期构建可维持缓存有效性

### DON'T

❌ **不要混用 debug/release** —— 它们的缓存键不同
❌ **不要提交缓存** —— 缓存目录应加入 `.gitignore`
❌ **不要使用过小的缓存** —— 中型项目至少设 5GB
❌ **不要在开发中禁用** —— 缓存能加速开发构建

## 高级用法

### 分布式缓存

**sccache + Redis**：
```bash
# 启动 Redis server
docker run -d -p 6379:6379 redis

# 配置 sccache
export SCCACHE_REDIS=redis://localhost:6379

# 构建（缓存在团队间共享）
ccgo build linux
```

**sccache + AWS S3**：
```bash
# 配置 AWS 凭据
export AWS_ACCESS_KEY_ID=...
export AWS_SECRET_ACCESS_KEY=...

# 配置 S3 bucket
export SCCACHE_BUCKET=my-team-cache
export SCCACHE_REGION=us-west-2

# 构建（缓存在 S3 中共享）
ccgo build linux
```

### 自定义缓存配置

创建 `.ccgo/cache_config.toml`：
```toml
[cache]
# auto、ccache、sccache 或 none
tool = "auto"

# 缓存最大容量
max_size = "10G"

# 启用压缩
compress = true

# 分布式缓存 URL（仅 sccache）
redis_url = "redis://localhost:6379"
```

**注意**：此功能计划在未来版本中提供。

## 另请参阅

- [构建系统](features/build-system.zh.md) —— 通用构建系统概览
- [增量构建](incremental-builds.md)
- [CMake 集成](reference/cmake.md)

## 更新日志

### v3.0.11 (2026-01-21)

- ✅ 实现 ccache/sccache 集成
- ✅ 自动检测可用的缓存工具
- ✅ 命令行 `--cache` 选项
- ✅ 自动配置 CMake compiler launcher
- ✅ 支持所有平台（Linux、macOS、Windows、iOS、Android、OHOS）

---

*编译器缓存可将构建时间减少 30-50% 甚至更多，使迭代开发更加高效。*
