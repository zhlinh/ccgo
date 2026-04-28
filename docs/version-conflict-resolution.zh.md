# 版本冲突解决

## 概述

CCGO 提供基于语义化版本的**智能版本冲突解决**，自动检测和解决依赖版本冲突。它确保项目使用所有依赖之间相互兼容的版本。

## 收益

- 🎯 **自动检测** —— 在依赖解析期间识别版本冲突
- 🔄 **智能解析** —— 使用 semver 规则查找兼容版本
- 📊 **清晰报告** —— 展示详细的冲突信息
- ✅ **正确性保证** —— 仅允许兼容版本
- 🚀 **零配置** —— 自动生效

## 工作原理

### 语义化版本

CCGO 使用 [Semantic Versioning（SemVer）](https://semver.org/) 进行版本解析：

- **格式**：`MAJOR.MINOR.PATCH`（例如 `1.2.3`）
- **版本范围**：支持 `^`、`~`、`>=`、`<` 等
- **兼容性**：判定版本之间能否协同工作

### 版本需求

CCGO 支持多种版本需求格式：

| 格式      | 示例              | 含义                         |
|-----------|-------------------|------------------------------|
| **精确**  | `1.2.3`           | 精确为 1.2.3                 |
| **Caret** | `^1.2.3`          | >= 1.2.3, < 2.0.0（兼容）    |
| **Tilde** | `~1.2.3`          | >= 1.2.3, < 1.3.0（仅 patch）|
| **通配符**| `1.2.*` 或 `*`    | 任意 patch 或任意版本        |
| **范围**  | `>=1.0, <2.0`     | 多个约束                     |

### 冲突检测

CCGO 在以下情况下检测到冲突：
1. 多个依赖要求同一包的不同版本
2. 这些版本需求按 semver 规则**不兼容**

```
Project
  ├─ dep_a (requires fmt@^10.0.0)
  └─ dep_b (requires fmt@^11.0.0)   ← 冲突！
```

### 解析策略

检测到冲突时，CCGO：

1. **分析需求** —— 检查每个包的所有版本需求
2. **查找兼容版本** —— 使用 semver 找到满足所有需求的版本
3. **选择最高版本** —— 优先选用满足条件的最高版本
4. **报告失败** —— 若不存在兼容版本，输出详细错误

## 使用方法

### 自动解析

版本冲突解析在 `ccgo install` 期间自动进行：

```bash
$ ccgo install

📊 Resolving dependency graph...
⚠️  Detected 1 version conflicts:

   Package: fmt
      dep_a requires ^10.0.0
      dep_b requires 10.1.0
   ✓ Resolved to: 10.1.0

✓ Dependency graph resolved
```

### 兼容需求

需求兼容时，CCGO 静默完成解析：

```toml
# 项目 CCGO.toml
[[dependencies]]
name = "dep_a"
# dep_a requires fmt@^10.0.0

[[dependencies]]
name = "dep_b"
# dep_b requires fmt@10.1.0
```

**解析结果**：使用 `10.1.0`（同时满足 `^10.0.0` 和 `10.1.0`）

### 不兼容需求

需求冲突时，CCGO 输出错误：

```bash
$ ccgo install

📊 Resolving dependency graph...
⚠️  Detected 1 version conflicts:

   Package: fmt
      dep_a requires 10.0.0
      dep_b requires 11.0.0

Error: Cannot resolve version conflict for 'fmt': incompatible requirements
  - dep_a requires 10.0.0
  - dep_b requires 11.0.0
```

## 示例

### 示例 1：Caret 范围兼容

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "fmt"
version = "^10.0.0"   # 允许 10.x.x，< 11.0.0

# dep_b/CCGO.toml
[[dependencies]]
name = "fmt"
version = "10.2.1"    # 范围内的具体版本
```

**结果**：✅ 解析为 `10.2.1`（同时满足两个需求）

### 示例 2：Tilde 范围

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "spdlog"
version = "~1.11.0"   # 允许 1.11.x patch

# dep_b/CCGO.toml
[[dependencies]]
name = "spdlog"
version = "1.11.2"    # patch 版本
```

**结果**：✅ 解析为 `1.11.2`

### 示例 3：主版本冲突

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "json"
version = "3.10.0"    # v3.x

# dep_b/CCGO.toml
[[dependencies]]
name = "json"
version = "4.0.0"     # v4.x
```

**结果**：❌ 错误 —— 主版本不兼容

### 示例 4：通配符

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "catch2"
version = "*"         # 任意版本

# dep_b/CCGO.toml
[[dependencies]]
name = "catch2"
version = "3.4.0"     # 具体版本
```

**结果**：✅ 解析为 `3.4.0`（通配符接受任意版本）

## 常见场景

### 场景 1：菱形依赖

```
    Project
   /        \
  A          B
   \        /
    C@1.0  C@1.1
```

若 C@1.1 与 A 的需求兼容（例如 A 需要 `^1.0`）：
- **解析结果**：使用 C@1.1 ✅

若不兼容（例如 A 需要精确的 `1.0.0`）：
- **解析结果**：错误 ❌

### 场景 2：深层传递冲突

```
Project → A → B → C@2.0
Project → D → E → C@3.0
```

即便嵌套较深，CCGO 也能检测到 C@2.0 与 C@3.0 之间的冲突。

### 场景 3：多路径指向同一依赖

```
Project → A → C@^1.0
Project → B → C@^1.0
Project → D → C@1.2.0
```

三个需求互相兼容 —— 解析为 C@1.2.0 ✅

## 版本需求最佳实践

### 推荐做法

✅ **库依赖使用 caret 范围**
```toml
version = "^1.2.3"   # 允许兼容更新
```

✅ **关键依赖使用精确版本**
```toml
version = "2.5.0"    # 锁定具体版本
```

✅ **保持项目内主版本一致**
```toml
# 良好：均使用 fmt v10
dep_a = { version = "^10.0.0" }
dep_b = { version = "10.1.0" }
```

✅ **定期更新**，避免冲突积累
```bash
ccgo update
```

### 应避免

❌ **生产环境不要使用通配符**
```toml
version = "*"        # 版本不可预测
```

❌ **避免无谓地混用主版本**
```toml
# 不好：主版本不一致
dep_a = "1.0.0"      # v1
dep_b = "2.0.0"      # v2  ← 极易冲突
```

❌ **不要过度收紧约束**
```toml
version = "=1.2.3"   # 过于严格，难以解析
```

## 故障排查

### 冲突无法解决

**症状**：报错提示需求不兼容

**方案 1 —— 更新依赖**：
```bash
# 更新依赖到兼容版本
ccgo update

# 查看可用版本
ccgo search <package_name>
```

**方案 2 —— 调整版本需求**：
```toml
# 之前（过于严格）
[[dependencies]]
name = "fmt"
version = "10.0.0"    # 精确版本

# 之后（更灵活）
[[dependencies]]
name = "fmt"
version = "^10.0.0"   # 允许兼容版本
```

**方案 3 —— 联系维护者**：
若依赖之间确有冲突需求，可联系维护者：
- 请求版本更新
- 上报兼容性问题
- 建议调整版本范围

### 理解冲突报告

```
⚠️  Detected 1 version conflicts:

   Package: boost
      graphics_lib requires ^1.75.0
      network_lib requires ^1.80.0
      core_lib requires 1.76.0
```

**分析**：
- `graphics_lib` 需要 Boost 1.75+（< 2.0）
- `network_lib` 需要 Boost 1.80+（< 2.0）
- `core_lib` 需要精确 1.76.0

**问题所在**：core_lib 的精确版本（1.76.0）与 network_lib 的下限（1.80.0）冲突

**修复**：将 core_lib 改为使用 `^1.76.0` 而非 `1.76.0`

### 版本未找到

**症状**：报错 "Cannot extract version from range"

**原因**：版本格式无效或不受支持

**解决方法**：使用标准 semver 格式
```toml
# 不好
version = "v1.2.3"     # 不要带 'v' 前缀
version = "1.2"        # 缺失 patch 号

# 良好
version = "1.2.3"      # 完整 semver
version = "^1.2.0"     # 合法范围
```

## 进阶主题

### 自定义版本解析

目前 CCGO 仅支持自动解析。后续版本可能支持：

```toml
[resolution]
# 强制指定版本（覆盖冲突）
fmt = "10.1.0"
boost = "1.80.0"
```

### 版本锁定文件

为了保证可复现构建，CCGO 将支持锁定文件：

```bash
# 生成锁定文件
ccgo install

# 这会创建 CCGO.lock，记录精确解析的版本

# 使用锁定版本
ccgo install --locked
```

### 冲突解析策略

未来可能新增的策略：

1. **最高兼容**（当前）—— 选择满足所有需求的最高版本
2. **最低兼容** —— 选择最低版本（更保守）
3. **始终最新** —— 使用最新可用版本
4. **用户指定** —— 在 CCGO.toml 中手动覆盖

## 实现细节

### 版本比较算法

```rust
// 伪代码
fn is_compatible(req1: &VersionReq, req2: &VersionReq) -> bool {
    // 检查是否存在某个版本能同时满足两个需求
    for candidate_version in all_versions {
        if req1.matches(candidate_version) && req2.matches(candidate_version) {
            return true;
        }
    }
    false
}
```

### 冲突解析算法

```rust
// 伪代码
fn resolve_conflict(requirements: Vec<VersionReq>) -> Result<Version> {
    // 找到满足所有需求的最高版本
    let mut candidates = vec![];

    for req in requirements {
        let versions = extract_versions_from(req);
        candidates.extend(versions);
    }

    // 按版本号排序（高版本在前）
    candidates.sort_by(|a, b| b.cmp(a));

    // 找到第一个满足所有需求的版本
    for version in candidates {
        if requirements.iter().all(|req| req.matches(&version)) {
            return Ok(version);
        }
    }

    Err("No compatible version found")
}
```

## 另请参阅

- [依赖管理](features/dependency-management.zh.md) —— 整体依赖系统
- [Semantic Versioning](https://semver.org/) —— SemVer 规范
- [依赖解析](dependency-resolution.zh.md) —— 传递依赖处理

## 变更日志

### v3.0.12 (2026-01-21)

- ✅ 实现版本冲突检测
- ✅ 语义化版本支持（精确、范围、通配符）
- ✅ 智能冲突解析，选择最高兼容版本
- ✅ 详细的冲突报告
- ✅ 支持 caret（^）、tilde（~）和范围操作符
- ✅ 含解决建议的全面错误信息

---

*版本冲突解决自动确保项目使用相互兼容的依赖版本。*
