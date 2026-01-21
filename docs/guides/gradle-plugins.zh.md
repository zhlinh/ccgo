# CCGO Gradle 插件参考

> 版本：v3.0.10 | 更新时间：2026-01-21

## 概述

CCGO Gradle 插件为使用 CCGO 进行原生 C++ 库开发的 Android 和 Kotlin 多平台项目提供基于约定的配置。这些插件在 CCGO 项目中标准化了构建配置、原生库集成和发布工作流程。

**发布到 Maven Central**：`com.mojeter.ccgo.gradle`

## 主要特性

- **基于约定的配置**：基于 CCGO 最佳实践的标准化构建设置
- **CCGO.toml 集成**：自动从 CCGO.toml 读取项目配置
- **原生构建支持**：基于 Python 的（ccgo CLI）和基于 CMake 的原生构建
- **发布就绪**：预配置 Maven Central 和自定义仓库发布
- **Android 和 KMP**：完全支持 Android 库和 Kotlin 多平台
- **类型安全**：基于 Kotlin DSL 的插件，带编译时验证

---

## 可用插件

### Android 库插件

#### `com.mojeter.ccgo.gradle.android.library`
支持 Kotlin 的基本 Android 库配置。

**应用**：
- `com.android.library`
- `org.jetbrains.kotlin.android`
- `com.mojeter.ccgo.gradle.android.lint`

**配置**：
- Kotlin 编译（JVM 目标、源代码兼容性）
- 产品变体（从 CCGO.toml 读取）
- Gradle 管理的测试设备
- APK 归档任务

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
}
```

#### `com.mojeter.ccgo.gradle.android.library.compose`
支持 Jetpack Compose 的 Android 库。

**扩展**：`android.library`

**额外配置**：
- Compose 编译器设置
- Compose 依赖

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library.compose)
}
```

#### `com.mojeter.ccgo.gradle.android.feature`
模块化 Android 应用的功能模块配置。

**扩展**：`android.library`

**额外配置**：
- 功能模块依赖
- Navigation 组件设置
- Hilt 依赖注入集成

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.feature)
}
```

---

### 原生构建插件

#### `com.mojeter.ccgo.gradle.android.library.native.python`
集成使用 `ccgo` CLI（基于 Python 的构建编排器）构建的原生 C++ 库。

**扩展**：`android.library`

**构建流程**：
1. 读取 CCGO.toml 获取项目配置
2. 创建 `buildLibrariesForMain` 任务
3. 执行 `ccgo build android --arch <archs> --native-only`
4. 将原生库（.so 文件）集成到 AAR 中

**Gradle 任务流程**：
```
cleanTheTargetDir → buildLibrariesForMain → mergeProdReleaseJniLibFolders → assembleProdRelease
```

**环境变量**：
- `ANDROID_HOME`：Android SDK 路径
- `NDK_ROOT`：Android NDK 路径
- `CMAKE_HOME`：CMake 安装路径

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
}
```

**构建命令**：
```bash
./gradlew assembleProdRelease
```

#### `com.mojeter.ccgo.gradle.android.library.native.cmake`
直接 CMake 集成，无需 ccgo CLI 即可进行原生构建。

**扩展**：`android.library`

**配置**：
```kotlin
android {
    externalNativeBuild {
        cmake {
            path = file("${rootDir.parentFile}/CMakeLists.txt")
            version = "3.22.1"
        }
    }
    defaultConfig {
        externalNativeBuild {
            cmake {
                cppFlags += listOf("-fpic", "-frtti", "-fexceptions", "-Wall")
                arguments += listOf(
                    "-GNinja",
                    "-DANDROID_PLATFORM=android-21",
                    "-DANDROID_TOOLCHAIN=clang",
                    "-DANDROID_STL=c++_shared"
                )
            }
        }
        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86_64")
        }
    }
}
```

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.cmake)
}
```

#### `com.mojeter.ccgo.gradle.android.library.native.empty`
没有原生代码的 Android 库（纯 Kotlin/Java 包装器）。

**使用场景**：
- 预构建原生库的 Kotlin 包装器
- 带 JNI 绑定的纯 Kotlin/Java SDK
- 无原生编译的测试和调试

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.empty)
}
```

---

### Android 应用插件

#### `com.mojeter.ccgo.gradle.android.application`
Android 应用配置。

**应用**：
- `com.android.application`
- `org.jetbrains.kotlin.android`
- `com.mojeter.ccgo.gradle.android.lint`

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.application)
}
```

#### `com.mojeter.ccgo.gradle.android.application.compose`
带 Jetpack Compose UI 的 Android 应用。

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.application.compose)
}
```

#### `com.mojeter.ccgo.gradle.android.application.flavors`
产品变体配置（dev、staging、prod）。

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.application.flavors)
}
```

---

### 代码质量和测试插件

#### `com.mojeter.ccgo.gradle.android.lint`
采用 CCGO 最佳实践的 Android Lint 配置。

**配置**：
- 警告严重级别
- 基线文件支持
- HTML 和 XML 报告

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.lint)
}
```

#### `com.mojeter.ccgo.gradle.android.library.jacoco`
Android 库的 Jacoco 代码覆盖率。

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library.jacoco)
}

// 生成覆盖率报告
./gradlew jacocoTestReport
```

#### `com.mojeter.ccgo.gradle.android.application.jacoco`
Android 应用的代码覆盖率。

---

### 依赖注入和数据库插件

#### `com.mojeter.ccgo.gradle.android.hilt`
Hilt 依赖注入设置。

**应用**：
- `com.google.dagger.hilt.android`
- 用于注解处理的 KSP

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.hilt)
}
```

#### `com.mojeter.ccgo.gradle.android.room`
使用 KSP 的 Room 数据库配置。

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.room)
}
```

---

### 发布插件

#### `com.mojeter.ccgo.gradle.android.publish`
Android 库的 Maven 发布配置。

**应用**：
- `com.vanniktech.maven.publish`
- `signing`

**特性**：
- 自动 CCGO.toml 集成
- 源代码和 Javadoc JAR 生成
- 带依赖的 POM 生成
- 签名配置（GPG 或内存密钥）
- Maven Central、本地和自定义仓库支持

**发布命令**：
```bash
# 发布到 Maven Local（~/.m2/repository）
./gradlew publishToMavenLocal

# 发布到 Maven Central
./gradlew publishToMavenCentral

# 发布到自定义仓库
./gradlew publishToMavenCustom
```

**配置源**（优先级顺序）：
1. 环境变量（CI/CD 覆盖）
2. CCGO.toml（项目根目录）
3. gradle.properties（项目级）
4. ~/.gradle/gradle.properties（用户级）

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.publish)
}
```

#### `com.mojeter.ccgo.gradle.kmp.publish`
Kotlin 多平台库发布。

**支持**：
- Android、iOS、JVM 目标
- 自动依赖配置
- 多平台产物发布

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.kmp.publish)
}
```

---

### Kotlin 多平台插件

#### `com.mojeter.ccgo.gradle.kmp.library.native.python`
使用 CCGO 构建原生代码的 KMP 库。

**目标**：
- Android（ARM、ARM64、x86_64）
- iOS（ARM64、模拟器 x86_64/ARM64）
- JVM

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.kmp.library.native.python)
}
```

#### `com.mojeter.ccgo.gradle.kmp.library.native.empty`
没有原生代码的 KMP 库。

---

### 根项目插件

#### `com.mojeter.ccgo.gradle.android.root`
多模块 Android 项目的根项目配置。

**配置**：
- 构建缓存设置
- 公共仓库
- Gradle 版本目录

**用法**（根 build.gradle.kts）：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.root) apply false
}
```

#### `com.mojeter.ccgo.gradle.kmp.root`
KMP 项目的根配置。

---

### JVM 插件

#### `com.mojeter.ccgo.gradle.jvm.library`
纯 JVM/Kotlin 库（无 Android）。

**应用**：
- `org.jetbrains.kotlin.jvm`
- Java 库约定

**用法**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.jvm.library)
}
```

---

## CCGO.toml 集成

所有插件自动从项目根目录（或父目录）的 `CCGO.toml` 读取配置。

### CCGO.toml 示例

```toml
[project]
name = "ccgonow"
version = "1.0.0"
repository = "https://github.com/zhlinh/ccgo-now"

[build]
cmake_version = "3.22.1"

[android]
compile_sdk = 34
min_sdk = 21
target_sdk = 34
ndk_version = "25.2.9519653"
build_tools = "34.0.0"
stl = "c++_shared"
default_archs = ["armeabi-v7a", "arm64-v8a", "x86_64"]

[publish.maven]
group_id = "com.mojeter.ccgo"
artifact_id = "ccgonow"
channel_desc = "beta"
dependencies = [
    "androidx.core:core-ktx:1.12.0",
    "org.jetbrains.kotlin:kotlin-stdlib:1.9.20"
]

[publish.kmp]
group_id = "com.mojeter.ccgo.kmp"
artifact_id = "ccgonow-kmp"
android_min_sdk = 24
ios_deployment_target = "14.0"

[[publish.kmp.dependencies]]
group = "org.jetbrains.kotlinx"
artifact = "kotlinx-coroutines-core"
version = "1.7.3"
```

### 插件读取的配置字段

| 字段 | 插件 | 用途 |
|------|------|------|
| `project.name` | 全部 | 项目名称 |
| `project.version` | 发布 | 版本号 |
| `project.repository` | 发布 | Git 仓库 URL |
| `android.compile_sdk` | Android | 编译 SDK 版本 |
| `android.min_sdk` | Android | 最低 SDK 版本 |
| `android.target_sdk` | Android | 目标 SDK 版本 |
| `android.ndk_version` | 原生 | NDK 版本 |
| `android.stl` | 原生 | STL 类型（c++_shared/c++_static） |
| `android.default_archs` | 原生 | 构建架构 |
| `build.cmake_version` | 原生 | CMake 版本 |
| `publish.maven.group_id` | 发布 | Maven group ID |
| `publish.maven.artifact_id` | 发布 | Maven artifact ID |
| `publish.maven.dependencies` | 发布 | POM 依赖 |
| `publish.kmp.*` | KMP | KMP 特定设置 |

---

## 设置和安装

### 1. 配置插件仓库

在 `settings.gradle.kts` 中：

```kotlin
pluginManagement {
    repositories {
        mavenCentral()  // 已发布的版本
        mavenLocal()    // 本地开发
        google()
        gradlePluginPortal()
    }
}
```

### 2. 声明插件版本

在 `gradle/libs.versions.toml` 中：

```toml
[versions]
ccgo-buildlogic = "1.0.0"

[plugins]
ccgo-android-library = { id = "com.mojeter.ccgo.gradle.android.library", version.ref = "ccgo-buildlogic" }
ccgo-android-library-native-python = { id = "com.mojeter.ccgo.gradle.android.library.native.python", version.ref = "ccgo-buildlogic" }
ccgo-android-publish = { id = "com.mojeter.ccgo.gradle.android.publish", version.ref = "ccgo-buildlogic" }
ccgo-kmp-library-native-python = { id = "com.mojeter.ccgo.gradle.kmp.library.native.python", version.ref = "ccgo-buildlogic" }
ccgo-kmp-publish = { id = "com.mojeter.ccgo.gradle.kmp.publish", version.ref = "ccgo-buildlogic" }
```

### 3. 应用插件

**根 build.gradle.kts**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.root) apply false
    alias(libs.plugins.ccgo.android.library) apply false
    alias(libs.plugins.ccgo.android.library.native.python) apply false
    alias(libs.plugins.ccgo.android.publish) apply false
}
```

**模块 build.gradle.kts**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
    alias(libs.plugins.ccgo.android.publish)
}

android {
    namespace = "com.mojeter.ccgo.example"
}
```

---

## 完整示例

### 示例 1：带原生代码的 Android 库（Python 构建）

**项目结构**：
```
myproject/
├── CCGO.toml                    # 项目配置
├── CMakeLists.txt               # 原生构建（ccgo CLI 使用）
├── src/                         # C++ 源代码
├── android/                     # Android Gradle 项目
│   ├── settings.gradle.kts
│   ├── build.gradle.kts
│   ├── gradle/
│   │   └── libs.versions.toml
│   └── library/
│       ├── build.gradle.kts
│       └── src/main/
│           ├── AndroidManifest.xml
│           └── kotlin/...       # Kotlin 包装器代码
```

**android/library/build.gradle.kts**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
    alias(libs.plugins.ccgo.android.publish)
}

android {
    namespace = "com.mojeter.ccgo.myproject"

    defaultConfig {
        consumerProguardFiles("consumer-rules.pro")
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.kotlin.stdlib)
}
```

**构建命令**：
```bash
# 构建带原生库的 AAR
cd android
./gradlew assembleProdRelease

# 发布到 Maven Local
./gradlew publishToMavenLocal

# 发布到 Maven Central
./gradlew publishToMavenCentral
```

---

### 示例 2：使用 CMake 的 Android 库（无 CCGO CLI）

**android/library/build.gradle.kts**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.cmake)
    alias(libs.plugins.ccgo.android.publish)
}

android {
    namespace = "com.example.native"

    externalNativeBuild {
        cmake {
            path = file("${rootDir.parentFile}/CMakeLists.txt")
        }
    }
}
```

---

### 示例 3：带原生代码的 Kotlin 多平台

**kmp/build.gradle.kts**：
```kotlin
plugins {
    alias(libs.plugins.ccgo.kmp.library.native.python)
    alias(libs.plugins.ccgo.kmp.publish)
}

kotlin {
    androidTarget {
        publishLibraryVariants("release")
    }

    iosX64()
    iosArm64()
    iosSimulatorArm64()

    jvm()

    sourceSets {
        val commonMain by getting {
            dependencies {
                implementation(libs.kotlinx.coroutines.core)
            }
        }

        val androidMain by getting {
            dependencies {
                implementation(libs.androidx.core.ktx)
            }
        }
    }
}

android {
    namespace = "com.mojeter.ccgo.kmp"
    compileSdk = 34
}
```

---

### 示例 4：发布配置

**在 `~/.gradle/gradle.properties` 中配置凭据**：

```properties
# Maven Central（从 https://central.sonatype.com/account 获取）
mavenCentralUsername=your-user-token-username
mavenCentralPassword=your-user-token-password

# 签名（内存 PGP 密钥）
signingInMemoryKey=-----BEGIN PGP PRIVATE KEY BLOCK-----\n...\n-----END PGP PRIVATE KEY BLOCK-----
signingInMemoryKeyPassword=your-key-password

# 自定义 Maven 仓库（可选）
mavenCustomUrls=https://maven.example.com/releases
mavenCustomUsernames=your-username
mavenCustomPasswords=your-password

# 本地 Maven 路径（可选，默认为 ~/.m2/repository）
mavenLocalPath=~/custom-maven-local
```

**或使用环境变量**（用于 CI/CD）：

```bash
export MAVEN_CENTRAL_USERNAME=your-username
export MAVEN_CENTRAL_PASSWORD=your-password
export SIGNING_IN_MEMORY_KEY="$(cat signing-key.asc)"
export SIGNING_IN_MEMORY_KEY_PASSWORD=your-password
export MAVEN_CUSTOM_URLS=https://maven.example.com/releases
export MAVEN_CUSTOM_USERNAMES=your-username
export MAVEN_CUSTOM_PASSWORDS=your-password

./gradlew publishToMavenCentral
```

---

## 配置优先级

配置值按以下优先级顺序解析（从高到低）：

1. **环境变量** - CI/CD 覆盖
2. **CCGO.toml** - 项目特定设置
3. **gradle.properties（项目级）** - 项目默认值
4. **gradle.properties（用户级 ~/.gradle/）** - 用户默认值
5. **插件默认值** - 回退值

**解析示例**：

```
NDK 版本解析：
1. 检查 CCGO.toml：[android].ndk_version = "25.2.9519653" ✓（使用）
2. 检查 gradle.properties：ndkVersion=26.0.0（忽略）
3. 插件默认值："25.2.9519653"（未到达）

结果："25.2.9519653"
```

---

## 发布工作流程

### 本地开发

```bash
# 1. 本地构建和测试
./gradlew assembleProdRelease
./gradlew test

# 2. 发布到 Maven Local
./gradlew publishToMavenLocal

# 3. 在另一个项目中使用
// settings.gradle.kts
repositories {
    mavenLocal()
}

// build.gradle.kts
dependencies {
    implementation("com.mojeter.ccgo:myproject:1.0.0")
}
```

### Maven Central 发布

```bash
# 1. 更新 CCGO.toml 中的版本
[project]
version = "1.0.0"

# 2. 构建和测试
./gradlew clean assembleProdRelease
./gradlew test

# 3. 发布到 Maven Central
./gradlew publishToMavenCentral

# 4. 在 https://central.sonatype.com 监控发布状态
```

### 自定义仓库

```bash
# 在 gradle.properties 或环境变量中配置
mavenCustomUrls=https://maven.example.com/releases
mavenCustomUsernames=deploy-user
mavenCustomPasswords=deploy-password

# 发布
./gradlew publishToMavenCustom
```

---

## 故障排除

### 问题：插件未找到

**错误**：
```
Plugin [id: 'com.mojeter.ccgo.gradle.android.library', version: '1.0.0'] was not found
```

**解决方案**：
```kotlin
// settings.gradle.kts
pluginManagement {
    repositories {
        mavenCentral()  // 添加这个！
        google()
        gradlePluginPortal()
    }
}
```

---

### 问题：CCGO.toml 未找到

**警告**：
```
[CCGOConfig] WARNING: CCGO.toml not found. Using default values.
```

**解决方案**：
- 将 `CCGO.toml` 放在项目根目录（android/ 目录的父目录）
- 或在 Gradle 根项目上方最多 3 层

**示例结构**：
```
myproject/              ← CCGO.toml 在这里
├── CCGO.toml          ← 或在这里
├── android/           ← Gradle 根目录
│   ├── CCGO.toml     ← 或在这里
│   └── build.gradle.kts
```

---

### 问题：原生构建失败

**错误**：
```
Task :library:buildLibrariesForMain FAILED
ccgo: command not found
```

**解决方案**：
```bash
# 安装 ccgo CLI
pip install ccgo

# 验证安装
which ccgo
ccgo --version

# 或使用 Rust CLI
cargo install --path ccgo-rs
```

---

### 问题：签名失败

**错误**：
```
[Signing] ERROR: Key does not start with '-----BEGIN PGP PRIVATE KEY BLOCK-----'
```

**解决方案**：
- 确保 PGP 密钥具有正确的格式和换行符
- 转换转义的换行符：`"...\n..."` → 实际换行符
- 本地开发使用 GPG 代理（无需密钥）

**GPG 代理设置**（本地开发）：
```bash
# 生成 GPG 密钥
gpg --full-generate-key

# 列出密钥（验证）
gpg --list-secret-keys

# 插件将自动检测并使用 GPG 代理
./gradlew publishToMavenLocal  # 无需签名配置！
```

---

### 问题：NDK 未找到

**错误**：
```
NDK is not installed
```

**解决方案**：
```bash
# 选项 1：设置环境变量
export NDK_ROOT=/path/to/android-sdk/ndk/25.2.9519653

# 选项 2：在 local.properties 中配置
ndk.dir=/path/to/android-sdk/ndk/25.2.9519653

# 选项 3：让 Gradle 从 CCGO.toml 自动检测
[android]
ndk_version = "25.2.9519653"  # 如果缺失，Gradle 会下载
```

---

## 最佳实践

### 1. 使用 CCGO.toml 进行配置

**❌ 不要**：在 build.gradle.kts 中硬编码
```kotlin
android {
    compileSdk = 34  // 硬编码
    defaultConfig {
        minSdk = 21  // 硬编码
    }
}
```

**✅ 应该**：在 CCGO.toml 中集中管理
```toml
[android]
compile_sdk = 34
min_sdk = 21
```

```kotlin
android {
    // 自动从 CCGO.toml 配置
}
```

---

### 2. 版本插件目录

**✅ 应该**：使用版本目录
```toml
[versions]
ccgo-buildlogic = "1.0.0"

[plugins]
ccgo-android-library = { id = "com.mojeter.ccgo.gradle.android.library", version.ref = "ccgo-buildlogic" }
```

```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
}
```

---

### 3. 将发布与构建分离

**✅ 应该**：单独应用发布插件
```kotlin
// library/build.gradle.kts
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
}

// publishing/build.gradle.kts（单独模块）
plugins {
    alias(libs.plugins.ccgo.android.publish)
}
```

---

### 4. 对 CI/CD 使用环境变量

**✅ 应该**：对敏感凭据使用环境变量
```yaml
# .github/workflows/publish.yml
env:
  MAVEN_CENTRAL_USERNAME: ${{ secrets.MAVEN_CENTRAL_USERNAME }}
  MAVEN_CENTRAL_PASSWORD: ${{ secrets.MAVEN_CENTRAL_PASSWORD }}
  SIGNING_IN_MEMORY_KEY: ${{ secrets.SIGNING_KEY }}
  SIGNING_IN_MEMORY_KEY_PASSWORD: ${{ secrets.SIGNING_PASSWORD }}

steps:
  - run: ./gradlew publishToMavenCentral
```

---

### 5. 发布前先本地测试

**✅ 应该**：发布前验证
```bash
# 1. 发布到 Maven Local
./gradlew publishToMavenLocal

# 2. 在示例项目中测试
cd sample-project
./gradlew build  # 使用本地产物

# 3. 如果成功，发布到 Maven Central
cd ../
./gradlew publishToMavenCentral
```

---

## 迁移

### 从手动 Gradle 配置

**之前**（手动配置）：
```kotlin
plugins {
    id("com.android.library")
    kotlin("android")
}

android {
    compileSdk = 34
    defaultConfig {
        minSdk = 21
        targetSdk = 34

        externalNativeBuild {
            cmake {
                cppFlags += "-std=c++17"
                arguments += "-DANDROID_STL=c++_shared"
            }
        }
    }
}

publishing {
    publications {
        create<MavenPublication>("release") {
            // 手动 POM 配置...
        }
    }
}
```

**之后**（使用 CCGO 插件）：
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
    alias(libs.plugins.ccgo.android.publish)
}

android {
    namespace = "com.example.lib"
    // 所有设置来自 CCGO.toml！
}
```

---

### 从其他构建系统

#### 从 Conan + CMake

详细说明请参见[从 Conan 迁移指南](migration-from-conan.md)。

#### 从 vcpkg

使用 CCGO.toml 依赖替换 vcpkg 清单，并使用 CCGO Gradle 插件进行 Android 构建。

---

## 常见问题

### Q: 可以不使用 CCGO.toml 而使用插件吗？

**A**：可以，插件将使用默认值。但建议使用 CCGO.toml 进行集中配置。

---

### Q: 如何自定义 NDK/CMake 版本？

**A**：在 CCGO.toml 中配置：
```toml
[android]
ndk_version = "26.0.0"

[build]
cmake_version = "3.24.0"
```

---

### Q: 可以同时发布到多个仓库吗？

**A**：可以，使用逗号分隔的 URL：
```properties
mavenCustomUrls=https://repo1.example.com,https://repo2.example.com
mavenCustomUsernames=user1,user2
mavenCustomPasswords=pass1,pass2
```

```bash
./gradlew publishToMavenCustom  # 发布到所有仓库
```

---

### Q: 如何为 Maven Central 签名产物？

**A**：两个选项：

**选项 1**：内存 PGP 密钥（推荐用于 CI/CD）
```properties
signingInMemoryKey=-----BEGIN PGP PRIVATE KEY BLOCK-----\n...\n-----END PGP PRIVATE KEY BLOCK-----
signingInMemoryKeyPassword=your-password
```

**选项 2**：GPG 代理（推荐用于本地开发）
```bash
# 生成密钥一次
gpg --full-generate-key

# 插件自动检测并使用它
./gradlew publishToMavenCentral
```

---

### Q: 可以同时使用 Python 和 CMake 插件吗？

**A**：不可以，选择一个：
- **Python 插件**：使用 `ccgo build android` 进行构建（推荐）
- **CMake 插件**：直接 CMake 与 Android Gradle 插件集成

---

## 其他资源

- [CCGO CLI 参考](../reference/cli.md)
- [CCGO.toml 配置](../reference/config.zh.md)
- [CMake 集成指南](cmake-integration.zh.md)
- [发布指南](../guides/publishing.zh.md)
- [插件源代码](https://github.com/zhlinh/ccgo-gradle-plugins)

---

## 更新日志

### v1.0.0（2024-01-21）
- 初始发布，包含 20+ 个约定插件
- CCGO.toml 集成
- Maven Central 发布支持
- Android 和 KMP 支持

---

*本文档是 CCGO 项目的一部分。有关贡献指南，请参见 [CONTRIBUTING.md](https://github.com/zhlinh/ccgo/blob/main/CONTRIBUTING.md)。*
