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

/// Get the cmake build directory
pub fn get_cmake_build_dir(project_root: &Path) -> PathBuf {
    project_root.join("cmake_build")
}

/// Ensure a directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}
