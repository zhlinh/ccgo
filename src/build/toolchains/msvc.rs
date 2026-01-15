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
    /// Detect MSVC installation using vswhere (Windows) or xwin (Linux/Docker)
    ///
    /// On Windows: Uses native Visual Studio installation
    /// On Linux: Checks for xwin + clang-cl setup (Docker environment)
    pub fn detect() -> Result<Self> {
        #[cfg(target_os = "windows")]
        {
            Self::detect_windows()
        }

        #[cfg(not(target_os = "windows"))]
        {
            Self::detect_xwin()
        }
    }

    /// Detect xwin-based MSVC setup (Linux/Docker with clang-cl)
    #[cfg(not(target_os = "windows"))]
    fn detect_xwin() -> Result<Self> {
        // Check for xwin SDK directory
        let xwin_sdk_path = PathBuf::from("/opt/xwin/sdk");
        if !xwin_sdk_path.exists() {
            bail!(
                "xwin Windows SDK not found at /opt/xwin/sdk\n\
                 For Docker builds with MSVC toolchain, use: --docker --toolchain msvc\n\
                 For cross-compilation, use MinGW instead: --toolchain mingw"
            );
        }

        // Check for clang-cl wrapper script
        let clang_cl_path = PathBuf::from("/usr/local/bin/clang-cl");
        if !clang_cl_path.exists() {
            bail!(
                "clang-cl wrapper not found at /usr/local/bin/clang-cl\n\
                 Your Docker image may be outdated. Please rebuild it:\n\
                 docker rmi ccgo-builder-windows-msvc"
            );
        }

        // Verify LLVM/Clang is available
        let clang_output = std::process::Command::new("clang")
            .arg("--version")
            .output();

        if clang_output.is_err() {
            bail!("clang not found. xwin requires LLVM/Clang to be installed.");
        }

        // Get LLVM/Clang version
        let version = if let Ok(output) = clang_output {
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("unknown")
                .to_string()
        } else {
            "unknown".to_string()
        };

        Ok(Self {
            vs_path: xwin_sdk_path,
            version,
            vc_tools_version: "xwin".to_string(),
        })
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
        // xwin environment (Linux + clang-cl) uses Ninja
        if self.vc_tools_version == "xwin" {
            return "Ninja";
        }

        // Native Windows MSVC uses Visual Studio generator
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

    /// Check if this is an xwin-based setup (Linux + clang-cl)
    pub fn is_xwin(&self) -> bool {
        self.vc_tools_version == "xwin"
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
        let mut vars = vec![
            (
                "CMAKE_GENERATOR".to_string(),
                self.cmake_generator().to_string(),
            ),
        ];

        // Native Windows MSVC needs platform specification
        if !self.is_xwin() {
            vars.push(("CMAKE_GENERATOR_PLATFORM".to_string(), "x64".to_string()));
        } else {
            // xwin environment uses a CMake toolchain file
            // This file sets CMAKE_SYSTEM_NAME, compilers, and all necessary flags
            // before the project() command, which is required for cross-compilation
            vars.push((
                "CMAKE_TOOLCHAIN_FILE".to_string(),
                "/opt/ccgo/windows-msvc.toolchain.cmake".to_string(),
            ));
        }

        vars
    }

    fn validate(&self) -> Result<()> {
        if !self.vs_path.exists() {
            if self.is_xwin() {
                bail!(
                    "xwin Windows SDK not found at: {}\n\
                     Your Docker image may be missing xwin installation.",
                    self.vs_path.display()
                );
            } else {
                bail!(
                    "Visual Studio installation not found at: {}",
                    self.vs_path.display()
                );
            }
        }

        // For native Windows, check vcvarsall.bat
        if !self.is_xwin() {
            let vcvarsall = self.vcvarsall_path();
            if !vcvarsall.exists() {
                bail!("vcvarsall.bat not found at: {}", vcvarsall.display());
            }
        } else {
            // For xwin, check clang-cl
            let clang_cl = PathBuf::from("/usr/local/bin/clang-cl");
            if !clang_cl.exists() {
                bail!(
                    "clang-cl wrapper not found at: {}\n\
                     Your Docker image may be outdated.",
                    clang_cl.display()
                );
            }
        }

        Ok(())
    }
}

/// Check if MSVC is available
pub fn is_msvc_available() -> bool {
    MsvcToolchain::detect().is_ok()
}
