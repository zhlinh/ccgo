//! Platform-specific build implementations
//!
//! Each platform has its own builder that implements the `PlatformBuilder` trait.
//! The builders handle:
//! - CMake configuration with platform-specific toolchains
//! - Multi-architecture builds
//! - Library packaging (frameworks, AARs, HARs, etc.)
//! - Archive creation

pub mod android;
pub mod benches;
pub mod conan;
pub mod ios;
pub mod kmp;
pub mod linux;
pub mod macos;
pub mod ohos;
pub mod tests;
pub mod tvos;
pub mod watchos;
pub mod windows;

use anyhow::{bail, Result};

use super::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::BuildTarget;

/// Get the appropriate platform builder for the target
pub fn get_builder(target: &BuildTarget) -> Result<Box<dyn PlatformBuilder>> {
    match target {
        BuildTarget::Linux => Ok(Box::new(linux::LinuxBuilder::new())),
        BuildTarget::Macos => Ok(Box::new(macos::MacosBuilder::new())),
        BuildTarget::Windows => Ok(Box::new(windows::WindowsBuilder::new())),
        BuildTarget::Ios => Ok(Box::new(ios::IosBuilder::new())),
        BuildTarget::Android => Ok(Box::new(android::AndroidBuilder::new())),
        BuildTarget::Ohos => Ok(Box::new(ohos::OhosBuilder::new())),
        BuildTarget::Tvos => Ok(Box::new(tvos::TvosBuilder::new())),
        BuildTarget::Watchos => Ok(Box::new(watchos::WatchosBuilder::new())),
        BuildTarget::Kmp => Ok(Box::new(kmp::KmpBuilder::new())),
        BuildTarget::Conan => Ok(Box::new(conan::ConanBuilder::new())),
        BuildTarget::All => bail!("Use build_all() for building all platforms"),
        BuildTarget::Apple => bail!("Use build_apple() for building all Apple platforms"),
    }
}

/// Build all platforms in parallel
pub fn build_all(ctx: &BuildContext) -> Result<Vec<BuildResult>> {
    use rayon::prelude::*;

    let platforms = vec![
        BuildTarget::Linux,
        BuildTarget::Macos,
        BuildTarget::Windows,
        BuildTarget::Ios,
        BuildTarget::Android,
        BuildTarget::Ohos,
        BuildTarget::Tvos,
        BuildTarget::Watchos,
    ];

    let results: Vec<Result<BuildResult>> = platforms
        .par_iter()
        .map(|target| {
            let builder = get_builder(target)?;
            let platform_ctx = BuildContext::new(
                ctx.project_root.clone(),
                ctx.config.clone(),
                super::BuildOptions {
                    target: target.clone(),
                    ..ctx.options.clone()
                },
            );
            builder.build(&platform_ctx)
        })
        .collect();

    // Collect successful results and report errors
    let mut successful = Vec::new();
    let mut errors = Vec::new();

    for (idx, result) in results.into_iter().enumerate() {
        match result {
            Ok(r) => successful.push(r),
            Err(e) => errors.push((platforms[idx].to_string(), e)),
        }
    }

    if !errors.is_empty() {
        eprintln!("Some platforms failed to build:");
        for (platform, error) in &errors {
            eprintln!("  {}: {}", platform, error);
        }
        if successful.is_empty() {
            bail!("All platforms failed to build");
        }
    }

    Ok(successful)
}

/// Build all Apple platforms (iOS, macOS, tvOS, watchOS)
pub fn build_apple(ctx: &BuildContext) -> Result<Vec<BuildResult>> {
    use rayon::prelude::*;

    let platforms = vec![
        BuildTarget::Ios,
        BuildTarget::Macos,
        BuildTarget::Tvos,
        BuildTarget::Watchos,
    ];

    let results: Vec<Result<BuildResult>> = platforms
        .par_iter()
        .map(|target| {
            let builder = get_builder(target)?;
            let platform_ctx = BuildContext::new(
                ctx.project_root.clone(),
                ctx.config.clone(),
                super::BuildOptions {
                    target: target.clone(),
                    ..ctx.options.clone()
                },
            );
            builder.build(&platform_ctx)
        })
        .collect();

    let mut successful = Vec::new();
    for result in results {
        successful.push(result?);
    }

    Ok(successful)
}
