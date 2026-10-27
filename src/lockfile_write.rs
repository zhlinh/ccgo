//! Replace lockfile contents without exposing a truncated destination.
use std::{fs, io::Write, path::Path};

use anyhow::{Context, Result};

pub(super) fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    // Preserve existing live symlink semantics; a dangling link fails without mutation.
    let target = if path.is_symlink() {
        path.canonicalize()
            .context("Failed to resolve lockfile symlink")?
    } else {
        path.to_path_buf()
    };
    let parent = target
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let permissions = match fs::metadata(&target) {
        Ok(metadata) => Some(metadata.permissions()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.into()),
    };
    let mut builder = tempfile::Builder::new();
    builder.prefix(".ccgo-lock-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(
            permissions
                .clone()
                .unwrap_or_else(|| fs::Permissions::from_mode(0o666)),
        );
    }
    let mut temporary = builder.tempfile_in(parent)?;
    temporary.write_all(contents)?;
    if let Some(permissions) = permissions {
        temporary.as_file().set_permissions(permissions)?;
    }
    temporary.as_file().sync_all()?;
    #[cfg(test)]
    checkpoint("prepared");
    temporary
        .persist(&target)
        .context("Failed to replace lockfile")?;
    #[cfg(test)]
    checkpoint("published");
    Ok(())
}

#[cfg(test)]
fn checkpoint(phase: &str) {
    if std::env::var("CCGO_TEST_LOCK_WRITE_EXIT").as_deref() == Ok(phase) {
        std::process::exit(75);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_worker() {
        let Some(path) = std::env::var_os("CCGO_TEST_LOCK_WRITE_PATH") else {
            return;
        };
        atomic_write(Path::new(&path), b"complete replacement").unwrap();
        panic!("child must exit at checkpoint");
    }

    fn check_interruption(phase: &str, initial: Option<&[u8]>, expected: Option<&[u8]>) {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CCGO.lock");
        if let Some(initial) = initial {
            fs::write(&path, initial).unwrap();
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "lockfile::write::tests::crash_worker"])
            .current_dir(tmp.path())
            .env("CCGO_TEST_LOCK_WRITE_PATH", "CCGO.lock")
            .env("CCGO_TEST_LOCK_WRITE_EXIT", phase)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(75), "{output:?}");
        assert_eq!(fs::read(&path).ok().as_deref(), expected);
        atomic_write(&path, b"retry").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"retry");
    }

    #[test]
    fn interruption_before_replacement_preserves_old_bytes() {
        check_interruption("prepared", Some(b"old lock"), Some(b"old lock"));
    }

    #[test]
    fn interruption_before_first_publication_leaves_destination_absent() {
        check_interruption("prepared", None, None);
    }

    #[test]
    fn interruption_after_replacement_leaves_complete_bytes() {
        check_interruption(
            "published",
            Some(b"old lock"),
            Some(b"complete replacement"),
        );
    }

    #[test]
    fn failed_publication_preserves_existing_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("CCGO.lock");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("sentinel"), b"original").unwrap();
        assert!(atomic_write(&path, b"new").is_err());
        assert_eq!(fs::read(path.join("sentinel")).unwrap(), b"original");
        assert_eq!(fs::read_dir(tmp.path()).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn replacement_preserves_file_mode_and_live_symlink() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("actual.lock");
        let link = tmp.path().join("CCGO.lock");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        std::os::unix::fs::symlink("actual.lock", &link).unwrap();
        atomic_write(&link, b"new").unwrap();
        assert!(link.is_symlink());
        assert_eq!(fs::read(&path).unwrap(), b"new");
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[cfg(unix)]
    #[test]
    fn first_write_respects_normal_file_creation_mode() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let normal = tmp.path().join("normal");
        let lock = tmp.path().join("CCGO.lock");
        fs::write(&normal, b"normal").unwrap();
        atomic_write(&lock, b"lock").unwrap();
        assert_eq!(
            fs::metadata(normal).unwrap().permissions().mode() & 0o777,
            fs::metadata(lock).unwrap().permissions().mode() & 0o777,
        );
    }
}
