//! MinGW-w64 toolchain detection for Windows cross-compilation
//!
//! Detects and configures MinGW-w64 for cross-compiling Windows binaries
//! from Linux or macOS.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Result};

use super::{find_executable, Toolchain};

/// MinGW-w64 target architecture
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MingwArch {
    /// 64-bit x86 (x86_64)
    X86_64,
    /// 32-bit x86 (i686)
    I686,
}

impl MingwArch {
    /// Get the target triple prefix
    pub fn triple_prefix(&self) -> &str {
        match self {
            MingwArch::X86_64 => "x86_64-w64-mingw32",
            MingwArch::I686 => "i686-w64-mingw32",
        }
    }

    /// Get the architecture string
    pub fn arch_string(&self) -> &str {
        match self {
            MingwArch::X86_64 => "x86_64",
            MingwArch::I686 => "i686",
        }
    }
}

/// MinGW-w64 toolchain for Windows cross-compilation
pub struct MingwToolchain {
    /// Path to gcc compiler
    gcc_path: PathBuf,
    /// Path to g++ compiler
    gxx_path: PathBuf,
    /// Path to windres (resource compiler)
    windres_path: Option<PathBuf>,
    /// Target architecture
    arch: MingwArch,
    /// GCC version
    version: String,
}

impl MingwToolchain {
    /// Detect MinGW-w64 installation for the default (x86_64) architecture
    pub fn detect() -> Result<Self> {
        Self::detect_for_arch(MingwArch::X86_64)
    }

    /// Detect MinGW-w64 installation for a specific architecture
    pub fn detect_for_arch(arch: MingwArch) -> Result<Self> {
        let prefix = arch.triple_prefix();

        // Look for the gcc compiler
        let gcc_name = format!("{}-gcc", prefix);
        let gcc_path = find_executable(&gcc_name).ok_or_else(|| {
            anyhow::anyhow!(
                "MinGW-w64 not found. Please install MinGW-w64 and ensure {} is in PATH.",
                gcc_name
            )
        })?;

        // Look for g++ compiler
        let gxx_name = format!("{}-g++", prefix);
        let gxx_path = find_executable(&gxx_name).ok_or_else(|| {
            anyhow::anyhow!("MinGW-w64 C++ compiler not found: {}", gxx_name)
        })?;

        // Look for windres (optional)
        let windres_name = format!("{}-windres", prefix);
        let windres_path = find_executable(&windres_name);

        // Get version
        let version = Self::detect_version(&gcc_path)?;

        Ok(Self {
            gcc_path,
            gxx_path,
            windres_path,
            arch,
            version,
        })
    }

    /// Detect GCC version
    fn detect_version(gcc_path: &PathBuf) -> Result<String> {
        let output = Command::new(gcc_path)
            .arg("--version")
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run {}: {}", gcc_path.display(), e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // First line typically contains version info
        let first_line = stdout.lines().next().unwrap_or("unknown");

        // Try to extract version number
        if let Some(version_start) = first_line.find(|c: char| c.is_ascii_digit()) {
            let version_part: String = first_line[version_start..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect();
            if !version_part.is_empty() {
                return Ok(version_part);
            }
        }

        Ok(first_line.to_string())
    }

    /// Get the GCC compiler path
    pub fn gcc(&self) -> &PathBuf {
        &self.gcc_path
    }

    /// Get the G++ compiler path
    pub fn gxx(&self) -> &PathBuf {
        &self.gxx_path
    }

    /// Get the windres path (if available)
    pub fn windres(&self) -> Option<&PathBuf> {
        self.windres_path.as_ref()
    }

    /// Get the target architecture
    pub fn arch(&self) -> MingwArch {
        self.arch
    }

    /// Get the GCC version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get CMake variables for cross-compilation
    pub fn cmake_variables_for_arch(&self) -> Vec<(String, String)> {
        let prefix = self.arch.triple_prefix();

        let mut vars = vec![
            ("CMAKE_SYSTEM_NAME".to_string(), "Windows".to_string()),
            (
                "CMAKE_C_COMPILER".to_string(),
                self.gcc_path.display().to_string(),
            ),
            (
                "CMAKE_CXX_COMPILER".to_string(),
                self.gxx_path.display().to_string(),
            ),
            (
                "CMAKE_FIND_ROOT_PATH".to_string(),
                format!("/usr/{}", prefix),
            ),
            (
                "CMAKE_FIND_ROOT_PATH_MODE_PROGRAM".to_string(),
                "NEVER".to_string(),
            ),
            (
                "CMAKE_FIND_ROOT_PATH_MODE_LIBRARY".to_string(),
                "ONLY".to_string(),
            ),
            (
                "CMAKE_FIND_ROOT_PATH_MODE_INCLUDE".to_string(),
                "ONLY".to_string(),
            ),
        ];

        // Add windres if available
        if let Some(windres) = &self.windres_path {
            vars.push((
                "CMAKE_RC_COMPILER".to_string(),
                windres.display().to_string(),
            ));
        }

        vars
    }

    /// Get the strip command path
    pub fn strip_path(&self) -> PathBuf {
        let strip_name = format!("{}-strip", self.arch.triple_prefix());
        find_executable(&strip_name).unwrap_or_else(|| PathBuf::from(&strip_name))
    }

    /// Get the ar (archiver) path
    pub fn ar_path(&self) -> PathBuf {
        let ar_name = format!("{}-ar", self.arch.triple_prefix());
        find_executable(&ar_name).unwrap_or_else(|| PathBuf::from(&ar_name))
    }

    /// Merge multiple static libraries into a single library using ar
    /// This is essential for KMP cinterop which expects a single complete library
    pub fn merge_static_libs(&self, src_libs: &[PathBuf], dst_lib: &PathBuf) -> Result<()> {
        use anyhow::Context;

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

        let ar_cmd = self.ar_path();

        // Extract all object files from source libraries
        for (idx, lib) in src_libs.iter().enumerate() {
            let extract_dir = temp_dir.join(format!("lib{}", idx));
            std::fs::create_dir_all(&extract_dir)?;

            // Extract objects: ar x libname.a
            let output = Command::new(&ar_cmd)
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
        let mut cmd = Command::new(&ar_cmd);
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

impl Toolchain for MingwToolchain {
    fn name(&self) -> &str {
        "mingw-w64"
    }

    fn is_available(&self) -> bool {
        self.gcc_path.exists() && self.gxx_path.exists()
    }

    fn path(&self) -> Option<PathBuf> {
        self.gcc_path.parent().map(|p| p.to_path_buf())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        self.cmake_variables_for_arch()
    }

    fn validate(&self) -> Result<()> {
        // Check gcc exists
        if !self.gcc_path.exists() {
            bail!(
                "MinGW GCC not found at: {}",
                self.gcc_path.display()
            );
        }

        // Check g++ exists
        if !self.gxx_path.exists() {
            bail!(
                "MinGW G++ not found at: {}",
                self.gxx_path.display()
            );
        }

        // Test compilation
        let test_result = Command::new(&self.gcc_path)
            .args(["--version"])
            .output();

        if test_result.is_err() {
            bail!("MinGW GCC failed to execute");
        }

        Ok(())
    }
}

/// Check if MinGW-w64 is available
pub fn is_mingw_available() -> bool {
    MingwToolchain::detect().is_ok()
}
