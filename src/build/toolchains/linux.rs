//! Linux toolchain detection (GCC, Clang)
//!
//! Detects and configures GCC or Clang compilers for Linux builds.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::{detect_default_compiler, find_executable, CompilerInfo, CompilerType, Toolchain};

/// Linux GCC/Clang toolchain
pub struct LinuxToolchain {
    compiler: CompilerInfo,
}

impl LinuxToolchain {
    /// Detect and create a Linux toolchain
    pub fn detect() -> Result<Self> {
        let compiler = detect_default_compiler()
            .context("No C/C++ compiler found. Please install GCC or Clang.")?;

        Ok(Self { compiler })
    }

    /// Prefer GCC
    pub fn prefer_gcc() -> Result<Self> {
        if let (Some(cc), Some(cxx)) = (find_executable("gcc"), find_executable("g++")) {
            let version = super::get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
            return Ok(Self {
                compiler: CompilerInfo {
                    compiler_type: CompilerType::Gcc,
                    cc,
                    cxx,
                    version,
                },
            });
        }
        Self::detect()
    }

    /// Prefer Clang
    pub fn prefer_clang() -> Result<Self> {
        if let (Some(cc), Some(cxx)) = (find_executable("clang"), find_executable("clang++")) {
            let version = super::get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
            return Ok(Self {
                compiler: CompilerInfo {
                    compiler_type: CompilerType::Clang,
                    cc,
                    cxx,
                    version,
                },
            });
        }
        Self::detect()
    }

    /// Get the compiler info
    pub fn compiler(&self) -> &CompilerInfo {
        &self.compiler
    }
}

impl Toolchain for LinuxToolchain {
    fn name(&self) -> &str {
        match self.compiler.compiler_type {
            CompilerType::Gcc => "gcc",
            CompilerType::Clang => "clang",
            _ => "cc",
        }
    }

    fn is_available(&self) -> bool {
        self.compiler.cc.exists() && self.compiler.cxx.exists()
    }

    fn path(&self) -> Option<PathBuf> {
        self.compiler.cc.parent().map(|p| p.to_path_buf())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        self.compiler.cmake_variables()
    }

    fn validate(&self) -> Result<()> {
        if !self.compiler.cc.exists() {
            bail!(
                "C compiler not found at: {}",
                self.compiler.cc.display()
            );
        }
        if !self.compiler.cxx.exists() {
            bail!(
                "C++ compiler not found at: {}",
                self.compiler.cxx.display()
            );
        }
        Ok(())
    }
}

/// Get compiler version from command output
fn get_compiler_version(compiler: &PathBuf) -> Option<String> {
    let output = std::process::Command::new(compiler)
        .arg("--version")
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout.lines().next().map(|s| s.to_string())
}
