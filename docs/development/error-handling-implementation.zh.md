# 错误处理实现指南

> 版本：v3.0.10 | 更新时间：2026-01-21

## 概述

CCGO 增强的错误处理系统提供：

1. **用户友好的错误消息**，附带可执行的提示
2. **优雅降级**，当可选工具缺失时
3. **全面的配置验证**，附带有用的建议
4. **更好的构建失败诊断**

## 架构

```
src/
├── error.rs                      # 自定义错误类型和提示
├── config/
│   ├── ccgo_toml.rs             # 配置解析（增强版）
│   └── validation.rs            # 全面验证
└── utils/
    └── tools.rs                 # 工具检测和验证
```

## 组件

### 1. 错误模块（`src/error.rs`）

带上下文提示的自定义错误类型：

```rust
use crate::error::{CcgoError, hints};

// 配置错误
CcgoError::config_error_with_hint(
    "Invalid package name",
    None,
    "Package name must be a valid C++ identifier"
)

// 依赖错误
CcgoError::dependency_error_with_hint(
    "fmt",
    "Invalid version requirement",
    "Version requirements support: ^1.0.0, ~1.2.0, >=1.0.0"
)

// 缺失工具错误
CcgoError::missing_tool(
    "cmake",
    "building projects",
    hints::cmake()  // 提供安装说明
)

// 带诊断信息的构建失败
CcgoError::build_failure_with_diagnostics(
    "Android",
    "NDK not found",
    vec!["Check ANDROID_NDK environment variable"],
    Some(hints::android_ndk())
)
```

#### 可用提示

`hints` 模块为常见工具提供安装说明：

- `hints::cmake()` - CMake 安装
- `hints::git()` - Git 安装
- `hints::android_ndk()` - Android NDK 设置
- `hints::xcode()` - Xcode 安装
- `hints::visual_studio()` - Visual Studio 设置
- `hints::mingw()` - MinGW 安装
- `hints::gradle()` - Gradle 安装
- `hints::python()` - Python 安装
- `hints::doxygen()` - Doxygen 安装

#### 常见错误模式

```rust
// CCGO.toml 未找到
hints::ccgo_toml_not_found()

// 无效的 CCGO.toml
hints::invalid_ccgo_toml()

// 依赖解析失败
hints::dependency_resolution()

// 构建配置问题
hints::build_config()

// Lockfile 不同步
hints::lockfile_mismatch()
```

### 2. 配置验证（`src/config/validation.rs`）

附带有用错误消息的全面验证：

```rust
use crate::config::validate_config;

// 加载并验证配置
let config = CcgoConfig::load()?;  // 自动验证

// 手动验证
validate_config(&config)?;
```

#### 验证检查项

**包元信息：**
- ✅ 包名（必须是有效的 C++ 标识符，不能是关键字）
- ✅ 版本（必须是有效的 semver：`1.0.0`、`2.3.4-alpha.1`）
- ✅ 许可证（对非标准 SPDX 标识符发出警告）

**工作区配置：**
- ✅ 仅工作区配置时成员列表非空
- ✅ 有效的解析器版本（"1" 或 "2"）

**依赖项：**
- ✅ 至少有一个有效来源（version、git 或 path）
- ✅ 有效的版本要求（`^1.0.0`、`~1.2.0`、`>=1.0.0`）
- ✅ 有效的 git URL（必须以 `https://`、`git://` 等开头）
- ✅ 不存在冲突的 git refs（不能同时指定 branch 和 tag）
- ✅ 路径依赖存在（仅警告）

**构建配置：**
- ✅ 并行任务数 > 0
- ✅ 合理的任务数量（超过 128 时发出警告）

**平台配置：**
- ✅ Android minSdk >= 16（低于推荐值时发出警告）
- ✅ 有效的 iOS 版本格式（`"12.0"`、`"14.0"`）

**二进制和示例：**
- ✅ 有效的名称（C++ 标识符）
- ✅ 源文件存在（仅警告）

### 3. 工具检测（`src/utils/tools.rs`）

通过优雅降级检测构建工具：

```rust
use crate::utils::tools::{
    check_tool,
    require_tool,
    check_tool_with_requirement,
    ToolRequirement,
    PlatformTools,
};

// 检查工具是否存在
if let Some(tool_info) = check_tool("cmake") {
    println!("CMake version: {:?}", tool_info.version);
}

// 要求某个工具（缺失则报错）
let cmake = require_tool("cmake", "building projects")?;

// 按需求级别检查
check_tool_with_requirement(
    "doxygen",
    ToolRequirement::Recommended,
    "documentation generation"
)?;
// ⚠️ 缺失时发出警告，但继续执行
```

#### 工具需求级别

```rust
pub enum ToolRequirement {
    /// 必需 - 缺失时以有用的错误信息失败
    Required,

    /// 可选 - 缺失时静默
    Optional,

    /// 推荐 - 缺失时发出警告，继续执行
    Recommended,
}
```

#### 平台特定的工具检查

```rust
use crate::utils::tools::{
    PlatformTools,
    check_android_environment,
    check_apple_environment,
    check_windows_environment,
};

// 检查平台特定的工具
let checker = PlatformTools::new("android");
let (required, optional) = checker.check_all()?;

// Android 环境
check_android_environment()?;  // 检查 ANDROID_NDK、ANDROID_HOME

// Apple 环境
check_apple_environment()?;    // 检查 Xcode、命令行工具

// Windows 环境
check_windows_environment("msvc")?;  // 检查 MSVC 或 MinGW
```

## 集成示例

### 示例 1：增强的构建命令

```rust
use crate::utils::tools::{PlatformTools, ToolRequirement};
use crate::error::CcgoError;

pub fn execute_build(platform: &str) -> Result<()> {
    // 检查必需工具
    let checker = PlatformTools::new(platform);
    let (required_tools, optional_tools) = checker.check_all()?;

    println!("✓ Required tools found:");
    for tool in &required_tools {
        println!("  • {}: {}", tool.name,
            tool.version.as_deref().unwrap_or("installed"));
    }

    if !optional_tools.is_empty() {
        println!("✓ Optional tools found:");
        for tool in &optional_tools {
            println!("  • {}", tool.name);
        }
    }

    // 继续构建...
    Ok(())
}
```

### 示例 2：增强的安装命令

```rust
use crate::error::{CcgoError, hints};
use crate::config::validate_config;

pub fn execute_install(dep_name: Option<&str>) -> Result<()> {
    // 加载并验证配置
    let config = CcgoConfig::load()?;  // 自动验证

    // 检查 git 依赖所需的 git
    let has_git_deps = config.dependencies.iter()
        .any(|d| d.git.is_some());

    if has_git_deps {
        require_tool("git", "installing git dependencies")?;
    }

    // 安装依赖...
    Ok(())
}
```

### 示例 3：更好的错误消息

**之前：**
```
Error: Failed to parse CCGO.toml
```

**之后：**
```
ERROR: Failed to parse CCGO.toml

Common issues:
• Invalid TOML syntax (check quotes, brackets, commas)
• Typo in section names (should be [package] or [workspace])
• Missing closing brackets or quotes

Validate your TOML at: https://www.toml-lint.com/
```

**之前：**
```
Error: Invalid version requirement '1.0'
```

**之后：**
```
ERROR: Invalid version requirement '1.0' for dependency 'fmt'

HINT: Version requirements support:
• Exact: "1.2.3" or "=1.2.3"
• Range: ">=1.0.0, <2.0.0"
• Caret: "^1.0.0" (allows 1.x.x)
• Tilde: "~1.2.0" (allows 1.2.x)
• Wildcard: "1.*" or "1.2.*"
```

## 最佳实践

### 1. 始终提供上下文

```rust
// 错误
bail!("Tool not found");

// 正确
Err(CcgoError::missing_tool(
    "cmake",
    "building C++ projects",
    hints::cmake()
).into())
```

### 2. 使用合适的错误类型

```rust
// 配置错误
CcgoError::config_error_with_hint(message, source, hint)

// 依赖错误
CcgoError::dependency_error_with_hint(dep_name, message, hint)

// 缺失工具
CcgoError::missing_tool(tool, required_for, hint)

// 构建失败
CcgoError::build_failure_with_diagnostics(platform, message, diagnostics, hint)
```

### 3. 尽早验证

```rust
// 加载后立即验证配置
let config = CcgoConfig::load()?;  // 自动验证

// 或显式验证
validate_config(&config)?;
```

### 4. 构建前检查工具

```rust
// 平台特定构建
let checker = PlatformTools::new("android");
checker.check_required()?;  // 通过有用的错误快速失败

// 可选特性
check_tool_with_requirement(
    "doxygen",
    ToolRequirement::Recommended,
    "documentation generation"
)?;
```

## 错误消息编写指南

编写错误消息时：

1. **具体明确**：说明发生了什么以及发生在哪里
2. **可操作**：告诉用户如何修复
3. **简洁**：让提示聚焦于最可能的解决方案
4. **有帮助**：包含示例、链接或可运行的命令

示例：
```
❌ 错误："Invalid configuration"
✅ 正确：
ERROR: Invalid package name 'class' in CCGO.toml

HINT: Package name 'class' is a C++ reserved keyword.
Choose a different name, e.g., 'class_lib' or 'libclass'
```

## 测试

所有验证逻辑都包含单元测试：

```bash
cargo test error
cargo test validation
cargo test tools
```

## 当前状态

✅ **已完成：**
- 带提示的自定义错误类型
- 全面的配置验证
- 带优雅降级的工具检测
- 增强的 CCGO.toml 解析错误
- 平台特定的工具检查

📝 **集成任务（后续）：**
- 将工具检查集成到 `build` 命令
- 将工具检查集成到 `publish` 命令
- 添加更多平台特定的验证
- 带常见解决方案的构建失败诊断

## 迁移指南

在现有代码中使用新的错误处理方式：

### 之前：
```rust
use anyhow::bail;

if tool_missing {
    bail!("cmake not found");
}
```

### 之后：
```rust
use crate::error::{CcgoError, hints};
use crate::utils::tools::require_tool;

// 选项 1：使用工具检测
require_tool("cmake", "building projects")?;

// 选项 2：手动构造错误
if tool_missing {
    return Err(CcgoError::missing_tool(
        "cmake",
        "building projects",
        hints::cmake()
    ).into());
}
```

## 参见

- [贡献指南](contributing.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [路线图](roadmap.zh.md)
