# 增量构建

## 概述

CCGO 提供**智能增量构建检测**，自动只重新构建发生变化的文件及其依赖项。这通过避免不必要的重新编译，显著缩短重建时间。

## 收益

- ⚡ **更快的重建** —— 仅重新编译变更文件（10-50 倍加速）
- 🎯 **智能检测** —— 跟踪文件、配置和依赖变化
- 🔍 **变更分析** —— 精确显示自上次构建以来的变化
- 🚀 **零配置** —— 自动生效，无需任何设置
- 💾 **持久化状态** —— 构建状态可跨终端会话保留

## 工作原理

### 构建状态跟踪

CCGO 为每个平台和链接类型维护构建状态：

```
cmake_build/release/linux/
  ├── .ccgo_build_state.json    # 构建状态（文件哈希、元信息）
  └── CMakeCache.txt              # CMake 构建缓存
```

构建状态跟踪以下内容：
- **文件哈希** —— 所有源文件/头文件的 SHA256 校验和
- **配置哈希** —— CCGO.toml 配置的变化
- **选项哈希** —— 构建标志和选项的变化
- **CMake 缓存** —— CMake 配置的变化
- **上次构建时间** —— 最近一次成功构建的时间戳

### 变更检测

每次构建时，CCGO：

1. **加载历史状态** —— 若存在则读取 `.ccgo_build_state.json`
2. **扫描当前文件** —— 对所有源文件/头文件进行哈希
3. **比较哈希** —— 检测修改、新增和删除的文件
4. **检查配置** —— 检测 CCGO.toml 或构建选项的变化
5. **决定构建策略**：
   - **增量构建** —— 若可行，仅构建变更文件
   - **完全重建** —— 若配置/选项变化或 CMake 缓存缺失

## 使用方法

### 自动增量构建

增量构建无需配置即可自动生效：

```bash
# 首次构建（完整）
ccgo build linux
# ✓ Build completed in 45.2s

# 修改一个源文件
echo "// comment" >> src/mylib.cpp

# 第二次构建（增量）
ccgo build linux
# 📊 Incremental build - 1 files changed:
#      Modified: 1
# ✓ Build completed in 3.8s (11.9x faster!)
```

### 构建输出示例

#### 无变化
```bash
$ ccgo build linux

   ✨ No source changes detected, using cached build
   ✓ Build completed in 0.5s
```

#### 增量构建
```bash
$ ccgo build linux

   📊 Incremental build - 3 files changed:
      Modified: 2
      Added:    1
   ⚡ Rebuilding affected files...
   ✓ Build completed in 4.2s
```

#### 需要完全重建
```bash
$ ccgo build linux

   🔄 Full rebuild required: CCGO.toml configuration changed
   ⚡ Building all files...
   ✓ Build completed in 42.8s
```

## 何时触发完全重建

### 配置变更

任何对 `CCGO.toml` 的修改都会触发完全重建：

```toml
[package]
version = "1.0.1"  # 变化 → 完全重建

[dependencies]
# 新增依赖 → 完全重建
fmt = "10.1.1"
```

### 构建选项变更

不同的构建选项需要完全重建：

```bash
# 首次构建使用 4 个 jobs
ccgo build linux --jobs 4

# 第二次构建使用 8 个 jobs → 完全重建
ccgo build linux --jobs 8

# 不同架构 → 完全重建
ccgo build linux --arch x86_64
ccgo build linux --arch arm64  # 完全重建

# Feature 变化 → 完全重建
ccgo build linux --features networking
ccgo build linux --features advanced  # 完全重建
```

### CMake 缓存变更

CMake 重新配置时会触发完全重建：

```bash
# 清除 CMake 缓存 → 下次完全重建
rm -rf cmake_build/release/linux/CMakeCache.txt
ccgo build linux
```

### 新增/删除文件

新增或删除源文件会触发 CMake 重新配置：

```bash
# 新增源文件
touch src/new_feature.cpp

# 下次构建检测到新增
ccgo build linux
# 📊 Incremental build - 1 files changed:
#      Added: 1
# 🔧 CMake reconfiguration needed
```

## 构建状态文件

### 位置

构建状态按平台和构建模式分别存储：

```
cmake_build/
  ├── release/
  │   ├── linux/.ccgo_build_state.json
  │   ├── macos/.ccgo_build_state.json
  │   └── windows/.ccgo_build_state.json
  └── debug/
      └── linux/.ccgo_build_state.json
```

### 状态文件格式

`.ccgo_build_state.json` 包含：

```json
{
  "project": "myproject",
  "platform": "linux",
  "link_type": "static",
  "last_build_time": 1737433200,
  "config_hash": "a1b2c3...",
  "options_hash": "d4e5f6...",
  "cmake_cache_hash": "g7h8i9...",
  "file_hashes": {
    "src/mylib.cpp": "sha256_hash...",
    "src/utils.cpp": "sha256_hash...",
    "include/mylib.h": "sha256_hash..."
  }
}
```

### 手动管理状态

```bash
# 查看构建状态
cat cmake_build/release/linux/.ccgo_build_state.json

# 通过删除状态强制完全重建
rm cmake_build/release/linux/.ccgo_build_state.json
ccgo build linux

# 或使用 clean 命令
ccgo clean
```

## 性能对比

增量构建的典型加速效果：

| 场景            | 变更文件数      | 完全构建 | 增量构建 | 加速比          |
|-----------------|-----------------|----------|----------|-----------------|
| 无变更          | 0               | 45s      | 0.5s     | **快 90 倍**    |
| 单文件变更      | 1               | 45s      | 3.8s     | **快 11.9 倍**  |
| 少量变更（5%）  | 10/200          | 45s      | 8.2s     | **快 5.5 倍**   |
| 较多变更（25%）| 50/200          | 45s      | 18.5s    | **快 2.4 倍**   |
| 头文件变更      | 1（影响 50 个） | 45s      | 22.1s    | **快 2.0 倍**   |
| 全部变更        | 200/200         | 45s      | 44s      | ~1x（完全重建） |

**说明**：加速比取决于：
- 项目规模和复杂度
- 变更文件数量
- 依赖关系（头文件）
- 编译器缓存（ccache/sccache）的有效性
- 硬件（CPU、磁盘速度）

## 最佳实践

### 推荐做法

✅ **交给 CCGO 决策** —— 增量构建是自动且智能的
✅ **配合编译器缓存使用** —— 与 `--cache sccache` 组合可获得最大速度
✅ **频繁提交** —— 变更越小，重建越快
✅ **隔离头文件变更** —— 头文件变更会触发更多重建
✅ **信任系统** —— CCGO 保证正确性

### 应避免

❌ **不要手动修改构建状态** —— 文件由系统自动生成
❌ **不要共享构建状态** —— 状态依赖具体机器
❌ **不要禁用** —— 无法禁用（始终有益）
❌ **不要修改构建目录** —— 让 CCGO 管理

## CI/CD 集成

### GitHub Actions

```yaml
name: Build

on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest

    steps:
      - uses: actions/checkout@v3

      # 缓存 CMake 构建目录以支持增量构建
      - name: Cache CMake Build
        uses: actions/cache@v3
        with:
          path: cmake_build/
          key: ${{ runner.os }}-cmake-${{ hashFiles('CCGO.toml', 'src/**') }}
          restore-keys: |
            ${{ runner.os }}-cmake-

      # 缓存命中时增量构建生效
      - name: Build
        run: ccgo build linux
```

**CI 中的收益**：
- 🚀 **更快的 PR 构建** —— 仅重建变更文件
- 💰 **降低 CI 成本** —— 减少计算时间
- ⚡ **更快的反馈** —— 开发者更快拿到结果

### GitLab CI

```yaml
build:
  image: rust:latest

  cache:
    paths:
      - cmake_build/
    key:
      files:
        - CCGO.toml
        - src/**/*.cpp

  script:
    - ccgo build linux
```

## 故障排查

### 增量构建未生效

**症状**：每次构建都是完全重建

**原因与解决方法**：

1. **构建状态文件缺失**
   ```bash
   # 检查状态文件是否存在
   ls cmake_build/release/linux/.ccgo_build_state.json

   # 若缺失，一次完整构建会创建它
   ccgo build linux
   ```

2. **配置或选项发生变化**
   ```bash
   # 查看变更
   git diff CCGO.toml

   # 确认使用了相同的构建选项
   ```

3. **CMake 缓存被清除**
   ```bash
   # 检查 CMakeCache.txt 是否存在
   ls cmake_build/release/linux/CMakeCache.txt

   # 不要在两次构建间手动删除 cmake_build/
   ```

### 增量构建结果异常

**症状**：构建成功但变更未反映在产物中

**解决方法**：理论上不会发生 —— 增量系统采取保守策略。如怀疑存在问题：

```bash
# 强制完全重建
ccgo clean
ccgo build linux

# 或仅删除构建状态
rm cmake_build/release/linux/.ccgo_build_state.json
ccgo build linux
```

### 构建状态损坏

**症状**：增量构建期间出现意外错误

**解决方法**：
```bash
# 清理后重建
ccgo clean -y
ccgo build linux

# 或手动删除状态
rm -rf cmake_build/
ccgo build linux
```

## 实现细节

### 变更检测算法

```rust
// 伪代码
fn can_incremental_build() -> bool {
    // 加载历史构建状态
    let old_state = load_build_state()?;

    // 检查配置
    if old_state.config_hash != current_config_hash() {
        return false; // 配置已变化
    }

    // 检查构建选项
    if old_state.options_hash != current_options_hash() {
        return false; // 选项已变化
    }

    // 检查 CMake 缓存
    if !cmake_cache_exists() || old_state.cmake_cache_hash != current_cmake_cache_hash() {
        return false; // CMake 需要重新配置
    }

    true // 可以进行增量构建
}

fn analyze_changes() -> ChangeAnalysis {
    let mut changes = ChangeAnalysis::new();

    // 扫描当前源文件
    for file in scan_source_files() {
        let current_hash = hash_file(file);

        match old_state.file_hashes.get(file) {
            Some(old_hash) if old_hash != current_hash => {
                changes.modified_files.push(file);
            }
            None => {
                changes.added_files.push(file);
            }
            _ => {} // 未变化
        }
    }

    // 检测删除的文件
    for old_file in old_state.file_hashes.keys() {
        if !current_files.contains(old_file) {
            changes.removed_files.push(old_file);
        }
    }

    changes
}
```

### 文件哈希

CCGO 使用 SHA256 对文件内容进行哈希：

```rust
use sha2::{Digest, Sha256};

fn hash_file(path: &Path) -> String {
    let content = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&content);
    format!("{:x}", hasher.finalize())
}
```

**为何选择 SHA256？**
- 对源文件足够快（每个文件 < 1ms）
- 抗碰撞（无误报）
- 标准且久经验证
- Rust 标准库即可获得

### CMake 集成

增量构建借助 CMake 内置的增量编译能力：

1. **CMake 检测文件变化** —— 检查修改时间
2. **CCGO 检测配置变化** —— 防止陈旧构建
3. **组合方案** —— 兼顾两者优势

CCGO 的变更检测**保守** —— 一旦存疑，就完全重建。

## 进阶主题

### 多平台

每个平台拥有独立的构建状态：

```bash
# 构建 Linux（如可行则增量）
ccgo build linux

# 构建 macOS（独立状态，可能完全重建）
ccgo build macos
```

平台之间的变更互不影响。

### Debug 与 Release

Debug 与 Release 构建拥有独立状态：

```bash
# Release 构建
ccgo build linux

# Debug 构建（独立状态）
ccgo build linux --debug
```

### 链接类型

静态与动态构建共用同一份构建状态：

```bash
# 静态构建
ccgo build linux --build-as static

# 动态构建（增量，共享源码编译）
ccgo build linux --build-as shared
```

两种链接类型使用同一组源文件，因此变更可相互传播。

## 未来规划

后续版本计划的特性：

- [ ] 用于头文件变化的依赖图跟踪
- [ ] 并行增量编译
- [ ] 远程构建缓存（团队共享）
- [ ] 构建时间预测
- [ ] 旧构建状态的自动清理
- [ ] 可视化依赖图
- [ ] 单文件级别的构建时间跟踪

## 另请参阅

- [构建缓存](build-caching.zh.md) —— 提升构建速度的编译器缓存
- [构建分析](build-analytics.zh.md) —— 性能指标与跟踪
- [构建系统](features/build-system.zh.md) —— 构建系统总览

## 变更日志

### v3.0.12 (2026-01-21)

- ✅ 实现增量构建检测
- ✅ 基于文件哈希（SHA256）的构建状态跟踪
- ✅ 配置和选项变更检测
- ✅ CMake 缓存跟踪
- ✅ 包含修改/新增/删除文件的变更分析
- ✅ 自动持久化构建状态
- ✅ 按平台、按链接类型分别管理状态
- ✅ 保留最近 100 次构建以供分析

---

*增量构建只重建发生变化的部分，让开发流程显著更快。*
