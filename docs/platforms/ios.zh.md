# iOS 平台

使用 CCGO 为 iOS 构建 C++ 库的完整指南。

## 概述

CCGO 提供全面的 iOS 支持：

- **多架构支持**：arm64、x86_64（模拟器）
- **输出格式**：静态/动态 Framework、XCFramework
- **构建方式**：本地构建（Xcode）或 Docker（跨平台）
- **Swift 互操作**：轻松与 Swift 代码集成
- **包管理器**：CocoaPods 和 Swift Package Manager
- **代码签名**：自动处理签名要求
- **Bitcode**：可选的 bitcode 支持（Xcode 14+ 已弃用）

## 前置条件

### 方式一：本地构建（需要 macOS）

**必需：**
- macOS 12.0+ (Monterey 或更高版本)
- Xcode 13.0+ 及命令行工具
- CMake 3.20+

**安装方法：**

```bash
# 从 Mac App Store 安装 Xcode
# 然后安装命令行工具
xcode-select --install

# 验证安装
xcode-select -p
# 应输出：/Applications/Xcode.app/Contents/Developer

# 安装 CMake（通过 Homebrew）
brew install cmake
```

### 方式二：Docker 构建（任何操作系统）

无需 Xcode！在 Linux 或 Windows 上使用 Docker 构建 iOS 库。

**必需：**
- 已安装并运行 Docker Desktop
- 10GB+ 磁盘空间用于 Docker 镜像

**优势：**
- 在任何操作系统上构建
- 无需 Xcode 许可证
- 一致的构建环境
- 与主机系统隔离

**限制：**
- 无法运行/测试 iOS 应用
- 初始下载较大（约 2.5GB 镜像）
- 比原生 Xcode 构建慢

详见 [Docker 构建](#docker-构建)部分。

## 快速开始

### 基本构建

```bash
# 为所有 iOS 架构构建（arm64 设备 + x86_64 模拟器）
ccgo build ios

# 使用 Docker 构建（无需 Xcode）
ccgo build ios --docker

# 构建特定架构
ccgo build ios --arch arm64                    # 仅设备
ccgo build ios --arch x86_64                   # 仅模拟器
ccgo build ios --arch arm64,x86_64            # 两者

# 构建类型
ccgo build ios --build-type debug             # Debug 构建
ccgo build ios --build-type release           # Release 构建（默认）

# 链接类型
ccgo build ios --link-type static             # 仅静态框架
ccgo build ios --link-type shared             # 仅动态框架
ccgo build ios --link-type both               # 两种类型（默认）
```

### 使用 XCFramework 构建

XCFramework 将多个架构打包到单个包中：

```bash
# 构建 XCFramework（推荐）
ccgo build ios --xcframework

# 同时构建 Framework 和 XCFramework
ccgo build ios --xcframework --framework
```

### 生成 Xcode 项目

```bash
# 为开发生成 Xcode 项目
ccgo build ios --ide-project

# 打开生成的项目
open cmake_build/ios/MyLib.xcodeproj
```

## 输出结构

### 默认输出 (`target/ios/`)

```
target/ios/
├── MyLib_iOS_SDK-1.0.0.zip          # 主包
│   ├── frameworks/
│   │   ├── static/
│   │   │   ├── MyLib.framework/     # 静态 Framework
│   │   │   │   ├── MyLib            # Fat 二进制（arm64 + x86_64）
│   │   │   │   ├── Headers/         # 公共头文件
│   │   │   │   │   └── MyLib.h
│   │   │   │   ├── Modules/
│   │   │   │   │   └── module.modulemap
│   │   │   │   └── Info.plist
│   │   │   └── MyLib.xcframework/   # XCFramework（如果构建）
│   │   │       ├── ios-arm64/
│   │   │       │   └── MyLib.framework/
│   │   │       ├── ios-arm64_x86_64-simulator/
│   │   │       │   └── MyLib.framework/
│   │   │       └── Info.plist
│   │   └── shared/
│   │       ├── MyLib.framework/     # 动态 Framework
│   │       └── MyLib.xcframework/   # 动态 XCFramework
│   ├── include/
│   │   └── mylib/                   # 头文件
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json              # 构建元数据
│
└── MyLib_iOS_SDK-1.0.0-SYMBOLS.zip  # 调试符号
    └── symbols/
        ├── static/
        │   └── MyLib.framework.dSYM/
        └── shared/
            └── MyLib.framework.dSYM/
```

### Framework 结构

**静态 Framework：**
- 包含 `.a` 静态库
- 必须在编译时链接
- 应用体积更小（死代码剥离）
- 分发更容易（无动态链接问题）

**动态 Framework：**
- 包含 `.dylib` 动态库
- 运行时加载
- 可在应用和扩展之间共享
- 需要代码签名

**XCFramework：**
- 设备和模拟器的统一包
- Xcode 自动选择正确的架构
- 推荐用于库分发
- 支持多个平台（iOS、Catalyst 等）

### 构建元数据

`build_info.json` 包含：

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My iOS library"
  },
  "build": {
    "platform": "ios",
    "architectures": ["arm64", "x86_64"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "xcode_version": "15.0"
  },
  "outputs": {
    "frameworks": [
      "frameworks/static/MyLib.framework",
      "frameworks/shared/MyLib.framework"
    ],
    "xcframeworks": [
      "frameworks/static/MyLib.xcframework",
      "frameworks/shared/MyLib.xcframework"
    ],
    "headers": "include/mylib/",
    "symbols": [
      "symbols/static/MyLib.framework.dSYM",
      "symbols/shared/MyLib.framework.dSYM"
    ]
  },
  "dependencies": {
    "spdlog": "1.12.0",
    "fmt": "10.1.1"
  }
}
```

## Swift 集成

### 在 Swift 中使用 Framework

**添加到 Xcode 项目：**

1. 将 `MyLib.framework` 或 `MyLib.xcframework` 拖入 Xcode 项目
2. 选择"Copy items if needed"
3. 添加到"Frameworks, Libraries, and Embedded Content"
4. 对于动态框架，设置为"Embed & Sign"

**在 Swift 中导入：**

```swift
import MyLib

class MyViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()

        // 通过桥接调用 C++ 代码
        let version = MyLib.getVersion()
        print("Library version: \(version)")

        // 创建 C++ 对象
        let lib = MyLibWrapper()
        lib.initialize()

        // 调用方法
        let result = lib.processData("Hello from Swift")
        print("Result: \(result)")
    }
}
```

### C++/Swift 桥接

**方式一：Objective-C++ 包装器（推荐）**

在 C++ 库中创建包装器：

```objc
// MyLibWrapper.h
#import <Foundation/Foundation.h>

@interface MyLibWrapper : NSObject

+ (NSString *)getVersion;
- (instancetype)init;
- (void)initialize;
- (NSString *)processData:(NSString *)input;

@end
```

```objc
// MyLibWrapper.mm
#import "MyLibWrapper.h"
#include "mylib/mylib.h"

@implementation MyLibWrapper {
    std::unique_ptr<mylib::MyLib> _impl;
}

+ (NSString *)getVersion {
    std::string version = mylib::get_version();
    return [NSString stringWithUTF8String:version.c_str()];
}

- (instancetype)init {
    self = [super init];
    if (self) {
        _impl = std::make_unique<mylib::MyLib>();
    }
    return self;
}

- (void)initialize {
    _impl->initialize();
}

- (NSString *)processData:(NSString *)input {
    std::string cppInput = [input UTF8String];
    std::string result = _impl->process(cppInput);
    return [NSString stringWithUTF8String:result.c_str()];
}

@end
```

**方式二：纯 Swift 包装器（Swift 5.9+）**

```swift
// MyLibSwift.swift
import MyLib

public class MyLibSwift {
    public static func getVersion() -> String {
        return String(cString: mylib_get_version())
    }

    private var handle: OpaquePointer?

    public init() {
        handle = mylib_create()
    }

    deinit {
        mylib_destroy(handle)
    }

    public func processData(_ input: String) -> String {
        let result = input.withCString { cString in
            return mylib_process(handle, cString)
        }
        return String(cString: result!)
    }
}
```

需要在库中提供 C 接口：

```cpp
// mylib_c_api.h
#ifdef __cplusplus
extern "C" {
#endif

const char* mylib_get_version(void);
void* mylib_create(void);
void mylib_destroy(void* handle);
const char* mylib_process(void* handle, const char* input);

#ifdef __cplusplus
}
#endif
```

### 模块映射

为了让 Swift 导入工作，您的框架需要模块映射：

```
// module.modulemap
framework module MyLib {
    umbrella header "MyLib.h"
    export *
    module * { export * }
}
```

CCGO 会自动在您的框架中生成这个文件。

## CocoaPods 集成

### 发布到 CocoaPods

```bash
# 生成 podspec
ccgo publish apple --manager cocoapods

# 验证 podspec
pod spec lint MyLib.podspec

# 发布到 CocoaPods Trunk
ccgo publish apple --manager cocoapods --push

# 发布到私有 spec 仓库
ccgo publish apple --manager cocoapods \
    --registry private \
    --remote-name myspecs \
    --url https://github.com/mycompany/specs.git
```

### 生成的 Podspec

```ruby
# MyLib.podspec
Pod::Spec.new do |s|
  s.name             = 'MyLib'
  s.version          = '1.0.0'
  s.summary          = 'My iOS library'
  s.description      = 'A cross-platform C++ library for iOS'
  s.homepage         = 'https://github.com/myuser/mylib'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 'Your Name' => 'you@example.com' }
  s.source           = { :git => 'https://github.com/myuser/mylib.git', :tag => s.version.to_s }

  s.ios.deployment_target = '12.0'
  s.swift_version = '5.0'

  # XCFramework（推荐）
  s.vendored_frameworks = 'target/ios/frameworks/static/MyLib.xcframework'

  # 或普通框架
  # s.vendored_frameworks = 'target/ios/frameworks/static/MyLib.framework'

  # 依赖项
  s.dependency 'Alamofire', '~> 5.0'
end
```

### 在 iOS 项目中使用

**Podfile：**

```ruby
platform :ios, '12.0'
use_frameworks!

target 'MyApp' do
  pod 'MyLib', '~> 1.0'
end
```

**安装：**

```bash
pod install
open MyApp.xcworkspace
```

## Swift Package Manager 集成

### 发布到 SPM

```bash
# 生成 Package.swift
ccgo publish apple --manager spm

# 推送到 Git（创建标签）
ccgo publish apple --manager spm --push
```

### 生成的 Package.swift

```swift
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "MyLib",
    platforms: [
        .iOS(.v12)
    ],
    products: [
        .library(
            name: "MyLib",
            targets: ["MyLib"]
        ),
    ],
    targets: [
        .binaryTarget(
            name: "MyLib",
            path: "target/ios/frameworks/static/MyLib.xcframework"
        )
    ]
)
```

### 在 iOS 项目中使用

**Package.swift：**

```swift
dependencies: [
    .package(url: "https://github.com/myuser/mylib.git", from: "1.0.0")
]
```

**或在 Xcode 中：**

1. File → Add Packages...
2. 输入仓库 URL
3. 选择版本规则
4. 添加到目标

## 代码签名

### 自动签名

CCGO 自动处理框架的代码签名：

```bash
# 使用默认身份签名
ccgo build ios

# 指定签名身份
export CODE_SIGN_IDENTITY="Apple Development: Your Name (TEAM123456)"
ccgo build ios
```

### 手动签名

```bash
# 查找可用的身份
security find-identity -v -p codesigning

# 签名框架
codesign --force --sign "Apple Development" \
    --timestamp \
    target/ios/frameworks/shared/MyLib.framework

# 验证签名
codesign --verify --verbose target/ios/frameworks/shared/MyLib.framework
```

### 分发签名

用于 App Store 分发：

```bash
# 使用分发证书签名
export CODE_SIGN_IDENTITY="Apple Distribution: Company Name (TEAM123456)"
ccgo build ios --build-type release
```

### 签名故障排除

```bash
# 检查当前签名
codesign -dvv target/ios/frameworks/shared/MyLib.framework

# 移除现有签名
codesign --remove-signature target/ios/frameworks/shared/MyLib.framework

# 使用特定权限签名
codesign --force --sign "Apple Development" \
    --entitlements Entitlements.plist \
    target/ios/frameworks/shared/MyLib.framework
```

## Docker 构建

使用 Docker 和 OSXCross 在任何操作系统上构建 iOS 库：

### 前置条件

```bash
# 安装 Docker Desktop
# 下载地址：https://www.docker.com/products/docker-desktop/

# 验证 Docker 正在运行
docker ps
```

### 使用 Docker 构建

```bash
# 首次构建下载预构建镜像（约 2.5GB）
ccgo build ios --docker

# 后续构建很快（无需下载）
ccgo build ios --docker --arch arm64

# 所有标准选项都可用
ccgo build ios --docker --xcframework --link-type static
```

### 工作原理

1. CCGO 使用 Docker Hub 的预构建 `ccgo-builder-apple` 镜像
2. 项目目录挂载到容器中
3. 构建在容器内使用 OSXCross 工具链运行
4. 输出写入主机文件系统

### 优势

- **跨平台**：在 Linux、Windows、macOS 上构建
- **无需 Xcode**：跳过 40GB+ 的 Xcode 安装
- **隔离**：干净的构建环境
- **可复现**：任何机器上都有相同的结果

### 限制

- **无法运行**：Docker 中没有 iOS 运行时
- **无模拟器**：无法在 iOS 模拟器中测试
- **无 Xcode**：无法打开生成的 Xcode 项目
- **构建体积大**：Docker 镜像约 2.5GB
- **首次运行慢**：初始镜像下载

### Docker 镜像详情

镜像：`ccgo-builder-apple:latest`
- 基础：Ubuntu 22.04
- 工具链：OSXCross（Clang 15）
- SDK：iOS 16.0 SDK
- 支持：iOS、macOS、watchOS、tvOS
- 大小：约 2.5GB 压缩后

## 平台配置

### CCGO.toml 设置

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"                  # static、shared 或 both

[build]
cpp_standard = "17"            # C++ 标准

[ios]
deployment_target = "12.0"     # 最低 iOS 版本
enable_bitcode = false         # Bitcode 支持（已弃用）
enable_arc = true              # 自动引用计数
frameworks = [                 # 要链接的系统框架
    "Foundation",
    "UIKit",
    "CoreGraphics"
]
```

### CMake 变量

为 iOS 构建时，这些变量可用：

```cmake
${PLATFORM}                    # "ios"
${ARCHITECTURE}                # "arm64" 或 "x86_64"
${BUILD_TYPE}                  # "Debug" 或 "Release"
${LINK_TYPE}                   # "static"、"shared" 或 "both"
${IOS_DEPLOYMENT_TARGET}       # "12.0"（来自 CCGO.toml）
${CMAKE_OSX_SYSROOT}           # iOS SDK 路径
${CMAKE_OSX_ARCHITECTURES}     # "arm64" 或 "x86_64"
```

### 条件编译

```cpp
// 在 C++ 代码中
#ifdef __APPLE__
#include <TargetConditionals.h>

#if TARGET_OS_IOS
    // iOS 特定代码
    #import <UIKit/UIKit.h>
    UIDevice *device = [UIDevice currentDevice];

#elif TARGET_OS_SIMULATOR
    // iOS 模拟器特定代码

#endif
#endif
```

## 最佳实践

### 1. 使用 XCFramework

XCFramework 是分发 iOS 库的现代方式：

```bash
# 总是为分发构建 XCFramework
ccgo build ios --xcframework
```

**优势：**
- 设备和模拟器的单个包
- Xcode 自动选择架构
- 支持多个平台
- 更好的 Xcode 集成

### 2. 版本化 iOS SDK

根据用户群匹配您的部署目标：

```toml
[ios]
deployment_target = "12.0"  # 覆盖 95%+ 用户
```

### 3. 静态 vs 动态

**使用静态时：**
- 分发独立库
- 想要更小的应用体积
- 不需要在应用/扩展之间共享代码

**使用动态时：**
- 在应用和扩展之间共享代码
- 需要运行时插件加载
- 想要更快的增量构建

### 4. 代码签名

总是签名动态框架：

```bash
# 分发构建需要适当的签名
export CODE_SIGN_IDENTITY="Apple Distribution: Company"
ccgo build ios --build-type release
```

### 5. 最小化依赖

保持库专注：

```toml
[dependencies]
# 仅必要的依赖项
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 平台特定依赖项
[target.'cfg(target_os = "ios")'.dependencies]
ios-utils = { path = "./ios-utils" }
```

### 6. 在设备上测试

模拟器和设备行为不同：

```bash
# 为两者构建
ccgo build ios --arch arm64,x86_64

# 在模拟器上测试（x86_64）
# 在真实设备上测试（arm64）
```

### 7. 调试符号

总是使用符号进行调试构建：

```bash
# 默认包含符号
ccgo build ios --build-type debug

# 符号在单独的包中
# MyLib_iOS_SDK-1.0.0-SYMBOLS.zip
```

## 高级主题

### Bitcode 支持（已弃用）

!!! warning "已弃用"
    Bitcode 在 Xcode 14+ 中已弃用，不建议用于新项目。

```toml
[ios]
enable_bitcode = false  # 保持禁用
```

### 通用二进制

构建包含多个架构的 fat 二进制：

```bash
# 构建两个架构
ccgo build ios --arch arm64,x86_64

# 结果是通用 Framework
lipo -info target/ios/frameworks/static/MyLib.framework/MyLib
# 输出：arm64 x86_64
```

### 最低操作系统版本

根据所需功能设置部署目标：

```toml
[ios]
deployment_target = "12.0"  # iOS 12+
# deployment_target = "13.0"  # 用于 SwiftUI
# deployment_target = "14.0"  # 用于 Widgets
# deployment_target = "15.0"  # 用于 async/await
```

### Framework 资源

在框架中包含资源：

```
MyLib.framework/
├── MyLib
├── Headers/
├── Modules/
├── Resources/         # 在这里添加资源
│   ├── images/
│   ├── configs/
│   └── Info.plist
└── Info.plist
```

### 应用扩展

为应用扩展单独构建：

```bash
# 主应用框架
ccgo build ios

# 扩展框架（有限 API）
export IOS_APP_EXTENSION=1
ccgo build ios
```

## 故障排除

### 未找到 Xcode

```
Error: Could not find Xcode installation
```

**解决方案：**

```bash
# 从 App Store 安装 Xcode
# 安装命令行工具
xcode-select --install

# 设置 Xcode 路径
sudo xcode-select --switch /Applications/Xcode.app

# 验证
xcode-select -p
```

### 架构不匹配

```
Error: Building for iOS, but linking in object file built for iOS Simulator
```

**解决方案：**

```bash
# 清理构建
ccgo clean -y

# 为特定架构构建
ccgo build ios --arch arm64        # 设备
ccgo build ios --arch x86_64       # 模拟器

# 或构建两者
ccgo build ios --arch arm64,x86_64
```

### 签名失败

```
Error: Code signing failed
```

**解决方案：**

1. 检查可用身份：
```bash
security find-identity -v -p codesigning
```

2. 设置正确的身份：
```bash
export CODE_SIGN_IDENTITY="Apple Development: Name (TEAM123)"
```

3. 对于开发构建，使用 ad-hoc 签名：
```bash
export CODE_SIGN_IDENTITY="-"
```

### 未找到模块

```
Error: No such module 'MyLib'
```

**解决方案：**

1. 确保框架已添加到目标
2. 检查框架中是否存在 module.modulemap
3. 在 Xcode 中添加框架搜索路径：
   - Build Settings → Framework Search Paths
   - 添加框架目录路径

4. 对于 Swift Package Manager：
```swift
// 确保目标依赖于 MyLib
.target(
    name: "MyApp",
    dependencies: ["MyLib"]
)
```

### 动态框架未加载

```
dyld: Library not loaded: @rpath/MyLib.framework/MyLib
```

**解决方案：**

1. 在应用中嵌入框架：
   - Xcode → Target → General
   - Frameworks, Libraries, and Embedded Content
   - 设置为"Embed & Sign"

2. 检查 Runpath Search Paths：
   - Build Settings → Runpath Search Paths
   - 应包含 `@executable_path/Frameworks`

### 构建极慢

```
Build taking very long time...
```

**解决方案：**

1. 使用增量构建：
```bash
# 构建之间不清理
ccgo build ios
```

2. 构建特定架构：
```bash
# 只构建所需的
ccgo build ios --arch arm64
```

3. 使用 Docker 获得干净环境：
```bash
# 有时原生构建很慢
ccgo build ios --docker
```

4. 检查 Xcode 索引：
```bash
# 禁用 Xcode 索引
defaults write com.apple.dt.Xcode IDEIndexDisable -bool YES
```

### Docker 构建失败

```
Error: Docker daemon not running
```

**解决方案：**

1. 启动 Docker Desktop
2. 等待 Docker 完全启动
3. 验证：`docker ps`

```
Error: Cannot pull Docker image
```

**解决方案：**

1. 检查网络连接
2. 检查 Docker Hub 状态
3. 尝试显式拉取：
```bash
docker pull ccgo-builder-apple:latest
```

## 性能提示

### 1. 缓存依赖项

```bash
# 首次构建后缓存依赖项
# ~/.ccgo/git/<repo>/
```

### 2. 增量构建

```bash
# 小改动之间不清理
ccgo build ios              # 快速增量

# 仅在需要时清理
ccgo clean -y
ccgo build ios              # 完全重建
```

### 3. 架构选择

```bash
# 只构建所需的
ccgo build ios --arch arm64              # 仅设备（快速）
ccgo build ios --arch x86_64             # 仅模拟器（快速）
ccgo build ios --arch arm64,x86_64      # 两者（较慢）
```

### 4. 链接类型

```bash
# 静态构建更快
ccgo build ios --link-type static

# 动态需要签名
ccgo build ios --link-type shared       # 较慢
```

### 5. 并行构建

CMake 自动使用并行构建：

```bash
# 默认使用所有 CPU 核心
ccgo build ios

# 如果需要限制并行度
export CMAKE_BUILD_PARALLEL_LEVEL=4
ccgo build ios
```

## 迁移指南

### 从手动 CMake

**之前（手动 CMake）：**

```bash
mkdir build-ios
cd build-ios
cmake .. \
    -G Xcode \
    -DCMAKE_SYSTEM_NAME=iOS \
    -DCMAKE_OSX_ARCHITECTURES=arm64 \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=12.0 \
    -DCMAKE_XCODE_ATTRIBUTE_DEVELOPMENT_TEAM=TEAM123
cmake --build . --config Release
```

**之后（CCGO）：**

```bash
# 简单的一行命令
ccgo build ios
```

### 从 CocoaPods Podspec

**MyLib.podspec → CCGO.toml：**

```ruby
# 之前（Podspec）
Pod::Spec.new do |s|
  s.name = 'MyLib'
  s.version = '1.0.0'
  s.ios.deployment_target = '12.0'
  s.source_files = 'src/**/*.{cpp,h}'
  s.dependency 'Alamofire'
end
```

```toml
# 之后（CCGO.toml）
[package]
name = "mylib"
version = "1.0.0"

[ios]
deployment_target = "12.0"

[dependencies]
alamofire = { git = "https://github.com/Alamofire/Alamofire.git", tag = "5.8.0" }
```

### 从 Xcode 项目

1. 创建 CCGO 项目：
```bash
ccgo new mylib
```

2. 复制源文件到 `src/`

3. 配置 CCGO.toml：
```toml
[ios]
deployment_target = "12.0"
frameworks = ["Foundation", "UIKit"]
```

4. 构建：
```bash
ccgo build ios
```

## 另请参阅

- [构建系统](../features/build-system.md)
- [依赖管理](../features/dependency-management.md)
- [发布管理](../features/publishing.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [平台概览](index.md)
