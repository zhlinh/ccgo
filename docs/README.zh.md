# CCGO 文档

本目录包含 CCGO 文档的源文件，使用 [MkDocs](https://www.mkdocs.org/) 和 [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/) 构建。

## 文档结构

```
docs/
├── index.md                 # 主页（英文）
├── index.zh.md              # 主页（中文）
├── getting-started/         # 入门指南
│   ├── installation.md
│   ├── installation.zh.md
│   ├── quickstart.md
│   ├── quickstart.zh.md
│   ├── configuration.md
│   └── project-structure.md
├── platforms/               # 平台特定指南
│   ├── index.md
│   ├── android.md
│   ├── ios.md
│   ├── macos.md
│   ├── windows.md
│   ├── linux.md
│   ├── ohos.md
│   └── kmp.md
├── features/                # 功能文档
│   ├── build-system.md
│   ├── dependency-management.md
│   ├── publishing.md
│   ├── docker-builds.md
│   ├── version-management.md
│   └── git-integration.md
├── reference/               # 参考文档
│   ├── cli.md
│   ├── ccgo-toml.md
│   ├── cmake.md
│   └── gradle-plugins.md
├── development/             # 开发指南
│   ├── contributing.md
│   ├── contributing.zh.md
│   ├── roadmap.md
│   ├── roadmap.zh.md
│   ├── changelog.md
│   └── architecture.md
└── requirements.txt         # Python 依赖
```

## 在本地构建文档

### 前置条件

```bash
# 安装 Python 3.8+
python3 --version

# 安装依赖
pip3 install -r docs/requirements.txt
```

### 启动文档服务

```bash
# 在项目根目录运行
mkdocs serve

# 在浏览器中打开 http://127.0.0.1:8000
```

### 构建静态站点

```bash
# 构建到 site/ 目录
mkdocs build

# 严格模式构建（遇到警告即失败）
mkdocs build --strict
```

## 多语言支持

文档支持英文和中文：

- 英文文件：`filename.md`
- 中文文件：`filename.zh.md`

语言切换器显示在站点头部。

### 添加新语言

1. 更新 `mkdocs.yml`：
   ```yaml
   plugins:
     - i18n:
         languages:
           - locale: fr
             name: Français
             build: true
   ```

2. 创建带 `.fr.md` 后缀的翻译文件

3. 在 `nav_translations` 部分添加翻译

## 撰写文档

### 风格指南

- 使用清晰、简洁的语言
- 为复杂概念提供代码示例
- 在有帮助的地方添加命令输出
- 对重要说明使用 admonition
- 在相关文档之间交叉链接

### 代码块

使用带语言标识的围栏代码块：

\`\`\`bash
ccgo build android --arch arm64-v8a
\`\`\`

\`\`\`toml
[package]
name = "mylib"
version = "1.0.0"
\`\`\`

### Admonition

```markdown
!!! note
    这是一条说明。

!!! warning
    这是一条警告。

!!! tip
    这是一条提示。
```

### 标签内容

```markdown
=== "Linux"
    Linux 特定内容

=== "macOS"
    macOS 特定内容

=== "Windows"
    Windows 特定内容
```

## 发布

### ReadTheDocs

每次推送到 main 分支时，文档会自动构建并发布到 ReadTheDocs。

- 站点：https://ccgo.readthedocs.io
- 管理：https://readthedocs.org/projects/ccgo/

### 手动部署

```bash
# 构建并部署到 GitHub Pages
mkdocs gh-deploy
```

## 贡献

提交文档贡献时：

1. 遵循现有结构和风格
2. 在本地用 `mkdocs serve` 测试
3. 用 `mkdocs build --strict` 检查失效链接
4. 同时更新英文和中文版本
5. 提交 pull request

详见[贡献指南](development/contributing.zh.md)。

## 故障排除

### 构建错误

```bash
# 清除构建缓存
rm -rf site/

# 重新构建
mkdocs build --strict
```

### 实时刷新失效

```bash
# 尝试不同端口
mkdocs serve --dev-addr 127.0.0.1:8001
```

### 缺少依赖

```bash
# 重新安装依赖
pip3 install -r docs/requirements.txt --upgrade
```

## 资源

- [MkDocs 文档](https://www.mkdocs.org/)
- [Material for MkDocs](https://squidfunk.github.io/mkdocs-material/)
- [Python Markdown 扩展](https://facelessuser.github.io/pymdown-extensions/)
- [mkdocs-static-i18n 插件](https://github.com/ultrabug/mkdocs-static-i18n)
