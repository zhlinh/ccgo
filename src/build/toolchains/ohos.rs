//! OpenHarmony (OHOS) SDK toolchain detection
//!
//! Detects OHOS Native SDK installation and provides CMake configuration
//! for building native libraries targeting OpenHarmony OS.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::Toolchain;

/// Default minimum SDK version for OHOS builds
pub const DEFAULT_MIN_SDK_VERSION: u32 = 10;

/// OHOS ABI (Application Binary Interface)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OhosAbi {
    /// ARM 64-bit
    Arm64V8a,
    /// ARM 32-bit
    ArmeabiV7a,
    /// x86 64-bit (for emulator)
    X86_64,
}

impl OhosAbi {
    /// Get the ABI string used in directory names and CMake
    pub fn abi_string(&self) -> &str {
        match self {
            OhosAbi::Arm64V8a => "arm64-v8a",
            OhosAbi::ArmeabiV7a => "armeabi-v7a",
            OhosAbi::X86_64 => "x86_64",
        }
    }

    /// Get the LLVM triple for this ABI
    pub fn llvm_triple(&self) -> &str {
        match self {
            OhosAbi::Arm64V8a => "aarch64-linux-ohos",
            OhosAbi::ArmeabiV7a => "arm-linux-ohos",
            OhosAbi::X86_64 => "x86_64-linux-ohos",
        }
    }

    /// Parse ABI from string. Accepts canonical ABI names (`arm64-v8a`,
    /// `armeabi-v7a`, `x86_64`) plus shorthand aliases:
    /// * `v8`, `a64`, `arm64`, `armv8`, `aarch64` → `arm64-v8a`
    /// * `v7`, `a32`, `arm32`, `armv7`, `aarch32` → `armeabi-v7a`
    /// * `x64`                                    → `x86_64`
    ///
    /// Matching is case-insensitive. Bare `arm` is intentionally NOT
    /// accepted — it is ambiguous between 32-bit and 64-bit ARM.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "arm64-v8a" | "v8" | "a64" | "arm64" | "armv8" | "aarch64" => Some(OhosAbi::Arm64V8a),
            "armeabi-v7a" | "v7" | "a32" | "arm32" | "armv7" | "aarch32" => {
                Some(OhosAbi::ArmeabiV7a)
            }
            "x86_64" | "x64" => Some(OhosAbi::X86_64),
            _ => None,
        }
    }

    /// Get all supported ABIs
    pub fn all() -> Vec<OhosAbi> {
        vec![OhosAbi::Arm64V8a, OhosAbi::ArmeabiV7a, OhosAbi::X86_64]
    }
}

impl std::fmt::Display for OhosAbi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.abi_string())
    }
}

/// OHOS SDK toolchain
pub struct OhosSdkToolchain {
    /// Path to OHOS SDK root
    sdk_path: PathBuf,
    /// SDK version string
    version: String,
}

impl OhosSdkToolchain {
    /// Detect OHOS SDK from environment variables
    ///
    /// Searches in order:
    /// 1. OHOS_SDK_HOME environment variable
    /// 2. HOS_SDK_HOME environment variable
    pub fn detect() -> Result<Self> {
        // Try OHOS_SDK_HOME first, then HOS_SDK_HOME
        let sdk_path = std::env::var("OHOS_SDK_HOME")
            .or_else(|_| std::env::var("HOS_SDK_HOME"))
            .ok()
            .map(PathBuf::from)
            .and_then(|p| if p.exists() { Some(p) } else { None });

        let sdk_path = sdk_path.ok_or_else(|| {
            anyhow::anyhow!(
                "OHOS SDK not found. Please set OHOS_SDK_HOME or HOS_SDK_HOME environment variable."
            )
        })?;

        // Check for native directory
        let native_dir = sdk_path.join("native");
        if !native_dir.exists() {
            bail!(
                "OHOS SDK native directory not found at: {}",
                native_dir.display()
            );
        }

        // Try to detect version from native directory
        let version = Self::detect_version(&sdk_path).unwrap_or_else(|| "unknown".to_string());

        Ok(Self { sdk_path, version })
    }

    /// Detect SDK version from OHOS SDK
    fn detect_version(sdk_path: &PathBuf) -> Option<String> {
        // Try to read version from native/version.txt if available
        let version_file = sdk_path.join("native").join("version.txt");
        if version_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&version_file) {
                return Some(content.trim().to_string());
            }
        }

        // Try oh-uni-package.json for version info
        let package_json = sdk_path.join("native").join("oh-uni-package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                // Simple JSON parsing for version field
                if let Some(start) = content.find("\"version\"") {
                    let rest = &content[start..];
                    if let Some(colon) = rest.find(':') {
                        let value_part = &rest[colon + 1..];
                        if let Some(quote_start) = value_part.find('"') {
                            let after_quote = &value_part[quote_start + 1..];
                            if let Some(quote_end) = after_quote.find('"') {
                                return Some(after_quote[..quote_end].to_string());
                            }
                        }
                    }
                }
            }
        }

        // Try to detect from directory structure
        // OHOS SDK typically has version in the path
        if let Some(name) = sdk_path.file_name() {
            let name_str = name.to_string_lossy();
            // Extract version number if present (e.g., "ohos-sdk-4.0" -> "4.0")
            if let Some(version_part) = name_str.split('-').next_back() {
                if version_part
                    .chars()
                    .next()
                    .map(|c| c.is_numeric())
                    .unwrap_or(false)
                {
                    return Some(version_part.to_string());
                }
            }
        }

        None
    }

    /// Get the SDK path
    pub fn sdk_path(&self) -> &PathBuf {
        &self.sdk_path
    }

    /// Get the native SDK path
    pub fn native_path(&self) -> PathBuf {
        self.sdk_path.join("native")
    }

    /// Get the CMake toolchain file path
    pub fn toolchain_file(&self) -> PathBuf {
        self.native_path()
            .join("build")
            .join("cmake")
            .join("ohos.toolchain.cmake")
    }

    /// Get the path to llvm-strip
    pub fn strip_path(&self) -> PathBuf {
        self.native_path()
            .join("llvm")
            .join("bin")
            .join("llvm-strip")
    }

    /// Get the path to libc++_shared.so for a specific ABI
    pub fn stl_path(&self, abi: OhosAbi) -> PathBuf {
        self.native_path()
            .join("llvm")
            .join("lib")
            .join(abi.llvm_triple())
            .join("libc++_shared.so")
    }

    /// Get CMake variables for building with this SDK
    pub fn cmake_variables_for_abi(
        &self,
        abi: OhosAbi,
        min_sdk_version: u32,
    ) -> Vec<(String, String)> {
        let native_path = self.native_path();
        let toolchain_file = self.toolchain_file();

        vec![
            ("OHOS".to_string(), "1".to_string()),
            ("__OHOS__".to_string(), "1".to_string()),
            ("OHOS_ARCH".to_string(), abi.abi_string().to_string()),
            ("OHOS_PLATFORM".to_string(), "OHOS".to_string()),
            (
                "CMAKE_TOOLCHAIN_FILE".to_string(),
                toolchain_file.display().to_string(),
            ),
            ("OHOS_TOOLCHAIN".to_string(), "clang".to_string()),
            (
                "OHOS_SDK_NATIVE".to_string(),
                format!("{}/", native_path.display()),
            ),
            (
                "OHOS_SDK_NATIVE_PLATFORM".to_string(),
                format!("ohos-{}", min_sdk_version),
            ),
            ("OHOS_STL".to_string(), "c++_shared".to_string()),
        ]
    }

    /// Get the path to llvm-ar
    pub fn llvm_ar_path(&self) -> PathBuf {
        self.native_path().join("llvm").join("bin").join("llvm-ar")
    }

    /// Merge multiple static libraries into a single library using llvm-ar
    pub fn merge_static_libs(&self, src_libs: &[PathBuf], dst_lib: &PathBuf) -> Result<()> {
        if src_libs.is_empty() {
            anyhow::bail!("No source libraries to merge");
        }

        if let Some(parent) = dst_lib.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let ar = self.llvm_ar_path();
        let temp_dir = std::env::temp_dir().join(format!(
            "ccgo-ohos-merge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        for (idx, lib) in src_libs.iter().enumerate() {
            let extract_dir = temp_dir.join(format!("lib{}", idx));
            std::fs::create_dir_all(&extract_dir)?;

            let output = std::process::Command::new(&ar)
                .arg("x")
                .arg(lib)
                .current_dir(&extract_dir)
                .output()
                .context("Failed to run llvm-ar for extraction")?;

            if !output.status.success() {
                std::fs::remove_dir_all(&temp_dir).ok();
                bail!(
                    "llvm-ar extraction failed for {}: {}",
                    lib.display(),
                    String::from_utf8_lossy(&output.stderr)
                );
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

        let mut cmd = std::process::Command::new(&ar);
        cmd.arg("rcs").arg(dst_lib);
        for obj in &object_files {
            cmd.arg(obj);
        }

        let output = cmd.output().context("Failed to run llvm-ar for merging")?;
        std::fs::remove_dir_all(&temp_dir).ok();

        if !output.status.success() {
            bail!(
                "llvm-ar merge failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }

        Ok(())
    }

    /// Get SDK version string
    pub fn version(&self) -> &str {
        &self.version
    }
}

impl Toolchain for OhosSdkToolchain {
    fn name(&self) -> &str {
        "ohos-sdk"
    }

    fn is_available(&self) -> bool {
        self.sdk_path.exists() && self.toolchain_file().exists()
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.sdk_path.clone())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        // Default to arm64-v8a with default SDK version
        self.cmake_variables_for_abi(OhosAbi::Arm64V8a, DEFAULT_MIN_SDK_VERSION)
    }

    fn validate(&self) -> Result<()> {
        // Check SDK path exists
        if !self.sdk_path.exists() {
            bail!("OHOS SDK path does not exist: {}", self.sdk_path.display());
        }

        // Check native directory
        let native_path = self.native_path();
        if !native_path.exists() {
            bail!(
                "OHOS SDK native directory not found: {}",
                native_path.display()
            );
        }

        // Check toolchain file
        let toolchain_file = self.toolchain_file();
        if !toolchain_file.exists() {
            bail!(
                "OHOS toolchain file not found: {}",
                toolchain_file.display()
            );
        }

        // Check llvm-strip
        let strip_path = self.strip_path();
        if !strip_path.exists() {
            bail!("OHOS llvm-strip not found: {}", strip_path.display());
        }

        Ok(())
    }
}

/// Check if OHOS SDK is available
pub fn is_ohos_sdk_available() -> bool {
    OhosSdkToolchain::detect().is_ok()
}
