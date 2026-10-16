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

发布默认使用 `--registry local`。各平台对照如下；Doc 仍使用 `--doc-*`，不读取 `--registry`。

| 目标 | `local` | `official` | `private` |
| --- | --- | --- | --- |
| Android / KMP | Maven Local | Maven Central | 配置的自定义 Maven 仓库 |
| OHOS | `ohpm prepublish` 本地校验 HAR | 固定官方 OHPM 地址 | `--remote` 指定的 OHPM 地址 |
| CocoaPods | 校验 podspec | CocoaPods Trunk | `--remote` 指定 Specs 仓库 |
| SPM | 校验 manifest；`--push` 创建本地版本 tag | 无统一官方直传入口，明确报错 | `--remote` 指定 Git remote，`--push` 推版本 tag |
| Conan | 导出到本地缓存 | 固定 ConanCenter 贡献流程，提示提交 PR；不直传 | `--remote` 指定 Conan remote |
| Index | 校验索引输入；`--push` 更新并提交本地索引 | 默认 CCGO 索引仓库 | `--remote` 查找已配置的 CCGO 索引仓库 |

SPM 与 Index 统一用 `--push` 执行发布。不传时只校验，不创建 tag、不更新或提交索引，也不 clone/pull 索引仓库。`local --push` 只写入本地；远端 registry 的 `--push` 才推送。旧 `--index-push` 是兼容别名。Apple 二进制托管包含 SPM 时也遵循不加 `--push` 只校验的规则；仅 CocoaPods 的显式 artifact-provider 上传流程保持原行为。

私有目的地统一使用 `--remote <URL或名称>`：Android/KMP 与 OHOS 接受仓库 URL；CocoaPods、Conan、SPM 与 Index 接受已配置的仓库名称。旧 `--url`、`--remote-name` 是 `--remote` 的兼容别名，同一次命令只能使用其中一个，避免冲突。Android/KMP 指定 `--remote` 会覆盖该次 Gradle 发布进程的 `CCGO_MAVEN_URL`；不指定则沿用已有 Maven 配置。

Apple 的 `--remote` 指定包描述目的地（Specs 名称或 SPM Git remote），不会覆盖 `--artifact-provider maven` 的 ZIP 托管地址；后者仍由 `CCGO_MAVEN_URL` 等变量配置。

SPM 与 Index 共用 `--version`、`--tag`：默认版本来自 `CCGO.toml`，默认 tag 为 `v<版本>`，与 `ccgo tag` 一致。`--version` 覆盖配置版本；`--tag` 覆盖 tag；单独传 tag 时移除开头 v/V 推导版本，自定义前缀请同时传版本。旧 `--index-version`、`--index-tag` 保留为兼容别名。顶层 `ccgo --version` 仍显示 CLI 版本。

SPM 直接通过包的 Git 仓库和版本 tag 分发，不要求仓库公开；使用者可以配置私有仓库访问凭据。Index 是可选的名字/版本查询目录，每个选定的索引是一个汇总入口，也可配置多个官方或私有索引，并非唯一全局中心。Index 发布引用已有源 tag，SPM 发布可以创建 tag；两者的实际发布职责不同，但参数和校验开关一致。

## 快速开始

### 基础发布

```bash
# 发布 Android 库到 Maven Local
ccgo publish android --registry local

# 发布到 Maven Central
ccgo publish android --registry official

# 发布到私有 Maven 仓库
ccgo publish android --registry private --remote https://maven.example.com

# 发布 iOS/macOS 库
ccgo publish apple --manager cocoapods      # CocoaPods
ccgo publish apple --manager spm --registry private --remote origin --push     # Swift Package Manager
ccgo publish apple --manager all --registry private --remote company --push     # 两者都发布

# 发布 OpenHarmony 库
ccgo publish ohos --registry official       # 官方 OHPM
ccgo publish ohos --registry private --remote https://ohpm.example.com

# 发布 Conan 包
ccgo publish conan --registry local         # Conan 本地缓存
ccgo publish conan --registry official      # ConanCenter：提示通过 PR 贡献，不直传
ccgo publish conan --registry private --remote myrepo

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

## 指定发布输入路径

不传 `--path` 时继续使用原来的默认路径。相对路径以执行命令的当前目录为基准；显式路径不存在、文件类型不匹配或目标不支持时，会在发布前报错。

统一记法：`--path` 指定发布输入，支持目录或该平台的描述/产物文件；Android、OHOS、KMP 还接受 SDK ZIP；Conan、Doc 接受包含 recipe / MkDocs 工程的 ZIP。

| 目标 | 目录输入 | 文件输入 | 不传时的默认行为 |
| --- | --- | --- | --- |
| Android | 含 `gradlew` 的工程目录，或含唯一 AAR 的产物目录 | `.aar`、含唯一 AAR 的 SDK `.zip` | 使用 `android/` 工程和默认 AAR 路径 |
| OHOS | 含 `oh-package.json5` 的包目录，或含唯一 HAR 的产物目录 | `.har`、含唯一 HAR 的 SDK `.zip` | 在 `ohos/` 执行 OHPM |
| KMP | Gradle 工程目录或 CCGO KMP SDK 目录 | CCGO KMP SDK `.zip` | 使用 `kmp/` 工程 |
| Apple / CocoaPods | 含唯一 podspec 的目录；指定 `--artifact-provider` 时为二进制目录 | `.podspec`；指定 provider 时支持 `.zip` | 查找 podspec；provider 模式查找 release Apple SDK ZIP |
| Apple / SPM | 含 `Package.swift` 的包仓库目录；指定 provider 时为二进制目录 | `Package.swift`；指定 provider 时支持 `.zip` | 当前目录；provider 模式查找 release Apple SDK ZIP |
| Conan | 含 `conanfile.py` 的 recipe 目录 | Python recipe 文件（通常为 `conanfile.py`）、包含该 recipe 的 `.zip` | 按原逻辑查找 CCGO 工程 |
| Doc | 含 `mkdocs.yml` 或 `mkdocs.yaml` 的工程目录 | MkDocs `.yml` / `.yaml` 配置文件、包含 MkDocs 工程的 `.zip` | 当前工程的 MkDocs 配置 |
| Index | 含唯一 SDK ZIP 的目录 | SDK `.zip`，用于计算 checksum | 默认 `target/release/package/` 路径 |

Android/OHOS 产物目录和 SDK ZIP 中会递归查找对应文件，Index 目录递归查找 ZIP；多个候选文件会报错，需指定精确文件。工程目录优先按工程发布：Android 以 `gradlew`、OHOS 以 `oh-package.json5` 为标识。CocoaPods 只在指定目录本层查找 podspec。Apple 的文件输入需要对应 `--manager`；`--manager all` 的目录必须同时含 podspec 和 `Package.swift`。

显式产物（单文件、产物目录、SDK ZIP）直接复用，不重新构建；工程目录沿用对应平台发布流程。ZIP 只临时解压，清理只删除本次创建的临时目录，绝不删除或修改原 ZIP、输入目录。

```bash
ccgo publish android --path ./artifacts/library.aar --registry local
ccgo publish ohos --path ./artifacts/library.har --registry official
ccgo publish apple --manager cocoapods --path ./specs/Library.podspec
ccgo publish kmp --path ./custom-kmp --registry local
ccgo publish android --path ./ANDROID_SDK.zip --registry local
ccgo publish ohos --path ./OHOS_SDK.zip --registry official
ccgo publish apple --manager spm --path ./swift-package
ccgo publish conan --path ./recipes/conanfile.py --registry local
ccgo publish doc --path ./docs-project/mkdocs.yml
ccgo publish doc --path ./docs-project.zip
ccgo publish conan --path ./conan-project.zip --registry local
ccgo publish index --version 1.2.3 --index-repo ./package-index \
  --path ./artifacts/SDK.zip --checksum \
  --archive-url-template 'https://cdn.example.com/{name}-{version}.zip'
```

Android 指定产物时仍需要当前项目的 `android/` Gradle 工程；指定含 `gradlew` 的工程目录时使用该目录。Gradle 工程提供 Maven 坐标、POM、签名与仓库配置。指定 AAR 的坐标由工程配置决定，不从文件名推断；`--release` 和 `--stl` 不会转换已有 AAR，请确保配置与产物一致。需要使用支持 `ccgoPublishPath` / `ccgoSkipPublishBuild` 属性的新版 CCGO Gradle 插件；CLI 会在执行发布任务前检查插件是否支持；旧版插件会报错，CLI 和插件需一起更新。

CocoaPods 会在指定 podspec 所在目录执行命令。未指定 artifact provider 时，Index 的 `--path` 必须同时使用 `--checksum` 和 `--archive-url-template`，且仅支持 `--archive-format zip`；指定文件读取失败会直接报错。ZIP 必须另行上传到模板所指的地址，此模式不会上传文件。

KMP 的 `--path` 支持两种目录和 SDK ZIP：

- Gradle 项目目录包含 `gradlew`，沿用工程构建与发布流程。
- SDK 目录包含 `lib/kmp/` 和 `meta/kmp/build_info.json`；ZIP 包含相同结构，也允许额外包一层目录。SDK 模式复用当前项目的 `kmp/` 工程及新版 CCGO Gradle 插件配置，只发布包内已有产物，不重新构建。

```bash
ccgo publish kmp --path ./STDCOMM_KMP_SDK-0.0.5.zip --registry local
ccgo publish kmp --path ./unpacked-sdk --registry private --remote https://maven.example.com
```

SDK 版本取自包内 `build_info.json`，项目名必须与当前项目匹配。Group ID 和基础 artifact ID 来自 `[publish.kmp.maven]`（沿用插件的回退配置）。例如包内只有 `lib/kmp/android/stdcomm-kmp-release.aar` 时，会发布 `<group_id>:<artifact_id>-android:0.0.5`。Desktop JAR 发布到 `-desktop`，Native KLIB 发布到小写 target 后缀（如 `-iosarm64`），cinterop KLIB 作为同模块的附加 classifier。

SDK 包没有完整 POM/Gradle module 元数据，因此这里只发布可直接引用的平台模块，不生成根 KMP 模块，不宣称支持 commonMain 的自动平台选择。POM 依赖来自项目配置的 KMP dependencies；SDK 使用者需要确保这些配置完整。Maven Central 仍要求完整的发布元信息、签名及配套制品，SDK 模式不会自动补造 sources/javadoc。ZIP 解压在临时目录中，结束后清理；路径穿越、符号链接、缺失元信息和同一平台的多个主产物会报错。

Doc ZIP 解压后需包含唯一的 `mkdocs.yml` 或 `mkdocs.yaml`；Conan ZIP 需包含唯一的 `conanfile.py`。允许带任意层外层目录，命令会递归查找描述文件；缺失或存在多个候选文件时会报错。请同时打包配置/recipe 引用的相对路径文件，原目录结构会保留。临时解压目录会在成功或失败后清理，原 ZIP 不受影响。

Doc ZIP 发布从执行命令时所在的 Git 仓库运行 `mkdocs gh-deploy`，并通过绝对路径读取解压后的配置；因此仍需在要部署文档的 Git 仓库中执行命令。文档 ZIP 包含 MkDocs 工程与源文档，不是仅含预生成 HTML 的站点压缩包。Conan ZIP 会在 recipe 所在目录执行原有 export/upload 流程，不会将任意二进制 ZIP 自动转成 Conan 包。

### Index 产物托管

Index 支持 `--artifact-provider maven|github`。`--path` 可指定 ZIP 或包含唯一 ZIP 的目录；默认查找 `target/release/package/<大写项目名>_CCGO_PACKAGE-<版本>.zip`。直接上传已有 ZIP，自动生成下载 URL 并计算 SHA-256 写入索引，无需手写 URL 或添加 `--checksum`。provider 模式不能同时指定 `--archive-url-template`。

```bash
ccgo publish index --registry private --remote company \
  --artifact-provider maven --artifact-group com.example \
  --path ./STDCOMM_CCGO_PACKAGE-0.0.5.zip --push
```

`--registry` 与 `--remote` 选择包索引仓库。Maven 的 ZIP 存储地址和凭据仍使用 `CCGO_MAVEN_URL`、`CCGO_MAVEN_USERNAME`、`CCGO_MAVEN_PASSWORD`。group 来自 `--artifact-group` 或 `[publish.index.maven].group_id`；artifact ID 默认为 `<项目名>-ccgo`，可用 `--artifact-id` 覆盖。GitHub 则使用 `--artifact-provider github --github-repo company/library`，选定 tag 对应的 Release 必须已经存在。

发布前源 Git tag 必须已存在，Index 不创建或推送源 tag；各平台可复用同一源 tag。不加 `--push` 只校验，不上传、不修改索引。加 `--push` 时先检查索引版本是否重复，再上传产物。后续步骤失败可能留下已上传的产物，不自动回滚。成功或失败都会清理临时副本，原始 ZIP 保留。

### Apple 二进制托管

`--artifact-provider maven|github` 启用 Apple 二进制上传。此时 `--path` 指向包含 XCFramework 的目录、单个 `.xcframework` 目录或 ZIP；当前工作目录需要 `CCGO.toml`，以及对应 manager 的 podspec / `Package.swift`。默认 manager 为 `all`，只发布 CocoaPods 时请指定 `--manager cocoapods`。

- `--artifact-provider` 选择 ZIP 托管位置，与包索引仓库独立。
- `--registry local|official|private` 选择 CocoaPods Specs 操作：验证、Trunk 或私有 Specs Git 仓库。`private` 需要 `--remote`。Apple 的 `--registry` 控制 CocoaPods Specs 发布，不控制 Maven/GitHub 的 ZIP 托管；SPM 的 local/private 控制包装仓库校验与 Git tag 发布；official 不提供直传。
- Maven 使用单个 `CCGO_MAVEN_URL` 和 `CCGO_MAVEN_USERNAME` / `CCGO_MAVEN_PASSWORD`。URL 必须是支持 PUT 上传的 hosted 仓库，不是只读代理或 Maven Central Portal。`--artifact-group` 或 `[publish.apple.maven].group_id` 指定 groupId；artifactId 默认 `<项目名>-apple`，可用 `--artifact-id` 覆盖。版本来自 `CCGO.toml`，podspec 版本必须一致。上传 ZIP、`packaging=zip` 的 POM 和各自的 SHA-256 文件。当前不接受逗号分隔的多个仓库 URL。
- GitHub 使用 `gh` 的认证，要求 `--github-repo owner/repo` 和已存在的 `v<版本>` Release。上传资产名同样为 `<artifactId>-<版本>.zip`，不会覆盖同名资产。

```bash
ccgo publish apple --manager cocoapods \
  --path ./STDCOMM_APPLE_SDK-0.0.5.zip \
  --artifact-provider maven --artifact-group com.example \
  --registry private --remote company-specs

ccgo publish apple --manager spm \
  --path ./Library.xcframework \
  --artifact-provider github --github-repo company/library --push
```

不传 `--path` 时，provider 模式在 `target/release/ios/` 和 `target/release/macos/` 查找当前项目版本的 SDK ZIP，排除 `-SYMBOLS.zip`；可用 `--platform ios|macos` 缩小范围。缺失或多个候选都会报错。此模式不触发构建。

原 ZIP 按原字节上传，目录则临时打包；输入保持不变，临时文件在成功或失败后清理。目录中的符号链接必须指向输入目录内部。CocoaPods 保留 podspec 的依赖、vendored 路径等元信息，只替换 `source.http` 和 `source.sha256`，请确保这些路径与 ZIP 内容匹配。生成文件位于 `target/ccgo-publish/apple/<版本>/`，该目录保留供检查，源 podspec / manifest 不被修改。

SPM 目前支持 ZIP 根目录有一个 XCFramework、现有 `Package.swift` 有一个使用字面量 URL 和 checksum 的远程 `binaryTarget`。provider 模式不加 `--push` 只校验输入和生成规则，不上传或写文件；加 `--push` 才上传 ZIP 并生成新 `Package.swift`。随后需将生成文件提交到 SPM 包装仓库，再以普通 SPM 发布模式推版本 tag，避免给旧 manifest 打标签。包含嵌套或多个 XCFramework 的完整 SDK ZIP 可选 CocoaPods，不会自动转换成 SPM 兼容 ZIP。

上传凭据不会写入描述文件，也不会自动配置到使用者机器。如果 ZIP 下载需要认证，使用者必须另行配置 CocoaPods / SPM 下载认证。`--registry local` 仍会执行显式选择的 provider 上传，然后运行 `pod spec lint` 验证远程 ZIP。网络发布不是事务：后续步骤失败不会撤回已上传的 ZIP/POM，请按输出地址检查远端状态再重试。

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
    --remote https://maven.example.com \
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
ccgo publish apple --manager cocoapods --registry official

# 发布到私有 spec 仓库
ccgo publish apple --manager cocoapods \
    --registry private \
    --remote myspecs \
    --remote https://github.com/company/specs.git
```

**Podspec 验证：**

```bash
# 发布前验证
pod spec lint MyLib.podspec

# 详细输出验证
pod spec lint MyLib.podspec --verbose
```

### Swift Package Manager 发布

`local` 默认运行 `swift package dump-package` 校验 manifest，加 `--push` 才创建本地 tag（不联系远端）。`private` 表示自定义 Git remote，可指向公共或私有仓库；用 `--remote origin` 等选定，只有加 `--push` 才创建并推送项目版本 tag。该 remote 必须已通过 `git remote add` 配置。发布前须提交所有包装仓库文件。加 `--push` 时先检查 tag：不存在才创建；已存在且指向当前 HEAD 则直接复用，重复推送也可以；若指向不同 commit 则报错，绝不覆盖。`official` 明确报错：SPM 没有与 CocoaPods Trunk 对应的官方直传中心。这里没有实现 Swift Package Registry HTTP 协议或 Swift Package Index 收录。


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
ccgo publish apple --manager spm --registry private --remote origin --push

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
ccgo publish apple --manager all --registry private --remote company --push

# company 必须同时是已配置的 CocoaPods Specs 仓库名和 Git remote 名
# 如果两个名称不同，请分别运行两个 manager
# SPM wrapper 的文件必须已经提交
```

## OpenHarmony 发布 (OHPM)

### 仓库类型

**local**: 使用 `ohpm prepublish <HAR>` 校验包内容，不上传、不安装到工程。需要支持 `prepublish` 的 OHPM 版本。工程目录中必须有唯一已构建的 HAR，否则请通过 `--path` 精确指定；此命令不自动构建。


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
ohpm config set publish_registry https://ohpm.example.com
```

### 发布命令

```bash
# 发布到官方 OHPM
ccgo publish ohos --registry official

# 发布到私有 OHPM
ccgo publish ohos --registry private --remote https://ohpm.example.com

# 使用认证发布
ccgo publish ohos --registry private \
    --remote https://ohpm.example.com

# 跳过构建并使用现有 HAR
ccgo publish ohos --skip-build
```

### HAR 发布流程

1. 选择已有 HAR：`--path` 文件 / SDK ZIP / 目录，或默认 `ohos/` 目录
2. `local` 执行 `ohpm prepublish` 后结束
3. `official` 使用 `--publish_registry https://ohpm.openharmony.cn/ohpm/` 上传
4. `private` 使用 `--publish_registry <url>` 上传，认证沿用 OHPM 配置

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

**local**: Conan 2 本地缓存 (~/.conan2/)
- 无需网络
- 立即可用
- 仅测试用

**official**: 固定 ConanCenter（`https://center2.conan.io`）
- 不会使用第一个 remote，也不会被 `--remote` 改写
- ConanCenter 不接受普通 CLI 直传，此模式返回说明和 [配方 PR 入口](https://github.com/conan-io/conan-center-index)
- 不会自动创建 PR 或上传到其他仓库

**private**: 自定义 Conan 远程仓库
- 必须指定 `--remote`，不会自动选择第一个 remote
- 仅上传本次导出的精确配方引用，不使用 `*`
- 公司仓库
- 需要认证
- 立即可用

### 配置

**设置 Conan:**

```bash
# 安装 Conan
pip install conan

# 添加 Conan Center
conan remote add conancenter https://center2.conan.io

# 添加私有远程仓库
conan remote add myrepo https://conan.example.com
conan remote login myrepo username
```

### 发布命令

```bash
# 导出到本地缓存
ccgo publish conan --registry local

# 显示 ConanCenter 的 PR 贡献入口（返回错误，不执行直传）
ccgo publish conan --registry official

# 发布到私有远程仓库
ccgo publish conan --registry private \
    --remote myrepo

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
ccgo publish apple --manager all --registry private --remote company --push

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

## Index 发布与仓库选择

Index 的 `--registry` 决定索引 Git 仓库，包的源仓库仍由项目 `origin` 提供。

版本默认读取 `CCGO.toml` 的 `[project].version` 或 `[package].version`，无需传 `--version`。默认引用已有 Git tag `v<版本>`；Index 不自动创建或推送源仓库 tag。`--version` 可覆盖版本；`--tag` 可覆盖 tag，单独指定 tag 时沿用从 tag 推导版本的规则（移除开头的 v/V）。使用自定义 tag 前缀时请同时显式指定版本。每次只追加一个版本，重复版本仍会报错。

```bash
# 校验默认版本和已有源 tag，不更新索引
ccgo publish index --registry local

# 离线更新并提交本地索引，默认 $CCGO_HOME/registry/publish/local
ccgo publish index --registry local --push

# 指定独立的本地索引目录（不能是项目源仓库或其父目录）
ccgo publish index --registry local --index-repo ./package-index --push

# 官方索引：固定使用 CCGO 默认仓库；--push 需要对应写权限
ccgo publish index --registry official --push

# 先配置私有索引；也可使用 CCGO.toml 的 [registries]
ccgo registry add company git@github.com:company/ccgo-index.git
ccgo publish index --registry private --remote company --push
```

`official` 固定为 `git@github.com:zhlinh/ccgo-index.git`。`private` 按 `--remote` 先查项目 `[registries]`，再查全局 `ccgo registry add` 配置；未配置时明确报错，不会回退到其他仓库。这里的 remote 是 CCGO registry 名称，不是项目 Git remote 名称。远端索引准备在 `$CCGO_HOME/registry/publish/<名称>/`（默认 `$CCGO_HOME` 为 `~/.ccgo`）。不加 `--push` 时只做本地校验，不创建/更新索引工作目录或提交，也不 clone/pull。该阶段无法确认远端重复版本；实际发布时会拉取索引并检查重复版本。

官方索引内部名称为 `ccgo-index`。SSH 地址要求配置 GitHub SSH 访问，发布还需仓库写权限。新索引需要根目录的 `index.json`（符合 `IndexMetadata` 格式），并将初始提交推送到默认分支；包文件会在发布时自动创建。如果已有发布缓存仍指向旧官方地址，请先保留其中的本地提交，再将旧工作目录移走后重试；CCGO 会拒绝 origin 不匹配，不会自动切换仓库。已显式保存的 registry URL 不会被自动重写。

`--index-repo` 现在仅用于 local 的目录路径，`--index-name` 仅命名 local 索引；远程 URL 的旧用法请迁移到 `ccgo registry add` 配合 `--registry private --remote`。`--index-push` 保留兼容，与 `--push` 完全相同。local 不会 clone/pull；`local --push` 更新并提交本地索引，不推送到网络。拉取失败保留工作目录；Git 提交失败返回错误，不会继续推送旧提交。
