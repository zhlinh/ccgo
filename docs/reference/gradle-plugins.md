# Gradle Plugins Reference

Complete reference for CCGO Gradle plugins for Android and Kotlin Multiplatform projects.

## Overview

CCGO provides a suite of Gradle convention plugins published to Maven Central:

- **Group ID**: `com.mojeter.ccgo.gradle`
- **Plugin Prefix**: `com.mojeter.ccgo.gradle.android.*`
- **Purpose**: Standardize Android/KMP build configurations
- **Integration**: First-class support for native C++ builds

**Available Plugins:**
- `android.library` - Android library configuration
- `android.library.native.python` - Android library with Python-based native builds (ccgo)
- `android.library.native.cmake` - Android library with CMake-based native builds
- `android.application` - Android application configuration
- `android.application.native.python` - Android app with Python-based native builds
- `android.application.native.cmake` - Android app with CMake-based native builds
- `android.feature` - Android feature module configuration
- `android.publish` - Maven publishing configuration

## Installation

### Add Plugin Repository

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

### Apply Plugins

```kotlin
// build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library") version "1.0.0"
    id("com.mojeter.ccgo.gradle.android.publish") version "1.0.0"
}
```

## Android Library Plugins

### android.library

Base plugin for Android library modules:

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

**Applies:**
- Android Library Plugin
- Kotlin Android Plugin
- Standard Android configuration
- Version catalog integration
- Code quality tools (Lint)

### android.library.native.python

Android library with Python-based native builds using `ccgo build`:

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.python")
}

android {
    namespace = "com.example.mylib"
}

ccgoNative {
    // CCGO project root directory
    projectDir.set(file("../../"))

    // Target architectures
    architectures.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))

    // Build type (debug or release)
    buildType.set("release")

    // Custom CCGO options
    options.set(listOf("--verbose", "--jobs=4"))
}
```

**Configuration:**

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `projectDir` | `DirectoryProperty` | `project.projectDir` | CCGO project root |
| `architectures` | `ListProperty<String>` | `["armeabi-v7a", "arm64-v8a"]` | Target ABIs |
| `buildType` | `Property<String>` | `"release"` | Build type |
| `options` | `ListProperty<String>` | `[]` | Additional ccgo options |

**Tasks:**
- `buildCcgoNative` - Builds native libraries using ccgo
- `cleanCcgoNative` - Cleans native build artifacts

**Integration:**
```kotlin
// build.gradle.kts
tasks.named("preBuild") {
    dependsOn("buildCcgoNative")
}
```

### android.library.native.cmake

Android library with CMake-based native builds:

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
    // CMake version
    version.set("3.22.1")

    // CMake arguments
    arguments.set(listOf(
        "-DCMAKE_BUILD_TYPE=Release",
        "-DBUILD_SHARED_LIBS=ON"
    ))

    // C++ flags
    cppFlags.set(listOf("-std=c++17", "-Wall", "-Wextra"))

    // Target architectures
    abiFilters.set(listOf("armeabi-v7a", "arm64-v8a", "x86_64"))
}
```

**Configuration:**

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `version` | `Property<String>` | `"3.22.1"` | CMake version |
| `arguments` | `ListProperty<String>` | `[]` | CMake arguments |
| `cppFlags` | `ListProperty<String>` | `["-std=c++17"]` | C++ compiler flags |
| `abiFilters` | `ListProperty<String>` | `["armeabi-v7a", "arm64-v8a"]` | Target ABIs |

## Android Application Plugins

### android.application

Base plugin for Android application modules:

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

**Applies:**
- Android Application Plugin
- Kotlin Android Plugin
- Standard Android configuration
- ProGuard/R8 configuration
- Signing configuration support

### android.application.native.python

Android application with Python-based native builds:

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

Android application with CMake-based native builds:

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

## Android Feature Plugin

### android.feature

Plugin for Android dynamic feature modules:

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

**Applies:**
- Dynamic Feature Plugin
- Kotlin Android Plugin
- Feature-specific configuration
- Modularization support

## Publishing Plugin

### android.publish

Maven publishing configuration using [vanniktech/gradle-maven-publish-plugin](https://github.com/vanniktech/gradle-maven-publish-plugin):

```kotlin
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
    id("com.mojeter.ccgo.gradle.android.publish")
}

publishing {
    // Configured automatically for Maven Central
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

**Publishing Tasks:**
- `publishToMavenLocal` - Publish to local Maven repository
- `publishAllPublicationsToMavenCentral` - Publish to Maven Central

**Required Configuration:**

```properties
# gradle.properties
signing.keyId=<KEY_ID>
signing.password=<PASSWORD>
signing.secretKeyRingFile=<PATH_TO_KEY>

mavenCentralUsername=<SONATYPE_USERNAME>
mavenCentralPassword=<SONATYPE_PASSWORD>
```

## Multi-Module Projects

### Project Structure

```
my-app/
├── settings.gradle.kts
├── build.gradle.kts
├── gradle.properties
├── app/                           # Application module
│   └── build.gradle.kts
├── feature-auth/                  # Feature module
│   └── build.gradle.kts
└── library/                       # Library module
    ├── build.gradle.kts
    └── src/
        ├── main/
        │   ├── cpp/               # Native code
        │   └── java/
        └── androidTest/
```

### Root build.gradle.kts

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

### Library Module

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
    // Auto-configured for Maven
}
```

### Application Module

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

## Version Catalog Integration

### gradle/libs.versions.toml

```toml
[versions]
ccgo-gradle = "1.0.0"
android-gradle = "8.1.0"
kotlin = "1.9.20"

[libraries]
# Android dependencies defined here

[plugins]
ccgo-android-library = { id = "com.mojeter.ccgo.gradle.android.library", version.ref = "ccgo-gradle" }
ccgo-android-publish = { id = "com.mojeter.ccgo.gradle.android.publish", version.ref = "ccgo-gradle" }
android-library = { id = "com.android.library", version.ref = "android-gradle" }
kotlin-android = { id = "org.jetbrains.kotlin.android", version.ref = "kotlin" }
```

### Using Version Catalog

```kotlin
// build.gradle.kts
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.publish)
}
```

## Advanced Configuration

### Custom Native Build

```kotlin
// build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library.native.python")
}

ccgoNative {
    projectDir.set(file("../../native"))

    // Dynamic architecture selection based on build variant
    architectures.set(provider {
        when {
            project.gradle.startParameter.taskNames.any { "Release" in it } ->
                listOf("armeabi-v7a", "arm64-v8a", "x86_64")
            else ->
                listOf("arm64-v8a") // Debug builds use single architecture
        }
    })

    // Build type based on variant
    buildType.set(provider {
        if (gradle.taskGraph.hasTask("assembleRelease")) {
            "release"
        } else {
            "debug"
        }
    })

    // Custom ccgo options
    options.set(listOf(
        "--verbose",
        "--jobs=${Runtime.getRuntime().availableProcessors()}"
    ))
}

// Hook into Android build lifecycle
tasks.named("preBuild") {
    dependsOn("buildCcgoNative")
}

tasks.named("clean") {
    dependsOn("cleanCcgoNative")
}
```

### Conditional Plugin Application

```kotlin
// build.gradle.kts
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
}

// Apply native plugin conditionally
if (file("src/main/cpp").exists()) {
    apply(plugin = "com.mojeter.ccgo.gradle.android.library.native.cmake")
}

// Apply publishing conditionally
if (project.hasProperty("publish")) {
    apply(plugin = "com.mojeter.ccgo.gradle.android.publish")
}
```

### Shared Configuration

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

// Use in modules
plugins {
    id("com.mojeter.ccgo.gradle.android.library")
    id("shared-android-config")
}
```

## Troubleshooting

### Plugin Not Found

**Problem:**
```
Plugin [id: 'com.mojeter.ccgo.gradle.android.library', version: '1.0.0'] was not found
```

**Solution:**
```kotlin
// settings.gradle.kts
pluginManagement {
    repositories {
        mavenCentral()  // Ensure Maven Central is included
        google()
        gradlePluginPortal()
    }
}
```

### Native Build Fails

**Problem:**
```
Task ':library:buildCcgoNative' FAILED
```

**Solution:**
```bash
# Verify ccgo installation
ccgo --version

# Check native project configuration
cd <projectDir>
ccgo check android

# Build manually to see detailed errors
ccgo build android --verbose
```

### Version Mismatch

**Problem:**
```
The plugin com.mojeter.ccgo.gradle.android.library:1.0.0 requires AGP 8.0+
```

**Solution:**
```kotlin
// Update Android Gradle Plugin version
plugins {
    id("com.android.library") version "8.1.0"
    id("com.mojeter.ccgo.gradle.android.library") version "1.0.0"
}
```

## Best Practices

### 1. Use Convention Plugins

Create your own convention plugins for shared configuration:

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

// Use in modules
plugins {
    id("myproject.android-library")
}
```

### 2. Separate Native Builds

Keep native builds in separate modules:

```
project/
├── app/               # Pure Android app
├── native-lib/        # Native library with CCGO plugins
└── common/            # Shared Kotlin code
```

### 3. Cache Native Builds

```kotlin
ccgoNative {
    // Enable incremental builds
    options.set(listOf("--incremental"))
}
```

### 4. Parallel Builds

```kotlin
// gradle.properties
org.gradle.parallel=true
org.gradle.caching=true
org.gradle.configureondemand=true
```

## Examples

### Complete Library Module

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

## Resources

### Official Documentation

- [Android Gradle Plugin](https://developer.android.com/build)
- [Gradle Plugins Portal](https://plugins.gradle.org/)
- [Maven Publishing Plugin](https://github.com/vanniktech/gradle-maven-publish-plugin)

### CCGO Documentation

- [CLI Reference](cli.md)
- [CCGO.toml Reference](ccgo-toml.md)
- [Android Platform](../platforms/android.md)
- [Publishing Guide](../features/publishing.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [Android Development](../platforms/android.md)
- [Kotlin Multiplatform](../platforms/kmp.md)
- [Publishing Guide](../features/publishing.md)
- [CMake Integration](cmake.md)
