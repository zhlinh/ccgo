//! Android NDK toolchain detection
//!
//! Detects and configures the Android NDK for cross-compilation.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::Toolchain;

/// Android ABI targets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AndroidAbi {
    Arm64V8a,
    ArmeabiV7a,
    X86_64,
    X86,
}

impl AndroidAbi {
    /// Get the ABI string used by Android/CMake
    pub fn abi_string(&self) -> &str {
        match self {
            AndroidAbi::Arm64V8a => "arm64-v8a",
            AndroidAbi::ArmeabiV7a => "armeabi-v7a",
            AndroidAbi::X86_64 => "x86_64",
            AndroidAbi::X86 => "x86",
        }
    }

    /// Get the LLVM triple for this ABI
    pub fn llvm_triple(&self) -> &str {
        match self {
            AndroidAbi::Arm64V8a => "aarch64-linux-android",
            AndroidAbi::ArmeabiV7a => "armv7a-linux-androideabi",
            AndroidAbi::X86_64 => "x86_64-linux-android",
            AndroidAbi::X86 => "i686-linux-android",
        }
    }

    /// Get the STL library directory name (different from LLVM triple for ARMv7)
    pub fn stl_lib_dir(&self) -> &str {
        match self {
            AndroidAbi::Arm64V8a => "aarch64-linux-android",
            AndroidAbi::ArmeabiV7a => "arm-linux-androideabi",  // Special case!
            AndroidAbi::X86_64 => "x86_64-linux-android",
            AndroidAbi::X86 => "i686-linux-android",
        }
    }

    /// Get all supported ABIs
    pub fn all() -> Vec<AndroidAbi> {
        vec![
            AndroidAbi::Arm64V8a,
            AndroidAbi::ArmeabiV7a,
            AndroidAbi::X86_64,
            AndroidAbi::X86,
        ]
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "arm64-v8a" => Some(AndroidAbi::Arm64V8a),
            "armeabi-v7a" => Some(AndroidAbi::ArmeabiV7a),
            "x86_64" => Some(AndroidAbi::X86_64),
            "x86" => Some(AndroidAbi::X86),
            _ => None,
        }
    }
}

impl std::fmt::Display for AndroidAbi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.abi_string())
    }
}

/// Minimum API level for 64-bit ABIs
pub const MIN_API_LEVEL_64BIT: u32 = 21;

/// Default API level for new projects
pub const DEFAULT_API_LEVEL: u32 = 24;

/// Android NDK toolchain
pub struct AndroidNdkToolchain {
    /// Path to NDK root
    ndk_path: PathBuf,
    /// NDK version
    version: String,
    /// NDK revision (major.minor.patch)
    revision: (u32, u32, u32),
}

impl AndroidNdkToolchain {
    /// Detect Android NDK installation
    pub fn detect() -> Result<Self> {
        // Try multiple environment variables in order of preference
        let ndk_path = Self::find_ndk_path()?;

        // Parse version from source.properties
        let (version, revision) = Self::parse_ndk_version(&ndk_path)?;

        Ok(Self {
            ndk_path,
            version,
            revision,
        })
    }

    /// Find NDK path from environment variables
    fn find_ndk_path() -> Result<PathBuf> {
        // Check ANDROID_NDK_HOME first (recommended)
        if let Ok(path) = std::env::var("ANDROID_NDK_HOME") {
            let path = PathBuf::from(path);
            if Self::is_valid_ndk(&path) {
                return Ok(path);
            }
        }

        // Check ANDROID_NDK_ROOT
        if let Ok(path) = std::env::var("ANDROID_NDK_ROOT") {
            let path = PathBuf::from(path);
            if Self::is_valid_ndk(&path) {
                return Ok(path);
            }
        }

        // Check ANDROID_NDK
        if let Ok(path) = std::env::var("ANDROID_NDK") {
            let path = PathBuf::from(path);
            if Self::is_valid_ndk(&path) {
                return Ok(path);
            }
        }

        // Check ANDROID_HOME/ndk-bundle (older layout)
        if let Ok(android_home) = std::env::var("ANDROID_HOME") {
            let path = PathBuf::from(android_home).join("ndk-bundle");
            if Self::is_valid_ndk(&path) {
                return Ok(path);
            }
        }

        // Check ANDROID_SDK_ROOT/ndk/<version> (newer layout)
        if let Ok(sdk_root) = std::env::var("ANDROID_SDK_ROOT") {
            if let Some(ndk) = Self::find_ndk_in_sdk(&PathBuf::from(sdk_root)) {
                return Ok(ndk);
            }
        }

        // Check ANDROID_HOME/ndk/<version> (newer layout)
        if let Ok(android_home) = std::env::var("ANDROID_HOME") {
            if let Some(ndk) = Self::find_ndk_in_sdk(&PathBuf::from(android_home)) {
                return Ok(ndk);
            }
        }

        // Check common installation paths
        let common_paths = [
            "/opt/android-ndk",
            "/usr/local/android-ndk",
        ];

        for path in common_paths {
            let path = PathBuf::from(path);
            if Self::is_valid_ndk(&path) {
                return Ok(path);
            }
        }

        bail!(
            "Android NDK not found. Set one of these environment variables:\n\
             - ANDROID_NDK_HOME (recommended)\n\
             - ANDROID_NDK_ROOT\n\
             - ANDROID_NDK\n\
             Or install NDK via Android Studio SDK Manager."
        )
    }

    /// Find NDK in SDK's ndk/ directory (newer layout)
    fn find_ndk_in_sdk(sdk_path: &PathBuf) -> Option<PathBuf> {
        let ndk_dir = sdk_path.join("ndk");
        if !ndk_dir.exists() {
            return None;
        }

        // Find the highest version
        let mut versions: Vec<(PathBuf, (u32, u32, u32))> = Vec::new();

        if let Ok(entries) = fs::read_dir(&ndk_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(ver) = Self::parse_version_from_dirname(&path) {
                        if Self::is_valid_ndk(&path) {
                            versions.push((path, ver));
                        }
                    }
                }
            }
        }

        // Sort by version descending and return highest
        versions.sort_by(|a, b| b.1.cmp(&a.1));
        versions.first().map(|(p, _)| p.clone())
    }

    /// Parse version from directory name (e.g., "25.2.9519653")
    fn parse_version_from_dirname(path: &PathBuf) -> Option<(u32, u32, u32)> {
        let name = path.file_name()?.to_str()?;
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() >= 2 {
            let major = parts[0].parse().ok()?;
            let minor = parts[1].parse().ok()?;
            let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
            return Some((major, minor, patch));
        }
        None
    }

    /// Check if a path contains a valid NDK
    fn is_valid_ndk(path: &PathBuf) -> bool {
        if !path.exists() || !path.is_dir() {
            return false;
        }

        // Check for source.properties (present in all modern NDKs)
        let source_props = path.join("source.properties");
        if !source_props.exists() {
            return false;
        }

        // Check for CMake toolchain file
        let toolchain = path.join("build/cmake/android.toolchain.cmake");
        toolchain.exists()
    }

    /// Parse NDK version from source.properties
    fn parse_ndk_version(ndk_path: &PathBuf) -> Result<(String, (u32, u32, u32))> {
        let props_path = ndk_path.join("source.properties");
        let content = fs::read_to_string(&props_path)
            .with_context(|| format!("Failed to read {}", props_path.display()))?;

        let props = Self::parse_properties(&content);

        let version = props
            .get("Pkg.Revision")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let revision = Self::parse_revision(&version);

        Ok((version, revision))
    }

    /// Parse properties file format
    fn parse_properties(content: &str) -> HashMap<String, String> {
        let mut props = HashMap::new();
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                props.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
        props
    }

    /// Parse version string into (major, minor, patch)
    fn parse_revision(version: &str) -> (u32, u32, u32) {
        let parts: Vec<&str> = version.split('.').collect();
        let major = parts.first().and_then(|p| p.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    }

    /// Get the CMake toolchain file path
    pub fn cmake_toolchain_file(&self) -> PathBuf {
        self.ndk_path.join("build/cmake/android.toolchain.cmake")
    }

    /// Get NDK version string
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get NDK major version
    pub fn major_version(&self) -> u32 {
        self.revision.0
    }

    /// Get CMake variables for a specific ABI and API level
    pub fn cmake_variables_for_abi(&self, abi: AndroidAbi, api_level: u32) -> Vec<(String, String)> {
        let host_tag = Self::host_tag();
        let toolchain_prefix = self.ndk_path
            .join("toolchains/llvm/prebuilt")
            .join(host_tag);

        // Get compiler paths
        let clang = toolchain_prefix.join("bin/clang");
        let clangxx = toolchain_prefix.join("bin/clang++");

        let mut vars = vec![
            ("ANDROID_NDK".to_string(), self.ndk_path.display().to_string()),
            (
                "CMAKE_TOOLCHAIN_FILE".to_string(),
                self.cmake_toolchain_file().display().to_string(),
            ),
            ("ANDROID_ABI".to_string(), abi.abi_string().to_string()),
            ("ANDROID_PLATFORM".to_string(), format!("android-{}", api_level)),
            ("ANDROID_STL".to_string(), "c++_shared".to_string()),
            // Explicitly set compilers to avoid detection issues
            ("CMAKE_C_COMPILER".to_string(), clang.display().to_string()),
            ("CMAKE_CXX_COMPILER".to_string(), clangxx.display().to_string()),
        ];

        // For newer NDKs (r23+), use the unified headers
        if self.major_version() >= 23 {
            vars.push(("ANDROID_USE_LEGACY_TOOLCHAIN_FILE".to_string(), "OFF".to_string()));
        }

        vars
    }

    /// Get the path to a toolchain binary (e.g., clang, ar)
    pub fn toolchain_bin(&self, tool: &str) -> PathBuf {
        let host_tag = Self::host_tag();
        self.ndk_path
            .join("toolchains/llvm/prebuilt")
            .join(host_tag)
            .join("bin")
            .join(tool)
    }

    /// Get the host tag for the current platform
    fn host_tag() -> &'static str {
        #[cfg(target_os = "linux")]
        return "linux-x86_64";

        #[cfg(target_os = "macos")]
        return "darwin-x86_64";

        #[cfg(target_os = "windows")]
        return "windows-x86_64";

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return "linux-x86_64";
    }

    /// Get the minimum API level for an ABI
    pub fn min_api_level(abi: AndroidAbi) -> u32 {
        match abi {
            AndroidAbi::Arm64V8a | AndroidAbi::X86_64 => MIN_API_LEVEL_64BIT,
            AndroidAbi::ArmeabiV7a | AndroidAbi::X86 => 16,
        }
    }

    /// Validate an API level for an ABI
    pub fn validate_api_level(abi: AndroidAbi, api_level: u32) -> Result<()> {
        let min = Self::min_api_level(abi);
        if api_level < min {
            bail!(
                "API level {} is too low for {}: minimum is {}",
                api_level, abi, min
            );
        }
        Ok(())
    }

    /// Get the path to llvm-strip binary
    pub fn llvm_strip_path(&self) -> PathBuf {
        self.toolchain_bin("llvm-strip")
    }

    /// Get the path to libc++_shared.so for the specified ABI
    pub fn stl_library_path(&self, abi: AndroidAbi) -> PathBuf {
        let host_tag = Self::host_tag();
        self.ndk_path
            .join("toolchains/llvm/prebuilt")
            .join(host_tag)
            .join("sysroot/usr/lib")
            .join(abi.stl_lib_dir())  // Use stl_lib_dir() instead of llvm_triple()
            .join("libc++_shared.so")
    }

    /// Strip debug symbols from a shared library
    ///
    /// Uses llvm-strip with --strip-unneeded to remove debug symbols
    /// while preserving the symbol table needed for dynamic linking.
    pub fn strip_library(&self, library_path: &PathBuf, verbose: bool) -> Result<()> {
        let strip_path = self.llvm_strip_path();

        if !strip_path.exists() {
            bail!("llvm-strip not found at: {}", strip_path.display());
        }

        if verbose {
            eprintln!("  Stripping {}...", library_path.display());
        }

        let status = std::process::Command::new(&strip_path)
            .arg("--strip-unneeded")
            .arg(library_path)
            .status()
            .with_context(|| format!("Failed to run llvm-strip on {}", library_path.display()))?;

        if !status.success() {
            bail!("llvm-strip failed for {}", library_path.display());
        }

        Ok(())
    }

    /// Copy STL library (libc++_shared.so) to destination directory
    pub fn copy_stl_library(&self, abi: AndroidAbi, dest_dir: &PathBuf) -> Result<PathBuf> {
        let stl_path = self.stl_library_path(abi);

        if !stl_path.exists() {
            bail!(
                "STL library not found at: {}\nMake sure your NDK installation is complete.",
                stl_path.display()
            );
        }

        std::fs::create_dir_all(dest_dir)?;
        let dest_path = dest_dir.join("libc++_shared.so");
        std::fs::copy(&stl_path, &dest_path).with_context(|| {
            format!(
                "Failed to copy {} to {}",
                stl_path.display(),
                dest_path.display()
            )
        })?;

        Ok(dest_path)
    }
}

impl Toolchain for AndroidNdkToolchain {
    fn name(&self) -> &str {
        "android-ndk"
    }

    fn is_available(&self) -> bool {
        self.ndk_path.exists() && self.cmake_toolchain_file().exists()
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.ndk_path.clone())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        // Default to arm64-v8a with API 24
        self.cmake_variables_for_abi(AndroidAbi::Arm64V8a, DEFAULT_API_LEVEL)
    }

    fn validate(&self) -> Result<()> {
        // Check NDK path exists
        if !self.ndk_path.exists() {
            bail!("Android NDK path does not exist: {}", self.ndk_path.display());
        }

        // Check toolchain file exists
        let toolchain = self.cmake_toolchain_file();
        if !toolchain.exists() {
            bail!("CMake toolchain file not found: {}", toolchain.display());
        }

        // Check for clang
        let clang = self.toolchain_bin("clang");
        if !clang.exists() {
            bail!("NDK clang not found: {}", clang.display());
        }

        // Recommend minimum version
        if self.major_version() < 21 {
            eprintln!(
                "Warning: NDK version {} is old. Consider upgrading to NDK 25 or later.",
                self.version
            );
        }

        Ok(())
    }
}

/// Check if Android NDK is available
pub fn is_android_ndk_available() -> bool {
    AndroidNdkToolchain::detect().is_ok()
}
