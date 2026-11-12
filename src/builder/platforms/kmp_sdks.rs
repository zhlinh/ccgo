//! Isolated, validated C/C++ SDK inputs for the KMP Gradle build.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use crate::builder::BuildContext;

pub(super) struct NativeSdks {
    // Gradle must finish before the extracted SDK directories are removed.
    _staging: tempfile::TempDir,
    roots: BTreeMap<String, PathBuf>,
}

impl NativeSdks {
    pub fn load(ctx: &BuildContext) -> Result<Self> {
        let mut inputs = Self {
            _staging: tempfile::tempdir().context("Failed to stage KMP native SDKs")?,
            roots: BTreeMap::new(),
        };
        let mut configured = ctx
            .config
            .kmp
            .as_ref()
            .map(|kmp| kmp.native_sdks.clone())
            .unwrap_or_default();
        configured.extend(ctx.options.native_sdks.clone());
        for (platform, input) in &configured {
            if !["android", "ios", "macos", "linux", "windows"].contains(&platform.as_str()) {
                bail!("Unsupported [kmp.native_sdks] platform: {platform}");
            }
            let root = inputs._staging.path().join(platform);
            let input = ctx.project_root.join(input);
            extract_sdk(&input, &root)
                .with_context(|| format!("Failed to import {platform} SDK {}", input.display()))?;
            validate_sdk(ctx, platform, &root)?;
            inputs.roots.insert(platform.clone(), root);
        }
        Ok(inputs)
    }

    pub fn contains(&self, platform: &str) -> bool {
        self.roots.contains_key(platform)
    }

    pub fn roots(&self) -> &BTreeMap<String, PathBuf> {
        &self.roots
    }

    pub fn receipt_path(&self) -> PathBuf {
        self._staging.path().join("gradle-native-sdk.ready")
    }

    pub fn verify_gradle_support(&self) -> Result<()> {
        if !self.roots.is_empty() && !self.receipt_path().is_file() {
            bail!("The KMP Gradle project did not acknowledge imported native SDKs. Update ccgo-gradle-plugins to a version supporting ccgoNativeSdk.<platform>, or update the custom Gradle integration; refusing to package potentially missing native code.");
        }
        Ok(())
    }
}

fn extract_sdk(input: &Path, root: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(std::fs::File::open(input)?)?;
    let mut seen = HashSet::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .context("Unsafe native SDK ZIP path")?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("Symlinks are not supported in native SDK ZIPs");
        }
        if !seen.insert(relative.clone()) {
            bail!("Duplicate native SDK ZIP entry: {}", relative.display());
        }
        let path = root.join(relative);
        if entry.is_dir() {
            std::fs::create_dir_all(path)?;
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut std::fs::File::create(path)?)?;
        }
    }
    Ok(())
}

fn validate_sdk(ctx: &BuildContext, platform: &str, root: &Path) -> Result<()> {
    let info = root.join("meta").join(platform).join("build_info.json");
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&info)
            .with_context(|| format!("Missing SDK metadata {}", info.display()))?,
    )?;
    // Current archives use nested project/build objects; older SDKs use strings.
    let project = metadata["project"]["name"]
        .as_str()
        .or_else(|| metadata["project"].as_str());
    let version = metadata["project"]["version"]
        .as_str()
        .or_else(|| metadata["version"].as_str());
    let actual_platform = metadata["build"]["platform"]
        .as_str()
        .or_else(|| metadata["platform"].as_str());
    if !project.is_some_and(|name| name.eq_ignore_ascii_case(ctx.lib_name()))
        || version != Some(ctx.version())
        || !actual_platform.is_some_and(|name| name.eq_ignore_ascii_case(platform))
    {
        bail!(
            "Native SDK metadata must match {} {} ({platform}): {}",
            ctx.lib_name(),
            ctx.version(),
            info.display()
        );
    }
    if !root.join("include").is_dir() || !root.join("lib").join(platform).is_dir() {
        bail!("Native SDK for {platform} must contain include/ and lib/{platform}/");
    }
    Ok(())
}
