//! Tool detection and validation with graceful degradation
//!
//! This module provides utilities for detecting required build tools
//! and providing helpful error messages when tools are missing.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use which::which;

use crate::error::{CcgoError, hints};

/// Tool detection result
#[derive(Debug, Clone)]
pub struct ToolInfo {
    /// Tool name
    pub name: String,
    /// Path to the tool executable
    pub path: PathBuf,
    /// Tool version string (if available)
    pub version: Option<String>,
}

/// Tool requirement level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolRequirement {
    /// Tool is required for the operation
    Required,
    /// Tool is optional, operation can proceed without it
    Optional,
    /// Tool is recommended, warn if missing but continue
    Recommended,
}

/// Check if a tool exists and return its information
pub fn check_tool(tool_name: &str) -> Option<ToolInfo> {
    match which(tool_name) {
        Ok(path) => {
            let version = get_tool_version(tool_name);
            Some(ToolInfo {
                name: tool_name.to_string(),
                path,
                version,
            })
        }
        Err(_) => None,
    }
}

/// Get tool version by running `tool --version`
fn get_tool_version(tool_name: &str) -> Option<String> {
    // Try --version first
    if let Ok(output) = Command::new(tool_name)
        .arg("--version")
        .output()
    {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some(version.lines().next().unwrap_or("").trim().to_string());
        }
    }

    // Try -version for some tools
    if let Ok(output) = Command::new(tool_name)
        .arg("-version")
        .output()
    {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            return Some(version.lines().next().unwrap_or("").trim().to_string());
        }
    }

    None
}

/// Require a tool to exist, return error with hint if missing
pub fn require_tool(tool_name: &str, required_for: &str) -> Result<ToolInfo> {
    match check_tool(tool_name) {
        Some(info) => Ok(info),
        None => {
            let hint = get_tool_hint(tool_name);
            Err(CcgoError::missing_tool(
                tool_name,
                required_for,
                hint,
            ).into())
        }
    }
}

/// Check for a tool with requirement level
pub fn check_tool_with_requirement(
    tool_name: &str,
    requirement: ToolRequirement,
    required_for: &str,
) -> Result<Option<ToolInfo>> {
    match check_tool(tool_name) {
        Some(info) => Ok(Some(info)),
        None => match requirement {
            ToolRequirement::Required => {
                let hint = get_tool_hint(tool_name);
                Err(CcgoError::missing_tool(
                    tool_name,
                    required_for,
                    hint,
                ).into())
            }
            ToolRequirement::Recommended => {
                eprintln!(
                    "⚠️  Warning: Recommended tool '{}' not found.\n\
                     Required for: {}\n\
                     {}\n",
                    tool_name,
                    required_for,
                    get_tool_hint(tool_name)
                );
                Ok(None)
            }
            ToolRequirement::Optional => Ok(None),
        },
    }
}

/// Get installation hint for a tool
fn get_tool_hint(tool_name: &str) -> &'static str {
    match tool_name {
        "cmake" => hints::cmake(),
        "git" => hints::git(),
        "gradle" => hints::gradle(),
        "python" | "python3" => hints::python(),
        "doxygen" => hints::doxygen(),
        _ => "Install this tool and ensure it's in your PATH",
    }
}

/// Check multiple tools and return results
pub fn check_tools(tool_names: &[&str]) -> Vec<(String, Option<ToolInfo>)> {
    tool_names
        .iter()
        .map(|name| {
            let info = check_tool(name);
            (name.to_string(), info)
        })
        .collect()
}

/// Platform-specific tool checker
pub struct PlatformTools {
    platform: String,
    required_tools: Vec<String>,
    optional_tools: Vec<String>,
}

impl PlatformTools {
    /// Create a new platform tool checker
    pub fn new(platform: &str) -> Self {
        let (required, optional) = match platform {
            "android" => (
                vec!["cmake".to_string(), "gradle".to_string()],
                vec!["python3".to_string()],
            ),
            "ios" | "macos" | "tvos" | "watchos" => (
                vec!["cmake".to_string(), "xcodebuild".to_string()],
                vec!["python3".to_string()],
            ),
            "windows" => (
                vec!["cmake".to_string()],
                vec!["python3".to_string(), "cl".to_string(), "g++".to_string()],
            ),
            "linux" => (
                vec!["cmake".to_string(), "make".to_string()],
                vec!["python3".to_string(), "g++".to_string(), "clang".to_string()],
            ),
            "ohos" => (
                vec!["cmake".to_string()],
                vec!["python3".to_string()],
            ),
            _ => (vec!["cmake".to_string()], vec![]),
        };

        Self {
            platform: platform.to_string(),
            required_tools: required,
            optional_tools: optional,
        }
    }

    /// Check all required tools for the platform
    pub fn check_required(&self) -> Result<Vec<ToolInfo>> {
        let mut tools = Vec::new();
        let mut missing = Vec::new();

        for tool_name in &self.required_tools {
            match check_tool(tool_name) {
                Some(info) => tools.push(info),
                None => missing.push(tool_name.clone()),
            }
        }

        if !missing.is_empty() {
            let missing_list = missing.join(", ");
            let hints = missing
                .iter()
                .map(|name| format!("• {}: {}", name, get_tool_hint(name)))
                .collect::<Vec<_>>()
                .join("\n");

            return Err(CcgoError::build_failure(
                &self.platform,
                format!("Missing required tools: {}", missing_list),
            )
            .into());
        }

        Ok(tools)
    }

    /// Check optional tools and warn if missing
    pub fn check_optional(&self) -> Vec<ToolInfo> {
        let mut tools = Vec::new();

        for tool_name in &self.optional_tools {
            match check_tool(tool_name) {
                Some(info) => tools.push(info),
                None => {
                    eprintln!(
                        "ℹ️  Optional tool '{}' not found (not required for {})",
                        tool_name, self.platform
                    );
                }
            }
        }

        tools
    }

    /// Check all tools (required + optional)
    pub fn check_all(&self) -> Result<(Vec<ToolInfo>, Vec<ToolInfo>)> {
        let required = self.check_required()?;
        let optional = self.check_optional();
        Ok((required, optional))
    }
}

/// Check Android-specific environment
pub fn check_android_environment() -> Result<()> {
    // Check for Android SDK
    if let Ok(sdk_root) = std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
    {
        println!("✓ Android SDK found at: {}", sdk_root);
    } else {
        return Err(CcgoError::missing_tool(
            "Android SDK",
            "Android builds",
            hints::android_ndk(),
        ).into());
    }

    // Check for Android NDK
    if let Ok(ndk_root) = std::env::var("ANDROID_NDK") {
        println!("✓ Android NDK found at: {}", ndk_root);
    } else if let Ok(sdk_root) = std::env::var("ANDROID_HOME")
        .or_else(|_| std::env::var("ANDROID_SDK_ROOT"))
    {
        // Try to find NDK in SDK
        let ndk_dir = PathBuf::from(sdk_root).join("ndk");
        if ndk_dir.exists() {
            println!("✓ Android NDK found in SDK");
        } else {
            eprintln!(
                "⚠️  Warning: ANDROID_NDK not set and NDK not found in SDK.\n\
                 {}",
                hints::android_ndk()
            );
        }
    }

    Ok(())
}

/// Check Apple development environment (Xcode)
pub fn check_apple_environment() -> Result<()> {
    // Check for xcodebuild
    require_tool("xcodebuild", "Apple platform builds")?;

    // Check Xcode version
    if let Ok(output) = Command::new("xcodebuild").arg("-version").output() {
        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stdout);
            println!("✓ {}", version.lines().next().unwrap_or("Xcode installed"));
        }
    }

    // Check for command line tools
    if let Ok(output) = Command::new("xcode-select").arg("-p").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("✓ Command Line Tools: {}", path);
        }
    } else {
        eprintln!(
            "⚠️  Warning: Xcode Command Line Tools may not be installed.\n\
             Run: sudo xcode-select --install"
        );
    }

    Ok(())
}

/// Check Windows development environment
pub fn check_windows_environment(toolchain: &str) -> Result<()> {
    match toolchain {
        "msvc" | "auto" => {
            // Check for Visual Studio or Build Tools
            if check_tool("cl").is_none() {
                if toolchain == "msvc" {
                    return Err(CcgoError::missing_tool(
                        "cl (MSVC compiler)",
                        "Windows MSVC builds",
                        hints::visual_studio(),
                    ).into());
                } else {
                    eprintln!(
                        "⚠️  MSVC compiler not found. Trying MinGW...\n\
                         For MSVC: {}",
                        hints::visual_studio()
                    );
                }
            } else {
                println!("✓ MSVC compiler found");
            }
        }
        "mingw" => {
            require_tool("g++", "Windows MinGW builds")?;
            println!("✓ MinGW compiler found");
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_common_tools() {
        // These tools should exist on most development systems
        let tools = vec!["sh", "echo"];
        let results = check_tools(&tools);

        for (name, info) in results {
            if info.is_some() {
                println!("✓ {} found", name);
            }
        }
    }

    #[test]
    fn test_platform_tools() {
        let checker = PlatformTools::new("linux");
        assert!(checker.required_tools.contains(&"cmake".to_string()));
    }
}
