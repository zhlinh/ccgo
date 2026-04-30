# Package Registry

> Added in v3.2.0

CCGO supports package registries - lightweight Git-based package indices that enable simplified dependency management without a central server.

## Overview

Following Swift Package Manager's approach, CCGO uses Git repositories as package indices. This design:

- Requires no server maintenance
- Leverages existing Git infrastructure
- Naturally supports private packages
- Works offline once cached

## Two-tier model

CCGO splits "what packages exist" from "where the bytes live":

**Tier 1 — Discovery (the index repo).** A Git repository whose JSON files
list every published package and version. Browseable on the web, auditable
via `git log`, queryable with `ccgo registry search`. Publishers run
`ccgo publish index` to append a new `VersionEntry` whenever they ship.
Because the index is just text in Git, code review, branch protection, and
signed commits all apply for free.

**Tier 2 — Resolution (the artifact archive).** Each `VersionEntry` may
record an `archive_url` pointing at a packaged build (zip or tar.gz) on a
CDN, artifactory, or release page, plus a SHA-256 `checksum`. When a
consumer's `ccgo fetch` resolves a `version`-only dependency through the
index, ccgo downloads that archive directly, verifies the checksum, and
extracts it into `.ccgo/deps/<name>/`. No git history transfer, no
post-clone build step, no source compilation on the consumer side.

The consumer's `CCGO.toml` carries no URL — only `version = "1.0.0"` plus
an optional `registry = "name"` selector. The index and the
`[registries]` table together resolve the rest.

## Configuration

Declare one or more registries in the project's `CCGO.toml`. The map key
is the registry name; the value is the index repository's Git URL.

```toml
[registries]
mna = "git@git.example.com:org/ccgo-index.git"
public = "https://github.com/example-org/ccgo-packages.git"

[[dependencies]]
name = "stdcomm"
version = "25.2.9519653"
registry = "mna"           # explicit selector

[[dependencies]]
name = "fmt"
version = "10.2.1"
# no registry = ... — ccgo walks all declared registries in declaration order,
# first match wins
```

The `[[dependencies]].registry` field is optional. When set, it pins the
lookup to a single registry (and errors if the name is not in
`[registries]`). When absent, ccgo iterates registries in TOML declaration
order and takes the first match — same precedence rule as Cargo.

## Publisher CI workflow

`ccgo publish index` is **append-only and per-version** (CocoaPods
`pod repo push`-style). One invocation publishes exactly one tag into
the index — re-publishing the same version is rejected. Pass
`--index-version` and/or `--index-tag` to identify the version:

```bash
ccgo build all --release
ccgo package --release        # produces NAME_CCGO_PACKAGE-VERSION.zip
# upload zip to your CDN/artifactory  (your script)
ccgo publish index \
  --index-repo git@example.com:org/index.git \
  --index-name org-index \
  --index-version 25.2.9519653 \
  --archive-url-template "https://artifacts.example.com/{name}/{name}_CCGO_PACKAGE-{version}.zip" \
  --checksum \
  --index-push
```

When you pass only `--index-tag v1.0.0`, version is auto-derived as
`1.0.0` (leading `v`/`V` stripped). When you pass only
`--index-version 1.0.0`, the tag defaults to `v1.0.0`. For tags that
don't follow the `v<version>` convention (e.g. `release-v1.0.0`, or a
monorepo prefix like `stdcomm-v1.0.0`), pass both flags explicitly:

```bash
ccgo publish index ... --index-version 1.0.0 --index-tag stdcomm-v1.0.0
```

Before publishing, ccgo runs `git rev-parse --verify <tag>` to make
sure the tag actually exists in the project's local repo. The new
`VersionEntry` is then appended to the existing `versions` array of
`<index>/<sharded>/<name>.json` and the array is sorted descending by
version string.

The placeholders `{name}`, `{version}`, and `{tag}` are substituted
into `--archive-url-template` for the new entry. With `--checksum` AND
a template, the SHA-256 hashes the local
`target/release/package/<NAME>_CCGO_PACKAGE-<version>.zip` so
consumer-side fetch verifies the same bytes the CDN serves.

See [Publishing to Index](#publishing-to-index) below for the full
flag reference.

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
      "tag": "v10.2.1",
      "checksum": "sha256:...",
      "archive_url": "https://artifacts.example.com/fmt/fmt_CCGO_PACKAGE-10.2.1.zip",
      "archive_format": "zip",
      "yanked": false
    },
    {
      "version": "10.1.1",
      "tag": "v10.1.1",
      "checksum": "sha256:...",
      "yanked": false
    }
  ]
}
```

`archive_url` and `archive_format` are optional; entries that omit them
remain valid and are simply skipped by the registry-resolution path
(consumers must then declare an explicit `git`/`zip` source for those
versions). `archive_format` defaults to `"zip"` when an `archive_url` is
present without a format hint; `"tar.gz"` is also supported.

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

When `ccgo fetch` resolves a `version`-only dependency through `[registries]`:

1. Walk declared registries in TOML declaration order (or the single
   registry named by `[[dependencies]].registry`).
2. For each registry, ensure the index is cloned into
   `~/.ccgo/registries/<name>/` (or pulled if already present).
3. Look up the package's sharded JSON entry.
4. Filter the entry's `versions[]` to non-yanked exact-version matches.
5. **First match wins.** Take the first `VersionEntry` with a matching
   `version`; subsequent registries are not consulted.
6. If the entry has an `archive_url`, download it, verify
   SHA-256 against `checksum` (when present), and extract to
   `.ccgo/deps/<name>/`.
7. The lockfile records `source = "registry+<index-url>"` plus the
   `checksum`, so `ccgo fetch --locked` reproduces the same bytes
   without re-resolving.

Note: this iteration does **not** parse semver requirements like `^1.0`
or `~2.1`. The `version = "x.y.z"` value is matched as an exact string
against `VersionEntry.version`. Range support is a planned follow-up.

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
| `--archive-url-template <T>` | URL template baked into each `VersionEntry.archive_url`. Placeholders: `{name}`, `{version}`, `{tag}`. |
| `--archive-format <fmt>` | Archive format hint stored in `VersionEntry.archive_format` (default `zip`; also `tar.gz`). Only applies when `--archive-url-template` is set. |

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
- [Migrating to a Registry-Based Setup](../guides/migrating-to-registry.md)
- [Dependency Resolution](../dependency-resolution.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
