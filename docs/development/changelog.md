# Changelog Management

Guide to managing changelogs in CCGO projects following Keep a Changelog format and best practices.

## Overview

CCGO follows [Keep a Changelog](https://keepachangelog.com/) principles for documenting changes:

- **Human-Readable** - Changes organized by version and category
- **Machine-Parseable** - Structured format for automated tools
- **Git Integration** - Automated generation from git history
- **Semantic Versioning** - Tied to SemVer version numbers
- **Release Notes** - Foundation for release documentation

## Changelog Format

### Standard Structure

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- New features that have been added

### Changed
- Changes in existing functionality

### Deprecated
- Features that will be removed in upcoming releases

### Removed
- Features that have been removed

### Fixed
- Bug fixes

### Security
- Security improvements and vulnerability fixes

## [1.0.0] - 2024-01-15

### Added
- Initial release
- Cross-platform C++ build system
- Support for Android, iOS, macOS, Windows, Linux

[Unreleased]: https://github.com/user/project/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/user/project/releases/tag/v1.0.0
```

### Change Categories

| Category | Description | Example |
|----------|-------------|---------|
| **Added** | New features | Added ARM64 support for Android |
| **Changed** | Modifications to existing features | Updated CMake minimum version to 3.20 |
| **Deprecated** | Features marked for removal | Deprecated `--legacy-build` flag |
| **Removed** | Deleted features | Removed Python 2 support |
| **Fixed** | Bug fixes | Fixed memory leak in iOS framework |
| **Security** | Security fixes | Patched XSS vulnerability |

## Creating a Changelog

### Initial Setup

```bash
# Create CHANGELOG.md in project root
cat > CHANGELOG.md << 'EOF'
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project setup

[Unreleased]: https://github.com/user/project/compare/v0.1.0...HEAD
EOF
```

### Adding Changes

```markdown
## [Unreleased]

### Added
- Android ARM64-v8a architecture support
- Docker-based cross-platform builds for Windows and Linux
- Automatic version injection from git tags

### Changed
- Updated minimum CMake version from 3.18 to 3.20
- Improved error messages for missing dependencies

### Fixed
- Fixed iOS framework symbol visibility issues
- Resolved Windows DLL export problems in MinGW builds
```

### Releasing a Version

1. **Move unreleased changes to new version:**

```markdown
## [Unreleased]

## [1.2.0] - 2024-01-15

### Added
- Android ARM64-v8a architecture support
- Docker-based cross-platform builds

### Changed
- Updated minimum CMake version from 3.18 to 3.20

### Fixed
- Fixed iOS framework symbol visibility issues

[Unreleased]: https://github.com/user/project/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/user/project/compare/v1.1.0...v1.2.0
```

2. **Create git tag:**

```bash
ccgo tag v1.2.0 --message "Release 1.2.0"
```

3. **Update version links:**

```markdown
[Unreleased]: https://github.com/user/project/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/user/project/compare/v1.1.0...v1.2.0
[1.1.0]: https://github.com/user/project/compare/v1.0.0...v1.1.0
```

## Automated Changelog Generation

### Using ccgo changelog

```bash
# Generate changelog from git history
ccgo changelog

# Output to file
ccgo changelog --output CHANGELOG.md

# Between specific versions
ccgo changelog --from v1.0.0 --to v2.0.0

# Include all commits
ccgo changelog --include-all

# Group by type (conventional commits)
ccgo changelog --group-by-type
```

### Git Commit Parser

CCGO parses [Conventional Commits](https://www.conventionalcommits.org/) for automatic categorization:

```bash
# Commit format: <type>(<scope>): <subject>

feat(android): add ARM64 support       → Added
fix(ios): resolve memory leak          → Fixed
docs: update installation guide        → (Documentation, not in changelog)
chore: bump version to 1.2.0          → (Maintenance, not in changelog)
refactor(core): simplify error handling → Changed
test: add unit tests for calculator    → (Testing, not in changelog)
perf(network): optimize data transfer  → Changed (performance improvement)
```

**Type Mapping:**

| Commit Type | Changelog Category |
|-------------|-------------------|
| `feat` | Added |
| `fix` | Fixed |
| `perf` | Changed |
| `refactor` | Changed |
| `revert` | Changed |
| `docs` | (Not included) |
| `style` | (Not included) |
| `test` | (Not included) |
| `chore` | (Not included) |
| `build` | (Not included) |
| `ci` | (Not included) |

### Breaking Changes

```bash
# Commit with breaking change
git commit -m "feat(api)!: change function signature

BREAKING CHANGE: Calculator.add() now returns Result<int> instead of int"
```

**In Changelog:**

```markdown
## [2.0.0] - 2024-01-15

### Changed
- **BREAKING:** Calculator.add() now returns Result<int> instead of int

### Migration Guide
```cpp
// Before
int result = Calculator.add(2, 3);

// After
auto result = Calculator.add(2, 3);
if (result.is_ok()) {
    int value = result.value();
}
```
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Update Changelog
on:
  push:
    tags:
      - 'v*'

jobs:
  changelog:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
        with:
          fetch-depth: 0  # Full history for changelog generation

      - name: Install CCGO
        run: pip install ccgo

      - name: Generate Changelog
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")

          if [ -n "$PREV_TAG" ]; then
            ccgo changelog --from $PREV_TAG --to v$VERSION --output CHANGELOG.new.md
          else
            ccgo changelog --to v$VERSION --output CHANGELOG.new.md
          fi

      - name: Update Changelog
        run: |
          # Prepend new changes to existing changelog
          cat CHANGELOG.new.md CHANGELOG.md > CHANGELOG.tmp.md
          mv CHANGELOG.tmp.md CHANGELOG.md

      - name: Commit Changelog
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add CHANGELOG.md
          git commit -m "docs: update changelog for ${GITHUB_REF#refs/tags/}"
          git push
```

### GitLab CI

```yaml
update-changelog:
  stage: deploy
  only:
    - tags
  script:
    - pip install ccgo
    - |
      VERSION=$(echo $CI_COMMIT_TAG | sed 's/^v//')
      PREV_TAG=$(git describe --tags --abbrev=0 $CI_COMMIT_TAG^ 2>/dev/null || echo "")
      ccgo changelog --from $PREV_TAG --to $CI_COMMIT_TAG --output CHANGELOG.new.md
      cat CHANGELOG.new.md CHANGELOG.md > CHANGELOG.tmp.md
      mv CHANGELOG.tmp.md CHANGELOG.md
    - |
      git config user.name "GitLab CI"
      git config user.email "ci@gitlab.com"
      git add CHANGELOG.md
      git commit -m "docs: update changelog for $CI_COMMIT_TAG"
      git push https://oauth2:${CI_JOB_TOKEN}@${CI_SERVER_HOST}/${CI_PROJECT_PATH}.git HEAD:main
```

## Release Notes

### Generating Release Notes

```bash
# Generate release notes from changelog
ccgo release-notes v1.2.0

# Output:
# Release 1.2.0
#
# Added:
# - Android ARM64-v8a architecture support
# - Docker-based cross-platform builds
#
# Changed:
# - Updated minimum CMake version from 3.18 to 3.20
#
# Fixed:
# - Fixed iOS framework symbol visibility issues
```

### GitHub Release Integration

```yaml
name: Create GitHub Release
on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install CCGO
        run: pip install ccgo

      - name: Generate Release Notes
        id: release_notes
        run: |
          ccgo release-notes ${GITHUB_REF#refs/tags/} --output release_notes.md
          echo "notes_file=release_notes.md" >> $GITHUB_OUTPUT

      - name: Create GitHub Release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          body_path: ${{ steps.release_notes.outputs.notes_file }}
          draft: false
          prerelease: false
```

## Best Practices

### 1. Update Changelog with Every PR

```markdown
## [Unreleased]

### Added
- Feature X (#123)
- Feature Y (#124)

### Fixed
- Bug in feature A (#125)
```

Include PR/issue numbers for traceability.

### 2. Write User-Focused Descriptions

```markdown
# Good
- Added support for building on Apple Silicon Macs natively

# Bad
- Updated build_macos.py to detect arm64 architecture
```

### 3. Group Related Changes

```markdown
### Android

#### Added
- Support for Android 13 (API 33)
- New material design components

#### Fixed
- Crash on startup in Android 11
- Memory leak in background service

### iOS

#### Added
- iOS 16 widget support
- SwiftUI previews

#### Fixed
- App Store submission issues
```

### 4. Include Migration Guides for Breaking Changes

```markdown
## [2.0.0] - 2024-01-15

### Changed
- **BREAKING:** Renamed `ccgo build-all` to `ccgo build --all`

### Migration Guide

Update your build scripts:
```bash
# Before
ccgo build-all

# After
ccgo build --all
```
```

### 5. Link to Documentation

```markdown
### Added
- Docker-based cross-platform builds ([documentation](../features/docker-builds.md))
- New `--docker` flag for `ccgo build` command
```

## Changelog Tools

### changelog-cli

```bash
# Install changelog-cli
npm install -g changelog-cli

# Add entry
changelog add "Added new feature" --type added

# Remove entry
changelog remove "Old entry"

# Release version
changelog release 1.2.0
```

### git-cliff

```bash
# Install git-cliff
cargo install git-cliff

# Generate changelog
git-cliff --output CHANGELOG.md

# Generate for specific range
git-cliff v1.0.0..v2.0.0
```

### conventional-changelog

```bash
# Install conventional-changelog
npm install -g conventional-changelog-cli

# Generate changelog
conventional-changelog -p angular -i CHANGELOG.md -s

# First release
conventional-changelog -p angular -i CHANGELOG.md -s -r 0
```

## CCGO Project Changelog

### Example

```markdown
# Changelog

## [Unreleased]

### Added
- Rust-based CLI rewrite for improved performance
- Support for Apple Watch and Apple TV platforms
- Unified archive structure across all platforms

### Changed
- Migrated from Python argparse to Rust clap for CLI parsing

## [3.0.10] - 2024-01-15

### Added
- Git versioning with automatic commit SHA injection
- Unified archive naming convention
- Symbols package generation for all platforms

### Changed
- Improved Docker build performance with prebuilt images
- Updated Android NDK requirement to r21+

### Fixed
- Fixed Windows MinGW build symbol exports
- Resolved iOS framework code signing issues

### Security
- Updated OpenSSL dependency to 1.1.1w

## [3.0.9] - 2023-12-01

### Added
- Docker-based cross-platform builds
- Support for OpenHarmony platform

### Fixed
- Fixed macOS universal binary generation
- Resolved Linux RPATH issues

[Unreleased]: https://github.com/zhlinh/ccgo/compare/v3.0.10...HEAD
[3.0.10]: https://github.com/zhlinh/ccgo/compare/v3.0.9...v3.0.10
[3.0.9]: https://github.com/zhlinh/ccgo/releases/tag/v3.0.9
```

## Resources

### Specifications

- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)

### Tools

- [changelog-cli](https://github.com/mc706/changelog-cli)
- [git-cliff](https://github.com/orhun/git-cliff)
- [conventional-changelog](https://github.com/conventional-changelog/conventional-changelog)

### CCGO Documentation

- [Version Management](../features/version-management.md)
- [Git Integration](../features/git-integration.md)
- [Contributing Guide](contributing.md)

### Community

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions)
- [Issue Tracker](https://github.com/zhlinh/ccgo/issues)

## Next Steps

- [Contributing Guide](contributing.md)
- [Version Management](../features/version-management.md)
- [Git Integration](../features/git-integration.md)
- [CI/CD Setup](contributing.md#cicd-integration)
