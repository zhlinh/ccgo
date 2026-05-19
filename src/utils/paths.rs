//! Path utilities for ccgo CLI

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Top-level build artifact directory created by ccgo inside every project.
pub const CCGO_BUILD_DIR: &str = "ccgo_build";

/// Sanitize an arbitrary string so it is safe to use as a path component.
///
/// Keeps ASCII alphanumerics, hyphens, and underscores; replaces everything
/// else (spaces, dots, slashes, …) with `_`.
pub fn sanitize_for_path(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

/// Remove all ccgo build directories for `platform` under the build root.
///
/// `build_root` is the top-level build directory, e.g.
/// `{project_root}/ccgo_build` (or a custom name from `[build].build_dir`).
///
/// Iterates every direct child of `build_root` (e.g. `release`, `debug`,
/// `release-my-profile`, `debug-my-profile`) and removes the `{platform}/`
/// subdirectory within each, covering all profile variants in one call.
pub fn clean_ccgo_build_platform(build_root: &Path, platform: &str) -> Result<()> {
    if !build_root.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(build_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let plat_dir = entry.path().join(platform);
        if plat_dir.exists() {
            std::fs::remove_dir_all(&plat_dir)
                .with_context(|| format!("Failed to clean {}", plat_dir.display()))?;
        }
    }
    Ok(())
}

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

/// Get the ccgo build base directory (without release/debug subdirectory)
pub fn get_ccgo_build_dir(project_root: &Path) -> PathBuf {
    project_root.join(CCGO_BUILD_DIR)
}

/// Get the ccgo build directory for a specific build mode (no profile)
/// Returns ccgo_build/{release|debug} path
pub fn get_ccgo_build_dir_with_mode(project_root: &Path, release: bool) -> PathBuf {
    let subdir = if release { "release" } else { "debug" };
    project_root.join(CCGO_BUILD_DIR).join(subdir)
}

/// Get the ccgo build directory for a specific platform and build mode (no profile)
/// Returns ccgo_build/{release|debug}/{platform} path
pub fn get_ccgo_build_dir_for_platform(
    project_root: &Path,
    platform: &str,
    release: bool,
) -> PathBuf {
    let subdir = if release { "release" } else { "debug" };
    project_root
        .join(CCGO_BUILD_DIR)
        .join(subdir)
        .join(platform.to_lowercase())
}

/// Ensure a directory exists
pub fn ensure_dir(path: &Path) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
    }
    Ok(())
}
