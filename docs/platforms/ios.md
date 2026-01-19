# iOS Platform

Complete guide to building C++ libraries for iOS with CCGO.

## Overview

CCGO provides comprehensive iOS support with:
- **Multiple architectures**: arm64, x86_64 (simulator)
- **Output formats**: Static/Dynamic Framework, XCFramework
- **Build methods**: Local (Xcode) or Docker (cross-platform)
- **Swift interop**: Easy integration with Swift code
- **Package managers**: CocoaPods and Swift Package Manager
- **Code signing**: Automatic handling of signing requirements
- **Bitcode**: Optional bitcode support (deprecated in Xcode 14+)

## Prerequisites

### Option 1: Local Build (macOS Required)

**Required:**
- macOS 12.0+ (Monterey or later)
- Xcode 13.0+ with Command Line Tools
- CMake 3.20+

**Installation:**

```bash
# Install Xcode from Mac App Store
# Then install Command Line Tools
xcode-select --install

# Verify installation
xcode-select -p
# Should output: /Applications/Xcode.app/Contents/Developer

# Install CMake (via Homebrew)
brew install cmake
```

### Option 2: Docker Build (Any OS)

No Xcode required! Build iOS libraries on Linux or Windows using Docker.

**Required:**
- Docker Desktop installed and running
- 10GB+ disk space for Docker image

**Advantages:**
- Build on any operating system
- No Xcode license required
- Consistent build environment
- Isolated from host system

**Limitations:**
- Cannot run/test iOS apps
- Larger initial download (~2.5GB image)
- Slower than native Xcode builds

See [Docker Builds](#docker-builds) section for details.

## Quick Start

### Basic Build

```bash
# Build for all iOS architectures (arm64 device + x86_64 simulator)
ccgo build ios

# Build with Docker (no Xcode needed)
ccgo build ios --docker

# Build specific architectures
ccgo build ios --arch arm64                    # Device only
ccgo build ios --arch x86_64                   # Simulator only
ccgo build ios --arch arm64,x86_64            # Both

# Build types
ccgo build ios --build-type debug             # Debug build
ccgo build ios --build-type release           # Release build (default)

# Link types
ccgo build ios --link-type static             # Static framework only
ccgo build ios --link-type shared             # Dynamic framework only
ccgo build ios --link-type both               # Both types (default)
```

### Build with XCFramework

XCFramework bundles multiple architectures into a single package:

```bash
# Build XCFramework (recommended)
ccgo build ios --xcframework

# Build with both Framework and XCFramework
ccgo build ios --xcframework --framework
```

### Generate Xcode Project

```bash
# Generate Xcode project for development
ccgo build ios --ide-project

# Open generated project
open cmake_build/ios/MyLib.xcodeproj
```

## Output Structure

### Default Output (`target/ios/`)

```
target/ios/
├── MyLib_iOS_SDK-1.0.0.zip          # Main package
│   ├── frameworks/
│   │   ├── static/
│   │   │   ├── MyLib.framework/     # Static Framework
│   │   │   │   ├── MyLib            # Fat binary (arm64 + x86_64)
│   │   │   │   ├── Headers/         # Public headers
│   │   │   │   │   └── MyLib.h
│   │   │   │   ├── Modules/
│   │   │   │   │   └── module.modulemap
│   │   │   │   └── Info.plist
│   │   │   └── MyLib.xcframework/   # XCFramework (if built)
│   │   │       ├── ios-arm64/
│   │   │       │   └── MyLib.framework/
│   │   │       ├── ios-arm64_x86_64-simulator/
│   │   │       │   └── MyLib.framework/
│   │   │       └── Info.plist
│   │   └── shared/
│   │       ├── MyLib.framework/     # Dynamic Framework
│   │       └── MyLib.xcframework/   # Dynamic XCFramework
│   ├── include/
│   │   └── mylib/                   # Header files
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json              # Build metadata
│
└── MyLib_iOS_SDK-1.0.0-SYMBOLS.zip  # Debug symbols
    └── symbols/
        ├── static/
        │   └── MyLib.framework.dSYM/
        └── shared/
            └── MyLib.framework.dSYM/
```

### Framework Structure

**Static Framework:**
- Contains `.a` static library
- Must be linked at compile time
- Smaller app size (dead code stripping)
- Easier distribution (no dynamic linking issues)

**Dynamic Framework:**
- Contains `.dylib` dynamic library
- Loaded at runtime
- Can be shared between app and extensions
- Requires code signing

**XCFramework:**
- Unified package for device and simulator
- Xcode automatically selects correct architecture
- Recommended for library distribution
- Supports multiple platforms (iOS, Catalyst, etc.)

### Build Metadata

`build_info.json` contains:

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

## Swift Integration

### Using Framework in Swift

**Add to Xcode Project:**

1. Drag `MyLib.framework` or `MyLib.xcframework` into Xcode project
2. Select "Copy items if needed"
3. Add to "Frameworks, Libraries, and Embedded Content"
4. For dynamic frameworks, set "Embed & Sign"

**Import in Swift:**

```swift
import MyLib

class MyViewController: UIViewController {
    override func viewDidLoad() {
        super.viewDidLoad()

        // Call C++ code through bridging
        let version = MyLib.getVersion()
        print("Library version: \(version)")

        // Create C++ object
        let lib = MyLibWrapper()
        lib.initialize()

        // Call methods
        let result = lib.processData("Hello from Swift")
        print("Result: \(result)")
    }
}
```

### C++/Swift Bridging

**Option 1: Objective-C++ Wrapper (Recommended)**

Create wrapper in your C++ library:

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

**Option 2: Pure Swift Wrapper (Swift 5.9+)**

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

Requires C interface in your library:

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

### Module Map

For Swift import to work, your framework needs a module map:

```
// module.modulemap
framework module MyLib {
    umbrella header "MyLib.h"
    export *
    module * { export * }
}
```

CCGO automatically generates this in your framework.

## CocoaPods Integration

### Publishing to CocoaPods

```bash
# Generate podspec
ccgo publish apple --manager cocoapods

# Validate podspec
pod spec lint MyLib.podspec

# Publish to CocoaPods Trunk
ccgo publish apple --manager cocoapods --push

# Publish to private spec repo
ccgo publish apple --manager cocoapods \
    --registry private \
    --remote-name myspecs \
    --url https://github.com/mycompany/specs.git
```

### Generated Podspec

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

  # XCFramework (recommended)
  s.vendored_frameworks = 'target/ios/frameworks/static/MyLib.xcframework'

  # Or regular framework
  # s.vendored_frameworks = 'target/ios/frameworks/static/MyLib.framework'

  # Dependencies
  s.dependency 'Alamofire', '~> 5.0'
end
```

### Using in iOS Project

**Podfile:**

```ruby
platform :ios, '12.0'
use_frameworks!

target 'MyApp' do
  pod 'MyLib', '~> 1.0'
end
```

**Install:**

```bash
pod install
open MyApp.xcworkspace
```

## Swift Package Manager Integration

### Publishing to SPM

```bash
# Generate Package.swift
ccgo publish apple --manager spm

# Push to Git (creates tag)
ccgo publish apple --manager spm --push
```

### Generated Package.swift

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

### Using in iOS Project

**Package.swift:**

```swift
dependencies: [
    .package(url: "https://github.com/myuser/mylib.git", from: "1.0.0")
]
```

**Or in Xcode:**

1. File → Add Packages...
2. Enter repository URL
3. Select version rule
4. Add to target

## Code Signing

### Automatic Signing

CCGO automatically handles code signing for frameworks:

```bash
# Sign with default identity
ccgo build ios

# Specify signing identity
export CODE_SIGN_IDENTITY="Apple Development: Your Name (TEAM123456)"
ccgo build ios
```

### Manual Signing

```bash
# Find available identities
security find-identity -v -p codesigning

# Sign framework
codesign --force --sign "Apple Development" \
    --timestamp \
    target/ios/frameworks/shared/MyLib.framework

# Verify signature
codesign --verify --verbose target/ios/frameworks/shared/MyLib.framework
```

### Distribution Signing

For App Store distribution:

```bash
# Sign with distribution certificate
export CODE_SIGN_IDENTITY="Apple Distribution: Company Name (TEAM123456)"
ccgo build ios --build-type release
```

### Troubleshooting Signing

```bash
# Check current signing
codesign -dvv target/ios/frameworks/shared/MyLib.framework

# Remove existing signature
codesign --remove-signature target/ios/frameworks/shared/MyLib.framework

# Sign with specific entitlements
codesign --force --sign "Apple Development" \
    --entitlements Entitlements.plist \
    target/ios/frameworks/shared/MyLib.framework
```

## Docker Builds

Build iOS libraries on any OS using Docker with OSXCross:

### Prerequisites

```bash
# Install Docker Desktop
# Download from: https://www.docker.com/products/docker-desktop/

# Verify Docker is running
docker ps
```

### Build with Docker

```bash
# First build downloads prebuilt image (~2.5GB)
ccgo build ios --docker

# Subsequent builds are fast (no download)
ccgo build ios --docker --arch arm64

# All standard options work
ccgo build ios --docker --xcframework --link-type static
```

### How It Works

1. CCGO uses prebuilt `ccgo-builder-apple` image from Docker Hub
2. Project directory mounted into container
3. Build runs inside container with OSXCross toolchain
4. Output written to host filesystem

### Advantages

- **Cross-platform**: Build on Linux, Windows, macOS
- **No Xcode**: Skip 40GB+ Xcode installation
- **Isolated**: Clean build environment
- **Reproducible**: Same results on any machine

### Limitations

- **Cannot run**: No iOS runtime in Docker
- **No simulator**: Cannot test in iOS Simulator
- **No Xcode**: Cannot open generated Xcode projects
- **Larger builds**: Docker image is ~2.5GB
- **Slower first run**: Initial image download

### Docker Image Details

Image: `ccgo-builder-apple:latest`
- Base: Ubuntu 22.04
- Toolchain: OSXCross (Clang 15)
- SDK: iOS 16.0 SDK
- Supported: iOS, macOS, watchOS, tvOS
- Size: ~2.5GB compressed

## Platform Configuration

### CCGO.toml Settings

```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "both"                  # static, shared, or both

[build]
cpp_standard = "17"            # C++ standard

[ios]
deployment_target = "12.0"     # Minimum iOS version
enable_bitcode = false         # Bitcode support (deprecated)
enable_arc = true              # Automatic Reference Counting
frameworks = [                 # System frameworks to link
    "Foundation",
    "UIKit",
    "CoreGraphics"
]
```

### CMake Variables

When building for iOS, these variables are available:

```cmake
${PLATFORM}                    # "ios"
${ARCHITECTURE}                # "arm64" or "x86_64"
${BUILD_TYPE}                  # "Debug" or "Release"
${LINK_TYPE}                   # "static", "shared", or "both"
${IOS_DEPLOYMENT_TARGET}       # "12.0" (from CCGO.toml)
${CMAKE_OSX_SYSROOT}           # Path to iOS SDK
${CMAKE_OSX_ARCHITECTURES}     # "arm64" or "x86_64"
```

### Conditional Compilation

```cpp
// In your C++ code
#ifdef __APPLE__
#include <TargetConditionals.h>

#if TARGET_OS_IOS
    // iOS-specific code
    #import <UIKit/UIKit.h>
    UIDevice *device = [UIDevice currentDevice];

#elif TARGET_OS_SIMULATOR
    // iOS Simulator-specific code

#endif
#endif
```

## Best Practices

### 1. Use XCFramework

XCFramework is the modern way to distribute iOS libraries:

```bash
# Always build XCFramework for distribution
ccgo build ios --xcframework
```

**Benefits:**
- Single package for device and simulator
- Xcode automatically selects architecture
- Supports multiple platforms
- Better Xcode integration

### 2. Version iOS SDK

Match your deployment target to user base:

```toml
[ios]
deployment_target = "12.0"  # Covers 95%+ of users
```

### 3. Static vs Dynamic

**Use Static when:**
- Distributing standalone library
- Want smaller app size
- Don't need shared code between app/extensions

**Use Dynamic when:**
- Sharing code between app and extensions
- Need runtime plugin loading
- Want faster incremental builds

### 4. Code Signing

Always sign dynamic frameworks:

```bash
# Distribution builds need proper signing
export CODE_SIGN_IDENTITY="Apple Distribution: Company"
ccgo build ios --build-type release
```

### 5. Minimize Dependencies

Keep your library focused:

```toml
[dependencies]
# Only essential dependencies
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Platform-specific dependencies
[target.'cfg(target_os = "ios")'.dependencies]
ios-utils = { path = "./ios-utils" }
```

### 6. Test on Device

Simulator and device behave differently:

```bash
# Build for both
ccgo build ios --arch arm64,x86_64

# Test on simulator (x86_64)
# Test on real device (arm64)
```

### 7. Debug Symbols

Always build with symbols for debugging:

```bash
# Symbols included by default
ccgo build ios --build-type debug

# Symbols in separate package
# MyLib_iOS_SDK-1.0.0-SYMBOLS.zip
```

## Advanced Topics

### Bitcode Support (Deprecated)

!!! warning "Deprecated"
    Bitcode is deprecated in Xcode 14+ and not recommended for new projects.

```toml
[ios]
enable_bitcode = false  # Keep disabled
```

### Universal Binary

Build a fat binary with multiple architectures:

```bash
# Build both architectures
ccgo build ios --arch arm64,x86_64

# Result is universal Framework
lipo -info target/ios/frameworks/static/MyLib.framework/MyLib
# Output: arm64 x86_64
```

### Minimum OS Version

Set deployment target based on features needed:

```toml
[ios]
deployment_target = "12.0"  # iOS 12+
# deployment_target = "13.0"  # For SwiftUI
# deployment_target = "14.0"  # For Widgets
# deployment_target = "15.0"  # For async/await
```

### Framework Resources

Include resources in your framework:

```
MyLib.framework/
├── MyLib
├── Headers/
├── Modules/
├── Resources/         # Add resources here
│   ├── images/
│   ├── configs/
│   └── Info.plist
└── Info.plist
```

### App Extensions

Build separately for app extensions:

```bash
# Main app framework
ccgo build ios

# Extension framework (limited APIs)
export IOS_APP_EXTENSION=1
ccgo build ios
```

## Troubleshooting

### Xcode Not Found

```
Error: Could not find Xcode installation
```

**Solution:**

```bash
# Install Xcode from App Store
# Install Command Line Tools
xcode-select --install

# Set Xcode path
sudo xcode-select --switch /Applications/Xcode.app

# Verify
xcode-select -p
```

### Architecture Mismatch

```
Error: Building for iOS, but linking in object file built for iOS Simulator
```

**Solution:**

```bash
# Clean build
ccgo clean -y

# Build for specific architecture
ccgo build ios --arch arm64        # Device
ccgo build ios --arch x86_64       # Simulator

# Or build both
ccgo build ios --arch arm64,x86_64
```

### Signing Failed

```
Error: Code signing failed
```

**Solutions:**

1. Check available identities:
```bash
security find-identity -v -p codesigning
```

2. Set correct identity:
```bash
export CODE_SIGN_IDENTITY="Apple Development: Name (TEAM123)"
```

3. For development builds, use ad-hoc signing:
```bash
export CODE_SIGN_IDENTITY="-"
```

### Module Not Found

```
Error: No such module 'MyLib'
```

**Solutions:**

1. Ensure framework is added to target
2. Check module.modulemap exists in framework
3. Add framework search path in Xcode:
   - Build Settings → Framework Search Paths
   - Add path to framework directory

4. For Swift Package Manager:
```swift
// Ensure target depends on MyLib
.target(
    name: "MyApp",
    dependencies: ["MyLib"]
)
```

### Dynamic Framework Not Loaded

```
dyld: Library not loaded: @rpath/MyLib.framework/MyLib
```

**Solutions:**

1. Embed framework in app:
   - Xcode → Target → General
   - Frameworks, Libraries, and Embedded Content
   - Set to "Embed & Sign"

2. Check Runpath Search Paths:
   - Build Settings → Runpath Search Paths
   - Should include `@executable_path/Frameworks`

### Build Extremely Slow

```
Build taking very long time...
```

**Solutions:**

1. Use incremental builds:
```bash
# Don't clean between builds
ccgo build ios
```

2. Build specific architecture:
```bash
# Build only what you need
ccgo build ios --arch arm64
```

3. Use Docker for clean environment:
```bash
# Sometimes native build is slow
ccgo build ios --docker
```

4. Check Xcode indexing:
```bash
# Disable Xcode indexing
defaults write com.apple.dt.Xcode IDEIndexDisable -bool YES
```

### Docker Build Fails

```
Error: Docker daemon not running
```

**Solutions:**

1. Start Docker Desktop
2. Wait for Docker to fully start
3. Verify: `docker ps`

```
Error: Cannot pull Docker image
```

**Solutions:**

1. Check internet connection
2. Check Docker Hub status
3. Try with explicit pull:
```bash
docker pull ccgo-builder-apple:latest
```

## Performance Tips

### 1. Cache Dependencies

```bash
# Dependencies cached after first build
# ~/.ccgo/git/<repo>/
```

### 2. Incremental Builds

```bash
# Don't clean between small changes
ccgo build ios              # Fast incremental

# Only clean when needed
ccgo clean -y
ccgo build ios              # Full rebuild
```

### 3. Architecture Selection

```bash
# Build only what you need
ccgo build ios --arch arm64              # Device only (fast)
ccgo build ios --arch x86_64             # Simulator only (fast)
ccgo build ios --arch arm64,x86_64      # Both (slower)
```

### 4. Link Type

```bash
# Static builds are faster
ccgo build ios --link-type static

# Dynamic needs signing
ccgo build ios --link-type shared       # Slower
```

### 5. Parallel Builds

CMake automatically uses parallel builds:

```bash
# Uses all CPU cores by default
ccgo build ios

# Limit parallelism if needed
export CMAKE_BUILD_PARALLEL_LEVEL=4
ccgo build ios
```

## Migration Guides

### From Manual CMake

**Before (manual CMake):**

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

**After (CCGO):**

```bash
# Simple one-liner
ccgo build ios
```

### From CocoaPods Podspec

**MyLib.podspec → CCGO.toml:**

```ruby
# Before (Podspec)
Pod::Spec.new do |s|
  s.name = 'MyLib'
  s.version = '1.0.0'
  s.ios.deployment_target = '12.0'
  s.source_files = 'src/**/*.{cpp,h}'
  s.dependency 'Alamofire'
end
```

```toml
# After (CCGO.toml)
[package]
name = "mylib"
version = "1.0.0"

[ios]
deployment_target = "12.0"

[dependencies]
alamofire = { git = "https://github.com/Alamofire/Alamofire.git", tag = "5.8.0" }
```

### From Xcode Project

1. Create CCGO project:
```bash
ccgo new mylib
```

2. Copy source files to `src/`

3. Configure CCGO.toml:
```toml
[ios]
deployment_target = "12.0"
frameworks = ["Foundation", "UIKit"]
```

4. Build:
```bash
ccgo build ios
```

## See Also

- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Publishing](../features/publishing.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Platforms Overview](index.md)
