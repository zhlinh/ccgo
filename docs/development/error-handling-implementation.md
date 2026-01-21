# Error Handling Implementation Guide

> Version: v3.0.10 | Updated: 2026-01-21

## Overview

CCGO's enhanced error handling system provides:

1. **User-friendly error messages** with actionable hints
2. **Graceful degradation** when optional tools are missing
3. **Comprehensive configuration validation** with helpful suggestions
4. **Better build failure diagnostics**

## Architecture

```
src/
├── error.rs                      # Custom error types and hints
├── config/
│   ├── ccgo_toml.rs             # Configuration parsing (enhanced)
│   └── validation.rs            # Comprehensive validation
└── utils/
    └── tools.rs                 # Tool detection and validation
```

## Components

### 1. Error Module (`src/error.rs`)

Custom error types with contextual hints:

```rust
use crate::error::{CcgoError, hints};

// Configuration errors
CcgoError::config_error_with_hint(
    "Invalid package name",
    None,
    "Package name must be a valid C++ identifier"
)

// Dependency errors
CcgoError::dependency_error_with_hint(
    "fmt",
    "Invalid version requirement",
    "Version requirements support: ^1.0.0, ~1.2.0, >=1.0.0"
)

// Missing tool errors
CcgoError::missing_tool(
    "cmake",
    "building projects",
    hints::cmake()  // Provides installation instructions
)

// Build failures with diagnostics
CcgoError::build_failure_with_diagnostics(
    "Android",
    "NDK not found",
    vec!["Check ANDROID_NDK environment variable"],
    Some(hints::android_ndk())
)
```

#### Available Hints

The `hints` module provides installation instructions for common tools:

- `hints::cmake()` - CMake installation
- `hints::git()` - Git installation
- `hints::android_ndk()` - Android NDK setup
- `hints::xcode()` - Xcode installation
- `hints::visual_studio()` - Visual Studio setup
- `hints::mingw()` - MinGW installation
- `hints::gradle()` - Gradle installation
- `hints::python()` - Python installation
- `hints::doxygen()` - Doxygen installation

#### Common Error Patterns

```rust
// CCGO.toml not found
hints::ccgo_toml_not_found()

// Invalid CCGO.toml
hints::invalid_ccgo_toml()

// Dependency resolution failure
hints::dependency_resolution()

// Build configuration issues
hints::build_config()

// Lockfile out of sync
hints::lockfile_mismatch()
```

### 2. Configuration Validation (`src/config/validation.rs`)

Comprehensive validation with helpful error messages:

```rust
use crate::config::validate_config;

// Load and validate configuration
let config = CcgoConfig::load()?;  // Automatically validates

// Manual validation
validate_config(&config)?;
```

#### Validation Checks

**Package Metadata:**
- ✅ Package name (must be valid C++ identifier, not a keyword)
- ✅ Version (must be valid semver: `1.0.0`, `2.3.4-alpha.1`)
- ✅ License (warns about non-standard SPDX identifiers)

**Workspace Configuration:**
- ✅ Non-empty members list for workspace-only configs
- ✅ Valid resolver version ("1" or "2")

**Dependencies:**
- ✅ At least one valid source (version, git, or path)
- ✅ Valid version requirements (`^1.0.0`, `~1.2.0`, `>=1.0.0`)
- ✅ Valid git URLs (must start with `https://`, `git://`, etc.)
- ✅ No conflicting git refs (can't have both branch and tag)
- ✅ Path dependencies exist (warning only)

**Build Configuration:**
- ✅ Parallel jobs > 0
- ✅ Reasonable job count (warns if > 128)

**Platform Configuration:**
- ✅ Android minSdk >= 16 (warns if below recommended)
- ✅ Valid iOS version format (`"12.0"`, `"14.0"`)

**Binaries and Examples:**
- ✅ Valid names (C++ identifiers)
- ✅ Source files exist (warning only)

### 3. Tool Detection (`src/utils/tools.rs`)

Detect build tools with graceful degradation:

```rust
use crate::utils::tools::{
    check_tool,
    require_tool,
    check_tool_with_requirement,
    ToolRequirement,
    PlatformTools,
};

// Check if a tool exists
if let Some(tool_info) = check_tool("cmake") {
    println!("CMake version: {:?}", tool_info.version);
}

// Require a tool (error if missing)
let cmake = require_tool("cmake", "building projects")?;

// Check with requirement level
check_tool_with_requirement(
    "doxygen",
    ToolRequirement::Recommended,
    "documentation generation"
)?;
// ⚠️ Warns if missing, but continues
```

#### Tool Requirement Levels

```rust
pub enum ToolRequirement {
    /// Required - fails with helpful error if missing
    Required,

    /// Optional - silent if missing
    Optional,

    /// Recommended - warns if missing, continues
    Recommended,
}
```

#### Platform-Specific Tool Checking

```rust
use crate::utils::tools::{
    PlatformTools,
    check_android_environment,
    check_apple_environment,
    check_windows_environment,
};

// Check platform-specific tools
let checker = PlatformTools::new("android");
let (required, optional) = checker.check_all()?;

// Android environment
check_android_environment()?;  // Checks ANDROID_NDK, ANDROID_HOME

// Apple environment
check_apple_environment()?;    // Checks Xcode, command line tools

// Windows environment
check_windows_environment("msvc")?;  // Checks MSVC or MinGW
```

## Integration Examples

### Example 1: Enhanced Build Command

```rust
use crate::utils::tools::{PlatformTools, ToolRequirement};
use crate::error::CcgoError;

pub fn execute_build(platform: &str) -> Result<()> {
    // Check required tools
    let checker = PlatformTools::new(platform);
    let (required_tools, optional_tools) = checker.check_all()?;

    println!("✓ Required tools found:");
    for tool in &required_tools {
        println!("  • {}: {}", tool.name,
            tool.version.as_deref().unwrap_or("installed"));
    }

    if !optional_tools.is_empty() {
        println!("✓ Optional tools found:");
        for tool in &optional_tools {
            println!("  • {}", tool.name);
        }
    }

    // Proceed with build...
    Ok(())
}
```

### Example 2: Enhanced Install Command

```rust
use crate::error::{CcgoError, hints};
use crate::config::validate_config;

pub fn execute_install(dep_name: Option<&str>) -> Result<()> {
    // Load and validate config
    let config = CcgoConfig::load()?;  // Auto-validates

    // Check git is available for git dependencies
    let has_git_deps = config.dependencies.iter()
        .any(|d| d.git.is_some());

    if has_git_deps {
        require_tool("git", "installing git dependencies")?;
    }

    // Install dependencies...
    Ok(())
}
```

### Example 3: Better Error Messages

**Before:**
```
Error: Failed to parse CCGO.toml
```

**After:**
```
ERROR: Failed to parse CCGO.toml

Common issues:
• Invalid TOML syntax (check quotes, brackets, commas)
• Typo in section names (should be [package] or [workspace])
• Missing closing brackets or quotes

Validate your TOML at: https://www.toml-lint.com/
```

**Before:**
```
Error: Invalid version requirement '1.0'
```

**After:**
```
ERROR: Invalid version requirement '1.0' for dependency 'fmt'

HINT: Version requirements support:
• Exact: "1.2.3" or "=1.2.3"
• Range: ">=1.0.0, <2.0.0"
• Caret: "^1.0.0" (allows 1.x.x)
• Tilde: "~1.2.0" (allows 1.2.x)
• Wildcard: "1.*" or "1.2.*"
```

## Best Practices

### 1. Always Provide Context

```rust
// Bad
bail!("Tool not found");

// Good
Err(CcgoError::missing_tool(
    "cmake",
    "building C++ projects",
    hints::cmake()
).into())
```

### 2. Use Appropriate Error Types

```rust
// For configuration errors
CcgoError::config_error_with_hint(message, source, hint)

// For dependency errors
CcgoError::dependency_error_with_hint(dep_name, message, hint)

// For missing tools
CcgoError::missing_tool(tool, required_for, hint)

// For build failures
CcgoError::build_failure_with_diagnostics(platform, message, diagnostics, hint)
```

### 3. Validate Early

```rust
// Validate configuration immediately after loading
let config = CcgoConfig::load()?;  // Validates automatically

// Or explicit validation
validate_config(&config)?;
```

### 4. Check Tools Before Building

```rust
// For platform-specific builds
let checker = PlatformTools::new("android");
checker.check_required()?;  // Fails fast with helpful errors

// For optional features
check_tool_with_requirement(
    "doxygen",
    ToolRequirement::Recommended,
    "documentation generation"
)?;
```

## Error Message Guidelines

When creating error messages:

1. **Be specific**: Say what went wrong and where
2. **Be actionable**: Tell user how to fix it
3. **Be concise**: Keep hints focused on the most likely solution
4. **Be helpful**: Include examples, links, or commands to run

Example:
```
❌ Bad: "Invalid configuration"
✅ Good:
ERROR: Invalid package name 'class' in CCGO.toml

HINT: Package name 'class' is a C++ reserved keyword.
Choose a different name, e.g., 'class_lib' or 'libclass'
```

## Testing

All validation logic includes unit tests:

```bash
cargo test error
cargo test validation
cargo test tools
```

## Current Status

✅ **Completed:**
- Custom error types with hints
- Comprehensive configuration validation
- Tool detection with graceful degradation
- Enhanced CCGO.toml parsing errors
- Platform-specific tool checking

📝 **Integration Tasks (Future):**
- Integrate tool checking into `build` command
- Integrate tool checking into `publish` command
- Add more platform-specific validations
- Build failure diagnostics with common solutions

## Migration Guide

To use the new error handling in existing code:

### Before:
```rust
use anyhow::bail;

if tool_missing {
    bail!("cmake not found");
}
```

### After:
```rust
use crate::error::{CcgoError, hints};
use crate::utils::tools::require_tool;

// Option 1: Use tool detection
require_tool("cmake", "building projects")?;

// Option 2: Manual error creation
if tool_missing {
    return Err(CcgoError::missing_tool(
        "cmake",
        "building projects",
        hints::cmake()
    ).into());
}
```

## See Also

- [Contributing Guide](contributing.md)
- [CCGO.toml Reference](../reference/ccgo-toml.md)
- [Roadmap](roadmap.md)
