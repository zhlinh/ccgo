# Transitive Dependency Resolution

## Overview

CCGO now supports automatic transitive dependency resolution. When you run `ccgo install`, it automatically discovers and installs dependencies of your dependencies, determines the correct build order, and detects circular dependencies.

## Features

### 1. Transitive Dependency Discovery

When installing a dependency that has its own `CCGO.toml` file with dependencies, CCGO will:
- Automatically read the dependency's CCGO.toml
- Discover its dependencies (transitive dependencies)
- Recursively resolve all dependencies in the entire dependency tree
- Handle both git and path dependencies

### 2. Dependency Graph Visualization

CCGO displays a visual tree of your dependencies showing:
- Direct dependencies (declared in your CCGO.toml)
- Transitive dependencies (dependencies of dependencies)
- Shared dependencies (used by multiple packages)
- Source information (git URL, path, version)

Example output:
```
Dependency tree:
mylib v1.0.0
├── fmt v9.1.0 (git: https://github.com/fmtlib/fmt)
│   └── gtest v1.12.0 (git: https://github.com/google/googletest)
└── json v3.11.2 (git: https://github.com/nlohmann/json)

3 unique dependencies found, 4 total (1 shared)
```

### 3. Topological Sorting for Build Order

CCGO uses topological sorting to determine the correct installation order:
- Dependencies with no dependencies are installed first
- Dependencies are installed before their dependents
- Ensures builds succeed by respecting dependency chains

Example:
```
📦 Installing in dependency order:
  1. gtest
  2. fmt
  3. mylib
```

### 4. Circular Dependency Detection

CCGO detects circular dependencies and reports them with the full cycle path:

```
Error: Circular dependency detected: libA -> libB -> libC -> libA
```

### 5. Version Conflict Warnings

When multiple packages depend on different versions of the same dependency, CCGO:
- Warns about version conflicts
- Uses the first version encountered (for now)
- Displays clear warnings to help resolve conflicts

Example:
```
⚠️  Version conflict for 'fmt': have 9.1.0, need 10.0.0
```

### 6. Max Depth Protection

To prevent infinite recursion, CCGO limits dependency depth to 50 levels and reports an error if exceeded.

## Implementation

### Architecture

The dependency resolution system consists of three main components:

#### 1. Dependency Graph (`src/dependency/graph.rs`)

- **DependencyNode**: Represents a single dependency with metadata
- **DependencyGraph**: Manages the entire dependency graph
- **Cycle Detection**: DFS-based algorithm to find circular dependencies
- **Topological Sort**: Kahn's algorithm for build order
- **Tree Formatting**: Pretty-print dependency tree
- **Statistics**: Calculate unique, shared, and total dependencies

#### 2. Dependency Resolver (`src/dependency/resolver.rs`)

- **DependencyResolver**: Main resolver that orchestrates dependency resolution
- **Recursive Resolution**: Traverses dependency tree recursively
- **Path Resolution**: Handles relative paths in transitive dependencies
- **Caching**: Visited set prevents duplicate processing
- **Error Handling**: Graceful degradation on resolution failures

#### 3. Install Command Integration (`src/commands/install.rs`)

- Calls resolver to build dependency graph
- Displays dependency tree and statistics
- Uses topological sort for installation order
- Falls back to direct dependencies on errors
- Installs dependencies in correct order

### Data Structures

```rust
pub struct DependencyNode {
    pub name: String,
    pub version: String,
    pub source: String,              // git+url or path+path
    pub dependencies: Vec<String>,   // Direct dependencies
    pub depth: usize,                // Depth in dependency tree
    pub config: DependencyConfig,    // Original config
}

pub struct DependencyGraph {
    nodes: HashMap<String, DependencyNode>,
    edges: Vec<(String, String)>,    // (from, to) edges
    roots: HashSet<String>,          // Root dependencies
}
```

### Key Algorithms

#### Cycle Detection (DFS)

```rust
pub fn detect_cycles(&self) -> Option<Vec<String>>
```

Uses depth-first search with a recursion stack to detect cycles. Returns the cycle path if found.

#### Topological Sort (Kahn's Algorithm)

```rust
pub fn topological_sort(&self) -> Result<Vec<String>>
```

Implements Kahn's algorithm with in-degree calculation to determine build order.

## Usage

### Basic Usage

Simply run `ccgo install` and CCGO will automatically:
1. Resolve transitive dependencies
2. Display the dependency tree
3. Show installation order
4. Install all dependencies in correct order

```bash
ccgo install
```

### What Gets Resolved

Given this project structure:

**Project CCGO.toml:**
```toml
[package]
name = "myapp"
version = "1.0.0"

[[dependencies]]
name = "libA"
version = "1.0.0"
path = "../libA"
```

**libA CCGO.toml:**
```toml
[package]
name = "libA"
version = "1.0.0"

[[dependencies]]
name = "libB"
version = "2.0.0"
path = "../libB"
```

**libB CCGO.toml:**
```toml
[package]
name = "libB"
version = "2.0.0"
# No dependencies
```

Running `ccgo install` will:
1. Discover libA (direct dependency)
2. Read libA's CCGO.toml
3. Discover libB (transitive dependency)
4. Read libB's CCGO.toml
5. Determine order: libB → libA → myapp
6. Install libB first, then libA

## Testing

The implementation includes comprehensive tests:

### Unit Tests

Located in `src/dependency/resolver.rs` and `src/dependency/graph.rs`:

- **test_simple_resolution**: Basic single dependency
- **test_transitive_dependencies**: Chain of dependencies (A → B → C)
- **test_circular_dependency_detection**: Detect cycles (A → B → C → A)
- **test_shared_dependency**: Diamond pattern (A → C, B → C)
- **test_missing_ccgo_toml**: Handle dependencies without CCGO.toml
- **test_version_conflict_warning**: Detect version conflicts
- **test_max_depth_exceeded**: Prevent infinite recursion
- **test_simple_graph**: Basic graph operations
- **test_cycle_detection**: Cycle detection algorithm
- **test_shared_dependency** (graph): Shared dependency statistics

Run tests with:
```bash
cargo test dependency
```

## Limitations & Future Work

### Current Limitations

1. **Version Resolution**: Currently uses "first version wins" strategy. Need proper semantic versioning resolution.
2. **Workspace Dependencies**: Not yet fully implemented for workspace inheritance.
3. **Lockfile**: No lockfile generation yet for reproducible builds.
4. **Dependency Patching**: No support for overriding transitive dependencies.

### Planned Enhancements

1. **Smart Version Resolution**:
   - Semantic versioning awareness
   - Minimum version selection
   - Version constraint satisfaction

2. **Lockfile Support**:
   - Generate CCGO.lock with exact versions
   - Verify lockfile on install
   - Update command to refresh lockfile

3. **Dependency Vendoring**:
   - Download and cache dependencies
   - Offline build support
   - Reproducible builds

4. **Dependency Overrides**:
   - Patch dependencies via CCGO.toml
   - Replace URLs for mirroring
   - Version pinning

5. **Build-time Dependencies**:
   - Separate build-only dependencies
   - Development dependencies
   - Optional dependencies

## Related Files

- `src/dependency/mod.rs` - Module definition
- `src/dependency/graph.rs` - Dependency graph implementation (~450 lines)
- `src/dependency/resolver.rs` - Dependency resolver (~620 lines)
- `src/commands/install.rs` - Install command integration
- `src/config/ccgo_toml.rs` - CCGO.toml configuration

## References

- **Topological Sorting**: [Kahn's Algorithm](https://en.wikipedia.org/wiki/Topological_sorting)
- **Cycle Detection**: [Depth-First Search](https://en.wikipedia.org/wiki/Cycle_detection)
- **Semantic Versioning**: [semver.org](https://semver.org/)

## Changelog

### v3.0.11 (2025-01-21)

- ✅ Implemented transitive dependency resolution
- ✅ Added dependency graph with cycle detection
- ✅ Added topological sorting for correct build order
- ✅ Added dependency tree visualization
- ✅ Added version conflict detection (warnings only)
- ✅ Integrated into install command
- ✅ Added comprehensive test suite (7 resolver tests + 3 graph tests)

---

*This feature is part of the Rust CLI rewrite (spec 001-rust-cli-rewrite) to achieve zero Python dependency.*
