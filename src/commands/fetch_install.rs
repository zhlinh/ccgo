//! Prepare replacements without touching the installed dependency.
use std::{fs, path::Path};

use anyhow::{Context, Result};

const RECEIPT: &str = "transaction.json";

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    schema: u32,
    destination: String,
}

impl Receipt {
    fn validate(&self) -> Result<()> {
        let name = &self.destination;
        if self.schema != 1
            || name.is_empty()
            || name == "."
            || name == ".."
            || name.contains(['/', '\\', ':'])
            || name.starts_with(".ccgo-install-")
        {
            anyhow::bail!("Unsafe or unsupported installation recovery record");
        }
        Ok(())
    }
}

pub(super) fn lock_project(project: &Path) -> Result<crate::registry::cache_access::CacheLock> {
    let guard = crate::registry::cache_access::project_lock(project)?;
    recover(&project.join(".ccgo/deps"))?;
    Ok(guard)
}

fn present(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

fn write_receipt(staging: &Path, destination: &Path) -> Result<()> {
    let name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .context("Installation name must be UTF-8")?;
    let receipt = Receipt {
        schema: 1,
        destination: name.to_owned(),
    };
    receipt.validate()?;
    let mut file = tempfile::NamedTempFile::new_in(staging)?;
    use std::io::Write;
    file.write_all(&serde_json::to_vec(&receipt)?)?;
    file.as_file().sync_all()?;
    file.persist(staging.join(RECEIPT))?;
    Ok(())
}

fn recover(parent: &Path) -> Result<()> {
    if !parent.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(parent)? {
        let entry = entry?;
        if entry
            .file_name()
            .to_string_lossy()
            .starts_with(".ccgo-install-")
            && entry.file_type()?.is_dir()
            && entry.path().join(RECEIPT).is_file()
        {
            recover_one(parent, &entry.path())?;
        }
    }
    Ok(())
}

fn recover_one(parent: &Path, staging: &Path) -> Result<()> {
    let receipt: Receipt =
        serde_json::from_slice(&fs::read(staging.join(RECEIPT))?).with_context(|| {
            format!(
                "Invalid installation recovery record: {}",
                staging.display()
            )
        })?;
    receipt
        .validate()
        .with_context(|| format!("Invalid recovery state: {}", staging.display()))?;
    let destination = parent.join(&receipt.destination);
    let backup = staging.join("previous");
    if !present(&destination)? && present(&backup)? {
        fs::rename(&backup, &destination)
            .with_context(|| format!("Failed to restore {}", destination.display()))?;
    }
    // An existing destination is either the untouched old tree or published new tree.
    fs::remove_dir_all(staging).with_context(|| {
        format!(
            "Failed to clean recovered installation: {}",
            staging.display()
        )
    })?;
    Ok(())
}

pub(super) fn replace(destination: &Path, prepare: impl FnOnce(&Path) -> Result<()>) -> Result<()> {
    let parent = destination.parent().context("Installation has no parent")?;
    fs::create_dir_all(parent)?;
    let staging = tempfile::Builder::new()
        .prefix(".ccgo-install-")
        .tempdir_in(parent)?;
    write_receipt(staging.path(), destination)?;
    let candidate = staging.path().join("new");
    prepare(&candidate)?;
    // After this point, only explicit recovery may remove a backup.
    let staging = staging.keep();
    let result = publish(destination, &candidate, &staging.join("previous"));
    if let Err(error) = recover_one(parent, &staging) {
        anyhow::bail!("Installation recovery failed: {error:#}; retained state at {} (publication: {result:?})", staging.display());
    }
    result
}

fn publish(destination: &Path, candidate: &Path, backup: &Path) -> Result<()> {
    if present(destination)? {
        fs::rename(destination, backup).context("Failed to back up installed dependency")?;
    }
    #[cfg(test)]
    crash_checkpoint("backed-up");
    fs::rename(candidate, destination).context("Failed to publish prepared dependency")?;
    #[cfg(test)]
    crash_checkpoint("published");
    Ok(())
}

#[cfg(test)]
fn crash_checkpoint(phase: &str) {
    if std::env::var("CCGO_TEST_INSTALL_CRASH_PHASE").as_deref() == Ok(phase) {
        // exit deliberately skips TempDir and lock destructors in the child test process.
        std::process::exit(75);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_worker() {
        let Some(destination) = std::env::var_os("CCGO_TEST_INSTALL_DESTINATION") else {
            return;
        };
        replace(Path::new(&destination), |candidate| {
            fs::create_dir(candidate)?;
            fs::write(candidate.join("payload"), "replacement")?;
            crash_checkpoint("prepared");
            Ok(())
        })
        .unwrap();
        panic!("child must exit at the requested checkpoint");
    }

    fn check_process_exit(phase: &str, expected: &str) {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        let destination = project.join(".ccgo/deps/leaf");
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("payload"), "original").unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "commands::fetch::installation::tests::crash_worker",
            ])
            .env("CCGO_TEST_INSTALL_DESTINATION", &destination)
            .env("CCGO_TEST_INSTALL_CRASH_PHASE", phase)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(75), "{output:?}");
        let guard = lock_project(project).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("payload")).unwrap(),
            expected
        );
        assert_eq!(fs::read_dir(project.join(".ccgo/deps")).unwrap().count(), 1);
        drop(guard);
        let _guard = lock_project(project).unwrap();
        assert_eq!(
            fs::read_to_string(destination.join("payload")).unwrap(),
            expected
        );
    }

    #[test]
    fn process_exit_during_preparation_keeps_old_installation() {
        check_process_exit("prepared", "original");
    }

    #[test]
    fn process_exit_after_backup_restores_old_installation() {
        check_process_exit("backed-up", "original");
    }

    #[test]
    fn process_exit_after_publication_keeps_complete_replacement() {
        check_process_exit("published", "replacement");
    }

    #[test]
    fn invalid_receipt_cannot_escape_dependency_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let parent = tmp.path().join("deps");
        let staging = parent.join(".ccgo-install-invalid");
        fs::create_dir_all(staging.join("previous")).unwrap();
        fs::write(
            staging.join(RECEIPT),
            br#"{"schema":1,"destination":"../outside"}"#,
        )
        .unwrap();
        assert!(recover(&parent).is_err());
        assert!(staging.join("previous").is_dir());
        assert!(!tmp.path().join("outside").exists());
    }

    #[test]
    fn preparation_failure_preserves_old_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("leaf");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("payload"), "original").unwrap();
        let result = replace(&destination, |candidate| {
            fs::create_dir(candidate)?;
            fs::write(candidate.join("partial"), "incomplete")?;
            anyhow::bail!("preparation failed")
        });
        assert!(result.is_err());
        assert_eq!(
            fs::read_to_string(destination.join("payload")).unwrap(),
            "original"
        );
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_publication_restores_old_tree() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("leaf");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("payload"), "original").unwrap();
        // A missing candidate forces rename failure after the backup was made.
        assert!(replace(&destination, |_| Ok(())).is_err());
        assert_eq!(
            fs::read_to_string(destination.join("payload")).unwrap(),
            "original"
        );
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[test]
    fn successful_replacement_removes_stale_files() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("leaf");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("stale"), "old").unwrap();
        replace(&destination, |candidate| {
            fs::create_dir(candidate)?;
            fs::write(candidate.join("payload"), "replacement")?;
            Ok(())
        })
        .unwrap();
        assert!(!destination.join("stale").exists());
        assert_eq!(
            fs::read_to_string(destination.join("payload")).unwrap(),
            "replacement"
        );
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn failed_publication_restores_dangling_symlink_without_following_it() {
        let tmp = tempfile::tempdir().unwrap();
        let destination = tmp.path().join("leaf");
        let missing = tmp.path().join("missing");
        std::os::unix::fs::symlink(&missing, &destination).unwrap();
        assert!(replace(&destination, |_| Ok(())).is_err());
        assert_eq!(fs::read_link(&destination).unwrap(), missing);
        replace(&destination, |candidate| {
            fs::create_dir(candidate)?;
            fs::write(candidate.join("payload"), "complete")?;
            Ok(())
        })
        .unwrap();
        assert!(!destination.is_symlink());
        assert!(!missing.exists());
        assert_eq!(
            fs::read_to_string(destination.join("payload")).unwrap(),
            "complete"
        );
    }
}
