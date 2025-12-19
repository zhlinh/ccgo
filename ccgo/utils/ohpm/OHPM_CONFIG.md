# OHPM Publishing Configuration Guide

This guide explains how to configure CCGO to publish OHOS/OpenHarmony artifacts to OHPM (OpenHarmony Package Manager) registries.

## Overview

CCGO now supports publishing to three types of OHPM registries:
1. **Official Registry** - The official OHPM registry (ohpm.openharmony.cn)
2. **Private Registry** - Your organization's private NPM-compatible registry
3. **Local Registry** - Local registry for testing (e.g., Verdaccio)

All configuration is done through `CCGO.toml`, with support for environment variables for sensitive data.

## Configuration in CCGO.toml

### Unified Field Names

CCGO uses unified field names across all publish configurations. The following field aliases are supported for backward compatibility:

| Unified Name | Legacy Alias | Description |
|--------------|--------------|-------------|
| `name` | `package_name` | Package name |
| `group_id` | `organization` | Scope/organization (for @scope/name format) |

**Priority:** New unified names take precedence over legacy aliases.

### New Configuration Format (Recommended)

```toml
[publish.ohos.ohpm]
registry = "official"  # Options: "official", "private", "local"
name = "my-ohos-lib"   # Package name (default: project.name)
group_id = "myorg"     # Scope (preferred, for @myorg/my-ohos-lib format)
version = "1.0.0"
description = "My OpenHarmony library"

# Dependencies (optional)
dependencies = [
    { name = "@ohos/library1", version = "^1.0.0" },
    { name = "@ohos/test-lib", version = "^1.0.0", dev = true },
]
```

### Legacy Configuration Format (Still Supported)

For backward compatibility, the old format is still supported:

```toml
[publish.ohos]
registry = "official"  # Options: "official", "private", "local"
package_name = "my-ohos-lib"
version = "1.0.0"
description = "My OpenHarmony library"

# Optional: For scoped packages
organization = "myorg"  # Results in @myorg/my-ohos-lib
```

### Publishing to Official OHPM Registry

The simplest option - public packages don't require authentication:

```toml
[publish.ohos.ohpm]
registry = "official"
package_name = "my-ohos-lib"
version = "1.0.0"
description = "An awesome OHOS library"
access = "public"  # or "restricted" for scoped packages

# Optional authentication for publishing updates
[publish.ohos.ohpm.auth]
[publish.ohos.ohpm.auth.credentials]
token = "${OHPM_TOKEN}"  # If you have publishing rights
```

### Publishing to Private Registry

For organization's private NPM-compatible registries:

```toml
[publish.ohos.ohpm]
registry = "private"
url = "https://npm.company.com"  # Your private registry URL
package_name = "internal-ohos-lib"
version = "2.1.0"
organization = "company"  # Optional, for @company/internal-ohos-lib

[publish.ohos.ohpm.auth]
[publish.ohos.ohpm.auth.credentials]
# Token authentication (recommended)
token = "${COMPANY_NPM_TOKEN}"

# Or username/password
# username = "${NPM_USERNAME}"
# password = "${NPM_PASSWORD}"
```

### Publishing to Local Registry

For testing with local Verdaccio or similar:

```toml
[publish.ohos.ohpm]
registry = "local"
url = "http://localhost:4873"  # Default for Verdaccio
package_name = "test-lib"
version = "0.1.0"

# Usually no authentication needed for local registry
```

## oh-package.json5 Configuration

Configure the generated oh-package.json5 file:

```toml
[publish.ohos.ohpm.oh_package]
author = "Your Name"
license = "MIT"
main = "index.ets"  # Entry point
type = "shared"  # or "static"
keywords = ["ohos", "harmony", "library"]

# Repository information
[publish.ohos.ohpm.oh_package.repository]
type = "git"
url = "https://github.com/username/project.git"

# Dependencies can be specified in two ways:

# 1. Using the dependencies array (recommended)
[publish.ohos.ohpm]
dependencies = [
    { name = "@ohos/library1", version = "^1.0.0" },
    { name = "@ohos/library2", version = "^2.0.0" },
    { name = "@ohos/test-lib", version = "^1.0.0", dev = true },
]

# 2. Or directly in oh_package (for more complex setups)
[publish.ohos.ohpm.oh_package.dependencies]
"@ohos/library1" = "^1.0.0"
"@ohos/library2" = "^2.0.0"

[publish.ohos.ohpm.oh_package.devDependencies]
"@ohos/test-lib" = "^1.0.0"
```

## Environment Variables

### Standard Variables

These environment variables are automatically recognized:

```bash
# OHPM authentication
export OHPM_TOKEN="your-access-token"
export OHPM_ACCESS_TOKEN="alternative-token-name"

# Username/password authentication (fallback)
export OHPM_USERNAME="your-username"
export OHPM_PASSWORD="your-password"

# Private registry specific
export OHPM_REGISTRY_TOKEN="private-registry-token"
```

### Custom Variables

You can reference any environment variable using `${VAR_NAME}` syntax:

```toml
[publish.ohos.auth.credentials]
token = "${MY_CUSTOM_TOKEN_VAR}"
username = "${MY_CUSTOM_USER_VAR}"
```

## Command Line Usage

### Interactive Mode

Simply run the publish command and follow prompts:

```bash
ccgo publish ohos
# Prompts for registry type selection
```

### Direct Publishing

Specify registry type via command line:

```bash
# Publish to Official OHPM Registry
ccgo publish ohos --ohpm-registry official

# Publish to Private Registry
ccgo publish ohos --ohpm-registry private --ohpm-url https://npm.company.com

# Publish to Local Registry (for testing)
ccgo publish ohos --ohpm-registry local

# Verify after publishing
ccgo publish ohos --verify
```

## Authentication Setup

### Official Registry

For the official OHPM registry, you need an account and publishing rights:

1. Register at https://ohpm.openharmony.cn/
2. Get your access token from your account settings
3. Set the environment variable:
   ```bash
   export OHPM_TOKEN="your-access-token"
   ```

### Private Registry

For private registries, authentication depends on your registry configuration:

#### Token Authentication (Recommended)
```bash
# Get token from your registry admin
export COMPANY_NPM_TOKEN="your-private-registry-token"
```

#### Username/Password
```bash
export OHPM_USERNAME="your-username"
export OHPM_PASSWORD="your-password"
```

### Local Registry

For local testing with Verdaccio:

```bash
# Install and start Verdaccio
npm install -g verdaccio
verdaccio

# Usually no authentication needed for local development
```

## Full Configuration Example

Here's a complete example for a production OHOS library:

```toml
[project]
name = "awesome-ohos-lib"
version = "3.2.1"
description = "An awesome OpenHarmony library"

[publish.ohos.ohpm]
registry = "official"
package_name = "${project.name}"  # Reference project name
version = "${project.version}"    # Reference project version
description = "${project.description}"
organization = "awesomeorg"  # Results in @awesomeorg/awesome-ohos-lib
access = "public"
tag = "latest"

# Dependencies
dependencies = [
    { name = "@ohos/base", version = "^1.0.0" },
    { name = "@ohos/test-utils", version = "^1.0.0", dev = true },
]

# Authentication
[publish.ohos.ohpm.auth]
[publish.ohos.ohpm.auth.credentials]
token = "${OHPM_TOKEN}"

# oh-package.json5 configuration
[publish.ohos.ohpm.oh_package]
author = "Awesome Developer <dev@awesome.com>"
license = "Apache-2.0"
main = "index.ets"
type = "shared"
keywords = ["ohos", "harmony", "awesome", "library"]

[publish.ohos.ohpm.oh_package.repository]
type = "git"
url = "https://github.com/awesomeorg/awesome-ohos-lib.git"
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Publish to OHPM
  env:
    OHPM_TOKEN: ${{ secrets.OHPM_TOKEN }}
  run: |
    ccgo publish ohos --ohpm-registry official
```

### GitLab CI

```yaml
publish-ohos:
  script:
    - ccgo publish ohos --ohpm-registry official
  variables:
    OHPM_TOKEN: $CI_OHPM_TOKEN
```

### Jenkins

```groovy
withCredentials([string(credentialsId: 'ohpm-token', variable: 'OHPM_TOKEN')]) {
    sh 'ccgo publish ohos --ohpm-registry official'
}
```

## Migration from Direct ohpm Commands

If you're migrating from direct `ohpm publish` commands:

### Old Way
```bash
# Manual steps
cd ohos
hvigorw assembleHar
ohpm publish path/to/file.har
```

### New Way (with CCGO)
```toml
# Configure once in CCGO.toml
[publish.ohos.ohpm]
registry = "official"
package_name = "my-lib"
version = "1.0.0"
dependencies = [
    { name = "@ohos/base", version = "^1.0.0" },
]
```

Then simply:
```bash
ccgo publish ohos
```

Benefits of the new approach:
- ✅ Automatic HAR building
- ✅ Centralized configuration in CCGO.toml
- ✅ Environment variable support
- ✅ Multiple registry support
- ✅ Automatic oh-package.json5 generation
- ✅ Verification support

## Troubleshooting

### Build Failed

```
ERROR: Build HAR failed
```

Check:
1. OHOS development environment is set up
2. `hvigorw` command is available in PATH
3. Project builds successfully with `ccgo build ohos`

### Authentication Failed

```
ERROR: Authentication failed
```

Solutions:
1. Check environment variables are set:
   ```bash
   echo $OHPM_TOKEN
   ```
2. Verify token is valid and has publishing rights
3. For private registries, check with your registry admin

### Package Name Already Exists

```
ERROR: Package name already exists
```

Solutions:
1. Use a different package name
2. If you own the package, increment the version
3. For scoped packages, use your organization scope

### Registry Not Accessible

```
ERROR: Cannot connect to registry
```

Check:
1. Registry URL is correct
2. Network connectivity
3. For private registries, VPN or firewall settings

### Verification Failed

After publishing with `--verify`:
1. Wait a few seconds for registry to update
2. Check package is visible in registry web interface
3. Try `ohpm view <package-name>` manually

## Best Practices

1. **Use Semantic Versioning** - Follow major.minor.patch format
2. **Scope Organization Packages** - Use `@org/package` naming
3. **Test Locally First** - Use local registry before publishing to official
4. **Automate in CI/CD** - Set up automated publishing on tags/releases
5. **Keep Tokens Secure** - Never commit tokens to version control
6. **Document Dependencies** - List all dependencies in oh-package.json5
7. **Tag Releases** - Use appropriate tags (latest, beta, next)

## Package Naming Conventions

### Official Registry
- Use lowercase letters, numbers, and hyphens
- Start with a letter
- Avoid trademarked names
- Examples: `ohos-utils`, `harmony-ui`, `my-awesome-lib`

### Scoped Packages
- Format: `@organization/package-name`
- Organization must be registered
- Examples: `@mycompany/ui-lib`, `@team/shared-utils`

## Registry URLs

### Official
- Registry: https://ohpm.openharmony.cn/registry
- Web: https://ohpm.openharmony.cn/

### Common Private Registries
- Nexus: https://nexus.company.com/repository/npm-ohos/
- Artifactory: https://artifactory.company.com/artifactory/api/npm/ohos-local/
- Verdaccio: http://localhost:4873/

## Additional Resources

- [OHPM Official Documentation](https://ohpm.openharmony.cn/docs)
- [OpenHarmony Developer Guide](https://www.openharmony.cn/docs)
- [oh-package.json5 Specification](https://ohpm.openharmony.cn/docs/package-spec)