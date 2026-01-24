# Package Registry

> Added in v3.2.0

CCGO supports package registries - lightweight Git-based package indices that enable simplified dependency management without a central server.

## Overview

Following Swift Package Manager's approach, CCGO uses Git repositories as package indices. This design:

- Requires no server maintenance
- Leverages existing Git infrastructure
- Naturally supports private packages
- Works offline once cached

## Registry Index Format

A registry is a Git repository containing JSON files that describe available packages.

Following Rust's crates.io-index naming convention for optimal Git performance:

| Name Length | Path Pattern | Example |
|-------------|--------------|---------|
| 1 char | `1/{name}.json` | `a` → `1/a.json` |
| 2 chars | `2/{name}.json` | `cc` → `2/cc.json` |
| 3 chars | `3/{first}/{name}.json` | `fmt` → `3/f/fmt.json` |
| 4+ chars | `{[0:2]}/{[2:4]}/{name}.json` | `spdlog` → `sp/dl/spdlog.json` |

```
ccgo-packages/
├── index.json              # Registry metadata
├── 1/
│   └── a.json              # 1-char package
├── 2/
│   └── cc.json             # 2-char package
├── 3/
│   └── f/
│       └── fmt.json        # 3-char package
├── sp/
│   └── dl/
│       └── spdlog.json     # 4+ char package
└── nl/
    └── oh/
        └── nlohmann-json.json
```

This directory structure:
- Avoids single directory having too many files (GitHub limits ~1000 files/dir)
- Improves Git performance (large directories slow down clone/pull)
- Evenly distributes packages, reducing merge conflicts

### index.json

```json
{
  "name": "ccgo-packages",
  "description": "Official CCGO package index",
  "version": "1.0.0",
  "package_count": 42,
  "updated_at": "2026-01-24T12:00:00Z",
  "homepage": "https://github.com/ArcticLampyrid/ccgo-packages"
}
```

### Package Entry (e.g., fmt.json)

```json
{
  "name": "fmt",
  "description": "A modern formatting library",
  "repository": "https://github.com/fmtlib/fmt.git",
  "license": "MIT",
  "platforms": ["android", "ios", "macos", "windows", "linux", "ohos"],
  "keywords": ["formatting", "string", "printf"],
  "versions": [
    {
      "version": "10.2.1",
      "git_tag": "10.2.1",
      "checksum": "sha256:...",
      "yanked": false
    },
    {
      "version": "10.1.1",
      "git_tag": "10.1.1",
      "checksum": "sha256:...",
      "yanked": false
    }
  ]
}
```

## Configuration

### Default Registry

CCGO comes with a default registry configured:

```toml
# Implicit default - no configuration needed
# Default: https://github.com/ArcticLampyrid/ccgo-packages.git
```

### Custom Registries

Add custom registries in `CCGO.toml`:

```toml
[registries]
company = "https://github.com/company/package-index.git"
private = "git@github.com:company/private-packages.git"
local = "file:///path/to/local/registry"
```

## Using Registries

### Simplified Dependencies

With registries, use simplified dependency syntax:

```toml
# Instead of:
[[dependencies]]
name = "fmt"
version = "0.0.0"
git = "https://github.com/fmtlib/fmt.git"
branch = "10.2.1"

# Use:
[dependencies]
fmt = "^10.2"
```

### Specify Registry

Use a specific registry for a dependency:

```toml
[dependencies.internal-lib]
version = "^1.0"
registry = "company"

# Or inline:
[dependencies]
public-lib = "^2.0"  # Uses default registry
```

## CLI Commands

### ccgo registry add

Add a new registry:

```bash
ccgo registry add <name> <url>

# Examples:
ccgo registry add company https://github.com/company/packages.git
ccgo registry add private git@github.com:company/private.git
```

### ccgo registry list

List configured registries:

```bash
ccgo registry list
ccgo registry list --details  # Show package counts and update times
```

Output:
```
================================================================================
CCGO Registry - Configured Registries
================================================================================

Registries:

  ✓ ccgo-packages (default)
    URL: https://github.com/ArcticLampyrid/ccgo-packages.git

  ✓ company
    URL: https://github.com/company/packages.git

💡 Update registries with: ccgo registry update
```

### ccgo registry update

Update registry indices:

```bash
ccgo registry update          # Update all registries
ccgo registry update company  # Update specific registry
```

### ccgo registry remove

Remove a registry:

```bash
ccgo registry remove company
```

Note: Cannot remove the default registry.

### ccgo registry info

Show registry details:

```bash
ccgo registry info ccgo-packages
```

Output:
```
================================================================================
CCGO Registry - Registry Information
================================================================================

Registry: ccgo-packages
  URL: https://github.com/ArcticLampyrid/ccgo-packages.git
  Cached: true

Index Metadata:
  Name: CCGO Packages
  Description: Official CCGO package index
  Version: 1.0.0
  Packages: 42
  Last Updated: 2026-01-24T12:00:00Z
  Homepage: https://github.com/ArcticLampyrid/ccgo-packages
```

### ccgo registry search

Search for packages:

```bash
ccgo registry search json
ccgo registry search json --registry company
ccgo registry search json --limit 5
```

## Enhanced Search Command

The `ccgo search` command now searches both registries and collections:

```bash
ccgo search json                    # Search all sources
ccgo search json --registry company # Search specific registry
ccgo search json --registries-only  # Skip collections
ccgo search json --collections-only # Skip registries
ccgo search json --details          # Show detailed info
```

## Cache Location

Registry indices are cached locally:

```
~/.ccgo/registries/
├── ccgo-packages/           # Cloned index repository
│   ├── index.json
│   └── ...
└── company/
    ├── index.json
    └── ...
```

## Creating a Registry

To create your own package registry:

1. Create a Git repository
2. Add `index.json` with registry metadata
3. Add package JSON files in single-letter directories
4. Commit and push

### Package JSON Schema

```json
{
  "name": "string (required)",
  "description": "string (required)",
  "repository": "string (required, Git URL)",
  "license": "string (optional)",
  "platforms": ["array", "of", "platforms"],
  "keywords": ["array", "of", "keywords"],
  "versions": [
    {
      "version": "semver string (required)",
      "git_tag": "string (required)",
      "checksum": "sha256:... (optional)",
      "yanked": "boolean (default: false)"
    }
  ]
}
```

## Version Resolution

When resolving a package from the registry:

1. Find the package in the specified registry (or default)
2. Filter versions matching the version requirement
3. Exclude yanked versions
4. Select the highest matching version
5. Use the `git_tag` to clone the repository

## Publishing to Index

Use `ccgo publish index` to add your package to an index repository:

```bash
# Publish to a custom index
ccgo publish index --index-repo https://github.com/user/my-packages.git

# With custom name and push
ccgo publish index \
  --index-repo https://github.com/company/packages.git \
  --index-name company \
  --index-push

# Custom commit message
ccgo publish index \
  --index-repo git@github.com:user/packages.git \
  --index-message "Add mylib v2.0.0"

# Generate SHA-256 checksums for each version
ccgo publish index \
  --index-repo https://github.com/user/packages.git \
  --checksum \
  --index-push
```

### What It Does

1. Reads package metadata from `CCGO.toml`
2. Discovers versions from Git tags (e.g., `v1.0.0`, `1.0.0`)
3. Generates JSON file in correct directory structure
4. Clones/updates the index repository
5. Commits changes (and optionally pushes)

### Options

| Option | Description |
|--------|-------------|
| `--index-repo <url>` | Index repository URL (required) |
| `--index-name <name>` | Registry name (default: custom-index) |
| `--index-push` | Push changes to remote after commit |
| `--index-message <msg>` | Custom commit message |
| `--checksum` | Generate SHA-256 checksums using git archive |

### Example Output

```
=== Publishing to Package Index ===

📦 Package: mylib
📝 Description: My awesome library
🔗 Repository: https://github.com/user/mylib.git

🔍 Discovering versions from Git tags...
   Found 3 version(s):
   - 2.0.0
   - 1.1.0
   - 1.0.0

📂 Index repository: https://github.com/user/my-packages.git
📥 Cloning index repository...
✅ Written: my/li/mylib.json
📊 Index metadata updated: 5 package(s)
✅ Committed: Update mylib to 2.0.0

✅ Package index updated successfully!

📋 To use this package:
   1. Add registry: ccgo registry add custom-index https://github.com/user/my-packages.git
   2. Add dependency: [dependencies]
      mylib = "^2.0.0"
```

## Best Practices

1. **Use semver**: Tag your packages with semantic versions
2. **Don't delete versions**: Mark them as `yanked` instead
3. **Add checksums**: Enable integrity verification
4. **Keep indices small**: Only include stable, released versions
5. **Update regularly**: Keep your local cache fresh with `ccgo registry update`

## See Also

- [Git Shorthand](git-shorthand.md)
- [Dependency Management](dependency-management.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
