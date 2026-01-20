# Git 集成

CCGO 项目中 Git 工作流集成的完整指南，包括自动化操作、钩子和最佳实践。

## 概览

CCGO 提供无缝的 Git 集成用于版本控制工作流：

- **自动标签** - 从 CCGO.toml 创建版本标签
- **Git 钩子** - 提交前验证和自动化
- **子模块管理** - 将 C++ 依赖作为子模块处理
- **CI/CD 集成** - git 事件触发的自动化构建
- **分支工作流** - 支持 GitFlow 和主干开发
- **提交元数据** - 将 git 信息注入到构建中

## Git 工作流模式

### 主干开发

```bash
# 主分支始终可部署
main ─────●─────●─────●─────●─────●──→
           \     \     \     \
            \─●   \─●   \─●   \─●  短期
              |     |     |     |   功能分支
              合并  合并  合并  合并
```

**工作流：**
```bash
# 1. 创建短期功能分支
git checkout -b feature/quick-fix

# 2. 进行更改并提交
ccgo build --all
git commit -am "feat: add new API endpoint"

# 3. 合并到 main（CI 通过后）
git checkout main
git merge feature/quick-fix

# 4. 标记发布
ccgo tag v1.2.3 --message "Release 1.2.3"
```

### GitFlow

```bash
main ────●─────────────●─────────────●──→  生产版本
          \           /               /
           \         /               /
develop ────●───●───●───●───●───●───●──→  集成分支
             \  |  /     \     /
              \ | /       \   /
feature-a      ●●●         \ /
feature-b                   ●●●
```

**工作流：**
```bash
# 1. 从 develop 创建功能分支
git checkout develop
git checkout -b feature/new-module

# 2. 开发功能
ccgo build --all
ccgo test

# 3. 合并回 develop
git checkout develop
git merge feature/new-module

# 4. 创建发布分支
git checkout -b release/1.2.0

# 5. 完成发布
# 更新 CCGO.toml: version = "1.2.0"
ccgo build --all --release
ccgo test --all

# 6. 合并到 main 和 develop
git checkout main
git merge release/1.2.0
ccgo tag v1.2.0 --message "Release 1.2.0"

git checkout develop
git merge release/1.2.0
```

## 自动标签

### 使用 ccgo tag

```bash
# 从 CCGO.toml 版本创建标签
ccgo tag

# 使用自定义版本创建标签
ccgo tag v2.0.0

# 创建带消息的注解标签
ccgo tag --annotate --message "Release 2.0.0 with new features"

# 推送标签到远程
ccgo tag --push

# 强制更新现有标签
ccgo tag --force
```

### 标签命名约定

CCGO 遵循语义化版本控制的标签规范：

```bash
# 发布标签
v1.0.0          # 主版本发布
v1.2.3          # 次版本/补丁发布
v2.0.0-rc.1     # 候选发布版
v1.5.0-beta.2   # Beta 发布

# 内部标签
build-20240115  # CI 构建标签
dev-john-123    # 开发标签
```

### CI 中的自动标签创建

```yaml
# GitHub Actions
name: Tag Release
on:
  push:
    branches: [main]

jobs:
  tag:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install CCGO
        run: pip install ccgo

      - name: Create tag if version changed
        run: |
          # 从 CCGO.toml 提取版本
          VERSION=$(grep '^version = ' CCGO.toml | cut -d'"' -f2)

          # 检查标签是否存在
          if ! git rev-parse "v$VERSION" >/dev/null 2>&1; then
            ccgo tag "v$VERSION" --message "Release $VERSION" --push
          fi
```

## Git 钩子

### 提交前钩子

提交前验证代码：

```bash
# .git/hooks/pre-commit
#!/bin/bash

# 格式检查
echo "正在运行代码格式检查..."
if ! ccgo check --format; then
    echo "错误：发现代码格式问题"
    echo "运行：ccgo format --fix"
    exit 1
fi

# 构建检查
echo "正在运行构建检查..."
if ! ccgo build --quick-check; then
    echo "错误：构建检查失败"
    exit 1
fi

# 许可证头
echo "正在检查许可证头..."
if ! ccgo check --license; then
    echo "错误：缺少许可证头"
    exit 1
fi

echo "提交前检查通过！"
```

### 推送前钩子

推送前运行测试：

```bash
# .git/hooks/pre-push
#!/bin/bash

# 运行单元测试
echo "正在运行单元测试..."
if ! ccgo test; then
    echo "错误：测试失败"
    exit 1
fi

# 检查已更改文件中的 TODO/FIXME
echo "正在检查未解决的 TODO..."
changed_files=$(git diff --name-only @{u}..HEAD)
if echo "$changed_files" | xargs grep -n "FIXME\|TODO" 2>/dev/null; then
    read -p "发现 TODO/FIXME。继续推送？(y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

echo "推送前检查通过！"
```

### 提交消息钩子

强制使用约定式提交：

```bash
# .git/hooks/commit-msg
#!/bin/bash

commit_msg=$(cat "$1")

# 检查约定式提交格式
if ! echo "$commit_msg" | grep -qE "^(feat|fix|docs|style|refactor|test|chore|perf|ci|build|revert)(\(.+\))?: .+"; then
    echo "错误：提交消息必须遵循约定式提交格式"
    echo ""
    echo "格式：<类型>(<范围>): <主题>"
    echo ""
    echo "类型：feat, fix, docs, style, refactor, test, chore, perf, ci, build, revert"
    echo ""
    echo "示例：feat(android): add new build configuration"
    exit 1
fi

echo "提交消息格式验证通过！"
```

### 安装钩子

```bash
# 使钩子可执行
chmod +x .git/hooks/pre-commit
chmod +x .git/hooks/pre-push
chmod +x .git/hooks/commit-msg

# 或使用 CCGO 安装钩子
ccgo install-hooks
```

## 子模块管理

### 将 C++ 依赖添加为子模块

```bash
# 添加第三方库作为子模块
git submodule add https://github.com/openssl/openssl.git third_party/openssl

# 初始化和更新子模块
git submodule update --init --recursive

# 更新子模块到最新版本
git submodule update --remote --recursive
```

### 带子模块的 CCGO.toml

```toml
[dependencies]
# Git 子模块依赖
openssl = { path = "third_party/openssl", type = "submodule" }

# 通过 git 标签指定版本
boost = {
    git = "https://github.com/boostorg/boost.git",
    tag = "boost-1.80.0",
    type = "submodule"
}

# 分支跟踪
nlohmann_json = {
    git = "https://github.com/nlohmann/json.git",
    branch = "develop",
    type = "submodule"
}
```

### 子模块工作流

```bash
# 克隆项目并包含子模块
git clone --recurse-submodules https://github.com/user/myproject.git

# 更新现有克隆
git submodule update --init --recursive

# 更新特定子模块
cd third_party/openssl
git checkout v3.0.0
cd ../..
git add third_party/openssl
git commit -m "chore: update openssl to v3.0.0"

# 删除子模块
git submodule deinit third_party/oldlib
git rm third_party/oldlib
```

## 提交元数据注入

### 构建时 Git 信息

CCGO 自动将 git 元数据注入到构建中：

```cpp
// 在 include/<project>/version.h 中自动生成
#define MYLIB_GIT_SHA "8f3a2b1c"
#define MYLIB_GIT_BRANCH "main"
#define MYLIB_GIT_TAG "v1.2.3"
#define MYLIB_GIT_DIRTY 0  // 如果有未提交的更改则为 1

namespace mylib {
    const char* get_git_sha();
    const char* get_git_branch();
    const char* get_git_tag();
    bool is_git_dirty();
}
```

### 在代码中使用 Git 信息

```cpp
#include "mylib/version.h"
#include <iostream>

void print_build_info() {
    std::cout << "版本：" << mylib::get_version() << "\n";
    std::cout << "Git SHA：" << mylib::get_git_sha() << "\n";
    std::cout << "Git 分支：" << mylib::get_git_branch() << "\n";
    std::cout << "Git 标签：" << mylib::get_git_tag() << "\n";

    if (mylib::is_git_dirty()) {
        std::cout << "警告：从不干净的工作树构建\n";
    }
}
```

### 禁用 Git 注入

```bash
# 构建时不包含 git 元数据
ccgo build android --no-git-inject

# 或在 CCGO.toml 中配置
[version]
inject_git_metadata = false
```

## 分支保护

### GitHub 分支保护规则

```yaml
# .github/branch-protection.yml
main:
  required_status_checks:
    strict: true
    contexts:
      - "build-android"
      - "build-ios"
      - "test-all"
      - "lint"

  required_pull_request_reviews:
    required_approving_review_count: 2
    dismiss_stale_reviews: true

  enforce_admins: false
  restrictions: null

develop:
  required_status_checks:
    strict: false
    contexts:
      - "build-android"
      - "test-all"

  required_pull_request_reviews:
    required_approving_review_count: 1
```

### 接收前钩子（服务器端）

```bash
# 防止直接推送到 main
#!/bin/bash

while read oldrev newrev refname; do
    if [ "$refname" = "refs/heads/main" ]; then
        echo "错误：不允许直接推送到 main"
        echo "请创建拉取请求"
        exit 1
    fi
done
```

## CI/CD 集成

### GitHub Actions

```yaml
name: CI Build
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]
  create:
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest]
        platform: [android, ios]

    steps:
      - uses: actions/checkout@v3
        with:
          submodules: recursive
          fetch-depth: 0  # 完整历史记录用于 git describe

      - name: Install CCGO
        run: pip install ccgo

      - name: Build
        run: ccgo build ${{ matrix.platform }} --release

      - name: Test
        run: ccgo test ${{ matrix.platform }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: ${{ matrix.platform }}-build
          path: target/${{ matrix.platform }}/
```

### GitLab CI

```yaml
stages:
  - build
  - test
  - deploy

variables:
  GIT_SUBMODULE_STRATEGY: recursive

build:android:
  stage: build
  tags: [android]
  script:
    - ccgo build android --release
  artifacts:
    paths:
      - target/android/
    expire_in: 1 week

build:ios:
  stage: build
  tags: [macos]
  script:
    - ccgo build ios --release
  artifacts:
    paths:
      - target/ios/
    expire_in: 1 week

test:all:
  stage: test
  script:
    - ccgo test --all
  dependencies:
    - build:android
    - build:ios

deploy:release:
  stage: deploy
  only:
    - tags
  script:
    - ccgo publish --all --registry official
```

### Jenkins Pipeline

```groovy
pipeline {
    agent any

    environment {
        CCGO_VERSION = sh(script: "grep '^version' CCGO.toml | cut -d'\"' -f2", returnStdout: true).trim()
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
                sh 'git submodule update --init --recursive'
            }
        }

        stage('Build') {
            parallel {
                stage('Android') {
                    steps {
                        sh 'ccgo build android --release'
                    }
                }
                stage('iOS') {
                    steps {
                        sh 'ccgo build ios --release'
                    }
                }
            }
        }

        stage('Test') {
            steps {
                sh 'ccgo test --all'
            }
        }

        stage('Tag') {
            when {
                branch 'main'
            }
            steps {
                sh "ccgo tag v${CCGO_VERSION} --message 'Release ${CCGO_VERSION}' --push"
            }
        }
    }
}
```

## 单一仓库管理

### 使用 Git Subtree 管理共享库

```bash
# 添加共享库作为 subtree
git subtree add --prefix=libs/common \
    https://github.com/company/common-lib.git main --squash

# 更新 subtree
git subtree pull --prefix=libs/common \
    https://github.com/company/common-lib.git main --squash

# 将更改推送回 subtree
git subtree push --prefix=libs/common \
    https://github.com/company/common-lib.git feature-branch
```

### 稀疏检出

```bash
# 仅克隆特定目录
git clone --filter=blob:none --sparse https://github.com/user/monorepo.git
cd monorepo
git sparse-checkout set libs/mylib docs/mylib
```

## Git LFS 用于二进制资源

### 设置

```bash
# 安装 Git LFS
git lfs install

# 跟踪二进制文件
git lfs track "*.so"
git lfs track "*.dylib"
git lfs track "*.dll"
git lfs track "*.a"
git lfs track "assets/*.png"

# 提交 .gitattributes
git add .gitattributes
git commit -m "chore: add Git LFS tracking"
```

### .gitattributes

```
# 原生库
*.so filter=lfs diff=lfs merge=lfs -text
*.dylib filter=lfs diff=lfs merge=lfs -text
*.dll filter=lfs diff=lfs merge=lfs -text
*.a filter=lfs diff=lfs merge=lfs -text

# 资源
*.png filter=lfs diff=lfs merge=lfs -text
*.jpg filter=lfs diff=lfs merge=lfs -text
*.mp4 filter=lfs diff=lfs merge=lfs -text
```

## 最佳实践

### 1. 提交消息

遵循[约定式提交](https://www.conventionalcommits.org/zh-hans/)：

```bash
# 良好的提交消息
git commit -m "feat(android): add ARMv7 support"
git commit -m "fix(ios): resolve memory leak in network module"
git commit -m "docs: update installation guide"
git commit -m "chore: bump version to 1.2.3"
git commit -m "refactor(core): simplify error handling"
git commit -m "test: add unit tests for calculator module"

# 破坏性更改
git commit -m "feat(api)!: change function signature

BREAKING CHANGE: Calculator.add() now returns Result<int>"
```

### 2. 分支命名

```bash
# 功能分支
feature/user-authentication
feature/android-arm64-support

# 错误修复分支
fix/memory-leak-ios
fix/crash-on-startup

# 发布分支
release/1.2.0
release/2.0.0-rc.1

# 热修复分支
hotfix/1.2.1
hotfix/security-patch
```

### 3. 拉取请求工作流

```bash
# 1. 创建功能分支
git checkout -b feature/new-api

# 2. 频繁提交更改
git commit -am "wip: implement new API"
git commit -am "feat: complete new API implementation"
git commit -am "test: add API tests"

# 3. PR 前 rebase 到 main
git fetch origin
git rebase origin/main

# 4. 推送并创建 PR
git push origin feature/new-api

# 5. 处理审查意见
git commit -am "fix: address review comments"
git push origin feature/new-api

# 6. 合并前压缩提交（可选）
git rebase -i origin/main
```

### 4. 签名提交

```bash
# 生成 GPG 密钥
gpg --gen-key

# 配置 git 使用 GPG 密钥
git config --global user.signingkey YOUR_KEY_ID
git config --global commit.gpgsign true

# 签名提交
git commit -S -m "feat: add signed commit"

# 验证签名
git log --show-signature
```

### 5. Git 别名

```bash
# 添加有用的别名到 ~/.gitconfig
[alias]
    co = checkout
    br = branch
    ci = commit
    st = status
    unstage = reset HEAD --
    last = log -1 HEAD
    visual = log --graph --oneline --decorate --all

    # CCGO 特定别名
    ccgo-build = "!f() { ccgo build $1 --release; }; f"
    ccgo-tag = "!f() { ccgo tag --push; }; f"
```

## 故障排除

### 子模块问题

**问题：**
```
fatal: No url found for submodule path 'third_party/lib'
```

**解决方案：**
```bash
# 重新初始化子模块
git submodule deinit -f third_party/lib
rm -rf .git/modules/third_party/lib
git submodule update --init --recursive
```

### 子模块中的分离 HEAD

**问题：**
```
You are in 'detached HEAD' state in submodule
```

**解决方案：**
```bash
cd third_party/lib
git checkout main  # 或适当的分支
cd ../..
git add third_party/lib
git commit -m "chore: fix submodule detached HEAD"
```

### 仓库大小过大

**问题：**
由于历史中的二进制文件导致仓库过大

**解决方案：**
```bash
# 使用 git filter-repo（比 filter-branch 更安全）
pip install git-filter-repo

# 从历史中删除大文件
git filter-repo --path-glob '*.so' --invert-paths
git filter-repo --path-glob '*.a' --invert-paths

# 迁移到 Git LFS
git lfs migrate import --include="*.so,*.a,*.dylib,*.dll"
```

## 资源

### 工具

- [Git](https://git-scm.com/)
- [Git LFS](https://git-lfs.github.com/)
- [约定式提交](https://www.conventionalcommits.org/zh-hans/)
- [GitFlow](https://nvie.com/posts/a-successful-git-branching-model/)

### CCGO 文档

- [CLI 参考](../reference/cli.zh.md)
- [版本管理](version-management.zh.md)
- [CI/CD 设置](../development/contributing.zh.md)
- [发布指南](publishing.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [版本管理](version-management.zh.md)
- [发布指南](publishing.zh.md)
- [CI/CD 设置](../development/contributing.zh.md)
- [CCGO.toml 参考](../reference/ccgo-toml.zh.md)
