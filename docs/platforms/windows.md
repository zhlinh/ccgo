# Windows Platform

Complete guide to building C++ libraries for Windows with CCGO.

## Overview

CCGO provides comprehensive Windows support with:

- **Multiple toolchains**: MSVC (Visual Studio) and MinGW (GCC)
- **Multiple architectures**: x86, x64, ARM64
- **Output formats**: Static libraries (.lib), Dynamic libraries (.dll)
- **Build methods**: Local (Visual Studio/MinGW) or Docker (cross-platform)
- **IDE support**: Visual Studio project generation
- **Subsystems**: Console and Windows subsystems
- **Runtime libraries**: Static and dynamic CRT linking

## Prerequisites

### Option 1: Local Build (Windows Required)

**For MSVC:**
- Windows 10+ (64-bit)
- Visual Studio 2019+ with C++ workload
- CMake 3.20+

**Installation:**

```powershell
# Install Visual Studio from visualstudio.microsoft.com
# Select "Desktop development with C++" workload

# Install CMake
# Download from cmake.org or use chocolatey
choco install cmake

# Verify installation
cmake --version
cl.exe
```

**For MinGW:**
- Windows 10+ (64-bit)
- MinGW-w64 or MSYS2
- CMake 3.20+

**Installation:**

```powershell
# Install MSYS2 from msys2.org
# Then install MinGW-w64
pacman -S mingw-w64-x86_64-gcc mingw-w64-x86_64-cmake

# Add to PATH
# C:\msys64\mingw64\bin

# Verify
gcc --version
g++ --version
```

### Option 2: Docker Build (Any OS)

Build Windows libraries on Linux or macOS using Docker with MinGW.

**Required:**
- Docker Desktop installed and running
- 5GB+ disk space for Docker image

**Advantages:**
- Build on any operating system
- No Windows license required
- Consistent build environment
- MinGW-w64 cross-compilation

**Limitations:**
- MinGW only (no MSVC)
- Cannot run/test Windows apps
- Larger initial download (~1.2GB image)

See [Docker Builds](#docker-builds) section for details.

## Quick Start

### Basic Build

```bash
# Build for x64 with default toolchain
ccgo build windows

# Build with Docker (MinGW cross-compile)
ccgo build windows --docker

# Specify toolchain
ccgo build windows --toolchain msvc      # MSVC (Windows only)
ccgo build windows --toolchain mingw     # MinGW
ccgo build windows --toolchain auto      # Both toolchains (default)

# Specify architecture
ccgo build windows --arch x86            # 32-bit
ccgo build windows --arch x64            # 64-bit (default)
ccgo build windows --arch arm64          # ARM64

# Build types
ccgo build windows --build-type debug    # Debug build
ccgo build windows --build-type release  # Release build (default)

# Link types
ccgo build windows --link-type static    # Static library only
ccgo build windows --link-type shared    # DLL only
ccgo build windows --link-type both      # Both types (default)
```

### Generate Visual Studio Project

```bash
# Generate Visual Studio solution
ccgo build windows --ide-project

# Open in Visual Studio
start cmake_build/windows/msvc/MyLib.sln
```

## Output Structure

### Default Output (`target/windows/`)

```
target/windows/
├── MyLib_Windows_SDK-1.0.0.zip          # Main package
│   ├── lib/
│   │   ├── static/
│   │   │   ├── msvc/
│   │   │   │   └── mylib.lib            # MSVC static library
│   │   │   └── mingw/
│   │   │       └── libmylib.a           # MinGW static library
│   │   └── shared/
│   │       ├── msvc/
│   │       │   ├── mylib.dll            # MSVC DLL
│   │       │   └── mylib.lib            # Import library
│   │       └── mingw/
│   │           ├── libmylib.dll         # MinGW DLL
│   │           └── libmylib.dll.a       # Import library
│   ├── bin/                             # DLLs (for runtime)
│   │   ├── msvc/
│   │   │   └── mylib.dll
│   │   └── mingw/
│   │       └── libmylib.dll
│   ├── include/
│   │   └── mylib/                       # Header files
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                  # Build metadata
│
└── MyLib_Windows_SDK-1.0.0-SYMBOLS.zip  # Debug symbols
    └── symbols/
        ├── msvc/
        │   └── mylib.pdb                # MSVC debug symbols
        └── mingw/
            └── libmylib.dll.debug       # MinGW debug symbols
```

### Library Types

**Static library:**
- MSVC: `.lib` file
- MinGW: `.a` file
- Linked at compile time
- Larger executable
- No runtime dependencies

**Dynamic library (DLL):**
- MSVC: `.dll` + `.lib` (import library)
- MinGW: `.dll` + `.dll.a` (import library)
- Loaded at runtime
- Smaller executable
- Requires DLL at runtime

### Build Metadata

`build_info.json` contains:

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My Windows library"
  },
  "build": {
    "platform": "windows",
    "architectures": ["x64"],
    "toolchains": ["msvc", "mingw"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "msvc_version": "19.38",
    "mingw_version": "13.2.0"
  },
  "outputs": {
    "libraries": {
      "msvc": {
        "static": "lib/static/msvc/mylib.lib",
        "shared": "lib/shared/msvc/mylib.dll"
      },
      "mingw": {
        "static": "lib/static/mingw/libmylib.a",
        "shared": "lib/shared/mingw/libmylib.dll"
      }
    },
    "headers": "include/mylib/",
    "symbols": {
      "msvc": "symbols/msvc/mylib.pdb",
      "mingw": "symbols/mingw/libmylib.dll.debug"
    }
  }
}
```

## MSVC vs MinGW

### MSVC (Microsoft Visual C++)

**Pros:**
- Official Microsoft compiler
- Best Windows integration
- Excellent debugging with Visual Studio
- Better optimization for Windows
- Compatible with Windows SDK

**Cons:**
- Windows-only
- Requires Visual Studio installation
- Larger toolchain

**When to use:**
- Windows-specific development
- Need Visual Studio integration
- Maximum Windows performance
- Using Windows SDK APIs

### MinGW (Minimalist GNU for Windows)

**Pros:**
- GCC-based (cross-platform compatible)
- Can cross-compile from Linux/macOS
- Smaller toolchain
- Open source
- Compatible with Unix tools

**Cons:**
- Some Windows APIs not fully supported
- Potentially slower than MSVC
- Less Windows-specific optimization

**When to use:**
- Cross-platform development
- Building on non-Windows systems
- Want GCC compatibility
- Don't need advanced Windows APIs

## Using Libraries in C++

### Linking Static Library

**CMakeLists.txt (MSVC):**

```cmake
# Find library
find_library(MYLIB_LIBRARY
    NAMES mylib
    PATHS "path/to/lib/static/msvc"
)

# Link to target
target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "path/to/include")
```

**CMakeLists.txt (MinGW):**

```cmake
find_library(MYLIB_LIBRARY
    NAMES mylib libmylib.a
    PATHS "path/to/lib/static/mingw"
)

target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "path/to/include")
```

### Linking Dynamic Library

**CMakeLists.txt:**

```cmake
# Link import library
target_link_libraries(myapp PRIVATE "path/to/lib/shared/msvc/mylib.lib")

# Copy DLL to output directory
add_custom_command(TARGET myapp POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "path/to/bin/msvc/mylib.dll"
        $<TARGET_FILE_DIR:myapp>
)
```

**Using in code:**

```cpp
#include <mylib/mylib.h>

int main() {
    // DLL functions are automatically resolved
    mylib::MyClass obj;
    obj.do_work();
    return 0;
}
```

## Docker Builds

Build Windows libraries on any OS using Docker with MinGW:

### Prerequisites

```bash
# Install Docker Desktop
# Download from: https://www.docker.com/products/docker-desktop/

# Verify Docker is running
docker ps
```

### Build with Docker

```bash
# First build downloads prebuilt image (~1.2GB)
ccgo build windows --docker

# Subsequent builds are fast
ccgo build windows --docker --arch x64

# All standard options work (MinGW only)
ccgo build windows --docker --link-type static
```

### How It Works

1. CCGO uses prebuilt `ccgo-builder-windows` image from Docker Hub
2. Project directory mounted into container
3. Build runs with MinGW-w64 cross-compiler
4. Output written to host filesystem

### Limitations

- **MinGW only**: Cannot build with MSVC in Docker
- **Cannot run**: No Windows runtime in Docker
- **Cannot test**: Cannot execute Windows binaries

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

[windows]
subsystem = "console"          # console or windows
runtime_library = "MD"         # MT, MD, MTd, MDd (MSVC only)
windows_sdk_version = "10.0"   # Windows SDK version
```

### CMake Variables

When building for Windows:

```cmake
${PLATFORM}                    # "windows"
${ARCHITECTURE}                # "x86", "x64", or "arm64"
${BUILD_TYPE}                  # "Debug" or "Release"
${LINK_TYPE}                   # "static", "shared", or "both"
${TOOLCHAIN}                   # "msvc" or "mingw"
${WINDOWS_SUBSYSTEM}           # "console" or "windows"
${MSVC_RUNTIME_LIBRARY}        # "MD", "MT", etc. (MSVC only)
```

### Conditional Compilation

```cpp
// Platform detection
#ifdef _WIN32
    // Windows-specific code
    #include <windows.h>

    #ifdef _WIN64
        // 64-bit Windows
    #else
        // 32-bit Windows
    #endif
#endif

// Compiler detection
#ifdef _MSC_VER
    // MSVC-specific code
    #pragma warning(disable: 4996)
#elif defined(__MINGW32__) || defined(__MINGW64__)
    // MinGW-specific code
#endif

// DLL export/import
#ifdef MYLIB_EXPORTS
    #define MYLIB_API __declspec(dllexport)
#else
    #define MYLIB_API __declspec(dllimport)
#endif

// Usage
class MYLIB_API MyClass {
public:
    void do_work();
};
```

## Best Practices

### 1. Support Both Toolchains

Build with both MSVC and MinGW:

```bash
# Build both (default)
ccgo build windows --toolchain auto
```

### 2. Use Proper DLL Export

Always use `__declspec(dllexport/dllimport)`:

```cpp
// mylib_export.h
#ifdef _WIN32
    #ifdef MYLIB_EXPORTS
        #define MYLIB_API __declspec(dllexport)
    #else
        #define MYLIB_API __declspec(dllimport)
    #endif
#else
    #define MYLIB_API
#endif
```

### 3. Handle Runtime Libraries

Choose correct CRT linking:

```toml
[windows]
runtime_library = "MD"  # Dynamic CRT (recommended)
# runtime_library = "MT"  # Static CRT (larger, no dependencies)
```

### 4. Include DLLs in Distribution

Always include DLLs with your binaries:

```
distribution/
├── myapp.exe
├── mylib.dll            # Your DLL
└── vcruntime140.dll     # MSVC runtime (if needed)
```

### 5. Test on Target Windows

Always test on actual Windows systems:
- Different Windows versions (10, 11)
- Different architectures (x86, x64)
- With and without Visual Studio installed

## Troubleshooting

### MSVC Not Found

```
Error: Could not find MSVC compiler
```

**Solution:**

```powershell
# Install Visual Studio with C++ workload
# Or install Build Tools

# Verify
where cl.exe

# Add to PATH if needed
$env:PATH += ";C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.38.33130\bin\Hostx64\x64"
```

### MinGW Not Found

```
Error: Could not find MinGW compiler
```

**Solution:**

```bash
# Install MSYS2/MinGW
# Add to PATH
export PATH="/c/msys64/mingw64/bin:$PATH"

# Verify
gcc --version
g++ --version
```

### DLL Not Found

```
Error: The code execution cannot proceed because mylib.dll was not found
```

**Solutions:**

1. Copy DLL to executable directory
2. Add DLL directory to PATH:
```powershell
$env:PATH += ";C:\path\to\dlls"
```

3. Use delayed loading (MSVC):
```cmake
target_link_options(myapp PRIVATE "/DELAYLOAD:mylib.dll")
```

### Symbol Not Found

```
Error: unresolved external symbol
```

**Solutions:**

1. Check DLL exports:
```powershell
dumpbin /EXPORTS mylib.dll
```

2. Verify __declspec(dllexport):
```cpp
class __declspec(dllexport) MyClass { ... };
```

3. Use .def file for exports:
```
LIBRARY mylib
EXPORTS
    MyFunction
    MyClass
```

## Performance Tips

### 1. Use Link-Time Optimization

```toml
[build]
cxxflags = ["/GL"]           # MSVC
ldflags = ["/LTCG"]          # MSVC
# cxxflags = ["-flto"]       # MinGW
```

### 2. Enable Optimizations

```toml
[build]
cxxflags = [
    "/O2",                   # MSVC: Optimize for speed
    "/arch:AVX2"             # Use AVX2 instructions
]
```

### 3. Static CRT for Standalone

For deployment without Visual C++ redistributable:

```toml
[windows]
runtime_library = "MT"       # Static CRT
```

## Migration Guides

### From Visual Studio Project

**Before:**
```
MyLib.vcxproj
MyLib.sln
```

**After:**

1. Create CCGO project:
```bash
ccgo new mylib
```

2. Copy source files to `src/`

3. Configure CCGO.toml:
```toml
[windows]
subsystem = "console"
runtime_library = "MD"
```

4. Build:
```bash
ccgo build windows
```

### From CMake

**CMakeLists.txt:**
```cmake
project(mylib)
add_library(mylib src/mylib.cpp)
```

**CCGO.toml:**
```toml
[package]
name = "mylib"
version = "1.0.0"
```

Then: `ccgo build windows`

## See Also

- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Platforms Overview](index.md)
