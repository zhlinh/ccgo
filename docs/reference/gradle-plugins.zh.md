# Gradle 插件参考

CCGO Gradle 插件完整参考，用于 Android 和 Kotlin 多平台项目。

## 概览

CCGO 提供一套发布到 Maven Central 的 Gradle 约定插件：

- **Group ID**: `com.mojeter.ccgo.gradle`
- **插件前缀**: `com.mojeter.ccgo.gradle.android.*`
- **目的**: 标准化 Android/KMP 构建配置
- **集成**: 对原生 C++ 构建的一流支持

**可用插件：**
- `android.library` - Android 库配置
- `android.library.native.python` - 带基于 Python 的原生构建的 Android 库（ccgo）
- `android.library.native.cmake` - 带基于 CMake 的原生构建的 Android 库
- `android.application` - Android 应用配置
- `android.application.native.python` - 带基于 Python 的原生构建的 Android 应用
- `android.application.native.cmake` - 带基于 CMake 的原生构建的 Android 应用
- `android.feature` - Android 功能模块配置
- `android.publish` - Maven 发布配置

## 安装

### 添加插件仓库

```kotlin
// settings.gradle.kts
pluginManagement {
    repositories {
        mavenCentral()
        google()
        gradlePluginPortal()
    }
}
```

### 应用插件

```kotlin
// build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library") version "1.0.0"
    id("com.mojeter.ccgo.gradle.android.publish") version "1.0.0"
}
```

## Android 库插件

### android.library

Android 库模块的基础插件：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
}

android {
    namespace = "com.example.mylib"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
        targetSdk = 33
    }
}
```

**应用：**
- Android Library 插件
- Kotlin Android 插件
- 标准 Android 配置
- 版本目录集成
- 代码质量工具（Lint）

### android.library.native.python

使用 `ccgo build` 进行基于 Python 的原生构建的 Android 库：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.python")
}

android {
    namespace = "com.example.mylib"
}

ccgoNative {
    // CCGO 项目根目录
    projectDir.set(file("../../"))

    // 目标架构
    architectures.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))

    // 构建类型（debug 或 release）
    buildType.set("release")

    // 自定义 CCGO 选项
    options.set(listOf("--verbose", "--jobs=4"))
}
```

**配置：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `projectDir` | `DirectoryProperty` | `project.projectDir` | CCGO 项目根目录 |
| `architectures` | `ListProperty<String>` | `["armeabi-v7a", "arm64-v8a"]` | 目标 ABI |
| `buildType` | `Property<String>` | `"release"` | 构建类型 |
| `options` | `ListProperty<String>` | `[]` | 额外的 ccgo 选项 |

**任务：**
- `buildCcgoNative` - 使用 ccgo 构建原生库
- `cleanCcgoNative` - 清理原生构建产物

**集成：**
```kotlin
// build.gradle.kts
tasks.named("preBuild") {
    dependsOn("buildCcgoNative")
}
```

### android.library.native.cmake

使用基于 CMake 的原生构建的 Android 库：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.cmake")
}

android {
    namespace = "com.example.mylib"

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    defaultConfig {
        externalNativeBuild {
            cmake {
                cppFlags += listOf("-std=c++17", "-Wall")
                arguments += listOf(
                    "-DANDROID_STL=c++_shared",
                    "-DANDROID_PLATFORM=android-21"
                )
            }
        }

        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a", "x86_64")
        }
    }
}

cmakeNative {
    // CMake 版本
    version.set("3.22.1")

    // CMake 参数
    arguments.set(listOf(
        "-DCMAKE_BUILD_TYPE=Release",
        "-DBUILD_SHARED_LIBS=ON"
    ))

    // C++ 标志
    cppFlags.set(listOf("-std=c++17", "-Wall", "-Wextra"))

    // 目标架构
    abiFilters.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))
}
```

**配置：**

| 属性 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `version` | `Property<String>` | `"3.22.1"` | CMake 版本 |
| `arguments` | `ListProperty<String>` | `[]` | CMake 参数 |
| `cppFlags` | `ListProperty<String>` | `["-std=c++17"]` | C++ 编译器标志 |
| `abiFilters` | `ListProperty<String>` | `["armeabi-v7a", "arm64-v8a"]` | 目标 ABI |

## Android 应用插件

### android.application

Android 应用模块的基础插件：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.application")
}

android {
    namespace = "com.example.myapp"
    compileSdk = 33

    defaultConfig {
        applicationId = "com.example.myapp"
        minSdk = 21
        targetSdk = 33
        versionCode = 1
        versionName = "1.0.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
}
```

**应用：**
- Android Application 插件
- Kotlin Android 插件
- 标准 Android 配置
- ProGuard/R8 配置
- 签名配置支持

### android.application.native.python

带基于 Python 的原生构建的 Android 应用：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.application.native.python")
}

android {
    namespace = "com.example.myapp"
}

ccgoNative {
    projectDir.set(file("../../native"))
    architectures.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))
    buildType.set(provider {
        if (gradle.taskGraph.hasTask(":app:assembleRelease")) {
            "release"
        } else {
            "debug"
        }
    })
}
```

### android.application.native.cmake

带基于 CMake 的原生构建的 Android 应用：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.application.native.cmake")
}

android {
    namespace = "com.example.myapp"

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
        }
    }
}

cmakeNative {
    version.set("3.22.1")
    abiFilters.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))
}
```

## Android 功能插件

### android.feature

Android 动态功能模块插件：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.feature")
}

android {
    namespace = "com.example.myapp.feature"
}

dependencies {
    implementation(project(":app"))
}
```

**应用：**
- Dynamic Feature 插件
- Kotlin Android 插件
- 功能特定配置
- 模块化支持

## 发布插件

### android.publish

使用 [vanniktech/gradle-maven-publish-plugin](https://github.com/vanniktech/gradle-maven-publish-plugin) 的 Maven 发布配置：

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
    id("com.mojeter.ccgo.gradle.android.publish")
}

publishing {
    // 自动为 Maven Central 配置
}

mavenPublishing {
    coordinates(
        groupId = "com.example",
        artifactId = "mylib",
        version = "1.0.0"
    )

    pom {
        name.set("My Library")
        description.set("A cross-platform C++ library")
        url.set("https://github.com/example/mylib")

        licenses {
            license {
                name.set("The Apache License, Version 2.0")
                url.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
            }
        }

        developers {
            developer {
                id.set("username")
                name.set("Your Name")
                email.set("you@example.com")
            }
        }

        scm {
            connection.set("scm:git:git://github.com/example/mylib.git")
            developerConnection.set("scm:git:ssh://github.com/example/mylib.git")
            url.set("https://github.com/example/mylib")
        }
    }
}
```

**发布任务：**
- `publishToMavenLocal` - 发布到本地 Maven 仓库
- `publishAllPublicationsToMavenCentral` - 发布到 Maven Central

**必需配置：**

```properties
# gradle.properties
signing.keyId=<KEY_ID>
signing.password=<PASSWORD>
signing.secretKeyRingFile=<PATH_TO_KEY>

mavenCentralUsername=<SONATYPE_USERNAME>
mavenCentralPassword=<SONATYPE_PASSWORD>
```

## 多模块项目

### 项目结构

```
my-app/
├── settings.gradle.kts
├── build.gradle.kts
├── gradle.properties
├── app/                           # 应用模块
│   └── build.gradle.kts
├── feature-auth/                  # 功能模块
│   └── build.gradle.kts
└── library/                       # 库模块
    ├── build.gradle.kts
    └── src/
        ├── main/
        │   ├── cpp/               # 原生代码
        │   └── java/
        └── androidTest/
```

### 根 build.gradle.kts

```kotlin
plugins {
    id("com.android.application") version "8.1.0" apply false
    id("com.android.library") version "8.1.0" apply false
    id("org.jetbrains.kotlin.android") version "1.9.20" apply false
    id("com.mojeter.ccgo.gradle.android.library") version "1.0.0" apply false
}

tasks.register<Delete>("clean") {
    delete(rootProject.buildDir)
}
```

### 库模块

```kotlin
// library/build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.python")
    id("com.mojeter.ccgo.gradle.android.publish")
}

android {
    namespace = "com.example.library"
}

ccgoNative {
    projectDir.set(file("../native"))
    architectures.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))
}

publishing {
    // 自动为 Maven 配置
}
```

### 应用模块

```kotlin
// app/build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.application")
}

android {
    namespace = "com.example.myapp"

    defaultConfig {
        applicationId = "com.example.myapp"
    }
}

dependencies {
    implementation(project(":library"))
}
```

## 版本目录集成

### gradle/libs.versions.toml

```toml
[versions]
ccgo-gradle = "1.0.0"
android-gradle = "8.1.0"
kotlin = "1.9.20"

[libraries]
# 在此定义 Android 依赖

[plugins]
ccgo-android-library = { id = "com.mojeter.ccgo.gradle.android.library", version.ref = "ccgo-gradle" }
ccgo-android-publish = { id = "com.mojeter.ccgo.gradle.android.publish", version.ref = "ccgo-gradle" }
android-library = { id = "com.android.library", version.ref = "android-gradle" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
```

### 使用版本目录

```kotlin
// build.gradle.kts
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.publish)
}
```

## 高级配置

### 自定义原生构建

```kotlin
// build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.python")
}

ccgoNative {
    projectDir.set(file("../../native"))

    // 基于构建变体的动态架构选择
    architectures.set(provider {
        when {
            project.gradle.startParameter.taskNames.any { "Release" in it } ->
                listOf("armeabi-v7a", "arm64-v8a", "x86_64")
            else ->
                listOf("arm64-v8a") // Debug 构建使用单个架构
        }
    })

    // 基于变体的构建类型
    buildType.set(provider {
        if (gradle.taskGraph.hasTask("assembleRelease")) {
            "release"
        } else {
            "debug"
        }
    })

    // 自定义 ccgo 选项
    options.set(listOf(
        "--verbose",
        "--jobs=${Runtime.getRuntime().availableProcessors()}"
    ))
}

// 挂钩到 Android 构建生命周期
tasks.named("preBuild") {
    dependsOn("buildCcgoNative")
}

tasks.named("clean") {
    dependsOn("cleanCcgoNative")
}
```

### 条件插件应用

```kotlin
// build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
}

// 有条件地应用原生插件
if (file("src/main/cpp").exists()) {
    apply(plugin = "com.mojeter.ccgo.gradle.android.library.native.cmake")
}

// 有条件地应用发布插件
if (project.hasProperty("publish")) {
    apply(plugin = "com.mojeter.ccgo.gradle.android.publish")
}
```

### 共享配置

```kotlin
// buildSrc/src/main/kotlin/shared-android-config.gradle.kts
import com.android.build.gradle.LibraryExtension

configure<LibraryExtension> {
    compileSdk = 33

    defaultConfig {
        minSdk = 21
        targetSdk = 33

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

// 在模块中使用
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
    id("shared-android-config")
}
```

## 故障排除

### 找不到插件

**问题：**
```
Plugin [id: 'com.mojeter.ccgo.gradle.android.library', version: '1.0.0'] was not found
```

**解决方案：**
```kotlin
// settings.gradle.kts
pluginManagement {
    repositories {
        mavenCentral()  // 确保包含 Maven Central
        google()
        gradlePluginPortal()
    }
}
```

### 原生构建失败

**问题：**
```
Task ':library:buildCcgoNative' FAILED
```

**解决方案：**
```bash
# 验证 ccgo 安装
ccgo --version

# 检查原生项目配置
cd <projectDir>
ccgo check android

# 手动构建查看详细错误
ccgo build android --verbose
```

### 版本不匹配

**问题：**
```
The plugin com.mojeter.ccgo.gradle.android.library:1.0.0 requires AGP 8.0+
```

**解决方案：**
```kotlin
// 更新 Android Gradle Plugin 版本
plugins {
    id("com.android.library") version "8.1.0"
    id("com.mojeter.ccgo.gradle.android.library") version "1.0.0"
}
```

## 最佳实践

### 1. 使用约定插件

为共享配置创建自己的约定插件：

```kotlin
// buildSrc/src/main/kotlin/myproject.android-library.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
}

android {
    compileSdk = 33
    defaultConfig {
        minSdk = 21
    }
}

// 在模块中使用
plugins {
    id("myproject.android-library")
}
```

### 2. 分离原生构建

将原生构建保持在单独的模块中：

```
project/
├── app/               # 纯 Android 应用
├── native-lib/        # 带 CCGO 插件的原生库
└── common/            # 共享 Kotlin 代码
```

### 3. 缓存原生构建

```kotlin
ccgoNative {
    // 启用增量构建
    options.set(listOf("--incremental"))
}
```

### 4. 并行构建

```kotlin
// gradle.properties
org.gradle.parallel=true
org.gradle.caching=true
org.gradle.configureondemand=true
```

## 示例

### 完整库模块

```kotlin
// library/build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.python")
    id("com.mojeter.ccgo.gradle.android.publish")
}

android {
    namespace = "com.example.mylib"
    compileSdk = 33

    defaultConfig {
        minSdk = 21
        targetSdk = 33
    }
}

ccgoNative {
    projectDir.set(file("../native"))
    architectures.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))
    buildType.set("release")
}

mavenPublishing {
    coordinates("com.example", "mylib", "1.0.0")
    pom {
        name.set("My Library")
        description.set("A cross-platform C++ library")
        url.set("https://github.com/example/mylib")
    }
}
```

## 资源

### 官方文档

- [Android Gradle Plugin](https://developer.android.com/build)
- [Gradle Plugins Portal](https://plugins.gradle.org/)
- [Maven Publishing Plugin](https://github.com/vanniktech/gradle-maven-publish-plugin)

### CCGO 文档

- [CLI 参考](cli.zh.md)
- [CCGO.toml 参考](ccgo-toml.zh.md)
- [Android 平台](../platforms/android.zh.md)
- [发布指南](../features/publishing.zh.md)

### 社区

- [GitHub 讨论](https://github.com/zhlinh/ccgo/discussions)
- [问题追踪](https://github.com/zhlinh/ccgo/issues)

## 下一步

- [Android 开发](../platforms/android.zh.md)
- [Kotlin 多平台](../platforms/kmp.zh.md)
- [发布指南](../features/publishing.zh.md)
- [CMake 集成](cmake.zh.md)
