# CCGO Gradle Plugins Reference

> Version: v3.0.10 | Updated: 2026-01-21

## Overview

CCGO Gradle plugins provide convention-based configurations for Android and Kotlin Multiplatform projects that use CCGO for native C++ library development. These plugins standardize build configurations, native library integration, and publishing workflows across CCGO projects.

**Published to Maven Central**: `com.mojeter.ccgo.gradle`

## Key Features

- **Convention-based Configuration**: Standardized build settings based on CCGO best practices
- **CCGO.toml Integration**: Automatically reads project configuration from CCGO.toml
- **Native Build Support**: Python-based (ccgo CLI) and CMake-based native builds
- **Publishing Ready**: Pre-configured Maven Central and custom repository publishing
- **Android & KMP**: Full support for Android libraries and Kotlin Multiplatform
- **Type-Safe**: Kotlin DSL-based plugins with compile-time validation

---

## Available Plugins

### Android Library Plugins

#### `com.mojeter.ccgo.gradle.android.library`
Basic Android library configuration with Kotlin support.

**Applies**:
- `com.android.library`
- `org.jetbrains.kotlin.android`
- `com.mojeter.ccgo.gradle.android.lint`

**Configures**:
- Kotlin compilation (JVM target, source compatibility)
- Product flavors (from CCGO.toml)
- Gradle managed devices for testing
- APK archiving tasks

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
}
```

#### `com.mojeter.ccgo.gradle.android.library.compose`
Android library with Jetpack Compose support.

**Extends**: `android.library`

**Additional Configuration**:
- Compose compiler settings
- Compose dependencies

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library.compose)
}
```

#### `com.mojeter.ccgo.gradle.android.feature`
Feature module configuration for modular Android apps.

**Extends**: `android.library`

**Additional Configuration**:
- Feature module dependencies
- Navigation component setup
- Hilt integration for dependency injection

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.feature)
}
```

---

### Native Build Plugins

#### `com.mojeter.ccgo.gradle.android.library.native.python`
Integrates native C++ libraries built with `ccgo` CLI (Python-based build orchestrator).

**Extends**: `android.library`

**Build Process**:
1. Reads CCGO.toml for project configuration
2. Creates `buildLibrariesForMain` task
3. Executes `ccgo build android --arch <archs> --native-only`
4. Integrates native libraries (.so files) into AAR

**Gradle Task Flow**:
```
cleanTheTargetDir → buildLibrariesForMain → mergeProdReleaseJniLibFolders → assembleProdRelease
```

**Environment Variables**:
- `ANDROID_HOME`: Android SDK path
- `NDK_ROOT`: Android NDK path
- `CMAKE_HOME`: CMake installation path

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
}
```

**Build Command**:
```bash
./gradlew assembleProdRelease
```

#### `com.mojeter.ccgo.gradle.android.library.native.cmake`
Direct CMake integration for native builds without ccgo CLI.

**Extends**: `android.library`

**Configuration**:
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

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.cmake)
}
```

#### `com.mojeter.ccgo.gradle.android.library.native.empty`
Android library without native code (pure Kotlin/Java wrapper).

**Use Cases**:
- Kotlin wrapper around pre-built native libraries
- Pure Kotlin/Java SDK with JNI bindings
- Testing and debugging without native compilation

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.empty)
}
```

---

### Android Application Plugins

#### `com.mojeter.ccgo.gradle.android.application`
Android application configuration.

**Applies**:
- `com.android.application`
- `org.jetbrains.kotlin.android`
- `com.mojeter.ccgo.gradle.android.lint`

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.application)
}
```

#### `com.mojeter.ccgo.gradle.android.application.compose`
Android application with Jetpack Compose UI.

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.application.compose)
}
```

#### `com.mojeter.ccgo.gradle.android.application.flavors`
Product flavors configuration (dev, staging, prod).

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.application.flavors)
}
```

---

### Code Quality & Testing Plugins

#### `com.mojeter.ccgo.gradle.android.lint`
Android Lint configuration with CCGO best practices.

**Configuration**:
- Warning severity levels
- Baseline file support
- HTML and XML reporting

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.lint)
}
```

#### `com.mojeter.ccgo.gradle.android.library.jacoco`
Code coverage with Jacoco for Android libraries.

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library.jacoco)
}

// Generate coverage report
./gradlew jacocoTestReport
```

#### `com.mojeter.ccgo.gradle.android.application.jacoco`
Code coverage for Android applications.

---

### Dependency Injection & Database Plugins

#### `com.mojeter.ccgo.gradle.android.hilt`
Hilt dependency injection setup.

**Applies**:
- `com.google.dagger.hilt.android`
- KSP for annotation processing

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.hilt)
}
```

#### `com.mojeter.ccgo.gradle.android.room`
Room database configuration with KSP.

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.room)
}
```

---

### Publishing Plugins

#### `com.mojeter.ccgo.gradle.android.publish`
Maven publishing configuration for Android libraries.

**Applies**:
- `com.vanniktech.maven.publish`
- `signing`

**Features**:
- Automatic CCGO.toml integration
- Sources and Javadoc JAR generation
- POM generation with dependencies
- Signing configuration (GPG or in-memory key)
- Maven Central, Local, and Custom repository support

**Publishing Commands**:
```bash
# Publish to Maven Local (~/.m2/repository)
./gradlew publishToMavenLocal

# Publish to Maven Central
./gradlew publishToMavenCentral

# Publish to custom repository
./gradlew publishToMavenCustom
```

**Configuration Sources** (priority order):
1. Environment variables (CI/CD override)
2. CCGO.toml (project root)
3. gradle.properties (project-level)
4. ~/.gradle/gradle.properties (user-level)

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.publish)
}
```

#### `com.mojeter.ccgo.gradle.kmp.publish`
Publishing for Kotlin Multiplatform libraries.

**Supports**:
- Android, iOS, JVM targets
- Automatic dependency configuration
- Multi-platform artifact publishing

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.kmp.publish)
}
```

---

### Kotlin Multiplatform Plugins

#### `com.mojeter.ccgo.gradle.kmp.library.native.python`
KMP library with CCGO-built native code.

**Targets**:
- Android (ARM, ARM64, x86_64)
- iOS (ARM64, Simulator x86_64/ARM64)
- JVM

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.kmp.library.native.python)
}
```

#### `com.mojeter.ccgo.gradle.kmp.library.native.empty`
KMP library without native code.

---

### Root Project Plugins

#### `com.mojeter.ccgo.gradle.android.root`
Root project configuration for multi-module Android projects.

**Configures**:
- Build cache settings
- Common repositories
- Gradle version catalogs

**Usage** (root build.gradle.kts):
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.root) apply false
}
```

#### `com.mojeter.ccgo.gradle.kmp.root`
Root configuration for KMP projects.

---

### JVM Plugin

#### `com.mojeter.ccgo.gradle.jvm.library`
Pure JVM/Kotlin library (no Android).

**Applies**:
- `org.jetbrains.kotlin.jvm`
- Java library conventions

**Usage**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.jvm.library)
}
```

---

## CCGO.toml Integration

All plugins automatically read configuration from `CCGO.toml` in the project root (or parent directory).

### Example CCGO.toml

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

### Configuration Fields Read by Plugins

| Field | Plugin | Purpose |
|-------|--------|---------|
| `project.name` | All | Project name |
| `project.version` | Publishing | Version number |
| `project.repository` | Publishing | Git repository URL |
| `android.compile_sdk` | Android | Compile SDK version |
| `android.min_sdk` | Android | Minimum SDK version |
| `android.target_sdk` | Android | Target SDK version |
| `android.ndk_version` | Native | NDK version |
| `android.stl` | Native | STL type (c++_shared/c++_static) |
| `android.default_archs` | Native | Build architectures |
| `build.cmake_version` | Native | CMake version |
| `publish.maven.group_id` | Publishing | Maven group ID |
| `publish.maven.artifact_id` | Publishing | Maven artifact ID |
| `publish.maven.dependencies` | Publishing | POM dependencies |
| `publish.kmp.*` | KMP | KMP-specific settings |

---

## Setup and Installation

### 1. Configure Plugin Repository

In `settings.gradle.kts`:

```kotlin
pluginManagement {
    repositories {
        mavenCentral()  // For published releases
        mavenLocal()    // For local development
        google()
        gradlePluginPortal()
    }
}
```

### 2. Declare Plugin Versions

In `gradle/libs.versions.toml`:

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

### 3. Apply Plugins

**Root build.gradle.kts**:
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.root) apply false
    alias(libs.plugins.ccgo.android.library) apply false
    alias(libs.plugins.ccgo.android.library.native.python) apply false
    alias(libs.plugins.ccgo.android.publish) apply false
}
```

**Module build.gradle.kts**:
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

## Complete Examples

### Example 1: Android Library with Native Code (Python Build)

**Project Structure**:
```
myproject/
├── CCGO.toml                    # Project configuration
├── CMakeLists.txt               # Native build (used by ccgo CLI)
├── src/                         # C++ source code
├── android/                     # Android Gradle project
│   ├── settings.gradle.kts
│   ├── build.gradle.kts
│   ├── gradle/
│   │   └── libs.versions.toml
│   └── library/
│       ├── build.gradle.kts
│       └── src/main/
│           ├── AndroidManifest.xml
│           └── kotlin/...       # Kotlin wrapper code
```

**android/library/build.gradle.kts**:
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

**Build Commands**:
```bash
# Build AAR with native libraries
cd android
./gradlew assembleProdRelease

# Publish to Maven Local
./gradlew publishToMavenLocal

# Publish to Maven Central
./gradlew publishToMavenCentral
```

---

### Example 2: Android Library with CMake (No CCGO CLI)

**android/library/build.gradle.kts**:
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

### Example 3: Kotlin Multiplatform with Native Code

**kmp/build.gradle.kts**:
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

### Example 4: Publishing Configuration

**Configure Credentials** in `~/.gradle/gradle.properties`:

```properties
# Maven Central (get from https://central.sonatype.com/account)
mavenCentralUsername=your-user-token-username
mavenCentralPassword=your-user-token-password

# Signing (in-memory PGP key)
signingInMemoryKey=-----BEGIN PGP PRIVATE KEY BLOCK-----\n...\n-----END PGP PRIVATE KEY BLOCK-----
signingInMemoryKeyPassword=your-key-password

# Custom Maven repository (optional)
mavenCustomUrls=https://maven.example.com/releases
mavenCustomUsernames=your-username
mavenCustomPasswords=your-password

# Local Maven path (optional, defaults to ~/.m2/repository)
mavenLocalPath=~/custom-maven-local
```

**Or use Environment Variables** (for CI/CD):

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

## Configuration Priority

Configuration values are resolved in the following priority order (highest to lowest):

1. **Environment Variables** - CI/CD override
2. **CCGO.toml** - Project-specific settings
3. **gradle.properties (project-level)** - Project defaults
4. **gradle.properties (user-level ~/.gradle/)** - User defaults
5. **Plugin defaults** - Fallback values

**Example Resolution**:

```
NDK Version Resolution:
1. Check CCGO.toml: [android].ndk_version = "25.2.9519653" ✓ (used)
2. Check gradle.properties: ndkVersion=26.0.0 (ignored)
3. Plugin default: "25.2.9519653" (not reached)

Result: "25.2.9519653"
```

---

## Publishing Workflows

### Local Development

```bash
# 1. Build and test locally
./gradlew assembleProdRelease
./gradlew test

# 2. Publish to Maven Local
./gradlew publishToMavenLocal

# 3. Use in another project
// settings.gradle.kts
repositories {
    mavenLocal()
}

// build.gradle.kts
dependencies {
    implementation("com.mojeter.ccgo:myproject:1.0.0")
}
```

### Maven Central Release

```bash
# 1. Update version in CCGO.toml
[project]
version = "1.0.0"

# 2. Build and test
./gradlew clean assembleProdRelease
./gradlew test

# 3. Publish to Maven Central
./gradlew publishToMavenCentral

# 4. Monitor release at https://central.sonatype.com
```

### Custom Repository

```bash
# Configure in gradle.properties or env vars
mavenCustomUrls=https://maven.example.com/releases
mavenCustomUsernames=deploy-user
mavenCustomPasswords=deploy-password

# Publish
./gradlew publishToMavenCustom
```

---

## Troubleshooting

### Issue: Plugin Not Found

**Error**:
```
Plugin [id: 'com.mojeter.ccgo.gradle.android.library', version: '1.0.0'] was not found
```

**Solution**:
```kotlin
// settings.gradle.kts
pluginManagement {
    repositories {
        mavenCentral()  // Add this!
        google()
        gradlePluginPortal()
    }
}
```

---

### Issue: CCGO.toml Not Found

**Warning**:
```
[CCGOConfig] WARNING: CCGO.toml not found. Using default values.
```

**Solution**:
- Place `CCGO.toml` in project root (parent of android/ directory)
- Or up to 3 levels above the Gradle root project

**Example Structure**:
```
myproject/              ← CCGO.toml here
├── CCGO.toml          ← Or here
├── android/           ← Gradle root
│   ├── CCGO.toml     ← Or here
│   └── build.gradle.kts
```

---

### Issue: Native Build Fails

**Error**:
```
Task :library:buildLibrariesForMain FAILED
ccgo: command not found
```

**Solution**:
```bash
# Install ccgo CLI
pip install ccgo

# Verify installation
which ccgo
ccgo --version

# Or use Rust CLI
cargo install --path ccgo-rs
```

---

### Issue: Signing Fails

**Error**:
```
[Signing] ERROR: Key does not start with '-----BEGIN PGP PRIVATE KEY BLOCK-----'
```

**Solution**:
- Ensure PGP key has proper format with line breaks
- Convert escaped newlines: `"...\n..."` → actual newlines
- Use GPG agent for local development (no key needed)

**GPG Agent Setup** (local development):
```bash
# Generate GPG key
gpg --full-generate-key

# List keys (verify)
gpg --list-secret-keys

# Plugin will auto-detect and use GPG agent
./gradlew publishToMavenLocal  # No signing config needed!
```

---

### Issue: NDK Not Found

**Error**:
```
NDK is not installed
```

**Solution**:
```bash
# Option 1: Set environment variable
export NDK_ROOT=/path/to/android-sdk/ndk/25.2.9519653

# Option 2: Configure in local.properties
ndk.dir=/path/to/android-sdk/ndk/25.2.9519653

# Option 3: Let Gradle auto-detect from CCGO.toml
[android]
ndk_version = "25.2.9519653"  # Gradle downloads if missing
```

---

## Best Practices

### 1. Use CCGO.toml for Configuration

**❌ Don't**: Hardcode in build.gradle.kts
```kotlin
android {
    compileSdk = 34  // Hardcoded
    defaultConfig {
        minSdk = 21  // Hardcoded
    }
}
```

**✅ Do**: Centralize in CCGO.toml
```toml
[android]
compile_sdk = 34
min_sdk = 21
```

```kotlin
android {
    // Automatically configured from CCGO.toml
}
```

---

### 2. Version Plugin Catalog

**✅ Do**: Use version catalog
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

### 3. Separate Publishing from Build

**✅ Do**: Apply publish plugin separately
```kotlin
// library/build.gradle.kts
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
}

// publishing/build.gradle.kts (separate module)
plugins {
    alias(libs.plugins.ccgo.android.publish)
}
```

---

### 4. Use Environment Variables for CI/CD

**✅ Do**: Use env vars for sensitive credentials
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

### 5. Test Locally Before Publishing

**✅ Do**: Validate before releasing
```bash
# 1. Publish to Maven Local
./gradlew publishToMavenLocal

# 2. Test in sample project
cd sample-project
./gradlew build  # Uses local artifact

# 3. If successful, publish to Maven Central
cd ../
./gradlew publishToMavenCentral
```

---

## Migration

### From Manual Gradle Configuration

**Before** (manual configuration):
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
            // Manual POM configuration...
        }
    }
}
```

**After** (with CCGO plugins):
```kotlin
plugins {
    alias(libs.plugins.ccgo.android.library)
    alias(libs.plugins.ccgo.android.library.native.python)
    alias(libs.plugins.ccgo.android.publish)
}

android {
    namespace = "com.example.lib"
    // All settings from CCGO.toml!
}
```

---

### From Other Build Systems

#### From Conan + CMake

See [Migration from Conan Guide](migration-from-conan.md) for detailed instructions.

#### From vcpkg

Replace vcpkg manifest with CCGO.toml dependencies and use CCGO Gradle plugins for Android builds.

---

## FAQ

### Q: Can I use plugins without CCGO.toml?

**A**: Yes, plugins will use default values. However, CCGO.toml is recommended for centralized configuration.

---

### Q: How do I customize NDK/CMake versions?

**A**: Configure in CCGO.toml:
```toml
[android]
ndk_version = "26.0.0"

[build]
cmake_version = "3.24.0"
```

---

### Q: Can I publish to multiple repositories at once?

**A**: Yes, use comma-separated URLs:
```properties
mavenCustomUrls=https://repo1.example.com,https://repo2.example.com
mavenCustomUsernames=user1,user2
mavenCustomPasswords=pass1,pass2
```

```bash
./gradlew publishToMavenCustom  # Publishes to all
```

---

### Q: How do I sign artifacts for Maven Central?

**A**: Two options:

**Option 1**: In-memory PGP key (recommended for CI/CD)
```properties
signingInMemoryKey=-----BEGIN PGP PRIVATE KEY BLOCK-----\n...\n-----END PGP PRIVATE KEY BLOCK-----
signingInMemoryKeyPassword=your-password
```

**Option 2**: GPG agent (recommended for local development)
```bash
# Generate key once
gpg --full-generate-key

# Plugin auto-detects and uses it
./gradlew publishToMavenCentral
```

---

### Q: Can I use Python and CMake plugins together?

**A**: No, choose one:
- **Python plugin**: Use `ccgo build android` for builds (recommended)
- **CMake plugin**: Direct CMake integration with Android Gradle Plugin

---

## Additional Resources

- [CCGO CLI Reference](../reference/cli.md)
- [CCGO.toml Configuration](../reference/config.md)
- [CMake Integration Guide](cmake-integration.md)
- [Publishing Guide](../guides/publishing.md)
- [Plugin Source Code](https://github.com/zhlinh/ccgo-gradle-plugins)

---

## Changelog

### v1.0.0 (2024-01-21)
- Initial release with 20+ convention plugins
- CCGO.toml integration
- Maven Central publishing support
- Android and KMP support

---

*This document is part of the CCGO project. For contribution guidelines, see [CONTRIBUTING.md](https://github.com/zhlinh/ccgo/blob/main/CONTRIBUTING.md).*
