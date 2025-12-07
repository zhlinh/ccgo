# Conan Package Manager Configuration Guide

This guide explains how to configure CCGO to build and manage C/C++ packages using Conan, the popular C/C++ package manager.

## Overview

CCGO now supports building Conan packages directly from your C/C++ projects. Conan is a decentralized package manager that allows you to:
- Create reusable C/C++ packages
- Manage dependencies effectively
- Share libraries across projects
- Support multiple build configurations
- Integrate with various build systems

All configuration is done through `CCGO.toml`, with automatic generation of `conanfile.py` when needed.

## Configuration in CCGO.toml

### Basic Conan Configuration

```toml
[build.conan]
# Package name (defaults to project.name)
name = "my-cpp-lib"

# Package version (defaults to project.version)
version = "1.0.0"

# Package description (defaults to project.description)
description = "A powerful C++ library"

# Build mode: "create" (full package), "export" (export only), "build" (local build)
mode = "create"

# Conan profile to use (default: "default")
profile = "default"

# Build folder for local builds
build_folder = "cmake_build/conan"
```

### Advanced Configuration

```toml
[build.conan]
name = "advanced-lib"
version = "2.0.0"
author = "Your Name <you@example.com>"
license = "MIT"
url = "https://github.com/username/advanced-lib"

# Build settings
settings = ["os", "compiler", "build_type", "arch"]

# Package options
[build.conan.options]
shared = [true, false]
fPIC = [true, false]
with_tests = [true, false]
with_docs = [true, false]

# Default option values
[build.conan.default_options]
shared = false
fPIC = true
with_tests = false
with_docs = false

# Package dependencies
[build.conan.requires]
dependencies = [
    "zlib/1.2.13",
    "openssl/3.0.7",
    "boost/1.82.0"
]

# Build requirements (tools needed during build)
[build.conan.build_requires]
tools = [
    "cmake/3.27.7",
    "ninja/1.11.1"
]
```

## Build Modes

### 1. Create Mode (Default)
Creates a complete Conan package and installs it to local cache:

```bash
ccgo build conan  # Uses mode = "create" by default
```

This mode:
- Builds the project
- Runs tests (if configured)
- Creates the package
- Installs to local Conan cache

### 2. Export Mode
Exports the recipe without building:

```toml
[build.conan]
mode = "export"
```

```bash
ccgo build conan
```

Useful for:
- CI/CD pipelines
- Quick recipe validation
- Preparing packages for remote upload

### 3. Build Mode
Builds locally for testing without creating a package:

```toml
[build.conan]
mode = "build"
```

```bash
ccgo build conan
```

Perfect for:
- Local development
- Testing build configurations
- Debugging build issues

## Command Line Usage

### Basic Build
```bash
# Build Conan package with default settings
ccgo build conan

# Check Conan installation and environment
ccgo check conan --verbose
```

### With Custom Settings
```bash
# Use specific Conan profile
ccgo build conan --profile my-profile

# Build with debug configuration
ccgo build conan --build-type Debug

# Build shared library
ccgo build conan --shared
```

## Generated conanfile.py

CCGO automatically generates a `conanfile.py` if it doesn't exist, based on your CCGO.toml configuration:

```python
from conan import ConanFile
from conan.tools.cmake import CMake, CMakeToolchain, CMakeDeps

class MyLibConan(ConanFile):
    name = "my-cpp-lib"
    version = "1.0.0"
    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False], "fPIC": [True, False]}
    default_options = {"shared": False, "fPIC": True}

    # Dependencies from CCGO.toml
    requires = "zlib/1.2.13", "openssl/3.0.7"

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        cmake = CMake(self)
        cmake.install()
```

## Conan Profiles

### Using Default Profile
```bash
ccgo build conan
```

### Using Custom Profile
```toml
[build.conan]
profile = "linux-gcc11-release"
```

### Creating Profiles
```bash
# Create a new profile
conan profile new gcc11 --detect

# Edit profile
conan profile update settings.compiler.libcxx=libstdc++11 gcc11
```

## Integration with CI/CD

### GitHub Actions
```yaml
- name: Install Conan
  run: pip install conan

- name: Build Conan Package
  run: ccgo build conan

- name: Upload to Artifactory
  run: conan upload "*" --remote artifactory
```

### GitLab CI
```yaml
build-conan:
  script:
    - pip install conan
    - ccgo build conan
    - conan upload "*" --remote gitlab
```

### Jenkins
```groovy
stage('Build Conan Package') {
    steps {
        sh 'pip install conan'
        sh 'ccgo build conan'
    }
}
```

## Consuming the Package

Once built, use your package in other projects:

### conanfile.txt
```ini
[requires]
my-cpp-lib/1.0.0

[generators]
CMakeDeps
CMakeToolchain

[options]
my-cpp-lib:shared=True
```

### conanfile.py
```python
from conan import ConanFile

class MyApp(ConanFile):
    requires = "my-cpp-lib/1.0.0"
    generators = "CMakeDeps", "CMakeToolchain"
```

## Multi-Configuration Builds

Build multiple configurations:

```bash
# Debug build
conan create . --build=missing -s build_type=Debug

# Release build
conan create . --build=missing -s build_type=Release

# Different compilers
conan create . --build=missing -s compiler=gcc -s compiler.version=11
conan create . --build=missing -s compiler=clang -s compiler.version=14
```

## Cross-Platform Support

### Windows
```toml
[build.conan.options]
shared = [true, false]  # DLL or static lib

[build.conan.default_options]
shared = true  # Build DLL by default on Windows
```

### Linux/macOS
```toml
[build.conan.options]
fPIC = [true, false]  # Position Independent Code

[build.conan.default_options]
fPIC = true  # Required for shared libraries
```

## Package Layout

Standard Conan package structure:
```
my-cpp-lib/
├── conanfile.py       # Conan recipe (auto-generated)
├── CMakeLists.txt     # CMake configuration
├── CCGO.toml         # CCGO configuration
├── include/          # Public headers
│   └── mylib/
│       └── mylib.h
├── src/              # Source files
│   └── mylib.cpp
└── test/             # Tests
    └── test_mylib.cpp
```

## Remote Repositories

### Upload to Conan Center
```bash
# Build and test locally
ccgo build conan

# Upload to Conan Center (after review)
conan upload my-cpp-lib/1.0.0 --remote conan-center
```

### Private Repositories
```bash
# Add private remote
conan remote add mycompany https://artifactory.company.com/artifactory/api/conan/conan-local

# Upload to private repository
conan upload my-cpp-lib/1.0.0 --remote mycompany
```

## Troubleshooting

### Conan Not Found
```
ERROR: Conan is not installed or not in PATH
```

Solution:
```bash
pip install conan
# or
pip3 install conan
```

### CMake Configuration Failed
```
ERROR: CMake configuration failed
```

Check:
1. CMakeLists.txt is valid
2. Required dependencies are available
3. Build tools are installed

### Package Not Found
```
ERROR: Unable to find 'dependency/1.0.0'
```

Solutions:
1. Add remote repository: `conan remote add <name> <url>`
2. Build missing dependencies: `--build=missing`
3. Check package name and version

### Profile Not Found
```
ERROR: Profile 'custom' not found
```

Create the profile:
```bash
conan profile new custom --detect
```

## Best Practices

1. **Semantic Versioning** - Use major.minor.patch format
2. **Test Packages** - Build locally before uploading
3. **Document Dependencies** - List all requirements clearly
4. **Use Profiles** - Create profiles for different configurations
5. **Version Lock** - Specify exact versions for reproducibility
6. **CI Integration** - Automate package building in CI/CD
7. **Package Signing** - Sign packages for security

## Advanced Features

### Conditional Dependencies
```toml
[build.conan]
# Platform-specific dependencies
[build.conan.requires.linux]
dependencies = ["systemd/251"]

[build.conan.requires.windows]
dependencies = ["winsdk/10.0.22621"]
```

### Custom Generators
```python
# In conanfile.py
generators = "CMakeDeps", "CMakeToolchain", "VirtualBuildEnv"
```

### Package Channels
```toml
[build.conan]
# Use different channels for stability levels
channel = "stable"  # or "testing", "dev"
```

## Migration from Manual Conan

### Old Way (Manual)
```bash
# Manual steps
cd project
conan install . --build=missing
conan build .
conan create .
conan upload ...
```

### New Way (with CCGO)
```toml
# Configure once in CCGO.toml
[build.conan]
name = "my-lib"
version = "1.0.0"
```

Then simply:
```bash
ccgo build conan
```

Benefits:
- ✅ Automatic conanfile.py generation
- ✅ Integrated with CCGO build system
- ✅ Consistent configuration
- ✅ Simplified workflow
- ✅ Cross-platform support

## Additional Resources

- [Conan Documentation](https://docs.conan.io/)
- [Conan Center](https://conan.io/center)
- [CMake Integration Guide](https://docs.conan.io/2/examples/tools/cmake.html)
- [Package Creation Tutorial](https://docs.conan.io/2/tutorial/creating_packages.html)