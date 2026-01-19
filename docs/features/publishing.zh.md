# 发布管理

使用 CCGO 将 C++ 库发布到各种包仓库的完整指南。

## 概述

CCGO 提供跨多个平台和包管理器的统一发布功能：

- **Android**: Maven Local、Maven Central、私有 Maven 仓库
- **iOS/macOS/Apple**: CocoaPods、Swift Package Manager (SPM)
- **OpenHarmony**: OHPM（官方和私有仓库）
- **跨平台**: Conan（本地和远程）
- **KMP**: Kotlin Multiplatform Maven 发布
- **文档**: GitHub Pages

所有发布命令使用一致的 `--registry` 标志进行目标选择。

## 快速开始

### 基础发布

```bash
# 发布 Android 库到 Maven Local
ccgo publish android --registry local

# 发布到 Maven Central
ccgo publish android --registry official

# 发布到私有 Maven 仓库
ccgo publish android --registry private --url https://maven.example.com

# 发布 iOS/macOS 库
ccgo publish apple --manager cocoapods      # CocoaPods
ccgo publish apple --manager spm --push     # Swift Package Manager
ccgo publish apple --manager all --push     # 两者都发布

# 发布 OpenHarmony 库
ccgo publish ohos --registry official       # 官方 OHPM
ccgo publish ohos --registry private --url https://ohpm.example.com

# 发布 Conan 包
ccgo publish conan --registry local         # Conan 本地缓存
ccgo publish conan --registry official      # 第一个配置的远程仓库
ccgo publish conan --registry private --remote-name myrepo --url URL

# 发布文档
ccgo publish doc --doc-branch gh-pages --doc-open
```

### 跳过构建

发布现有制品而不重新构建：

```bash
# 使用现有 AAR
ccgo publish android --registry local --skip-build

# 使用现有 HAR
ccgo publish ohos --registry official --skip-build
```

## Android 发布 (Maven)

### 仓库类型

**local**: Maven Local (~/.m2/repository/)
- 无需认证
- 立即可用
- 适合本地测试
- 其他人无法访问

**official**: Maven Central (sonatype.org)
- 需要 Sonatype 账号
- 需要 PGP 签名
- 审核流程（2-4 小时）
- 全球可访问

**private**: 自定义 Maven 仓库
- 公司/团队仓库
- 需要认证
- 立即可用
- 团队可访问

### 配置

**gradle.properties:**

```properties
# Maven Central 凭据
SONATYPE_USERNAME=your-username
SONATYPE_PASSWORD=your-password

# PGP 签名
signing.keyId=12345678
signing.password=your-password
signing.secretKeyRingFile=/path/to/secring.gpg

# 私有仓库
PRIVATE_MAVEN_URL=https://maven.example.com
PRIVATE_MAVEN_USERNAME=your-username
PRIVATE_MAVEN_PASSWORD=your-password
```

### 发布命令

```bash
# 发布到 Maven Local 进行测试
ccgo publish android --registry local

# 发布到 Maven Central（生产环境）
ccgo publish android --registry official

# 发布到私有 Maven
ccgo publish android --registry private \
    --url https://maven.example.com \
    --username admin \
    --password secret

# 指定 group ID 和 artifact ID
ccgo publish android --registry official \
    --group-id com.example \
    --artifact-id mylib
```

### Maven Central 发布

**前置条件：**

1. **创建 Sonatype 账号**: https://issues.sonatype.org/
2. **生成 PGP 密钥：**
```bash
gpg --gen-key
gpg --list-secret-keys --keyid-format LONG
gpg --keyserver hkp://pool.sks-keyservers.net --send-keys YOUR_KEY_ID
```

3. **导出私钥：**
```bash
gpg --export-secret-keys YOUR_KEY_ID > ~/.gnupg/secring.gpg
```

4. **在 `~/.gradle/gradle.properties` 中配置凭据**

**发布流程：**

```bash
# 构建并发布
ccgo publish android --registry official

# 上传后，登录 Sonatype 进行发布：
# https://s01.oss.sonatype.org/
# 1. 找到 staging 仓库
# 2. 点击 "Close" 按钮
# 3. 等待验证
# 4. 点击 "Release" 按钮
```

## iOS/macOS 发布 (Apple 平台)

### 包管理器

**CocoaPods**: 传统依赖管理器
- 基于 Podspec
- 中央仓库（CocoaPods Trunk）
- 广泛采用

**Swift Package Manager (SPM)**: Apple 官方解决方案
- 基于 Git
- 无中央仓库
- 原生 Xcode 集成

### CocoaPods 发布

**设置：**

```bash
# 注册 CocoaPods Trunk
pod trunk register your@email.com 'Your Name'

# 验证注册（检查邮件）
```

**发布：**

```bash
# 发布到 CocoaPods Trunk（官方）
ccgo publish apple --manager cocoapods

# 发布到私有 spec 仓库
ccgo publish apple --manager cocoapods \
    --registry private \
    --remote-name myspecs \
    --url https://github.com/company/specs.git
```

**Podspec 验证：**

```bash
# 发布前验证
pod spec lint MyLib.podspec

# 详细输出验证
pod spec lint MyLib.podspec --verbose
```

### Swift Package Manager 发布

**设置：**

```bash
# SPM 使用 Git 标签作为版本
# 确保仓库已初始化
git init
git add .
git commit -m "Initial commit"
```

**发布：**

```bash
# 标记并推送（SPM 发布）
ccgo publish apple --manager spm --push

# 这会创建 git 标签并推送到远程
# SPM 用户可以引用您的仓库
```

**手动 SPM 发布：**

```bash
# 创建版本标签
git tag 1.0.0
git push origin 1.0.0

# 用户添加到 Package.swift:
# .package(url: "https://github.com/user/repo.git", from: "1.0.0")
```

### 发布到两者

```bash
# 同时发布到 CocoaPods 和 SPM
ccgo publish apple --manager all --push

# 这将：
# 1. 发布到 CocoaPods Trunk
# 2. 为 SPM 创建 git 标签
# 3. 推送标签到远程
```

## OpenHarmony 发布 (OHPM)

### 仓库类型

**official**: OpenHarmony Package Manager (ohpm.openharmony.cn)
- 官方仓库
- 需要账号
- 公共包
- 全球可访问

**private**: 自定义 OHPM 仓库
- 公司/团队仓库
- 需要认证
- 私有包

### 配置

**设置 OHPM:**

```bash
# 安装 OHPM
npm install -g @ohos/hpm-cli

# 登录官方仓库
ohpm login

# 配置私有仓库
ohpm config set registry https://ohpm.example.com
```

### 发布命令

```bash
# 发布到官方 OHPM
ccgo publish ohos --registry official

# 发布到私有 OHPM
ccgo publish ohos --registry private --url https://ohpm.example.com

# 使用认证发布
ccgo publish ohos --registry private \
    --url https://ohpm.example.com \
    --token your-auth-token

# 跳过构建并使用现有 HAR
ccgo publish ohos --skip-build
```

### HAR 发布流程

1. CCGO 构建 HAR 包
2. 验证 oh-package.json5
3. 上传到仓库
4. 仓库验证包
5. 包变为可用

**所需元数据：**

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My OpenHarmony library"
authors = ["Your Name <your@email.com>"]
license = "MIT"
homepage = "https://github.com/user/mylib"
repository = "https://github.com/user/mylib"
```

## Conan 发布

### 仓库类型

**local**: Conan 本地缓存 (~/.conan/data/)
- 无需网络
- 立即可用
- 仅测试用

**official**: 第一个配置的 Conan 远程仓库
- 通常是 Conan Center
- 公共包
- 需要审核

**private**: 自定义 Conan 远程仓库
- 公司仓库
- 需要认证
- 立即可用

### 配置

**设置 Conan:**

```bash
# 安装 Conan
pip install conan

# 添加 Conan Center
conan remote add conancenter https://center.conan.io

# 添加私有远程仓库
conan remote add myrepo https://conan.example.com
conan user -p password -r myrepo username
```

### 发布命令

```bash
# 导出到本地缓存
ccgo publish conan --registry local

# 发布到 Conan Center（需要 PR）
ccgo publish conan --registry official

# 发布到私有远程仓库
ccgo publish conan --registry private \
    --remote-name myrepo \
    --url https://conan.example.com

# 跳过构建，仅导出配方
ccgo publish conan --skip-build
```

### Conan 包配方

CCGO 生成 `conanfile.py`:

```python
from conan import ConanFile
from conan.tools.files import copy

class MylibConan(ConanFile):
    name = "mylib"
    version = "1.0.0"
    description = "My C++ library"
    license = "MIT"
    url = "https://github.com/user/mylib"

    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False]}
    default_options = {"shared": False}

    def package(self):
        copy(self, "*.h", src=self.source_folder, dst=self.package_folder)
        copy(self, "*.a", src=self.build_folder, dst=self.package_folder)
        copy(self, "*.so", src=self.build_folder, dst=self.package_folder)
```

## 文档发布 (GitHub Pages)

### 设置

**启用 GitHub Pages:**

1. 仓库设置 → Pages
2. 来源：从分支部署
3. 分支：`gh-pages`（将由 CCGO 创建）

### 发布

```bash
# 生成并发布文档
ccgo publish doc --doc-branch gh-pages --doc-open

# 强制推送（覆盖现有文档）
ccgo publish doc --doc-branch gh-pages --doc-force

# 自定义提交消息
ccgo publish doc --doc-branch gh-pages --doc-message "Update docs for v1.0.0"
```

### 流程

1. CCGO 使用 Doxygen 生成文档
2. 转换为 HTML
3. 创建 `gh-pages` 分支（如果不存在）
4. 提交文档
5. 推送到远程
6. 在浏览器中打开文档 URL

### 自定义域名

**添加 CNAME 文件：**

```toml
[doc]
custom_domain = "docs.example.com"
```

CCGO 在 gh-pages 分支中创建 `CNAME` 文件。

## 版本管理

### 语义化版本控制

CCGO 遵循语义化版本控制（semver）：

```
主版本号.次版本号.修订号
1.0.0 -> 1.0.1（bug 修复）
1.0.1 -> 1.1.0（新功能）
1.1.0 -> 2.0.0（破坏性变更）
```

### CCGO.toml 中的版本

```toml
[package]
name = "mylib"
version = "1.2.3"  # 用于所有发布
```

### 版本标记

```bash
# 创建版本标签
ccgo tag

# 自定义标签
ccgo tag v1.2.3 --message "Release version 1.2.3"

# 这会创建匹配 CCGO.toml 版本的 git 标签
```

## 认证

### 凭据存储

**环境变量：**

```bash
# Maven Central
export SONATYPE_USERNAME=your-username
export SONATYPE_PASSWORD=your-password

# 私有 Maven
export PRIVATE_MAVEN_URL=https://maven.example.com
export PRIVATE_MAVEN_USERNAME=admin
export PRIVATE_MAVEN_PASSWORD=secret

# OHPM token
export OHPM_TOKEN=your-token

# Conan
export CONAN_LOGIN_USERNAME=your-username
export CONAN_PASSWORD=your-password
```

**配置文件：**

```bash
# Gradle: ~/.gradle/gradle.properties
SONATYPE_USERNAME=your-username
SONATYPE_PASSWORD=your-password

# OHPM: ~/.ohpm/auth.json
{
  "registry": {
    "https://ohpm.openharmony.cn": {
      "token": "your-token"
    }
  }
}

# Conan: ~/.conan/remotes.json
{
  "remotes": [
    {
      "name": "myrepo",
      "url": "https://conan.example.com",
      "verify_ssl": true
    }
  ]
}
```

### 安全最佳实践

1. **永远不要提交凭据**到仓库
2. **在 CI/CD 中使用环境变量**
3. **定期轮换令牌**
4. **在可能的情况下使用只读令牌**
5. **在包仓库上启用 2FA**

## CI/CD 集成

### GitHub Actions

```yaml
name: Publish Library

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.10'

      - name: Install CCGO
        run: pip install ccgo

      - name: Publish to Maven Central
        env:
          SONATYPE_USERNAME: ${{ secrets.SONATYPE_USERNAME }}
          SONATYPE_PASSWORD: ${{ secrets.SONATYPE_PASSWORD }}
        run: ccgo publish android --registry official

      - name: Publish to CocoaPods
        env:
          COCOAPODS_TRUNK_TOKEN: ${{ secrets.COCOAPODS_TRUNK_TOKEN }}
        run: ccgo publish apple --manager cocoapods
```

### GitLab CI

```yaml
publish:
  stage: deploy
  only:
    - tags
  script:
    - pip install ccgo
    - ccgo publish android --registry official
    - ccgo publish ohos --registry official
  variables:
    SONATYPE_USERNAME: $SONATYPE_USERNAME
    SONATYPE_PASSWORD: $SONATYPE_PASSWORD
    OHPM_TOKEN: $OHPM_TOKEN
```

### Jenkins

```groovy
pipeline {
    agent any

    stages {
        stage('Publish') {
            when {
                tag '*'
            }
            steps {
                sh 'pip install ccgo'
                withCredentials([
                    usernamePassword(
                        credentialsId: 'maven-central',
                        usernameVariable: 'SONATYPE_USERNAME',
                        passwordVariable: 'SONATYPE_PASSWORD'
                    )
                ]) {
                    sh 'ccgo publish android --registry official'
                }
            }
        }
    }
}
```

## 最佳实践

### 1. 先本地测试

始终使用本地仓库测试：

```bash
# 测试 Maven 发布
ccgo publish android --registry local

# 验证安装
# （在消费者项目中）
implementation 'com.example:mylib:1.0.0'
```

### 2. 版本一致性

确保版本在以下位置匹配：
- CCGO.toml
- Git 标签
- 包元数据

```bash
# CCGO 自动处理这个
ccgo tag        # 从 CCGO.toml 创建标签
ccgo publish    # 使用 CCGO.toml 版本
```

### 3. 变更日志

维护 CHANGELOG.md：

```markdown
# 变更日志

## [1.2.0] - 2024-01-15
### 新增
- 新功能 X
- 支持平台 Y

### 修复
- 模块 Z 中的 bug

## [1.1.0] - 2024-01-01
...
```

### 4. 发布检查清单

发布前：

- [ ] 更新 CCGO.toml 中的版本
- [ ] 更新 CHANGELOG.md
- [ ] 运行测试：`ccgo test`
- [ ] 测试本地构建：`ccgo build`
- [ ] 测试本地发布：`ccgo publish <platform> --registry local`
- [ ] 创建 git 标签：`ccgo tag`
- [ ] 发布：`ccgo publish <platform> --registry official`
- [ ] 验证包可用
- [ ] 更新文档
- [ ] 宣布发布

### 5. 多平台发布

发布到所有平台：

```bash
#!/bin/bash
# publish-all.sh

VERSION=$(ccgo version)

echo "发布版本 $VERSION 到所有平台..."

# Android
ccgo publish android --registry official

# iOS/macOS
ccgo publish apple --manager all --push

# OpenHarmony
ccgo publish ohos --registry official

# Conan
ccgo publish conan --registry official

# 文档
ccgo publish doc --doc-branch gh-pages

echo "发布完成！"
```

## 故障排除

### Maven 发布失败

```
Error: Failed to upload to Maven Central
```

**解决方案：**

1. **检查凭据：**
```bash
echo $SONATYPE_USERNAME
echo $SONATYPE_PASSWORD
```

2. **验证 PGP 签名：**
```bash
gpg --list-secret-keys
cat ~/.gradle/gradle.properties | grep signing
```

3. **检查网络：**
```bash
curl -I https://s01.oss.sonatype.org/
```

4. **验证 POM：**
```bash
# 检查生成的 POM
cat build/publications/release/pom-default.xml
```

### CocoaPods 推送失败

```
Error: Unable to find a pod with name 'MyLib'
```

**解决方案：**

1. **验证 trunk 注册：**
```bash
pod trunk me
```

2. **验证 podspec：**
```bash
pod spec lint MyLib.podspec --verbose
```

3. **检查 spec 仓库：**
```bash
pod repo list
pod repo update
```

### OHPM 发布失败

```
Error: Package already exists
```

**解决方案：**

1. **增加版本：**
```toml
[package]
version = "1.0.1"  # 增加版本
```

2. **检查现有包：**
```bash
ohpm view mylib
```

3. **验证认证：**
```bash
ohpm whoami
```

### Conan 上传失败

```
Error: Recipe 'mylib/1.0.0' already exists
```

**解决方案：**

1. **删除现有版本：**
```bash
conan remove mylib/1.0.0 -r myrepo
```

2. **使用新版本：**
```toml
[package]
version = "1.0.1"
```

3. **检查远程配置：**
```bash
conan remote list
conan user -r myrepo
```

## 另请参阅

- [构建系统](build-system.md)
- [依赖管理](dependency-management.md)
- [Android 平台](../platforms/android.md)
- [iOS 平台](../platforms/ios.md)
- [OpenHarmony 平台](../platforms/openharmony.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
