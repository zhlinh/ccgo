//! Durable intent for resuming one interrupted publication per working clone.
use crate::registry::{index_writer, PackageIndex, VersionEntry};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, process::Command};

#[derive(Serialize, Deserialize)]
pub(super) struct Publication {
    package: String,
    repository: String,
    pub version: VersionEntry,
    pub uploaded: bool,
}

fn same_intent(left: &VersionEntry, right: &VersionEntry) -> Result<bool> {
    let mut left = serde_json::to_value(left)?;
    let mut right = serde_json::to_value(right)?;
    left.as_object_mut().unwrap().remove("released_at");
    right.as_object_mut().unwrap().remove("released_at");
    Ok(left == right)
}

impl Publication {
    pub fn resume(
        path: &Path,
        package: &str,
        repository: &str,
        version: &VersionEntry,
        verbose: bool,
    ) -> Result<Option<Self>> {
        let file = path.join(".git/ccgo-publication.json");
        let bytes = match fs::read(&file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let pending: Self =
            serde_json::from_slice(&bytes).context("Invalid pending publication")?;
        if pending.package != package
            || pending.repository != repository
            || !same_intent(&pending.version, version)?
        {
            bail!("An interrupted publication is pending in {}; retry its original package, version and artifact before publishing another", file.display());
        }
        let status = Command::new("git")
            .current_dir(path)
            .args(["status", "--porcelain", "-z"])
            .output()?;
        if !status.status.success() {
            bail!("Failed to inspect interrupted publication");
        }
        if !status.stdout.is_empty() {
            let package_file = PackageIndex::package_index_path(package);
            let package_file = package_file.to_string_lossy();
            for change in status
                .stdout
                .split(|b| *b == 0)
                .filter(|entry| !entry.is_empty())
            {
                let file = std::str::from_utf8(change.get(3..).context("Invalid Git status")?)?;
                if file != "index.json" && file != package_file {
                    bail!("Unrelated local changes prevent publication recovery: {file}; local changes are preserved");
                }
            }
            pending.verify_entry(path)?;
            if !pending.uploaded {
                bail!(
                    "Pending index changes have no completed upload; local changes are preserved"
                );
            }
            index_writer::commit_changes(
                path,
                &format!("Resume {} {}", package, version.version),
                verbose,
            )?;
        }
        Ok(Some(pending))
    }

    pub fn start(
        path: &Path,
        package: &str,
        repository: &str,
        version: VersionEntry,
    ) -> Result<Self> {
        let pending = Self {
            package: package.to_owned(),
            repository: repository.to_owned(),
            version,
            uploaded: false,
        };
        pending.save(path)?;
        Ok(pending)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        use std::io::Write;
        let mut temporary = tempfile::NamedTempFile::new_in(path.join(".git"))?;
        temporary.write_all(&serde_json::to_vec_pretty(self)?)?;
        temporary.as_file().sync_all()?;
        temporary.persist(path.join(".git/ccgo-publication.json"))?;
        Ok(())
    }

    pub fn verify_entry(&self, path: &Path) -> Result<()> {
        let value: crate::registry::PackageEntry = serde_json::from_slice(&fs::read(
            path.join(PackageIndex::package_index_path(&self.package)),
        )?)?;
        let version = value
            .versions
            .iter()
            .find(|version| version.version == self.version.version)
            .context("Interrupted publication version is missing")?;
        if serde_json::to_value(version)? != serde_json::to_value(&self.version)? {
            bail!("Pending publication differs from the index; refusing to overwrite its history");
        }
        Ok(())
    }

    pub fn finish(&self, path: &Path) -> Result<()> {
        fs::remove_file(path.join(".git/ccgo-publication.json"))?;
        Ok(())
    }
}
