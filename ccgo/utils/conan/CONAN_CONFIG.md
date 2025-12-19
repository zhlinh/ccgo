# Conan Publishing Configuration Guide

This guide explains how to configure CCGO to publish C/C++ libraries to Conan package manager.

## Overview

CCGO now supports publishing to three types of Conan repositories:
1. **Local Cache** - Your local Conan cache (~/.conan2/)
2. **Official Remote** - First configured remote repository
3. **Private Remote** - Custom Conan-compatible repository (Artifactory, etc.)

All configuration is done through `CCGO.toml`, with support for environment variables for sensitive data.

## Configuration in CCGO.toml

### Unified Field Names

CCGO uses unified field names across all publish configurations. The following field aliases are supported for backward compatibility:

| Unified Name | Legacy Alias | Description |
|--------------|--------------|-------------|
| `name` | `package_name` | Package name |
| `group_id` | `user`, `organization` | User/organization for @user/channel |
| `registry` | `repository` | Registry type (local/official/private) |
| `description` | - | Package description |

**Priority:** New unified names take precedence over legacy aliases.

### Basic Configuration

```toml
[publish.conan]
registry = "local"           # Options: local, official, private
name = "mylib"               # Package name (default: project.name)
group_id = "myorg"           # User in name/version@user/channel
channel = "stable"           # Channel: stable, testing, dev
version = "1.0.0"
description = "My C/C++ library"
```

### Conan Package Reference Format

Conan 2.x uses the following package reference format:
```
name/version@user/channel
```

For example:
- `mylib/1.0.0` - Basic reference (no user/channel)
- `mylib/1.0.0@myorg/stable` - Full reference with user and channel

### Group ID Behavior

The `group_id` field controls the user/channel in the package reference:

```toml
# 1. Fallback to project.group_id (default behavior)
[publish.conan]
name = "mylib"
# → mylib/1.0.0@<last_segment_of_project.group_id>/stable

# 2. Explicitly set group_id
[publish.conan]
name = "mylib"
group_id = "myorg"
# → mylib/1.0.0@myorg/stable

# 3. Explicitly disable group_id (no user/channel)
[publish.conan]
name = "mylib"
group_id = ""
# → mylib/1.0.0
```

**Note:** Unlike Maven where `group_id` is required, Conan and OHPM allow packages without user/organization. Use `group_id = ""` to explicitly publish without user/channel.

### Publishing to Local Cache

The simplest option - no authentication required:

```toml
[publish.conan]
registry = "local"
name = "mylib"
version = "1.0.0"
```

### Publishing to Remote Repository

For uploading to Artifactory, Nexus, or other Conan servers:

```toml
[publish.conan]
registry = "private"
url = "https://conan.company.com/artifactory/api/conan/conan-local"
remote_name = "company-conan"  # Name for the remote
name = "mylib"
group_id = "myorg"             # Results in mylib/1.0.0@myorg/stable
channel = "stable"
version = "1.0.0"

[publish.conan.auth]
[publish.conan.auth.credentials]
username = "${CONAN_USERNAME}"
password = "${CONAN_PASSWORD}"
```

### Dependencies

Specify Conan dependencies that will be included in the generated conanfile.py:

```toml
[publish.conan]
name = "mylib"
version = "1.0.0"

# Dependencies as array
dependencies = [
    { name = "zlib", version = "1.2.13" },
    { name = "openssl", version = "3.0.8", user = "conan", channel = "stable" },
]
```

Or use string format:
```toml
dependencies = [
    "zlib/1.2.13",
    "openssl/3.0.8@conan/stable",
]
```

## Environment Variables

### Standard Variables

These environment variables are automatically recognized:

```bash
# Conan authentication
export CONAN_LOGIN_USERNAME="your-username"
export CONAN_LOGIN_PASSWORD="your-password"

# Alternative names
export CONAN_USERNAME="your-username"
export CONAN_PASSWORD="your-password"
```

### Remote-Specific Variables

For authenticating to specific remotes:

```bash
# Format: CONAN_LOGIN_USERNAME_{REMOTE_NAME} (uppercase)
export CONAN_LOGIN_USERNAME_ARTIFACTORY="your-username"
export CONAN_LOGIN_PASSWORD_ARTIFACTORY="your-password"
```

### Custom Variables

You can reference any environment variable using `${VAR_NAME}` syntax:

```toml
[publish.conan.auth.credentials]
username = "${MY_CUSTOM_USER_VAR}"
password = "${MY_CUSTOM_PASS_VAR}"
```

## Command Line Usage

### Interactive Mode

Simply run the publish command and follow prompts:

```bash
ccgo publish conan
# Prompts for registry type selection
```

### Direct Publishing

Specify registry type via command line:

```bash
# Publish to Local Cache
ccgo publish conan --conan local

# Publish to first configured remote
ccgo publish conan --conan official

# Publish to private remote
ccgo publish conan --conan private --conan-name myremote --conan-url https://conan.company.com

# Skip confirmation prompts
ccgo publish conan -y
```

## Remote Setup

### Adding a Remote

Before uploading to a remote, you need to configure it:

```bash
# Add a remote
conan remote add myremote https://conan.company.com/artifactory/api/conan/conan-local

# Login to remote
conan remote login myremote username -p password

# List remotes
conan remote list
```

### Common Remote URLs

- **Artifactory**: `https://company.jfrog.io/artifactory/api/conan/conan-local`
- **Nexus**: `https://nexus.company.com/repository/conan-hosted/`
- **Local Verdaccio**: `http://localhost:9300`

## CI/CD Integration

### GitHub Actions

```yaml
- name: Publish to Conan
  env:
    CONAN_LOGIN_USERNAME_MYREMOTE: ${{ secrets.CONAN_USERNAME }}
    CONAN_LOGIN_PASSWORD_MYREMOTE: ${{ secrets.CONAN_PASSWORD }}
  run: |
    conan remote add myremote https://conan.company.com
    ccgo publish conan --conan private --conan-name myremote -y
```

### GitLab CI

```yaml
publish-conan:
  script:
    - conan remote add myremote https://conan.company.com
    - conan remote login myremote $CONAN_USERNAME -p $CONAN_PASSWORD
    - ccgo publish conan --conan official -y
  variables:
    CONAN_USERNAME: $CI_CONAN_USERNAME
    CONAN_PASSWORD: $CI_CONAN_PASSWORD
```

## Full Configuration Example

Here's a complete example for a production C/C++ library:

```toml
[project]
name = "awesome-lib"
version = "2.3.1"
description = "An awesome C/C++ library"
group_id = "com.awesomecompany"

[publish.conan]
registry = "private"
url = "https://conan.awesomecompany.com/artifactory/api/conan/conan-local"
remote_name = "awesome-conan"
name = "awesome-lib"
group_id = "awesomecompany"    # Results in awesome-lib/2.3.1@awesomecompany/stable
channel = "stable"
version = "${project.version}"
description = "${project.description}"
license = "MIT"

# Build options
settings = ["os", "compiler", "build_type", "arch"]
options = { "with_openssl": [true, false] }
default_options = { "with_openssl": true }

# Dependencies
dependencies = [
    { name = "zlib", version = "1.2.13" },
    { name = "openssl", version = "3.0.8" },
]

# Authentication
[publish.conan.auth]
[publish.conan.auth.credentials]
username = "${CONAN_USERNAME}"
password = "${CONAN_PASSWORD}"
```

Then publish with:
```bash
ccgo publish conan --conan private -y
```

## Troubleshooting

### Remote Not Found

```
ERROR: Remote 'myremote' not found
```

Solution: Add the remote first:
```bash
conan remote add myremote https://your-conan-server.com
```

### Authentication Failed

```
ERROR: Permission denied for user
```

Solutions:
1. Check environment variables are set:
   ```bash
   echo $CONAN_LOGIN_USERNAME_MYREMOTE
   ```
2. Login to remote manually:
   ```bash
   conan remote login myremote username -p password
   ```

### Package Already Exists

```
ERROR: Package already exists in remote
```

Solutions:
1. Increment the version in CCGO.toml
2. Use `--force` flag if overwriting is allowed
3. Delete the existing package from remote

### Build Failed

```
ERROR: Conan package creation failed
```

Check:
1. Conan is installed: `conan --version`
2. Default profile exists: `conan profile show`
3. CMake is installed and in PATH
4. Project builds successfully with `ccgo build conan`

## Best Practices

1. **Use Semantic Versioning** - Follow major.minor.patch format
2. **Use Channels** - `stable` for releases, `testing` for pre-release, `dev` for development
3. **Test Locally First** - Use `--conan local` before publishing to remote
4. **Automate in CI/CD** - Set up automated publishing on tags/releases
5. **Keep Credentials Secure** - Never commit credentials to version control
6. **Document Dependencies** - List all required dependencies in conanfile
7. **Use User/Channel** - Namespace your packages to avoid conflicts

## Package Naming Conventions

- Use lowercase letters, numbers, underscores, and hyphens
- Start with a letter
- Examples: `mylib`, `awesome-utils`, `my_project`

### User/Channel Conventions

- **User**: Your organization or username (e.g., `mycompany`, `username`)
- **Channel**: Release stage (e.g., `stable`, `testing`, `dev`)
- Full example: `mylib/1.0.0@mycompany/stable`
