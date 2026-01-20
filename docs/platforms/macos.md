# macOS Platform

Complete guide to building C++ libraries for macOS with CCGO.

## Overview

CCGO provides comprehensive macOS support with:

- **Multiple architectures**: x86_64 (Intel), arm64 (Apple Silicon)
- **Universal binaries**: Fat binaries containing both architectures
- **Output formats**: Static/Dynamic Framework, dylib
- **Build methods**: Local (Xcode) or Docker (cross-platform)
- **Swift interop**: Easy integration with Swift code
- **Package managers**: CocoaPods and Swift Package Manager
- **Code signing**: Automatic handling and notarization support
- **Mac Catalyst**: Support for iPad apps on Mac

## Prerequisites

### Option 1: Local Build (macOS Required)

**Required:**
- macOS 10.15+ (Catalina or later)
- Xcode 12.0+ with Command Line Tools
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

No Xcode required! Build macOS libraries on Linux or Windows using Docker.

**Required:**
- Docker Desktop installed and running
- 10GB+ disk space for Docker image

**Advantages:**
- Build on any operating system
- No Xcode license required
- Consistent build environment
- Isolated from host system

**Limitations:**
- Cannot run/test macOS apps
- Larger initial download (~2.5GB image)
- Slower than native Xcode builds

See [Docker Builds](#docker-builds) section for details.

## Quick Start

### Basic Build

```bash
# Build for all macOS architectures (x86_64 + arm64 universal binary)
ccgo build macos

# Build with Docker (no Xcode needed)
ccgo build macos --docker

# Build specific architectures
ccgo build macos --arch x86_64                 # Intel only
ccgo build macos --arch arm64                  # Apple Silicon only
ccgo build macos --arch x86_64,arm64          # Universal binary (default)

# Build types
ccgo build macos --build-type debug           # Debug build
ccgo build macos --build-type release         # Release build (default)

# Link types
ccgo build macos --link-type static           # Static library/framework only
ccgo build macos --link-type shared           # Dynamic library/framework only
ccgo build macos --link-type both             # Both types (default)
```

### Build with Framework

```bash
# Build Framework (recommended)
ccgo build macos --framework

# Build dylib (traditional)
ccgo build macos --dylib
```

### Generate Xcode Project

```bash
# Generate Xcode project for development
ccgo build macos --ide-project

# Open generated project
open cmake_build/macos/MyLib.xcodeproj
```

## Output Structure

### Default Output (`target/macos/`)

```
target/macos/
├── MyLib_macOS_SDK-1.0.0.zip            # Main package
│   ├── lib/
│   │   ├── static/
│   │   │   ├── libmylib.a               # Static library (universal)
│   │   │   └── x86_64/                  # Architecture-specific (optional)
│   │   │       └── libmylib.a
│   │   └── shared/
│   │       ├── libmylib.dylib           # Dynamic library (universal)
│   │       └── arm64/                   # Architecture-specific (optional)
│   │           └── libmylib.dylib
│   ├── frameworks/
│   │   ├── static/
│   │   │   └── MyLib.framework/         # Static Framework
│   │   │       ├── MyLib                # Universal binary
│   │   │       ├── Headers/             # Public headers
│   │   │       │   └── MyLib.h
│   │   │       ├── Modules/
│   │   │       │   └── module.modulemap
│   │   │       ├── Resources/           # Resources (if any)
│   │   │       └── Info.plist
│   │   └── shared/
│   │       └── MyLib.framework/         # Dynamic Framework
│   ├── include/
│   │   └── mylib/                       # Header files
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                  # Build metadata
│
└── MyLib_macOS_SDK-1.0.0-SYMBOLS.zip    # Debug symbols
    └── symbols/
        ├── static/
        │   └── libmylib.a.dSYM/
        └── shared/
            └── libmylib.dylib.dSYM/
```

### Library Types

**Static library (.a):**
- Compiled into executable
- Larger executable size
- Faster startup
- No runtime dependencies

**Dynamic library (.dylib):**
- Loaded at runtime
- Smaller executable
- Can be updated independently
- Requires library at runtime

**Framework:**
- Bundle containing library, headers, and resources
- Preferred for macOS distribution
- Better Xcode integration
- Support for versioning

### Universal Binaries

Universal binaries contain code for multiple architectures:

```bash
# Check architectures in binary
lipo -info target/macos/lib/static/libmylib.a
# Output: Architectures in the fat file: libmylib.a are: x86_64 arm64

# Extract specific architecture
lipo target/macos/lib/static/libmylib.a -thin arm64 -output libmylib_arm64.a

# Create universal binary from separate architectures
lipo -create libmylib_x86_64.a libmylib_arm64.a -output libmylib_universal.a
```

### Build Metadata

`build_info.json` contains:

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

## Swift Integration

### Using Framework in Swift

**Add to Xcode Project:**

1. Drag `MyLib.framework` into Xcode project
2. Select "Copy items if needed"
3. Add to "Frameworks, Libraries, and Embedded Content"
4. For dynamic frameworks, set "Embed & Sign" or "Do Not Embed"

**Import in Swift:**

```swift
import MyLib

class MyApp {
    func run() {
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
  s.summary          = 'My macOS library'
  s.description      = 'A cross-platform C++ library for macOS'
  s.homepage         = 'https://github.com/myuser/mylib'
  s.license          = { :type => 'MIT', :file => 'LICENSE' }
  s.author           = { 'Your Name' => 'you@example.com' }
  s.source           = { :git => 'https://github.com/myuser/mylib.git', :tag => s.version.to_s }

  s.osx.deployment_target = '10.15'
  s.swift_version = '5.0'

  # Framework (recommended)
  s.vendored_frameworks = 'target/macos/frameworks/static/MyLib.framework'

  # Or dylib
  # s.vendored_libraries = 'target/macos/lib/shared/libmylib.dylib'
  # s.source_files = 'include/**/*.h'

  # Dependencies
  s.dependency 'Alamofire', '~> 5.0'
end
```

### Using in macOS Project

**Podfile:**

```ruby
platform :osx, '10.15'
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

### Using in macOS Project

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
ccgo build macos

# Specify signing identity
export CODE_SIGN_IDENTITY="Developer ID Application: Your Name (TEAM123456)"
ccgo build macos
```

### Manual Signing

```bash
# Find available identities
security find-identity -v -p codesigning

# Sign framework
codesign --force --sign "Developer ID Application" \
    --timestamp \
    --options runtime \
    target/macos/frameworks/shared/MyLib.framework

# Verify signature
codesign --verify --verbose target/macos/frameworks/shared/MyLib.framework

# Check signature details
codesign -dvv target/macos/frameworks/shared/MyLib.framework
```

### Distribution Signing

For Mac App Store or direct distribution:

```bash
# Sign with distribution certificate
export CODE_SIGN_IDENTITY="3rd Party Mac Developer Application: Company (TEAM123)"
ccgo build macos --build-type release

# For direct distribution (outside App Store)
export CODE_SIGN_IDENTITY="Developer ID Application: Company (TEAM123)"
ccgo build macos --build-type release
```

### Notarization

Required for macOS 10.15+ distribution outside App Store:

```bash
# Build and sign
ccgo build macos --build-type release

# Create archive for notarization
ditto -c -k --keepParent \
    target/macos/frameworks/shared/MyLib.framework \
    MyLib.zip

# Submit for notarization
xcrun notarytool submit MyLib.zip \
    --apple-id "you@example.com" \
    --team-id "TEAM123456" \
    --password "app-specific-password" \
    --wait

# Staple notarization ticket
xcrun stapler staple target/macos/frameworks/shared/MyLib.framework

# Verify notarization
spctl -a -vv target/macos/frameworks/shared/MyLib.framework
```

### Hardened Runtime

Required for notarization:

```bash
# Enable hardened runtime (automatic in CCGO)
codesign --force --sign "Developer ID Application" \
    --timestamp \
    --options runtime \
    target/macos/frameworks/shared/MyLib.framework
```

## Docker Builds

Build macOS libraries on any OS using Docker with OSXCross:

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
ccgo build macos --docker

# Subsequent builds are fast (no download)
ccgo build macos --docker --arch arm64

# All standard options work
ccgo build macos --docker --framework --link-type static
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

- **Cannot run**: No macOS runtime in Docker
- **No Xcode**: Cannot open generated Xcode projects
- **Larger builds**: Docker image is ~2.5GB
- **Slower first run**: Initial image download
- **No notarization**: Cannot notarize in Docker

### Docker Image Details

Image: `ccgo-builder-apple:latest`
- Base: Ubuntu 22.04
- Toolchain: OSXCross (Clang 15)
- SDK: macOS 13.0 SDK
- Supported: macOS, iOS, watchOS, tvOS
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

[macos]
deployment_target = "10.15"    # Minimum macOS version
enable_hardened_runtime = true # Hardened runtime (required for notarization)
frameworks = [                 # System frameworks to link
    "Foundation",
    "AppKit",
    "CoreGraphics"
]
```

### CMake Variables

When building for macOS, these variables are available:

```cmake
${PLATFORM}                    # "macos"
${ARCHITECTURE}                # "x86_64" or "arm64"
${BUILD_TYPE}                  # "Debug" or "Release"
${LINK_TYPE}                   # "static", "shared", or "both"
${MACOS_DEPLOYMENT_TARGET}     # "10.15" (from CCGO.toml)
${CMAKE_OSX_SYSROOT}           # Path to macOS SDK
${CMAKE_OSX_ARCHITECTURES}     # "x86_64;arm64" for universal
```

### Conditional Compilation

```cpp
// In your C++ code
#ifdef __APPLE__
#include <TargetConditionals.h>

#if TARGET_OS_MAC && !TARGET_OS_IPHONE
    // macOS-specific code
    #import <AppKit/AppKit.h>
    NSApplication *app = [NSApplication sharedApplication];

#endif
#endif

// Architecture-specific
#ifdef __x86_64__
    // Intel-specific code
#elif defined(__arm64__)
    // Apple Silicon-specific code
#endif
```

## Mac Catalyst

Build iOS apps that run on macOS:

```bash
# Build for Catalyst (requires iOS build first)
ccgo build ios --catalyst

# Or specify in CCGO.toml
```

```toml
[ios]
enable_catalyst = true
catalyst_min_version = "14.0"
```

Catalyst apps use iOS SDK but run on macOS.

## Best Practices

### 1. Build Universal Binaries

Support both Intel and Apple Silicon:

```bash
# Always build universal binaries for distribution
ccgo build macos --arch x86_64,arm64
```

**Benefits:**
- Single binary for all Macs
- Better user experience
- Future-proof for Apple Silicon transition

### 2. Use Frameworks

Frameworks are the standard for macOS:

```bash
# Always prefer frameworks
ccgo build macos --framework
```

**Benefits:**
- Better Xcode integration
- Resource bundling
- Versioning support
- Standard macOS distribution

### 3. Enable Hardened Runtime

Required for notarization:

```toml
[macos]
enable_hardened_runtime = true
```

### 4. Code Sign Everything

Always sign dynamic libraries and frameworks:

```bash
# Distribution builds need proper signing
export CODE_SIGN_IDENTITY="Developer ID Application: Company"
ccgo build macos --build-type release
```

### 5. Notarize for Distribution

Required for macOS 10.15+ outside App Store:

```bash
# Build, sign, notarize
ccgo build macos --build-type release
# Then submit for notarization (see above)
```

### 6. Minimize Dependencies

Keep your library focused:

```toml
[dependencies]
# Only essential dependencies
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Platform-specific dependencies
[target.'cfg(target_os = "macos")'.dependencies]
macos-utils = { path = "./macos-utils" }
```

### 7. Test on Both Architectures

Intel and Apple Silicon may behave differently:

```bash
# Build universal binary
ccgo build macos --arch x86_64,arm64

# Test on both architectures if possible
```

### 8. Debug Symbols

Always build with symbols for debugging:

```bash
# Symbols included by default
ccgo build macos --build-type debug

# Symbols in separate package
# MyLib_macOS_SDK-1.0.0-SYMBOLS.zip
```

## Advanced Topics

### @rpath and Install Name

Control dynamic library loading:

```bash
# Check install name
otool -D target/macos/lib/shared/libmylib.dylib

# Change install name
install_name_tool -id "@rpath/libmylib.dylib" \
    target/macos/lib/shared/libmylib.dylib

# Add rpath to executable
install_name_tool -add_rpath "@executable_path/../Frameworks" MyApp
```

### Framework Versioning

Support multiple versions:

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

### Minimum OS Version

Set deployment target based on features:

```toml
[macos]
deployment_target = "10.15"    # macOS Catalina (notarization required)
# deployment_target = "11.0"   # Big Sur (supports Apple Silicon)
# deployment_target = "12.0"   # Monterey (M1 Pro/Max support)
# deployment_target = "13.0"   # Ventura (latest features)
```

### System Integrity Protection (SIP)

Libraries installed in system locations need special handling:

```bash
# Check SIP status
csrutil status

# Libraries should use @rpath, not absolute paths
```

### Sandboxing

For App Store distribution:

```bash
# Sign with sandbox entitlements
codesign --force --sign "3rd Party Mac Developer Application" \
    --entitlements Sandbox.entitlements \
    target/macos/frameworks/shared/MyLib.framework
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
Error: Building for macOS, but linking in object file built for iOS
```

**Solution:**

```bash
# Clean build
ccgo clean -y

# Build for specific architecture
ccgo build macos --arch x86_64     # Intel
ccgo build macos --arch arm64      # Apple Silicon

# Or build universal
ccgo build macos --arch x86_64,arm64
```

### Code Signing Failed

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
export CODE_SIGN_IDENTITY="Developer ID Application: Name (TEAM123)"
```

3. For development builds, use ad-hoc signing:
```bash
export CODE_SIGN_IDENTITY="-"
```

### dylib Not Found

```
dyld: Library not loaded: libmylib.dylib
```

**Solutions:**

1. Use @rpath:
```bash
install_name_tool -id "@rpath/libmylib.dylib" libmylib.dylib
```

2. Add rpath to executable:
```bash
install_name_tool -add_rpath "@executable_path" MyApp
```

3. Set DYLD_LIBRARY_PATH (development only):
```bash
export DYLD_LIBRARY_PATH=/path/to/libs:$DYLD_LIBRARY_PATH
```

### Notarization Failed

```
Error: Notarization failed
```

**Solutions:**

1. Ensure hardened runtime:
```bash
codesign -dvv --entitlements - MyLib.framework
```

2. Check signing identity:
```bash
# Must use Developer ID
codesign -dvv MyLib.framework | grep Authority
```

3. Verify all nested code is signed:
```bash
codesign --verify --deep --strict --verbose=2 MyLib.framework
```

### Apple Silicon Issues

```
Error: Bad CPU type in executable
```

**Solutions:**

1. Build universal binary:
```bash
ccgo build macos --arch x86_64,arm64
```

2. Check architectures:
```bash
lipo -info MyLib.framework/MyLib
```

3. Run with Rosetta (Intel apps on Apple Silicon):
```bash
arch -x86_64 ./MyApp
```

## Performance Tips

### 1. Universal Binaries

Single binary for all architectures:

```bash
# Build universal (slightly larger, but convenient)
ccgo build macos --arch x86_64,arm64
```

### 2. Architecture-Specific Builds

Optimize for specific architecture:

```bash
# Apple Silicon only (smaller, faster)
ccgo build macos --arch arm64

# Intel only
ccgo build macos --arch x86_64
```

### 3. Link-Time Optimization

Enable LTO for better performance:

```toml
[build]
cxxflags = ["-flto"]
ldflags = ["-flto"]
```

### 4. Framework vs dylib

Frameworks have slight overhead:

```bash
# For performance-critical, use dylib
ccgo build macos --dylib --link-type shared

# For distribution, use framework
ccgo build macos --framework
```

### 5. Static Linking

Fastest startup, no dynamic loading:

```bash
# Static framework
ccgo build macos --framework --link-type static
```

## Migration Guides

### From Manual CMake

**Before (manual CMake):**

```bash
mkdir build-macos
cd build-macos
cmake .. \
    -DCMAKE_OSX_ARCHITECTURES="x86_64;arm64" \
    -DCMAKE_OSX_DEPLOYMENT_TARGET=10.15 \
    -DCMAKE_BUILD_TYPE=Release
cmake --build . --config Release
```

**After (CCGO):**

```bash
# Simple one-liner
ccgo build macos
```

### From CocoaPods Podspec

**MyLib.podspec → CCGO.toml:**

```ruby
# Before (Podspec)
Pod::Spec.new do |s|
  s.name = 'MyLib'
  s.version = '1.0.0'
  s.osx.deployment_target = '10.15'
  s.source_files = 'src/**/*.{cpp,h}'
end
```

```toml
# After (CCGO.toml)
[package]
name = "mylib"
version = "1.0.0"

[macos]
deployment_target = "10.15"
```

### From Xcode Project

1. Create CCGO project:
```bash
ccgo new mylib
```

2. Copy source files to `src/`

3. Configure CCGO.toml:
```toml
[macos]
deployment_target = "10.15"
frameworks = ["Foundation", "AppKit"]
```

4. Build:
```bash
ccgo build macos
```

## See Also

- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Publishing](../features/publishing.md)
- [iOS Platform](ios.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Platforms Overview](index.md)
