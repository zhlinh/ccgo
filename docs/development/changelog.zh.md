# 更新日志管理

遵循 Keep a Changelog 格式和最佳实践管理 CCGO 项目更新日志的指南。

## 概览

CCGO 遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/) 原则记录更改：

- **人类可读** - 按版本和类别组织更改
- **机器可解析** - 自动化工具的结构化格式
- **Git 集成** - 从 git 历史自动生成
- **语义化版本** - 与 SemVer 版本号绑定
- **发布说明** - 发布文档的基础

## 更新日志格式

### 标准结构

```markdown
# 更新日志

本项目的所有重要更改都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

## [未发布]

### 新增
- 已添加的新功能

### 变更
- 现有功能的更改

### 弃用
- 即将在未来版本中删除的功能

### 移除
- 已删除的功能

### 修复
- 错误修复

### 安全
- 安全改进和漏洞修复

## [1.0.0] - 2024-01-15

### 新增
- 初始发布
- 跨平台 C++ 构建系统
- 支持 Android、iOS、macOS、Windows、Linux

[未发布]: https://github.com/user/project/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/user/project/releases/tag/v1.0.0
```

### 更改类别

| 类别 | 说明 | 示例 |
|------|------|------|
| **新增** | 新功能 | 为 Android 添加 ARM64 支持 |
| **变更** | 对现有功能的修改 | 将 CMake 最低版本更新至 3.20 |
| **弃用** | 标记为待删除的功能 | 弃用 `--legacy-build` 标志 |
| **移除** | 已删除的功能 | 移除 Python 2 支持 |
| **修复** | 错误修复 | 修复 iOS 框架中的内存泄漏 |
| **安全** | 安全修复 | 修补 XSS 漏洞 |

## 创建更新日志

### 初始设置

```bash
# 在项目根目录创建 CHANGELOG.md
cat > CHANGELOG.md << 'EOF'
# 更新日志

本项目的所有重要更改都将记录在此文件中。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/spec/v2.0.0.html)。

## [未发布]

### 新增
- 初始项目设置

[未发布]: https://github.com/user/project/compare/v0.1.0...HEAD
EOF
```

### 添加更改

```markdown
## [未发布]

### 新增
- Android ARM64-v8a 架构支持
- 基于 Docker 的 Windows 和 Linux 跨平台构建
- 从 git 标签自动注入版本

### 变更
- 将最低 CMake 版本从 3.18 更新至 3.20
- 改进缺少依赖的错误消息

### 修复
- 修复 iOS 框架符号可见性问题
- 解决 MinGW 构建中的 Windows DLL 导出问题
```

### 发布版本

1. **将未发布的更改移至新版本：**

```markdown
## [未发布]

## [1.2.0] - 2024-01-15

### 新增
- Android ARM64-v8a 架构支持
- 基于 Docker 的跨平台构建

### 变更
- 将最低 CMake 版本从 3.18 更新至 3.20

### 修复
- 修复 iOS 框架符号可见性问题

[未发布]: https://github.com/user/project/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/user/project/compare/v1.1.0...v1.2.0
```

2. **创建 git 标签：**

```bash
ccgo tag v1.2.0 --message "Release 1.2.0"
```

3. **更新版本链接：**

```markdown
[未发布]: https://github.com/user/project/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/user/project/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/user/project/compare/v1.0.0...v1.1.0
```

## 自动生成更新日志

### 使用 ccgo changelog

```bash
# 从 git 历史生成更新日志
ccgo changelog

# 输出到文件
ccgo changelog --output CHANGELOG.md

# 在特定版本之间
ccgo changelog --from v1.0.0 --to v2.0.0

# 包含所有提交
ccgo changelog --include-all

# 按类型分组（约定式提交）
ccgo changelog --group-by-type
```

### Git 提交解析器

CCGO 解析[约定式提交](https://www.conventionalcommits.org/zh-hans/)以自动分类：

```bash
# 提交格式：<类型>(<范围>): <主题>

feat(android): add ARM64 support       → 新增
fix(ios): resolve memory leak          → 修复
docs: update installation guide        → （文档，不在更新日志中）
chore: bump version to 1.2.0          → （维护，不在更新日志中）
refactor(core): simplify error handling → 变更
test: add unit tests for calculator    → （测试，不在更新日志中）
perf(network): optimize data transfer  → 变更（性能改进）
```

**类型映射：**

| 提交类型 | 更新日志类别 |
|---------|-------------|
| `feat` | 新增 |
| `fix` | 修复 |
| `perf` | 变更 |
| `refactor` | 变更 |
| `revert` | 变更 |
| `docs` | （不包含） |
| `style` | （不包含） |
| `test` | （不包含） |
| `chore` | （不包含） |
| `build` | （不包含） |
| `ci` | （不包含） |

### 破坏性更改

```bash
# 带破坏性更改的提交
git commit -m "feat(api)!: change function signature

BREAKING CHANGE: Calculator.add() now returns Result<int> instead of int"
```

**在更新日志中：**

```markdown
## [2.0.0] - 2024-01-15

### 变更
- **破坏性更改：** Calculator.add() 现在返回 Result<int> 而不是 int

### 迁移指南
```cpp
// 之前
int result = Calculator.add(2, 3);

// 之后
auto result = Calculator.add(2, 3);
if (result.is_ok()) {
    int value = result.value();
}
```
```

## CI/CD 集成

### GitHub Actions

```yaml
name: Update Changelog
on:
  push:
    tags:
      - 'v*'

jobs:
  changelog:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # 完整历史用于更新日志生成

      - name: Install CCGO
        run: pip install ccgo

      - name: Generate Changelog
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")

          if [ -n "$PREV_TAG" ]; then
            ccgo changelog --from $PREV_TAG --to v$VERSION --output CHANGELOG.new.md
          else
            ccgo changelog --to v$VERSION --output CHANGELOG.new.md
          fi

      - name: Update Changelog
        run: |
          # 将新更改添加到现有更新日志之前
          cat CHANGELOG.new.md CHANGELOG.md > CHANGELOG.tmp.md
          mv CHANGELOG.tmp.md CHANGELOG.md

      - name: Commit Changelog
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add CHANGELOG.md
          git commit -m "docs: update changelog for ${GITHUB_REF#refs/tags/}"
          git push
```

### GitLab CI

```yaml
update-changelog:
  stage: deploy
  only:
    - tags
  script:
    - pip install ccgo
    - |
      VERSION=$(echo $CI_COMMIT_TAG | sed 's/^v//')
      PREV_TAG=$(git describe --tags --abbrev=0 $CI_COMMIT_TAG^ 2>/dev/null || echo "")
      ccgo changelog --from $PREV_TAG --to $CI_COMMIT_TAG --output CHANGELOG.new.md
      cat CHANGELOG.new.md CHANGELOG.md > CHANGELOG.tmp.md
      mv CHANGELOG.tmp.md CHANGELOG.md
    - |
      git config user.name "GitLab CI"
      git config user.email "ci@gitlab.com"
      git add CHANGELOG.md
      git commit -m "docs: update changelog for $CI_COMMIT_TAG"
      git push https://oauth2:${CI_JOB_TOKEN}@${CI_SERVER_HOST}/${CI_PROJECT_PATH}.git HEAD:main
```

## 发布说明

### 生成发布说明

```bash
# 从更新日志生成发布说明
ccgo release-notes v1.2.0

# 输出：
# Release 1.2.0
#
# 新增：
# - Android ARM64-v8a 架构支持
# - 基于 Docker 的跨平台构建
#
# 变更：
# - 将最低 CMake 版本从 3.18 更新至 3.20
#
# 修复：
# - 修复 iOS 框架符号可见性问题
```

### GitHub Release 集成

```yaml
name: Create GitHub Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install CCGO
        run: pip install ccgo

      - name: Generate Release Notes
        id: release_notes
        run: |
          ccgo release-notes ${GITHUB_REF#refs/tags/} --output release_notes.md
          echo "notes_file=release_notes.md" >> $GITHUB_OUTPUT

      - name: Create GitHub Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          body_path: ${{ steps.release_notes.outputs.notes_file }}
          draft: false
          prerelease: false
```

## 最佳实践

### 1. 在每个 PR 中更新更新日志

```markdown
## [未发布]

### 新增
- 功能 X (#123)
- 功能 Y (#124)

### 修复
- 功能 A 中的错误 (#125)
```

包含 PR/issue 编号以便追溯。

### 2. 编写以用户为中心的描述

```markdown
# 好
- 添加对在 Apple Silicon Mac 上原生构建的支持

# 坏
- 更新 build_macos.py 以检测 arm64 架构
```

### 3. 分组相关更改

```markdown
### Android

#### 新增
- 支持 Android 13（API 33）
- 新的 Material Design 组件

#### 修复
- Android 11 中启动时崩溃
- 后台服务中的内存泄漏

### iOS

#### 新增
- iOS 16 小部件支持
- SwiftUI 预览

#### 修复
- App Store 提交问题
```

### 4. 为破坏性更改包含迁移指南

```markdown
## [2.0.0] - 2024-01-15

### 变更
- **破坏性更改：** 将 `ccgo build-all` 重命名为 `ccgo build --all`

### 迁移指南

更新您的构建脚本：
```bash
# 之前
ccgo build-all

# 之后
ccgo build --all
```
```

### 5. 链接到文档

```markdown
### 新增
- 基于 Docker 的跨平台构建（[文档](../features/docker-builds.zh.md)）
- 为 `ccgo build` 命令添加新的 `--docker` 标志
```

## 更新日志工具

### changelog-cli

```bash
# 安装 changelog-cli
npm install -g changelog-cli

# 添加条目
changelog add "Added new feature" --type added

# 删除条目
changelog remove "Old entry"

# 发布版本
changelog release 1.2.0
```

### git-cliff

```bash
# 安装 git-cliff
cargo install git-cliff

# 生成更新日志
git-cliff --output CHANGELOG.md

# 为特定范围生成
git-cliff v1.0.0..v2.0.0
```

### conventional-changelog

```bash
# 安装 conventional-changelog
npm install -g conventional-changelog-cli

# 生成更新日志
conventional-changelog -p angular -i CHANGELOG.md -s

# 首次发布
conventional-changelog -p angular -i CHANGELOG.md -s -r 0
```

## CCGO 项目更新日志

### 示例

```markdown
# 更新日志

## [未发布]

### 新增
- 基于 Rust 的 CLI 重写以提高性能
- 支持 Apple Watch 和 Apple TV 平台
- 所有平台统一的归档结构

### 变更
- 从 Python argparse 迁移到 Rust clap 进行 CLI 解析

## [3.0.10] - 2024-01-15

### 新增
- Git 版本控制与自动提交 SHA 注入
- 统一的归档命名约定
- 为所有平台生成符号包

### 变更
- 通过预构建镜像改进 Docker 构建性能
- 将 Android NDK 要求更新至 r21+

### 修复
- 修复 Windows MinGW 构建符号导出
- 解决 iOS 框架代码签名问题

### 安全
- 将 OpenSSL 依赖更新至 1.1.1w

## [3.0.9] - 2023-12-01

### 新增
- 基于 Docker 的跨平台构建
- 支持 OpenHarmony 平台

### 修复
- 修复 macOS 通用二进制文件生成
- 解决 Linux RPATH 问题

[未发布]: https://github.com/zhlinh/ccgo/compare/v3.0.10...HEAD
[3.0.10]: https://github.com/zhlinh/ccgo/compare/v3.0.9...v3.0.10
[3.0.9]: https://github.com/zhlinh/ccgo/releases/tag/v3.0.9
```

## 资源

### 规范

- [Keep a Changelog](https://keepachangelog.com/zh-CN/)
- [语义化版本](https://semver.org/lang/zh-CN/)
- [约定式提交](https://www.conventionalcommits.org/zh-hans/)

### 工具

- [changelog-cli](https://github.com/mc706/changelog-cli)
- [git-cliff](https://github.com/orhun/git-cliff)
- [conventional-changelog](https://github.com/conventional-changelog/conventional-changelog)

### CCGO 文档

- [版本管理](../features/version-management.zh.md)
- [Git 集成](../features/git-integration.zh.md)
- [贡献指南](contributing.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [贡献指南](contributing.zh.md)
- [版本管理](../features/version-management.zh.md)
- [Git 集成](../features/git-integration.zh.md)
- [CI/CD 设置](contributing.zh.md#cicd-集成)
