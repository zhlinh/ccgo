# 平台支持

CCGO 为 C++ 项目提供全面的跨平台构建支持。本节涵盖平台特定的指南、要求和最佳实践。

## 支持的平台

### 移动平台

| 平台 | 架构 | 输出格式 | 状态 |
|------|------|---------|------|
| [Android](android.md) | arm64-v8a, armeabi-v7a, x86, x86_64 | AAR, .so, .a | ✅ 稳定 |
| [iOS](ios.md) | armv7, arm64, simulator (x86_64, arm64) | Framework, XCFramework | ✅ 稳定 |
| [OpenHarmony](openharmony.md) | arm64-v8a, armeabi-v7a, x86_64 | HAR, .so, .a | ✅ 稳定 |

### 桌面平台

| 平台 | 架构 | 输出格式 | 状态 |
|------|------|---------|------|
| [macOS](macos.md) | x86_64, arm64 (Apple Silicon) | Framework, XCFramework, dylib | ✅ 稳定 |
| [Windows](windows.md) | x86, x86_64 | DLL, LIB (MSVC/MinGW) | ✅ 稳定 |
| [Linux](linux.md) | x86_64, aarch64 | .so, .a | ✅ 稳定 |

### 电视和可穿戴平台

| 平台 | 架构 | 输出格式 | 状态 |
|------|------|---------|------|
| watchOS | armv7k, arm64_32, simulator | Framework, XCFramework | ✅ 稳定 |
| tvOS | arm64, simulator (x86_64, arm64) | Framework, XCFramework | ✅ 稳定 |

### 多平台

| 平台 | 描述 | 状态 |
|------|------|------|
| Kotlin 多平台 | 带原生 C++ 的 KMP 库 | 🚧 即将推出 |

## 快速开始

### 基本构建

```bash
# 为当前平台构建
ccgo build

# 为特定平台构建
ccgo build android --arch arm64-v8a
ccgo build ios
ccgo build windows --toolchain msvc
```

### 基于 Docker 的构建

在任何主机操作系统上构建任何平台：

```bash
# 在 macOS/Windows 上构建 Linux 库
ccgo build linux --docker

# 在 Linux/macOS 上构建 Windows 库
ccgo build windows --docker

# 在 Linux/Windows 上构建 macOS/iOS 库（实验性）
ccgo build macos --docker
```

## 平台选择指南

### 移动应用

- **Android**：使用 AAR 轻松集成 Android Studio/Gradle
- **iOS**：使用 XCFramework 支持设备和模拟器
- **OpenHarmony**：使用 HAR 集成 DevEco Studio

### 桌面应用

- **Windows**：MSVC 用于 Visual Studio 项目，MinGW 用于 GCC 兼容性
- **macOS**：Framework 用于 Xcode 项目，dylib 用于通用用途
- **Linux**：共享库（.so）用于大多数应用

### 多平台

- **Kotlin 多平台**：跨 Android、iOS、macOS、Linux、Windows 的统一 API

## 构建选项

### 架构选择

```bash
# 单一架构
ccgo build android --arch arm64-v8a

# 多个架构
ccgo build android --arch arm64-v8a,armeabi-v7a,x86_64

# 所有架构（默认）
ccgo build android
```

### 链接类型

```bash
# 仅静态库
ccgo build --link-type static

# 仅共享库
ccgo build --link-type shared

# 两者都有（默认）
ccgo build --link-type both
```

### 工具链选择（Windows）

```bash
# MSVC（Windows 默认）
ccgo build windows --toolchain msvc

# MinGW
ccgo build windows --toolchain mingw

# 两者
ccgo build windows --toolchain auto
```

## 平台要求

### 开发先决条件

| 平台 | 要求 |
|------|------|
| Android | Android SDK/NDK 或 Docker |
| iOS | 带 Xcode 的 macOS 或 Docker（实验性）|
| macOS | 带 Xcode 的 macOS 或 Docker（实验性）|
| Windows | Visual Studio 或 MinGW 或 Docker |
| Linux | GCC/Clang 或 Docker |
| OpenHarmony | OpenHarmony SDK 或 Docker |
| watchOS/tvOS | 带 Xcode 的 macOS |

### Docker 要求

所有平台都可以使用 Docker 构建，无需本地工具链设置：

- 安装 [Docker Desktop](https://www.docker.com/products/docker-desktop)
- 运行 `ccgo build <platform> --docker`
- 首次构建下载预构建镜像（约 2-10 分钟）
- 后续构建使用缓存镜像（即时启动）

## 平台特定指南

- [Android 开发](android.md) - AAR 打包、JNI、Gradle 集成
- [iOS 开发](ios.md) - Framework/XCFramework、Swift 互操作
- [macOS 开发](macos.md) - 通用二进制、代码签名
- [Windows 开发](windows.md) - MSVC vs MinGW、DLL 导出
- [Linux 开发](linux.md) - 系统库、打包
- [OpenHarmony 开发](openharmony.md) - HAR 打包、ArkTS 互操作

## 常见任务

### 发布

```bash
# 发布到 Maven（Android/OpenHarmony）
ccgo publish android --registry official

# 发布到 CocoaPods（iOS/macOS）
ccgo publish apple --manager cocoapods

# 发布到 Swift Package Manager
ccgo publish apple --manager spm --push

# 发布到 Conan（所有平台）
ccgo publish conan --registry official
```

### IDE 项目

```bash
# 生成 Android Studio 项目
ccgo build android --ide-project

# 生成 Xcode 项目
ccgo build ios --ide-project

# 生成 Visual Studio 项目
ccgo build windows --ide-project --toolchain msvc
```

### 检查平台支持

```bash
# 检查是否满足平台要求
ccgo check android
ccgo check ios --verbose

# 检查所有平台
ccgo check --all
```

## 平台特定配置

每个平台都可以在 `CCGO.toml` 中配置：

```toml
[android]
min_sdk_version = 21
target_sdk_version = 33
ndk_version = "25.2.9519653"

[ios]
min_deployment_target = "12.0"
enable_bitcode = false

[windows]
msvc_runtime = "dynamic"  # 或 "static"
```

完整选项请参阅 [CCGO.toml 参考](../reference/ccgo-toml.md)。

## 故障排除

### 构建失败

1. 检查平台要求：`ccgo check <platform>`
2. 尝试 Docker 构建：`ccgo build <platform> --docker`
3. 启用详细日志：`ccgo build <platform> --verbose`

### Docker 问题

1. 确保 Docker 正在运行：`docker ps`
2. 清除 Docker 缓存：`docker system prune`
3. 重新拉取镜像：`docker pull ccgo-builder-<platform>`

### 平台特定问题

详细故障排除请参阅各个平台指南。

## 下一步

- 选择上述目标平台指南
- 查看[构建系统](../features/build-system.md)文档
- 探索[发布选项](../features/publishing.md)
- 查看[Docker 构建](../features/docker-builds.md)了解通用编译
