# 依赖管理

CCGO 项目依赖管理的完整指南。

## 概述

CCGO 提供强大的依赖管理系统：

- **多种来源**：Git 仓库、本地路径、注册表（未来）
- **版本控制**：语义化版本与灵活约束
- **锁定文件**：使用 CCGO.lock 实现可重现构建
- **可选依赖**：基于特性的条件依赖
- **平台特定**：仅适用于特定平台的依赖
- **Vendoring**：所有依赖的本地副本
- **自动解析**：依赖树解析和冲突检测

## 依赖源

### Git 仓库

C++ 库最常见的来源：

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", branch = "master" }
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280a619340f3f76dc77292b7e9d1b1c2d83e" }
```

**Git 引用类型：**
- `tag`：特定发布标签（推荐用于稳定性）
- `branch`：跟踪分支（用于最新功能/修复）
- `rev`：精确提交哈希（最大可重现性）

**规则：**
- `tag`、`branch` 或 `rev` 只能指定一个
- Git 依赖缓存在 `~/.ccgo/git/<repo>/`
- 默认使用浅克隆以加快下载速度

### 本地路径

用于本地开发和工作空间依赖：

```toml
[dependencies]
mylib = { path = "../mylib" }
utils = { path = "./libs/utils" }
common = { path = "/absolute/path/to/common" }
```

**路径类型：**
- 相对路径：相对于当前 CCGO.toml 位置
- 绝对路径：完整文件系统路径

**使用场景：**
- 多模块项目
- 依赖的本地开发
- 第三方库的 vendor

### 注册表（未来）

未来支持包注册表：

```toml
[dependencies]
fmt = "10.1.1"              # 精确版本
spdlog = "^1.12.0"          # 兼容（>=1.12.0, <2.0.0）
boost = "~1.80"             # 次要版本（>=1.80.0, <1.81.0）
protobuf = ">=3.20"         # 最低版本
```

## 版本要求

### 精确版本

```toml
[dependencies]
fmt = "10.1.1"              # 仅版本 10.1.1
```

### 插入符（兼容）

```toml
[dependencies]
spdlog = "^1.12.0"          # >=1.12.0, <2.0.0
```

允许次要和补丁更新，无破坏性更改。

### 波浪号（次要版本）

```toml
[dependencies]
boost = "~1.80.0"           # >=1.80.0, <1.81.0
```

仅允许补丁更新。

### 比较运算符

```toml
[dependencies]
protobuf = ">=3.20.0"       # 任何版本 >= 3.20.0
openssl = ">1.1.0, <3.0.0"  # 1.1.0 和 3.0.0 之间的版本
```

### 通配符

```toml
[dependencies]
catch2 = "3.*"              # 任何 3.x 版本
```

## 管理依赖

### 添加依赖

**命令行：**

```bash
# 从 Git 添加带标签
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# 从 Git 添加带分支
ccgo add fmt --git https://github.com/fmtlib/fmt.git --branch master

# 从本地路径添加
ccgo add mylib --path ../mylib

# 从 Git 添加特定提交
ccgo add json --git https://github.com/nlohmann/json.git --rev a1b2c3d
```

**手动编辑 CCGO.toml：**

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

然后安装：
```bash
ccgo install
```

### 删除依赖

**命令行：**

```bash
ccgo remove spdlog
```

**手动：**

从 CCGO.toml 中删除，然后：
```bash
ccgo install
```

### 更新依赖

**更新所有依赖：**

```bash
ccgo update
```

**更新特定依赖：**

```bash
ccgo update spdlog
```

**试运行（显示将要更新的内容）：**

```bash
ccgo update --dry-run
```

**更新过程：**
1. 获取匹配约束的最新版本
2. 使用新版本更新 CCGO.lock
3. 下载/更新缓存副本
4. 如需要则重新构建项目

## CCGO.lock 文件

### 目的

`CCGO.lock` 通过记录以下内容确保可重现构建：
- 解析的精确依赖版本
- Git 依赖的 Git 提交哈希
- 依赖树结构
- 用于验证的校验和

### 结构

```toml
# CCGO.lock - 自动生成，请勿手动编辑

[[package]]
name = "spdlog"
version = "1.12.0"
source = "git+https://github.com/gabime/spdlog.git?tag=v1.12.0#a1b2c3d4"
checksum = "sha256:..."
dependencies = ["fmt"]

[[package]]
name = "fmt"
version = "10.1.1"
source = "git+https://github.com/fmtlib/fmt.git?tag=10.1.1#e1f2g3h4"
checksum = "sha256:..."
dependencies = []
```

### 使用锁定文件

**生成锁定文件：**

```bash
ccgo install          # 创建/更新 CCGO.lock
```

**使用锁定版本：**

```bash
ccgo install --locked # 从 CCGO.lock 安装精确版本
```

**更新锁定文件：**

```bash
ccgo update           # 使用新版本更新 CCGO.lock
```

**版本控制：**
- 应用程序**提交** CCGO.lock（可重现构建）
- 库**不提交** CCGO.lock（让用户解析）

## 可选依赖

### 定义可选依赖

```toml
[dependencies]
# 必需依赖
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 可选依赖
[dependencies.optional]
networking = { git = "https://github.com/user/networking.git", tag = "v1.0.0" }
database = { git = "https://github.com/user/database.git", tag = "v2.0.0" }
```

### 特性

使用特性启用可选依赖：

```toml
[features]
default = ["basic"]           # 默认特性
basic = []                    # 基本特性（无依赖）
network = ["networking"]      # 启用 networking 依赖
db = ["database"]             # 启用 database 依赖
full = ["basic", "network", "db"]  # 所有特性
```

### 使用特性

```bash
# 使用特定特性构建
ccgo build --features network,db

# 使用所有特性构建
ccgo build --all-features

# 不使用默认特性构建
ccgo build --no-default-features

# 使用特定特性组合构建
ccgo build --no-default-features --features network
```

## 平台特定依赖

### 条件依赖

```toml
[dependencies]
# 所有平台的通用依赖
common = { git = "https://github.com/user/common.git", tag = "v1.0.0" }

# 仅 Android 依赖
[target.'cfg(target_os = "android")'.dependencies]
android-utils = { path = "./android-utils" }

# 仅 iOS 依赖
[target.'cfg(target_os = "ios")'.dependencies]
ios-helpers = { path = "./ios-helpers" }

# 仅 Windows 依赖
[target.'cfg(target_os = "windows")'.dependencies]
windows-api = { git = "https://github.com/user/windows-api.git", tag = "v1.0.0" }

# 仅 Linux 依赖
[target.'cfg(target_os = "linux")'.dependencies]
linux-sys = { git = "https://github.com/user/linux-sys.git", tag = "v1.0.0" }
```

### 平台目标

| 平台 | 目标 | 示例 |
|------|------|------|
| Android | `target_os = "android"` | Android 特定 |
| iOS | `target_os = "ios"` | iOS 特定 |
| macOS | `target_os = "macos"` | macOS 特定 |
| Windows | `target_os = "windows"` | Windows 特定 |
| Linux | `target_os = "linux"` | Linux 特定 |
| OpenHarmony | `target_os = "ohos"` | OHOS 特定 |

## Vendoring

### 什么是 Vendoring？

Vendoring 在项目中创建所有依赖的本地副本：
- 即使上游仓库消失也能确保可用性
- 启用离线构建
- 提供对依赖源的完全控制

### Vendor 依赖

```bash
# 将所有依赖 vendor 到 vendor/ 目录
ccgo vendor

# 保留现有 vendor 目录
ccgo vendor --no-delete
```

**结果：**

```
project/
├── CCGO.toml
├── vendor/
│   ├── spdlog/          # spdlog 的完整副本
│   ├── fmt/             # fmt 的完整副本
│   └── json/            # json 的完整副本
└── src/
```

### 使用 Vendored 依赖

如果存在 `vendor/`，CCGO 自动使用 vendored 依赖：

```bash
# 使用 vendored 副本
ccgo build android

# 强制离线构建（仅 vendored 依赖）
ccgo install --offline
```

### 使用 Vendoring 的版本控制

**小型项目：**
- 提交 vendor/ 目录
- 自包含仓库

**大型项目：**
- 将 `vendor/` 添加到 `.gitignore`
- 在 README 中记录 vendoring 过程
- 考虑使用 Git LFS 处理大型 vendored 文件

## 依赖解析

### 解析算法

CCGO 使用以下算法：

1. **解析依赖树**：读取 CCGO.toml 并构建完整依赖图
2. **版本解析**：找到满足所有约束的版本
3. **冲突检测**：检测版本冲突
4. **下载**：获取缺失的依赖
5. **构建顺序**：拓扑排序确定构建顺序

### 冲突解决

**冲突示例：**

```toml
# 你的项目
[dependencies]
libA = { git = "...", tag = "v1.0.0" }  # 依赖 libC ^1.0.0
libB = { git = "...", tag = "v2.0.0" }  # 依赖 libC ^2.0.0
```

CCGO 将报告：
```
Error: Conflicting dependencies for libC:
  - libA requires libC ^1.0.0
  - libB requires libC ^2.0.0
```

**解决方案：**
1. 更新 libA 或 libB 到兼容版本
2. 使用兼容版本的分叉
3. 修改依赖约束

### 构建顺序

CCGO 以正确顺序构建依赖：

```
Project
  ├── libA（依赖 libC）
  ├── libB（依赖 libC）
  └── libC（无依赖）
```

**构建顺序：** libC → libA, libB → Project

## 最佳实践

### 1. 固定依赖版本

**好：**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

**坏：**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", branch = "master" }
```

**原因：** 标记的版本确保可重现构建。

### 2. 使用 CCGO.lock

```bash
# 生成锁定文件
ccgo install

# 提交到版本控制（用于应用程序）
git add CCGO.lock
git commit -m "Add dependency lock file"
```

### 3. 最小依赖

仅添加实际需要的依赖：
- 减少构建时间
- 简化依赖管理
- 提高安全性（更少的攻击向量）

### 4. 记录依赖

**README.md：**
```markdown
## 依赖

- spdlog (v1.12.0): 日志库
- fmt (v10.1.1): 格式化库
- nlohmann/json (v3.11.2): JSON 解析
```

### 5. 定期更新

```bash
# 每月检查更新
ccgo update --dry-run

# 测试后更新
ccgo update
ccgo build android --test
```

### 6. 安全考虑

- 添加前审查依赖源代码
- 使用官方仓库
- 检查安全公告
- 保持依赖更新
- Vendor 关键依赖

## 高级用法

### 依赖覆盖

覆盖依赖源：

```toml
[dependencies]
libA = { git = "https://github.com/user/libA.git", tag = "v1.0.0" }

# 覆盖 libA 对 libB 的依赖
[patch."https://github.com/user/libA.git"]
libB = { git = "https://github.com/me/libB-fork.git", tag = "v2.0-custom" }
```

### 私有 Git 仓库

**SSH 认证：**
```toml
[dependencies]
private-lib = { git = "git@github.com:company/private-lib.git", tag = "v1.0.0" }
```

**HTTPS 带凭据：**
```bash
# 配置 Git 凭据
git config --global credential.helper store

# 或使用 SSH 密钥
ssh-add ~/.ssh/id_rsa
```

### 工作空间依赖

用于 mono-repo 设置：

```
workspace/
├── CCGO.toml（工作空间根）
├── lib1/
│   └── CCGO.toml
├── lib2/
│   └── CCGO.toml（依赖 lib1）
└── app/
    └── CCGO.toml（依赖 lib1, lib2）
```

**lib2/CCGO.toml：**
```toml
[dependencies]
lib1 = { path = "../lib1" }
```

**app/CCGO.toml：**
```toml
[dependencies]
lib1 = { path = "../lib1" }
lib2 = { path = "../lib2" }
```

## 故障排除

### 未找到依赖

```
Error: Could not find dependency 'spdlog'
```

**解决方案：**
1. 检查仓库 URL 是否正确
2. 验证 Git 标签/分支存在
3. 检查网络连接
4. 尝试手动克隆：`git clone <url>`

### 版本冲突

```
Error: Conflicting versions for dependency 'fmt'
```

**解决方案：**
1. 运行 `ccgo update` 解决冲突
2. 手动调整版本约束
3. 使用 `ccgo vendor` 锁定特定版本

### 更新后构建失败

```
Error: Build failed after dependency update
```

**解决方案：**
1. 恢复到之前的 CCGO.lock：`git checkout CCGO.lock`
2. 安装锁定版本：`ccgo install --locked`
3. 单独测试依赖
4. 检查兼容性矩阵

### 依赖解析缓慢

```
正在解析依赖...
```

**解决方案：**
1. 使用 `--locked` 跳过解析
2. Vendor 依赖：`ccgo vendor`
3. 检查网络速度
4. 清除缓存：`rm -rf ~/.ccgo/cache/`

### Git 认证失败

```
Error: Authentication failed for Git repository
```

**解决方案：**
1. 设置 SSH 密钥：`ssh-keygen`
2. 将密钥添加到 Git 提供商
3. 或使用个人访问令牌
4. 配置 Git 凭据管理器

## 依赖生态系统

### 流行的 C++ 依赖

**日志：**
- spdlog: https://github.com/gabime/spdlog
- glog: https://github.com/google/glog

**格式化：**
- fmt: https://github.com/fmtlib/fmt

**JSON：**
- nlohmann/json: https://github.com/nlohmann/json
- RapidJSON: https://github.com/Tencent/rapidjson

**测试：**
- Catch2: https://github.com/catchorg/Catch2
- Google Test: https://github.com/google/googletest

**网络：**
- Boost.Asio: https://github.com/boostorg/asio
- cpp-httplib: https://github.com/yhirose/cpp-httplib

**序列化：**
- protobuf: https://github.com/protocolbuffers/protobuf
- flatbuffers: https://github.com/google/flatbuffers

### 查找依赖

- **GitHub**：搜索 C++ 库
- **Awesome C++**：https://github.com/fffaraz/awesome-cpp
- **Conan Center**：https://conan.io/center/
- **vcpkg**：https://github.com/microsoft/vcpkg

## 迁移指南

### 从 Conan

**conanfile.txt：**
```ini
[requires]
spdlog/1.12.0
fmt/10.1.1

[options]
spdlog:shared=False
```

**CCGO.toml：**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[library]
type = "static"
```

### 从 vcpkg

**vcpkg.json：**
```json
{
  "dependencies": [
    "spdlog",
    "fmt",
    "nlohmann-json"
  ]
}
```

**CCGO.toml：**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }
json = { git = "https://github.com/nlohmann/json.git", tag = "v3.11.2" }
```

### 从 Git Submodules

**替换：**
```bash
git submodule add https://github.com/gabime/spdlog.git third_party/spdlog
```

**为：**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

然后：
```bash
ccgo install
```

## 另请参阅

- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
- [构建系统](build-system.zh.md)
- [发布](publishing.zh.md)
- [项目结构](../getting-started/project-structure.zh.md)
