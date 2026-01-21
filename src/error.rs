//! Error types and helpers for user-friendly error messages
//!
//! This module provides custom error types with actionable hints and suggestions
//! to help users quickly resolve common issues.

use std::fmt;
use std::path::PathBuf;

use thiserror::Error;

/// Custom error types with helpful context and suggestions
#[derive(Error, Debug)]
pub enum CcgoError {
    /// Configuration file errors
    #[error("Configuration error: {message}")]
    Config {
        message: String,
        #[source]
        source: Option<anyhow::Error>,
        hint: Option<String>,
    },

    /// Dependency-related errors
    #[error("Dependency error for '{dependency}': {message}")]
    Dependency {
        dependency: String,
        message: String,
        #[source]
        source: Option<anyhow::Error>,
        hint: Option<String>,
    },

    /// Tool/executable not found or misconfigured
    #[error("Missing tool: {tool}")]
    MissingTool {
        tool: String,
        required_for: String,
        hint: String,
    },

    /// Build failure
    #[error("Build failed for {platform}: {message}")]
    BuildFailure {
        platform: String,
        message: String,
        #[source]
        source: Option<anyhow::Error>,
        diagnostics: Vec<String>,
        hint: Option<String>,
    },

    /// Invalid project structure
    #[error("Invalid project structure: {message}")]
    ProjectStructure {
        message: String,
        expected: Vec<String>,
        hint: String,
    },

    /// Version-related errors
    #[error("Version error: {message}")]
    Version {
        message: String,
        current: Option<String>,
        expected: Option<String>,
        hint: String,
    },
}

impl CcgoError {
    /// Create a configuration error with a hint
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
            source: None,
            hint: None,
        }
    }

    /// Create a configuration error with source and hint
    pub fn config_error_with_hint(
        message: impl Into<String>,
        source: Option<anyhow::Error>,
        hint: impl Into<String>,
    ) -> Self {
        Self::Config {
            message: message.into(),
            source,
            hint: Some(hint.into()),
        }
    }

    /// Create a dependency error
    pub fn dependency_error(
        dependency: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::Dependency {
            dependency: dependency.into(),
            message: message.into(),
            source: None,
            hint: None,
        }
    }

    /// Create a dependency error with hint
    pub fn dependency_error_with_hint(
        dependency: impl Into<String>,
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::Dependency {
            dependency: dependency.into(),
            message: message.into(),
            source: None,
            hint: Some(hint.into()),
        }
    }

    /// Create a missing tool error
    pub fn missing_tool(
        tool: impl Into<String>,
        required_for: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::MissingTool {
            tool: tool.into(),
            required_for: required_for.into(),
            hint: hint.into(),
        }
    }

    /// Create a build failure error
    pub fn build_failure(
        platform: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self::BuildFailure {
            platform: platform.into(),
            message: message.into(),
            source: None,
            diagnostics: Vec::new(),
            hint: None,
        }
    }

    /// Create a build failure error with diagnostics
    pub fn build_failure_with_diagnostics(
        platform: impl Into<String>,
        message: impl Into<String>,
        diagnostics: Vec<String>,
        hint: Option<String>,
    ) -> Self {
        Self::BuildFailure {
            platform: platform.into(),
            message: message.into(),
            source: None,
            diagnostics,
            hint,
        }
    }

    /// Create a project structure error
    pub fn project_structure_error(
        message: impl Into<String>,
        expected: Vec<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::ProjectStructure {
            message: message.into(),
            expected,
            hint: hint.into(),
        }
    }

    /// Create a version error
    pub fn version_error(
        message: impl Into<String>,
        hint: impl Into<String>,
    ) -> Self {
        Self::Version {
            message: message.into(),
            current: None,
            expected: None,
            hint: hint.into(),
        }
    }

    /// Display error with formatting and hints
    pub fn display_with_hints(&self) {
        use console::style;

        eprintln!("\n{} {}", style("ERROR:").red().bold(), self);

        match self {
            CcgoError::Config { hint, .. }
            | CcgoError::Dependency { hint, .. }
            | CcgoError::BuildFailure { hint, .. } => {
                if let Some(h) = hint {
                    eprintln!("\n{} {}", style("HINT:").yellow().bold(), h);
                }
            }
            CcgoError::MissingTool { hint, .. }
            | CcgoError::ProjectStructure { hint, .. }
            | CcgoError::Version { hint, .. } => {
                eprintln!("\n{} {}", style("HINT:").yellow().bold(), hint);
            }
        }

        // Display diagnostics for build failures
        if let CcgoError::BuildFailure { diagnostics, .. } = self {
            if !diagnostics.is_empty() {
                eprintln!("\n{}", style("DIAGNOSTICS:").cyan().bold());
                for diag in diagnostics {
                    eprintln!("  • {}", diag);
                }
            }
        }

        // Display expected structure for project errors
        if let CcgoError::ProjectStructure { expected, .. } = self {
            if !expected.is_empty() {
                eprintln!("\n{}", style("EXPECTED:").cyan().bold());
                for exp in expected {
                    eprintln!("  • {}", exp);
                }
            }
        }

        eprintln!();
    }
}

/// Helper trait for adding hints to Result types
pub trait ResultExt<T> {
    /// Add a hint to an error
    fn with_hint(self, hint: impl Into<String>) -> Result<T, CcgoError>;

    /// Add context with a hint
    fn context_with_hint(
        self,
        context: impl Into<String>,
        hint: impl Into<String>,
    ) -> Result<T, CcgoError>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn with_hint(self, hint: impl Into<String>) -> Result<T, CcgoError> {
        self.map_err(|e| CcgoError::config_error_with_hint(
            e.to_string(),
            Some(e.into()),
            hint,
        ))
    }

    fn context_with_hint(
        self,
        context: impl Into<String>,
        hint: impl Into<String>,
    ) -> Result<T, CcgoError> {
        self.map_err(|e| CcgoError::config_error_with_hint(
            format!("{}: {}", context.into(), e),
            Some(e.into()),
            hint,
        ))
    }
}

/// Common error hints for missing tools
pub mod hints {
    /// Get hint for missing CMake
    pub fn cmake() -> &'static str {
        "Install CMake from https://cmake.org/ or use your package manager:\n\
         • macOS: brew install cmake\n\
         • Ubuntu: sudo apt install cmake\n\
         • Windows: winget install Kitware.CMake"
    }

    /// Get hint for missing Git
    pub fn git() -> &'static str {
        "Install Git from https://git-scm.com/ or use your package manager:\n\
         • macOS: brew install git\n\
         • Ubuntu: sudo apt install git\n\
         • Windows: winget install Git.Git"
    }

    /// Get hint for missing Android NDK
    pub fn android_ndk() -> &'static str {
        "Install Android NDK via Android Studio SDK Manager:\n\
         1. Open Android Studio\n\
         2. Go to Tools → SDK Manager\n\
         3. Select SDK Tools tab\n\
         4. Check 'NDK (Side by side)' and 'CMake'\n\
         5. Click Apply\n\
         \n\
         Or set ANDROID_NDK environment variable to your NDK installation path."
    }

    /// Get hint for missing Xcode
    pub fn xcode() -> &'static str {
        "Install Xcode from the App Store:\n\
         1. Open App Store\n\
         2. Search for 'Xcode'\n\
         3. Click Install\n\
         4. Run: sudo xcode-select --install"
    }

    /// Get hint for missing Visual Studio
    pub fn visual_studio() -> &'static str {
        "Install Visual Studio with C++ support:\n\
         1. Download from https://visualstudio.microsoft.com/\n\
         2. Select 'Desktop development with C++' workload\n\
         3. Install\n\
         \n\
         Or use Visual Studio Build Tools for CI/headless environments."
    }

    /// Get hint for missing MinGW
    pub fn mingw() -> &'static str {
        "Install MinGW-w64:\n\
         • Windows: winget install mingw-w64\n\
         • Or download from: https://www.mingw-w64.org/\n\
         \n\
         Ensure MinGW bin directory is in your PATH."
    }

    /// Get hint for missing Gradle
    pub fn gradle() -> &'static str {
        "Install Gradle:\n\
         • macOS: brew install gradle\n\
         • Or use gradlew wrapper if available in the project"
    }

    /// Get hint for missing Python
    pub fn python() -> &'static str {
        "Install Python 3.7+ from https://www.python.org/ or use your package manager:\n\
         • macOS: brew install python3\n\
         • Ubuntu: sudo apt install python3\n\
         • Windows: winget install Python.Python.3.11"
    }

    /// Get hint for missing Doxygen
    pub fn doxygen() -> &'static str {
        "Install Doxygen for documentation generation:\n\
         • macOS: brew install doxygen\n\
         • Ubuntu: sudo apt install doxygen\n\
         • Windows: winget install Doxygen.Doxygen"
    }

    /// Get hint for CCGO.toml not found
    pub fn ccgo_toml_not_found() -> &'static str {
        "Could not find CCGO.toml in current directory or any parent directory.\n\
         \n\
         To create a new CCGO project:\n\
         • Run: ccgo new my-project\n\
         \n\
         To initialize CCGO in an existing project:\n\
         • Run: ccgo init"
    }

    /// Get hint for invalid CCGO.toml
    pub fn invalid_ccgo_toml() -> &'static str {
        "CCGO.toml is invalid. Common issues:\n\
         • Missing [package] section\n\
         • Invalid TOML syntax (check quotes, brackets, commas)\n\
         • Invalid version string (must be semver like '1.0.0')\n\
         \n\
         See documentation: https://ccgo.dev/reference/ccgo-toml/"
    }

    /// Get hint for dependency resolution failure
    pub fn dependency_resolution() -> &'static str {
        "Dependency resolution failed. Try:\n\
         • Check dependency versions in CCGO.toml\n\
         • Run: ccgo update to update to latest versions\n\
         • Check network connection for git dependencies\n\
         • Check that dependency paths exist for path dependencies"
    }

    /// Get hint for build configuration issues
    pub fn build_config() -> &'static str {
        "Build configuration is invalid. Common issues:\n\
         • Unsupported architecture specified\n\
         • Invalid CMake variables in [build] section\n\
         • Missing platform-specific configuration\n\
         \n\
         See documentation: https://ccgo.dev/reference/build-config/"
    }

    /// Get hint for lockfile issues
    pub fn lockfile_mismatch() -> &'static str {
        "CCGO.lock is out of sync with CCGO.toml.\n\
         \n\
         To fix:\n\
         • Run: ccgo install to update lockfile\n\
         • Or delete CCGO.lock and run ccgo install again\n\
         \n\
         In CI, use --locked flag to enforce lockfile usage."
    }
}
