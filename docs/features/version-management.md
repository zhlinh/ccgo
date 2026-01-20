# Version Management

Complete guide to managing versions in CCGO projects using semantic versioning, git tags, and automated version injection.

## Overview

CCGO provides comprehensive version management features:

- **Semantic Versioning** - Follow SemVer 2.0.0 specifications
- **Automated Tagging** - Create git tags from CCGO.toml version
- **Version Injection** - Automatically inject version info into builds
- **Multi-Platform Support** - Consistent versioning across all platforms
- **Build Metadata** - Include git commit SHA, build timestamp in binaries
- **Release Management** - Simplify release workflows

## Version Format

### Semantic Versioning

CCGO follows [Semantic Versioning 2.0.0](https://semver.org/):

```
MAJOR.MINOR.PATCH[-PRERELEASE][+BUILDMETADATA]
```

**Components:**
- `MAJOR`: Incompatible API changes
- `MINOR`: New backward-compatible functionality
- `PATCH`: Backward-compatible bug fixes
- `PRERELEASE`: Optional pre-release identifier (alpha, beta, rc)
- `BUILDMETADATA`: Optional build metadata (commit SHA, timestamp)

**Examples:**
```
1.0.0           # Stable release
1.2.3           # Patch update
2.0.0-alpha.1   # Pre-release
1.5.0+20240115  # With build metadata
2.1.0-rc.2+g8f3a  # Pre-release with git hash
```

## Configuration

### CCGO.toml

```toml
[package]
name = "mylib"
version = "1.2.3"  # SemVer format
authors = ["Your Name <you@example.com>"]

[version]
# Version injection settings
inject_build_metadata = true  # Include git SHA and timestamp
inject_to_code = true          # Generate version header files
prerelease_suffix = "beta"     # Optional: alpha, beta, rc

# Platform-specific version overrides (optional)
[version.android]
version_code = 10203  # Android integer version (auto-calculated if not set)

[version.ios]
build_number = "123"  # iOS build number (defaults to PATCH)

[version.windows]
file_version = "1.2.3.0"  # Windows 4-part version
```

### Version Auto-Calculation

CCGO automatically calculates platform-specific version numbers:

**Android version_code:**
```
version_code = MAJOR * 10000 + MINOR * 100 + PATCH

Example: 1.2.3 → 10203
```

**iOS build number:**
```
build_number = PATCH (default)
Or custom: build_number = "123"
```

**Windows file version:**
```
file_version = MAJOR.MINOR.PATCH.0

Example: 1.2.3 → 1.2.3.0
```

## Creating Version Tags

### Using ccgo tag

```bash
# Create tag from CCGO.toml version
ccgo tag

# Create tag with custom version
ccgo tag v2.0.0

# Create tag with message
ccgo tag --message "Release version 2.0.0 with new features"

# Create annotated tag
ccgo tag --annotate

# Push tag to remote
ccgo tag --push
```

### Tag Format

```bash
# CCGO creates tags in format: v{VERSION}
v1.0.0
v1.2.3-beta.1
v2.0.0
```

### Manual Tagging

```bash
# Create lightweight tag
git tag v1.0.0

# Create annotated tag
git tag -a v1.0.0 -m "Release 1.0.0"

# Push tag to remote
git push origin v1.0.0

# Push all tags
git push --tags
```

## Version Injection

### Build-Time Injection

CCGO automatically injects version information during builds:

```bash
# Version info injected into all builds
ccgo build android

# Disable version injection
ccgo build android --no-version-inject
```

### Generated Version Header

**C++ Header (`include/<project>/version.h`):**
```cpp
#pragma once

#define MYLIB_VERSION "1.2.3"
#define MYLIB_VERSION_MAJOR 1
#define MYLIB_VERSION_MINOR 2
#define MYLIB_VERSION_PATCH 3

#define MYLIB_GIT_SHA "8f3a2b1c"
#define MYLIB_GIT_BRANCH "main"
#define MYLIB_BUILD_TIMESTAMP "2024-01-15T10:30:00Z"
#define MYLIB_BUILD_TYPE "Release"

// Platform-specific
#ifdef __ANDROID__
#define MYLIB_VERSION_CODE 10203
#elif defined(__APPLE__)
#define MYLIB_BUNDLE_VERSION "123"
#elif defined(_WIN32)
#define MYLIB_FILE_VERSION "1.2.3.0"
#endif

namespace mylib {
    const char* get_version();
    const char* get_git_sha();
    const char* get_build_timestamp();
}
```

**Implementation:**
```cpp
// src/version.cpp (auto-generated)
#include "mylib/version.h"

namespace mylib {
    const char* get_version() {
        return MYLIB_VERSION;
    }

    const char* get_git_sha() {
        return MYLIB_GIT_SHA;
    }

    const char* get_build_timestamp() {
        return MYLIB_BUILD_TIMESTAMP;
    }
}
```

### Using Version in Code

```cpp
#include "mylib/version.h"
#include <iostream>

void print_version() {
    std::cout << "MyLib version: " << mylib::get_version() << "\n";
    std::cout << "Git SHA: " << mylib::get_git_sha() << "\n";
    std::cout << "Built: " << mylib::get_build_timestamp() << "\n";
}
```

## Platform-Specific Versioning

### Android

**Version Code and Version Name:**
```toml
[package]
version = "1.2.3"

[version.android]
version_code = 10203       # Integer for Play Store
version_name = "1.2.3"     # Display name (defaults to package.version)
```

**Gradle Integration:**
```kotlin
// Generated in build.gradle.kts
android {
    defaultConfig {
        versionCode = 10203
        versionName = "1.2.3"
    }
}
```

### iOS

**Bundle Version:**
```toml
[package]
version = "1.2.3"

[version.ios]
bundle_short_version = "1.2.3"  # CFBundleShortVersionString
build_number = "123"             # CFBundleVersion
```

**Info.plist:**
```xml
<key>CFBundleShortVersionString</key>
<string>1.2.3</string>
<key>CFBundleVersion</key>
<string>123</string>
```

### OpenHarmony

**HAR Version:**
```toml
[package]
version = "1.2.3"

[version.ohos]
app_version_code = 10203
app_version_name = "1.2.3"
```

**module.json5:**
```json
{
  "module": {
    "versionCode": 10203,
    "versionName": "1.2.3"
  }
}
```

### Windows

**File Version:**
```toml
[package]
version = "1.2.3"

[version.windows]
file_version = "1.2.3.0"        # Four-part version
product_version = "1.2"          # Display version
company_name = "Your Company"
copyright = "Copyright © 2024"
```

**Resource File (.rc):**
```rc
VS_VERSION_INFO VERSIONINFO
FILEVERSION 1,2,3,0
PRODUCTVERSION 1,2,0,0
{
    VALUE "FileVersion", "1.2.3.0"
    VALUE "ProductVersion", "1.2"
    VALUE "CompanyName", "Your Company"
    VALUE "LegalCopyright", "Copyright © 2024"
}
```

## Pre-Release Versions

### Alpha Releases

```toml
[package]
version = "2.0.0-alpha.1"

[version]
prerelease_suffix = "alpha"
```

```bash
# Tag alpha release
ccgo tag v2.0.0-alpha.1 --message "Alpha release 1"
```

### Beta Releases

```toml
[package]
version = "2.0.0-beta.2"

[version]
prerelease_suffix = "beta"
```

### Release Candidates

```toml
[package]
version = "2.0.0-rc.1"

[version]
prerelease_suffix = "rc"
```

### Promotion to Stable

```bash
# Promote RC to stable
# 1. Update CCGO.toml
version = "2.0.0"  # Remove -rc.1

# 2. Create stable tag
ccgo tag v2.0.0 --message "Stable release 2.0.0"
```

## Build Metadata

### Git Information

CCGO automatically includes:
- **Commit SHA**: Current git commit hash
- **Branch**: Current git branch name
- **Tag**: Closest git tag (if any)
- **Dirty**: Whether working directory has uncommitted changes

### Timestamp

```cpp
// ISO 8601 format
#define MYLIB_BUILD_TIMESTAMP "2024-01-15T10:30:00Z"
```

### Build Type

```cpp
// Release or Debug
#define MYLIB_BUILD_TYPE "Release"
```

### Custom Metadata

```toml
[version]
custom_metadata = [
    "jenkins_build_123",
    "ci_pipeline_456"
]
```

## Version Queries

### Check Current Version

```bash
# Show version from CCGO.toml
ccgo --version

# Show detailed version info
ccgo version --detailed

# Output:
# CCGO version: 3.0.10
# Project: mylib
# Version: 1.2.3
# Git SHA: 8f3a2b1c
# Git branch: main
# Modified: no
```

### Runtime Version Query

```cpp
// In your application
#include "mylib/mylib.h"

const char* version = mylib::get_version();
printf("Library version: %s\n", version);
```

## Versioning Workflows

### Development Workflow

```bash
# 1. Start new feature
git checkout -b feature/new-api
# CCGO.toml: version = "1.2.0"

# 2. Develop and test
ccgo build --all
ccgo test

# 3. Merge to main
git checkout main
git merge feature/new-api

# 4. Bump version
# Update CCGO.toml: version = "1.3.0"

# 5. Create tag
ccgo tag v1.3.0 --message "Add new API"

# 6. Push
git push origin main --tags
```

### Release Workflow

```bash
# 1. Create release branch
git checkout -b release/2.0
# CCGO.toml: version = "2.0.0-rc.1"

# 2. Test release candidate
ccgo build --all
ccgo test --all

# 3. Fix bugs if needed
# ... bug fixes ...

# 4. Promote to stable
# Update CCGO.toml: version = "2.0.0"
ccgo tag v2.0.0 --message "Release 2.0.0"

# 5. Merge back to main
git checkout main
git merge release/2.0

# 6. Push
git push origin main --tags
```

### Hotfix Workflow

```bash
# 1. Create hotfix branch from tag
git checkout -b hotfix/1.2.4 v1.2.3
# CCGO.toml: version = "1.2.4"

# 2. Fix critical bug
# ... fix ...

# 3. Test
ccgo build --all
ccgo test

# 4. Create tag
ccgo tag v1.2.4 --message "Hotfix: critical bug"

# 5. Merge to main and release branch
git checkout main
git cherry-pick hotfix/1.2.4

git checkout release/1.2
git cherry-pick hotfix/1.2.4

# 6. Push
git push origin main release/1.2 --tags
```

## Changelog Integration

### Automatic Changelog Generation

```bash
# Generate changelog from git tags
ccgo changelog

# Output to file
ccgo changelog --output CHANGELOG.md

# Between specific versions
ccgo changelog --from v1.0.0 --to v2.0.0
```

### Changelog Format

```markdown
# Changelog

## [2.0.0] - 2024-01-15

### Added
- New authentication API
- Support for OAuth 2.0

### Changed
- Updated dependency versions
- Improved error handling

### Fixed
- Memory leak in network module
- Crash on invalid input

### Breaking Changes
- Removed deprecated APIs
- Changed function signatures

## [1.2.3] - 2024-01-01

### Fixed
- Critical security vulnerability
```

## Version Constraints

### Dependency Versions

```toml
[dependencies]
openssl = { version = "^1.1.0" }  # Compatible with 1.1.x
boost = { version = "~1.80.0" }   # Compatible with 1.80.x
zlib = { version = "1.2.11" }     # Exact version

# Version ranges
protobuf = { version = ">=3.0.0, <4.0.0" }
```

**Operators:**
- `^`: Compatible with (same major version)
- `~`: Approximately compatible (same minor version)
- `>=`, `<=`, `>`, `<`: Comparison operators
- `,`: AND operator

### Platform-Specific Constraints

```toml
[dependencies.android]
androidx-core = { version = "1.12.0" }

[dependencies.ios]
alamofire = { version = "~5.8.0" }
```

## Best Practices

### 1. Version Numbering

- **Start at 1.0.0** for first stable release
- **0.y.z** for initial development (unstable API)
- **Increment MAJOR** for breaking changes
- **Increment MINOR** for new features
- **Increment PATCH** for bug fixes

### 2. Tag Management

```bash
# Always use annotated tags for releases
git tag -a v1.0.0 -m "Release 1.0.0"

# Lightweight tags for internal use only
git tag build-123

# Push tags explicitly
git push origin v1.0.0
```

### 3. Version in Commit Messages

```bash
# Good commit messages
git commit -m "chore: bump version to 1.2.3"
git commit -m "release: v2.0.0"
git commit -m "hotfix: v1.2.4 - fix critical bug"
```

### 4. Pre-Release Testing

```bash
# Test all platforms before release
ccgo build --all --release
ccgo test --all

# Verify version info
ccgo version --detailed
```

### 5. Documentation

- Update CHANGELOG.md before each release
- Document breaking changes clearly
- Maintain migration guides for major versions

## CI/CD Integration

### GitHub Actions

```yaml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Extract version
        id: version
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          echo "version=$VERSION" >> $GITHUB_OUTPUT

      - name: Build all platforms
        run: ccgo build --all --release

      - name: Create GitHub Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ steps.version.outputs.version }}
          draft: false
          prerelease: false
```

### GitLab CI

```yaml
release:
  stage: deploy
  only:
    - tags
  script:
    - ccgo build --all --release
    - ccgo publish --all --registry official
  artifacts:
    paths:
      - target/
```

## Troubleshooting

### Version Mismatch

**Problem:**
```
Warning: CCGO.toml version (1.2.3) doesn't match git tag (v1.2.2)
```

**Solution:**
```bash
# Update CCGO.toml to match tag
# Or create new tag matching CCGO.toml
ccgo tag --force
```

### Invalid Version Format

**Problem:**
```
Error: Invalid version format: "1.2.3.4.5"
```

**Solution:**
```toml
# Use SemVer format
version = "1.2.3"  # Not "1.2.3.4.5"
```

### Tag Already Exists

**Problem:**
```
Error: tag 'v1.0.0' already exists
```

**Solution:**
```bash
# Delete old tag (if intentional)
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0

# Create new tag
ccgo tag v1.0.0
```

## Examples

### Complete Versioning Setup

```toml
# CCGO.toml
[package]
name = "mylib"
version = "1.2.3"
authors = ["Your Name <you@example.com>"]

[version]
inject_build_metadata = true
inject_to_code = true

[version.android]
version_code = 10203

[version.ios]
build_number = "123"

[version.windows]
file_version = "1.2.3.0"
company_name = "Your Company"
copyright = "Copyright © 2024"
```

## Resources

### Tools

- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Conventional Commits](https://www.conventionalcommits.org/)

### CCGO Documentation

- [CLI Reference](../reference/cli.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Git Integration](git-integration.md)
- [Publishing Guide](publishing.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [Git Integration](git-integration.md)
- [Publishing Guide](publishing.md)
- [CI/CD Setup](../development/contributing.md)
- [Changelog Management](../development/changelog.md)
