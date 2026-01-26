# VS Code Extension Publishing Guide

## Pre-release Preparation

### 1. Install vsce (VS Code Extension Manager)

```bash
npm install -g @vscode/vsce
```

### 2. Create Azure DevOps Account and Personal Access Token (PAT)

1. Visit [Azure DevOps](https://dev.azure.com)
2. Register/Login
3. Click on user icon in top right → **Personal access tokens**
4. Click **+ New Token**
5. Configure Token:
   - Name: `vscode-marketplace`
   - Organization: **All accessible organizations**
   - Expiration: Custom (recommended 90 days or longer)
   - Scopes: Select **Custom defined**
     - **Marketplace**: Check **Manage**
6. Click **Create**, **Copy Token immediately** (shown only once!)

### 3. Create Publisher

```bash
# Login with your PAT
vsce login <publisher-name>

# For example:
vsce login ccgo
```

Enter the PAT you just copied.

**Or** create a Publisher through the [Visual Studio Marketplace Publisher Management](https://marketplace.visualstudio.com/manage) page.

### 4. Update publisher field in package.json

Ensure the `publisher` field in `package.json` matches your created publisher name:

```json
{
  "publisher": "ccgo"
}
```

## Pre-release Checklist

### 1. Check Required Fields

Ensure `package.json` contains the following fields:

- ✅ `name` - Extension name (lowercase, no spaces)
- ✅ `displayName` - Display name
- ✅ `description` - Description
- ✅ `version` - Version number (follows semver)
- ✅ `publisher` - Publisher name
- ✅ `engines.vscode` - Minimum VS Code version
- ✅ `categories` - Categories
- ✅ `keywords` - Keywords
- ✅ `repository` - Repository URL
- ✅ `license` - License

### 2. Add README.md

Create `README.md` file containing:
- Extension introduction
- Features
- Installation method
- Usage instructions
- Configuration options
- Screenshots/GIF demos

### 3. Add CHANGELOG.md

Create `CHANGELOG.md` to record version changes:

```markdown
# Change Log

## [0.1.0] - 2025-01-22

### Added
- Initial release
- Syntax highlighting for CCGO.toml
- CCGO.toml validation with JSON Schema
- Build tasks for all platforms
- Dependency tree view
- Code snippets for CCGO.toml
```

### 4. Add LICENSE

Ensure LICENSE file exists (currently MIT).

### 5. Add Icon (optional but recommended)

Add a 128x128 PNG icon:

```json
{
  "icon": "icon.png"
}
```

### 6. Build and Test

```bash
# Install dependencies
npm install

# Build
npm run build

# Test
npm test

# Test extension locally
code --install-extension ./ccgo-0.1.0.vsix
```

## Publishing Steps

### Method 1: Publish via vsce command line

#### 1. Package the extension

```bash
# In vscode-ccgo directory
vsce package

# Generates ccgo-0.1.0.vsix file
```

#### 2. Publish to Marketplace

```bash
# First publish
vsce publish

# Or auto-increment version
vsce publish patch  # 0.1.0 -> 0.1.1
vsce publish minor  # 0.1.0 -> 0.2.0
vsce publish major  # 0.1.0 -> 1.0.0

# Or publish an existing .vsix file
vsce publish -p <path-to-vsix>
```

#### 3. Verify Publication

Wait a few minutes after publishing, then visit:
- Marketplace: `https://marketplace.visualstudio.com/items?itemName=<publisher>.<name>`
- Example: `https://marketplace.visualstudio.com/items?itemName=ccgo.ccgo`

### Method 2: Upload via Web Interface

1. Visit [Marketplace Publisher Management](https://marketplace.visualstudio.com/manage)
2. Login with Azure DevOps account
3. Select your Publisher
4. Click **+ New extension** → **Visual Studio Code**
5. Upload `.vsix` file
6. Fill in additional information and publish

## Updating Published Extension

### 1. Update Version Number

Edit `package.json`:

```json
{
  "version": "0.1.1"
}
```

### 2. Update CHANGELOG.md

Record changes in this update.

### 3. Rebuild and Publish

```bash
npm run build
vsce publish
```

## Common Issues

### 1. Publish failed: Missing README.md

**Solution**: Ensure `README.md` file exists in root directory.

### 2. Publish failed: Missing LICENSE

**Solution**: Add `LICENSE` file, or add to `package.json`:

```json
{
  "license": "SEE LICENSE IN LICENSE.txt"
}
```

### 3. Publish failed: Invalid Personal Access Token

**Solution**:
```bash
vsce logout
vsce login <publisher-name>
# Enter new PAT
```

### 4. Package too large

**Solution**: Add `.vscodeignore` file to exclude unnecessary files:

```
.vscode/**
.github/**
.gitignore
.eslintrc.json
tsconfig.json
webpack.config.js
src/**
out/**
node_modules/**
*.vsix
```

### 5. Need to change Publisher

**Solution**:
1. Create new publisher in Azure DevOps
2. Update `publisher` field in `package.json`
3. Republish

## Post-release Management

### View Statistics

Visit [Marketplace Publisher Management](https://marketplace.visualstudio.com/manage) to view:
- Downloads
- Ratings
- Reviews
- Installation trends

### Respond to User Feedback

- Monitor GitHub Issues
- Reply to Marketplace reviews
- Fix bugs promptly

### Version Management

Follow [Semantic Versioning](https://semver.org/):
- **Patch** (0.0.x): Bug fixes
- **Minor** (0.x.0): New features, backward compatible
- **Major** (x.0.0): Breaking changes

## Complete Publishing Script

Create `scripts/publish.sh`:

```bash
#!/bin/bash

# Check for uncommitted changes
if [[ -n $(git status -s) ]]; then
  echo "Error: Working directory not clean"
  exit 1
fi

# Build
echo "Building..."
npm run build

# Test
echo "Testing..."
npm test

# Package
echo "Packaging..."
vsce package

# Publish
echo "Publishing..."
vsce publish

echo "Done!"
```

Usage:
```bash
chmod +x scripts/publish.sh
./scripts/publish.sh
```

## Quick Publish Commands

```bash
# One-liner: build, package, publish
npm run build && vsce publish patch

# Or package first to test, then publish after confirmation
npm run build && vsce package
# Test the .vsix file
code --install-extension ./ccgo-0.1.0.vsix
# Publish after confirmation
vsce publish
```

## Reference Links

- [VS Code Publishing Extensions](https://code.visualstudio.com/api/working-with-extensions/publishing-extension)
- [vsce Documentation](https://github.com/microsoft/vscode-vsce)
- [Extension Manifest](https://code.visualstudio.com/api/references/extension-manifest)
- [Marketplace Publishing](https://marketplace.visualstudio.com/manage)
