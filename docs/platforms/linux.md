# Linux Platform

Complete guide to building C++ libraries for Linux with CCGO.

## Overview

CCGO provides comprehensive Linux support with:

- **Multiple compilers**: GCC and Clang
- **Multiple architectures**: x86_64, ARM64 (aarch64), ARMv7
- **Output formats**: Static libraries (.a), Shared libraries (.so)
- **Build methods**: Local or Docker (cross-platform)
- **IDE support**: CodeLite project generation
- **Distribution compatibility**: Ubuntu, Debian, CentOS, Fedora, Alpine
- **C library variants**: glibc and musl support

## Prerequisites

### Option 1: Local Build (Linux Required)

**For Ubuntu/Debian:**

```bash
# Install GCC toolchain
sudo apt update
sudo apt install -y build-essential cmake pkg-config

# Install Clang (optional)
sudo apt install -y clang

# Verify installation
gcc --version
g++ --version
cmake --version
```

**For CentOS/Fedora/RHEL:**

```bash
# Install GCC toolchain
sudo yum groupinstall -y "Development Tools"
sudo yum install -y cmake

# Or on Fedora
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y cmake

# Install Clang (optional)
sudo yum install -y clang
# Or on Fedora
sudo dnf install -y clang

# Verify
gcc --version
g++ --version
cmake --version
```

**For Alpine Linux:**

```bash
# Install GCC toolchain (musl libc)
apk add --no-cache build-base cmake

# Install Clang (optional)
apk add --no-cache clang

# Verify
gcc --version
g++ --version
cmake --version
```

### Option 2: Docker Build (Any OS)

Build Linux libraries on macOS or Windows using Docker.

**Required:**
- Docker Desktop installed and running
- 3GB+ disk space for Docker image

**Advantages:**
- Build on any operating system
- Consistent build environment
- No Linux dependencies required
- Support for multiple Linux distributions

**Limitations:**
- Cannot run/test applications (unless using Docker shell)
- Larger initial download (~800MB image)

See [Docker Builds](#docker-builds) section for details.

## Quick Start

### Basic Build

```bash
# Build for x86_64 with default compiler
ccgo build linux

# Build with Docker (cross-compile from any OS)
ccgo build linux --docker

# Specify compiler
ccgo build linux --compiler gcc       # GCC (default)
ccgo build linux --compiler clang     # Clang
ccgo build linux --compiler auto      # Both compilers

# Specify architecture
ccgo build linux --arch x86_64        # 64-bit Intel/AMD (default)
ccgo build linux --arch arm64         # 64-bit ARM (aarch64)
ccgo build linux --arch armv7         # 32-bit ARM

# Build types
ccgo build linux --build-type debug    # Debug build
ccgo build linux --build-type release  # Release build (default)

# Link types
ccgo build linux --link-type static    # Static library only
ccgo build linux --link-type shared    # Shared library only
ccgo build linux --link-type both      # Both types (default)
```

### Generate CodeLite Project

```bash
# Generate CodeLite workspace
ccgo build linux --ide-project

# Open in CodeLite
codelite cmake_build/linux/MyLib.workspace
```

## Output Structure

### Default Output (`target/linux/`)

```
target/linux/
├── MyLib_Linux_SDK-1.0.0.zip           # Main package
│   ├── lib/
│   │   ├── static/
│   │   │   ├── gcc/
│   │   │   │   └── libmylib.a          # GCC static library
│   │   │   └── clang/
│   │   │       └── libmylib.a          # Clang static library
│   │   └── shared/
│   │       ├── gcc/
│   │       │   ├── libmylib.so         # GCC shared library
│   │       │   └── libmylib.so.1.0.0   # Versioned library
│   │       └── clang/
│   │           ├── libmylib.so
│   │           └── libmylib.so.1.0.0
│   ├── include/
│   │   └── mylib/                      # Header files
│   │       ├── mylib.h
│   │       └── version.h
│   └── build_info.json                 # Build metadata
│
└── MyLib_Linux_SDK-1.0.0-SYMBOLS.zip   # Debug symbols
    └── symbols/
        ├── gcc/
        │   └── libmylib.so.debug       # GCC debug symbols
        └── clang/
            └── libmylib.so.debug       # Clang debug symbols
```

### Library Types

**Static library (.a):**
- Archive of object files
- Linked at compile time
- Larger executable
- No runtime dependencies
- All symbols included

**Shared library (.so):**
- Dynamically linked at runtime
- Smaller executable
- Shared between processes
- Versioned (libmylib.so.1.0.0)
- Symlinks for compatibility:
  - `libmylib.so` → `libmylib.so.1` → `libmylib.so.1.0.0`

### Build Metadata

`build_info.json` contains:

```json
{
  "project": {
    "name": "MyLib",
    "version": "1.0.0",
    "description": "My Linux library"
  },
  "build": {
    "platform": "linux",
    "architectures": ["x86_64"],
    "compilers": ["gcc", "clang"],
    "build_type": "release",
    "link_types": ["static", "shared"],
    "timestamp": "2024-01-15T10:30:00Z",
    "ccgo_version": "0.1.0",
    "gcc_version": "11.4.0",
    "clang_version": "14.0.0",
    "libc": "glibc-2.35"
  },
  "outputs": {
    "libraries": {
      "gcc": {
        "static": "lib/static/gcc/libmylib.a",
        "shared": "lib/shared/gcc/libmylib.so"
      },
      "clang": {
        "static": "lib/static/clang/libmylib.a",
        "shared": "lib/shared/clang/libmylib.so"
      }
    },
    "headers": "include/mylib/",
    "symbols": {
      "gcc": "symbols/gcc/libmylib.so.debug",
      "clang": "symbols/clang/libmylib.so.debug"
    }
  }
}
```

## GCC vs Clang

### GCC (GNU Compiler Collection)

**Pros:**
- Default on most Linux distributions
- Excellent optimization
- Wide architecture support
- Better C++20/C++23 support (recent versions)
- Larger community

**Cons:**
- Slower compilation than Clang
- Less helpful error messages
- Larger binaries sometimes

**When to use:**
- Standard Linux development
- Maximum compatibility
- Latest C++ standards
- Best optimization

### Clang (LLVM)

**Pros:**
- Faster compilation
- Better error messages and warnings
- Excellent static analysis
- Better for development
- Modular architecture

**Cons:**
- May produce slightly slower code
- Less common as default
- Some architectures less optimized

**When to use:**
- Development and debugging
- Need better diagnostics
- Using LLVM ecosystem
- Static analysis required

## Using Libraries in C++

### Linking Static Library

**CMakeLists.txt:**

```cmake
# Find library
find_library(MYLIB_LIBRARY
    NAMES mylib libmylib.a
    PATHS "/path/to/lib/static/gcc"
)

# Link to target
target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "/path/to/include")
```

**Manual compilation:**

```bash
# With GCC
g++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/static/gcc -lmylib

# With Clang
clang++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/static/clang -lmylib
```

### Linking Shared Library

**CMakeLists.txt:**

```cmake
# Find shared library
find_library(MYLIB_LIBRARY
    NAMES mylib
    PATHS "/path/to/lib/shared/gcc"
)

target_link_libraries(myapp PRIVATE ${MYLIB_LIBRARY})
target_include_directories(myapp PRIVATE "/path/to/include")

# Set RPATH for finding library at runtime
set_target_properties(myapp PROPERTIES
    BUILD_RPATH "/path/to/lib/shared/gcc"
    INSTALL_RPATH "$ORIGIN:$ORIGIN/../lib"
)
```

**Manual compilation:**

```bash
# With GCC
g++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/shared/gcc -lmylib \
    -Wl,-rpath,/path/to/lib/shared/gcc

# Run with LD_LIBRARY_PATH
LD_LIBRARY_PATH=/path/to/lib/shared/gcc ./myapp
```

**Using in code:**

```cpp
#include <mylib/mylib.h>

int main() {
    mylib::MyClass obj;
    obj.do_work();
    return 0;
}
```

## Docker Builds

Build Linux libraries on any OS using Docker:

### Prerequisites

```bash
# Install Docker Desktop
# Download from: https://www.docker.com/products/docker-desktop/

# Verify Docker is running
docker ps
```

### Build with Docker

```bash
# First build downloads prebuilt image (~800MB)
ccgo build linux --docker

# Subsequent builds are fast
ccgo build linux --docker --arch x86_64

# All standard options work
ccgo build linux --docker --compiler gcc --link-type static
```

### How It Works

1. CCGO uses prebuilt `ccgo-builder-linux` image from Docker Hub
2. Project directory mounted into container
3. Build runs with Ubuntu 22.04 + GCC/Clang
4. Output written to host filesystem

### Limitations

- **Cannot run**: No X11 display in Docker
- **Cannot test**: GUI applications won't work
- Use `docker exec -it <container> bash` to run CLI apps

## Distribution Compatibility

### glibc vs musl

**glibc (GNU C Library):**
- Standard on most distributions
- Better performance
- Wider compatibility
- Larger binary size

**musl:**
- Used on Alpine Linux
- Smaller and simpler
- Static linking friendly
- Strictly POSIX compliant

### ABI Compatibility

Libraries built on older distributions generally work on newer ones:

```
# Build on Ubuntu 18.04 (glibc 2.27)
# Works on Ubuntu 20.04, 22.04, 24.04

# Build on Ubuntu 22.04 (glibc 2.35)
# May NOT work on Ubuntu 18.04, 20.04
```

**Best practice**: Build on the oldest distribution you need to support.

### Versioning Shared Libraries

CCGO automatically creates versioned shared libraries:

```bash
# Created files
libmylib.so.1.0.0         # Real library with full version
libmylib.so.1             # SONAME symlink
libmylib.so               # Development symlink

# Check SONAME
objdump -p libmylib.so.1.0.0 | grep SONAME
# Output: SONAME      libmylib.so.1
```

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

[linux]
compiler = "gcc"               # gcc, clang, or auto
libc = "glibc"                 # glibc or musl
position_independent_code = true  # PIC for shared libraries
strip_symbols = false          # Strip debug symbols
```

### CMake Variables

When building for Linux:

```cmake
${PLATFORM}                    # "linux"
${ARCHITECTURE}                # "x86_64", "arm64", or "armv7"
${BUILD_TYPE}                  # "Debug" or "Release"
${LINK_TYPE}                   # "static", "shared", or "both"
${COMPILER}                    # "gcc" or "clang"
${LINUX_LIBC}                  # "glibc" or "musl"
```

### Conditional Compilation

```cpp
// Platform detection
#ifdef __linux__
    // Linux-specific code
    #include <unistd.h>

    #ifdef __x86_64__
        // x86_64-specific code
    #elif defined(__aarch64__)
        // ARM64-specific code
    #elif defined(__arm__)
        // ARMv7-specific code
    #endif
#endif

// Compiler detection
#ifdef __GNUC__
    // GCC or Clang
    #define MYLIB_API __attribute__((visibility("default")))

    #ifdef __clang__
        // Clang-specific code
    #else
        // GCC-specific code
    #endif
#endif

// glibc detection
#ifdef __GLIBC__
    // glibc-specific code
    #include <gnu/libc-version.h>
#endif

// Usage
class MYLIB_API MyClass {
public:
    void do_work();
};
```

### Symbol Visibility

Control exported symbols in shared libraries:

```cpp
// mylib_export.h
#ifdef __linux__
    #ifdef MYLIB_EXPORTS
        #define MYLIB_API __attribute__((visibility("default")))
    #else
        #define MYLIB_API
    #endif
    #define MYLIB_LOCAL __attribute__((visibility("hidden")))
#else
    #define MYLIB_API
    #define MYLIB_LOCAL
#endif

// Public API
class MYLIB_API PublicClass {
public:
    void public_method();
};

// Internal implementation (not exported)
class MYLIB_LOCAL InternalClass {
public:
    void internal_method();
};
```

## Best Practices

### 1. Control Symbol Visibility

Hide internal symbols to reduce library size and improve loading time:

```cmake
# CMakeLists.txt
set(CMAKE_CXX_VISIBILITY_PRESET hidden)
set(CMAKE_VISIBILITY_INLINES_HIDDEN YES)
```

```cpp
// Explicitly export public API
class __attribute__((visibility("default"))) MyPublicClass { ... };
```

### 2. Use RPATH for Distribution

Set RPATH so applications can find shared libraries:

```cmake
# Install libraries to lib/ directory relative to binary
set_target_properties(myapp PROPERTIES
    INSTALL_RPATH "$ORIGIN:$ORIGIN/../lib"
)
```

Directory structure:
```
myapp/
├── bin/
│   └── myapp              # Executable
└── lib/
    └── libmylib.so        # Libraries
```

### 3. Version Shared Libraries

Follow semantic versioning for shared libraries:

```toml
[package]
version = "1.2.3"          # Creates libmylib.so.1.2.3
```

### 4. Static Linking for Simplicity

For simpler distribution without dependencies:

```bash
# Build static library only
ccgo build linux --link-type static

# All code is embedded in executable
g++ -o myapp main.cpp -I/path/to/include -L/path/to/lib/static -lmylib
```

### 5. Test on Target Distributions

Always test on actual target distributions:
- Ubuntu 20.04, 22.04, 24.04
- Debian 11, 12
- CentOS 7, 8, 9
- Fedora (latest)
- Alpine (for musl)

### 6. Use Position Independent Code

Always enable PIC for shared libraries:

```toml
[linux]
position_independent_code = true
```

### 7. Strip Release Binaries

Reduce library size for release builds:

```bash
# Strip debug symbols
strip libmylib.so

# Or configure in CCGO.toml
[linux]
strip_symbols = true
```

## Troubleshooting

### Compiler Not Found

```
Error: g++ not found
```

**Solution:**

```bash
# Ubuntu/Debian
sudo apt install -y build-essential

# CentOS/Fedora
sudo yum groupinstall -y "Development Tools"

# Verify
which g++
g++ --version
```

### Library Not Found at Runtime

```
error while loading shared libraries: libmylib.so: cannot open shared object file
```

**Solutions:**

1. **Add to LD_LIBRARY_PATH:**
```bash
export LD_LIBRARY_PATH=/path/to/lib:$LD_LIBRARY_PATH
./myapp
```

2. **Install to system directory:**
```bash
sudo cp libmylib.so /usr/local/lib/
sudo ldconfig
```

3. **Use RPATH (recommended):**
```cmake
set_target_properties(myapp PROPERTIES
    INSTALL_RPATH "$ORIGIN/../lib"
)
```

4. **Check library path:**
```bash
ldd myapp
# Shows: libmylib.so => not found

# After fixing:
ldd myapp
# Shows: libmylib.so => /path/to/lib/libmylib.so
```

### Symbol Not Found

```
undefined reference to 'mylib::MyClass::do_work()'
```

**Solutions:**

1. **Check symbol exists:**
```bash
nm -C libmylib.so | grep do_work
# Should show: 00001234 T mylib::MyClass::do_work()
```

2. **Verify library is linked:**
```bash
ldd myapp | grep mylib
# Should show libmylib.so
```

3. **Check symbol visibility:**
```cpp
// Ensure symbols are exported
class __attribute__((visibility("default"))) MyClass { ... };
```

### Version Mismatch

```
version `GLIBC_2.35' not found
```

**Solution:**

Build on an older distribution or use static linking:

```bash
# Check required glibc version
objdump -T libmylib.so | grep GLIBC

# Check system glibc version
ldd --version

# Build on older system or use Docker with older base image
```

### CMake Configuration Fails

```
Could not find a package configuration file provided by "MyLib"
```

**Solution:**

Ensure CMake can find the library:

```cmake
# Set CMAKE_PREFIX_PATH
set(CMAKE_PREFIX_PATH "/path/to/MyLib/lib/cmake")
find_package(MyLib REQUIRED)

# Or set as environment variable
export CMAKE_PREFIX_PATH=/path/to/MyLib/lib/cmake
```

## Performance Tips

### 1. Use Link-Time Optimization

```toml
[build]
cxxflags = ["-flto"]       # Enable LTO
ldflags = ["-flto"]
```

### 2. Enable Optimizations

```toml
[build]
cxxflags = [
    "-O3",                 # Maximum optimization
    "-march=native",       # Use CPU-specific instructions
    "-mtune=native"
]
```

### 3. Profile-Guided Optimization

```bash
# 1. Build with profiling
CXXFLAGS="-fprofile-generate" ccgo build linux

# 2. Run with typical workload
./benchmark

# 3. Rebuild with profile data
CXXFLAGS="-fprofile-use" ccgo build linux
```

### 4. Static Linking for Performance

Static linking can be faster due to better optimization:

```bash
ccgo build linux --link-type static
```

### 5. Disable Exceptions (If Not Needed)

```toml
[build]
cxxflags = ["-fno-exceptions"]
```

## Packaging and Distribution

### System Package Integration

**Debian/Ubuntu (.deb):**

```bash
# Install checkinstall
sudo apt install checkinstall

# Create .deb package
cd target/linux
sudo checkinstall --pkgname=mylib --pkgversion=1.0.0 \
    --provides=mylib make install
```

**RPM-based (.rpm):**

```bash
# Create RPM package
rpmbuild -ba mylib.spec
```

**AppImage (Portable):**

```bash
# Bundle application with dependencies
appimagetool myapp.AppDir myapp.AppImage
```

### Snap Package

```yaml
# snapcraft.yaml
name: mylib
version: '1.0.0'
summary: My Linux library
description: Complete C++ library for Linux

parts:
  mylib:
    plugin: cmake
    source: .
```

```bash
# Build snap
snapcraft
```

### Flatpak

```json
{
  "app-id": "com.example.mylib",
  "runtime": "org.freedesktop.Platform",
  "sdk": "org.freedesktop.Sdk",
  "command": "myapp"
}
```

```bash
# Build flatpak
flatpak-builder build-dir com.example.mylib.json
```

## Migration Guides

### From Makefile

**Before:**
```makefile
CC = gcc
CFLAGS = -O2 -Wall
TARGET = libmylib.so

$(TARGET): mylib.o
    $(CC) -shared -o $(TARGET) mylib.o
```

**After:**

1. Create CCGO project:
```bash
ccgo new mylib
```

2. Copy source files to `src/`

3. Configure CCGO.toml:
```toml
[linux]
compiler = "gcc"
```

4. Build:
```bash
ccgo build linux
```

### From CMake

**CMakeLists.txt:**
```cmake
project(mylib)
add_library(mylib SHARED src/mylib.cpp)
target_include_directories(mylib PUBLIC include)
```

**CCGO.toml:**
```toml
[package]
name = "mylib"
version = "1.0.0"

[library]
type = "shared"
```

Then: `ccgo build linux`

### From Autotools

**Before:**
```bash
./configure
make
make install
```

**After:**

1. Extract source files from `src/` directory
2. Create CCGO project with `ccgo new`
3. Copy sources to new project structure
4. Configure dependencies in CCGO.toml
5. Build with `ccgo build linux`

## Advanced Topics

### Cross-Compilation

Build for different architectures:

```bash
# Build for ARM64 on x86_64
ccgo build linux --arch arm64 --docker

# Build for ARMv7
ccgo build linux --arch armv7 --docker
```

### Static musl Builds

For truly portable static binaries:

```bash
# Use Alpine Linux Docker image
ccgo build linux --docker --libc musl --link-type static
```

### Sanitizers

Enable address sanitizer for debugging:

```toml
[build]
cxxflags = ["-fsanitize=address", "-fno-omit-frame-pointer"]
ldflags = ["-fsanitize=address"]
```

### Coverage Analysis

```bash
# Build with coverage
CXXFLAGS="-fprofile-arcs -ftest-coverage" ccgo build linux

# Run tests
ccgo test

# Generate report
gcov src/mylib.cpp
lcov --capture --directory . --output-file coverage.info
genhtml coverage.info --output-directory coverage_html
```

## See Also

- [Build System](../features/build-system.md)
- [Dependency Management](../features/dependency-management.md)
- [Docker Builds](../features/docker-builds.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Platforms Overview](index.md)
