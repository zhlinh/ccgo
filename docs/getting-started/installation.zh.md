# 安装

## 前置条件

### 系统要求

- **Python 3.8+**（用于 Python 工作流）
- **Rust 1.75+**（用于 Rust CLI，可选）
- **CMake 3.20+**
- **Git**（用于依赖管理）

### 平台特定要求

=== "macOS"
    ```bash
    # 安装 Homebrew（如果尚未安装）
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # 安装依赖
    brew install cmake python@3.11 git

    # iOS/macOS 开发
    xcode-select --install
    ```

=== "Windows"
    ```powershell
    # 安装 Chocolatey（如果尚未安装）
    Set-ExecutionPolicy Bypass -Scope Process -Force
    iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))

    # 安装依赖
    choco install cmake python git

    # MSVC 构建（可选）
    # 安装 Visual Studio 2019 或更高版本，带 C++ 工作负载
    ```

=== "Linux"
    ```bash
    # Ubuntu/Debian
    sudo apt-get update
    sudo apt-get install cmake python3 python3-pip git build-essential

    # Fedora/RHEL
    sudo dnf install cmake python3 python3-pip git gcc-c++

    # Arch Linux
    sudo pacman -S cmake python python-pip git base-devel
    ```

## 安装 CCGO

### 通过 pip（推荐）

```bash
pip3 install ccgo
```

### 通过 Rust Cargo（最新开发版本）

```bash
cargo install --git https://github.com/zhlinh/ccgo --path ccgo
```

### 从源码安装

```bash
# 克隆仓库
git clone https://github.com/zhlinh/ccgo.git
cd ccgo

# 安装 Python 版本
cd ccgo
pip3 install -e .

# 或构建 Rust 版本
cd ../ccgo-rs
cargo build --release
cargo install --path .
```

## 验证安装

```bash
ccgo --version
```

预期输出：
```
ccgo 3.0.10
```

## 平台特定工具链

CCGO 支持两种平台构建方式：

### 1. 基于 Docker 的构建（推荐）

无需本地工具链！Docker 构建可在任何主机操作系统上运行：

```bash
# 安装 Docker Desktop
# macOS: https://docs.docker.com/desktop/mac/install/
# Windows: https://docs.docker.com/desktop/windows/install/
# Linux: https://docs.docker.com/engine/install/

# 验证 Docker 安装
docker --version
```

### 2. 本地工具链（可选）

对于不使用 Docker 的本地构建：

=== "Android"
    ```bash
    # 下载 Android SDK/NDK
    # 通过 Android Studio 或命令行工具

    # 设置环境变量
    export ANDROID_HOME=$HOME/Android/Sdk
    export ANDROID_NDK=$HOME/Android/Sdk/ndk/25.2.9519653
    ```

=== "iOS/macOS"
    ```bash
    # 从 App Store 安装 Xcode
    xcode-select --install

    # 接受 Xcode 许可
    sudo xcodebuild -license accept
    ```

=== "Windows"
    ```bash
    # 选项 1：Visual Studio (MSVC)
    # 安装 Visual Studio 2019+ 带 C++ 工作负载

    # 选项 2：MinGW（推荐使用 Docker）
    # Docker 构建无需本地安装
    ```

=== "Linux"
    ```bash
    # GCC/Clang 已通过 build-essential 安装
    # 无需额外设置
    ```

=== "OpenHarmony"
    ```bash
    # 下载 OpenHarmony SDK
    # https://developer.harmonyos.com/

    # 设置环境变量
    export OHOS_SDK_HOME=$HOME/openharmony/sdk
    ```

## 下一步

- [快速开始](quickstart.md) - 创建第一个项目
- [配置](configuration.md) - 配置 CCGO
- [Docker 构建](../features/docker-builds.md) - 了解通用交叉编译
