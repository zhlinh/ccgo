# Installation

## Prerequisites

### System Requirements

- **Python 3.8+** (for Python-based workflow)
- **Rust 1.75+** (for Rust CLI, optional)
- **CMake 3.20+**
- **Git** (for dependency management)

### Platform-Specific Requirements

=== "macOS"
    ```bash
    # Install Homebrew if not already installed
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Install dependencies
    brew install cmake python@3.11 git

    # For iOS/macOS development
    xcode-select --install
    ```

=== "Windows"
    ```powershell
    # Install Chocolatey if not already installed
    Set-ExecutionPolicy Bypass -Scope Process -Force
    iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))

    # Install dependencies
    choco install cmake python git

    # For MSVC builds (optional)
    # Install Visual Studio 2019 or later with C++ workload
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

## Install CCGO

### Via pip (Recommended)

```bash
pip3 install ccgo
```

### Via Rust Cargo (Latest Development Version)

```bash
cargo install --git https://github.com/zhlinh/ccgo --path ccgo
```

### From Source

```bash
# Clone the repository
git clone https://github.com/zhlinh/ccgo.git
cd ccgo

# Install Python version
cd ccgo
pip3 install -e .

# Or build Rust version
cd ../ccgo-rs
cargo build --release
cargo install --path .
```

## Verify Installation

```bash
ccgo --version
```

Expected output:
```
ccgo 3.0.10
```

## Platform-Specific Toolchains

CCGO supports two approaches for platform-specific builds:

### 1. Docker-Based Builds (Recommended)

No local toolchains required! Docker builds work on any host OS:

```bash
# Install Docker Desktop
# macOS: https://docs.docker.com/desktop/mac/install/
# Windows: https://docs.docker.com/desktop/windows/install/
# Linux: https://docs.docker.com/engine/install/

# Verify Docker installation
docker --version
```

### 2. Native Toolchains (Optional)

For native builds without Docker:

=== "Android"
    ```bash
    # Download Android SDK/NDK
    # Via Android Studio or command line tools

    # Set environment variables
    export ANDROID_HOME=$HOME/Android/Sdk
    export ANDROID_NDK=$HOME/Android/Sdk/ndk/25.2.9519653
    ```

=== "iOS/macOS"
    ```bash
    # Install Xcode from App Store
    xcode-select --install

    # Accept Xcode license
    sudo xcodebuild -license accept
    ```

=== "Windows"
    ```bash
    # Option 1: Visual Studio (MSVC)
    # Install Visual Studio 2019+ with C++ workload

    # Option 2: MinGW (via Docker recommended)
    # No local installation needed for Docker builds
    ```

=== "Linux"
    ```bash
    # GCC/Clang already installed via build-essential
    # No additional setup needed
    ```

=== "OpenHarmony"
    ```bash
    # Download OpenHarmony SDK
    # https://developer.harmonyos.com/

    # Set environment variable
    export OHOS_SDK_HOME=$HOME/openharmony/sdk
    ```

## Next Steps

- [Quick Start](quickstart.md) - Create your first project
- [Configuration](configuration.md) - Configure CCGO for your needs
- [Docker Builds](../features/docker-builds.md) - Learn about universal cross-compilation
