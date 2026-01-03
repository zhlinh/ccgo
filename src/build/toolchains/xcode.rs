//! Xcode toolchain detection for Apple platforms
//!
//! Detects and configures Xcode for macOS, iOS, tvOS, and watchOS builds.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context, Result};

use super::Toolchain;

/// Apple platform targets
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ApplePlatform {
    MacOS,
    IOS,
    IOSSimulator,
    TvOS,
    TvOSSimulator,
    WatchOS,
    WatchOSSimulator,
}

impl ApplePlatform {
    /// Get the SDK name for this platform
    pub fn sdk_name(&self) -> &str {
        match self {
            ApplePlatform::MacOS => "macosx",
            ApplePlatform::IOS => "iphoneos",
            ApplePlatform::IOSSimulator => "iphonesimulator",
            ApplePlatform::TvOS => "appletvos",
            ApplePlatform::TvOSSimulator => "appletvsimulator",
            ApplePlatform::WatchOS => "watchos",
            ApplePlatform::WatchOSSimulator => "watchsimulator",
        }
    }

    /// Get the deployment target variable name
    pub fn deployment_target_var(&self) -> &str {
        match self {
            ApplePlatform::MacOS => "CMAKE_OSX_DEPLOYMENT_TARGET",
            ApplePlatform::IOS | ApplePlatform::IOSSimulator => "CMAKE_OSX_DEPLOYMENT_TARGET",
            ApplePlatform::TvOS | ApplePlatform::TvOSSimulator => "CMAKE_OSX_DEPLOYMENT_TARGET",
            ApplePlatform::WatchOS | ApplePlatform::WatchOSSimulator => "CMAKE_OSX_DEPLOYMENT_TARGET",
        }
    }

    /// Get the minimum deployment target version
    pub fn min_deployment_target(&self) -> &str {
        match self {
            ApplePlatform::MacOS => "10.15",
            ApplePlatform::IOS | ApplePlatform::IOSSimulator => "12.0",
            ApplePlatform::TvOS | ApplePlatform::TvOSSimulator => "12.0",
            ApplePlatform::WatchOS | ApplePlatform::WatchOSSimulator => "5.0",
        }
    }

    /// Get valid architectures for this platform
    pub fn valid_architectures(&self) -> Vec<&str> {
        match self {
            ApplePlatform::MacOS => vec!["x86_64", "arm64"],
            ApplePlatform::IOS => vec!["arm64"],
            ApplePlatform::IOSSimulator => vec!["x86_64", "arm64"],
            ApplePlatform::TvOS => vec!["arm64"],
            // tvOS Simulator only supports arm64 (Apple Silicon) since Xcode 14
            ApplePlatform::TvOSSimulator => vec!["arm64"],
            ApplePlatform::WatchOS => vec!["arm64_32", "armv7k"],
            // watchOS Simulator only supports arm64 (Apple Silicon) since Xcode 14
            ApplePlatform::WatchOSSimulator => vec!["arm64"],
        }
    }
}

/// Xcode toolchain for Apple platforms
pub struct XcodeToolchain {
    /// Path to Xcode developer directory
    developer_dir: PathBuf,
    /// Xcode version string
    version: String,
    /// Xcode build version
    build_version: String,
}

impl XcodeToolchain {
    /// Detect Xcode installation
    pub fn detect() -> Result<Self> {
        // Run xcode-select -p to get developer directory
        let output = Command::new("xcode-select")
            .arg("-p")
            .output()
            .context("Failed to run xcode-select. Is Xcode installed?")?;

        if !output.status.success() {
            bail!("xcode-select failed. Please run: xcode-select --install");
        }

        let developer_dir = PathBuf::from(
            String::from_utf8_lossy(&output.stdout).trim()
        );

        if !developer_dir.exists() {
            bail!("Xcode developer directory not found: {}", developer_dir.display());
        }

        // Get Xcode version
        let (version, build_version) = Self::get_xcode_version()?;

        Ok(Self {
            developer_dir,
            version,
            build_version,
        })
    }

    /// Get Xcode version and build version
    fn get_xcode_version() -> Result<(String, String)> {
        let output = Command::new("xcodebuild")
            .arg("-version")
            .output()
            .context("Failed to run xcodebuild")?;

        if !output.status.success() {
            bail!("xcodebuild -version failed");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        let version = lines
            .first()
            .and_then(|l| l.strip_prefix("Xcode "))
            .unwrap_or("unknown")
            .to_string();

        let build_version = lines
            .get(1)
            .and_then(|l| l.strip_prefix("Build version "))
            .unwrap_or("unknown")
            .to_string();

        Ok((version, build_version))
    }

    /// Get the SDK path for a platform
    pub fn sdk_path(&self, platform: ApplePlatform) -> Result<PathBuf> {
        let output = Command::new("xcrun")
            .args(["--sdk", platform.sdk_name(), "--show-sdk-path"])
            .output()
            .context("Failed to run xcrun")?;

        if !output.status.success() {
            bail!("Failed to find SDK for {}", platform.sdk_name());
        }

        let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
        if !path.exists() {
            bail!("SDK path does not exist: {}", path.display());
        }

        Ok(path)
    }

    /// Get the SDK version for a platform
    pub fn sdk_version(&self, platform: ApplePlatform) -> Result<String> {
        let output = Command::new("xcrun")
            .args(["--sdk", platform.sdk_name(), "--show-sdk-version"])
            .output()
            .context("Failed to run xcrun")?;

        if !output.status.success() {
            bail!("Failed to get SDK version for {}", platform.sdk_name());
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the Xcode version string
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get the Xcode build version
    pub fn build_version(&self) -> &str {
        &self.build_version
    }

    /// Get CMake variables for a specific platform
    pub fn cmake_variables_for_platform(&self, platform: ApplePlatform) -> Result<Vec<(String, String)>> {
        let sdk_path = self.sdk_path(platform)?;

        let mut vars = vec![
            ("CMAKE_OSX_SYSROOT".to_string(), sdk_path.display().to_string()),
            (
                platform.deployment_target_var().to_string(),
                platform.min_deployment_target().to_string(),
            ),
        ];

        // Set architectures
        let archs = platform.valid_architectures().join(";");
        vars.push(("CMAKE_OSX_ARCHITECTURES".to_string(), archs));

        Ok(vars)
    }

    /// Run lipo to create a universal binary
    pub fn create_universal_binary(
        &self,
        input_libs: &[PathBuf],
        output: &PathBuf,
    ) -> Result<()> {
        let mut cmd = Command::new("lipo");
        cmd.arg("-create");

        for lib in input_libs {
            cmd.arg(lib);
        }

        cmd.arg("-output").arg(output);

        let status = cmd.status().context("Failed to run lipo")?;
        if !status.success() {
            bail!("lipo failed to create universal binary");
        }

        Ok(())
    }

    /// Run install_name_tool to fix library paths
    pub fn fix_install_name(
        &self,
        library: &PathBuf,
        old_path: &str,
        new_path: &str,
    ) -> Result<()> {
        let status = Command::new("install_name_tool")
            .args(["-change", old_path, new_path])
            .arg(library)
            .status()
            .context("Failed to run install_name_tool")?;

        if !status.success() {
            bail!("install_name_tool failed");
        }

        Ok(())
    }

    /// Create an XCFramework from multiple frameworks/libraries
    /// Merge multiple static libraries into a single static library using libtool
    ///
    /// This is essential for creating a complete static library from multiple
    /// module libraries (e.g., libfoo-api.a, libfoo-base.a -> libfoo.a)
    pub fn merge_static_libs(&self, src_libs: &[PathBuf], dst_lib: &PathBuf) -> Result<()> {
        if src_libs.is_empty() {
            bail!("No source libraries to merge");
        }

        // Ensure output directory exists
        if let Some(parent) = dst_lib.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Use libtool to merge static libraries
        let mut cmd = Command::new("libtool");
        cmd.arg("-static")
           .arg("-no_warning_for_no_symbols")
           .arg("-o")
           .arg(dst_lib);

        for lib in src_libs {
            cmd.arg(lib);
        }

        let output = cmd.output().context("Failed to run libtool")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("libtool failed: {}", stderr);
        }

        Ok(())
    }

    pub fn create_xcframework(
        &self,
        inputs: &[(PathBuf, Option<PathBuf>)], // (library/framework path, optional dSYM)
        output: &PathBuf,
    ) -> Result<()> {
        let mut cmd = Command::new("xcodebuild");
        cmd.arg("-create-xcframework");

        for (lib, dsym) in inputs {
            if lib.extension().map_or(false, |e| e == "framework") {
                cmd.arg("-framework").arg(lib);
            } else {
                cmd.arg("-library").arg(lib);
            }

            if let Some(dsym_path) = dsym {
                cmd.arg("-debug-symbols").arg(dsym_path);
            }
        }

        cmd.arg("-output").arg(output);

        let status = cmd.status().context("Failed to run xcodebuild -create-xcframework")?;
        if !status.success() {
            bail!("Failed to create XCFramework");
        }

        Ok(())
    }
}

impl Toolchain for XcodeToolchain {
    fn name(&self) -> &str {
        "xcode"
    }

    fn is_available(&self) -> bool {
        self.developer_dir.exists()
    }

    fn path(&self) -> Option<PathBuf> {
        Some(self.developer_dir.clone())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        // Default to macOS variables
        self.cmake_variables_for_platform(ApplePlatform::MacOS)
            .unwrap_or_default()
    }

    fn validate(&self) -> Result<()> {
        // Check that developer directory exists
        if !self.developer_dir.exists() {
            bail!("Xcode developer directory not found: {}", self.developer_dir.display());
        }

        // Verify we can run xcodebuild
        let status = Command::new("xcodebuild")
            .arg("-version")
            .status()
            .context("Cannot run xcodebuild")?;

        if !status.success() {
            bail!("xcodebuild validation failed. Check Xcode license agreement.");
        }

        Ok(())
    }
}

/// Check if Xcode is available on the system
pub fn is_xcode_available() -> bool {
    Command::new("xcode-select")
        .arg("-p")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if command line tools are installed (without full Xcode)
pub fn is_command_line_tools_installed() -> bool {
    let result = Command::new("xcode-select")
        .arg("-p")
        .output()
        .ok();

    if let Some(output) = result {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout);
            return path.contains("CommandLineTools");
        }
    }
    false
}
