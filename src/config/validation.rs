//! Enhanced configuration validation with helpful error messages
//!
//! This module provides comprehensive validation for CCGO.toml with
//! actionable hints for common configuration issues.

use anyhow::{bail, Context, Result};
use regex::Regex;
use std::path::Path;

use super::{CcgoConfig, DependencyConfig, PackageConfig};
use crate::error::{CcgoError, hints};

/// Validate the entire CCGO configuration
pub fn validate_config(config: &CcgoConfig) -> Result<()> {
    // Validate package section if present
    if let Some(ref package) = config.package {
        validate_package(package)?;
    }

    // Validate workspace section if present
    if let Some(ref workspace) = config.workspace {
        validate_workspace(workspace, config)?;
    }

    // Validate dependencies
    for dep in &config.dependencies {
        validate_dependency(dep).with_context(|| {
            format!("Invalid dependency configuration for '{}'", dep.name)
        })?;
    }

    // Validate build configuration if present
    if let Some(ref build) = config.build {
        validate_build_config(build)?;
    }

    // Validate platform configurations if present
    if let Some(ref platforms) = config.platforms {
        validate_platform_configs(platforms)?;
    }

    // Validate bins and examples
    for bin in &config.bins {
        validate_bin_config(bin)?;
    }

    for example in &config.examples {
        validate_example_config(example)?;
    }

    Ok(())
}

/// Validate package metadata
fn validate_package(package: &PackageConfig) -> Result<()> {
    // Validate package name (must be valid C++ identifier)
    validate_package_name(&package.name)?;

    // Validate version (must be valid semver)
    validate_version(&package.version)?;

    // Validate license if present (should be SPDX identifier)
    if let Some(ref license) = package.license {
        validate_license(license)?;
    }

    Ok(())
}

/// Validate package name (must be valid C++ identifier)
fn validate_package_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Err(CcgoError::config_error_with_hint(
            "Package name cannot be empty",
            None,
            "Provide a valid package name like 'my_library' or 'awesome-lib'",
        ).into());
    }

    // Check for C++ keywords
    const CPP_KEYWORDS: &[&str] = &[
        "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor",
        "bool", "break", "case", "catch", "char", "class", "compl", "const",
        "constexpr", "const_cast", "continue", "decltype", "default", "delete",
        "do", "double", "dynamic_cast", "else", "enum", "explicit", "export",
        "extern", "false", "float", "for", "friend", "goto", "if", "inline",
        "int", "long", "mutable", "namespace", "new", "noexcept", "not", "not_eq",
        "nullptr", "operator", "or", "or_eq", "private", "protected", "public",
        "register", "reinterpret_cast", "return", "short", "signed", "sizeof",
        "static", "static_assert", "static_cast", "struct", "switch", "template",
        "this", "thread_local", "throw", "true", "try", "typedef", "typeid",
        "typename", "union", "unsigned", "using", "virtual", "void", "volatile",
        "wchar_t", "while", "xor", "xor_eq",
    ];

    if CPP_KEYWORDS.contains(&name) {
        return Err(CcgoError::config_error_with_hint(
            format!("Package name '{}' is a C++ reserved keyword", name),
            None,
            format!("Choose a different name, e.g., '{}_lib' or 'lib{}'", name, name),
        ).into());
    }

    // Valid C++ identifier pattern: starts with letter or underscore, followed by letters, digits, or underscores
    let valid_identifier = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    let valid_kebab_case = Regex::new(r"^[a-z][a-z0-9-]*[a-z0-9]$").unwrap();

    if !valid_identifier.is_match(name) && !valid_kebab_case.is_match(name) {
        return Err(CcgoError::config_error_with_hint(
            format!("Package name '{}' is not a valid identifier", name),
            None,
            "Package name must:\n\
             • Start with a letter or underscore\n\
             • Contain only letters, digits, underscores, or hyphens\n\
             • Examples: my_library, awesome-lib, LibFoo",
        ).into());
    }

    Ok(())
}

/// Validate semantic version string
fn validate_version(version: &str) -> Result<()> {
    crate::version::Version::parse(version).with_context(|| {
        CcgoError::version_error(
            format!("Invalid version string '{}'", version),
            "Version must follow semantic versioning (semver):\n\
             • Format: MAJOR.MINOR.PATCH (e.g., '1.0.0', '2.3.4')\n\
             • Optional pre-release: 1.0.0-alpha.1\n\
             • Optional build metadata: 1.0.0+20130313144700\n\
             \n\
             Examples: 1.0.0, 0.1.0-beta.2, 2.1.3+build.456",
        ).to_string()
    })?;
    Ok(())
}

/// Validate SPDX license identifier
fn validate_license(license: &str) -> Result<()> {
    // Common SPDX license identifiers
    const COMMON_LICENSES: &[&str] = &[
        "MIT", "Apache-2.0", "GPL-3.0", "GPL-2.0", "BSD-3-Clause", "BSD-2-Clause",
        "ISC", "MPL-2.0", "LGPL-3.0", "LGPL-2.1", "AGPL-3.0", "Unlicense",
        "CC0-1.0", "BSD-4-Clause", "BSL-1.0", "WTFPL",
    ];

    // This is just a warning for uncommon licenses
    if !COMMON_LICENSES.contains(&license) && !license.contains(" OR ") && !license.contains(" AND ") {
        eprintln!(
            "⚠️  Warning: '{}' is not a common SPDX license identifier.\n\
             Common licenses: MIT, Apache-2.0, GPL-3.0, BSD-3-Clause, etc.\n\
             See https://spdx.org/licenses/ for the full list.",
            license
        );
    }

    Ok(())
}

/// Validate workspace configuration
fn validate_workspace(workspace: &super::WorkspaceConfig, config: &CcgoConfig) -> Result<()> {
    // Ensure members list is not empty if it's a workspace-only config
    if config.package.is_none() && workspace.members.is_empty() {
        return Err(CcgoError::config_error_with_hint(
            "Workspace has no members",
            None,
            "Add workspace members in [workspace] section:\n\
             [workspace]\n\
             members = [\"crate1\", \"crate2\", \"libs/*\"]",
        ).into());
    }

    // Validate resolver version
    if workspace.resolver != "1" && workspace.resolver != "2" {
        return Err(CcgoError::config_error_with_hint(
            format!("Invalid resolver version '{}'", workspace.resolver),
            None,
            "Resolver must be \"1\" or \"2\":\n\
             • \"1\": Legacy resolver (default)\n\
             • \"2\": New resolver with better feature unification",
        ).into());
    }

    Ok(())
}

/// Validate dependency configuration
pub fn validate_dependency(dep: &DependencyConfig) -> Result<()> {
    // If workspace dependency, version is optional
    if dep.workspace {
        return Ok(());
    }

    // Ensure dependency has a valid source
    let has_version = !dep.version.is_empty();
    let has_git = dep.git.is_some();
    let has_path = dep.path.is_some();

    if !has_version && !has_git && !has_path {
        return Err(CcgoError::dependency_error_with_hint(
            &dep.name,
            "No valid source specified",
            "Dependency must have at least one of:\n\
             • version = \"1.0.0\" - for registry dependencies\n\
             • git = \"https://github.com/...\" - for git dependencies\n\
             • path = \"../my-lib\" - for local path dependencies",
        ).into());
    }

    // Validate version requirement if provided
    if has_version {
        crate::version::VersionReq::parse(&dep.version).with_context(|| {
            CcgoError::dependency_error_with_hint(
                &dep.name,
                format!("Invalid version requirement '{}'", dep.version),
                "Version requirements support:\n\
                 • Exact: \"1.2.3\" or \"=1.2.3\"\n\
                 • Range: \">=1.0.0, <2.0.0\"\n\
                 • Caret: \"^1.0.0\" (allows 1.x.x)\n\
                 • Tilde: \"~1.2.0\" (allows 1.2.x)\n\
                 • Wildcard: \"1.*\" or \"1.2.*\"",
            ).to_string()
        })?;
    }

    // Validate git source if provided
    if has_git {
        validate_git_dependency(dep)?;
    }

    // Validate path source if provided
    if has_path {
        validate_path_dependency(dep)?;
    }

    Ok(())
}

/// Validate git dependency
fn validate_git_dependency(dep: &DependencyConfig) -> Result<()> {
    let git_url = dep.git.as_ref().unwrap();

    // Check if git URL is valid (basic check)
    if !git_url.starts_with("http://") &&
       !git_url.starts_with("https://") &&
       !git_url.starts_with("git://") &&
       !git_url.starts_with("ssh://") &&
       !git_url.starts_with("git@") {
        return Err(CcgoError::dependency_error_with_hint(
            &dep.name,
            format!("Invalid git URL '{}'", git_url),
            "Git URL must start with:\n\
             • https:// (recommended)\n\
             • http://\n\
             • git://\n\
             • ssh://\n\
             • git@\n\
             \n\
             Example: https://github.com/user/repo.git",
        ).into());
    }

    // Check for conflicting git refs
    let mut ref_count = 0;
    if dep.branch.is_some() {
        ref_count += 1;
    }
    // Note: tag is not in DependencyConfig, so we skip this check
    // if dep.tag.is_some() {
    //     ref_count += 1;
    // }

    if ref_count > 1 {
        return Err(CcgoError::dependency_error_with_hint(
            &dep.name,
            "Multiple git refs specified",
            "Specify only ONE of:\n\
             • branch = \"main\"\n\
             • tag = \"v1.0.0\"\n\
             • rev = \"abc123\"",
        ).into());
    }

    Ok(())
}

/// Validate path dependency
fn validate_path_dependency(dep: &DependencyConfig) -> Result<()> {
    let path_str = dep.path.as_ref().unwrap();
    let path = Path::new(path_str);

    // Check if path is absolute (usually not recommended)
    if path.is_absolute() {
        eprintln!(
            "⚠️  Warning: Dependency '{}' uses absolute path '{}'\n\
             Absolute paths are not portable across machines.\n\
             Consider using a relative path instead.",
            dep.name, path_str
        );
    }

    Ok(())
}

/// Validate build configuration
fn validate_build_config(build: &super::BuildConfig) -> Result<()> {
    // Validate parallel jobs count
    if let Some(jobs) = build.jobs {
        if jobs == 0 {
            return Err(CcgoError::config_error_with_hint(
                "Build jobs cannot be 0",
                None,
                "Set jobs to a positive number (e.g., 4) or remove it to use default",
            ).into());
        }
        if jobs > 128 {
            eprintln!(
                "⚠️  Warning: Build jobs set to {}, which is very high.\n\
                 Recommended: 2-16 jobs depending on your CPU cores.",
                jobs
            );
        }
    }

    Ok(())
}

/// Validate platform configurations
fn validate_platform_configs(platforms: &super::PlatformConfigs) -> Result<()> {
    // Validate Android config
    if let Some(ref android) = platforms.android {
        if let Some(min_sdk) = android.min_sdk {
            if min_sdk < 16 {
                eprintln!(
                    "⚠️  Warning: Android minSdk {} is below 16 (Android 4.1).\n\
                     Many libraries don't support APIs below 21 (Android 5.0).",
                    min_sdk
                );
            }
        }
    }

    // Validate iOS config
    if let Some(ref ios) = platforms.ios {
        if let Some(ref min_version) = ios.min_version {
            // Parse version to check if it's valid
            let version_parts: Vec<&str> = min_version.split('.').collect();
            if version_parts.is_empty() {
                return Err(CcgoError::config_error_with_hint(
                    format!("Invalid iOS minimum version '{}'", min_version),
                    None,
                    "iOS version must be in format: \"12.0\" or \"14.0\"",
                ).into());
            }
        }
    }

    Ok(())
}

/// Validate binary configuration
fn validate_bin_config(bin: &super::BinConfig) -> Result<()> {
    validate_package_name(&bin.name)?;

    let path = Path::new(&bin.path);
    if !path.exists() {
        eprintln!(
            "⚠️  Warning: Binary source file '{}' for '{}' does not exist.\n\
             Expected path: {}",
            bin.path, bin.name, path.display()
        );
    }

    Ok(())
}

/// Validate example configuration
fn validate_example_config(example: &super::ExampleConfig) -> Result<()> {
    validate_package_name(&example.name)?;

    if let Some(ref path_str) = example.path {
        let path = Path::new(path_str);
        if !path.exists() {
            eprintln!(
                "⚠️  Warning: Example source file '{}' for '{}' does not exist.\n\
                 Expected path: {}",
                path_str, example.name, path.display()
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_package_name() {
        // Valid names
        assert!(validate_package_name("my_library").is_ok());
        assert!(validate_package_name("awesome-lib").is_ok());
        assert!(validate_package_name("LibFoo").is_ok());
        assert!(validate_package_name("_private").is_ok());

        // Invalid names
        assert!(validate_package_name("").is_err());
        assert!(validate_package_name("123start").is_err());
        assert!(validate_package_name("class").is_err()); // C++ keyword
        assert!(validate_package_name("my library").is_err()); // space
        assert!(validate_package_name("my.lib").is_err()); // dot
    }

    #[test]
    fn test_validate_version() {
        // Valid versions
        assert!(validate_version("1.0.0").is_ok());
        assert!(validate_version("0.1.0").is_ok());
        assert!(validate_version("2.3.4-alpha.1").is_ok());

        // Invalid versions
        assert!(validate_version("1").is_err());
        assert!(validate_version("1.0").is_err());
        assert!(validate_version("v1.0.0").is_err());
        assert!(validate_version("abc").is_err());
    }
}
