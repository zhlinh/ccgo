# CCGO JetBrains Plugin

A JetBrains IDE plugin for CCGO - the C++ cross-platform build system.

## Features

### CCGO.toml Support
- **Syntax Highlighting**: Full TOML syntax highlighting for CCGO configuration files
- **Schema Validation**: Real-time validation with error hints based on JSON Schema
- **Auto-completion**: Context-aware completions for CCGO sections, platforms, and options
- **Live Templates**: Code snippets for common patterns (type `ccgo-` to see available templates)

### Build Integration
- **Run Configurations**: Create and save build configurations for different platforms
- **Build Actions**: Quick access to build, test, clean, and other commands via Tools > CCGO
- **Terminal Integration**: Execute commands in the integrated terminal or run window

### Dependency Management
- **Dependency Tree**: Visualize project dependencies in a dedicated tool window
- **Auto-refresh**: Automatically update the dependency tree when CCGO.toml changes

### Supported Platforms
- Android (AAR)
- iOS (XCFramework)
- macOS (XCFramework)
- Windows (DLL/LIB)
- Linux (SO/A)
- OpenHarmony (HAR)
- Kotlin Multiplatform

## Supported IDEs

- IntelliJ IDEA 2024.1+
- CLion 2024.1+
- Android Studio 2024.1+

## Installation

### From JetBrains Marketplace (Recommended)
1. Open your IDE
2. Go to **Settings/Preferences** > **Plugins**
3. Click **Marketplace** tab
4. Search for "CCGO"
5. Click **Install**

### From Disk
1. Download the plugin ZIP from [Releases](https://github.com/zhlinh/ccgo/releases)
2. Open your IDE
3. Go to **Settings/Preferences** > **Plugins**
4. Click the gear icon and select **Install Plugin from Disk...**
5. Select the downloaded ZIP file

## Usage

### Quick Start
1. Open a project containing a `CCGO.toml` file
2. The plugin will automatically detect the project and show:
   - **CCGO Dependencies** tool window on the right
   - **CCGO** menu under **Tools**

### Building
1. Go to **Tools** > **CCGO** > **Build...**
2. Select your target platform
3. Configure build options (Release, IDE project, architectures)
4. Click **OK** to start the build

Or use Run Configurations:
1. Go to **Run** > **Edit Configurations...**
2. Click **+** and select **CCGO Build**
3. Configure the build options
4. Click **Run** or **Debug**

### Live Templates
Type these prefixes in a CCGO.toml file to insert snippets:

| Prefix | Description |
|--------|-------------|
| `ccgo-package` | Package metadata section |
| `ccgo-build` | Build configuration section |
| `ccgo-deps` | Dependencies section |
| `ccgo-dep-git` | Git dependency |
| `ccgo-dep-path` | Path dependency |
| `ccgo-android` | Android platform config |
| `ccgo-ios` | iOS platform config |
| `ccgo-macos` | macOS platform config |
| `ccgo-maven` | Maven publishing config |
| `ccgo-cocoapods` | CocoaPods publishing config |
| `ccgo-full` | Full CCGO.toml template |

### Configuration
Go to **Settings/Preferences** > **Tools** > **CCGO** to configure:

- **Executable path**: Path to the CCGO executable (default: `ccgo`)
- **Default platform**: Default target platform for builds
- **Auto-refresh dependencies**: Automatically refresh when CCGO.toml changes
- **Show notifications**: Enable/disable build notifications
- **Run builds in terminal**: Execute in terminal vs run window

## Development

### Prerequisites
- JDK 17+
- IntelliJ IDEA (for plugin development)

### Building
```bash
cd jetbrains-ccgo
./gradlew buildPlugin
```

The plugin ZIP will be in `build/distributions/`.

### Running in Sandbox
```bash
./gradlew runIde
```

This launches a sandboxed IDE instance with the plugin installed.

### Testing
```bash
./gradlew test
```

## Project Structure

```
jetbrains-ccgo/
├── build.gradle.kts              # Gradle build configuration
├── settings.gradle.kts           # Gradle settings
├── gradle.properties             # Plugin version, IDE versions
├── src/main/
│   ├── kotlin/com/ccgo/plugin/
│   │   ├── CcgoBundle.kt         # i18n message bundle
│   │   ├── CcgoIcons.kt          # Icon definitions
│   │   ├── CcgoProjectService.kt # Project-level service
│   │   ├── CcgoStartupActivity.kt
│   │   ├── settings/             # Plugin settings
│   │   ├── schema/               # JSON Schema validation
│   │   ├── toolwindow/           # Dependency tree view
│   │   ├── actions/              # Build, test, clean actions
│   │   ├── run/                  # Run configurations
│   │   └── completion/           # Auto-completion
│   └── resources/
│       ├── META-INF/plugin.xml   # Plugin descriptor
│       ├── icons/                # Plugin icons (SVG)
│       ├── messages/             # i18n properties
│       ├── schemas/              # JSON Schema for validation
│       └── liveTemplates/        # Code snippets
└── README.md
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `./gradlew test`
5. Build: `./gradlew buildPlugin`
6. Test in sandbox: `./gradlew runIde`
7. Submit a pull request

## License

MIT License - see [LICENSE](../LICENSE) for details.

## Links

- [CCGO Documentation](https://github.com/zhlinh/ccgo)
- [Report Issues](https://github.com/zhlinh/ccgo/issues)
- [JetBrains Marketplace](https://plugins.jetbrains.com/)
