//! Registry retrieval anchored to an existing lock, independently of fresh resolution.

use std::path::Path;

use anyhow::{bail, Context, Result};

use super::FetchCommand;
use crate::config::{CcgoConfig, DependencyConfig};
use crate::lockfile::LockedPackage;
use crate::registry::{RegistryCache, ResolvedRegistryDep};

impl FetchCommand {
    pub(super) fn install_locked_registry(
        &self,
        dep: &DependencyConfig,
        config: &CcgoConfig,
        locked: &LockedPackage,
        install_path: &Path,
        ccgo_home: &Path,
    ) -> Result<LockedPackage> {
        let cache = locked_registry(dep, config, locked)?;
        if config
            .patch
            .find_patch(&dep.name, Some(&Self::build_source_string(dep)))
            .is_some()
        {
            bail!(
                "Patch for '{}' differs from its registry lock; run fetch without --locked",
                dep.name
            );
        }

        let revision = locked.git_revision();
        if let Some(rev) = revision {
            if !matches!(rev.len(), 40 | 64) || !rev.bytes().all(|b| b.is_ascii_hexdigit()) {
                bail!(
                    "Registry lock for '{}' has no full Git commit; run ccgo fetch --force without --locked to regenerate CCGO.lock",
                    dep.name
                );
            }
        } else {
            let checksum = locked.checksum.as_deref().unwrap_or_default();
            let digest = checksum.strip_prefix("sha256:").unwrap_or_default();
            if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                bail!(
                    "Registry lock for '{}' has no SHA-256 checksum; run ccgo fetch --force without --locked to regenerate CCGO.lock",
                    dep.name
                );
            }
        }

        cache
            .ensure_synced(false)
            .context("Failed to sync locked registry")?;
        let package = cache.lookup(&dep.name)?.with_context(|| {
            format!("Locked package '{}' is missing from its registry", dep.name)
        })?;
        // Yank excludes new selection, but must not invalidate an existing content lock.
        let version = package
            .versions
            .iter()
            .find(|entry| entry.version == locked.version)
            .with_context(|| {
                format!(
                    "Locked version '{} {}' is missing from its registry",
                    dep.name, locked.version
                )
            })?
            .clone();

        if let Some(rev) = revision {
            if package.repository.is_empty() {
                bail!("Locked Git package '{}' has no source repository", dep.name);
            }
            let installed = self.install_from_git(
                &dep.name,
                &locked.version,
                &package.repository,
                None,
                Some(rev),
                install_path,
                ccgo_home,
            )?;
            if installed.git_revision() != Some(rev) {
                bail!("Git commit mismatch for locked package '{}'", dep.name);
            }
        } else {
            if let Some(checksum) = version.checksum.as_deref() {
                if Some(checksum) != locked.checksum.as_deref() {
                    bail!(
                        "Index checksum changed for locked package '{} {}'",
                        dep.name,
                        locked.version
                    );
                }
            }
            let url = version
                .archive_url
                .clone()
                .with_context(|| format!("Locked archive '{}' has no download URL", dep.name))?;
            let mut resolved = ResolvedRegistryDep {
                registry_name: dep.registry.clone().unwrap_or_else(|| "locked".to_owned()),
                registry_url: cache.url().to_owned(),
                package_name: dep.name.clone(),
                package_repository: package.repository,
                version_entry: version,
            };
            // Missing or rewritten index metadata must never replace the lock's content anchor.
            resolved.version_entry.checksum = locked.checksum.clone();
            self.install_from_registry_archive(dep, &resolved, &url, install_path)?;
        }

        // Preserve timestamps, source spelling, dependency list and all other locked metadata.
        Ok(locked.clone())
    }
}

fn locked_registry(
    dep: &DependencyConfig,
    config: &CcgoConfig,
    locked: &LockedPackage,
) -> Result<RegistryCache> {
    if locked.name != dep.name || locked.version != dep.version {
        bail!(
            "Dependency '{}' differs from CCGO.lock; run fetch without --locked",
            dep.name
        );
    }
    let mut candidates: Vec<_> = match dep.registry.as_deref() {
        Some(name) => vec![(
            name,
            config.registries.get(name).with_context(|| {
                format!(
                    "Locked dependency '{}' references unknown registry '{name}'",
                    dep.name
                )
            })?,
        )],
        None => config
            .registries
            .iter()
            .map(|(name, url)| (name.as_str(), url))
            .collect(),
    };
    candidates.sort_by_key(|(name, _)| *name);

    let legacy = format!("registry+{}@{}", dep.name, dep.version);
    if locked.source == legacy {
        if candidates.len() != 1 {
            bail!("Legacy registry lock for '{}' has no unique registry identity; select a registry or regenerate CCGO.lock", dep.name);
        }
    } else {
        candidates.retain(|(_, url)| locked.source == format!("registry+{url}"));
    }
    let (name, url) = candidates.first().with_context(|| {
        format!(
            "Registry source changed for locked dependency '{}'; run fetch without --locked",
            dep.name
        )
    })?;
    Ok(RegistryCache::new(*name, (*url).clone()))
}
