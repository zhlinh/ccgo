//! Linux toolchain detection (GCC, Clang)
//!
//! Detects and configures GCC or Clang compilers for Linux builds.
//! Supports native x86_64 builds and cross-compilation to aarch64.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::{detect_default_compiler, find_executable, CompilerInfo, CompilerType, Toolchain};

/// Linux target architecture
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinuxArch {
    /// 64-bit x86 (native)
    X86_64,
    /// 64-bit ARM (cross-compilation via aarch64-linux-gnu-gcc)
    Aarch64,
}

impl LinuxArch {
    /// GNU cross-compiler prefix
    pub fn triple_prefix(self) -> &'static str {
        match self {
            LinuxArch::X86_64 => "x86_64-linux-gnu",
            LinuxArch::Aarch64 => "aarch64-linux-gnu",
        }
    }

    /// Canonical architecture string used in archive paths and CMake
    pub fn arch_string(self) -> &'static str {
        match self {
            LinuxArch::X86_64 => "x86_64",
            LinuxArch::Aarch64 => "aarch64",
        }
    }

    /// `CMAKE_SYSTEM_PROCESSOR` value for cross-compilation
    pub fn cmake_system_processor(self) -> &'static str {
        match self {
            LinuxArch::X86_64 => "x86_64",
            LinuxArch::Aarch64 => "aarch64",
        }
    }

    /// Parse from a user-supplied string (canonical or alias)
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "x86_64" | "x64" => Ok(LinuxArch::X86_64),
            "aarch64" | "arm64" => Ok(LinuxArch::Aarch64),
            other => bail!("Unknown Linux architecture '{}'. Supported: x86_64, aarch64", other),
        }
    }
}

/// Linux GCC/Clang toolchain
pub struct LinuxToolchain {
    compiler: CompilerInfo,
    /// Target architecture (affects CMake variables for cross-compilation)
    arch: LinuxArch,
}

impl LinuxToolchain {
    /// Detect native toolchain for the host architecture (x86_64)
    pub fn detect() -> Result<Self> {
        Self::detect_for_arch(LinuxArch::X86_64)
    }

    /// Detect toolchain for a specific target architecture.
    ///
    /// For `X86_64` on an x86_64 host: uses native GCC/Clang.
    /// For `Aarch64`: searches for `aarch64-linux-gnu-gcc` cross-compiler.
    pub fn detect_for_arch(arch: LinuxArch) -> Result<Self> {
        let compiler = match arch {
            LinuxArch::X86_64 => {
                detect_default_compiler()
                    .context("No C/C++ compiler found. Please install GCC or Clang.")?
            }
            LinuxArch::Aarch64 => {
                let prefix = arch.triple_prefix();
                let cc_name = format!("{}-gcc", prefix);
                let cxx_name = format!("{}-g++", prefix);
                let cc = find_executable(&cc_name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "aarch64 cross-compiler not found: {}\n\
                         Install it with: apt-get install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu",
                        cc_name
                    )
                })?;
                let cxx = find_executable(&cxx_name).ok_or_else(|| {
                    anyhow::anyhow!("aarch64 C++ cross-compiler not found: {}", cxx_name)
                })?;
                let version = super::get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
                CompilerInfo { compiler_type: CompilerType::Gcc, cc, cxx, version }
            }
        };
        Ok(Self { compiler, arch })
    }

    /// Prefer GCC (native x86_64)
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
                arch: LinuxArch::X86_64,
            });
        }
        Self::detect()
    }

    /// Prefer Clang (native x86_64)
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
                arch: LinuxArch::X86_64,
            });
        }
        Self::detect()
    }

    /// Get the compiler info
    pub fn compiler(&self) -> &CompilerInfo {
        &self.compiler
    }

    /// Get the target architecture
    pub fn arch(&self) -> LinuxArch {
        self.arch
    }

    /// Path to the `ar` archiver for this toolchain.
    ///
    /// For cross-compilation returns the prefixed archiver
    /// (e.g. `aarch64-linux-gnu-ar`); falls back to plain `ar`.
    pub fn ar_path(&self) -> PathBuf {
        match self.arch {
            LinuxArch::X86_64 => {
                find_executable("ar").unwrap_or_else(|| PathBuf::from("ar"))
            }
            LinuxArch::Aarch64 => {
                let ar_name = format!("{}-ar", self.arch.triple_prefix());
                find_executable(&ar_name).unwrap_or_else(|| PathBuf::from("ar"))
            }
        }
    }

    /// CMake variables for cross-compilation (empty for native x86_64 build).
    pub fn cross_cmake_variables(&self) -> Vec<(String, String)> {
        match self.arch {
            LinuxArch::X86_64 => vec![],
            LinuxArch::Aarch64 => {
                let prefix = self.arch.triple_prefix();
                vec![
                    ("CMAKE_SYSTEM_NAME".to_string(), "Linux".to_string()),
                    ("CMAKE_SYSTEM_PROCESSOR".to_string(), self.arch.cmake_system_processor().to_string()),
                    ("CMAKE_C_COMPILER".to_string(), self.compiler.cc.display().to_string()),
                    ("CMAKE_CXX_COMPILER".to_string(), self.compiler.cxx.display().to_string()),
                    ("CMAKE_FIND_ROOT_PATH".to_string(), format!("/usr/{}", prefix)),
                    ("CMAKE_FIND_ROOT_PATH_MODE_PROGRAM".to_string(), "NEVER".to_string()),
                    ("CMAKE_FIND_ROOT_PATH_MODE_LIBRARY".to_string(), "ONLY".to_string()),
                    ("CMAKE_FIND_ROOT_PATH_MODE_INCLUDE".to_string(), "ONLY".to_string()),
                ]
            }
        }
    }

    /// Merge multiple static libraries into a single library using the
    /// architecture-appropriate `ar`.
    pub fn merge_static_libs(&self, src_libs: &[PathBuf], dst_lib: &PathBuf) -> Result<()> {
        if src_libs.is_empty() {
            bail!("No source libraries to merge");
        }

        if let Some(parent) = dst_lib.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let temp_dir = std::env::temp_dir().join(format!(
            "ccgo-merge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        let ar_cmd = self.ar_path();

        for (idx, lib) in src_libs.iter().enumerate() {
            let extract_dir = temp_dir.join(format!("lib{}", idx));
            std::fs::create_dir_all(&extract_dir)?;

            let output = std::process::Command::new(&ar_cmd)
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

        if dst_lib.exists() {
            std::fs::remove_file(dst_lib)?;
        }

        let mut cmd = std::process::Command::new(&ar_cmd);
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
        // For cross-compilation, use cross_cmake_variables (includes system/processor/compiler).
        // For native builds, just pass compiler paths via CompilerInfo.
        match self.arch {
            LinuxArch::X86_64 => self.compiler.cmake_variables(),
            LinuxArch::Aarch64 => self.cross_cmake_variables(),
        }
    }

    fn validate(&self) -> Result<()> {
        if !self.compiler.cc.exists() {
            bail!("C compiler not found at: {}", self.compiler.cc.display());
        }
        if !self.compiler.cxx.exists() {
            bail!("C++ compiler not found at: {}", self.compiler.cxx.display());
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
