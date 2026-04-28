# 工作区依赖

## 概述

CCGO 提供 **工作区支持**，用于在 monorepo 结构中管理多个相关的 C++ 包。工作区可实现：

- 🏢 **Monorepo 管理** - 在单个仓库中组织多个包
- 🔗 **共享依赖** - 一次定义依赖，所有成员共用
- 🔄 **依赖继承** - 成员继承工作区依赖
- 📦 **协调构建** - 按依赖顺序构建多个包
- 🎯 **选择性构建** - 构建特定成员或一次性构建所有成员

## 优势

- **简化的依赖管理** - 在一处定义共享依赖
- **一致的版本** - 所有成员使用相同的共享依赖版本
- **更快的开发** - 无需发布内部依赖
- **灵活的配置** - 成员可以覆盖或扩展工作区依赖
- **构建优化** - 基于成员间依赖的智能构建排序

## 快速开始

### 1. 创建工作区根

创建一个包含多个包的工作区：

```toml
# /workspace/CCGO.toml
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/experimental"]

# 所有工作区成员的共享依赖
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"

[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
```

### 2. 创建工作区成员

每个成员都有自己的 CCGO.toml：

```toml
# /workspace/core/CCGO.toml
[package]
name = "core"
version = "1.0.0"

# 继承工作区依赖
[[dependencies]]
name = "fmt"
workspace = true

# 添加额外特性
[[dependencies]]
name = "spdlog"
workspace = true
features = ["async"]

# 成员特定的依赖
[[dependencies]]
name = "boost"
version = "1.80.0"
```

### 3. 构建工作区

```bash
# 构建所有工作区成员
ccgo build --workspace

# 构建特定成员
ccgo build --package core

# 按依赖顺序构建
ccgo build --workspace --ordered
```

## 工作区配置

### Workspace 段

在根 `CCGO.toml` 中定义工作区：

```toml
[workspace]
# 必填：成员包列表（支持 glob 模式）
members = [
    "core",           # 直接路径
    "libs/*",         # libs/ 下的所有目录
    "examples/**"     # 递归 glob
]

# 可选：排除特定路径
exclude = [
    "examples/test",
    "old/*"
]

# 可选：未指定包时构建的默认成员
default_members = ["core", "utils"]

# 可选：工作区级别的解析器版本（默认："2"）
resolver = "2"

# 工作区级别的依赖
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

### 成员配置

每个工作区成员都是一个常规的 CCGO 包，可选地继承工作区配置：

```toml
[package]
name = "my-package"
version = "1.0.0"

# 继承工作区依赖
[[dependencies]]
name = "fmt"
workspace = true

# 继承并扩展特性
[[dependencies]]
name = "spdlog"
workspace = true
features = ["async", "custom-formatter"]

# 继承并覆盖 default_features
[[dependencies]]
name = "boost"
workspace = true
default_features = false
features = ["filesystem"]

# 成员特定的依赖（不来自工作区）
[[dependencies]]
name = "rapidjson"
version = "1.1.0"
```

## 工作区成员发现

### Glob 模式

CCGO 支持使用 glob 模式发现工作区成员：

```toml
[workspace]
members = [
    "core",              # 精确路径
    "libs/*",            # 所有直接子目录
    "examples/**",       # 递归（所有后代）
    "tests/integration_*" # 模式匹配
]
```

**模式语法**：
- `*` - 匹配除路径分隔符外的任意字符
- `**` - 匹配包括路径分隔符在内的任意字符（递归）
- `?` - 匹配任意单个字符
- `[abc]` - 匹配方括号中的任意字符

**示例结构**：
```
workspace/
├── CCGO.toml (工作区根)
├── core/
│   └── CCGO.toml
├── libs/
│   ├── utils/
│   │   └── CCGO.toml
│   └── helpers/
│       └── CCGO.toml
└── examples/
    ├── basic/
    │   └── CCGO.toml
    └── advanced/
        └── CCGO.toml
```

使用 `members = ["core", "libs/*", "examples/*"]`，可发现全部 5 个包。

### 排除模式

从工作区中排除特定路径：

```toml
[workspace]
members = ["libs/*", "examples/*"]
exclude = [
    "examples/experimental",  # 排除特定目录
    "libs/old"                # 排除过时代码
]
```

## 依赖继承

### 基本继承

成员使用 `workspace = true` 继承依赖：

**工作区根**：
```toml
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

**成员**：
```toml
[[dependencies]]
name = "fmt"
workspace = true  # 继承版本 10.0.0
```

### 特性扩展

成员可以添加额外的特性：

**工作区根**：
```toml
[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
features = ["console"]
```

**成员**：
```toml
[[dependencies]]
name = "spdlog"
workspace = true
features = ["async"]  # 最终特性: ["console", "async"]
```

### 覆盖 default_features

成员可以控制默认特性：

**工作区根**：
```toml
[[workspace.dependencies]]
name = "boost"
version = "1.80.0"
default_features = true
```

**成员**：
```toml
[[dependencies]]
name = "boost"
workspace = true
default_features = false  # 覆盖以禁用默认特性
features = ["filesystem"]  # 仅启用特定特性
```

### 混合依赖

成员可以同时拥有工作区依赖和成员特定依赖：

```toml
[package]
name = "my-package"

# 来自工作区
[[dependencies]]
name = "fmt"
workspace = true

# 成员特定
[[dependencies]]
name = "rapidjson"
version = "1.1.0"

# 来自工作区并扩展
[[dependencies]]
name = "spdlog"
workspace = true
features = ["custom"]
```

## 构建顺序与依赖

### 成员间依赖

工作区成员可以相互依赖：

```toml
# core/CCGO.toml
[package]
name = "core"

# utils/CCGO.toml
[package]
name = "utils"

[[dependencies]]
name = "core"
path = "../core"  # 依赖同级包
```

### 拓扑构建顺序

CCGO 自动根据依赖确定构建顺序：

```
workspace/
├── core/          (无依赖)
├── utils/         (依赖 core)
└── app/           (依赖 utils)

构建顺序: core → utils → app
```

**用法**：
```bash
# 按依赖顺序构建
ccgo build --workspace --ordered

# 检测到循环依赖时构建失败
```

### 循环依赖检测

CCGO 会检测并阻止循环依赖：

```
core → utils → helpers → core  ❌ 循环！

错误: Circular dependency detected involving 'core'
```

## 命令

### 构建命令

**构建所有工作区成员**：
```bash
ccgo build --workspace
```

**构建特定成员**：
```bash
ccgo build --package core
ccgo build -p utils
```

**构建多个成员**：
```bash
ccgo build --package core --package utils
```

**按依赖顺序构建**：
```bash
ccgo build --workspace --ordered
```

**仅构建默认成员**：
```bash
# 如指定，则使用 workspace.default_members
ccgo build
```

### 列出命令

**列出所有工作区成员**：
```bash
ccgo workspace list

# 输出:
# Workspace: /path/to/workspace
# Members: 5
#   - core (1.0.0)
#   - utils (1.0.0)
#   - helpers (1.0.0)
#   - example-basic (0.1.0)
#   - example-advanced (0.1.0)
```

**列出成员依赖**：
```bash
ccgo workspace deps core

# 输出:
# Dependencies for core:
#   fmt 10.0.0 (from workspace)
#   spdlog 1.12.0 (from workspace)
#   boost 1.80.0 (member-specific)
```

### 检查命令

**检查工作区一致性**：
```bash
ccgo workspace check

# 验证:
# - 所有成员有有效的 CCGO.toml
# - 没有重复的成员名
# - 工作区依赖存在
# - 没有循环依赖
```

## 示例

### 示例 1：简单的库工作区

**结构**：
```
mylib/
├── CCGO.toml
├── core/
│   ├── CCGO.toml
│   ├── include/
│   └── src/
└── utils/
    ├── CCGO.toml
    ├── include/
    └── src/
```

**工作区根**（`mylib/CCGO.toml`）：
```toml
[workspace]
members = ["core", "utils"]

[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"

[[workspace.dependencies]]
name = "gtest"
version = "1.14.0"
```

**Core 包**（`mylib/core/CCGO.toml`）：
```toml
[package]
name = "mylib-core"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true
```

**Utils 包**（`mylib/utils/CCGO.toml`）：
```toml
[package]
name = "mylib-utils"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true

[[dependencies]]
name = "mylib-core"
path = "../core"
```

**构建**：
```bash
cd mylib
ccgo build --workspace --ordered
# 构建顺序: 先 core，后 utils
```

### 示例 2：Glob 模式发现

**结构**：
```
project/
├── CCGO.toml
├── core/
│   └── CCGO.toml
├── libs/
│   ├── utils/
│   │   └── CCGO.toml
│   ├── helpers/
│   │   └── CCGO.toml
│   └── common/
│       └── CCGO.toml
└── examples/
    ├── basic/
    │   └── CCGO.toml
    └── advanced/
        └── CCGO.toml
```

**工作区根**：
```toml
[workspace]
members = [
    "core",
    "libs/*",        # 发现: utils, helpers, common
    "examples/*"     # 发现: basic, advanced
]
exclude = []

# 总成员数: 6
```

### 示例 3：特性扩展

**工作区根**：
```toml
[workspace]
members = ["server", "client", "shared"]

[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
features = ["console"]  # 基础特性
```

**Server**（`server/CCGO.toml`）：
```toml
[package]
name = "server"

[[dependencies]]
name = "spdlog"
workspace = true
features = ["async", "multithreaded"]
# 最终特性: ["console", "async", "multithreaded"]
```

**Client**（`client/CCGO.toml`）：
```toml
[package]
name = "client"

[[dependencies]]
name = "spdlog"
workspace = true
# 仅使用工作区特性: ["console"]
```

### 示例 4：默认成员

**工作区根**：
```toml
[workspace]
members = ["core", "utils", "examples/*"]
default_members = ["core", "utils"]  # 默认不构建示例

[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

**构建行为**：
```bash
# 仅构建 core 和 utils（默认成员）
ccgo build

# 构建包括示例在内的所有成员
ccgo build --workspace

# 构建特定示例
ccgo build --package example-advanced
```

## 高级用法

### 解析器版本

控制依赖解析行为：

```toml
[workspace]
resolver = "2"  # 使用解析器 v2（推荐）
members = ["core", "utils"]
```

**解析器 v1**：
- 旧版解析器
- 在特性合并方面可能存在边界情况

**解析器 v2**（推荐）：
- 现代解析器
- 更好的特性合并
- 改进的性能
- 新项目的默认值

### 虚拟工作区

创建一个没有根包的工作区：

```toml
# 工作区根没有 [package] 段
[workspace]
members = ["package1", "package2"]

# 这是"虚拟工作区"——仅用于组织
# 工作区根没有库/二进制
```

### 嵌套工作区

**不支持**：工作区不能嵌套。

```
workspace/
├── CCGO.toml (工作区)
└── subproject/
    └── CCGO.toml (工作区)  ❌ 错误！
```

**替代方案**：使用工作区成员：
```
workspace/
├── CCGO.toml (工作区)
├── project1/
│   └── CCGO.toml (成员)
└── project2/
    └── CCGO.toml (成员)
```

## 故障排查

### 重复的成员名

**错误**：
```
Error: Duplicate workspace member name 'utils' at libs/utils
```

**解决方案**：确保所有工作区成员在其 [package] 段中具有唯一的名称。

### 缺失工作区依赖

**错误**：
```
Error: Member 'core' declares dependency 'fmt' with workspace=true,
but it's not defined in workspace dependencies
```

**解决方案**：将依赖添加到工作区根：
```toml
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

### 循环依赖

**错误**：
```
Error: Circular dependency detected involving 'core'
```

**解决方案**：审视成员间依赖并打破循环：
```
core → utils → core  ❌

解决方案:
core → utils ✅
（移除 utils 对 core 的依赖）
```

### 找不到成员

**错误**：
```
Error: Workspace member not found: libs/utils
```

**解决方案**：
1. 确认路径存在
2. 确保成员有 `CCGO.toml`
3. 检查 workspace.members 中是否有拼写错误

### Glob 模式问题

**问题**：未发现预期的成员

**调试**：
```bash
# 列出已发现的成员
ccgo workspace list

# 手动检查路径
ls -la libs/*/CCGO.toml
```

**常见原因**：
- 模式中遗漏 `*` 或 `**`
- 路径在 exclude 列表中
- 成员缺少 CCGO.toml

## 最佳实践

### 应该

✅ **对共享库使用工作区依赖**：
```toml
# 工作区根
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

✅ **对一致的目录结构使用 glob 模式**：
```toml
members = ["libs/*", "examples/*"]
```

✅ **为常见构建定义 default_members**：
```toml
default_members = ["core", "app"]  # 跳过 examples/tests
```

✅ **为工作区成员使用语义化版本**：
```toml
[package]
name = "core"
version = "1.2.3"  # 规范的 semver
```

✅ **对相互依赖的成员按依赖顺序构建**：
```bash
ccgo build --workspace --ordered
```

### 不应该

❌ **不要嵌套工作区**：
```toml
# 不支持——使用单一工作区
```

❌ **不要在成员中重复声明工作区依赖**：
```toml
# 错误
[[dependencies]]
name = "fmt"
version = "10.0.0"  # 应使用 workspace = true

# 正确
[[dependencies]]
name = "fmt"
workspace = true
```

❌ **不要使用不一致的版本**：
```toml
# 错误：成员使用不同版本
# member1: fmt 9.0.0
# member2: fmt 10.0.0

# 正确：所有成员都使用工作区依赖
```

❌ **不要创建循环依赖**：
```toml
# 错误: core → utils → core
# 正确: core → utils（单向）
```

## 实现细节

### 成员发现算法

1. 读取 workspace.members 模式
2. 将 glob 模式展开为具体路径
3. 过滤掉 workspace.exclude 模式
4. 对每个路径：
   - 检查目录是否存在
   - 检查是否存在 CCGO.toml
   - 加载并验证配置
5. 检查是否存在重复名称
6. 返回已发现的成员

### 依赖解析

1. 对每个成员依赖：
   - 如果 `workspace = true`：
     - 在 workspace.dependencies 中查找该依赖
     - 继承版本、特性等
     - 合并成员特定的特性
     - 应用成员的 default_features 覆盖
   - 否则：
     - 使用成员自身的依赖说明
2. 返回已解析的依赖

### 构建顺序计算

使用拓扑排序（深度优先搜索）：

1. 创建工作区成员的依赖图
2. 检测循环（循环依赖）
3. 执行 DFS 对成员排序
4. 返回构建顺序（依赖先于被依赖者）

**时间复杂度**：O(V + E)，其中 V = 成员数，E = 依赖数

## 参见

- [依赖管理](features/dependency-management.zh.md) - 整体依赖系统
- [版本冲突解决](version-conflict-resolution.md) - 处理版本冲突
- [项目结构](getting-started/project-structure.zh.md) - 项目组织

## 更新日志

### v3.0.12 (2026-01-21)

- ✅ 用于 monorepo 管理的工作区支持
- ✅ 工作区成员的 glob 模式发现
- ✅ 通过 `workspace = true` 实现依赖继承
- ✅ 成员间依赖解析
- ✅ 拓扑构建排序
- ✅ 循环依赖检测
- ✅ 特性扩展和 default_features 覆盖

---

*工作区依赖通过共享依赖和协调构建实现高效的 monorepo 管理。*
