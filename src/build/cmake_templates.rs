//! Embedded CMake template files
//!
//! This module embeds CMake templates into the Rust binary for zero Python dependency.
//! Templates are extracted from Python ccgo's build_scripts/cmake/ directory.

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

// Embed main CMake utility files
const CMAKE_UTILS: &str = include_str!("../../cmake/CMakeUtils.cmake");
const CMAKE_CONFIG: &str = include_str!("../../cmake/CMakeConfig.cmake");
const CMAKE_FUNCTIONS: &str = include_str!("../../cmake/CMakeFunctions.cmake");
const CMAKE_EXTRA_FLAGS: &str = include_str!("../../cmake/CMakeExtraFlags.cmake");
const CCGO_DEPENDENCIES: &str = include_str!("../../cmake/CCGODependencies.cmake");
const FIND_CCGO_DEPENDENCIES: &str = include_str!("../../cmake/FindCCGODependencies.cmake");

// Embed toolchain files
const IOS_TOOLCHAIN: &str = include_str!("../../cmake/ios.toolchain.cmake");
const TVOS_TOOLCHAIN: &str = include_str!("../../cmake/tvos.toolchain.cmake");
const WATCHOS_TOOLCHAIN: &str = include_str!("../../cmake/watchos.toolchain.cmake");
const WINDOWS_MSVC_TOOLCHAIN: &str = include_str!("../../cmake/windows-msvc.toolchain.cmake");

// Embed template files
const TEMPLATE_ROOT_CMAKELISTS: &str = include_str!("../../cmake/template/Root.CMakeLists.txt.in");
const TEMPLATE_SRC_CMAKELISTS: &str = include_str!("../../cmake/template/Src.CMakeLists.txt.in");
const TEMPLATE_TESTS_CMAKELISTS: &str = include_str!("../../cmake/template/Tests.CMakeLists.txt.in");
const TEMPLATE_BENCHES_CMAKELISTS: &str = include_str!("../../cmake/template/Benches.CMakeLists.txt.in");
const TEMPLATE_THIRD_PARTY_CMAKELISTS: &str = include_str!("../../cmake/template/ThirdParty.CMakeLists.txt.in");
const TEMPLATE_EXTERNAL_CMAKELISTS: &str = include_str!("../../cmake/template/External.CMakeLists.txt.in");
const TEMPLATE_EXTERNAL_DOWNLOAD: &str = include_str!("../../cmake/template/External.Download.txt.in");
const TEMPLATE_SRC_SUBDIR_CMAKELISTS: &str = include_str!("../../cmake/template/Src.SubDir.CMakeLists.txt.in");

/// Get or create the CMake templates directory
///
/// Templates are written to: ~/.ccgo/cmake/ for persistent caching
/// This avoids recreating files on every build while keeping them user-accessible
pub fn get_cmake_dir() -> Result<PathBuf> {
    // Use user's home directory for persistent storage
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory from HOME or USERPROFILE env var")?;
    let home = PathBuf::from(home);

    let cmake_dir = home.join(".ccgo/cmake");

    // Check if templates are already extracted and up-to-date
    if cmake_dir.exists() && is_templates_current(&cmake_dir) {
        return Ok(cmake_dir);
    }

    // Create directory
    fs::create_dir_all(&cmake_dir)
        .with_context(|| format!("Failed to create CMake directory: {}", cmake_dir.display()))?;

    // Write all template files
    write_templates(&cmake_dir)?;

    Ok(cmake_dir)
}

/// Check if extracted templates are current (not stale)
fn is_templates_current(cmake_dir: &Path) -> bool {
    // Check if key files exist
    let key_files = [
        "CMakeUtils.cmake",
        "CMakeFunctions.cmake",
        "ios.toolchain.cmake",
        "template/Root.CMakeLists.txt.in",
    ];

    for file in &key_files {
        if !cmake_dir.join(file).exists() {
            return false;
        }
    }

    true
}

/// Write all embedded templates to the target directory
fn write_templates(cmake_dir: &Path) -> Result<()> {
    // Write main CMake files
    write_file(cmake_dir, "CMakeUtils.cmake", CMAKE_UTILS)?;
    write_file(cmake_dir, "CMakeConfig.cmake", CMAKE_CONFIG)?;
    write_file(cmake_dir, "CMakeFunctions.cmake", CMAKE_FUNCTIONS)?;
    write_file(cmake_dir, "CMakeExtraFlags.cmake", CMAKE_EXTRA_FLAGS)?;
    write_file(cmake_dir, "CCGODependencies.cmake", CCGO_DEPENDENCIES)?;
    write_file(cmake_dir, "FindCCGODependencies.cmake", FIND_CCGO_DEPENDENCIES)?;

    // Write toolchain files
    write_file(cmake_dir, "ios.toolchain.cmake", IOS_TOOLCHAIN)?;
    write_file(cmake_dir, "tvos.toolchain.cmake", TVOS_TOOLCHAIN)?;
    write_file(cmake_dir, "watchos.toolchain.cmake", WATCHOS_TOOLCHAIN)?;
    write_file(cmake_dir, "windows-msvc.toolchain.cmake", WINDOWS_MSVC_TOOLCHAIN)?;

    // Create template subdirectory
    let template_dir = cmake_dir.join("template");
    fs::create_dir_all(&template_dir)?;

    // Write template files
    write_file(&template_dir, "Root.CMakeLists.txt.in", TEMPLATE_ROOT_CMAKELISTS)?;
    write_file(&template_dir, "Src.CMakeLists.txt.in", TEMPLATE_SRC_CMAKELISTS)?;
    write_file(&template_dir, "Tests.CMakeLists.txt.in", TEMPLATE_TESTS_CMAKELISTS)?;
    write_file(&template_dir, "Benches.CMakeLists.txt.in", TEMPLATE_BENCHES_CMAKELISTS)?;
    write_file(&template_dir, "ThirdParty.CMakeLists.txt.in", TEMPLATE_THIRD_PARTY_CMAKELISTS)?;
    write_file(&template_dir, "External.CMakeLists.txt.in", TEMPLATE_EXTERNAL_CMAKELISTS)?;
    write_file(&template_dir, "External.Download.txt.in", TEMPLATE_EXTERNAL_DOWNLOAD)?;
    write_file(&template_dir, "Src.SubDir.CMakeLists.txt.in", TEMPLATE_SRC_SUBDIR_CMAKELISTS)?;

    Ok(())
}

/// Write a single file to the filesystem
fn write_file(dir: &Path, filename: &str, content: &str) -> Result<()> {
    let path = dir.join(filename);
    fs::write(&path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Clean up the CMake templates directory (for troubleshooting)
#[allow(dead_code)]
pub fn clean_cmake_dir() -> Result<()> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory from HOME or USERPROFILE env var")?;
    let home = PathBuf::from(home);
    let cmake_dir = home.join(".ccgo/cmake");

    if cmake_dir.exists() {
        fs::remove_dir_all(&cmake_dir)
            .with_context(|| format!("Failed to remove {}", cmake_dir.display()))?;
    }

    Ok(())
}
