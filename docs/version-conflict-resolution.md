# Version Conflict Resolution

## Overview

CCGO provides **intelligent version conflict resolution** using semantic versioning to automatically detect and resolve dependency version conflicts. This ensures your project uses compatible versions of all dependencies.

## Benefits

- 🎯 **Automatic Detection** - Identifies version conflicts during dependency resolution
- 🔄 **Smart Resolution** - Uses semver rules to find compatible versions
- 📊 **Clear Reporting** - Shows detailed conflict information
- ✅ **Correctness Guarantees** - Only allows compatible versions
- 🚀 **Zero Configuration** - Works automatically

## How It Works

### Semantic Versioning

CCGO uses [Semantic Versioning (SemVer)](https://semver.org/) for version resolution:

- **Format**: `MAJOR.MINOR.PATCH` (e.g., `1.2.3`)
- **Version Ranges**: Support for `^`, `~`, `>=`, `<`, etc.
- **Compatibility**: Determines if versions can work together

### Version Requirements

CCGO supports several version requirement formats:

| Format | Example | Meaning |
|--------|---------|---------|
| **Exact** | `1.2.3` | Exactly version 1.2.3 |
| **Caret** | `^1.2.3` | >= 1.2.3, < 2.0.0 (compatible) |
| **Tilde** | `~1.2.3` | >= 1.2.3, < 1.3.0 (patch updates) |
| **Wildcard** | `1.2.*` or `*` | Any patch version or any version |
| **Range** | `>=1.0, <2.0` | Multiple constraints |

### Conflict Detection

CCGO detects conflicts when:
1. Multiple dependencies require different versions of the same package
2. The required versions are **incompatible** according to semver rules

```
Project
  ├─ dep_a (requires fmt@^10.0.0)
  └─ dep_b (requires fmt@^11.0.0)   ← CONFLICT!
```

### Resolution Strategy

When conflicts are detected, CCGO:

1. **Analyzes Requirements** - Checks all version requirements for each package
2. **Finds Compatible Version** - Uses semver to find a version that satisfies all requirements
3. **Selects Highest Version** - Prefers the highest compatible version
4. **Reports Failures** - If no compatible version exists, reports detailed error

## Usage

### Automatic Resolution

Version conflict resolution happens automatically during `ccgo install`:

```bash
$ ccgo install

📊 Resolving dependency graph...
⚠️  Detected 1 version conflicts:

   Package: fmt
      dep_a requires ^10.0.0
      dep_b requires 10.1.0
   ✓ Resolved to: 10.1.0

✓ Dependency graph resolved
```

### Compatible Requirements

When requirements are compatible, CCGO silently resolves them:

```toml
# Project CCGO.toml
[[dependencies]]
name = "dep_a"
# dep_a requires fmt@^10.0.0

[[dependencies]]
name = "dep_b"
# dep_b requires fmt@10.1.0
```

**Resolution**: Uses `10.1.0` (satisfies both `^10.0.0` and `10.1.0`)

### Incompatible Requirements

When requirements conflict, CCGO reports an error:

```bash
$ ccgo install

📊 Resolving dependency graph...
⚠️  Detected 1 version conflicts:

   Package: fmt
      dep_a requires 10.0.0
      dep_b requires 11.0.0

Error: Cannot resolve version conflict for 'fmt': incompatible requirements
  - dep_a requires 10.0.0
  - dep_b requires 11.0.0
```

## Examples

### Example 1: Caret Range Compatibility

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "fmt"
version = "^10.0.0"   # Allows 10.x.x, < 11.0.0

# dep_b/CCGO.toml
[[dependencies]]
name = "fmt"
version = "10.2.1"    # Specific version within range
```

**Result**: ✅ Resolved to `10.2.1` (satisfies both requirements)

### Example 2: Tilde Range

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "spdlog"
version = "~1.11.0"   # Allows 1.11.x patches

# dep_b/CCGO.toml
[[dependencies]]
name = "spdlog"
version = "1.11.2"    # Patch version
```

**Result**: ✅ Resolved to `1.11.2`

### Example 3: Major Version Conflict

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "json"
version = "3.10.0"    # Version 3.x

# dep_b/CCGO.toml
[[dependencies]]
name = "json"
version = "4.0.0"     # Version 4.x
```

**Result**: ❌ Error - incompatible major versions

### Example 4: Wildcard

```toml
# dep_a/CCGO.toml
[[dependencies]]
name = "catch2"
version = "*"         # Any version

# dep_b/CCGO.toml
[[dependencies]]
name = "catch2"
version = "3.4.0"     # Specific version
```

**Result**: ✅ Resolved to `3.4.0` (wildcard accepts any)

## Common Scenarios

### Scenario 1: Diamond Dependency

```
    Project
   /        \
  A          B
   \        /
    C@1.0  C@1.1
```

If C@1.1 is compatible with A's requirement (e.g., A needs `^1.0`):
- **Resolution**: Use C@1.1 ✅

If not compatible (e.g., A needs exactly `1.0.0`):
- **Resolution**: Error ❌

### Scenario 2: Deep Transitive Conflict

```
Project → A → B → C@2.0
Project → D → E → C@3.0
```

Even with deep nesting, CCGO detects the conflict between C@2.0 and C@3.0.

### Scenario 3: Multiple Paths to Same Dependency

```
Project → A → C@^1.0
Project → B → C@^1.0
Project → D → C@1.2.0
```

All three requirements are compatible - resolved to C@1.2.0 ✅

## Version Requirement Best Practices

### DO

✅ **Use caret ranges** for libraries
```toml
version = "^1.2.3"   # Allows compatible updates
```

✅ **Use exact versions** for critical dependencies
```toml
version = "2.5.0"    # Pin specific version
```

✅ **Keep major versions aligned** across your project
```toml
# Good: All use fmt v10
dep_a = { version = "^10.0.0" }
dep_b = { version = "10.1.0" }
```

✅ **Update regularly** to avoid accumulating conflicts
```bash
ccgo update
```

### DON'T

❌ **Don't use wildcards in production**
```toml
version = "*"        # Unpredictable versions
```

❌ **Don't mix major versions** unnecessarily
```toml
# Bad: Different major versions
dep_a = "1.0.0"      # v1
dep_b = "2.0.0"      # v2  ← Will likely conflict
```

❌ **Don't over-constrain** dependencies
```toml
version = "=1.2.3"   # Too restrictive, hard to resolve
```

## Troubleshooting

### Conflict Cannot Be Resolved

**Symptom**: Error message about incompatible requirements

**Solution 1 - Update Dependencies**:
```bash
# Update dependencies to compatible versions
ccgo update

# Check available versions
ccgo search <package_name>
```

**Solution 2 - Adjust Version Requirements**:
```toml
# Before (too restrictive)
[[dependencies]]
name = "fmt"
version = "10.0.0"    # Exact version

# After (more flexible)
[[dependencies]]
name = "fmt"
version = "^10.0.0"   # Allow compatible versions
```

**Solution 3 - Contact Maintainers**:
If your dependencies have conflicting requirements, contact the maintainers to:
- Request version updates
- Report compatibility issues
- Suggest version range adjustments

### Understanding Conflict Reports

```
⚠️  Detected 1 version conflicts:

   Package: boost
      graphics_lib requires ^1.75.0
      network_lib requires ^1.80.0
      core_lib requires 1.76.0
```

**Analysis**:
- `graphics_lib` needs Boost 1.75+ (< 2.0)
- `network_lib` needs Boost 1.80+ (< 2.0)
- `core_lib` needs exactly 1.76.0

**Issue**: core_lib's exact version (1.76.0) conflicts with network_lib's minimum (1.80.0)

**Fix**: Update core_lib to use `^1.76.0` instead of `1.76.0`

### Version Not Found

**Symptom**: "Cannot extract version from range" error

**Cause**: Invalid or unsupported version format

**Solution**: Use standard semver format
```toml
# Bad
version = "v1.2.3"     # Don't use 'v' prefix
version = "1.2"        # Missing patch version

# Good
version = "1.2.3"      # Complete semver
version = "^1.2.0"     # Valid range
```

## Advanced Topics

### Custom Version Resolution

Currently, CCGO uses automatic resolution. Future versions may support:

```toml
[resolution]
# Force specific versions (override conflicts)
fmt = "10.1.0"
boost = "1.80.0"
```

### Version Lock File

To ensure reproducible builds, CCGO will support a lock file:

```bash
# Generate lock file
ccgo install

# This creates CCGO.lock with exact resolved versions

# Use locked versions
ccgo install --locked
```

### Conflict Resolution Strategies

Future strategies may include:

1. **Highest Compatible** (current) - Choose highest version that satisfies all requirements
2. **Lowest Compatible** - Choose lowest version (more conservative)
3. **Latest Available** - Always use latest available version
4. **User Specified** - Manual override in CCGO.toml

## Implementation Details

### Version Comparison Algorithm

```rust
// Pseudocode
fn is_compatible(req1: &VersionReq, req2: &VersionReq) -> bool {
    // Check if both requirements can be satisfied by some version
    for candidate_version in all_versions {
        if req1.matches(candidate_version) && req2.matches(candidate_version) {
            return true;
        }
    }
    false
}
```

### Conflict Resolution Algorithm

```rust
// Pseudocode
fn resolve_conflict(requirements: Vec<VersionReq>) -> Result<Version> {
    // Find highest version that satisfies all requirements
    let mut candidates = vec![];

    for req in requirements {
        let versions = extract_versions_from(req);
        candidates.extend(versions);
    }

    // Sort by version (highest first)
    candidates.sort_by(|a, b| b.cmp(a));

    // Find first version that satisfies all requirements
    for version in candidates {
        if requirements.iter().all(|req| req.matches(&version)) {
            return Ok(version);
        }
    }

    Err("No compatible version found")
}
```

## See Also

- [Dependency Management](dependency-management.md) - Overall dependency system
- [Semantic Versioning](https://semver.org/) - SemVer specification
- [Dependency Graph](dependency-graph.md) - Dependency graph visualization

## Changelog

### v3.0.12 (2026-01-21)

- ✅ Implemented version conflict detection
- ✅ Semantic versioning support (exact, ranges, wildcards)
- ✅ Smart conflict resolution with highest compatible version
- ✅ Detailed conflict reporting
- ✅ Caret (^), tilde (~), and range operators support
- ✅ Comprehensive error messages with resolution hints

---

*Version conflict resolution ensures your project uses compatible dependency versions automatically.*
