//! Serialize local writers and reconcile version records after a rejected push.
use super::{commit_changes, update_index_metadata};
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use serde_json::Value;
use std::{collections::BTreeMap, fs, path::Path, process::Command};

pub struct IndexWriteLock(fs::File);

impl Drop for IndexWriteLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

/// Keep the file (and its inode) permanently: removing it would race waiters.
pub fn lock_index(directory: &Path) -> Result<IndexWriteLock> {
    let parent = directory
        .parent()
        .context("Index directory has no parent")?;
    fs::create_dir_all(parent)?;
    let canonical = if directory.exists() {
        fs::canonicalize(directory)?
    } else {
        fs::canonicalize(parent)?.join(directory.file_name().context("Invalid index path")?)
    };
    let lock_dir = canonical
        .parent()
        .context("Invalid index path")?
        .join(".ccgo-write-locks");
    fs::create_dir_all(&lock_dir)?;
    let lock_path = lock_dir.join(format!(
        "{:x}.lock",
        md5::compute(canonical.to_string_lossy().as_bytes())
    ));
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(lock_path)?;
    file.try_lock_exclusive().with_context(|| {
        format!(
            "Another publish or yank is using {}; retry after it finishes",
            directory.display()
        )
    })?;
    Ok(IndexWriteLock(file))
}

fn git(path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if !output.status.success() {
        bail!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn package_at(path: &Path, revision: &str, file: &str) -> Result<Option<Value>> {
    let spec = format!("{revision}:{file}");
    let exists = Command::new("git")
        .current_dir(path)
        .args(["cat-file", "-e", &spec])
        .output()?;
    if !exists.status.success() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&git(path, &["show", &spec])?)?))
}

fn versions(value: Option<&Value>) -> Result<BTreeMap<String, Value>> {
    let mut result = BTreeMap::new();
    if let Some(value) = value {
        for entry in value
            .get("versions")
            .and_then(Value::as_array)
            .context("Invalid index versions")?
        {
            let version = entry
                .get("version")
                .and_then(Value::as_str)
                .context("Invalid index version")?;
            if result.insert(version.to_owned(), entry.clone()).is_some() {
                bail!("Duplicate index version {version}");
            }
        }
    }
    Ok(result)
}

/// Apply only this writer's changes to the remote package, preserving other versions.
fn merge_package(base: Option<&Value>, local: &Value, remote: Option<&Value>) -> Result<Value> {
    let before = versions(base)?;
    let after = versions(Some(local))?;
    let mut merged = versions(remote)?;
    for version in before.keys() {
        if !after.contains_key(version) {
            bail!("Index history deletion is not supported: {version}");
        }
        if !merged.contains_key(version) {
            bail!("Remote index history is missing version {version}");
        }
    }
    for (version, value) in &after {
        if before.get(version) == Some(value) {
            continue;
        }
        if let Some(original) = before.get(version) {
            verify_immutable_version(original, value, version)?;
        }
        let existing = merged.get(version);
        if existing != before.get(version) && existing != Some(value) {
            bail!("Concurrent index change for version {version}; refusing to overwrite history");
        }
        merged.insert(version.clone(), value.clone());
    }
    let mut result = remote.unwrap_or(local).clone();
    let fields = local.as_object().context("Invalid package entry")?;
    for (key, value) in fields.iter().filter(|(key, _)| key.as_str() != "versions") {
        if base.and_then(|b| b.get(key)) == Some(value) {
            continue;
        }
        let existing = remote.and_then(|r| r.get(key));
        if existing != base.and_then(|b| b.get(key)) && existing != Some(value) {
            bail!("Concurrent package metadata change for {key}");
        }
        result[key] = value.clone();
    }
    result["versions"] = Value::Array(merged.into_iter().rev().map(|(_, value)| value).collect());
    Ok(result)
}

fn verify_immutable_version(original: &Value, value: &Value, version: &str) -> Result<()> {
    let mut original = original.clone();
    let mut intended = value.clone();
    for record in [&mut original, &mut intended] {
        let fields = record.as_object_mut().context("Invalid version record")?;
        fields.remove("yanked");
        fields.remove("yanked_reason");
    }
    if original != intended {
        bail!("Immutable version metadata changed for {version}");
    }
    Ok(())
}

/// Validate a dirty package against both parents of an interrupted merge.
pub fn matches_pending_merge(path: &Path, file: &str, bytes: &[u8]) -> Result<bool> {
    let remote = match git(path, &["rev-parse", "--verify", "MERGE_HEAD"]) {
        Ok(remote) => remote,
        Err(_) => return Ok(false),
    };
    let local = git(path, &["rev-parse", "HEAD"])?;
    let base = git(path, &["merge-base", &local, &remote])?;
    let before = package_at(path, &base, file)?;
    let intended = package_at(path, &local, file)?.context("Missing local merge package")?;
    let latest = package_at(path, &remote, file)?;
    let expected = merge_package(before.as_ref(), &intended, latest.as_ref())?;
    Ok(serde_json::from_slice::<Value>(bytes)? == expected)
}

/// Fetch and merge remote history without rebasing JSON text or discarding local commits.
/// Returns whether HEAD advanced. Callers must hold the index write lock.
pub fn reconcile_remote(path: &Path, name: &str, verbose: bool) -> Result<bool> {
    if !git(path, &["status", "--porcelain"])?.is_empty() {
        bail!("Index has uncommitted changes; resolve or resume them before updating. Local changes are preserved");
    }
    git(path, &["fetch", "origin"])?;
    let remote = match git(path, &["rev-parse", "--verify", "FETCH_HEAD"]) {
        Ok(head) => head,
        Err(_) => return Ok(false), // Empty remote: the first push creates its branch.
    };
    let local = git(path, &["rev-parse", "HEAD"])?;
    if local == remote {
        return Ok(false);
    }
    let base = git(path, &["merge-base", &local, &remote])?;
    if base == remote {
        return Ok(false); // Pending local commits, including a previous rejected push.
    }
    if base == local {
        git(path, &["merge", "--ff-only", &remote])?;
        return Ok(true);
    }

    let (metadata, replacements) = plan_merge(path, &base, &local, &remote)?;
    apply_merge(path, name, &remote, metadata, replacements, verbose)?;
    Ok(true)
}

type MergePlan = (Option<Value>, Vec<(String, Value)>);

fn plan_merge(path: &Path, base: &str, local: &str, remote: &str) -> Result<MergePlan> {
    let changed = git(path, &["diff", "--name-only", base, local])?;
    let metadata =
        package_at(path, remote, "index.json")?.or(package_at(path, local, "index.json")?);
    let mut replacements = Vec::new();
    for file in changed.lines().filter(|file| *file != "index.json") {
        if !file.ends_with(".json") || file.starts_with('.') {
            bail!("Cannot automatically reconcile local index file {file}; local commits are preserved");
        }
        let original = package_at(path, base, file)?;
        let intended =
            package_at(path, local, file)?.context("Cannot remove a package from the index")?;
        let latest = package_at(path, remote, file)?;
        let merged = merge_package(original.as_ref(), &intended, latest.as_ref())
            .with_context(|| format!("Cannot reconcile {file}; local commits are preserved"))?;
        replacements.push((file.to_owned(), merged));
    }

    Ok((metadata, replacements))
}

fn apply_merge(
    path: &Path,
    name: &str,
    remote: &str,
    metadata: Option<Value>,
    replacements: Vec<(String, Value)>,
    verbose: bool,
) -> Result<()> {
    let output = Command::new("git")
        .current_dir(path)
        .args(["merge", "--no-commit", "--no-ff", remote])
        .output()?;
    if !output.status.success() {
        let conflicts = git(path, &["diff", "--name-only", "--diff-filter=U"])?;
        if conflicts.is_empty()
            || conflicts.lines().any(|file| {
                file != "index.json" && !replacements.iter().any(|(known, _)| known == file)
            })
        {
            let _ = git(path, &["merge", "--abort"]);
            bail!(
                "Cannot merge index: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    for (file, value) in replacements {
        fs::write(path.join(file), serde_json::to_string_pretty(&value)?)?;
    }
    if let Some(metadata) = metadata {
        fs::write(
            path.join("index.json"),
            serde_json::to_string_pretty(&metadata)?,
        )?;
    }
    update_index_metadata(path, name)?;
    // Both original histories remain parents of the reconciliation commit.
    commit_changes(path, "Merge concurrent registry updates", verbose)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn distinct_versions_merge_and_conflicting_changes_fail() {
        let base = json!({"name":"demo", "versions":[{"version":"1", "yanked":false}]});
        let mut local = base.clone();
        local["versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"version":"2"}));
        let mut remote = base.clone();
        remote["versions"][0]["yanked"] = true.into();
        remote["versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"version":"3"}));
        let merged = merge_package(Some(&base), &local, Some(&remote)).unwrap();
        assert_eq!(merged["versions"].as_array().unwrap().len(), 3);
        assert_eq!(merged["versions"][2]["yanked"], true);
        remote["versions"]
            .as_array_mut()
            .unwrap()
            .push(json!({"version":"2", "tag":"other"}));
        assert!(merge_package(Some(&base), &local, Some(&remote)).is_err());
    }

    #[test]
    fn lock_is_exclusive_and_released_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let guard = lock_index(dir.path()).unwrap();
        assert!(lock_index(dir.path()).is_err());
        drop(guard);
        assert!(lock_index(dir.path()).is_ok());
    }
}
