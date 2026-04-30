# 传递依赖解析

> **另请参阅：** [`dependency-linkage.zh.md`](dependency-linkage.zh.md) —— 每个拉取到的依赖如何融入你的构建产物（shared-external、static-embedded、static-external 之间的取舍）。

## 概述

CCGO 现在支持自动传递依赖解析。运行 `ccgo install` 时，它会自动发现并安装依赖的依赖、确定正确的构建顺序，并检测循环依赖。

## 解析优先级

对每个 `[[dependencies]]` 条目，`ccgo fetch` 按下列源类型顺序决定字节
来源。第一个与依赖声明字段匹配的种类胜出：

1. **`path = "..."`** —— 本地路径依赖。从消费者目录中的相对/绝对路径
   软链或拷贝过来。
2. **`git = "..."`（搭配 `branch` / `tag` / `rev`）** —— Git 依赖。
   浅克隆到 `~/.ccgo/git/<repo>/`。
3. **`zip = "..."`** —— 归档依赖。`https://` 直接下载，`file://` 或本地
   路径直接读取，再解压到 `.ccgo/deps/<name>/`。
4. **注册表解析** —— 当 `[registries]` 非空、且该依赖未写
   `path`/`git`/`zip` 源时启用。ccgo 按 TOML 声明顺序遍历所有注册表
   （或 `[[dependencies]].registry` 指定的那一个），取首个未 yanked、
   版本号精确匹配的 `VersionEntry`,然后走两条子路径之一:
   * **Archive** —— 当 `VersionEntry.archive_url` 已填:下载该字节流、
     SHA-256 校验、解压。
   * **Git+tag 回退** —— 当 `archive_url` 为空,但 `PackageEntry.repository`
     已填(`ccgo publish index` 总会从项目的 `git remote` 自动写入):
     执行 `git clone --branch <tag>` 拉源码仓库。Lockfile 记录
     `source = "registry+<index-url>"` 加 `locked.git.revision`,保证
     `--locked` 重新拉取的字节确定性。

   索引 schema 与 `[registries]` 配置详见
   [`features/registry.zh.md`](features/registry.zh.md)。
5. **仅版本号的本地缓存回退** —— 当上面四种都没解析成功，且依赖的
   `version` 字段非空时，ccgo 查 `~/.ccgo/packages/<name>/<version>/`
   （由源项目里的 `ccgo install` 写入的缓存），把其中内容拷贝到
   `.ccgo/deps/<name>/`。这与 Cargo / Maven 的工作流一致 —— 依赖以
   名称+版本号识别，位置完全在开发机缓存里。

当第 4 步返回"未命中"时会平滑落到第 5 步 —— 已有的、不在任何已配置
注册表中的 `version`-only 依赖仍可正常工作。`[registries]` 为空时，
第 4 步整段跳过。这就是注册表层既可选又可渐进的原因：旧的
`git`/`zip` 声明永远有效。

## 功能

### 1. 传递依赖发现

当一个被安装的依赖自带 `CCGO.toml` 且其中声明了依赖时，CCGO 会：
- 自动读取该依赖的 CCGO.toml
- 发现其依赖（传递依赖）
- 递归解析整个依赖树中的所有依赖
- 同时处理 git 与 path 类型的依赖

### 2. 依赖图可视化

CCGO 会以可视化的树形输出展示依赖：
- 直接依赖（在你的 CCGO.toml 中声明）
- 传递依赖（依赖的依赖）
- 共享依赖（被多个包引用）
- 来源信息（git URL、路径、版本）

示例输出：
```
Dependency tree:
mylib v1.0.0
├── fmt v9.1.0 (git: https://github.com/fmtlib/fmt)
│   └── gtest v1.12.0 (git: https://github.com/google/googletest)
└── json v3.11.2 (git: https://github.com/nlohmann/json)

3 unique dependencies found, 4 total (1 shared)
```

### 3. 拓扑排序确定构建顺序

CCGO 使用拓扑排序确定正确的安装顺序：
- 没有依赖的依赖最先安装
- 依赖必须在其依赖者之前安装
- 通过尊重依赖链确保构建成功

示例：
```
📦 Installing in dependency order:
  1. gtest
  2. fmt
  3. mylib
```

### 4. 循环依赖检测

CCGO 会检测循环依赖并报告完整的循环路径：

```
Error: Circular dependency detected: libA -> libB -> libC -> libA
```

### 5. 版本冲突警告

当多个包依赖同一个依赖的不同版本时，CCGO 会：
- 对版本冲突发出警告
- 当前使用首次出现的版本
- 给出清晰的警告以便你解决冲突

示例：
```
⚠️  Version conflict for 'fmt': have 9.1.0, need 10.0.0
```

### 6. 最大深度保护

为防止无限递归，CCGO 将依赖深度限制在 50 层，超出则报错。

## 实现

### 架构

依赖解析系统由三个主要组件组成：

#### 1. 依赖图（`src/dependency/graph.rs`）

- **DependencyNode**：携带元信息的单个依赖
- **DependencyGraph**：管理整个依赖图
- **环检测**：基于 DFS 的循环依赖发现算法
- **拓扑排序**：使用 Kahn 算法确定构建顺序
- **树形格式化**：美化打印依赖树
- **统计**：计算独立、共享、总依赖数

#### 2. 依赖解析器（`src/dependency/resolver.rs`）

- **DependencyResolver**：编排依赖解析的主解析器
- **递归解析**：递归遍历依赖树
- **路径解析**：处理传递依赖中的相对路径
- **缓存**：visited 集合防止重复处理
- **错误处理**：解析失败时优雅降级

#### 3. install 命令集成（`src/commands/install.rs`）

- 调用解析器构建依赖图
- 显示依赖树与统计信息
- 用拓扑排序确定安装顺序
- 出错时回退到直接依赖
- 按正确顺序安装依赖

### 数据结构

```rust
pub struct DependencyNode {
    pub name: String,
    pub version: String,
    pub source: String,              // git+url 或 path+path
    pub dependencies: Vec<String>,   // 直接依赖
    pub depth: usize,                // 依赖树中的深度
    pub config: DependencyConfig,    // 原始配置
}

pub struct DependencyGraph {
    nodes: HashMap<String, DependencyNode>,
    edges: Vec<(String, String)>,    // (from, to) 边
    roots: HashSet<String>,          // 根依赖
}
```

### 关键算法

#### 环检测（DFS）

```rust
pub fn detect_cycles(&self) -> Option<Vec<String>>
```

使用带递归栈的深度优先搜索检测环。如发现则返回环路径。

#### 拓扑排序（Kahn 算法）

```rust
pub fn topological_sort(&self) -> Result<Vec<String>>
```

通过入度计算实现 Kahn 算法以确定构建顺序。

## 使用

### 基本用法

直接运行 `ccgo install`，CCGO 会自动：
1. 解析传递依赖
2. 显示依赖树
3. 显示安装顺序
4. 按正确顺序安装所有依赖

```bash
ccgo install
```

### 哪些会被解析

给定如下项目结构：

**项目 CCGO.toml：**
```toml
[package]
name = "myapp"
version = "1.0.0"

[[dependencies]]
name = "libA"
version = "1.0.0"
path = "../libA"
```

**libA CCGO.toml：**
```toml
[package]
name = "libA"
version = "1.0.0"

[[dependencies]]
name = "libB"
version = "2.0.0"
path = "../libB"
```

**libB CCGO.toml：**
```toml
[package]
name = "libB"
version = "2.0.0"
# 无依赖
```

执行 `ccgo install` 时会：
1. 发现 libA（直接依赖）
2. 读取 libA 的 CCGO.toml
3. 发现 libB（传递依赖）
4. 读取 libB 的 CCGO.toml
5. 确定顺序：libB → libA → myapp
6. 先装 libB，再装 libA

## 测试

实现包含全面的测试：

### 单元测试

位于 `src/dependency/resolver.rs` 与 `src/dependency/graph.rs`：

- **test_simple_resolution**：基础单依赖
- **test_transitive_dependencies**：依赖链（A → B → C）
- **test_circular_dependency_detection**：环检测（A → B → C → A）
- **test_shared_dependency**：菱形结构（A → C，B → C）
- **test_missing_ccgo_toml**：处理无 CCGO.toml 的依赖
- **test_version_conflict_warning**：检测版本冲突
- **test_max_depth_exceeded**：防止无限递归
- **test_simple_graph**：基础图操作
- **test_cycle_detection**：环检测算法
- **test_shared_dependency**（图）：共享依赖统计

运行测试：
```bash
cargo test dependency
```

## 局限与未来工作

### 当前局限

1. **版本解析**：当前采用"首个版本胜出"策略。需要正式的语义化版本解析。
2. **工作区依赖**：尚未完整实现工作区继承。
3. **锁文件**：尚未生成锁文件以保证可复现构建。
4. **依赖打补丁**：尚不支持覆盖传递依赖。

### 计划中的增强

1. **智能版本解析**：
   - 语义化版本感知
   - 最小版本选择
   - 版本约束求解

2. **锁文件支持**：
   - 生成包含精确版本的 CCGO.lock
   - 安装时校验锁文件
   - update 命令刷新锁文件

3. **依赖 vendor**：
   - 下载并缓存依赖
   - 支持离线构建
   - 可复现构建

4. **依赖覆盖**：
   - 通过 CCGO.toml 给依赖打补丁
   - 替换 URL 用于镜像
   - 版本钉死

5. **构建期依赖**：
   - 区分仅构建依赖
   - 开发依赖
   - 可选依赖

## 相关文件

- `src/dependency/mod.rs` —— 模块定义
- `src/dependency/graph.rs` —— 依赖图实现（约 450 行）
- `src/dependency/resolver.rs` —— 依赖解析器（约 620 行）
- `src/commands/install.rs` —— install 命令集成
- `src/config/ccgo_toml.rs` —— CCGO.toml 配置

## 参考

- **拓扑排序**：[Kahn 算法](https://en.wikipedia.org/wiki/Topological_sorting)
- **环检测**：[深度优先搜索](https://en.wikipedia.org/wiki/Cycle_detection)
- **语义化版本**：[semver.org](https://semver.org/)

## 更新日志

### v3.0.11 (2025-01-21)

- ✅ 实现传递依赖解析
- ✅ 添加带环检测的依赖图
- ✅ 添加拓扑排序以确定正确的构建顺序
- ✅ 添加依赖树可视化
- ✅ 添加版本冲突检测（仅警告）
- ✅ 集成至 install 命令
- ✅ 添加完整测试套件（7 个 resolver 测试 + 3 个 graph 测试）

---

*该功能是 Rust CLI 重写（spec 001-rust-cli-rewrite）的一部分，目标是零 Python 依赖。*
