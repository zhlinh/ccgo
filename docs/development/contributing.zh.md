# 为 CCGO 贡献

感谢您对为 CCGO 做出贡献的兴趣！本指南将帮助您开始。

## 贡献方式

- **报告错误**：提交包含详细复现步骤的问题
- **建议功能**：提出新功能或改进
- **编写文档**：改进文档、添加示例、修复错别字
- **提交代码**：修复错误、实现功能、改进性能
- **分享反馈**：告诉我们您使用 CCGO 的体验

## 开始

### 1. Fork 和 Clone

```bash
# 在 GitHub 上 Fork 仓库
# 然后 clone 您的 fork
git clone https://github.com/YOUR_USERNAME/ccgo.git
cd ccgo
```

### 2. 设置开发环境

```bash
# 安装 Python 依赖
cd ccgo
pip3 install -e ".[dev]"

# 或安装 Rust 版本
cd ccgo-rs
cargo build
```

### 3. 创建分支

```bash
git checkout -b feature/my-new-feature
# 或
git checkout -b fix/issue-123
```

## 开发工作流

### Python CLI (`/ccgo/`)

```bash
cd ccgo

# 以可编辑模式安装
pip3 install -e .

# 运行测试
pytest tests/

# 运行 linters
flake8 .
black .
mypy .

# 测试 CLI 命令
ccgo --version
```

### Rust CLI (`/ccgo-rs/`)

```bash
cd ccgo-rs

# 构建
cargo build

# 运行测试
cargo test

# 运行 linters
cargo clippy
cargo fmt

# 本地安装以供测试
cargo install --path .
```

### Gradle 插件 (`/ccgo-gradle-plugins/`)

```bash
cd ccgo-gradle-plugins

# 构建插件
./gradlew build

# 发布到 Maven Local 以供测试
./gradlew publishToMavenLocal

# 在项目中测试
# 将 mavenLocal() 添加到 pluginManagement.repositories
```

### 模板 (`/ccgo-template/`)

```bash
# 测试模板生成
copier copy ccgo-template/ test-output/ --vcs-ref HEAD --trust

# 在现有项目中测试
cd existing-project
copier update --vcs-ref HEAD
```

## 代码风格

### Python
- 遵循 [PEP 8](https://www.python.org/dev/peps/pep-0008/)
- 使用 [Black](https://black.readthedocs.io/) 格式化
- 尽可能使用类型提示
- 为公共 API 编写文档字符串

### Rust
- 遵循 [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- 使用 `cargo fmt` 格式化
- 使用 `cargo clippy` 检查
- 为公共项编写文档注释

### 提交消息

遵循 [Conventional Commits](https://www.conventionalcommits.org/)：

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

类型：
- `feat`：新功能
- `fix`：错误修复
- `docs`：文档更改
- `style`：代码风格更改（格式化等）
- `refactor`：代码重构
- `test`：添加或更新测试
- `chore`：维护任务

示例：
```
feat(build): add support for RISC-V architecture
fix(android): resolve AAR packaging issue with native libs
docs(readme): update installation instructions
```

## 测试

### 编写测试

- 为新功能添加单元测试
- 为面向用户的功能添加集成测试
- 确保测试在所有支持的平台上通过
- 目标代码覆盖率 > 80%

### 运行测试

```bash
# Python
pytest tests/ -v

# Rust
cargo test

# 集成测试
cd ccgo-now/ccgonow
ccgo build android --arch arm64-v8a
ccgo test
```

## 文档

### 更新文档

- 为新 API 添加文档字符串/文档注释
- 如果添加面向用户的功能，更新 README.md
- 更新 `/docs/` 目录中的相关文档
- 为复杂功能添加示例

### 本地构建文档

```bash
# 安装 MkDocs
pip install -r docs/requirements.txt

# 本地服务文档
mkdocs serve

# 在浏览器中打开 http://localhost:8000
```

## Pull Request 流程

### 1. 准备您的 PR

- 确保所有测试通过
- 更新文档
- 在 CHANGELOG.md 中添加条目（如适用）
- 在最新的 main 分支上 rebase

```bash
git fetch upstream
git rebase upstream/main
```

### 2. 提交 PR

- 将您的分支推送到您的 fork
- 在 GitHub 上打开 Pull Request
- 完整填写 PR 模板
- 使用 #issue_number 链接相关问题

### 3. 代码审查

- 处理审查者的反馈
- 保持提交干净和原子化
- 对问题/建议做出响应
- CI 检查必须通过

### 4. 合并

- 如果被要求，压缩提交
- 准备好后维护者将合并
- 合并后删除您的分支

## Issue 指南

### 报告错误

包括：
- CCGO 版本 (`ccgo --version`)
- 操作系统和版本
- 复现步骤
- 预期与实际行为
- 错误消息和日志
- 如果可能，提供最小复现案例

### 请求功能

包括：
- 功能的清晰描述
- 用例和好处
- 提议的 API/接口（如适用）
- 您考虑过的替代方案

## 社区指南

- 尊重和包容
- 遵循我们的[行为准则](https://github.com/zhlinh/ccgo/blob/main/CODE_OF_CONDUCT.md)
- 在讨论中帮助他人
- 给予建设性反馈
- 庆祝贡献

## 获取帮助

- 查看现有[文档](https://ccgo.readthedocs.io)
- 搜索[现有问题](https://github.com/zhlinh/ccgo/issues)
- 在 [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions) 中提问
- 加入我们的社区聊天（即将推出）

## 许可证

通过贡献，您同意您的贡献将根据 MIT 许可证授权。

---

感谢您为 CCGO 做出贡献！🎉
