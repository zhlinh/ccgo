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

    /// Merge multiple static libraries into a single library using ar
    /// This is essential for KMP cinterop which expects a single complete library
    pub fn merge_static_libs(&self, src_libs: &[PathBuf], dst_lib: &PathBuf) -> Result<()> {
        if src_libs.is_empty() {
            bail!("No source libraries to merge");
        }

        // Ensure output directory exists
        if let Some(parent) = dst_lib.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Create a temporary directory for extracting object files
        let temp_dir = std::env::temp_dir().join(format!(
            "ccgo-merge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        // Extract all object files from source libraries
        for (idx, lib) in src_libs.iter().enumerate() {
            let extract_dir = temp_dir.join(format!("lib{}", idx));
            std::fs::create_dir_all(&extract_dir)?;

            // Extract objects: ar x libname.a
            let output = std::process::Command::new("ar")
                .arg("x")
                .arg(lib)
                .current_dir(&extract_dir)
                .output()
                .context("Failed to run ar for extraction")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                std::fs::remove_dir_all(&temp_dir).ok();
                bail!("ar extraction failed for {}: {}", lib.display(), stderr);
            }
        }

        // Collect all object files (.o or .obj) from all extraction directories
        let mut object_files: Vec<PathBuf> = Vec::new();
        for entry in walkdir::WalkDir::new(&temp_dir) {
            let entry = entry?;
            let ext = entry.path().extension().and_then(|e| e.to_str());
            if matches!(ext, Some("o") | Some("obj")) {
                object_files.push(entry.path().to_path_buf());
            }
        }

        if object_files.is_empty() {
            std::fs::remove_dir_all(&temp_dir).ok();
            bail!("No object files found in source libraries");
        }

        // Remove existing output library if it exists
        if dst_lib.exists() {
            std::fs::remove_file(dst_lib)?;
        }

        // Create the merged library: ar rcs output.a obj1.o obj2.o ...
        let mut cmd = std::process::Command::new("ar");
        cmd.arg("rcs").arg(dst_lib);
        for obj in &object_files {
            cmd.arg(obj);
        }

        let output = cmd.output().context("Failed to run ar for merging")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            std::fs::remove_dir_all(&temp_dir).ok();
            bail!("ar merge failed: {}", stderr);
        }

        // Clean up temporary directory
        std::fs::remove_dir_all(&temp_dir).ok();

        Ok(())
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
