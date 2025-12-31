//! Toolchain detection and configuration
//!
//! This module provides utilities for detecting installed toolchains
//! (compilers, SDKs, build tools) and configuring CMake to use them.

pub mod android_ndk;
pub mod linux;
pub mod mingw;
pub mod msvc;
pub mod ohos;
pub mod xcode;

use std::path::PathBuf;

use anyhow::Result;

/// Generic toolchain trait
pub trait Toolchain {
    /// Get the toolchain name
    fn name(&self) -> &str;

    /// Check if the toolchain is available
    fn is_available(&self) -> bool;

    /// Get the path to the toolchain (if applicable)
    fn path(&self) -> Option<PathBuf>;

    /// Get CMake variables for this toolchain
    fn cmake_variables(&self) -> Vec<(String, String)>;

    /// Validate the toolchain is properly configured
    fn validate(&self) -> Result<()>;
}

/// Find an executable in PATH
pub fn find_executable(name: &str) -> Option<PathBuf> {
    which::which(name).ok()
}

/// Find an executable, checking multiple possible names
pub fn find_executable_any(names: &[&str]) -> Option<PathBuf> {
    for name in names {
        if let Some(path) = find_executable(name) {
            return Some(path);
        }
    }
    None
}

/// Get an environment variable as PathBuf
pub fn get_env_path(name: &str) -> Option<PathBuf> {
    std::env::var(name).ok().map(PathBuf::from)
}

/// Check if a path exists and is a directory
pub fn is_valid_directory(path: &PathBuf) -> bool {
    path.exists() && path.is_dir()
}

/// Check if a path exists and is a file
pub fn is_valid_file(path: &PathBuf) -> bool {
    path.exists() && path.is_file()
}

/// Compiler type for C/C++ toolchains
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerType {
    /// GNU Compiler Collection
    Gcc,
    /// LLVM Clang
    Clang,
    /// Microsoft Visual C++
    Msvc,
    /// Other/Unknown
    Other(String),
}

impl std::fmt::Display for CompilerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerType::Gcc => write!(f, "gcc"),
            CompilerType::Clang => write!(f, "clang"),
            CompilerType::Msvc => write!(f, "msvc"),
            CompilerType::Other(name) => write!(f, "{}", name),
        }
    }
}

/// C/C++ compiler information
#[derive(Debug, Clone)]
pub struct CompilerInfo {
    /// Compiler type
    pub compiler_type: CompilerType,
    /// Path to C compiler
    pub cc: PathBuf,
    /// Path to C++ compiler
    pub cxx: PathBuf,
    /// Compiler version string
    pub version: String,
}

impl CompilerInfo {
    /// Get CMake variables for this compiler
    pub fn cmake_variables(&self) -> Vec<(String, String)> {
        vec![
            ("CMAKE_C_COMPILER".to_string(), self.cc.display().to_string()),
            (
                "CMAKE_CXX_COMPILER".to_string(),
                self.cxx.display().to_string(),
            ),
        ]
    }
}

/// Detect the default C/C++ compiler
pub fn detect_default_compiler() -> Option<CompilerInfo> {
    // Try clang first (preferred on macOS)
    if let (Some(cc), Some(cxx)) = (find_executable("clang"), find_executable("clang++")) {
        let version = get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
        return Some(CompilerInfo {
            compiler_type: CompilerType::Clang,
            cc,
            cxx,
            version,
        });
    }

    // Try gcc
    if let (Some(cc), Some(cxx)) = (find_executable("gcc"), find_executable("g++")) {
        let version = get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
        return Some(CompilerInfo {
            compiler_type: CompilerType::Gcc,
            cc,
            cxx,
            version,
        });
    }

    // Try cc/c++ (generic)
    if let (Some(cc), Some(cxx)) = (find_executable("cc"), find_executable("c++")) {
        let version = get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
        return Some(CompilerInfo {
            compiler_type: CompilerType::Other("cc".to_string()),
            cc,
            cxx,
            version,
        });
    }

    None
}

/// Get compiler version string
fn get_compiler_version(compiler: &PathBuf) -> Option<String> {
    let output = std::process::Command::new(compiler)
        .arg("--version")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|s| s.to_string())
}
