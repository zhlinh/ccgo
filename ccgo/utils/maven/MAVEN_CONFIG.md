# Maven Publishing Configuration Guide

This guide explains how to configure CCGO to publish Android and KMP artifacts to Maven repositories.

## Overview

CCGO now supports publishing to three types of Maven repositories:
1. **Maven Local** - Your local Maven repository (~/.m2/repository/)
2. **Maven Central** - The central Maven repository (via Sonatype OSSRH)
3. **Custom Repository** - Any custom Maven-compatible repository (Nexus, Artifactory, etc.)

All configuration is done through `CCGO.toml`, with support for environment variables for sensitive data.

## Configuration in CCGO.toml

### Unified Field Names

CCGO uses unified field names across all publish configurations. The following field aliases are supported for backward compatibility:

| Unified Name | Legacy Alias | Description |
|--------------|--------------|-------------|
| `name` | `artifact_id` | Package/artifact name |
| `registry` | `repository` | Registry type (local/central/custom) |
| `description` | `pom_description` | Package description |

**Priority:** New unified names take precedence over legacy aliases.

### Automatic Environment Variable Detection

CCGO automatically detects appropriate environment variables based on the repository type:
- **Maven Central**: Checks for `OSSRH_USERNAME`/`OSSRH_PASSWORD` first, then falls back to `MAVEN_USERNAME`/`MAVEN_PASSWORD`
- **Custom Repositories**: Checks for `MAVEN_USERNAME`/`MAVEN_PASSWORD`
- **All Types**: You can explicitly reference any variable using `${VAR_NAME}` syntax in CCGO.toml

### Basic Android Configuration

```toml
[publish.android]
repository = "local"  # Options: "local", "central", "custom"
group_id = "com.example"
artifact_id = "mylib-android"
version = "1.0.0"

# Optional: Override project defaults
pom_name = "My Android Library"
pom_description = "An awesome Android library"
pom_url = "https://github.com/username/project"
```

### Publishing to Maven Local

The simplest option - no authentication required:

```toml
[publish.android]
repository = "local"
group_id = "com.example"
artifact_id = "mylib-android"
version = "1.0.0"
```

### Publishing to Maven Central

Requires authentication and GPG signing.

#### Option 1: Minimal Configuration (Relies on Environment Variables)

```toml
[publish.android]
repository = "central"
group_id = "com.example"  # Must match your OSSRH namespace
artifact_id = "mylib-android"
version = "1.0.0"
sign = true

# CCGO will automatically detect OSSRH_USERNAME, OSSRH_PASSWORD,
# SIGNING_KEY_ID, SIGNING_PASSWORD, and SIGNING_KEY from environment
```

#### Option 2: Explicit Configuration

```toml
[publish.android]
repository = "central"
group_id = "com.example"  # Must match your OSSRH namespace
artifact_id = "mylib-android"
version = "1.0.0"

# Required for Maven Central
sign = true
sources = true  # Include sources JAR
javadoc = true  # Include javadoc JAR

# Repository URLs (optional, uses defaults if not specified)
url = "https://s01.oss.sonatype.org/service/local/staging/deploy/maven2/"
snapshot_url = "https://s01.oss.sonatype.org/content/repositories/snapshots/"

[publish.android.auth]
[publish.android.auth.credentials]
# For Maven Central, use OSSRH environment variables
username = "${OSSRH_USERNAME}"
password = "${OSSRH_PASSWORD}"
# Note: You can also leave these empty - CCGO will automatically
# check for OSSRH_USERNAME/OSSRH_PASSWORD environment variables

# GPG signing configuration (required for Central)
signing_key_id = "${SIGNING_KEY_ID}"
signing_password = "${SIGNING_PASSWORD}"
signing_key = "${SIGNING_KEY}"

# POM metadata (required for Central)
pom_name = "My Android Library"
pom_description = "Description of your library"
pom_url = "https://github.com/username/project"
license_name = "Apache License 2.0"
license_url = "https://www.apache.org/licenses/LICENSE-2.0"
developer_id = "yourid"
developer_name = "Your Name"
developer_email = "you@example.com"
scm_url = "https://github.com/username/project"
scm_connection = "scm:git:git://github.com/username/project.git"
scm_dev_connection = "scm:git:ssh://github.com/username/project.git"
```

### Publishing to Custom Repository

For private Nexus, Artifactory, or other Maven repositories:

```toml
[publish.android]
repository = "custom"
url = "https://maven.company.com/repository/releases/"
snapshot_url = "https://maven.company.com/repository/snapshots/"  # Optional
group_id = "com.company"
artifact_id = "internal-lib"
version = "2.1.0"

[publish.android.auth]
[publish.android.auth.credentials]
# For custom repos, use generic MAVEN variables or custom ones
username = "${MAVEN_USERNAME}"  # Or "${COMPANY_MAVEN_USER}"
password = "${MAVEN_PASSWORD}"  # Or "${COMPANY_MAVEN_PASS}"

# Signing is optional for custom repos
sign = false
```

## KMP Configuration

Kotlin Multiplatform projects use similar configuration:

```toml
[publish.kmp]
repository = "central"
group_id = "com.example"
artifact_id = "mylib-kmp"
version = "1.0.0"

[publish.kmp.auth]
[publish.kmp.auth.credentials]
username = "${MAVEN_USERNAME}"
password = "${MAVEN_PASSWORD}"
```

## Environment Variables

### Variable Priority by Repository Type

CCGO automatically checks for environment variables in this order:

#### For Maven Central
1. Variables specified in CCGO.toml (e.g., `${OSSRH_USERNAME}`)
2. `OSSRH_USERNAME` / `OSSRH_PASSWORD` (recommended)
3. `SONATYPE_USERNAME` / `SONATYPE_PASSWORD` (alternative)
4. `MAVEN_USERNAME` / `MAVEN_PASSWORD` (fallback)

#### For Custom Repositories
1. Variables specified in CCGO.toml (e.g., `${COMPANY_MAVEN_USER}`)
2. `MAVEN_USERNAME` / `MAVEN_PASSWORD` (standard)

### Standard Variables

```bash
# Maven Central authentication (use these for Central)
export OSSRH_USERNAME="your-sonatype-username"
export OSSRH_PASSWORD="your-sonatype-password"

# Alternative names for Maven Central
export SONATYPE_USERNAME="your-sonatype-username"
export SONATYPE_PASSWORD="your-sonatype-password"

# Generic Maven authentication (use for custom repos)
export MAVEN_USERNAME="your-username"
export MAVEN_PASSWORD="your-password"

# GPG signing (required for Maven Central)
export SIGNING_KEY_ID="your-gpg-key-id"
export SIGNING_PASSWORD="your-gpg-passphrase"
export SIGNING_KEY="-----BEGIN PGP PRIVATE KEY BLOCK-----
...your GPG private key content...
-----END PGP PRIVATE KEY BLOCK-----"
```

### Custom Variables

You can reference any environment variable using `${VAR_NAME}` syntax:

```toml
[publish.android.auth.credentials]
username = "${MY_CUSTOM_USER_VAR}"
password = "${MY_CUSTOM_PASS_VAR}"
```

## Command Line Usage

### Interactive Mode

Simply run the publish command and follow prompts:

```bash
ccgo publish android
# Prompts for repository type selection
```

### Direct Publishing

Specify repository type via command line:

```bash
# Publish to Maven Local
ccgo publish android --repo local

# Publish to Maven Central
ccgo publish android --repo central

# Publish to Custom Repository
ccgo publish android --repo custom --repo-url https://maven.company.com

# Verify after publishing
ccgo publish android --repo local --verify
```

## Maven Central Setup

### Prerequisites

1. **OSSRH Account**: Create account at https://issues.sonatype.org/
2. **Namespace**: Claim your group ID namespace (e.g., com.example)
3. **GPG Key**: Generate and publish GPG key for signing

### GPG Key Setup

```bash
# Generate GPG key
gpg --gen-key

# List keys to find your key ID
gpg --list-secret-keys --keyid-format LONG
# Look for line like: sec   rsa4096/XXXXXXXXXXXXXXXX

# Export public key to keyserver
gpg --keyserver keyserver.ubuntu.com --send-keys XXXXXXXXXXXXXXXX

# Export private key for CI/CD
gpg --armor --export-secret-keys XXXXXXXXXXXXXXXX > private-key.asc
```

### Setting Up Environment

```bash
# In your shell profile (.bashrc, .zshrc, etc.)
export OSSRH_USERNAME="your-sonatype-username"
export OSSRH_PASSWORD="your-sonatype-password"
export SIGNING_KEY_ID="XXXXXXXXXXXXXXXX"
export SIGNING_PASSWORD="your-gpg-passphrase"

# For the private key, you can either:
# Option 1: Export directly (be careful with newlines)
export SIGNING_KEY=$(cat private-key.asc)

# Option 2: Reference a file in CCGO.toml
# signing_key_file = "/path/to/private-key.asc"
```

## CI/CD Integration

### GitHub Actions

```yaml
- name: Publish to Maven Central
  env:
    OSSRH_USERNAME: ${{ secrets.OSSRH_USERNAME }}
    OSSRH_PASSWORD: ${{ secrets.OSSRH_PASSWORD }}
    SIGNING_KEY_ID: ${{ secrets.SIGNING_KEY_ID }}
    SIGNING_PASSWORD: ${{ secrets.SIGNING_PASSWORD }}
    SIGNING_KEY: ${{ secrets.SIGNING_KEY }}
  run: |
    ccgo publish android --repo central
```

### GitLab CI

```yaml
publish:
  script:
    - ccgo publish android --repo central
  variables:
    OSSRH_USERNAME: $CI_OSSRH_USERNAME
    OSSRH_PASSWORD: $CI_OSSRH_PASSWORD
    SIGNING_KEY_ID: $CI_SIGNING_KEY_ID
    SIGNING_PASSWORD: $CI_SIGNING_PASSWORD
    SIGNING_KEY: $CI_SIGNING_KEY
```

## Migration from local.properties

If you're migrating from the old `local.properties` system:

### Old Way (local.properties)
```properties
GROUP=com.example
POM_ARTIFACT_ID=mylib
VERSION_NAME=1.0.0
OSSRH_USERNAME=username
OSSRH_PASSWORD=password
```

### New Way (CCGO.toml)
```toml
[publish.android]
repository = "central"
group_id = "com.example"
artifact_id = "mylib"
version = "1.0.0"

[publish.android.auth.credentials]
username = "${OSSRH_USERNAME}"
password = "${OSSRH_PASSWORD}"
```

Benefits of the new approach:
- ✅ Centralized configuration in CCGO.toml
- ✅ Support for multiple repository types
- ✅ Environment variable expansion
- ✅ No need to manage multiple properties files
- ✅ Consistent with other CCGO configurations

## Troubleshooting

### Authentication Failed

```bash
# Check environment variables are set
echo $MAVEN_USERNAME
echo $MAVEN_PASSWORD

# For Maven Central, also check
echo $SIGNING_KEY_ID
```

### Missing GPG Key

```
ERROR: Maven Central requires signing configuration
```

Solution: Ensure all three signing variables are set:
- SIGNING_KEY_ID
- SIGNING_PASSWORD
- SIGNING_KEY

### Custom Repository 404

```
ERROR: Failed to upload: 404 Not Found
```

Check:
1. Repository URL is correct
2. You have write permissions
3. The repository accepts the artifact type (AAR, JAR)

### Verification Failed

After publishing with `--verify`, if verification fails:
1. Check network connectivity
2. For Maven Local, check file permissions in ~/.m2/repository/
3. For remote repos, allow time for synchronization

## Best Practices

1. **Never commit credentials** - Always use environment variables
2. **Use .gitignore** - Exclude any local credential files
3. **Test locally first** - Publish to Maven Local before Central
4. **Use semantic versioning** - Follow major.minor.patch format
5. **Automate in CI/CD** - Set up automated publishing on tags
6. **Verify publications** - Use `--verify` flag to confirm uploads

## Complete Example

Here's a complete example for a production Android library:

```toml
[project]
name = "awesome-lib"
version = "2.3.1"
description = "An awesome Android library"

[publish.android]
repository = "central"
group_id = "com.awesomecompany"
artifact_id = "awesome-lib-android"
version = "${project.version}"  # Reference project version

# Publishing options
sign = true
sources = true
javadoc = true

# POM metadata
pom_name = "Awesome Android Library"
pom_description = "An awesome library for Android development"
pom_url = "https://github.com/awesomecompany/awesome-lib"

license_name = "MIT License"
license_url = "https://opensource.org/licenses/MIT"

developer_id = "awesomedev"
developer_name = "Awesome Developer"
developer_email = "dev@awesomecompany.com"

scm_url = "https://github.com/awesomecompany/awesome-lib"
scm_connection = "scm:git:git://github.com/awesomecompany/awesome-lib.git"
scm_dev_connection = "scm:git:ssh://github.com/awesomecompany/awesome-lib.git"

[publish.android.auth]
[publish.android.auth.credentials]
username = "${OSSRH_USERNAME}"
password = "${OSSRH_PASSWORD}"
signing_key_id = "${SIGNING_KEY_ID}"
signing_password = "${SIGNING_PASSWORD}"
signing_key = "${SIGNING_KEY}"
```

Then publish with:
```bash
ccgo publish android --repo central --verify
```