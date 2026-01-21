//! Build cache detection and configuration
//!
//! This module provides support for compiler caching tools like ccache and sccache
//! to speed up C++ compilation by 30-50% through caching of compilation artifacts.

use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Command;

/// Compiler cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    /// ccache (C/C++ compiler cache)
    CCache,
    /// sccache (Shared Compilation Cache)
    SCache,
    /// No cache
    None,
}

impl CacheType {
    /// Get the executable name for this cache type
    pub fn executable(&self) -> Option<&str> {
        match self {
            CacheType::CCache => Some("ccache"),
            CacheType::SCache => Some("sccache"),
            CacheType::None => None,
        }
    }

    /// Get the display name for this cache type
    pub fn name(&self) -> &str {
        match self {
            CacheType::CCache => "ccache",
            CacheType::SCache => "sccache",
            CacheType::None => "none",
        }
    }
}

/// Cache configuration and detection
#[derive(Debug)]
pub struct CacheConfig {
    cache_type: CacheType,
    executable_path: Option<PathBuf>,
}

impl CacheConfig {
    /// Detect available cache automatically (prefers sccache over ccache)
    pub fn auto() -> Result<Self> {
        // Try sccache first (faster, more features)
        if let Ok(config) = Self::detect_sccache() {
            return Ok(config);
        }

        // Fall back to ccache
        if let Ok(config) = Self::detect_ccache() {
            return Ok(config);
        }

        // No cache available
        Ok(Self {
            cache_type: CacheType::None,
            executable_path: None,
        })
    }

    /// Detect ccache
    pub fn detect_ccache() -> Result<Self> {
        let path = which::which("ccache")
            .context("ccache not found in PATH")?;

        // Verify ccache works
        let output = Command::new(&path)
            .arg("--version")
            .output()
            .context("Failed to run ccache --version")?;

        if !output.status.success() {
            anyhow::bail!("ccache --version failed");
        }

        Ok(Self {
            cache_type: CacheType::CCache,
            executable_path: Some(path),
        })
    }

    /// Detect sccache
    pub fn detect_sccache() -> Result<Self> {
        let path = which::which("sccache")
            .context("sccache not found in PATH")?;

        // Verify sccache works
        let output = Command::new(&path)
            .arg("--version")
            .output()
            .context("Failed to run sccache --version")?;

        if !output.status.success() {
            anyhow::bail!("sccache --version failed");
        }

        Ok(Self {
            cache_type: CacheType::SCache,
            executable_path: Some(path),
        })
    }

    /// Create a config with explicit cache type
    pub fn with_type(cache_type: CacheType) -> Result<Self> {
        match cache_type {
            CacheType::CCache => Self::detect_ccache(),
            CacheType::SCache => Self::detect_sccache(),
            CacheType::None => Ok(Self {
                cache_type: CacheType::None,
                executable_path: None,
            }),
        }
    }

    /// Create a disabled cache config
    pub fn disabled() -> Self {
        Self {
            cache_type: CacheType::None,
            executable_path: None,
        }
    }

    /// Get the cache type
    pub fn cache_type(&self) -> CacheType {
        self.cache_type
    }

    /// Get the executable path
    pub fn executable_path(&self) -> Option<&PathBuf> {
        self.executable_path.as_ref()
    }

    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.cache_type != CacheType::None
    }

    /// Get cache statistics (if supported)
    pub fn get_stats(&self) -> Result<String> {
        let path = self.executable_path.as_ref()
            .context("No cache executable available")?;

        let output = match self.cache_type {
            CacheType::CCache => {
                Command::new(path)
                    .arg("-s")
                    .output()
                    .context("Failed to run ccache -s")?
            }
            CacheType::SCache => {
                Command::new(path)
                    .arg("--show-stats")
                    .output()
                    .context("Failed to run sccache --show-stats")?
            }
            CacheType::None => anyhow::bail!("No cache enabled"),
        };

        if !output.status.success() {
            anyhow::bail!("Failed to get cache statistics");
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Zero cache statistics (if supported)
    pub fn zero_stats(&self) -> Result<()> {
        let path = self.executable_path.as_ref()
            .context("No cache executable available")?;

        let status = match self.cache_type {
            CacheType::CCache => {
                Command::new(path)
                    .arg("-z")
                    .status()
                    .context("Failed to run ccache -z")?
            }
            CacheType::SCache => {
                Command::new(path)
                    .arg("--zero-stats")
                    .status()
                    .context("Failed to run sccache --zero-stats")?
            }
            CacheType::None => anyhow::bail!("No cache enabled"),
        };

        if !status.success() {
            anyhow::bail!("Failed to zero cache statistics");
        }

        Ok(())
    }

    /// Get CMake variables for configuring compiler launcher
    pub fn cmake_variables(&self) -> Vec<(String, String)> {
        if let Some(path) = &self.executable_path {
            let path_str = path.to_string_lossy().to_string();
            vec![
                ("CMAKE_C_COMPILER_LAUNCHER".to_string(), path_str.clone()),
                ("CMAKE_CXX_COMPILER_LAUNCHER".to_string(), path_str),
            ]
        } else {
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_type_name() {
        assert_eq!(CacheType::CCache.name(), "ccache");
        assert_eq!(CacheType::SCache.name(), "sccache");
        assert_eq!(CacheType::None.name(), "none");
    }

    #[test]
    fn test_cache_type_executable() {
        assert_eq!(CacheType::CCache.executable(), Some("ccache"));
        assert_eq!(CacheType::SCache.executable(), Some("sccache"));
        assert_eq!(CacheType::None.executable(), None);
    }

    #[test]
    fn test_disabled_cache() {
        let config = CacheConfig::disabled();
        assert_eq!(config.cache_type(), CacheType::None);
        assert!(!config.is_enabled());
        assert!(config.executable_path().is_none());
    }
}
