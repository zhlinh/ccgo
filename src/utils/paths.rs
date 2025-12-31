//! Path utilities for ccgo CLI

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Find the project root by looking for CCGO.toml
pub fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    find_project_root_from(&current_dir)
}

/// Find the project root starting from a specific directory
pub fn find_project_root_from(start: &Path) -> Result<PathBuf> {
    let mut dir = start;
    loop {
        if dir.join("CCGO.toml").exists() {
            return Ok(dir.to_path_buf());
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                anyhow::bail!("Could not find CCGO.toml in current directory or any parent")
            }
        }
    }
}

/// Get the target directory for build outputs
pub fn get_target_dir(project_root: &Path) -> PathBuf {
    project_root.join("target")
}

/// Get the cmake build base directory (without release/debug subdirectory)
pub fn get_cmake_build_dir(project_root: &Path) -> PathBuf {
    project_root.join("cmake_build")
}

/// Get the cmake build directory for a specific build mode
/// Returns cmake_build/{release|debug} path
pub fn get_cmake_build_dir_with_mode(project_root: &Path, release: bool) -> PathBuf {
    let subdir = if release { "release" } else { "debug" };
    project_root.join("cmake_build").join(subdir)
}

/// Get the cmake build directory for a specific platform and build mode
/// Returns cmake_build/{release|debug}/{platform} path
pub fn get_cmake_build_dir_for_platform(project_root: &Path, platform: &str, release: bool) -> PathBuf {
    let subdir = if release { "release" } else { "debug" };
    project_root.join("cmake_build").join(subdir).join(platform.to_lowercase())
}

/// Ensure a directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}
