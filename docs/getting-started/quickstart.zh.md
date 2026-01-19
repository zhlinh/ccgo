# 快速开始

5 分钟开始使用 CCGO！本指南将引导您创建第一个跨平台 C++ 项目。

## 创建新项目

```bash
# 创建名为 "hello" 的新项目
ccgo new hello

# 进入项目目录
cd hello/hello
```

生成的项目结构：

```
hello/
└── hello/           # 主项目目录
    ├── CCGO.toml    # 项目配置
    ├── CMakeLists.txt
    ├── include/     # 公共头文件
    │   └── hello/
    │       └── hello.h
    ├── src/         # 源文件
    │   └── hello.cpp
    ├── tests/       # 单元测试
    │   └── test_hello.cpp
    ├── benches/     # 基准测试
    │   └── bench_hello.cpp
    └── examples/    # 示例程序
        └── example_hello.cpp
```

## 为您的平台构建

=== "本地构建"
    ```bash
    # 为当前平台构建
    ccgo build
    ```

=== "Android"
    ```bash
    # 为 Android 构建（多架构）
    ccgo build android --arch arm64-v8a,armeabi-v7a
    ```

=== "iOS"
    ```bash
    # 为 iOS 构建（需要 macOS）
    ccgo build ios
    ```

=== "Docker 构建"
    ```bash
    # 使用 Docker 为任何平台构建（在任何操作系统上工作）
    ccgo build linux --docker
    ccgo build windows --docker
    ccgo build macos --docker
    ```

## 运行测试

```bash
# 运行单元测试
ccgo test

# 运行基准测试
ccgo bench
```

## 添加依赖

编辑 `CCGO.toml`：

```toml
[dependencies]
# 从 Git 仓库
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 从本地路径
# mylib = { path = "../mylib" }

# 从注册表（即将推出）
# fmt = "10.1.1"
```

安装依赖：

```bash
ccgo install
```

在代码中使用依赖（`src/hello.cpp`）：

```cpp
#include <spdlog/spdlog.h>

void greet(const std::string& name) {
    spdlog::info("Hello, {}!", name);
}
```

## 发布您的库

=== "Maven Local"
    ```bash
    # 构建并发布到 Maven Local
    ccgo publish android --registry local
    ```

=== "CocoaPods"
    ```bash
    # 构建并发布到 CocoaPods
    ccgo publish apple --manager cocoapods
    ```

=== "Swift Package Manager"
    ```bash
    # 构建并发布到 SPM
    ccgo publish apple --manager spm --push
    ```

## 配置您的项目

编辑 `CCGO.toml` 自定义项目：

```toml
[package]
name = "hello"
version = "1.0.0"
description = "跨平台 C++ 库"
authors = ["您的名字 <you@example.com>"]
license = "MIT"

[library]
type = "both"  # "static", "shared" 或 "both"
namespace = "hello"

[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

[build]
cpp_standard = 17
cmake_minimum_version = "3.20"

[android]
min_sdk_version = 21
target_sdk_version = 33

[ios]
min_deployment_target = "12.0"
```

## 下一步

- [配置指南](configuration.md) - 了解所有 CCGO.toml 选项
- [平台支持](../platforms/index.md) - 特定平台构建指南
- [功能特性](../features/build-system.md) - 探索 CCGO 功能
- [CLI 参考](../reference/cli.md) - 完整命令参考

## 常用命令

```bash
# 项目创建
ccgo new <name>          # 创建新项目
ccgo init                # 在现有项目中初始化 CCGO

# 构建
ccgo build <platform>    # 为特定平台构建
ccgo build --docker      # 使用 Docker 构建
ccgo clean               # 清理构建产物

# 测试
ccgo test                # 运行测试
ccgo bench               # 运行基准测试

# 依赖管理
ccgo install             # 安装依赖
ccgo install --locked    # 使用锁文件中的精确版本
ccgo vendor              # 本地 vendor 依赖

# 发布
ccgo publish <platform> --registry <type>  # 发布库
ccgo tag                 # 创建版本标签

# 工具
ccgo check <platform>    # 检查平台需求
ccgo doc --open          # 生成并打开文档
```

## 故障排除

### 构建失败

```bash
# 检查平台要求
ccgo check android

# 如果本地工具链有问题，尝试 Docker 构建
ccgo build android --docker
```

### 依赖问题

```bash
# 删除锁文件并重新安装
rm CCGO.lock
ccgo install

# Vendor 依赖以进行离线构建
ccgo vendor
```

### 需要帮助？

- 查看[文档](https://ccgo.readthedocs.io)
- 浏览[示例](https://github.com/zhlinh/ccgo-now)
- 在 [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions) 提问
- 在 [GitHub Issues](https://github.com/zhlinh/ccgo/issues) 报告错误
