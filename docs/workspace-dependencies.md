# Workspace Dependencies

## Overview

CCGO provides **workspace support** for managing multiple related C++ packages in a monorepo structure. Workspaces enable:

- 🏢 **Monorepo Management** - Organize multiple packages in a single repository
- 🔗 **Shared Dependencies** - Define dependencies once, use across all members
- 🔄 **Dependency Inheritance** - Members inherit workspace dependencies
- 📦 **Coordinated Builds** - Build multiple packages in dependency order
- 🎯 **Selective Builds** - Build specific members or all at once

## Benefits

- **Simplified Dependency Management** - Define shared dependencies in one place
- **Consistent Versions** - All members use the same versions of shared dependencies
- **Faster Development** - No need to publish internal dependencies
- **Flexible Configuration** - Members can override or extend workspace dependencies
- **Build Optimization** - Smart build ordering based on inter-member dependencies

## Quick Start

### 1. Create Workspace Root

Create a workspace with multiple packages:

```toml
# /workspace/CCGO.toml
[workspace]
members = ["core", "utils", "examples/*"]
exclude = ["examples/experimental"]

# Shared dependencies for all workspace members
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"

[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
```

### 2. Create Workspace Members

Each member has its own CCGO.toml:

```toml
# /workspace/core/CCGO.toml
[package]
name = "core"
version = "1.0.0"

# Inherit workspace dependency
[[dependencies]]
name = "fmt"
workspace = true

# Add additional features
[[dependencies]]
name = "spdlog"
workspace = true
features = ["async"]

# Member-specific dependency
[[dependencies]]
name = "boost"
version = "1.80.0"
```

### 3. Build Workspace

```bash
# Build all workspace members
ccgo build --workspace

# Build specific member
ccgo build --package core

# Build in dependency order
ccgo build --workspace --ordered
```

## Workspace Configuration

### Workspace Section

Define the workspace in the root `CCGO.toml`:

```toml
[workspace]
# Required: List of member packages (supports glob patterns)
members = [
    "core",           # Direct path
    "libs/*",         # All directories in libs/
    "examples/**"     # Recursive glob
]

# Optional: Exclude specific paths
exclude = [
    "examples/test",
    "old/*"
]

# Optional: Default members to build when no package specified
default_members = ["core", "utils"]

# Optional: Workspace-level resolver version (default: "2")
resolver = "2"

# Workspace-level dependencies
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

### Member Configuration

Each workspace member is a regular CCGO package with optional workspace inheritance:

```toml
[package]
name = "my-package"
version = "1.0.0"

# Inherit workspace dependency
[[dependencies]]
name = "fmt"
workspace = true

# Inherit and extend with features
[[dependencies]]
name = "spdlog"
workspace = true
features = ["async", "custom-formatter"]

# Inherit and override default_features
[[dependencies]]
name = "boost"
workspace = true
default_features = false
features = ["filesystem"]

# Member-specific dependency (not from workspace)
[[dependencies]]
name = "rapidjson"
version = "1.1.0"
```

## Workspace Member Discovery

### Glob Patterns

CCGO supports glob patterns for discovering workspace members:

```toml
[workspace]
members = [
    "core",              # Exact path
    "libs/*",            # All immediate children
    "examples/**",       # Recursive (all descendants)
    "tests/integration_*" # Pattern matching
]
```

**Pattern Syntax**:
- `*` - Matches any characters except path separator
- `**` - Matches any characters including path separators (recursive)
- `?` - Matches any single character
- `[abc]` - Matches any character in brackets

**Example Structure**:
```
workspace/
├── CCGO.toml (workspace root)
├── core/
│   └── CCGO.toml
├── libs/
│   ├── utils/
│   │   └── CCGO.toml
│   └── helpers/
│       └── CCGO.toml
└── examples/
    ├── basic/
    │   └── CCGO.toml
    └── advanced/
        └── CCGO.toml
```

With `members = ["core", "libs/*", "examples/*"]`, all 5 packages are discovered.

### Exclude Patterns

Exclude specific paths from workspace:

```toml
[workspace]
members = ["libs/*", "examples/*"]
exclude = [
    "examples/experimental",  # Exclude specific directory
    "libs/old"                # Exclude outdated code
]
```

## Dependency Inheritance

### Basic Inheritance

Members use `workspace = true` to inherit dependencies:

**Workspace Root**:
```toml
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

**Member**:
```toml
[[dependencies]]
name = "fmt"
workspace = true  # Inherits version 10.0.0
```

### Feature Extension

Members can add additional features:

**Workspace Root**:
```toml
[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
features = ["console"]
```

**Member**:
```toml
[[dependencies]]
name = "spdlog"
workspace = true
features = ["async"]  # Final features: ["console", "async"]
```

### Override default_features

Members can control default features:

**Workspace Root**:
```toml
[[workspace.dependencies]]
name = "boost"
version = "1.80.0"
default_features = true
```

**Member**:
```toml
[[dependencies]]
name = "boost"
workspace = true
default_features = false  # Override to disable defaults
features = ["filesystem"]  # Only enable specific features
```

### Mixed Dependencies

Members can have both workspace and member-specific dependencies:

```toml
[package]
name = "my-package"

# From workspace
[[dependencies]]
name = "fmt"
workspace = true

# Member-specific
[[dependencies]]
name = "rapidjson"
version = "1.1.0"

# From workspace with extensions
[[dependencies]]
name = "spdlog"
workspace = true
features = ["custom"]
```

## Build Order and Dependencies

### Inter-Member Dependencies

Workspace members can depend on each other:

```toml
# core/CCGO.toml
[package]
name = "core"

# utils/CCGO.toml
[package]
name = "utils"

[[dependencies]]
name = "core"
path = "../core"  # Depend on sibling package
```

### Topological Build Order

CCGO automatically determines build order based on dependencies:

```
workspace/
├── core/          (no dependencies)
├── utils/         (depends on core)
└── app/           (depends on utils)

Build order: core → utils → app
```

**Usage**:
```bash
# Build in dependency order
ccgo build --workspace --ordered

# Build fails if circular dependencies detected
```

### Circular Dependency Detection

CCGO detects and prevents circular dependencies:

```
core → utils → helpers → core  ❌ CIRCULAR!

Error: Circular dependency detected involving 'core'
```

## Commands

### Build Commands

**Build all workspace members**:
```bash
ccgo build --workspace
```

**Build specific member**:
```bash
ccgo build --package core
ccgo build -p utils
```

**Build multiple members**:
```bash
ccgo build --package core --package utils
```

**Build in dependency order**:
```bash
ccgo build --workspace --ordered
```

**Build default members only**:
```bash
# Uses workspace.default_members if specified
ccgo build
```

### List Commands

**List all workspace members**:
```bash
ccgo workspace list

# Output:
# Workspace: /path/to/workspace
# Members: 5
#   - core (1.0.0)
#   - utils (1.0.0)
#   - helpers (1.0.0)
#   - example-basic (0.1.0)
#   - example-advanced (0.1.0)
```

**List member dependencies**:
```bash
ccgo workspace deps core

# Output:
# Dependencies for core:
#   fmt 10.0.0 (from workspace)
#   spdlog 1.12.0 (from workspace)
#   boost 1.80.0 (member-specific)
```

### Check Commands

**Check workspace consistency**:
```bash
ccgo workspace check

# Validates:
# - All members have valid CCGO.toml
# - No duplicate member names
# - Workspace dependencies exist
# - No circular dependencies
```

## Examples

### Example 1: Simple Library Workspace

**Structure**:
```
mylib/
├── CCGO.toml
├── core/
│   ├── CCGO.toml
│   ├── include/
│   └── src/
└── utils/
    ├── CCGO.toml
    ├── include/
    └── src/
```

**Workspace Root** (`mylib/CCGO.toml`):
```toml
[workspace]
members = ["core", "utils"]

[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"

[[workspace.dependencies]]
name = "gtest"
version = "1.14.0"
```

**Core Package** (`mylib/core/CCGO.toml`):
```toml
[package]
name = "mylib-core"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true
```

**Utils Package** (`mylib/utils/CCGO.toml`):
```toml
[package]
name = "mylib-utils"
version = "1.0.0"

[[dependencies]]
name = "fmt"
workspace = true

[[dependencies]]
name = "mylib-core"
path = "../core"
```

**Build**:
```bash
cd mylib
ccgo build --workspace --ordered
# Builds: core first, then utils
```

### Example 2: Glob Pattern Discovery

**Structure**:
```
project/
├── CCGO.toml
├── core/
│   └── CCGO.toml
├── libs/
│   ├── utils/
│   │   └── CCGO.toml
│   ├── helpers/
│   │   └── CCGO.toml
│   └── common/
│       └── CCGO.toml
└── examples/
    ├── basic/
    │   └── CCGO.toml
    └── advanced/
        └── CCGO.toml
```

**Workspace Root**:
```toml
[workspace]
members = [
    "core",
    "libs/*",        # Discovers: utils, helpers, common
    "examples/*"     # Discovers: basic, advanced
]
exclude = []

# Total members: 6
```

### Example 3: Feature Extensions

**Workspace Root**:
```toml
[workspace]
members = ["server", "client", "shared"]

[[workspace.dependencies]]
name = "spdlog"
version = "1.12.0"
features = ["console"]  # Base feature
```

**Server** (`server/CCGO.toml`):
```toml
[package]
name = "server"

[[dependencies]]
name = "spdlog"
workspace = true
features = ["async", "multithreaded"]
# Final features: ["console", "async", "multithreaded"]
```

**Client** (`client/CCGO.toml`):
```toml
[package]
name = "client"

[[dependencies]]
name = "spdlog"
workspace = true
# Only uses workspace features: ["console"]
```

### Example 4: Default Members

**Workspace Root**:
```toml
[workspace]
members = ["core", "utils", "examples/*"]
default_members = ["core", "utils"]  # Don't build examples by default

[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

**Build Behavior**:
```bash
# Builds only core and utils (default members)
ccgo build

# Build all members including examples
ccgo build --workspace

# Build specific example
ccgo build --package example-advanced
```

## Advanced Usage

### Resolver Versions

Control dependency resolution behavior:

```toml
[workspace]
resolver = "2"  # Use resolver v2 (recommended)
members = ["core", "utils"]
```

**Resolver v1**:
- Legacy resolver
- May have edge cases with feature unification

**Resolver v2** (recommended):
- Modern resolver
- Better feature unification
- Improved performance
- Default for new projects

### Virtual Workspaces

Create a workspace without a root package:

```toml
# Workspace root with no [package] section
[workspace]
members = ["package1", "package2"]

# This is a "virtual workspace" - only for organization
# No library/binary at workspace root
```

### Nested Workspaces

**Not Supported**: Workspaces cannot be nested.

```
workspace/
├── CCGO.toml (workspace)
└── subproject/
    └── CCGO.toml (workspace)  ❌ Error!
```

**Alternative**: Use workspace members:
```
workspace/
├── CCGO.toml (workspace)
├── project1/
│   └── CCGO.toml (member)
└── project2/
    └── CCGO.toml (member)
```

## Troubleshooting

### Duplicate Member Names

**Error**:
```
Error: Duplicate workspace member name 'utils' at libs/utils
```

**Solution**: Ensure all workspace members have unique names in their [package] section.

### Missing Workspace Dependency

**Error**:
```
Error: Member 'core' declares dependency 'fmt' with workspace=true,
but it's not defined in workspace dependencies
```

**Solution**: Add the dependency to workspace root:
```toml
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

### Circular Dependencies

**Error**:
```
Error: Circular dependency detected involving 'core'
```

**Solution**: Review inter-member dependencies and break the cycle:
```
core → utils → core  ❌

Solution:
core → utils ✅
(Remove utils dependency on core)
```

### Member Not Found

**Error**:
```
Error: Workspace member not found: libs/utils
```

**Solution**:
1. Verify the path exists
2. Ensure member has `CCGO.toml`
3. Check for typos in workspace.members

### Glob Pattern Issues

**Problem**: Expected members not discovered

**Debug**:
```bash
# List discovered members
ccgo workspace list

# Check paths manually
ls -la libs/*/CCGO.toml
```

**Common Issues**:
- Forgot `*` or `**` in pattern
- Path is in exclude list
- Member missing CCGO.toml

## Best Practices

### DO

✅ **Use workspace dependencies** for shared libraries:
```toml
# Workspace root
[[workspace.dependencies]]
name = "fmt"
version = "10.0.0"
```

✅ **Use glob patterns** for consistent directory structures:
```toml
members = ["libs/*", "examples/*"]
```

✅ **Define default_members** for common builds:
```toml
default_members = ["core", "app"]  # Skip examples/tests
```

✅ **Use semantic versioning** for workspace members:
```toml
[package]
name = "core"
version = "1.2.3"  # Proper semver
```

✅ **Build in dependency order** for inter-dependent members:
```bash
ccgo build --workspace --ordered
```

### DON'T

❌ **Don't nest workspaces**:
```toml
# Not supported - use single workspace
```

❌ **Don't duplicate workspace dependencies in members**:
```toml
# Bad
[[dependencies]]
name = "fmt"
version = "10.0.0"  # Should use workspace = true

# Good
[[dependencies]]
name = "fmt"
workspace = true
```

❌ **Don't use inconsistent versions**:
```toml
# Bad: Members use different versions
# member1: fmt 9.0.0
# member2: fmt 10.0.0

# Good: All use workspace dependency
```

❌ **Don't create circular dependencies**:
```toml
# Bad: core → utils → core
# Good: core → utils (one direction)
```

## Implementation Details

### Member Discovery Algorithm

1. Read workspace.members patterns
2. Expand glob patterns to concrete paths
3. Filter out workspace.exclude patterns
4. For each path:
   - Check if directory exists
   - Check for CCGO.toml presence
   - Load and validate configuration
5. Check for duplicate names
6. Return discovered members

### Dependency Resolution

1. For each member dependency:
   - If `workspace = true`:
     - Find dependency in workspace.dependencies
     - Inherit version, features, etc.
     - Merge member-specific features
     - Apply member default_features override
   - Else:
     - Use member's own dependency spec
2. Return resolved dependencies

### Build Order Calculation

Uses topological sort (depth-first search):

1. Create dependency graph of workspace members
2. Detect cycles (circular dependencies)
3. Perform DFS to order members
4. Return build order (dependencies before dependents)

**Time Complexity**: O(V + E) where V = members, E = dependencies

## See Also

- [Dependency Management](dependency-management.md) - Overall dependency system
- [Version Conflict Resolution](version-conflict-resolution.md) - Handle version conflicts
- [Monorepo Guide](monorepo-guide.md) - Best practices for monorepos

## Changelog

### v3.0.12 (2026-01-21)

- ✅ Workspace support for monorepo management
- ✅ Glob pattern discovery for workspace members
- ✅ Dependency inheritance via `workspace = true`
- ✅ Inter-member dependency resolution
- ✅ Topological build ordering
- ✅ Circular dependency detection
- ✅ Feature extension and default_features override

---

*Workspace dependencies enable efficient monorepo management with shared dependencies and coordinated builds.*
