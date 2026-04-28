# 构建分析

## 概述

CCGO 提供**构建分析**功能，用于跟踪和分析构建性能随时间的变化。它有助于识别瓶颈、跟踪改进效果并优化构建配置。

## 优势

- 📊 **性能跟踪** —— 监控构建时间并识别趋势
- 🎯 **瓶颈识别** —— 找出最耗时的阶段
- 📈 **缓存有效性** —— 跟踪编译器缓存命中/未命中率
- 🔍 **构建历史** —— 查看历史构建并对比性能
- 📉 **优化洞察** —— 数据驱动的构建优化决策

## 快速开始

构建分析数据在构建过程中**自动收集**，并存储在本地的 `~/.ccgo/analytics/`。

### 查看分析数据

```bash
# 显示最近的构建（默认 10 条）
ccgo analytics show

# 显示更多构建
ccgo analytics show -n 20

# 显示汇总统计
ccgo analytics summary

# 列出所有有分析数据的项目
ccgo analytics list
```

## 命令

### `ccgo analytics show`

显示包含关键指标的最近构建历史：

```bash
$ ccgo analytics show

================================================================================
Build Analytics for myproject
================================================================================

Build #1 - linux (2026-01-21T15:30:00+08:00)
  Duration:    45.30s
  Jobs:        8
  Success:     ✓
  Cache Tool:  sccache
  Cache Rate:  78.5%

Build #2 - linux (2026-01-21T16:00:00+08:00)
  Duration:    12.50s
  Jobs:        8
  Success:     ✓
  Cache Tool:  sccache
  Cache Rate:  95.2%

Build #3 - macos (2026-01-21T16:15:00+08:00)
  Duration:    38.70s
  Jobs:        8
  Success:     ✓
  Cache Tool:  ccache
  Cache Rate:  82.3%
```

**选项**：
- `-n, --count <NUM>` —— 显示的构建数量（默认 10）

### `ccgo analytics summary`

显示所有构建的聚合统计信息：

```bash
$ ccgo analytics summary

================================================================================
Build Analytics Summary for myproject
================================================================================

Total Builds:      25
Successful:        24 (96.0%)

Build Duration:
  Average:         32.45s
  Fastest:         11.20s
  Slowest:         58.90s

Cache Statistics:
  Builds with cache: 23
  Avg Hit Rate:      85.3%

Platform Breakdown:
  linux............... 15
  macos............... 8
  ios................. 2

================================================================================
```

### `ccgo analytics clear`

清除当前项目的分析历史：

```bash
$ ccgo analytics clear

This will delete 25 build analytics entries for 'myproject'
Continue? [y/N] y
✓ Cleared analytics for 'myproject'
```

**选项**：
- `-y, --yes` —— 跳过确认提示

### `ccgo analytics export`

将分析数据导出到 JSON 文件：

```bash
$ ccgo analytics export -o builds.json

✓ Exported 25 build analytics to builds.json
```

**选项**：
- `-o, --output <FILE>` —— 输出文件路径

### `ccgo analytics list`

列出所有有分析数据的项目：

```bash
$ ccgo analytics list

================================================================================
Projects with Analytics
================================================================================

  myproject.................................... 25 builds
  another-lib.................................. 12 builds
  experimental................................. 5 builds

Use 'ccgo analytics show' from a project directory to view details.
================================================================================
```

## 收集的指标

### 构建概览

- **项目名称** —— 正在构建的项目
- **平台** —— 目标平台（linux、macos、windows 等）
- **时间戳** —— 构建开始时间（ISO 8601）
- **总时长** —— 完整构建耗时（秒）
- **并行 Job 数** —— 并行编译任务数
- **成功状态** —— 构建是否成功
- **错误/警告** —— 编译诊断计数

### 阶段分解

构建被划分为多个阶段，每个阶段单独计时：

1. **依赖解析** —— 安装并解析依赖
2. **CMake 配置** —— CMake configure 步骤
3. **编译** —— C/C++ 源文件编译
4. **链接** —— 链接库
5. **归档** —— 创建 ZIP 归档
6. **后处理** —— 额外的打包步骤

每个阶段会跟踪：
- 时长（秒）
- 占总构建时间的百分比

### 缓存统计

对于使用 ccache/sccache 的构建：

- **缓存工具** —— 正在使用的工具（ccache、sccache）
- **缓存命中** —— 从缓存重用的编译产物
- **缓存未命中** —— 新增到缓存的编译结果
- **命中率** —— 缓存命中百分比（0-100%）

### 文件统计

- **源文件** —— .c/.cc/.cpp 文件数量
- **头文件** —— .h/.hpp 文件数量
- **总行数** —— 代码总行数
- **产物大小** —— 最终输出大小（字节）

## 数据存储

分析数据存储在本地：

```
~/.ccgo/analytics/
├── myproject.json      # myproject 的分析数据
├── another-lib.json    # another-lib 的分析数据
└── ...
```

每个项目文件包含：
- 最近 100 次构建（更早的构建会被自动裁剪）
- JSON 格式，便于解析和导出
- 不包含任何可识别个人身份的信息

## 分析 API（Rust）

在 CCGO 内部以编程方式访问：

```rust
use ccgo::build::analytics::{BuildAnalytics, AnalyticsCollector, BuildPhase};

// 创建收集器
let mut collector = AnalyticsCollector::new(
    "myproject".to_string(),
    "linux".to_string(),
    8, // 并行 job 数
);

// 阶段计时
collector.start_phase(BuildPhase::Compilation);
// ... 编译工作 ...
collector.end_phase(BuildPhase::Compilation);

// 记录诊断
collector.add_diagnostics(2, 15); // 2 个错误，15 个警告

// 设置成功状态
collector.set_success(true);

// 完成并保存
let analytics = collector.finalize(cache_stats, file_stats);
analytics.save()?;

// 加载历史
let history = BuildAnalytics::load_history("myproject")?;

// 获取平均构建时间
let avg = BuildAnalytics::average_build_time("myproject")?;
```

## 使用场景

### 性能回归检测

```bash
# 改动后
ccgo build linux

# 检查构建是否变慢
ccgo analytics summary

# 期望：平均时长不应显著增加
```

### 缓存有效性

```bash
# 第一次构建（冷缓存）
ccgo build linux --cache sccache
# 记录耗时

# 第二次构建（热缓存）
ccgo build linux --cache sccache
# 应快 50-80%

# 检查缓存命中率
ccgo analytics show -n 1
# 期望：高缓存命中率（>80%）
```

### CI/CD 监控

```yaml
# .github/workflows/build.yml
- name: Build
  run: ccgo build linux

- name: Show Analytics
  run: ccgo analytics show -n 1

- name: Export Analytics
  run: ccgo analytics export -o build-stats.json

- name: Upload Analytics
  uses: actions/upload-artifact@v3
  with:
    name: build-analytics
    path: build-stats.json
```

### 跨平台对比

```bash
# 构建多个平台
ccgo build linux
ccgo build macos
ccgo build windows

# 查看汇总
ccgo analytics summary

# Platform Breakdown 部分展示相对性能
```

## 最佳实践

### DO

✅ **定期查看** —— 在重大变更后检查分析数据
✅ **跟踪趋势** —— 监控构建是否随时间变慢
✅ **优化热点路径** —— 聚焦占比最高的阶段
✅ **启用缓存** —— 编译器缓存可显著改善指标
✅ **导出报告** —— 使用 JSON 导出进行趋势分析

### DON'T

❌ **不要提交分析数据** —— 数据仅在本地机器
❌ **不要手动编辑** —— 分析文件由系统自动生成
❌ **不要依赖首次构建** —— 冷缓存构建总是更慢
❌ **不要跨机器对比** —— 硬件会影响计时

## 故障排除

### 没有分析数据

**症状**：`ccgo analytics show` 提示 "No build analytics available"

**解决**：分析收集是自动的，但需要：
```bash
# 先运行一次构建
ccgo build linux

# 然后查看分析数据
ccgo analytics show
```

### 分析数据未更新

**症状**：新构建未出现在分析数据中

**解决**：检查构建是否成功完成：
```bash
# 验证构建成功
ccgo build linux

# 查看最近的分析数据
ccgo analytics show -n 1
```

### 看到错误项目的分析数据

**症状**：看到的是其他项目的分析数据

**解决**：分析数据按 `CCGO.toml` 中的项目名称关联：
```bash
# 检查当前项目
grep "name =" CCGO.toml

# 确认在正确目录中
pwd
```

### 存储位置

分析数据存储在：
- **Linux**：`~/.ccgo/analytics/`
- **macOS**：`~/.ccgo/analytics/`
- **Windows**：`%USERPROFILE%\.ccgo\analytics\`

## 未来增强

未来版本计划的功能：

- [ ] 实时构建进度可视化
- [ ] 内存使用跟踪
- [ ] 依赖编译分解
- [ ] 历史趋势图表
- [ ] 导出为 CSV/Excel
- [ ] 与构建仪表板集成
- [ ] 跨团队对比分析
- [ ] 自动回归告警

## 另请参阅

- [构建缓存](build-caching.zh.md) —— 通过缓存改善构建时间
- [构建系统](features/build-system.zh.md) —— 通用构建系统概览
- [增量构建](incremental-builds.md) —— 更快的重建策略

## 更新日志

### v3.0.11 (2026-01-21)

- ✅ 实现构建分析系统
- ✅ 添加 `ccgo analytics` 命令及其 show/summary/clear/export/list 子命令
- ✅ 自动收集构建指标
- ✅ 阶段计时分解
- ✅ 缓存统计集成
- ✅ 文件与错误/警告跟踪
- ✅ 本地存储于 `~/.ccgo/analytics/`
- ✅ 每项目最多保留最近 100 次构建（自动裁剪）

---

*构建分析帮助你基于数据做出关于构建优化与配置的决策。*
