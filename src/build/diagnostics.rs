//! Build failure diagnostics and pattern matching
//!
//! This module analyzes build errors and provides actionable solutions
//! based on common failure patterns.

use regex::Regex;
use std::path::Path;

/// Build error category
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Missing dependency
    MissingDependency,
    /// Compiler error
    CompilerError,
    /// Linker error
    LinkerError,
    /// CMake configuration error
    CMakeError,
    /// Android NDK error
    AndroidNdkError,
    /// Xcode/Apple toolchain error
    XcodeError,
    /// Permission error
    PermissionError,
    /// Disk space error
    DiskSpaceError,
    /// Unknown error
    Unknown,
}

/// Diagnostic result with suggested solutions
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Error category
    pub category: ErrorCategory,
    /// Brief description of the issue
    pub description: String,
    /// Suggested solutions (ordered by likelihood)
    pub solutions: Vec<String>,
    /// Original error messages that matched
    pub matched_errors: Vec<String>,
}

impl Diagnostic {
    /// Create a new diagnostic
    pub fn new(
        category: ErrorCategory,
        description: impl Into<String>,
        solutions: Vec<String>,
    ) -> Self {
        Self {
            category,
            description: description.into(),
            solutions,
            matched_errors: Vec::new(),
        }
    }

    /// Add a matched error message
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.matched_errors.push(error.into());
        self
    }

    /// Print the diagnostic in a user-friendly format
    pub fn print(&self) {
        use console::style;

        eprintln!("\n{}", style("=".repeat(80)).dim());
        eprintln!("{} Build Failure Diagnostics", style("🔍").cyan());
        eprintln!("{}", style("=".repeat(80)).dim());

        eprintln!("\n{} {}", style("Category:").bold(), self.category.as_str());
        eprintln!("{} {}", style("Issue:").bold(), self.description);

        if !self.matched_errors.is_empty() {
            eprintln!("\n{}", style("Matched Errors:").bold());
            for (i, error) in self.matched_errors.iter().enumerate().take(3) {
                eprintln!("  {}. {}", i + 1, style(error).dim());
            }
            if self.matched_errors.len() > 3 {
                eprintln!("  ... and {} more", self.matched_errors.len() - 3);
            }
        }

        eprintln!("\n{}", style("Suggested Solutions:").yellow().bold());
        for (i, solution) in self.solutions.iter().enumerate() {
            eprintln!("  {}. {}", i + 1, solution);
        }

        eprintln!();
    }
}

impl ErrorCategory {
    fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::MissingDependency => "Missing Dependency",
            ErrorCategory::CompilerError => "Compiler Error",
            ErrorCategory::LinkerError => "Linker Error",
            ErrorCategory::CMakeError => "CMake Configuration Error",
            ErrorCategory::AndroidNdkError => "Android NDK Error",
            ErrorCategory::XcodeError => "Xcode/Apple Toolchain Error",
            ErrorCategory::PermissionError => "Permission Error",
            ErrorCategory::DiskSpaceError => "Disk Space Error",
            ErrorCategory::Unknown => "Unknown Error",
        }
    }
}

/// Analyze build output and provide diagnostics
pub fn analyze_build_failure(
    output: &str,
    platform: &str,
    _project_root: &Path,
) -> Option<Diagnostic> {
    // Try each pattern matcher in order of specificity
    if let Some(diag) = check_missing_dependency(output) {
        return Some(diag);
    }

    if let Some(diag) = check_cmake_errors(output) {
        return Some(diag);
    }

    if let Some(diag) = check_android_ndk_errors(output, platform) {
        return Some(diag);
    }

    if let Some(diag) = check_xcode_errors(output, platform) {
        return Some(diag);
    }

    if let Some(diag) = check_linker_errors(output) {
        return Some(diag);
    }

    if let Some(diag) = check_compiler_errors(output) {
        return Some(diag);
    }

    if let Some(diag) = check_permission_errors(output) {
        return Some(diag);
    }

    if let Some(diag) = check_disk_space_errors(output) {
        return Some(diag);
    }

    None
}

/// Check for missing dependency errors
fn check_missing_dependency(output: &str) -> Option<Diagnostic> {
    let patterns = [
        (r"fatal error: (.+\.h[pp]*): No such file or directory", "C++ header"),
        (r"Could not find package '(.+)'", "CMake package"),
        (r"Package '(.+)' not found", "pkg-config package"),
        (r"library not found for -l(\w+)", "library"),
    ];

    for (pattern, dep_type) in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(cap) = re.captures(output) {
                let dep_name = cap.get(1).map_or("unknown", |m| m.as_str());

                let diag = Diagnostic::new(
                    ErrorCategory::MissingDependency,
                    format!("Missing {} dependency: {}", dep_type, dep_name),
                    vec![
                        format!("Install the {} package/library", dep_name),
                        format!("Add {} to your CCGO.toml dependencies", dep_name),
                        "Run: ccgo install to install dependencies".to_string(),
                        "Check if the dependency path is correct in CMakeLists.txt".to_string(),
                    ],
                ).with_error(cap.get(0).map_or("", |m| m.as_str()));

                return Some(diag);
            }
        }
    }

    None
}

/// Check for CMake configuration errors
fn check_cmake_errors(output: &str) -> Option<Diagnostic> {
    if output.contains("CMake Error") || output.contains("CMake configuration failed") {
        let solutions = if output.contains("Could not find") {
            vec![
                "Install missing CMake dependencies".to_string(),
                "Check your system package manager (apt, brew, etc.)".to_string(),
                "Verify CMake package paths in CMakeLists.txt".to_string(),
            ]
        } else if output.contains("No CMAKE_") || output.contains("CMAKE_.*_COMPILER") {
            vec![
                "Install a C++ compiler (g++, clang, or MSVC)".to_string(),
                "Set CMAKE_CXX_COMPILER environment variable".to_string(),
                "On macOS: install Xcode command line tools with 'xcode-select --install'".to_string(),
                "On Linux: install build-essential with 'sudo apt install build-essential'".to_string(),
            ]
        } else if output.contains("version") {
            vec![
                "Upgrade CMake to a newer version".to_string(),
                "Check minimum CMake version in CMakeLists.txt".to_string(),
                "Install CMake: brew install cmake (macOS) or sudo apt install cmake (Linux)".to_string(),
            ]
        } else {
            vec![
                "Check CMakeLists.txt for syntax errors".to_string(),
                "Verify all CMake variables are set correctly".to_string(),
                "Clean build directory and try again: ccgo clean -y".to_string(),
            ]
        };

        return Some(Diagnostic::new(
            ErrorCategory::CMakeError,
            "CMake configuration failed",
            solutions,
        ));
    }

    None
}

/// Check for Android NDK errors
fn check_android_ndk_errors(output: &str, platform: &str) -> Option<Diagnostic> {
    if !platform.contains("android") && !platform.contains("Android") {
        return None;
    }

    if output.contains("ANDROID_NDK") || output.contains("ndk") && output.contains("not found") {
        return Some(Diagnostic::new(
            ErrorCategory::AndroidNdkError,
            "Android NDK not found or not configured",
            vec![
                "Install Android NDK via Android Studio SDK Manager".to_string(),
                "Set ANDROID_NDK environment variable to your NDK path".to_string(),
                "Example: export ANDROID_NDK=/path/to/android-sdk/ndk/25.1.8937393".to_string(),
                "Or set ANDROID_HOME if using SDK default location".to_string(),
            ],
        ));
    }

    if output.contains("unsupported NDK version") || output.contains("NDK version") {
        return Some(Diagnostic::new(
            ErrorCategory::AndroidNdkError,
            "Incompatible NDK version",
            vec![
                "Update Android NDK to a compatible version (25.x or higher recommended)".to_string(),
                "Check project's minSdk requirements in CCGO.toml".to_string(),
                "Install via Android Studio: Tools → SDK Manager → SDK Tools → NDK".to_string(),
            ],
        ));
    }

    None
}

/// Check for Xcode/Apple toolchain errors
fn check_xcode_errors(output: &str, platform: &str) -> Option<Diagnostic> {
    let is_apple = platform.contains("ios") || platform.contains("macos") ||
                   platform.contains("tvos") || platform.contains("watchos") ||
                   platform.contains("apple");

    if !is_apple {
        return None;
    }

    if output.contains("xcode-select") || output.contains("xcrun") && output.contains("error") {
        return Some(Diagnostic::new(
            ErrorCategory::XcodeError,
            "Xcode command line tools not found",
            vec![
                "Install Xcode from the App Store".to_string(),
                "Install command line tools: sudo xcode-select --install".to_string(),
                "Accept Xcode license: sudo xcodebuild -license accept".to_string(),
                "Set Xcode path: sudo xcode-select -s /Applications/Xcode.app".to_string(),
            ],
        ));
    }

    if output.contains("code signing") || output.contains("DEVELOPMENT_TEAM") {
        return Some(Diagnostic::new(
            ErrorCategory::XcodeError,
            "Code signing configuration required",
            vec![
                "Set DEVELOPMENT_TEAM in Xcode project settings".to_string(),
                "Or disable code signing for development builds".to_string(),
                "For frameworks, code signing is optional unless running on device".to_string(),
            ],
        ));
    }

    if output.contains("deployment target") {
        return Some(Diagnostic::new(
            ErrorCategory::XcodeError,
            "Deployment target version mismatch",
            vec![
                "Update minimum deployment target in CCGO.toml".to_string(),
                "Ensure all dependencies support your minimum version".to_string(),
                "Check platform-specific configuration: [platforms.ios] or [platforms.macos]".to_string(),
            ],
        ));
    }

    None
}

/// Check for linker errors
fn check_linker_errors(output: &str) -> Option<Diagnostic> {
    if output.contains("undefined reference") || output.contains("unresolved external symbol") {
        return Some(Diagnostic::new(
            ErrorCategory::LinkerError,
            "Undefined symbols (linker error)",
            vec![
                "Missing library or object file during linking".to_string(),
                "Add missing libraries to target_link_libraries in CMakeLists.txt".to_string(),
                "Check if all source files are included in the build".to_string(),
                "Verify function/symbol declarations match implementations".to_string(),
            ],
        ));
    }

    if output.contains("duplicate symbol") {
        return Some(Diagnostic::new(
            ErrorCategory::LinkerError,
            "Duplicate symbols (linker error)",
            vec![
                "Multiple definitions of the same symbol".to_string(),
                "Remove duplicate source files from CMakeLists.txt".to_string(),
                "Use inline or static functions to avoid multiple definitions".to_string(),
                "Check for header-only libraries included multiple times".to_string(),
            ],
        ));
    }

    if output.contains("cannot find -l") {
        return Some(Diagnostic::new(
            ErrorCategory::LinkerError,
            "Library not found during linking",
            vec![
                "Install the required system library".to_string(),
                "Add library path to CMAKE_LIBRARY_PATH or LDFLAGS".to_string(),
                "Check that the library name in CMakeLists.txt is correct".to_string(),
            ],
        ));
    }

    None
}

/// Check for compiler errors
fn check_compiler_errors(output: &str) -> Option<Diagnostic> {
    if output.contains("error:") && (output.contains(".cpp:") || output.contains(".cc:") || output.contains(".h:")) {
        // Generic compiler error
        if output.contains("C++") && output.contains("standard") {
            return Some(Diagnostic::new(
                ErrorCategory::CompilerError,
                "C++ standard version incompatibility",
                vec![
                    "Update C++ standard in CMakeLists.txt: set(CMAKE_CXX_STANDARD 17)".to_string(),
                    "Ensure all dependencies support the same C++ standard".to_string(),
                    "Common standards: C++11, C++14, C++17, C++20".to_string(),
                ],
            ));
        }

        return Some(Diagnostic::new(
            ErrorCategory::CompilerError,
            "Compilation failed",
            vec![
                "Fix syntax errors in source code".to_string(),
                "Check compiler warnings for hints".to_string(),
                "Ensure all headers are included correctly".to_string(),
                "Verify C++ standard compatibility".to_string(),
            ],
        ));
    }

    None
}

/// Check for permission errors
fn check_permission_errors(output: &str) -> Option<Diagnostic> {
    if output.contains("Permission denied") || output.contains("EACCES") {
        return Some(Diagnostic::new(
            ErrorCategory::PermissionError,
            "Permission denied",
            vec![
                "Check file/directory permissions".to_string(),
                "Run with appropriate user permissions (may need sudo)".to_string(),
                "Ensure build directory is writable".to_string(),
                "On macOS: check Gatekeeper and Security settings".to_string(),
            ],
        ));
    }

    None
}

/// Check for disk space errors
fn check_disk_space_errors(output: &str) -> Option<Diagnostic> {
    if output.contains("No space left on device") || output.contains("ENOSPC") {
        return Some(Diagnostic::new(
            ErrorCategory::DiskSpaceError,
            "Insufficient disk space",
            vec![
                "Free up disk space on your system".to_string(),
                "Clean build artifacts: ccgo clean -y".to_string(),
                "Remove unused Docker images: docker system prune -a".to_string(),
                "Check disk usage: df -h (Linux/macOS) or dir (Windows)".to_string(),
            ],
        ));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_header() {
        let output = "fatal error: fmt/core.h: No such file or directory";
        let diag = analyze_build_failure(output, "linux", Path::new("."));
        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.category, ErrorCategory::MissingDependency);
        assert!(diag.description.contains("fmt/core.h"));
    }

    #[test]
    fn test_cmake_error() {
        let output = "CMake Error: Could not find package FooBar";
        let diag = analyze_build_failure(output, "linux", Path::new("."));
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().category, ErrorCategory::CMakeError);
    }

    #[test]
    fn test_android_ndk_error() {
        let output = "Error: ANDROID_NDK environment variable not set";
        let diag = analyze_build_failure(output, "android", Path::new("."));
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().category, ErrorCategory::AndroidNdkError);
    }

    #[test]
    fn test_xcode_error() {
        let output = "error: xcode-select: command not found";
        let diag = analyze_build_failure(output, "ios", Path::new("."));
        assert!(diag.is_some());
        assert_eq!(diag.unwrap().category, ErrorCategory::XcodeError);
    }
}
