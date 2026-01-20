# macOS 平台

使用 CCGO 为 macOS 构建 C++ 库的完整指南。

## 概述

CCGO 提供全面的 macOS 支持：

- **多架构支持**：x86_64（Intel）、arm64（Apple Silicon）
- **通用二进制**：包含两种架构的 Fat 二进制文件
- **输出格式**：静态/动态 Framework、dylib
- **构建方式**：本地构建（Xcode）或 Docker（跨平台）
- **Swift 互操作**：轻松与 Swift 代码集成
- **包管理器**：CocoaPods 和 Swift Package Manager
- **代码签名**：自动处理和公证支持
- **Mac Catalyst**：支持在 Mac 上运行 iPad 应用

## 前置条件

### 方式一：本地构建（需要 macOS）

**必需：**
- macOS 10.15+（Catalina 或更高版本）
- Xcode 12.0+ 及命令行工具
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

无需 Xcode！在 Linux 或 Windows 上使用 Docker 构建 macOS 库。

**必需：**
- 已安装并运行 Docker Desktop
- 10GB+ 磁盘空间用于 Docker 镜像

**优势：**
- 在任何操作系统上构建
- 无需 Xcode 许可证
- 一致的构建环境
- 与主机系统隔离

**限制：**
- 无法运行/测试 macOS 应用
- 初始下载较大（约 2.5GB 镜像）
- 比原生 Xcode 构建慢

详见 [Docker 构建](#docker-构建)部分。

## 快速开始

### 基本构建

```bash
# 为所有 macOS 架构构建（x86_64 + arm64 通用二进制）
ccgo build macos

# 使用 Docker 构建（无需 Xcode）
ccgo build macos --docker

# 构建特定架构
ccgo build macos --arch x86_64                 # 仅 Intel
ccgo build macos --arch arm64                  # 仅 Apple Silicon
ccgo build macos --arch x86_64,arm64          # 通用二进制（默认）

# 构建类型
ccgo build macos --build-type debug           # Debug 构建
ccgo build macos --build-type release         # Release 构建（默认）

# 链接类型
ccgo build macos --link-type static           # 仅静态库/框架
ccgo build macos --link-type shared           # 仅动态库/框架
ccgo build macos --link-type both             # 两种类型（默认）
```

### 使用 Framework 构建

```bash
# 构建 Framework（推荐）
ccgo build macos --framework

# 构建 dylib（传统）
ccgo build macos --dylib
```

### 生成 Xcode 项目

```bash
# 为开发生成 Xcode 项目
ccgo build macos --ide-project

# 打开生成的项目
open cmake_build/macos/MyLib.xcodeproj
```

## 输出结构

### 默认输出 (`target/macos/`)

```
target/macos/
├── MyLib_macOS_SDK-1.0.0.zip            # 主包
│   ├── lib/
│   │   ├── static/
│   │   │   ├── libmylib.a               # 静态库（通用）
│   │   │   └── x86_64/                  # 架构专用（可选）
│   │   │       └── libmylib.a
│   │   └── shared/
│   │       ├── libmylib.dylib           # 动态库（通用）
│   │       └── arm64/                   # 架构专用（可选）
│   │           └── libmylib.dylib
│   ├── frameworks/
│   │   ├── static/
│   │   │   └── MyLib.framework/         # 静态 Framework
│   │   │       ├── MyLib                # 通用二进制
│   │   │       ├── Headers/             # 公共头文件
│   │   │       │   └── MyLib.h
│   │   │       ├── Modules/
│   │   │       │   └── module.modulemap
│   │   │       ├── Resources/           # 资源（如果有）
│   │   │       └── Info.plist
│   │   └── shared/
│   │       └── MyLib.framework/         # 动态 Framework
│   ├── include/
│   │   └── mylib/                       # 头文件
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                  # 构建元数据
│
└── MyLib_macOS_SDK-1.0.0-SYMBOLS.zip    # 调试符号
    └── symbols/
        ├── static/
        │   └── libmylib.a.dSYM/
        └── shared/
            └── libmylib.dylib.dSYM/
```

### 库类型

**静态库 (.a)：**
- 编译到可执行文件中
- 可执行文件体积更大
- 启动更快
- 无运行时依赖

**动态库 (.dylib)：**
- 运行时加载
- 可执行文件更小
- 可独立更新
- 需要库在运行时存在

**Framework：**
- 包含库、头文件和资源的捆绑包
- macOS 分发的首选
- 更好的 Xcode 集成
- 支持版本控制

### 通用二进制

通用二进制包含多个架构的代码：

```bash
# 检查二进制中的架构
lipo -info target/macos/lib/static/libmylib.a
# 输出：Architectures in the fat file: libmylib.a are: x86_64 arm64

# 提取特定架构
lipo target/macos/lib/static/libmylib.a -thin arm64 -output libmylib_arm64.a

# 从单独的架构创建通用二进制
lipo -create libmylib_x86_64.a libmylib_arm64.a -output libmylib_universal.a
```

### 构建元数据

`build_info.json` 包含：

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My macOS library"
  },
  "build": {
    "platform": "macos",
    "architectures": ["x86_64", "arm64"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "xcode_version": "15.0"
  },
  "outputs": {
    "libraries": [
      "lib/static/libmylib.a",
      "lib/shared/libmylib.dylib"
    ],
    "frameworks": [
      "frameworks/static/MyLib.framework",
      "frameworks/shared/MyLib.framework"
    ],
    "headers": "include/mylib/",
    "symbols": [
      "symbols/static/libmylib.a.dSYM",
      "symbols/shared/libmylib.dylib.dSYM"
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

1. 将 `MyLib.framework` 拖入 Xcode 项目
2. 选择"Copy items if needed"
3. 添加到"Frameworks, Libraries, and Embedded Content"
4. 对于动态框架，设置为"Embed & Sign"或"Do Not Embed"

**在 Swift 中导入：**

```swift
import MyLib

class MyApp {
    func run() {
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
  s.summary          = 'My macOS library'
  s.description      = 'A cross-platform C++ library for macOS'
  s.homepage         = 'https://github.com/myuser/mylib'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 'Your Name' => 'you@example.com' }
  s.source           = { :git => 'https://github.com/myuser/mylib.git', :tag => s.version.to_s }

  s.osx.deployment_target = '10.15'
  s.swift_version = '5.0'

  # Framework（推荐）
  s.vendored_frameworks = 'target/macos/frameworks/static/MyLib.framework'

  # 或 dylib
  # s.vendored_libraries = 'target/macos/lib/shared/libmylib.dylib'
  # s.source_files = 'include/**/*.h'

  # 依赖项
  s.dependency 'Alamofire', '~> 5.0'
end
```

### 在 macOS 项目中使用

**Podfile：**

```ruby
platform :osx, '10.15'
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
        .macOS(.v10_15)
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
            path: "target/macos/frameworks/static/MyLib.framework"
        )
    ]
)
```

### 在 macOS 项目中使用

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
ccgo build macos

# 指定签名身份
export CODE_SIGN_IDENTITY="Developer ID Application: Your Name (TEAM123456)"
ccgo build macos
```

### 手动签名

```bash
# 查找可用的身份
security find-identity -v -p codesigning

# 签名框架
codesign --force --sign "Developer ID Application" \
    --timestamp \
    --options runtime \
    target/macos/frameworks/shared/MyLib.framework

# 验证签名
codesign --verify --verbose target/macos/frameworks/shared/MyLib.framework

# 检查签名详情
codesign -dvv target/macos/frameworks/shared/MyLib.framework
```

### 分发签名

用于 Mac App Store 或直接分发：

```bash
# 使用分发证书签名
export CODE_SIGN_IDENTITY="3rd Party Mac Developer Application: Company (TEAM123)"
ccgo build macos --build-type release

# 用于直接分发（App Store 外）
export CODE_SIGN_IDENTITY="Developer ID Application: Company (TEAM123)"
ccgo build macos --build-type release
```

### 公证

macOS 10.15+ 在 App Store 外分发时需要：

```bash
# 构建和签名
ccgo build macos --build-type release

# 为公证创建归档
ditto -c -k --keepParent \
    target/macos/frameworks/shared/MyLib.framework \
    MyLib.zip

# 提交公证
xcrun notarytool submit MyLib.zip \
    --apple-id "you@example.com" \
    --team-id "TEAM123456" \
    --password "app-specific-password" \
    --wait

# 装订公证票据
xcrun stapler staple target/macos/frameworks/shared/MyLib.framework

# 验证公证
spctl -a -vv target/macos/frameworks/shared/MyLib.framework
```

### 加固运行时

公证所需：

```bash
# 启用加固运行时（CCGO 中自动）
codesign --force --sign "Developer ID Application" \
    --timestamp \
    --options runtime \
    target/macos/frameworks/shared/MyLib.framework
```

## Docker 构建

使用 Docker 和 OSXCross 在任何操作系统上构建 macOS 库：

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
ccgo build macos --docker

# 后续构建很快（无需下载）
ccgo build macos --docker --arch arm64

# 所有标准选项都可用
ccgo build macos --docker --framework --link-type static
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

- **无法运行**：Docker 中没有 macOS 运行时
- **无 Xcode**：无法打开生成的 Xcode 项目
- **构建体积大**：Docker 镜像约 2.5GB
- **首次运行慢**：初始镜像下载
- **无公证**：无法在 Docker 中公证

### Docker 镜像详情

镜像：`ccgo-builder-apple:latest`
- 基础：Ubuntu 22.04
- 工具链：OSXCross（Clang 15）
- SDK：macOS 13.0 SDK
- 支持：macOS、iOS、watchOS、tvOS
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

[macos]
deployment_target = "10.15"    # 最低 macOS 版本
enable_hardened_runtime = true # 加固运行时（公证需要）
frameworks = [                 # 要链接的系统框架
    "Foundation",
    "AppKit",
    "CoreGraphics"
]
```

### CMake 变量

为 macOS 构建时，这些变量可用：

```cmake
${PLATFORM}                    # "macos"
${ARCHITECTURE}                # "x86_64" 或 "arm64"
${BUILD_TYPE}                  # "Debug" 或 "Release"
${LINK_TYPE}                   # "static"、"shared" 或 "both"
${MACOS_DEPLOYMENT_TARGET}     # "10.15"（来自 CCGO.toml）
${CMAKE_OSX_SYSROOT}           # macOS SDK 路径
${CMAKE_OSX_ARCHITECTURES}     # "x86_64;arm64" 用于通用
```

### 条件编译

```cpp
// 在 C++ 代码中
#ifdef __APPLE__
#include <TargetConditionals.h>

#if TARGET_OS_MAC && !TARGET_OS_IPHONE
    // macOS 特定代码
    #import <AppKit/AppKit.h>
    NSApplication *app = [NSApplication sharedApplication];

#endif
#endif

// 架构特定
#ifdef __x86_64__
    // Intel 特定代码
#elif defined(__arm64__)
    // Apple Silicon 特定代码
#endif
```

## Mac Catalyst

构建在 Mac 上运行的 iOS 应用：

```bash
# 为 Catalyst 构建（需要先构建 iOS）
ccgo build ios --catalyst

# 或在 CCGO.toml 中指定
```

```toml
[ios]
enable_catalyst = true
catalyst_min_version = "14.0"
```

Catalyst 应用使用 iOS SDK 但在 macOS 上运行。

## 最佳实践

### 1. 构建通用二进制

支持 Intel 和 Apple Silicon：

```bash
# 总是为分发构建通用二进制
ccgo build macos --arch x86_64,arm64
```

**优势：**
- 所有 Mac 的单个二进制文件
- 更好的用户体验
- 为 Apple Silicon 过渡做好准备

### 2. 使用 Frameworks

Frameworks 是 macOS 的标准：

```bash
# 总是首选框架
ccgo build macos --framework
```

**优势：**
- 更好的 Xcode 集成
- 资源打包
- 版本控制支持
- 标准 macOS 分发

### 3. 启用加固运行时

公证所需：

```toml
[macos]
enable_hardened_runtime = true
```

### 4. 签名所有内容

总是签名动态库和框架：

```bash
# 分发构建需要适当的签名
export CODE_SIGN_IDENTITY="Developer ID Application: Company"
ccgo build macos --build-type release
```

### 5. 为分发公证

macOS 10.15+ 在 App Store 外需要：

```bash
# 构建、签名、公证
ccgo build macos --build-type release
# 然后提交公证（见上文）
```

### 6. 最小化依赖

保持库专注：

```toml
[dependencies]
# 仅必要的依赖项
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# 平台特定依赖项
[target.'cfg(target_os = "macos")'.dependencies]
macos-utils = { path = "./macos-utils" }
```

### 7. 在两种架构上测试

Intel 和 Apple Silicon 可能表现不同：

```bash
# 构建通用二进制
ccgo build macos --arch x86_64,arm64

# 如果可能在两种架构上测试
```

### 8. 调试符号

总是使用符号进行调试构建：

```bash
# 默认包含符号
ccgo build macos --build-type debug

# 符号在单独的包中
# MyLib_macOS_SDK-1.0.0-SYMBOLS.zip
```

## 高级主题

### @rpath 和安装名称

控制动态库加载：

```bash
# 检查安装名称
otool -D target/macos/lib/shared/libmylib.dylib

# 更改安装名称
install_name_tool -id "@rpath/libmylib.dylib" \
    target/macos/lib/shared/libmylib.dylib

# 向可执行文件添加 rpath
install_name_tool -add_rpath "@executable_path/../Frameworks" MyApp
```

### Framework 版本控制

支持多个版本：

```
MyLib.framework/
├── MyLib -> Versions/Current/MyLib
├── Headers -> Versions/Current/Headers
├── Resources -> Versions/Current/Resources
└── Versions/
    ├── A/
    │   ├── MyLib
    │   ├── Headers/
    │   └── Resources/
    └── Current -> A
```

### 最低操作系统版本

根据功能设置部署目标：

```toml
[macos]
deployment_target = "10.15"    # macOS Catalina（需要公证）
# deployment_target = "11.0"   # Big Sur（支持 Apple Silicon）
# deployment_target = "12.0"   # Monterey（M1 Pro/Max 支持）
# deployment_target = "13.0"   # Ventura（最新功能）
```

### 系统完整性保护（SIP）

安装在系统位置的库需要特殊处理：

```bash
# 检查 SIP 状态
csrutil status

# 库应使用 @rpath，而不是绝对路径
```

### 沙箱

用于 App Store 分发：

```bash
# 使用沙箱权限签名
codesign --force --sign "3rd Party Mac Developer Application" \
    --entitlements Sandbox.entitlements \
    target/macos/frameworks/shared/MyLib.framework
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
Error: Building for macOS, but linking in object file built for iOS
```

**解决方案：**

```bash
# 清理构建
ccgo clean -y

# 为特定架构构建
ccgo build macos --arch x86_64     # Intel
ccgo build macos --arch arm64      # Apple Silicon

# 或构建通用
ccgo build macos --arch x86_64,arm64
```

### 代码签名失败

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
export CODE_SIGN_IDENTITY="Developer ID Application: Name (TEAM123)"
```

3. 对于开发构建，使用 ad-hoc 签名：
```bash
export CODE_SIGN_IDENTITY="-"
```

### 找不到 dylib

```
dyld: Library not loaded: libmylib.dylib
```

**解决方案：**

1. 使用 @rpath：
```bash
install_name_tool -id "@rpath/libmylib.dylib" libmylib.dylib
```

2. 向可执行文件添加 rpath：
```bash
install_name_tool -add_rpath "@executable_path" MyApp
```

3. 设置 DYLD_LIBRARY_PATH（仅开发）：
```bash
export DYLD_LIBRARY_PATH=/path/to/libs:$DYLD_LIBRARY_PATH
```

### 公证失败

```
Error: Notarization failed
```

**解决方案：**

1. 确保加固运行时：
```bash
codesign -dvv --entitlements - MyLib.framework
```

2. 检查签名身份：
```bash
# 必须使用 Developer ID
codesign -dvv MyLib.framework | grep Authority
```

3. 验证所有嵌套代码都已签名：
```bash
codesign --verify --deep --strict --verbose=2 MyLib.framework
```

### Apple Silicon 问题

```
Error: Bad CPU type in executable
```

**解决方案：**

1. 构建通用二进制：
```bash
ccgo build macos --arch x86_64,arm64
```

2. 检查架构：
```bash
lipo -info MyLib.framework/MyLib
```

3. 使用 Rosetta 运行（Apple Silicon 上的 Intel 应用）：
```bash
arch -x86_64 ./MyApp
```

## 性能提示

### 1. 通用二进制

所有架构的单个二进制文件：

```bash
# 构建通用（稍大，但方便）
ccgo build macos --arch x86_64,arm64
```

### 2. 架构特定构建

针对特定架构优化：

```bash
# 仅 Apple Silicon（更小，更快）
ccgo build macos --arch arm64

# 仅 Intel
ccgo build macos --arch x86_64
```

### 3. 链接时优化

启用 LTO 以获得更好的性能：

```toml
[build]
cxxflags = ["-flto"]
ldflags = ["-flto"]
```

### 4. Framework vs dylib

Frameworks 有轻微开销：

```bash
# 对于性能关键，使用 dylib
ccgo build macos --dylib --link-type shared

# 对于分发，使用 framework
ccgo build macos --framework
```

### 5. 静态链接

最快启动，无动态加载：

```bash
# 静态框架
ccgo build macos --framework --link-type static
```

## 迁移指南

### 从手动 CMake

**之前（手动 CMake）：**

```bash
mkdir build-macos
cd build-macos
cmake .. \
    -DCMAKE_OSX_ARCHITECTURES="x86_64;arm64" \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=10.15 \
    -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

**之后（CCGO）：**

```bash
# 简单的一行命令
ccgo build macos
```

### 从 CocoaPods Podspec

**MyLib.podspec → CCGO.toml：**

```ruby
# 之前（Podspec）
Pod::Spec.new do |s|
  s.name = 'MyLib'
  s.version = '1.0.0'
  s.osx.deployment_target = '10.15'
  s.source_files = 'src/**/*.{cpp,h}'
end
```

```toml
# 之后（CCGO.toml）
[package]
name = "mylib"
version = "1.0.0"

[macos]
deployment_target = "10.15"
```

### 从 Xcode 项目

1. 创建 CCGO 项目：
```bash
ccgo new mylib
```

2. 复制源文件到 `src/`

3. 配置 CCGO.toml：
```toml
[macos]
deployment_target = "10.15"
frameworks = ["Foundation", "AppKit"]
```

4. 构建：
```bash
ccgo build macos
```

## 另请参阅

- [构建系统](../features/build-system.md)
- [依赖管理](../features/dependency-management.md)
- [发布管理](../features/publishing.md)
- [iOS 平台](ios.md)
- [CCGO.toml 参考](../reference/ccgo-toml.md)
- [平台概览](index.md)
