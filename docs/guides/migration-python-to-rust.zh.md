# 从 Python CLI 迁移到 Rust CLI

> 版本：v3.1.0 | 更新时间：2026-01-21

## 概述

CCGO v3.1 引入了新的基于 Rust 的 CLI，替代了 Python 实现（v3.0）。Rust CLI 提供了更好的性能、更简单的安装（单一二进制文件）和改进的错误处理，同时与大多数 v3.0 命令保持 API 兼容性。

### 为什么要迁移到 Rust CLI？

| 功能 | Python CLI (v3.0) | Rust CLI (v3.1+) |
|------|-------------------|------------------|
| **性能** | Python 解释器开销 | 原生二进制（快 2-5 倍） |
| **安装** | `pip install ccgo` + 依赖 | 单一二进制下载 |
| **启动时间** | ~500ms | ~10ms |
| **内存使用** | 50-100MB | 10-20MB |
| **依赖** | Python 3.8+、pip、系统库 | 无（静态二进制） |
| **错误消息** | Python 堆栈跟踪 | 用户友好的错误提示 |
| **类型安全** | 运行时错误 | 编译时验证 |
| **分发** | PyPI | GitHub Releases + Cargo |

### 迁移工作量

**大多数项目**：0-30 分钟（直接替换）
**有自定义脚本的项目**：1-3 小时（更新路径/调用）
**使用 Python API 的项目**：2-8 小时（移植到 Rust 或保留两者）

---

## 兼容性状态

### 完全兼容的命令（无需更改）

✅ 这些命令在 Rust CLI 中工作方式相同：

- `ccgo build <platform>` - 所有平台支持
- `ccgo test` - 测试执行
- `ccgo bench` - 基准测试执行
- `ccgo doc` - 文档生成
- `ccgo clean` - 构建产物清理
- `ccgo check <platform>` - 依赖检查
- `ccgo install` - 依赖安装
- `ccgo tag` - 版本标记
- `ccgo package` - 源代码打包

### 有细微差异的兼容命令

⚠️ 这些命令可以工作但有轻微的行为变化：

- `ccgo publish` - 相同的标志，改进的进度显示
- `ccgo new` / `ccgo init` - 相同的接口，更快的模板生成
- `ccgo --version` - 不同的版本格式（`v3.1.0` vs `3.0.10`）

### 尚未实现（使用 Python CLI）

❌ 这些命令已计划但尚未在 Rust CLI v3.1 中实现：

- `ccgo vendor` - 依赖 vendoring（计划在 v3.2）
- `ccgo update` - 依赖更新（计划在 v3.2）
- `ccgo run` - 运行示例/二进制文件（计划在 v3.2）
- `ccgo ci` - CI 编排（计划在 v3.3）

---

## 安装

### 选项 1：在 Python CLI 旁边安装 Rust CLI

**推荐用于渐进式迁移**

```bash
# 保留 Python CLI
pip install ccgo  # v3.0.x

# 安装 Rust CLI 为 ccgo-rs
cargo install ccgo-rs --locked
# 或从 GitHub Releases 下载二进制文件

# 使用 Python CLI
ccgo build android  # Python（默认）

# 显式使用 Rust CLI
ccgo-rs build android  # Rust

# 或使用完整路径
~/.cargo/bin/ccgo build android  # Rust
```

### 选项 2：用 Rust CLI 替换 Python CLI

**适用于准备完全迁移的项目**

```bash
# 卸载 Python CLI
pip uninstall ccgo

# 安装 Rust CLI 为 ccgo
cargo install ccgo --locked
# 或符号链接：ln -s ~/.cargo/bin/ccgo-rs ~/.cargo/bin/ccgo

# 验证
which ccgo  # 应指向 Rust 二进制文件
ccgo --version  # 应显示 v3.1.0+
```

---

## 分步迁移

### 步骤 1：验证当前设置

迁移前，记录当前 Python CLI 设置：

```bash
# 检查 Python CLI 版本
ccgo --version
# 输出：ccgo 3.0.10 (Python 3.11.5)

# 列出已安装的工具
which ccgo python pip

# 检查 CCGO.toml 版本
grep "^version" CCGO.toml

# 测试简单构建
ccgo build android --arch arm64-v8a
```

---

### 步骤 2：安装 Rust CLI（测试模式）

在 Python CLI 旁边安装以进行测试：

```bash
# 安装 Rust 工具链（如果尚未安装）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 Rust CLI
cargo install ccgo-rs --locked

# 测试 Rust CLI
ccgo-rs --version
# 输出：ccgo v3.1.0 (rust 1.75.0)

# 测试构建
ccgo-rs build android --arch arm64-v8a
```

---

### 步骤 3：更新脚本和自动化

#### CI/CD 工作流

**之前（Python CLI）**：
```yaml
# .github/workflows/build.yml
name: Build
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install CCGO
        run: pip install ccgo

      - name: Build for Android
        run: ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64
```

**之后（Rust CLI）**：
```yaml
# .github/workflows/build.yml
name: Build
on: [push]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install CCGO
        run: cargo install ccgo --locked
        # 或使用预构建的二进制文件：
        # run: |
        #   curl -LO https://github.com/zhlinh/ccgo/releases/download/v3.1.0/ccgo-linux-x86_64
        #   chmod +x ccgo-linux-x86_64
        #   sudo mv ccgo-linux-x86_64 /usr/local/bin/ccgo

      - name: Build for Android
        run: ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64
```

**好处**：
- ✅ 更快的安装（二进制 vs pip）
- ✅ 无 Python 依赖
- ✅ 更好的缓存支持

---

#### 本地构建脚本

**之前（build.sh - Python CLI）**：
```bash
#!/bin/bash
set -e

# 确保 Python CLI 可用
if ! command -v ccgo &> /dev/null; then
    echo "Installing ccgo..."
    pip install ccgo
fi

# 为所有平台构建
ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64
ccgo build ios
ccgo build macos
```

**之后（build.sh - Rust CLI）**：
```bash
#!/bin/bash
set -e

# 确保 Rust CLI 可用
if ! command -v ccgo &> /dev/null; then
    echo "Installing ccgo (Rust CLI)..."
    cargo install ccgo --locked
fi

# 为所有平台构建（相同的命令！）
ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64
ccgo build ios
ccgo build macos
```

---

#### Gradle 集成

**之前（Python CLI - build.gradle.kts）**：
```kotlin
// 使用 Python ccgo CLI 构建原生库
tasks.register<Exec>("buildNativeLibraries") {
    workingDir = rootProject.projectDir.parentFile
    commandLine("ccgo", "build", "android", "--arch", "arm64-v8a,armeabi-v7a,x86_64", "--native-only")
}
```

**之后（Rust CLI - build.gradle.kts）**：
```kotlin
// 使用 Rust ccgo CLI 构建原生库
tasks.register<Exec>("buildNativeLibraries") {
    workingDir = rootProject.projectDir.parentFile

    // 自动检测 ccgo 或 ccgo-rs
    val ccgoCmd = if (File("${System.getenv("HOME")}/.cargo/bin/ccgo").exists()) {
        "ccgo"
    } else if (File("${System.getenv("HOME")}/.cargo/bin/ccgo-rs").exists()) {
        "ccgo-rs"
    } else {
        "ccgo"  // 回退（如果未安装将失败）
    }

    commandLine(ccgoCmd, "build", "android", "--arch", "arm64-v8a,armeabi-v7a,x86_64", "--native-only")
}
```

**或使用显式路径**：
```kotlin
tasks.register<Exec>("buildNativeLibraries") {
    workingDir = rootProject.projectDir.parentFile
    commandLine("${System.getenv("HOME")}/.cargo/bin/ccgo", "build", "android", ...)
}
```

---

### 步骤 4：测试所有工作流程

使用 Rust CLI 系统地测试每个工作流程：

```bash
# 测试依赖安装
ccgo-rs install
diff -r .ccgo/deps_python .ccgo/deps_rust  # 如需要可以比较

# 测试构建
for platform in android ios macos windows linux; do
    echo "Testing $platform..."
    ccgo-rs build $platform
done

# 测试测试和基准测试
ccgo-rs test
ccgo-rs bench

# 测试文档
ccgo-rs doc --open

# 测试发布（干运行）
ccgo-rs publish android --registry local --skip-build

# 测试清理
ccgo-rs clean --dry-run
```

---

### 步骤 5：更新文档

更新项目文档以引用 Rust CLI：

**README.md**：
```markdown
## 安装

### CCGO CLI（Rust - 推荐）

```bash
cargo install ccgo --locked
```

或从 [Releases](https://github.com/zhlinh/ccgo/releases) 下载预构建的二进制文件。

### 旧版 Python CLI（已弃用）

```bash
pip install ccgo  # 仅 v3.0.x
```
```

**CONTRIBUTING.md**：
```markdown
## 构建项目

### 前置要求

- Rust 1.75+（`rustup install stable`）
- CCGO CLI：`cargo install ccgo --locked`

### 构建命令

```bash
# Android
ccgo build android --arch arm64-v8a

# iOS
ccgo build ios

# 所有平台
ccgo build --all
```
```

---

### 步骤 6：切换到 Rust CLI

测试完成后：

```bash
# 删除 Python CLI
pip uninstall ccgo

# 重命名/符号链接 Rust CLI
ln -sf ~/.cargo/bin/ccgo-rs ~/.cargo/bin/ccgo

# 或直接重新安装为 'ccgo'
cargo install ccgo --locked

# 验证
ccgo --version  # 应显示 v3.1.0+
```

---

## API 兼容性

### 命令行接口

**100% 兼容**：
```bash
# 这些在两个 CLI 中的工作方式相同
ccgo build android --arch arm64-v8a
ccgo build ios --ide-project
ccgo build windows --docker --toolchain msvc
ccgo test --filter MyTest
ccgo clean -y
ccgo install
ccgo tag v1.2.3
```

**细微差异**：

| 命令 | Python CLI | Rust CLI | 说明 |
|------|-----------|----------|------|
| `--version` | `ccgo 3.0.10 (Python 3.11)` | `ccgo v3.1.0 (rust 1.75)` | 格式变化 |
| 进度 | 文本输出 | 进度条 + 颜色 | 更好的用户体验 |
| 错误 | Python 回溯 | 结构化错误 | 更易读 |
| `--help` | argparse 格式 | clap 格式 | 布局略有不同 |

---

### CCGO.toml 配置

**100% 兼容**：Rust CLI 读取相同的 `CCGO.toml` 格式。

```toml
[package]
name = "myproject"
version = "1.0.0"

[dependencies]
fmt = { git = "https://github.com/fmtlib/fmt", tag = "10.1.1" }

[android]
min_sdk = 21
compile_sdk = 34
```

切换 CLI 时 **无需更改** CCGO.toml。

---

### 构建输出结构

**100% 兼容**：Rust CLI 生成相同的输出结构。

```
target/
├── android/
│   ├── arm64-v8a/
│   │   └── libmyproject.so
│   └── armeabi-v7a/
│       └── libmyproject.so
├── ios/
│   └── MyProject.framework/
└── macos/
    └── libmyproject.dylib
```

**归档命名**相同：`MYPROJECT_ANDROID_SDK-1.0.0.zip`

---

## 故障排除

### 问题：安装后找不到 Rust CLI

**症状**：
```bash
ccgo --version
# ccgo: command not found
```

**解决方案**：
```bash
# 检查 Cargo bin 目录
ls ~/.cargo/bin/ccgo*

# 添加到 PATH（如果尚未添加）
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  # 或 ~/.zshrc
source ~/.bashrc

# 或使用完整路径
~/.cargo/bin/ccgo --version
```

---

### 问题：版本检查显示 Python CLI

**症状**：
```bash
ccgo --version
# ccgo 3.0.10 (Python 3.11.5)  # 仍然是 Python！
```

**解决方案**：
```bash
# 检查正在使用哪个 ccgo
which ccgo
# /usr/local/bin/ccgo  # Python pip 安装位置

type -a ccgo
# ccgo is /usr/local/bin/ccgo
# ccgo is ~/.cargo/bin/ccgo

# 删除 Python CLI 或调整 PATH
pip uninstall ccgo
# 或在 PATH 前面加上 Cargo bin
export PATH="$HOME/.cargo/bin:$PATH"
```

---

### 问题：不同的构建行为

**症状**：Rust CLI 的构建方式与 Python CLI 不同

**诊断**：
```bash
# 使用详细输出比较
ccgo-rs build android --verbose 2>&1 | tee rust-build.log
python -m ccgo build android --verbose 2>&1 | tee python-build.log
diff -u python-build.log rust-build.log
```

**常见原因**：
- 不同的 CCGO.toml 解析（罕见）
- 不同的依赖解析（罕见）
- 不同的 CMake 变量传递（罕见）

**解决方案**：将日志报告到 [GitHub Issues](https://github.com/zhlinh/ccgo/issues)。

---

### 问题：缺少命令（vendor、update、run、ci）

**症状**：
```bash
ccgo vendor
# error: unrecognized subcommand 'vendor'
```

**解决方案**：对未实现的命令使用 Python CLI：
```bash
# 保持 Python CLI 安装
pip install ccgo  # v3.0.x

# 对 vendor 使用 Python CLI
python -m ccgo vendor

# 或直接使用 pip 脚本
ccgo-3.0 vendor  # 如果一起安装
```

---

### 问题：Gradle 集成失败

**症状**：
```
Task :buildNativeLibraries FAILED
> ccgo: command not found
```

**解决方案**：
```kotlin
// build.gradle.kts - 使用显式路径
tasks.register<Exec>("buildNativeLibraries") {
    workingDir = rootProject.projectDir.parentFile

    // 选项 1：使用 Cargo bin 路径
    commandLine("${System.getenv("HOME")}/.cargo/bin/ccgo", "build", "android", ...)

    // 选项 2：设置 PATH 环境
    environment("PATH", "${System.getenv("PATH")}:${System.getenv("HOME")}/.cargo/bin")
    commandLine("ccgo", "build", "android", ...)
}
```

---

## 性能对比

### 启动时间

| 操作 | Python CLI | Rust CLI | 改进 |
|------|-----------|----------|------|
| `ccgo --version` | 450ms | 8ms | **快 56 倍** |
| `ccgo --help` | 520ms | 12ms | **快 43 倍** |
| `ccgo build --dry-run` | 680ms | 25ms | **快 27 倍** |

### 构建时间（Android arm64-v8a）

| 项目大小 | Python CLI | Rust CLI | 改进 |
|---------|-----------|----------|------|
| 小型（5 个依赖） | 2m 15s | 2m 10s | **快 3%** |
| 中型（15 个依赖） | 6m 30s | 6m 00s | **快 8%** |
| 大型（30 个依赖） | 15m 20s | 14m 10s | **快 8%** |

**注意**：构建时间改进来自：
- 更快的依赖解析
- 并行处理优化
- 更少的 Python/子进程开销

### 内存使用

| 操作 | Python CLI | Rust CLI | 减少 |
|------|-----------|----------|------|
| 空闲 | 45MB | 8MB | **减少 82%** |
| 构建（峰值） | 120MB | 35MB | **减少 71%** |

---

## 迁移策略

### 策略 1：大爆炸迁移（1-2 小时）

**最适合**：小型项目，单个开发者

**步骤**：
1. 安装 Rust CLI
2. 卸载 Python CLI
3. 一次性更新所有脚本/文档
4. 测试所有工作流程
5. 提交更改

**优点**：干净的切换，无版本混合
**缺点**：风险较高，需要测试所有内容

---

### 策略 2：渐进式迁移（1-2 周）

**最适合**：大型项目，团队

**步骤**：
1. 安装 Rust CLI 为 `ccgo-rs`
2. 保留 Python CLI 为 `ccgo`
3. 首先迁移 CI/CD
4. 随着时间推移迁移开发者工作流程
5. 团队采用后切换默认值
6. 删除 Python CLI

**优点**：低风险，渐进式学习
**缺点**：需要同时安装两个 CLI

---

### 策略 3：混合模式（无限期）

**最适合**：需要未实现命令的项目

**步骤**：
1. 安装两个 CLI
2. 对常用命令使用 Rust CLI（build、test 等）
3. 对缺少的命令使用 Python CLI（vendor、update 等）
4. 监控 Rust CLI 版本以获取新功能
5. 准备好后迁移到完整 Rust

**优点**：访问所有功能
**缺点**：两个 CLI 的复杂性

---

## 最佳实践

### 1. 在脚本中使用显式 CLI

**✅ 应该**：指定要使用的 CLI 版本
```bash
# 好 - 显式
~/.cargo/bin/ccgo build android

# 更好 - 带版本检查
CCGO_VERSION=$(ccgo --version | grep -oP 'v?\d+\.\d+')
if [[ "$CCGO_VERSION" < "3.1" ]]; then
    echo "Error: Requires CCGO v3.1+"
    exit 1
fi
```

**❌ 不要**：假设 `ccgo` 是 Rust CLI
```bash
# 不好 - 不明确
ccgo build android  # 可能是 Python 或 Rust
```

---

### 2. 在 README 中记录 CLI 版本

**✅ 应该**：指定最低版本
```markdown
## 要求

- CCGO CLI v3.1+（基于 Rust）
  - 安装：`cargo install ccgo --locked`
  - 验证：`ccgo --version` 应显示 `v3.1.0` 或更高版本

或

- CCGO CLI v3.0.x（基于 Python，已弃用）
  - 安装：`pip install ccgo`
```

---

### 3. 在过渡期间测试两个 CLI

**✅ 应该**：确保兼容性
```bash
# 使用 Python CLI 测试
pip install ccgo==3.0.10
ccgo build android
mv target target-python

# 使用 Rust CLI 测试
cargo install ccgo --locked
ccgo build android
mv target target-rust

# 比较输出
diff -r target-python target-rust
```

---

### 4. 在 CI 中使用 Cargo 安装 Rust CLI

**✅ 应该**：在 CI 中固定版本
```yaml
- name: Install CCGO
  run: |
    cargo install ccgo --locked --version 3.1.0
    ccgo --version
```

**或使用预构建的二进制文件以加快速度**：
```yaml
- name: Install CCGO
  run: |
    curl -LO https://github.com/zhlinh/ccgo/releases/download/v3.1.0/ccgo-linux-x86_64
    chmod +x ccgo-linux-x86_64
    sudo mv ccgo-linux-x86_64 /usr/local/bin/ccgo
```

---

## 常见问题

### Q: Python CLI 会继续维护吗？

**A**：Python CLI（v3.0.x）处于**维护模式**：
- ✅ 仅关键错误修复
- ❌ 无新功能
- ❌ 无新平台支持

**建议**：迁移到 Rust CLI 以获取新功能。

---

### Q: 我可以同时使用两个 CLI 吗？

**A**：可以！安装为不同的名称：
```bash
pip install ccgo  # Python CLI 为 'ccgo'
cargo install ccgo-rs --locked  # Rust CLI 为 'ccgo-rs'

# 使用 Python CLI
ccgo build android

# 使用 Rust CLI
ccgo-rs build android
```

---

### Q: 如果我在 Rust CLI 中发现错误怎么办？

**A**：报告它并使用 Python CLI 作为回退：
1. 报告问题：https://github.com/zhlinh/ccgo/issues
2. 暂时使用 Python CLI：`pip install ccgo==3.0.10`
3. 监控问题以获取修复
4. 修复后升级 Rust CLI：`cargo install ccgo --locked --force`

---

### Q: 如何降级回 Python CLI？

**A**：简单：
```bash
# 卸载 Rust CLI
cargo uninstall ccgo

# 安装 Python CLI
pip install ccgo==3.0.10

# 验证
ccgo --version  # 应显示 3.0.10
```

---

### Q: CCGO.toml 格式兼容吗？

**A**：**是的**，100% 兼容。Rust CLI 读取与 Python CLI 相同的 CCGO.toml。无需更改。

---

### Q: 我的构建脚本会中断吗？

**A**：**可能不会**。Rust CLI 与 Python CLI 保持 CLI 兼容性。唯一的区别：
- 进度输出（外观）
- 错误消息格式（外观）
- 一些未实现的命令（使用 Python CLI 回退）

---

### Q: 迁移需要多长时间？

**A**：取决于项目复杂性：
- **简单项目**（仅 `ccgo build`）：10-30 分钟
- **CI/CD 集成**：1-2 小时
- **复杂自动化**：2-8 小时

大多数项目：**总共 1-2 小时**。

---

## 检查清单

### 迁移前

- [ ] 记录当前 Python CLI 版本
- [ ] 列出项目中使用的所有 ccgo 命令
- [ ] 检查 CI/CD 工作流程
- [ ] 识别使用 ccgo 的自定义脚本
- [ ] 验证未实现的命令（vendor、update、run、ci）

### 迁移中

- [ ] 安装 Rust CLI（测试模式）
- [ ] 测试基本命令（`build`、`test`、`doc`）
- [ ] 测试所有目标平台
- [ ] 更新 CI/CD 工作流程
- [ ] 更新构建脚本
- [ ] 更新开发者文档
- [ ] 与团队成员一起测试
- [ ] 切换默认 CLI（卸载 Python 或调整 PATH）

### 迁移后

- [ ] 如果不再需要，删除 Python CLI
- [ ] 归档 Python CLI 脚本以供参考
- [ ] 监控 Rust CLI 的问题
- [ ] 培训团队新 CLI 功能
- [ ] 更新入职文档

---

## 总结

从 Python CLI 迁移到 Rust CLI 提供：

**优势**：
1. ✅ **快 2-56 倍**的启动和执行
2. ✅ **单一二进制**分发（无 Python 依赖）
3. ✅ **更好的错误消息**和提示
4. ✅ **更低的内存使用**（减少 70-80%）
5. ✅ **类型安全**（更少的运行时错误）

**迁移工作量**：大多数项目 1-2 小时

**兼容性**：接近 100% CLI 兼容性，100% CCGO.toml 兼容性

**建议**：准备好后迁移；在过渡期间保留 Python CLI 作为回退。

---

## 其他资源

- [CCGO Rust CLI 源代码](https://github.com/zhlinh/ccgo/tree/main/ccgo-rs)
- [CCGO Releases](https://github.com/zhlinh/ccgo/releases)
- [CCGO CLI 参考](../reference/cli.md)
- [CCGO GitHub Issues](https://github.com/zhlinh/ccgo/issues)

---

*本指南是 CCGO 文档的一部分。如有问题或改进建议，请在 [GitHub](https://github.com/zhlinh/ccgo/issues) 上提出 issue。*
