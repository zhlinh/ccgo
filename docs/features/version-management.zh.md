# 版本管理

使用语义化版本控制、git 标签和自动版本注入管理 CCGO 项目版本的完整指南。

## 概览

CCGO 提供全面的版本管理功能：

- **语义化版本控制** - 遵循 SemVer 2.0.0 规范
- **自动标签** - 从 CCGO.toml 版本创建 git 标签
- **版本注入** - 自动将版本信息注入到构建中
- **多平台支持** - 所有平台的版本保持一致
- **构建元数据** - 在二进制文件中包含 git commit SHA、构建时间戳
- **发布管理** - 简化发布工作流

## 版本格式

### 语义化版本控制

CCGO 遵循 [语义化版本 2.0.0](https://semver.org/lang/zh-CN/)：

```
主版本号.次版本号.修订号[-预发布版本][+构建元数据]
```

**组成部分：**
- `主版本号`：不兼容的 API 更改
- `次版本号`：向后兼容的新功能
- `修订号`：向后兼容的问题修正
- `预发布版本`：可选的预发布标识符（alpha、beta、rc）
- `构建元数据`：可选的构建元数据（commit SHA、时间戳）

**示例：**
```
1.0.0           # 稳定版本
1.2.3           # 修订更新
2.0.0-alpha.1   # 预发布版
1.5.0+20240115  # 带构建元数据
2.1.0-rc.2+g8f3a  # 预发布版带 git hash
```

## 配置

### CCGO.toml

```toml
[package]
name = "mylib"
version = "1.2.3"  # SemVer 格式
authors = ["Your Name <you@example.com>"]

[version]
# 版本注入设置
inject_build_metadata = true  # 包含 git SHA 和时间戳
inject_to_code = true          # 生成版本头文件
prerelease_suffix = "beta"     # 可选：alpha、beta、rc

# 平台特定版本覆盖（可选）
[version.android]
version_code = 10203  # Android 整数版本（未设置时自动计算）

[version.ios]
build_number = "123"  # iOS 构建号（默认为 PATCH）

[version.windows]
file_version = "1.2.3.0"  # Windows 4 部分版本
```

### 版本自动计算

CCGO 自动计算平台特定的版本号：

**Android version_code：**
```
version_code = 主版本号 * 10000 + 次版本号 * 100 + 修订号

示例：1.2.3 → 10203
```

**iOS build number：**
```
build_number = 修订号（默认）
或自定义：build_number = "123"
```

**Windows file version：**
```
file_version = 主版本号.次版本号.修订号.0

示例：1.2.3 → 1.2.3.0
```

## 创建版本标签

### 使用 ccgo tag

```bash
# 从 CCGO.toml 版本创建标签
ccgo tag

# 使用自定义版本创建标签
ccgo tag v2.0.0

# 带消息创建标签
ccgo tag --message "发布版本 2.0.0 及新功能"

# 创建注解标签
ccgo tag --annotate

# 推送标签到远程
ccgo tag --push
```

### 标签格式

```bash
# CCGO 创建格式为：v{VERSION} 的标签
v1.0.0
v1.2.3-beta.1
v2.0.0
```

### 手动标签

```bash
# 创建轻量标签
git tag v1.0.0

# 创建注解标签
git tag -a v1.0.0 -m "发布 1.0.0"

# 推送标签到远程
git push origin v1.0.0

# 推送所有标签
git push --tags
```

## 版本注入

### 构建时注入

CCGO 在构建时自动注入版本信息：

```bash
# 版本信息注入到所有构建
ccgo build android

# 禁用版本注入
ccgo build android --no-version-inject
```

### 生成的版本头文件

**C++ 头文件（`include/<project>/version.h`）：**
```cpp
#pragma once

#define MYLIB_VERSION "1.2.3"
#define MYLIB_VERSION_MAJOR 1
#define MYLIB_VERSION_MINOR 2
#define MYLIB_VERSION_PATCH 3

#define MYLIB_GIT_SHA "8f3a2b1c"
#define MYLIB_GIT_BRANCH "main"
#define MYLIB_BUILD_TIMESTAMP "2024-01-15T10:30:00Z"
#define MYLIB_BUILD_TYPE "Release"

// 平台特定
#ifdef __ANDROID__
#define MYLIB_VERSION_CODE 10203
#elif defined(__APPLE__)
#define MYLIB_BUNDLE_VERSION "123"
#elif defined(_WIN32)
#define MYLIB_FILE_VERSION "1.2.3.0"
#endif

namespace mylib {
    const char* get_version();
    const char* get_git_sha();
    const char* get_build_timestamp();
}
```

### 在代码中使用版本

```cpp
#include "mylib/version.h"
#include <iostream>

void print_version() {
    std::cout << "MyLib 版本：" << mylib::get_version() << "\n";
    std::cout << "Git SHA：" << mylib::get_git_sha() << "\n";
    std::cout << "构建时间：" << mylib::get_build_timestamp() << "\n";
}
```

## 平台特定版本控制

### Android

**版本代码和版本名称：**
```toml
[package]
version = "1.2.3"

[version.android]
version_code = 10203       # Play Store 的整数
version_name = "1.2.3"     # 显示名称（默认为 package.version）
```

**Gradle 集成：**
```kotlin
// 在 build.gradle.kts 中生成
android {
    defaultConfig {
        versionCode = 10203
        versionName = "1.2.3"
    }
}
```

### iOS

**Bundle 版本：**
```toml
[package]
version = "1.2.3"

[version.ios]
bundle_short_version = "1.2.3"  # CFBundleShortVersionString
build_number = "123"             # CFBundleVersion
```

**Info.plist：**
```xml
<key>CFBundleShortVersionString</key>
<string>1.2.3</string>
<key>CFBundleVersion</key>
<string>123</string>
```

### OpenHarmony

**HAR 版本：**
```toml
[package]
version = "1.2.3"

[version.ohos]
app_version_code = 10203
app_version_name = "1.2.3"
```

**module.json5：**
```json
{
  "module": {
    "versionCode": 10203,
    "versionName": "1.2.3"
  }
}
```

### Windows

**文件版本：**
```toml
[package]
version = "1.2.3"

[version.windows]
file_version = "1.2.3.0"        # 四部分版本
product_version = "1.2"          # 显示版本
company_name = "您的公司"
copyright = "版权所有 © 2024"
```

## 预发布版本

### Alpha 发布

```toml
[package]
version = "2.0.0-alpha.1"

[version]
prerelease_suffix = "alpha"
```

```bash
# 标记 alpha 发布
ccgo tag v2.0.0-alpha.1 --message "Alpha 发布 1"
```

### Beta 发布

```toml
[package]
version = "2.0.0-beta.2"

[version]
prerelease_suffix = "beta"
```

### 候选发布

```toml
[package]
version = "2.0.0-rc.1"

[version]
prerelease_suffix = "rc"
```

### 提升为稳定版

```bash
# 将 RC 提升为稳定版
# 1. 更新 CCGO.toml
version = "2.0.0"  # 移除 -rc.1

# 2. 创建稳定版标签
ccgo tag v2.0.0 --message "稳定版 2.0.0"
```

## 构建元数据

### Git 信息

CCGO 自动包含：
- **Commit SHA**：当前 git commit 哈希
- **分支**：当前 git 分支名称
- **标签**：最近的 git 标签（如果有）
- **脏标志**：工作目录是否有未提交的更改

### 时间戳

```cpp
// ISO 8601 格式
#define MYLIB_BUILD_TIMESTAMP "2024-01-15T10:30:00Z"
```

### 构建类型

```cpp
// Release 或 Debug
#define MYLIB_BUILD_TYPE "Release"
```

## 版本查询

### 检查当前版本

```bash
# 显示 CCGO.toml 中的版本
ccgo --version

# 显示详细版本信息
ccgo version --detailed

# 输出：
# CCGO 版本：3.0.10
# 项目：mylib
# 版本：1.2.3
# Git SHA：8f3a2b1c
# Git 分支：main
# 已修改：否
```

### 运行时版本查询

```cpp
// 在您的应用程序中
#include "mylib/mylib.h"

const char* version = mylib::get_version();
printf("库版本：%s\n", version);
```

## 版本控制工作流

### 开发工作流

```bash
# 1. 开始新功能
git checkout -b feature/new-api
# CCGO.toml：version = "1.2.0"

# 2. 开发和测试
ccgo build --all
ccgo test

# 3. 合并到 main
git checkout main
git merge feature/new-api

# 4. 提升版本
# 更新 CCGO.toml：version = "1.3.0"

# 5. 创建标签
ccgo tag v1.3.0 --message "添加新 API"

# 6. 推送
git push origin main --tags
```

### 发布工作流

```bash
# 1. 创建发布分支
git checkout -b release/2.0
# CCGO.toml：version = "2.0.0-rc.1"

# 2. 测试候选发布
ccgo build --all
ccgo test --all

# 3. 如需修复错误
# ... 错误修复 ...

# 4. 提升为稳定版
# 更新 CCGO.toml：version = "2.0.0"
ccgo tag v2.0.0 --message "发布 2.0.0"

# 5. 合并回 main
git checkout main
git merge release/2.0

# 6. 推送
git push origin main --tags
```

### 热修复工作流

```bash
# 1. 从标签创建热修复分支
git checkout -b hotfix/1.2.4 v1.2.3
# CCGO.toml：version = "1.2.4"

# 2. 修复关键错误
# ... 修复 ...

# 3. 测试
ccgo build --all
ccgo test

# 4. 创建标签
ccgo tag v1.2.4 --message "热修复：关键错误"

# 5. 合并到 main 和发布分支
git checkout main
git cherry-pick hotfix/1.2.4

git checkout release/1.2
git cherry-pick hotfix/1.2.4

# 6. 推送
git push origin main release/1.2 --tags
```

## 最佳实践

### 1. 版本编号

- **从 1.0.0 开始**作为首个稳定版本
- **0.y.z** 用于初始开发（不稳定 API）
- **递增主版本号**表示破坏性更改
- **递增次版本号**表示新功能
- **递增修订号**表示错误修复

### 2. 标签管理

```bash
# 始终使用注解标签发布
git tag -a v1.0.0 -m "发布 1.0.0"

# 轻量标签仅用于内部使用
git tag build-123

# 明确推送标签
git push origin v1.0.0
```

### 3. 提交消息中的版本

```bash
# 良好的提交消息
git commit -m "chore: 提升版本至 1.2.3"
git commit -m "release: v2.0.0"
git commit -m "hotfix: v1.2.4 - 修复关键错误"
```

## 故障排除

### 版本不匹配

**问题：**
```
警告：CCGO.toml 版本（1.2.3）与 git 标签（v1.2.2）不匹配
```

**解决方案：**
```bash
# 更新 CCGO.toml 以匹配标签
# 或创建匹配 CCGO.toml 的新标签
ccgo tag --force
```

### 无效版本格式

**问题：**
```
错误：无效的版本格式："1.2.3.4.5"
```

**解决方案：**
```toml
# 使用 SemVer 格式
version = "1.2.3"  # 不是 "1.2.3.4.5"
```

## 资源

### 工具

- [语义化版本](https://semver.org/lang/zh-CN/)
- [Keep a Changelog](https://keepachangelog.com/zh-CN/)
- [约定式提交](https://www.conventionalcommits.org/zh-hans/)

### CCGO 文档

- [CLI 参考](../reference/cli.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
- [Git 集成](git-integration.zh.md)
- [发布指南](publishing.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [Git 集成](git-integration.zh.md)
- [发布指南](publishing.zh.md)
- [CI/CD 设置](../development/contributing.zh.md)
- [更新日志管理](../development/changelog.zh.md)
