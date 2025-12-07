# Canon Platform Configuration Guide

This guide explains how to configure CCGO to publish artifacts to Canon platform.

## Configuration in CCGO.toml

Add the following section to your `CCGO.toml` file:

```toml
# Canon publishing configuration
[publish.canon]
registry = "https://canon.example.com"  # Your Canon registry URL
group_id = "com.example"                 # Maven-style group ID
artifact_id = "mylib"                    # Artifact name (defaults to project name)

# Authentication configuration
[publish.canon.auth]
method = "token"  # Authentication method: "token", "oauth2", or "basic"

# Credentials configuration (choose based on auth method)
[publish.canon.auth.credentials]
# For token authentication (recommended)
token = "your-canon-token"
# Or use a token file:
# token_file = "/path/to/canon-token.txt"

# For OAuth2 authentication
# client_id = "your-client-id"
# client_secret = "your-client-secret"
# scope = "publish"  # Optional, defaults to "publish"
# token_url = "https://canon.example.com/oauth/token"  # Optional

# For basic authentication
# username = "your-username"
# password = "your-password"
```

## Environment Variables

You can also use environment variables to provide credentials (takes precedence over CCGO.toml):

### Token Authentication
```bash
export CANON_TOKEN="your-canon-token"
```

### OAuth2 Authentication
```bash
export CANON_CLIENT_ID="your-client-id"
export CANON_CLIENT_SECRET="your-client-secret"
```

### Basic Authentication
```bash
export CANON_USERNAME="your-username"
export CANON_PASSWORD="your-password"
```

## Publishing Commands

### Publish all platforms
```bash
ccgo publish canon
```

### Publish specific platform
```bash
ccgo publish canon --platform android
ccgo publish canon --platform ios
ccgo publish canon --platform linux
```

### Override registry URL
```bash
ccgo publish canon --registry https://alt-canon.example.com
```

### Verify after upload
```bash
ccgo publish canon --verify
```

## Artifact Naming Convention

Artifacts are published with the following naming patterns:

- **Android AAR**: `{artifact_id}-{version}.aar`
- **iOS Library**: `{artifact_id}-{version}-ios-a.a`
- **Windows Library**: `{artifact_id}-{version}-windows-lib.lib`
- **Linux Library**: `{artifact_id}-{version}-linux-a.a`
- **macOS Library**: `{artifact_id}-{version}-macos-a.a`

## Full Example

Here's a complete example for a cross-platform library project:

```toml
[project]
name = "mylib"
version = "1.2.3"
description = "My cross-platform library"

[publish.canon]
registry = "https://canon.company.com"
group_id = "com.company.libs"
artifact_id = "mylib"

[publish.canon.auth]
method = "oauth2"

[publish.canon.auth.credentials]
client_id = "${CANON_CLIENT_ID}"  # Will read from environment
client_secret = "${CANON_CLIENT_SECRET}"  # Will read from environment
scope = "publish read"
```

## Security Best Practices

1. **Never commit credentials to version control**
   - Use environment variables for sensitive data
   - Use `.gitignore` to exclude token files

2. **Use token files for local development**
   ```toml
   [publish.canon.auth.credentials]
   token_file = "~/.canon/token"  # Keep token in home directory
   ```

3. **Use OAuth2 for production**
   - OAuth2 provides better security with token expiration
   - Tokens are automatically refreshed

4. **Restrict token permissions**
   - Use tokens with minimal required permissions
   - Create separate tokens for CI/CD vs local development

## Troubleshooting

### Authentication Failed
- Check if credentials are correctly configured
- Verify environment variables are set
- Test authentication: The publish command will validate auth before uploading

### No Artifacts Found
- Ensure you've built the project first: `ccgo build <platform>`
- Check that artifacts exist in the `bin/` directory
- Verify platform naming in artifact files

### Upload Failed
- Check network connectivity to Canon registry
- Verify you have write permissions for the group_id
- Check if the artifact already exists (may need versioning)

### Verification Failed
- This usually indicates a checksum mismatch
- Try re-uploading the artifact
- Check for network issues during upload