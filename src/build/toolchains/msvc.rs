//! MSVC toolchain detection for Windows
//!
//! Detects and configures Microsoft Visual C++ for Windows builds.
//! Note: Native MSVC builds are only available on Windows.

use std::path::PathBuf;
#[cfg(target_os = "windows")]
use std::process::Command;

use anyhow::{bail, Result};

use super::Toolchain;

/// MSVC toolchain for Windows builds
pub struct MsvcToolchain {
    /// Visual Studio installation path
    vs_path: PathBuf,
    /// MSVC version
    version: String,
    /// VC tools version (e.g., 14.29.30133)
    vc_tools_version: String,
}

impl MsvcToolchain {
    /// Detect MSVC installation using vswhere
    ///
    /// This only works on Windows where Visual Studio is installed.
    pub fn detect() -> Result<Self> {
        // On non-Windows platforms, MSVC is not available
        #[cfg(not(target_os = "windows"))]
        {
            bail!(
                "MSVC toolchain is only available on Windows. \
                 Use MinGW for cross-compilation from macOS/Linux."
            );
        }

        #[cfg(target_os = "windows")]
        {
            Self::detect_windows()
        }
    }

    #[cfg(target_os = "windows")]
    fn detect_windows() -> Result<Self> {
        // Try to find vswhere
        let vswhere_paths = [
            r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe",
            r"C:\Program Files\Microsoft Visual Studio\Installer\vswhere.exe",
        ];

        let vswhere_path = vswhere_paths
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Visual Studio not found. Please install Visual Studio 2019 or later."
                )
            })?;

        // Run vswhere to find VS installation
        let output = Command::new(vswhere_path)
            .args([
                "-latest",
                "-requires",
                "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "-property",
                "installationPath",
            ])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run vswhere: {}", e))?;

        let vs_path = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        if vs_path.is_empty() {
            bail!("No Visual Studio installation with C++ tools found");
        }

        let vs_path = PathBuf::from(&vs_path);

        // Get VS version
        let version_output = Command::new(vswhere_path)
            .args(["-latest", "-property", "installationVersion"])
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to get VS version: {}", e))?;

        let version = String::from_utf8_lossy(&version_output.stdout)
            .trim()
            .to_string();

        // Get VC tools version
        let vc_version_file = vs_path
            .join("VC")
            .join("Auxiliary")
            .join("Build")
            .join("Microsoft.VCToolsVersion.default.txt");

        let vc_tools_version = if vc_version_file.exists() {
            std::fs::read_to_string(&vc_version_file)
                .unwrap_or_default()
                .trim()
                .to_string()
        } else {
            "unknown".to_string()
        };

        Ok(Self {
            vs_path,
            version,
            vc_tools_version,
        })
    }

    /// Get the Visual Studio installation path
    pub fn vs_path(&self) -> &PathBuf {
        &self.vs_path
    }

    /// Get the Visual Studio version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the VC tools version
    pub fn vc_tools_version(&self) -> &str {
        &self.vc_tools_version
    }

    /// Get the path to vcvarsall.bat
    pub fn vcvarsall_path(&self) -> PathBuf {
        self.vs_path
            .join("VC")
            .join("Auxiliary")
            .join("Build")
            .join("vcvarsall.bat")
    }

    /// Get CMake generator for this MSVC version
    pub fn cmake_generator(&self) -> &str {
        // Parse major version to determine generator
        let major = self
            .version
            .split('.')
            .next()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(16);

        match major {
            17 => "Visual Studio 17 2022",
            16 => "Visual Studio 16 2019",
            15 => "Visual Studio 15 2017",
            _ => "Visual Studio 16 2019", // Default to VS 2019
        }
    }
}

impl Toolchain for MsvcToolchain {
    fn name(&self) -> &str {
        "msvc"
    }

    fn is_available(&self) -> bool {
        self.vs_path.exists() && self.vcvarsall_path().exists()
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.vs_path.clone())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        vec![
            (
                "CMAKE_GENERATOR".to_string(),
                self.cmake_generator().to_string(),
            ),
            ("CMAKE_GENERATOR_PLATFORM".to_string(), "x64".to_string()),
        ]
    }

    fn validate(&self) -> Result<()> {
        if !self.vs_path.exists() {
            bail!(
                "Visual Studio installation not found at: {}",
                self.vs_path.display()
            );
        }

        let vcvarsall = self.vcvarsall_path();
        if !vcvarsall.exists() {
            bail!("vcvarsall.bat not found at: {}", vcvarsall.display());
        }

        Ok(())
    }
}

/// Check if MSVC is available
pub fn is_msvc_available() -> bool {
    MsvcToolchain::detect().is_ok()
}
