# Git Shorthand and Version Discovery

> Added in v3.1.1

CCGO supports simplified Git dependency syntax and automatic version discovery, inspired by Swift Package Manager's approach.

## Git URL Shorthand

Instead of writing full Git URLs, use shorthand notation:

```bash
# GitHub shorthand
ccgo add github:fmtlib/fmt
ccgo add gh:nlohmann/json           # 'gh' is an alias

# GitLab shorthand
ccgo add gitlab:user/repo
ccgo add gl:user/repo               # 'gl' is an alias

# Bitbucket shorthand
ccgo add bitbucket:user/repo
ccgo add bb:user/repo               # 'bb' is an alias

# Gitee shorthand
ccgo add gitee:user/repo

# Bare owner/repo (assumes GitHub)
ccgo add fmtlib/fmt
```

### With Version Tag

Specify a version directly in the shorthand:

```bash
ccgo add github:fmtlib/fmt@v10.1.1
ccgo add gh:nlohmann/json@v3.11.0
```

## Automatic Version Discovery

Use `--latest` to automatically discover and use the latest Git tag:

```bash
# Find and use the latest stable version
ccgo add github:fmtlib/fmt --latest

# Include pre-release versions
ccgo add github:fmtlib/fmt --latest --prerelease
```

### How It Works

1. CCGO runs `git ls-remote --tags` to fetch all tags
2. Tags are parsed as semantic versions
3. Versions are sorted (highest first)
4. The latest stable (non-prerelease) version is selected
5. With `--prerelease`, prereleases are included

## Examples

### Adding Dependencies

```bash
# All these are equivalent:
ccgo add github:fmtlib/fmt@v10.1.1
ccgo add gh:fmtlib/fmt@v10.1.1
ccgo add fmtlib/fmt@v10.1.1
ccgo add fmt --git https://github.com/fmtlib/fmt.git --tag v10.1.1

# Auto-discover latest version
ccgo add github:gabime/spdlog --latest
```

### Generated CCGO.toml

```toml
[[dependencies]]
name = "fmt"
version = "0.0.0"
git = "https://github.com/fmtlib/fmt.git"
branch = "v10.1.1"

[[dependencies]]
name = "spdlog"
version = "0.0.0"
git = "https://github.com/gabime/spdlog.git"
branch = "v1.12.0"
```

## Shorthand in --git Option

You can also use shorthand in the `--git` option:

```bash
# These are equivalent:
ccgo add fmt --git github:fmtlib/fmt
ccgo add fmt --git gh:fmtlib/fmt
ccgo add fmt --git https://github.com/fmtlib/fmt.git
```

## Supported Providers

| Provider | Prefix | Alias | Base URL |
|----------|--------|-------|----------|
| GitHub | `github:` | `gh:` | https://github.com |
| GitLab | `gitlab:` | `gl:` | https://gitlab.com |
| Bitbucket | `bitbucket:` | `bb:` | https://bitbucket.org |
| Gitee | `gitee:` | - | https://gitee.com |

## SSH URLs

CCGO can generate SSH URLs for private repositories:

```rust
// In your code
let spec = expand_git_shorthand("github:company/private-lib")?;
let ssh_url = spec.ssh_url(); // git@github.com:company/private-lib.git
```

## Package Registry Integration

> Added in v3.2.0

CCGO now supports package registries for simplified dependency management.

### Simplified Dependency Syntax

Use table-style syntax in CCGO.toml:

```toml
# Simplified version syntax via registry
[dependencies]
fmt = "^10.1"
spdlog = "1.12.0"

# Or with more options
[dependencies.json]
version = "^3.11"
features = ["ordered_map"]
registry = "company-internal"  # Use a specific registry
```

### Registry Commands

```bash
# Add a custom registry
ccgo registry add company https://github.com/company/package-index.git

# List configured registries
ccgo registry list
ccgo registry list --details

# Update registry indices
ccgo registry update           # Update all registries
ccgo registry update company   # Update specific registry

# Show registry information
ccgo registry info ccgo-packages

# Search packages
ccgo registry search json
ccgo registry search json --registry company --limit 10
```

### Private Registries

Configure private registries in CCGO.toml:

```toml
[registries]
company = "https://github.com/company/package-index.git"
private = "git@github.com:company/private-index.git"
```

### Default Registry

CCGO uses `ccgo-packages` as the default registry, hosted at:
`https://github.com/ArcticLampyrid/ccgo-packages.git`

### How Registry Resolution Works

1. CCGO checks if the dependency specifies a registry
2. Looks up the package in the registry's index
3. Resolves the Git URL and version from the index
4. Falls back to Git URL if not found in any registry

## See Also

- [Registry Reference](registry.md)
- [Dependency Management](dependency-management.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Roadmap](../development/roadmap.md)
