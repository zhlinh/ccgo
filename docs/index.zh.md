# CCGO

[![PyPI](https://img.shields.io/pypi/v/ccgo.svg)](https://pypi.org/project/ccgo/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Documentation](https://readthedocs.org/projects/ccgo/badge/?version=latest)](https://ccgo.readthedocs.io/)

现代化的 C++ 跨平台构建系统和项目生成器，简化 Android、iOS、macOS、Windows、Linux、OpenHarmony 和 Kotlin 多平台的原生库构建。

## 功能特性

- **通用跨平台**: 从单一代码库构建 8+ 个平台
- **零配置**: 开箱即用，提供合理的默认配置
- **Docker 构建**: 在任何操作系统上构建任何平台，无需本地工具链
- **统一发布**: 一条命令发布到 Maven、CocoaPods、SPM、OHPM 和 Conan
- **智能依赖管理**: 支持 Git、路径和仓库依赖，带锁文件
- **模板驱动**: 使用现代 C++ 最佳实践生成新项目
- **Git 集成**: 自动版本管理和提交管理
- **CMake 集成**: 利用 CMake 的强大功能，简化配置

## 支持平台

| 平台 | 架构 | 输出格式 |
|------|------|---------|
| **Android** | armeabi-v7a, arm64-v8a, x86, x86_64 | AAR, 静态/动态库 |
| **iOS** | armv7, arm64, x86_64, arm64-simulator | Framework, XCFramework |
| **macOS** | x86_64, arm64 (Apple Silicon) | Framework, XCFramework |
| **Windows** | x86, x86_64 | DLL, 静态库 (MSVC/MinGW) |
| **Linux** | x86_64, aarch64 | 动态/静态库 |
| **OpenHarmony** | armeabi-v7a, arm64-v8a, x86_64 | HAR 包 |
| **watchOS** | armv7k, arm64_32, x86_64 | Framework, XCFramework |
| **tvOS** | arm64, x86_64 | Framework, XCFramework |

## 快速链接

- [安装](getting-started/installation.md) - 安装 CCGO
- [快速开始](getting-started/quickstart.md) - 5 分钟创建第一个项目
- [配置](getting-started/configuration.md) - 为项目配置 CCGO
- [平台支持](platforms/index.md) - 特定平台构建指南
- [CLI 参考](reference/cli.md) - 完整命令行参考

## 使用示例

```bash
# 创建新项目
ccgo new myproject

# 构建 Android
cd myproject/myproject
ccgo build android --arch arm64-v8a,armeabi-v7a

# 构建 iOS（需要 macOS）
ccgo build ios

# 使用 Docker 构建 Windows（在任何操作系统上）
ccgo build windows --docker

# 运行测试
ccgo test

# 发布到 Maven Local
ccgo publish android --registry local
```

## 为什么选择 CCGO？

1. **简单**: 一个工具、一个配置文件 (CCGO.toml)、所有平台
2. **快速**: 并行构建、增量编译、Docker 缓存
3. **灵活**: 支持基于 Python 和纯 CMake 的工作流
4. **现代**: 使用 Rust 构建，可靠且高性能
5. **通用**: Docker 构建支持任意平台交叉编译
6. **团队友好**: 锁文件确保团队成员间构建可复现

## 架构

CCGO 由四个主要组件组成：

- **ccgo**: Python/Rust CLI 工具，编排构建和管理项目
- **ccgo-template**: 基于 Copier 的模板，生成新 C++ 项目
- **ccgo-gradle-plugins**: Gradle 约定插件，用于 Android/KMP 构建
- **ccgo-now**: 示例项目，展示 CCGO 能力

## 社区

- [GitHub Issues](https://github.com/zhlinh/ccgo/issues) - 错误报告和功能请求
- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions) - 问题和社区支持
- [路线图](development/roadmap.md) - 查看即将推出的功能

## 许可证

CCGO 使用 MIT 许可证。详见 [LICENSE](https://github.com/zhlinh/ccgo/blob/main/LICENSE)。
