# Dependency Management

Complete guide to managing dependencies in CCGO projects.

## Overview

CCGO provides a powerful dependency management system with:

- **Multiple sources**: Git repositories, local paths, registries (future)
- **Version control**: Semantic versioning with flexible constraints
- **Lock files**: Reproducible builds with CCGO.lock
- **Optional dependencies**: Feature-based conditional dependencies
- **Platform-specific**: Dependencies for specific platforms only
- **Vendoring**: Local copies of all dependencies
- **Automatic resolution**: Dependency tree resolution and conflict detection

## Dependency Sources

### Git Repository

Most common source for C++ libraries:

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", branch = "master" }
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280a619340f3f76dc77292b7e9d1b1c2d83e" }
```

**Git reference types:**
- `tag`: Specific release tag (recommended for stability)
- `branch`: Track a branch (for latest features/fixes)
- `rev`: Exact commit hash (for maximum reproducibility)

**Rules:**
- Only ONE of `tag`, `branch`, or `rev` can be specified
- Git dependencies are cached in `~/.ccgo/git/<repo>/`
- Shallow clones used by default for faster downloads

### Local Path

For local development and workspace dependencies:

```toml
[dependencies]
mylib = { path = "../mylib" }
utils = { path = "./libs/utils" }
common = { path = "/absolute/path/to/common" }
```

**Path types:**
- Relative paths: Relative to current CCGO.toml location
- Absolute paths: Full filesystem path

**Use cases:**
- Multi-module projects
- Local development of dependencies
- Vendored third-party libraries

### Registry (Future)

Future support for package registries:

```toml
[dependencies]
fmt = "10.1.1"              # Exact version
spdlog = "^1.12.0"          # Compatible (>=1.12.0, <2.0.0)
boost = "~1.80"             # Minor version (>=1.80.0, <1.81.0)
protobuf = ">=3.20"         # Minimum version
```

## Version Requirements

### Exact Version

```toml
[dependencies]
fmt = "10.1.1"              # Only version 10.1.1
```

### Caret (Compatible)

```toml
[dependencies]
spdlog = "^1.12.0"          # >=1.12.0, <2.0.0
```

Allows minor and patch updates, no breaking changes.

### Tilde (Minor Version)

```toml
[dependencies]
boost = "~1.80.0"           # >=1.80.0, <1.81.0
```

Allows patch updates only.

### Comparison Operators

```toml
[dependencies]
protobuf = ">=3.20.0"       # Any version >= 3.20.0
openssl = ">1.1.0, <3.0.0"  # Version between 1.1.0 and 3.0.0
```

### Wildcard

```toml
[dependencies]
catch2 = "3.*"              # Any 3.x version
```

## Managing Dependencies

### Adding Dependencies

**Command line:**

```bash
# Add from Git with tag
ccgo add spdlog --git https://github.com/gabime/spdlog.git --tag v1.12.0

# Add from Git with branch
ccgo add fmt --git https://github.com/fmtlib/fmt.git --branch master

# Add from local path
ccgo add mylib --path ../mylib

# Add from Git with specific commit
ccgo add json --git https://github.com/nlohmann/json.git --rev a1b2c3d
```

**Manual CCGO.toml edit:**

```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

Then install:
```bash
ccgo install
```

### Removing Dependencies

**Command line:**

```bash
ccgo remove spdlog
```

**Manual:**

Remove from CCGO.toml, then:
```bash
ccgo install
```

### Updating Dependencies

**Update all dependencies:**

```bash
ccgo update
```

**Update specific dependency:**

```bash
ccgo update spdlog
```

**Dry run (show what would be updated):**

```bash
ccgo update --dry-run
```

**Update process:**
1. Fetches latest versions matching constraints
2. Updates CCGO.lock with new versions
3. Downloads/updates cached copies
4. Rebuilds project if needed

## CCGO.lock File

### Purpose

`CCGO.lock` ensures reproducible builds by recording:
- Exact dependency versions resolved
- Git commit hashes for Git dependencies
- Dependency tree structure
- Checksums for verification

### Structure

```toml
# CCGO.lock - Auto-generated, do not edit manually

[[package]]
name = "spdlog"
version = "1.12.0"
source = "git+https://github.com/gabime/spdlog.git?tag=v1.12.0#a1b2c3d4"
checksum = "sha256:..."
dependencies = ["fmt"]

[[package]]
name = "fmt"
version = "10.1.1"
source = "git+https://github.com/fmtlib/fmt.git?tag=10.1.1#e1f2g3h4"
checksum = "sha256:..."
dependencies = []
```

### Working with Lock Files

**Generate lock file:**

```bash
ccgo install          # Creates/updates CCGO.lock
```

**Use locked versions:**

```bash
ccgo install --locked # Install exact versions from CCGO.lock
```

**Update lock file:**

```bash
ccgo update           # Updates CCGO.lock with new versions
```

**Version control:**
- **Commit** CCGO.lock for applications (reproducible builds)
- **Don't commit** CCGO.lock for libraries (let users resolve)

## Optional Dependencies

### Defining Optional Dependencies

```toml
[dependencies]
# Required dependencies
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }

# Optional dependencies
[dependencies.optional]
networking = { git = "https://github.com/user/networking.git", tag = "v1.0.0" }
database = { git = "https://github.com/user/database.git", tag = "v2.0.0" }
```

### Features

Enable optional dependencies with features:

```toml
[features]
default = ["basic"]           # Default features
basic = []                    # Basic feature (no deps)
network = ["networking"]      # Enables networking dependency
db = ["database"]             # Enables database dependency
full = ["basic", "network", "db"]  # All features
```

### Using Features

```bash
# Build with specific features
ccgo build --features network,db

# Build with all features
ccgo build --all-features

# Build without default features
ccgo build --no-default-features

# Build with specific features combination
ccgo build --no-default-features --features network
```

## Platform-Specific Dependencies

### Conditional Dependencies

```toml
[dependencies]
# Common dependencies for all platforms
common = { git = "https://github.com/user/common.git", tag = "v1.0.0" }

# Android-only dependencies
[target.'cfg(target_os = "android")'.dependencies]
android-utils = { path = "./android-utils" }

# iOS-only dependencies
[target.'cfg(target_os = "ios")'.dependencies]
ios-helpers = { path = "./ios-helpers" }

# Windows-only dependencies
[target.'cfg(target_os = "windows")'.dependencies]
windows-api = { git = "https://github.com/user/windows-api.git", tag = "v1.0.0" }

# Linux-only dependencies
[target.'cfg(target_os = "linux")'.dependencies]
linux-sys = { git = "https://github.com/user/linux-sys.git", tag = "v1.0.0" }
```

### Platform Targets

| Platform | Target | Example |
|----------|--------|---------|
| Android | `target_os = "android"` | Android-specific |
| iOS | `target_os = "ios"` | iOS-specific |
| macOS | `target_os = "macos"` | macOS-specific |
| Windows | `target_os = "windows"` | Windows-specific |
| Linux | `target_os = "linux"` | Linux-specific |
| OpenHarmony | `target_os = "ohos"` | OHOS-specific |

## Vendoring

### What is Vendoring?

Vendoring creates local copies of all dependencies in your project:
- Ensures availability even if upstream repositories disappear
- Enables offline builds
- Provides complete control over dependency source

### Vendor Dependencies

```bash
# Vendor all dependencies to vendor/ directory
ccgo vendor

# Keep existing vendor directory
ccgo vendor --no-delete
```

**Result:**

```
project/
├── CCGO.toml
├── vendor/
│   ├── spdlog/          # Full copy of spdlog
│   ├── fmt/             # Full copy of fmt
│   └── json/            # Full copy of json
└── src/
```

### Using Vendored Dependencies

CCGO automatically uses vendored dependencies if `vendor/` exists:

```bash
# Uses vendored copies
ccgo build android

# Force offline build (only vendored deps)
ccgo install --offline
```

### Version Control with Vendoring

**Small projects:**
- Commit vendor/ directory
- Self-contained repository

**Large projects:**
- Add `vendor/` to `.gitignore`
- Document vendoring process in README
- Consider Git LFS for large vendored files

## Dependency Resolution

### Resolution Algorithm

CCGO uses the following algorithm:

1. **Parse dependency tree**: Read CCGO.toml and build full dependency graph
2. **Version resolution**: Find versions that satisfy all constraints
3. **Conflict detection**: Detect version conflicts
4. **Download**: Fetch missing dependencies
5. **Build order**: Topological sort for build order

### Conflict Resolution

**Example conflict:**

```toml
# Your project
[dependencies]
libA = { git = "...", tag = "v1.0.0" }  # Depends on libC ^1.0.0
libB = { git = "...", tag = "v2.0.0" }  # Depends on libC ^2.0.0
```

CCGO will report:
```
Error: Conflicting dependencies for libC:
  - libA requires libC ^1.0.0
  - libB requires libC ^2.0.0
```

**Solutions:**
1. Update libA or libB to compatible versions
2. Use fork with compatible version
3. Modify dependency constraints

### Build Order

CCGO builds dependencies in correct order:

```
Project
  ├── libA (depends on libC)
  ├── libB (depends on libC)
  └── libC (no dependencies)
```

**Build order:** libC → libA, libB → Project

## Best Practices

### 1. Pin Dependency Versions

**Good:**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

**Bad:**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", branch = "master" }
```

**Reason:** Tagged versions ensure reproducible builds.

### 2. Use CCGO.lock

```bash
# Generate lock file
ccgo install

# Commit to version control (for applications)
git add CCGO.lock
git commit -m "Add dependency lock file"
```

### 3. Minimal Dependencies

Only add dependencies you actually need:
- Reduces build time
- Simplifies dependency management
- Improves security (fewer attack vectors)

### 4. Document Dependencies

**README.md:**
```markdown
## Dependencies

- spdlog (v1.12.0): Logging library
- fmt (v10.1.1): Formatting library
- nlohmann/json (v3.11.2): JSON parsing
```

### 5. Regular Updates

```bash
# Check for updates monthly
ccgo update --dry-run

# Update after testing
ccgo update
ccgo build android --test
```

### 6. Security Considerations

- Review dependency source code before adding
- Use official repositories
- Check for security advisories
- Keep dependencies updated
- Vendor critical dependencies

## Advanced Usage

### Dependency Patches

Patches allow you to override dependency sources, similar to Cargo's `[patch]` feature. This is useful for:
- Testing bug fixes before they're merged upstream
- Using a fork with custom changes
- Applying local patches to third-party dependencies
- Working around version conflicts

#### Basic Patch Syntax

Patches are defined in the `[patch]` section of CCGO.toml:

```toml
[dependencies]
mylib = { git = "https://github.com/user/mylib.git", tag = "v1.0.0" }

# Patch a dependency with a fork
[patch."https://github.com/user/mylib.git"]
fmt = { git = "https://github.com/me/fmt-fork.git", branch = "bugfix" }
```

#### Registry Patches (Future)

For dependencies from a package registry:

```toml
[dependencies]
mylib = "1.0.0"

# Patch registry dependency
[patch.crates-io]
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.2.1" }
```

#### Source-Specific Patches

Override dependencies from specific sources:

```toml
[dependencies]
libA = { git = "https://github.com/user/libA.git", tag = "v1.0.0" }
libB = { git = "https://github.com/user/libB.git", tag = "v2.0.0" }

# Patch libA's dependency on common-utils
[patch."https://github.com/user/libA.git"]
common-utils = { git = "https://github.com/me/common-utils-fork.git", tag = "v1.5-custom" }

# Patch libB's dependency on common-utils (different patch)
[patch."https://github.com/user/libB.git"]
common-utils = { git = "https://github.com/me/common-utils.git", rev = "abc123" }
```

**Priority:** Source-specific patches take precedence over registry patches.

#### Local Path Patches

Use local versions for development:

```toml
[dependencies]
myapp = { git = "https://github.com/user/myapp.git", tag = "v1.0.0" }

# Use local version of fmt for testing
[patch."https://github.com/user/myapp.git"]
fmt = { path = "../fmt-local" }
```

#### Git Patch Options

All git options are supported in patches:

```toml
[patch.crates-io]
# Specific branch
spdlog = { git = "https://github.com/gabime/spdlog.git", branch = "v1.x" }

# Specific tag
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.2.1" }

# Exact revision
json = { git = "https://github.com/nlohmann/json.git", rev = "9cca280a" }
```

#### Patch Resolution Rules

1. **Source matching**: Patches are matched against the original dependency source
2. **Priority order**: Source-specific patches > Registry patches
3. **Lockfile records**: Patches are tracked in CCGO.lock with original and replacement sources
4. **Locked mode**: In `ccgo install --locked`, locked revisions are respected

#### Example: Testing Upstream Fix

```toml
[dependencies]
mylib = { git = "https://github.com/upstream/mylib.git", tag = "v1.0.0" }

# Test a pending PR before it's merged
[patch."https://github.com/upstream/mylib.git"]
problematic-dep = {
    git = "https://github.com/contributor/problematic-dep.git",
    branch = "fix-issue-123"
}
```

Then install:
```bash
ccgo install
```

Output shows patch is applied:
```
📦 Installing mylib...
   Source: https://github.com/upstream/mylib.git

📦 Installing problematic-dep...
   🔧 Applying patch for problematic-dep...
   Original: git+https://github.com/original/problematic-dep.git
   Patched:  git+https://github.com/contributor/problematic-dep.git
```

#### Example: Local Development Workflow

```toml
[dependencies]
production-lib = { git = "https://github.com/company/lib.git", tag = "v2.0.0" }

# Override with local development version
[patch."https://github.com/company/lib.git"]
utils = { path = "../utils-dev" }
logger = { path = "../logger-dev" }
```

This allows you to develop multiple dependencies simultaneously without modifying the main dependency definitions.

#### Lockfile Format

When patches are applied, CCGO.lock records both sources:

```toml
[[package]]
name = "fmt"
version = "10.2.1"
source = "git+https://github.com/me/fmt-fork.git#abc123"
git.revision = "abc123"
git.branch = "bugfix"
patch.patched_source = "git+https://github.com/fmtlib/fmt.git"
patch.replacement_source = "git+https://github.com/me/fmt-fork.git"
patch.is_path_patch = false
```

#### Removing Patches

To stop using a patch, simply remove it from CCGO.toml and re-run:

```bash
ccgo install --force
```

This will reinstall dependencies using their original sources.

### Private Git Repositories

**SSH authentication:**
```toml
[dependencies]
private-lib = { git = "git@github.com:company/private-lib.git", tag = "v1.0.0" }
```

**HTTPS with credentials:**
```bash
# Configure Git credentials
git config --global credential.helper store

# Or use SSH key
ssh-add ~/.ssh/id_rsa
```

### Workspace Dependencies

For mono-repo setups:

```
workspace/
├── CCGO.toml (workspace root)
├── lib1/
│   └── CCGO.toml
├── lib2/
│   └── CCGO.toml (depends on lib1)
└── app/
    └── CCGO.toml (depends on lib1, lib2)
```

**lib2/CCGO.toml:**
```toml
[dependencies]
lib1 = { path = "../lib1" }
```

**app/CCGO.toml:**
```toml
[dependencies]
lib1 = { path = "../lib1" }
lib2 = { path = "../lib2" }
```

## Troubleshooting

### Dependency Not Found

```
Error: Could not find dependency 'spdlog'
```

**Solutions:**
1. Check repository URL is correct
2. Verify Git tag/branch exists
3. Check network connectivity
4. Try manual clone: `git clone <url>`

### Version Conflict

```
Error: Conflicting versions for dependency 'fmt'
```

**Solutions:**
1. Run `ccgo update` to resolve conflicts
2. Manually adjust version constraints
3. Use `ccgo vendor` to lock specific versions

### Build Fails After Update

```
Error: Build failed after dependency update
```

**Solutions:**
1. Revert to previous CCGO.lock: `git checkout CCGO.lock`
2. Install locked versions: `ccgo install --locked`
3. Test dependencies individually
4. Check compatibility matrix

### Slow Dependency Resolution

```
Taking too long to resolve dependencies...
```

**Solutions:**
1. Use `--locked` to skip resolution
2. Vendor dependencies: `ccgo vendor`
3. Check network speed
4. Clear cache: `rm -rf ~/.ccgo/cache/`

### Git Authentication Failed

```
Error: Authentication failed for Git repository
```

**Solutions:**
1. Set up SSH keys: `ssh-keygen`
2. Add key to Git provider
3. Or use personal access token
4. Configure Git credentials manager

## Dependency Ecosystem

### Popular C++ Dependencies

**Logging:**
- spdlog: https://github.com/gabime/spdlog
- glog: https://github.com/google/glog

**Formatting:**
- fmt: https://github.com/fmtlib/fmt

**JSON:**
- nlohmann/json: https://github.com/nlohmann/json
- RapidJSON: https://github.com/Tencent/rapidjson

**Testing:**
- Catch2: https://github.com/catchorg/Catch2
- Google Test: https://github.com/google/googletest

**Networking:**
- Boost.Asio: https://github.com/boostorg/asio
- cpp-httplib: https://github.com/yhirose/cpp-httplib

**Serialization:**
- protobuf: https://github.com/protocolbuffers/protobuf
- flatbuffers: https://github.com/google/flatbuffers

### Finding Dependencies

- **GitHub**: Search for C++ libraries
- **Awesome C++**: https://github.com/fffaraz/awesome-cpp
- **Conan Center**: https://conan.io/center/
- **vcpkg**: https://github.com/microsoft/vcpkg

## Migration Guides

### From Conan

**conanfile.txt:**
```ini
[requires]
spdlog/1.12.0
fmt/10.1.1

[options]
spdlog:shared=False
```

**CCGO.toml:**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }

[library]
type = "static"
```

### From vcpkg

**vcpkg.json:**
```json
{
  "dependencies": [
    "spdlog",
    "fmt",
    "nlohmann-json"
  ]
}
```

**CCGO.toml:**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
fmt = { git = "https://github.com/fmtlib/fmt.git", tag = "10.1.1" }
json = { git = "https://github.com/nlohmann/json.git", tag = "v3.11.2" }
```

### From Git Submodules

**Replace:**
```bash
git submodule add https://github.com/gabime/spdlog.git third_party/spdlog
```

**With:**
```toml
[dependencies]
spdlog = { git = "https://github.com/gabime/spdlog.git", tag = "v1.12.0" }
```

Then:
```bash
ccgo install
```

## See Also

- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Build System](build-system.md)
- [Publishing](publishing.md)
- [Project Structure](../getting-started/project-structure.md)
