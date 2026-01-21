# 从 Conan 迁移到 CCGO

> 版本：v3.0.10 | 更新时间：2026-01-21

## 概述

本指南帮助您将 C++ 项目从 [Conan](https://conan.io/) 迁移到 CCGO。两者都是 C++ 包管理器，但 CCGO 提供了更集成的跨平台构建系统，针对移动和嵌入式平台进行了优化。

### 为什么要迁移到 CCGO？

| 功能 | Conan | CCGO |
|------|-------|------|
| **跨平台构建** | 每个平台手动设置 | 单条命令支持 8+ 平台 |
| **移动平台支持** | Android/iOS 需要复杂设置 | 一流的 Android、iOS、OpenHarmony 支持 |
| **配置文件** | conanfile.py/txt + CMake | 统一的 CCGO.toml |
| **发布** | Conan Center、自定义服务器 | Maven、CocoaPods、SPM、OHPM、Conan |
| **Docker 构建** | 手动设置 | 内置通用交叉编译 |
| **依赖锁定** | conan.lock | CCGO.lock |
| **渐进式迁移** | 不适用 | 可以将 Conan 包作为 CCGO 依赖使用 |

### 迁移工作量

**典型小项目**：1-2 小时
**有 10+ 依赖的中型项目**：4-8 小时
**有自定义 Conan 配方的大型项目**：1-2 天

---

## 快速对比

### Conan vs CCGO 概念

| Conan 概念 | CCGO 等价物 | 说明 |
|-----------|-------------|------|
| `conanfile.txt` | `CCGO.toml` | TOML 格式的依赖 |
| `conanfile.py` | `CCGO.toml` + CMakeLists.txt | 构建逻辑移至 CMake |
| `conan install` | `ccgo install` | 安装依赖 |
| `conan create` | `ccgo build` | 构建包 |
| `conan upload` | `ccgo publish` | 发布到仓库 |
| Conan Center | CCGO Registry（计划中） | 目前使用 Git 依赖 |
| `requires` | `[dependencies]` | 依赖声明 |
| `tool_requires` | 不适用 | 使用系统工具或 Docker |
| Profile | 平台标志 | `--arch`、`--toolchain` 等 |
| Generator | CMake 集成 | 直接 CMake 模块 |
| Package recipe | CCGO.toml + CMakeLists.txt | 更简单的声明式格式 |

---

## 迁移路径

### 路径 1：仅简单依赖（conanfile.txt）

**最适合**：仅使用包的项目，无自定义配方

**步骤**：
1. 转换 `conanfile.txt` → `CCGO.toml`
2. 更新 CMakeLists.txt 包含路径
3. 测试构建

**时间**：30 分钟 - 2 小时

---

### 路径 2：自定义包配方（conanfile.py）

**最适合**：有自定义 Conan 包的项目

**步骤**：
1. 提取依赖列表 → `CCGO.toml` `[dependencies]`
2. 转换构建逻辑 → `CMakeLists.txt`
3. 更新发布配置 → `CCGO.toml` `[publish]`
4. 测试构建和发布

**时间**：2-8 小时

---

### 路径 3：混合方法（渐进式迁移）

**最适合**：大型项目，最小化中断

**步骤**：
1. 保留 Conan 处理大多数依赖
2. 使用 CCGO 进行跨平台构建
3. 逐步用 CCGO/Git 依赖替换 Conan 依赖
4. 准备就绪后移除 Conan

**时间**：分散在数周/数月

---

## 分步迁移

### 步骤 1：分析当前 Conan 设置

#### 识别依赖来源

```bash
# 列出所有 Conan 依赖
conan info . --only requires

# 检查哪些依赖来自 Conan Center
conan search <package> --remote=conancenter

# 识别自定义配方（本地或私有服务器）
conan search --remote=all
```

**对依赖进行分类**：
- **Conan Center**：可能有 Git 替代方案
- **自定义配方**：需要移植或保留为 Conan
- **系统库**：可以使用系统库或从源代码构建

---

### 步骤 2：创建 CCGO.toml

#### 从 conanfile.txt 转换

**之前（conanfile.txt）**：
```ini
[requires]
fmt/10.1.1
spdlog/1.12.0
boost/1.82.0

[generators]
cmake_find_package
cmake_paths

[options]
fmt:shared=False
boost:shared=False
```

**之后（CCGO.toml）**：
```toml
[package]
name = "myproject"
version = "1.0.0"

[dependencies]
# 选项 1：使用 Conan 包（混合模式）
fmt = { version = "10.1.1", source = "conan" }
spdlog = { version = "1.12.0", source = "conan" }

# 选项 2：使用 Git 仓库
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }
spdlog = { git = "https://github.com/gabime/spdlog", tag = "v1.12.0" }

# 选项 3：混合使用
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }
boost = { version = "1.82.0", source = "conan" }  # 对复杂依赖保留 Conan

[build]
cmake_version = "3.22.1"
```

**Git 依赖的优势**：
- ✅ 不需要 Conan 服务器
- ✅ 锁定确切的 commit/tag
- ✅ 使用你的确切标志从源代码构建
- ❌ 首次构建较慢（无二进制缓存）

---

#### 从 conanfile.py 转换

**之前（conanfile.py）**：
```python
from conan import ConanFile
from conan.tools.cmake import CMake, cmake_layout

class MyProjectConan(ConanFile):
    name = "myproject"
    version = "1.0.0"
    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False]}
    default_options = {"shared": False}
    exports_sources = "CMakeLists.txt", "src/*", "include/*"

    def requirements(self):
        self.requires("fmt/10.1.1")
        if self.options.shared:
            self.requires("spdlog/1.12.0")

    def layout(self):
        cmake_layout(self)

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        cmake = CMake(self)
        cmake.install()
```

**之后（CCGO.toml）**：
```toml
[package]
name = "myproject"
version = "1.0.0"
description = "My C++ project"
license = "MIT"

[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

[dependencies.spdlog]
git = "https://github.com/gabime/spdlog"
tag = "v1.12.0"
optional = true

[features]
default = []
with-logging = ["spdlog"]  # 条件依赖

[build]
cmake_version = "3.22.1"
link_type = "static"  # 或 "shared"

[android]
min_sdk = 21
compile_sdk = 34
default_archs = ["armeabi-v7a", "arm64-v8a", "x86_64"]

[ios]
deployment_target = "13.0"

[publish.maven]
group_id = "com.example"
artifact_id = "myproject"
```

**注意**：构建逻辑（CMake 配置/构建/安装）移至您的 `CMakeLists.txt` - 见步骤 3。

---

### 步骤 3：更新 CMake 集成

#### Conan CMake 集成

**之前（使用 Conan）**：
```cmake
cmake_minimum_required(VERSION 3.15)
project(MyProject)

# Conan 集成（多种方法）

# 选项 1：cmake-conan
include(${CMAKE_BINARY_DIR}/conan.cmake)
conan_cmake_configure(REQUIRES fmt/10.1.1
                      GENERATORS cmake_find_package)
conan_cmake_install(...)

# 选项 2：CMakeDeps + CMakeToolchain 生成器
find_package(fmt REQUIRED)
find_package(spdlog REQUIRED)

add_executable(myapp main.cpp)
target_link_libraries(myapp fmt::fmt spdlog::spdlog)
```

---

#### CCGO CMake 集成

**之后（使用 CCGO）**：
```cmake
cmake_minimum_required(VERSION 3.18)
project(MyProject VERSION 1.0.0)

# CCGO 集成
include(${CCGO_CMAKE_DIR}/CMakeUtils.cmake)
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

# 收集源文件（CCGO 辅助函数）
add_sub_layer_sources_recursively(MYAPP_SOURCES ${CMAKE_CURRENT_SOURCE_DIR}/src)

# 创建库/可执行文件
add_library(myproject STATIC ${MYAPP_SOURCES})

# 添加所有依赖的包含路径
ccgo_add_dependencies(myproject)

# 链接特定依赖（CCGO 将在已安装的依赖中找到它们）
ccgo_link_dependency(myproject fmt fmt)
ccgo_link_dependency(myproject spdlog spdlog)

# 或者如果依赖提供了 find_package，使用 CMake 的 find_package
find_package(fmt REQUIRED)
find_package(spdlog REQUIRED)
target_link_libraries(myproject PRIVATE fmt::fmt spdlog::spdlog)
```

**主要区别**：
- `${CCGO_CMAKE_DIR}` 由 ccgo 构建系统设置
- `ccgo_add_dependencies()` 添加所有依赖的包含路径
- `ccgo_link_dependency()` 查找并链接特定库
- 比 Conan 更简单，样板代码更少

---

### 步骤 4：安装依赖

```bash
# Conan
conan install . --output-folder=build --build=missing

# CCGO
ccgo install
# 读取 CCGO.toml，安装到 .ccgo/deps/
# 生成 CCGO.lock 用于版本锁定
```

**CCGO 依赖安装**：
- Git 依赖：克隆到 `.ccgo/deps/<name>`
- Conan 依赖（混合模式）：内部使用 `conan install`
- 锁文件：`CCGO.lock` 确保可重现构建

---

### 步骤 5：更新构建命令

#### 平台构建

**Conan**：
```bash
# Android（复杂设置）
conan install . --profile=android-armv8 --build=missing
conan build .

# iOS（复杂设置）
conan install . --profile=ios-armv8 --build=missing
conan build .
```

**CCGO**：
```bash
# Android
ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64

# iOS
ccgo build ios

# macOS
ccgo build macos

# 使用 Docker 的所有平台（任何主机操作系统！）
ccgo build android --docker
ccgo build ios --docker
ccgo build windows --docker
```

**CCGO 优势**：
- ✅ 每个平台一条命令
- ✅ 自动工具链设置
- ✅ 基于 Docker 的通用交叉编译
- ✅ 并行架构构建

---

### 步骤 6：更新发布

#### 发布到 Maven（Android）

**Conan**：
```bash
# 自定义 conanfile.py deploy() 方法
conan create . --profile=android-armv8
conan upload myproject/1.0.0 --remote=myremote
```

**CCGO**：
```bash
# 在 CCGO.toml 中配置一次
[publish.maven]
group_id = "com.example"
artifact_id = "myproject"

# 发布到 Maven Central
ccgo publish android --registry official

# 发布到自定义 Maven
ccgo publish android --registry private --url https://maven.example.com
```

---

#### 发布到 CocoaPods（iOS）

**Conan**：不直接支持，需要自定义脚本

**CCGO**：
```bash
# 在 CCGO.toml 中配置一次
[publish.cocoapods]
summary = "My C++ library"
homepage = "https://github.com/user/myproject"

# 发布
ccgo publish apple --manager cocoapods
```

---

## 常见迁移场景

### 场景 1：仅头文件库（例如 {fmt}）

**Conan conanfile.py**：
```python
class FmtConan(ConanFile):
    name = "fmt"
    version = "10.1.1"
    # 仅头文件，无 build()

    def package(self):
        self.copy("*.h", dst="include", src="include")
```

**CCGO CCGO.toml**：
```toml
[package]
name = "fmt-wrapper"
version = "10.1.1"

[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

# CMakeLists.txt 处理包含路径设置
```

**CMakeLists.txt**：
```cmake
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)

add_library(fmt INTERFACE)
ccgo_add_dependencies(fmt)  # 添加 fmt 包含路径

# 或使用 CMake 的 find_package
add_subdirectory(.ccgo/deps/fmt)
target_link_libraries(myapp PRIVATE fmt::fmt-header-only)
```

---

### 场景 2：带构建选项的库

**Conan conanfile.py**：
```python
class MyLibConan(ConanFile):
    options = {
        "shared": [True, False],
        "with_ssl": [True, False]
    }
    default_options = {
        "shared": False,
        "with_ssl": True
    }

    def requirements(self):
        if self.options.with_ssl:
            self.requires("openssl/3.1.0")
```

**CCGO CCGO.toml**：
```toml
[package]
name = "mylib"

[dependencies]
openssl = { git = "https://github.com/openssl/openssl", tag = "openssl-3.1.0", optional = true }

[features]
default = ["ssl"]
ssl = ["openssl"]

[build]
link_type = "static"  # 或 "shared"
```

**有/无 SSL 构建**：
```bash
# 有 SSL（默认）
ccgo build android

# 无 SSL
ccgo build android --no-default-features
```

---

### 场景 3：多包工作区

**Conan**（每个包单独的 conanfile.py）：
```
myworkspace/
├── core/conanfile.py      # 不依赖任何东西
├── utils/conanfile.py     # 依赖 core
└── app/conanfile.py       # 依赖 utils
```

**CCGO**（根目录单个 CCGO.toml）：
```toml
[workspace]
members = ["core", "utils", "app"]
resolver = "2"

# 工作区级别的依赖（被所有成员继承）
[workspace.dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }
```

**core/CCGO.toml**：
```toml
[package]
name = "core"
version = "1.0.0"

[dependencies]
fmt = { workspace = true }  # 从工作区继承
```

**utils/CCGO.toml**：
```toml
[package]
name = "utils"
version = "1.0.0"

[dependencies]
core = { path = "../core" }  # 本地依赖
fmt = { workspace = true }
```

---

### 场景 4：私有 Conan 服务器

**Conan**：
```bash
# 配置远程仓库
conan remote add mycompany https://conan.example.com
conan remote login mycompany admin -p password

# 使用包
conan install . --remote=mycompany
```

**CCGO 混合模式**（保留 Conan 处理私有依赖）：
```toml
[dependencies]
# 公共包：使用 Git
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

# 私有包：使用 Conan
internal-lib = { version = "2.3.0", source = "conan", remote = "mycompany" }
```

**或迁移到 Git**（推荐）：
```toml
[dependencies]
# 在 Git 上托管私有包（GitHub/GitLab/Bitbucket）
internal-lib = { git = "https://github.com/mycompany/internal-lib", tag = "v2.3.0" }
```

---

## 依赖映射

### 流行的 Conan 包 → CCGO 等价物

| Conan 包 | CCGO 推荐方法 |
|---------|-------------|
| `fmt` | `{ git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }` |
| `spdlog` | `{ git = "https://github.com/gabime/spdlog", tag = "v1.12.0" }` |
| `catch2` | `{ git = "https://github.com/catchorg/Catch2", tag = "v3.4.0" }` |
| `gtest` | `{ git = "https://github.com/google/googletest", tag = "v1.14.0" }` |
| `nlohmann_json` | `{ git = "https://github.com/nlohmann/json", tag = "v3.11.2" }` |
| `boost` | 保留 Conan 或使用系统包（太复杂，不适合 Git） |
| `openssl` | 保留 Conan 或使用系统包（安全更新） |
| `protobuf` | `{ git = "https://github.com/protocolbuffers/protobuf", tag = "v23.4" }` |
| `grpc` | 保留 Conan（复杂构建）或使用系统包 |
| `sqlite3` | `{ git = "https://github.com/sqlite/sqlite", tag = "version-3.42.0" }` |

---

## 故障排除

### 问题：CCGO 中缺少 Conan 包

**问题**：Conan Center 中有的包，但没有 Git 仓库

**解决方案**：
1. **混合模式**：对该包继续使用 Conan
   ```toml
   [dependencies]
   mypackage = { version = "1.0.0", source = "conan" }
   ```

2. **查找 Git 源**：在 GitHub/GitLab 上搜索上游
   ```bash
   # 例如：protobuf 在 GitHub 上
   https://github.com/protocolbuffers/protobuf
   ```

3. **内置代码**：将源代码复制到您的项目中
   ```toml
   [dependencies]
   myvendored = { path = "third_party/myvendored" }
   ```

---

### 问题：conanfile.py 中的复杂构建配方

**问题**：`build()` 方法中的自定义构建逻辑

**解决方案**：将逻辑移至 `CMakeLists.txt`

**Conan conanfile.py**：
```python
def build(self):
    cmake = CMake(self)
    cmake.definitions["CUSTOM_OPTION"] = "ON"
    cmake.definitions["BUILD_SHARED"] = self.options.shared
    cmake.configure(source_folder="subfolder")
    cmake.build(target="mylib")
```

**CCGO CMakeLists.txt**：
```cmake
cmake_minimum_required(VERSION 3.18)

# 直接设置选项
option(CUSTOM_OPTION "Custom option" ON)
option(BUILD_SHARED_LIBS "Build shared" OFF)

# 配置子文件夹
add_subdirectory(subfolder)

# 构建目标
add_library(mylib ...)
```

---

### 问题：Conan 生成器不可用

**问题**：CCGO 中缺少 `cmake_find_package` 生成器

**解决方案**：使用 CCGO 的 CMake 集成

**之前**：
```cmake
find_package(fmt REQUIRED)  # 由 Conan 生成
```

**之后**：
```cmake
include(${CCGO_CMAKE_DIR}/CCGODependencies.cmake)
ccgo_add_dependencies(myapp)
ccgo_link_dependency(myapp fmt fmt)
```

---

### 问题：Profile 配置

**问题**：Conan profile 指定编译器、设置

**Conan ~/.conan/profiles/android-armv8**：
```ini
[settings]
os=Android
os.api_level=21
arch=armv8
compiler=clang
compiler.version=14
compiler.libcxx=c++_shared
build_type=Release
```

**CCGO**：平台设置在 CCGO.toml 中，架构在命令中

**CCGO.toml**：
```toml
[android]
min_sdk = 21
compile_sdk = 34
stl = "c++_shared"
```

**命令**：
```bash
ccgo build android --arch arm64-v8a --config release
```

---

## 最佳实践

### 1. 渐进式迁移策略

**✅ 应该**：从混合模式开始（Conan + CCGO）
```toml
[dependencies]
# 简单依赖：迁移到 Git
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

# 复杂依赖：暂时保留 Conan
boost = { version = "1.82.0", source = "conan" }
```

**✅ 应该**：逐个包、逐个冲刺迁移

---

### 2. 锁定依赖

**✅ 应该**：将 CCGO.lock 提交到版本控制
```bash
ccgo install       # 生成 CCGO.lock
git add CCGO.lock  # 锁定确切版本
git commit -m "Lock dependency versions"
```

---

### 3. 尽早测试跨平台构建

**✅ 应该**：迁移后验证所有目标平台
```bash
ccgo build android --arch arm64-v8a
ccgo build ios
ccgo build macos
ccgo build windows --docker  # 在 macOS/Linux 上测试
```

---

### 4. 记录自定义补丁

**✅ 应该**：在迁移笔记中记录任何 Conan 包补丁
```toml
[dependencies]
mypackage = { git = "https://github.com/user/mypackage-fork", branch = "custom-fixes" }
# 带 Android NDK r25 修复的自定义分支
```

---

### 5. 使用 Docker 确保可重现性

**✅ 应该**：在 CI/CD 中使用 Docker 构建
```yaml
# .github/workflows/build.yml
- name: Build for all platforms
  run: |
    ccgo build android --docker --arch arm64-v8a,armeabi-v7a,x86_64
    ccgo build ios --docker
    ccgo build macos --docker
```

---

## 迁移检查清单

使用此检查清单跟踪您的迁移进度：

### 迁移前

- [ ] 审核所有 Conan 依赖
- [ ] 识别自定义 Conan 配方
- [ ] 检查平台特定的构建配置
- [ ] 记录当前构建工作流程
- [ ] 设置测试环境

### 迁移中

- [ ] 从 `conanfile.txt`/`conanfile.py` 创建 `CCGO.toml`
- [ ] 转换依赖声明
- [ ] 更新 `CMakeLists.txt` 包含路径
- [ ] 测试依赖安装（`ccgo install`）
- [ ] 验证所有平台上的构建
- [ ] 测试发布工作流程（如适用）
- [ ] 更新 CI/CD 管道
- [ ] 更新开发者文档

### 迁移后

- [ ] 删除 Conan 文件（`conanfile.*`、`conan.lock`）
- [ ] 删除与 Conan 相关的 CI/CD 步骤
- [ ] 归档 Conan 配置以供参考
- [ ] 培训团队 CCGO 工作流程
- [ ] 监控构建性能
- [ ] 收集团队反馈

---

## 性能对比

### 构建时间（示例项目：10 个依赖）

| 操作 | Conan | CCGO | 说明 |
|------|-------|------|------|
| 安装依赖（首次） | 5-10 分钟 | 3-7 分钟 | CCGO 从源代码构建 |
| 安装依赖（已缓存） | 30 秒 | 10 秒 | CCGO 使用 Git 缓存 |
| Android 构建 | 2 分钟 | 90 秒 | CCGO 并行架构 |
| iOS 构建 | 3 分钟 | 2 分钟 | CCGO 优化的工具链 |
| 跨平台（4 个平台） | 15 分钟 | 6 分钟 | CCGO Docker 并行 |

*时间因项目大小和硬件而异*

---

## 其他资源

- [Conan 官方文档](https://docs.conan.io/2/)
- [CCGO CLI 参考](../reference/cli.md)
- [CCGO.toml 配置](../reference/config.zh.md)
- [CMake 集成指南](cmake-integration.zh.md)
- [CCGO 依赖管理](../features/dependency-management.zh.md)

**社区支持**：
- [CCGO GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [CCGO Discord](https://discord.gg/ccgo)（即将推出）

---

## 常见问题

### Q: 我可以在 CCGO 旁边继续使用 Conan 吗？

**A**：可以！CCGO 支持混合模式：
```toml
[dependencies]
conan-package = { version = "1.0.0", source = "conan" }
git-package = { git = "https://github.com/user/package" }
```

---

### Q: 二进制包呢？Conan 有预构建的二进制文件。

**A**：CCGO 目前从源代码构建以获得最大灵活性。好处：
- ✅ 精确的编译器/标志控制
- ✅ 无 ABI 兼容性问题
- ✅ 安全性（自行构建）
- ❌ 首次构建较慢（通过缓存缓解）

未来：CCGO Registry 计划支持二进制缓存。

---

### Q: 我可以从 CCGO 发布到 Conan Center 吗？

**A**：不能直接发布。要发布到 Conan Center：
1. 保留用于发布的 `conanfile.py`
2. 使用 CCGO 进行开发构建
3. 导出到 Conan：`conan create . --profile=...`

或逐步将消费者迁移到 CCGO。

---

### Q: 如何在 CCGO 中处理 Conan 选项？

**A**：使用 CCGO 特性：

**Conan**：
```python
options = {"shared": [True, False], "with_ssl": [True, False]}
```

**CCGO**：
```toml
[features]
shared = []
ssl = ["openssl"]
```

```bash
ccgo build --features shared,ssl
```

---

### Q: `tool_requires`（构建工具）怎么办？

**A**：CCGO 使用系统工具或 Docker：

**Conan**：
```python
tool_requires = ["cmake/3.24.0", "ninja/1.11.1"]
```

**CCGO**：
```bash
# 通过系统包管理器安装工具
brew install cmake ninja  # macOS
apt install cmake ninja-build  # Ubuntu

# 或使用 Docker（包含工具）
ccgo build --docker
```

---

## 总结

从 Conan 迁移到 CCGO 简化了跨平台 C++ 开发：

**主要优势**：
1. ✅ **统一配置**：单个 `CCGO.toml` vs 多个文件
2. ✅ **移动优先**：Android、iOS、OpenHarmony 开箱即用
3. ✅ **更简单的 CMake**：更少的样板代码，更清晰的依赖关系
4. ✅ **Docker 集成**：通用交叉编译
5. ✅ **现代发布**：Maven、CocoaPods、SPM 支持

**迁移工作量**：大多数项目通常为 1-8 小时

**建议**：从混合模式开始，逐步迁移

---

**来源**：
- [GitHub - conan-io/conan: Conan - The open-source C and C++ package manager](https://github.com/conan-io/conan)
- [Conan 2 - C and C++ Package Manager Documentation](https://docs.conan.io/2/)
- [GitHub - conan-io/cmake-conan: CMake wrapper for conan C and C++ package manager](https://github.com/conan-io/cmake-conan)

---

*本指南是 CCGO 文档的一部分。如有问题或改进建议，请在 [GitHub](https://github.com/zhlinh/ccgo/issues) 上提出 issue。*
