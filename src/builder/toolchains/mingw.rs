//! MinGW-w64 toolchain detection for Windows cross-compilation
//!
//! Detects and configures MinGW-w64 for cross-compiling Windows binaries
//! from Linux or macOS.

use std::path::{Path, PathBuf};
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
    /// 64-bit ARM (aarch64-w64-mingw32, requires llvm-mingw or similar)
    Aarch64,
}

impl MingwArch {
    /// Get the target triple prefix
    pub fn triple_prefix(&self) -> &str {
        match self {
            MingwArch::X86_64 => "x86_64-w64-mingw32",
            MingwArch::I686 => "i686-w64-mingw32",
            MingwArch::Aarch64 => "aarch64-w64-mingw32",
        }
    }

    /// Get the architecture string
    pub fn arch_string(&self) -> &str {
        match self {
            MingwArch::X86_64 => "x86_64",
            MingwArch::I686 => "i686",
            MingwArch::Aarch64 => "aarch64",
        }
    }

    /// Parse from a user-supplied string
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "x86_64" | "x64" => Ok(MingwArch::X86_64),
            "i686" | "x86" | "i386" => Ok(MingwArch::I686),
            "aarch64" | "arm64" => Ok(MingwArch::Aarch64),
            other => bail!(
                "Unknown MinGW architecture '{}'. Supported: x86_64, i686, aarch64",
                other
            ),
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

    /// Detect MinGW-w64 installation for a specific architecture.
    ///
    /// For x86_64/i686: searches standard GCC-based toolchain, then llvm-mingw clang as fallback.
    /// For aarch64: standard mingw-w64 has no GCC cross-compiler; searches llvm-mingw clang first
    ///              (llvm-mingw provides `aarch64-w64-mingw32-clang`).
    pub fn detect_for_arch(arch: MingwArch) -> Result<Self> {
        let prefix = arch.triple_prefix();

        // Search order for C compiler:
        //   1. {prefix}-gcc-posix  (standard mingw-w64, posix threads)
        //   2. {prefix}-gcc        (standard mingw-w64)
        //   3. {prefix}-clang      (llvm-mingw — primary compiler for aarch64)
        let gcc_posix = format!("{}-gcc-posix", prefix);
        let gcc_name = format!("{}-gcc", prefix);
        let clang_name = format!("{}-clang", prefix);
        let gcc_path = find_executable(&gcc_posix)
            .or_else(|| find_executable(&gcc_name))
            .or_else(|| find_executable(&clang_name))
            .ok_or_else(|| {
                let install_hint = if arch == MingwArch::Aarch64 {
                    "Install llvm-mingw: https://github.com/mstorsjo/llvm-mingw/releases\n  \
                     Then add its bin/ directory to PATH."
                } else {
                    "Install MinGW-w64: apt-get install mingw-w64  (or equivalent)"
                };
                anyhow::anyhow!(
                    "MinGW cross-compiler not found for target {}.\n  {}\n  \
                     Looked for: {}, {}, {}",
                    prefix,
                    install_hint,
                    gcc_posix,
                    gcc_name,
                    clang_name,
                )
            })?;

        // Search order for C++ compiler:
        //   1. {prefix}-g++-posix  (standard mingw-w64, posix threads)
        //   2. {prefix}-g++        (standard mingw-w64)
        //   3. {prefix}-clang++    (llvm-mingw)
        let gxx_posix = format!("{}-g++-posix", prefix);
        let gxx_name = format!("{}-g++", prefix);
        let clangxx_name = format!("{}-clang++", prefix);
        let gxx_path = find_executable(&gxx_posix)
            .or_else(|| find_executable(&gxx_name))
            .or_else(|| find_executable(&clangxx_name))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "MinGW C++ compiler not found for target {}.\n  \
                     Looked for: {}, {}, {}",
                    prefix,
                    gxx_posix,
                    gxx_name,
                    clangxx_name,
                )
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
            ("CMAKE_C_COMPILER".to_string(), cmake_path(&self.gcc_path)),
            ("CMAKE_CXX_COMPILER".to_string(), cmake_path(&self.gxx_path)),
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
            vars.push(("CMAKE_RC_COMPILER".to_string(), cmake_path(windres)));
        }

        vars
    }

    /// Get the strip command path
    pub fn strip_path(&self) -> PathBuf {
        self.binutil_path("strip")
    }

    /// Get the ar (archiver) path
    pub fn ar_path(&self) -> PathBuf {
        self.binutil_path("ar")
    }

    fn binutil_path(&self, tool: &str) -> PathBuf {
        let name = format!("{}-{tool}", self.arch.triple_prefix());
        // Prefer tools from the selected compiler, including MSYS2's plain names.
        sibling_binutil(&self.gcc_path, &name, tool)
            .or_else(|| find_executable(&name))
            .unwrap_or_else(|| PathBuf::from(name))
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

        extract_static_libs(&ar_cmd, src_libs, &temp_dir)?;

        // Collect all object files (.o or .obj) from all extraction directories
        let object_files = collect_object_files(&temp_dir)?;

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

fn extract_static_libs(ar_cmd: &Path, src_libs: &[PathBuf], temp_dir: &Path) -> Result<()> {
    use anyhow::Context;

    // Extract all object files from source libraries
    for (idx, lib) in src_libs.iter().enumerate() {
        let extract_dir = temp_dir.join(format!("lib{}", idx));
        std::fs::create_dir_all(&extract_dir)?;

        // Extract objects: ar x libname.a
        let output = Command::new(ar_cmd)
            .arg("x")
            .arg(lib)
            .current_dir(&extract_dir)
            .output()
            .context("Failed to run ar for extraction")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            std::fs::remove_dir_all(temp_dir).ok();
            bail!("ar extraction failed for {}: {}", lib.display(), stderr);
        }
    }

    Ok(())
}

fn collect_object_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut objects = Vec::new();
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        let ext = entry.path().extension().and_then(|ext| ext.to_str());
        if matches!(ext, Some("o") | Some("obj")) {
            objects.push(entry.path().to_path_buf());
        }
    }
    Ok(objects)
}

fn cmake_path(path: &Path) -> String {
    // CMake persists compiler paths in scripts where backslashes become escapes.
    path.to_string_lossy().replace('\\', "/")
}

fn sibling_binutil(compiler: &Path, prefixed: &str, plain: &str) -> Option<PathBuf> {
    let directory = compiler.parent()?;
    let suffix = if compiler.extension().is_some_and(|ext| ext == "exe") {
        ".exe"
    } else {
        ""
    };
    [prefixed, plain]
        .into_iter()
        // Unix cross-compilers often share /usr/bin with unrelated host tools.
        .filter(|name| *name == prefixed || suffix == ".exe")
        .map(|name| directory.join(format!("{name}{suffix}")))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiler_paths_are_safe_for_cmake_scripts() {
        let toolchain = MingwToolchain {
            gcc_path: PathBuf::from(r"C:\Program Files\msys64\ucrt64\bin\gcc.exe"),
            gxx_path: PathBuf::from(r"C:\msys64/bin\g++.exe"),
            windres_path: Some(PathBuf::from(r"C:\msys64\bin\windres.exe")),
            arch: MingwArch::X86_64,
            version: String::new(),
        };
        let variables = toolchain.cmake_variables_for_arch();
        for name in [
            "CMAKE_C_COMPILER",
            "CMAKE_CXX_COMPILER",
            "CMAKE_RC_COMPILER",
        ] {
            let value = &variables.iter().find(|(key, _)| key == name).unwrap().1;
            assert!(!value.contains('\\'), "{name}: {value}");
        }
        assert!(variables
            .iter()
            .any(|(_, value)| value == "C:/Program Files/msys64/ucrt64/bin/gcc.exe"));
    }

    #[test]
    fn binutils_prefer_compiler_siblings_and_support_plain_exe_names() {
        let dir = tempfile::tempdir().unwrap();
        let compiler = dir.path().join("gcc.exe");
        let prefixed = "x86_64-w64-mingw32-ar";
        assert_eq!(sibling_binutil(&compiler, prefixed, "ar"), None);
        let plain = dir.path().join("ar.exe");
        std::fs::write(&plain, "").unwrap();
        assert_eq!(sibling_binutil(&compiler, prefixed, "ar"), Some(plain));
        let target = dir.path().join(format!("{prefixed}.exe"));
        std::fs::write(&target, "").unwrap();
        assert_eq!(sibling_binutil(&compiler, prefixed, "ar"), Some(target));
    }

    #[test]
    fn unix_cross_compilers_do_not_fall_back_to_host_binutils() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ar"), "").unwrap();
        assert_eq!(
            sibling_binutil(
                &dir.path().join("x86_64-w64-mingw32-gcc"),
                "x86_64-w64-mingw32-ar",
                "ar",
            ),
            None
        );
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
            bail!("MinGW GCC not found at: {}", self.gcc_path.display());
        }

        // Check g++ exists
        if !self.gxx_path.exists() {
            bail!("MinGW G++ not found at: {}", self.gxx_path.display());
        }

        // Test compilation
        let test_result = Command::new(&self.gcc_path).args(["--version"]).output();

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
