# Docker 构建

使用 CCGO 的 Docker 进行通用跨平台编译构建 C++ 库的完整指南。

## 概述

CCGO 的 Docker 构建实现了**通用跨平台编译** - 从任何主机操作系统为任何平台构建库，无需安装特定平台的工具链。

**主要特性：**

- **随处构建**：在任何操作系统上编译 Linux、Windows、macOS、iOS、watchOS、tvOS、Android
- **零依赖**：无需安装 Xcode、Visual Studio、Android Studio 或 SDK
- **预构建镜像**：使用 Docker Hub 的镜像快速设置（比手动构建快 3-20 倍）
- **一致的环境**：所有开发者使用相同的工具链版本
- **隔离构建**：不与主机系统工具冲突
- **可重现**：保证在不同机器上构建相同

## 为什么使用 Docker 构建？

### 不使用 Docker

**限制：**
- **平台锁定**：iOS/macOS 需要 macOS，MSVC 需要 Windows，Linux 需要 Linux
- **复杂设置**：安装和配置多个 SDK 和工具链
- **版本冲突**：不同项目可能需要不同的工具链版本
- **存储开销**：每个平台的 SDK 可能消耗 10-50GB
- **设置时间**：安装和配置所有工具需要数小时

### 使用 Docker

**优势：**
- **平台无关**：从任何操作系统构建任何平台
- **快速设置**：下载预构建镜像（2-10 分钟）
- **更小的占用空间**：镜像为 800MB-3.5GB vs 10-50GB SDK
- **即时切换**：轻松切换工具链版本
- **CI/CD 就绪**：非常适合自动化构建
- **可重现**：到处都是相同的环境

## 支持的平台

| 平台 | Docker 镜像 | 大小 | 工具链 | 主机操作系统 |
|----------|-------------|------|-----------|---------|
| Linux | `ccgo-builder-linux` | ~800MB | GCC, Clang, CMake | 任意 |
| Windows | `ccgo-builder-windows` | ~1.2GB | MinGW-w64, CMake | 任意 |
| macOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | 任意 |
| iOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | 任意 |
| watchOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | 任意 |
| tvOS | `ccgo-builder-apple` | ~2.5GB | OSXCross, CMake | 任意 |
| Android | `ccgo-builder-android` | ~3.5GB | Android NDK, CMake | 任意 |

**注意**：Apple 平台（macOS、iOS、watchOS、tvOS）共享同一镜像。

## 前置条件

### 安装 Docker Desktop

**macOS:**
```bash
# 从 docker.com 下载或使用 Homebrew
brew install --cask docker

# 启动 Docker Desktop
open -a Docker
```

**Windows:**
```bash
# 从 docker.com 下载 Docker Desktop
# 运行安装程序并重启

# 验证
docker --version
```

**Linux:**
```bash
# Ubuntu/Debian
sudo apt install docker.io docker-compose
sudo systemctl start docker
sudo systemctl enable docker

# 将用户添加到 docker 组
sudo usermod -aG docker $USER
# 注销并重新登录

# 验证
docker --version
```

### 系统要求

- **磁盘空间**：所有 Docker 镜像需要 5-10GB
- **内存**：推荐 4GB+ RAM
- **CPU**：推荐现代多核处理器
- **网络**：首次镜像下载需要快速互联网

## 快速开始

### 基础 Docker 构建

```bash
# 从任何操作系统为 Linux 构建
ccgo build linux --docker

# 从 macOS 或 Linux 为 Windows 构建
ccgo build windows --docker

# 从 Windows 或 Linux 为 macOS 构建
ccgo build macos --docker

# 从 Windows 或 Linux 为 iOS 构建
ccgo build ios --docker

# 从任何操作系统为 Android 构建
ccgo build android --docker
```

### 首次构建（镜像下载）

```bash
# 首次：下载预构建镜像
$ ccgo build linux --docker

正在拉取 Docker 镜像 ccgo-builder-linux:latest...
latest: Pulling from ccgo/ccgo-builder-linux
Digest: sha256:abc123...
Status: Downloaded newer image for ccgo-builder-linux:latest

在 Docker 容器中构建...
[构建输出...]
构建完成！
```

**下载时间（仅首次构建）：**
- Linux: ~2-3 分钟（~800MB）
- Windows: ~3-4 分钟（~1.2GB）
- Apple: ~5-8 分钟（~2.5GB）
- Android: ~8-10 分钟（~3.5GB）

### 后续构建

```bash
# 所有后续构建使用缓存的镜像（即时启动）
$ ccgo build linux --docker

使用现有的 Docker 镜像 ccgo-builder-linux:latest...
在 Docker 容器中构建...
[构建输出...]
构建完成！
```

## Docker 构建工作原理

### 构建流程

1. **镜像选择**：CCGO 为目标平台选择适当的 Docker 镜像
2. **镜像下载**：从 Docker Hub 拉取预构建镜像（仅首次）
3. **容器启动**：启动挂载项目目录的 Docker 容器
4. **构建执行**：在容器内使用平台工具链运行构建
5. **输出收集**：将构建制品写入主机文件系统
6. **清理**：删除容器（镜像保持缓存）

### 文件系统挂载

```
主机                          Docker 容器
====                          ================
/project/                 --> /workspace/
  ├── src/                    ├── src/
  ├── include/                ├── include/
  ├── CCGO.toml               ├── CCGO.toml
  └── target/             <-- └── target/
       └── linux/                  └── linux/
```

- **项目目录**以只读方式挂载
- **输出目录**以读写方式挂载
- **构建制品**写入主机文件系统

### 工具链环境

**Linux 容器：**
```bash
# Ubuntu 22.04
gcc 11.4.0
clang 14.0.0
cmake 3.22.1
```

**Windows 容器：**
```bash
# Ubuntu + MinGW-w64
x86_64-w64-mingw32-gcc 10.0.0
cmake 3.22.1
```

**Apple 容器：**
```bash
# Ubuntu + OSXCross
clang 14.0.0 (LLVM)
OSXCross targeting macOS 10.13+
cmake 3.22.1
```

**Android 容器：**
```bash
# Ubuntu + Android NDK
Android NDK r25c
cmake 3.22.1
```

## Docker 镜像管理

### 列出已下载的镜像

```bash
# 列出 CCGO Docker 镜像
docker images | grep ccgo-builder

# 输出：
# ccgo-builder-linux    latest    abc123    2 weeks ago    800MB
# ccgo-builder-windows  latest    def456    2 weeks ago    1.2GB
# ccgo-builder-apple    latest    ghi789    2 weeks ago    2.5GB
```

### 更新镜像

```bash
# 将所有镜像更新到最新版本
docker pull ccgo-builder-linux:latest
docker pull ccgo-builder-windows:latest
docker pull ccgo-builder-apple:latest
docker pull ccgo-builder-android:latest
```

### 删除镜像

```bash
# 删除特定镜像
docker rmi ccgo-builder-linux:latest

# 删除所有 CCGO 镜像
docker rmi $(docker images -q "ccgo-builder-*")

# 回收空间
docker system prune -a
```

### 磁盘空间

```bash
# 检查 Docker 磁盘使用情况
docker system df

# 输出：
# TYPE            TOTAL     ACTIVE    SIZE      RECLAIMABLE
# Images          4         0         7.8GB     7.8GB (100%)
# Containers      0         0         0B        0B
# Local Volumes   0         0         0B        0B
```

## 高级用法

### 构建多个平台

```bash
# 为所有移动平台构建
ccgo build android --docker
ccgo build ios --docker

# 为所有桌面平台构建
ccgo build linux --docker
ccgo build windows --docker
ccgo build macos --docker
```

### 架构选择

```bash
# Android - 多个架构
ccgo build android --docker --arch armeabi-v7a,arm64-v8a,x86_64

# iOS - 为设备和模拟器构建
ccgo build ios --docker --arch arm64,x86_64

# Windows - 不同架构
ccgo build windows --docker --arch x86,x64,arm64
```

### 自定义 Docker 选项

```bash
# 传递自定义 Docker 运行选项
DOCKER_OPTS="--cpus=4 --memory=8g" ccgo build linux --docker

# 挂载额外的卷
DOCKER_OPTS="-v /extra/libs:/libs:ro" ccgo build linux --docker
```

### 并行构建

```bash
# 并行构建多个平台
ccgo build linux --docker &
ccgo build windows --docker &
ccgo build macos --docker &
wait

echo "所有构建完成！"
```

## 平台特定说明

### Linux Docker 构建

**优势：**
- 在 macOS 或 Windows 上构建
- 一致的 glibc 版本
- 同时提供 GCC 和 Clang

**限制：**
- 无法运行 GUI 应用程序
- 无法测试应用程序（没有 X11 显示）

**使用：**
```bash
# 为 x86_64 构建
ccgo build linux --docker --arch x86_64

# 为 ARM64 构建
ccgo build linux --docker --arch arm64

# 使用特定编译器构建
ccgo build linux --docker --compiler clang
```

### Windows Docker 构建

**优势：**
- 在 macOS 或 Linux 上构建
- 无需 Visual Studio

**限制：**
- 仅 MinGW（无 MSVC）
- 输出可能与 MSVC 构建的代码不兼容
- 无法运行 Windows 应用程序

**使用：**
```bash
# 为 x64 构建
ccgo build windows --docker --arch x64

# 为 x86 构建
ccgo build windows --docker --arch x86

# 构建两种架构
ccgo build windows --docker --arch x86,x64
```

**MSVC 兼容性注意事项：**
- Docker 构建使用 MinGW-w64（Windows 的 GCC）
- 对于 MSVC 构建，使用原生 Windows 构建
- MinGW 和 MSVC 有不同的 ABI（二进制不兼容）

### Apple Docker 构建

**优势：**
- 在 Windows 或 Linux 上构建 macOS/iOS/watchOS/tvOS
- 无需 Xcode 或 macOS
- 使用 OSXCross（基于 LLVM）

**限制：**
- 无法对应用程序进行代码签名
- 无法运行或测试应用程序
- 无法公证应用程序
- 无 Xcode 项目生成

**使用：**
```bash
# 为 macOS 构建
ccgo build macos --docker

# 为 iOS 构建
ccgo build ios --docker

# 为多个 Apple 平台构建
ccgo build macos --docker
ccgo build ios --docker
ccgo build watchos --docker
ccgo build tvos --docker
```

### Android Docker 构建

**优势：**
- 在任何操作系统上构建
- 无需 Android Studio
- 一致的 NDK 版本

**限制：**
- 无法在 Android 设备上运行
- 无法生成 Android Studio 项目
- 无法运行测试

**使用：**
```bash
# 为所有 Android 架构构建
ccgo build android --docker --arch armeabi-v7a,arm64-v8a,x86,x86_64

# 构建 AAR 包
ccgo build android --docker --aar
```

## 故障排除

### 找不到 Docker

```
Error: docker command not found
```

**解决方案：**
```bash
# 安装 Docker Desktop 并确保其运行
docker --version

# macOS: 检查 Docker Desktop 是否运行
open -a Docker
```

### 权限被拒绝

```
Error: permission denied while trying to connect to the Docker daemon
```

**解决方案（Linux）：**
```bash
# 将用户添加到 docker 组
sudo usermod -aG docker $USER

# 注销并重新登录
# 或使用 sudo 运行（不推荐）
sudo ccgo build linux --docker
```

### 镜像拉取失败

```
Error: Error response from daemon: manifest for ccgo-builder-linux:latest not found
```

**解决方案：**
```bash
# 检查互联网连接
ping docker.io

# 尝试手动拉取
docker pull ccgo-builder-linux:latest

# 检查 Docker Hub 状态
# https://status.docker.com/
```

### 磁盘空间问题

```
Error: no space left on device
```

**解决方案：**
```bash
# 检查磁盘空间
docker system df

# 清理未使用的镜像
docker system prune -a

# 删除特定镜像
docker rmi ccgo-builder-linux:latest
```

### 找不到构建制品

```
Error: Build artifacts not found in target/
```

**解决方案：**
```bash
# 检查 Docker 挂载权限
ls -la target/

# 确保 target 目录存在且可写
mkdir -p target
chmod 755 target

# 检查 Docker 日志
docker logs $(docker ps -lq)
```

### 构建缓慢

```
Docker 内的构建时间太长
```

**解决方案：**
```bash
# 为 Docker 分配更多资源
# Docker Desktop → 首选项 → 资源
# 增加：CPU（4+）、内存（8GB+）、磁盘空间

# 使用 Docker 原生而非虚拟化
#（在 Linux 上更快）

# 启用 BuildKit 以更快地拉取镜像
export DOCKER_BUILDKIT=1
```

## 最佳实践

### 1. 在 CI/CD 中使用 Docker

非常适合自动化构建：

```yaml
# .github/workflows/build.yml
name: Build
on: [push]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Build all platforms
        run: |
          ccgo build linux --docker
          ccgo build windows --docker
          ccgo build macos --docker
```

### 2. 缓存 Docker 镜像

不要在构建之间删除镜像：

```bash
# 保持镜像缓存以快速构建
# 仅在需要时更新
docker pull ccgo-builder-linux:latest
```

### 3. 用于可重现的构建

到处都是相同的环境：

```bash
# 开发
ccgo build linux --docker

# CI/CD
ccgo build linux --docker

# 结果：相同的二进制文件
```

### 4. 与原生构建结合

使用 Docker 进行跨平台，使用原生进行平台特定：

```bash
# 在 macOS 上
ccgo build macos              # 原生（代码签名、测试）
ccgo build linux --docker     # Docker（交叉编译）
ccgo build windows --docker   # Docker（交叉编译）
```

### 5. 定期镜像更新

```bash
# 每月：更新镜像
docker pull ccgo-builder-linux:latest
docker pull ccgo-builder-windows:latest
docker pull ccgo-builder-apple:latest
docker pull ccgo-builder-android:latest
```

### 6. 资源分配

```bash
# 分配足够的资源
# Docker Desktop → 首选项 → 资源：
# - CPU：4-8（更多 = 更快的并行构建）
# - 内存：8-16GB
# - 磁盘：50GB+
```

## 性能优化

### 并行平台构建

```bash
#!/bin/bash
# build-all.sh

echo "构建所有平台..."

ccgo build linux --docker &
PID_LINUX=$!

ccgo build windows --docker &
PID_WINDOWS=$!

ccgo build macos --docker &
PID_MACOS=$!

ccgo build android --docker &
PID_ANDROID=$!

wait $PID_LINUX $PID_WINDOWS $PID_MACOS $PID_ANDROID

echo "所有构建完成！"
```

### 增量构建

Docker 构建支持增量编译：

```bash
# 首次构建（清洁）
ccgo build linux --docker

# 后续构建（增量）
# 仅重新编译更改的文件
ccgo build linux --docker
```

### 资源调整

```bash
# 限制单个构建的 Docker 资源
DOCKER_OPTS="--cpus=2 --memory=4g" ccgo build linux --docker

# 最大资源以更快构建
DOCKER_OPTS="--cpus=8 --memory=16g" ccgo build linux --docker
```

## 安全考虑

### 1. 镜像验证

```bash
# 验证镜像签名（如果可用）
docker trust inspect ccgo-builder-linux:latest

# 检查镜像摘要
docker images --digests | grep ccgo-builder
```

### 2. 网络隔离

```bash
# 无网络访问构建（镜像下载后）
DOCKER_OPTS="--network=none" ccgo build linux --docker
```

### 3. 只读挂载

```bash
# 以只读方式挂载源代码（CCGO 中的默认设置）
DOCKER_OPTS="-v $(pwd):/workspace:ro" ccgo build linux --docker
```

### 4. 用户权限

```bash
# 在容器内以非 root 用户运行（CCGO 默认）
# 输出文件由当前用户拥有
ls -l target/linux/
```

## 比较：原生 vs Docker

| 方面 | 原生构建 | Docker 构建 |
|--------|-------------|-------------|
| **设置时间** | 数小时（SDK 安装） | 数分钟（镜像下载） |
| **磁盘空间** | 每平台 10-50GB | 每平台 800MB-3.5GB |
| **构建速度** | 更快（原生） | 稍慢（~10%） |
| **跨平台** | 有限（需要目标操作系统） | 完整（从任何操作系统到任何目标） |
| **可重现性** | 中等（版本漂移） | 高（固定环境） |
| **代码签名** | 是 | 否（Apple 平台） |
| **测试** | 是 | 有限/否 |
| **CI/CD** | 复杂 | 简单 |
| **维护** | 手动更新 | 自动（拉取镜像） |

**建议：**
- **开发**：目标平台的原生构建（更好的测试/调试）
- **CI/CD**：所有平台的 Docker（一致性、可重现性）
- **交叉编译**：始终使用 Docker（没有目标操作系统的唯一选项）

## 另请参阅

- [构建系统](build-system.md)
- [Linux 平台](../platforms/linux.md)
- [Windows 平台](../platforms/windows.md)
- [macOS 平台](../platforms/macos.md)
- [iOS 平台](../platforms/ios.md)
- [Android 平台](../platforms/android.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
