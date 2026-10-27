//! Resume a single interrupted yank without accepting unrelated dirty edits.
use super::YankCommand;
use crate::registry::{index_writer, PackageEntry, PackageIndex, VersionEntry};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, process::Command};

#[derive(Serialize, Deserialize)]
pub(super) struct PendingYank {
    repository: String,
    package: String,
    desired: VersionEntry,
    before_sha256: String,
    after_sha256: String,
    push: bool,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(path.parent().context("Missing parent")?)?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    Ok(())
}

impl PendingYank {
    pub fn start(
        path: &Path,
        repository: &str,
        package: &str,
        desired: VersionEntry,
        before: &[u8],
        after: &[u8],
        push: bool,
    ) -> Result<Self> {
        let pending = Self {
            repository: repository.to_owned(),
            package: package.to_owned(),
            desired,
            before_sha256: digest(before),
            after_sha256: digest(after),
            push,
        };
        write_atomic(
            &path.join(".git/ccgo-yank.json"),
            &serde_json::to_vec_pretty(&pending)?,
        )?;
        Ok(pending)
    }

    pub fn resume(
        path: &Path,
        command: &YankCommand,
        package: &str,
        verbose: bool,
    ) -> Result<Option<Self>> {
        let bytes = match fs::read(path.join(".git/ccgo-yank.json")) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let pending: Self = serde_json::from_slice(&bytes).context("Invalid pending yank")?;
        let repository = command
            .index_repo
            .as_deref()
            .context("Missing index repository")?;
        let version = command.vers.as_str();
        if pending.repository != repository
            || pending.package != package
            || pending.desired.version != version
            || pending.desired.yanked == command.undo
            || pending.desired.yanked_reason != command.reason
        {
            bail!("An interrupted yank is pending; retry its original repository, package, version, action and reason");
        }
        if pending.push && !command.push {
            bail!("Pending yank requires --push; retry the original command");
        }
        index_writer::verify_origin(path, repository)?;
        let status = Command::new("git")
            .current_dir(path)
            .args(["status", "--porcelain", "-z"])
            .output()?;
        if !status.status.success() {
            bail!("Failed to inspect pending yank");
        }
        let file = PackageIndex::package_index_path(package);
        for change in status
            .stdout
            .split(|b| *b == 0)
            .filter(|entry| !entry.is_empty())
        {
            let changed = std::str::from_utf8(change.get(3..).context("Invalid Git status")?)?;
            if changed != "index.json" && changed != file.to_string_lossy() {
                bail!("Unrelated local changes prevent yank recovery: {changed}; local changes are preserved");
            }
        }
        let current = fs::read(path.join(&file))?;
        let hash = digest(&current);
        if hash == pending.before_sha256 {
            let mut entry: PackageEntry = serde_json::from_slice(&current)?;
            let target = entry
                .versions
                .iter_mut()
                .find(|entry| entry.version == version)
                .context("Pending yank version is missing")?;
            *target = pending.desired.clone();
            let after = serde_json::to_vec_pretty(&entry)?;
            if digest(&after) != pending.after_sha256 {
                bail!("Pending yank snapshot mismatch");
            }
            write_atomic(&path.join(&file), &after)?;
        } else if !status.stdout.is_empty()
            && hash != pending.after_sha256
            && !index_writer::matches_pending_merge(path, &file.to_string_lossy(), &current)?
        {
            bail!("Pending yank package contains unrelated edits; local changes are preserved");
        }
        pending.verify_target(path)?;
        if !status.stdout.is_empty() || hash == pending.before_sha256 {
            index_writer::update_index_metadata(path, "index")?;
            index_writer::commit_changes(
                path,
                &format!("Resume yank: {package} {version}"),
                verbose,
            )?;
        }
        Ok(Some(pending))
    }

    pub fn verify_target(&self, path: &Path) -> Result<()> {
        let entry: PackageEntry = serde_json::from_slice(&fs::read(
            path.join(PackageIndex::package_index_path(&self.package)),
        )?)?;
        let target = entry
            .versions
            .iter()
            .find(|entry| entry.version == self.desired.version)
            .context("Pending yank version is missing")?;
        if serde_json::to_value(target)? != serde_json::to_value(&self.desired)? {
            bail!("Pending yank differs from the index; refusing to overwrite version history");
        }
        Ok(())
    }

    pub fn finish(&self, path: &Path) -> Result<()> {
        fs::remove_file(path.join(".git/ccgo-yank.json"))?;
        Ok(())
    }
}
