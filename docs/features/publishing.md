# Publishing Management

Complete guide to publishing C++ libraries to various package registries with CCGO.

## Overview

CCGO provides unified publishing capabilities across multiple platforms and package managers:

- **Android**: Maven Local, Maven Central, Private Maven repositories
- **iOS/macOS/Apple**: CocoaPods, Swift Package Manager (SPM)
- **OpenHarmony**: OHPM (official and private registries)
- **Cross-platform**: Conan (local and remote)
- **KMP**: Kotlin Multiplatform Maven publishing
- **Documentation**: GitHub Pages

All publishing commands use a consistent `--registry` flag for target selection.

## Quick Start

### Basic Publishing

```bash
# Publish Android library to Maven Local
ccgo publish android --registry local

# Publish to Maven Central
ccgo publish android --registry official

# Publish to private Maven repository
ccgo publish android --registry private --url https://maven.example.com

# Publish iOS/macOS libraries
ccgo publish apple --manager cocoapods      # CocoaPods
ccgo publish apple --manager spm --push     # Swift Package Manager
ccgo publish apple --manager all --push     # Both

# Publish OpenHarmony library
ccgo publish ohos --registry official       # Official OHPM
ccgo publish ohos --registry private --url https://ohpm.example.com

# Publish Conan package
ccgo publish conan --registry local         # Conan local cache
ccgo publish conan --registry official      # First configured remote
ccgo publish conan --registry private --remote-name myrepo --url URL

# Publish documentation
ccgo publish doc --doc-branch gh-pages --doc-open
```

### Skip Build

Publish existing artifacts without rebuilding:

```bash
# Use existing AAR
ccgo publish android --registry local --skip-build

# Use existing HAR
ccgo publish ohos --registry official --skip-build
```

## Android Publishing (Maven)

### Registry Types

**local**: Maven Local (~/.m2/repository/)
- No authentication required
- Immediate availability
- Perfect for local testing
- Not accessible to others

**official**: Maven Central (sonatype.org)
- Requires Sonatype account
- PGP signing required
- Review process (2-4 hours)
- Globally accessible

**private**: Custom Maven repository
- Company/team repositories
- Authentication required
- Immediate availability
- Team accessible

### Configuration

**gradle.properties:**

```properties
# Maven Central credentials
SONATYPE_USERNAME=your-username
SONATYPE_PASSWORD=your-password

# PGP signing
signing.keyId=12345678
signing.password=your-password
signing.secretKeyRingFile=/path/to/secring.gpg

# Private repository
PRIVATE_MAVEN_URL=https://maven.example.com
PRIVATE_MAVEN_USERNAME=your-username
PRIVATE_MAVEN_PASSWORD=your-password
```

### Publishing Commands

```bash
# Publish to Maven Local for testing
ccgo publish android --registry local

# Publish to Maven Central (production)
ccgo publish android --registry official

# Publish to private Maven
ccgo publish android --registry private \
    --url https://maven.example.com \
    --username admin \
    --password secret

# Specify group ID and artifact ID
ccgo publish android --registry official \
    --group-id com.example \
    --artifact-id mylib
```

### Maven Central Publishing

**Prerequisites:**

1. **Create Sonatype account**: https://issues.sonatype.org/
2. **Generate PGP key:**
```bash
gpg --gen-key
gpg --list-secret-keys --keyid-format LONG
gpg --keyserver hkp://pool.sks-keyservers.net --send-keys YOUR_KEY_ID
```

3. **Export secret key:**
```bash
gpg --export-secret-keys YOUR_KEY_ID > ~/.gnupg/secring.gpg
```

4. **Configure credentials** in `~/.gradle/gradle.properties`

**Publishing process:**

```bash
# Build and publish
ccgo publish android --registry official

# After upload, sign in to Sonatype to release:
# https://s01.oss.sonatype.org/
# 1. Find staging repository
# 2. Click "Close" button
# 3. Wait for validation
# 4. Click "Release" button
```

## iOS/macOS Publishing (Apple Platforms)

### Package Managers

**CocoaPods**: Traditional dependency manager
- Podspec-based
- Central repository (CocoaPods Trunk)
- Wide adoption

**Swift Package Manager (SPM)**: Apple's official solution
- Git-based
- No central repository
- Native Xcode integration

### CocoaPods Publishing

**Setup:**

```bash
# Register with CocoaPods Trunk
pod trunk register your@email.com 'Your Name'

# Verify registration (check email)
```

**Publish:**

```bash
# Publish to CocoaPods Trunk (official)
ccgo publish apple --manager cocoapods

# Publish to private spec repo
ccgo publish apple --manager cocoapods \
    --registry private \
    --remote-name myspecs \
    --url https://github.com/company/specs.git
```

**Podspec validation:**

```bash
# Validate before publishing
pod spec lint MyLib.podspec

# Validate with verbose output
pod spec lint MyLib.podspec --verbose
```

### Swift Package Manager Publishing

**Setup:**

```bash
# SPM uses Git tags for versions
# Ensure repository is initialized
git init
git add .
git commit -m "Initial commit"
```

**Publish:**

```bash
# Tag and push (SPM publishing)
ccgo publish apple --manager spm --push

# This creates git tag and pushes to remote
# SPM users can then reference your repository
```

**Manual SPM publishing:**

```bash
# Create version tag
git tag 1.0.0
git push origin 1.0.0

# Users add to Package.swift:
# .package(url: "https://github.com/user/repo.git", from: "1.0.0")
```

### Publishing Both

```bash
# Publish to both CocoaPods and SPM
ccgo publish apple --manager all --push

# This:
# 1. Publishes to CocoaPods Trunk
# 2. Creates git tag for SPM
# 3. Pushes tag to remote
```

## OpenHarmony Publishing (OHPM)

### Registry Types

**official**: OpenHarmony Package Manager (ohpm.openharmony.cn)
- Official registry
- Requires account
- Public packages
- Globally accessible

**private**: Custom OHPM registry
- Company/team registries
- Authentication required
- Private packages

### Configuration

**Setup OHPM:**

```bash
# Install OHPM
npm install -g @ohos/hpm-cli

# Login to official registry
ohpm login

# Configure private registry
ohpm config set registry https://ohpm.example.com
```

### Publishing Commands

```bash
# Publish to official OHPM
ccgo publish ohos --registry official

# Publish to private OHPM
ccgo publish ohos --registry private --url https://ohpm.example.com

# Publish with authentication
ccgo publish ohos --registry private \
    --url https://ohpm.example.com \
    --token your-auth-token

# Skip build and use existing HAR
ccgo publish ohos --skip-build
```

### HAR Publishing Process

1. CCGO builds HAR package
2. Validates oh-package.json5
3. Uploads to registry
4. Registry validates package
5. Package becomes available

**Required metadata:**

```toml
[package]
name = "mylib"
version = "1.0.0"
description = "My OpenHarmony library"
authors = ["Your Name <your@email.com>"]
license = "MIT"
homepage = "https://github.com/user/mylib"
repository = "https://github.com/user/mylib"
```

## Conan Publishing

### Registry Types

**local**: Conan local cache (~/.conan/data/)
- No network required
- Immediate availability
- Testing only

**official**: First configured Conan remote
- Usually Conan Center
- Public packages
- Requires review

**private**: Custom Conan remote
- Company repositories
- Authentication required
- Immediate availability

### Configuration

**Setup Conan:**

```bash
# Install Conan
pip install conan

# Add Conan Center
conan remote add conancenter https://center.conan.io

# Add private remote
conan remote add myrepo https://conan.example.com
conan user -p password -r myrepo username
```

### Publishing Commands

```bash
# Export to local cache
ccgo publish conan --registry local

# Publish to Conan Center (requires PR)
ccgo publish conan --registry official

# Publish to private remote
ccgo publish conan --registry private \
    --remote-name myrepo \
    --url https://conan.example.com

# Skip build, only export recipe
ccgo publish conan --skip-build
```

### Conan Package Recipe

CCGO generates `conanfile.py`:

```python
from conan import ConanFile
from conan.tools.files import copy

class MylibConan(ConanFile):
    name = "mylib"
    version = "1.0.0"
    description = "My C++ library"
    license = "MIT"
    url = "https://github.com/user/mylib"

    settings = "os", "compiler", "build_type", "arch"
    options = {"shared": [True, False]}
    default_options = {"shared": False}

    def package(self):
        copy(self, "*.h", src=self.source_folder, dst=self.package_folder)
        copy(self, "*.a", src=self.build_folder, dst=self.package_folder)
        copy(self, "*.so", src=self.build_folder, dst=self.package_folder)
```

## Documentation Publishing (GitHub Pages)

### Setup

**Enable GitHub Pages:**

1. Repository Settings → Pages
2. Source: Deploy from branch
3. Branch: `gh-pages` (will be created by CCGO)

### Publishing

```bash
# Generate and publish documentation
ccgo publish doc --doc-branch gh-pages --doc-open

# Force push (overwrites existing docs)
ccgo publish doc --doc-branch gh-pages --doc-force

# Custom commit message
ccgo publish doc --doc-branch gh-pages --doc-message "Update docs for v1.0.0"
```

### Process

1. CCGO generates documentation with Doxygen
2. Converts to HTML
3. Creates `gh-pages` branch (if doesn't exist)
4. Commits documentation
5. Pushes to remote
6. Opens browser to docs URL

### Custom Domain

**Add CNAME file:**

```toml
[doc]
custom_domain = "docs.example.com"
```

CCGO creates `CNAME` file in gh-pages branch.

## Version Management

### Semantic Versioning

CCGO follows semantic versioning (semver):

```
MAJOR.MINOR.PATCH
1.0.0 -> 1.0.1 (bug fix)
1.0.1 -> 1.1.0 (new feature)
1.1.0 -> 2.0.0 (breaking change)
```

### Version in CCGO.toml

```toml
[package]
name = "mylib"
version = "1.2.3"  # Used for all publications
```

### Version Tagging

```bash
# Create version tag
ccgo tag

# Custom tag
ccgo tag v1.2.3 --message "Release version 1.2.3"

# This creates git tag matching CCGO.toml version
```

## Authentication

### Credential Storage

**Environment variables:**

```bash
# Maven Central
export SONATYPE_USERNAME=your-username
export SONATYPE_PASSWORD=your-password

# Private Maven
export PRIVATE_MAVEN_URL=https://maven.example.com
export PRIVATE_MAVEN_USERNAME=admin
export PRIVATE_MAVEN_PASSWORD=secret

# OHPM token
export OHPM_TOKEN=your-token

# Conan
export CONAN_LOGIN_USERNAME=your-username
export CONAN_PASSWORD=your-password
```

**Configuration files:**

```bash
# Gradle: ~/.gradle/gradle.properties
SONATYPE_USERNAME=your-username
SONATYPE_PASSWORD=your-password

# OHPM: ~/.ohpm/auth.json
{
  "registry": {
    "https://ohpm.openharmony.cn": {
      "token": "your-token"
    }
  }
}

# Conan: ~/.conan/remotes.json
{
  "remotes": [
    {
      "name": "myrepo",
      "url": "https://conan.example.com",
      "verify_ssl": true
    }
  ]
}
```

### Security Best Practices

1. **Never commit credentials** to repository
2. **Use environment variables** in CI/CD
3. **Rotate tokens regularly**
4. **Use read-only tokens** where possible
5. **Enable 2FA** on package registries

## CI/CD Integration

### GitHub Actions

```yaml
name: Publish Library

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.10'

      - name: Install CCGO
        run: pip install ccgo

      - name: Publish to Maven Central
        env:
          SONATYPE_USERNAME: ${{ secrets.SONATYPE_USERNAME }}
          SONATYPE_PASSWORD: ${{ secrets.SONATYPE_PASSWORD }}
        run: ccgo publish android --registry official

      - name: Publish to CocoaPods
        env:
          COCOAPODS_TRUNK_TOKEN: ${{ secrets.COCOAPODS_TRUNK_TOKEN }}
        run: ccgo publish apple --manager cocoapods
```

### GitLab CI

```yaml
publish:
  stage: deploy
  only:
    - tags
  script:
    - pip install ccgo
    - ccgo publish android --registry official
    - ccgo publish ohos --registry official
  variables:
    SONATYPE_USERNAME: $SONATYPE_USERNAME
    SONATYPE_PASSWORD: $SONATYPE_PASSWORD
    OHPM_TOKEN: $OHPM_TOKEN
```

### Jenkins

```groovy
pipeline {
    agent any

    stages {
        stage('Publish') {
            when {
                tag '*'
            }
            steps {
                sh 'pip install ccgo'
                withCredentials([
                    usernamePassword(
                        credentialsId: 'maven-central',
                        usernameVariable: 'SONATYPE_USERNAME',
                        passwordVariable: 'SONATYPE_PASSWORD'
                    )
                ]) {
                    sh 'ccgo publish android --registry official'
                }
            }
        }
    }
}
```

## Best Practices

### 1. Test Locally First

Always test with local registries:

```bash
# Test Maven publishing
ccgo publish android --registry local

# Verify installation
# (in consumer project)
implementation 'com.example:mylib:1.0.0'
```

### 2. Version Consistency

Ensure version matches across:
- CCGO.toml
- Git tags
- Package metadata

```bash
# CCGO handles this automatically
ccgo tag        # Creates tag from CCGO.toml
ccgo publish    # Uses CCGO.toml version
```

### 3. Changelog

Maintain CHANGELOG.md:

```markdown
# Changelog

## [1.2.0] - 2024-01-15
### Added
- New feature X
- Support for platform Y

### Fixed
- Bug in module Z

## [1.1.0] - 2024-01-01
...
```

### 4. Publishing Checklist

Before publishing:

- [ ] Update version in CCGO.toml
- [ ] Update CHANGELOG.md
- [ ] Run tests: `ccgo test`
- [ ] Test local build: `ccgo build`
- [ ] Test local publish: `ccgo publish <platform> --registry local`
- [ ] Create git tag: `ccgo tag`
- [ ] Publish: `ccgo publish <platform> --registry official`
- [ ] Verify package is available
- [ ] Update documentation
- [ ] Announce release

### 5. Multi-Platform Publishing

Publish to all platforms:

```bash
#!/bin/bash
# publish-all.sh

VERSION=$(ccgo version)

echo "Publishing version $VERSION to all platforms..."

# Android
ccgo publish android --registry official

# iOS/macOS
ccgo publish apple --manager all --push

# OpenHarmony
ccgo publish ohos --registry official

# Conan
ccgo publish conan --registry official

# Documentation
ccgo publish doc --doc-branch gh-pages

echo "Publishing complete!"
```

## Troubleshooting

### Maven Publishing Failed

```
Error: Failed to upload to Maven Central
```

**Solutions:**

1. **Check credentials:**
```bash
echo $SONATYPE_USERNAME
echo $SONATYPE_PASSWORD
```

2. **Verify PGP signing:**
```bash
gpg --list-secret-keys
cat ~/.gradle/gradle.properties | grep signing
```

3. **Check network:**
```bash
curl -I https://s01.oss.sonatype.org/
```

4. **Validate POM:**
```bash
# Check generated POM
cat build/publications/release/pom-default.xml
```

### CocoaPods Push Failed

```
Error: Unable to find a pod with name 'MyLib'
```

**Solutions:**

1. **Verify trunk registration:**
```bash
pod trunk me
```

2. **Validate podspec:**
```bash
pod spec lint MyLib.podspec --verbose
```

3. **Check spec repo:**
```bash
pod repo list
pod repo update
```

### OHPM Publishing Failed

```
Error: Package already exists
```

**Solutions:**

1. **Increment version:**
```toml
[package]
version = "1.0.1"  # Bump version
```

2. **Check existing package:**
```bash
ohpm view mylib
```

3. **Verify authentication:**
```bash
ohpm whoami
```

### Conan Upload Failed

```
Error: Recipe 'mylib/1.0.0' already exists
```

**Solutions:**

1. **Remove existing version:**
```bash
conan remove mylib/1.0.0 -r myrepo
```

2. **Use new version:**
```toml
[package]
version = "1.0.1"
```

3. **Check remote configuration:**
```bash
conan remote list
conan user -r myrepo
```

## See Also

- [Build System](build-system.md)
- [Dependency Management](dependency-management.md)
- [Android Platform](../platforms/android.md)
- [iOS Platform](../platforms/ios.md)
- [OpenHarmony Platform](../platforms/openharmony.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
