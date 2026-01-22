# CCGO - VS Code Extension

A comprehensive VS Code extension for CCGO, the C++ cross-platform build system.

## Features

### Syntax Highlighting
- Full syntax highlighting for `CCGO.toml` configuration files
- Recognizes all CCGO sections: `[package]`, `[[dependencies]]`, `[build]`, `[platforms.*]`, `[publish.*]`

### JSON Schema Validation
- Real-time validation of `CCGO.toml` files
- Helpful error messages and suggestions
- Auto-completion for known keys and values

### Code Snippets
- Quick snippets for common CCGO configurations
- Platform configuration templates
- Dependency declaration helpers

### Build Tasks
- Integrated build tasks for all supported platforms
- Quick access to build, test, clean, and install commands
- Debug and Release build configurations

### Dependency Tree View
- Visual dependency tree in the Explorer sidebar
- Shows direct and transitive dependencies
- Auto-refreshes when `CCGO.toml` changes

### Commands
- `CCGO: Build Project` - Build for default platform
- `CCGO: Build for Platform...` - Select platform and build type
- `CCGO: Install Dependencies` - Install project dependencies
- `CCGO: Run Tests` - Execute test suite
- `CCGO: Clean Build Artifacts` - Remove build outputs
- `CCGO: Generate IDE Project` - Create platform-specific IDE project
- `CCGO: Refresh Dependencies` - Update dependency tree view

## Requirements

- [CCGO CLI](https://github.com/zhlinh/ccgo) must be installed and available in PATH
- VS Code 1.85.0 or later

## Installation

### From VS Code Marketplace
Search for "CCGO" in the VS Code Extensions view and click Install.

### From VSIX
1. Download the `.vsix` file from releases
2. In VS Code, go to Extensions view
3. Click `...` menu and select "Install from VSIX..."

### From Source
```bash
cd vscode-ccgo
npm install
npm run compile
# Press F5 in VS Code to launch Extension Development Host
```

## Configuration

### Settings

| Setting | Default | Description |
|---------|---------|-------------|
| `ccgo.executablePath` | `ccgo` | Path to the CCGO executable |
| `ccgo.defaultPlatform` | `null` | Default platform for builds |
| `ccgo.autoRefreshDependencies` | `true` | Auto-refresh dependency tree on CCGO.toml changes |

## Supported Platforms

The extension provides build tasks for all CCGO-supported platforms:
- Android
- iOS
- macOS
- Linux
- Windows
- OpenHarmony (OHOS)
- Kotlin Multiplatform (KMP)

## Development

### Building
```bash
npm install
npm run compile
```

### Testing
```bash
npm test
```

### Packaging
```bash
npm run package
```

This creates a `.vsix` file that can be installed or published.

## License

MIT License - see [LICENSE](../LICENSE) for details.
