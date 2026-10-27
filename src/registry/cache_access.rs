//! Locks live outside CCGO_HOME so cache cleanup cannot replace an active lock.
use anyhow::{Context, Result};
use fs2::FileExt;
use std::{fs::File, path::Path};

pub(super) struct CacheLock(File);

impl CacheLock {
    pub(super) fn acquire(path: &Path, exclusive: bool) -> Result<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        if exclusive {
            FileExt::lock_exclusive(&file)?;
        } else {
            FileExt::lock_shared(&file)?;
        }
        Ok(Self(file))
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self.0);
    }
}

pub(super) fn lock_directory(home: &Path) -> Result<std::path::PathBuf> {
    let absolute = if home.is_absolute() {
        home.to_path_buf()
    } else {
        std::env::current_dir()?.join(home)
    };
    let identity = if absolute.exists() {
        absolute.canonicalize()?
    } else {
        let parent = absolute.parent().context("CCGO_HOME must have a parent")?;
        std::fs::create_dir_all(parent)?;
        parent
            .canonicalize()?
            .join(absolute.file_name().context("Invalid CCGO_HOME")?)
    };
    let parent = identity.parent().context("CCGO_HOME must have a parent")?;
    let directory = parent.join(".ccgo-registry-locks").join(format!(
        "{:x}",
        md5::compute(identity.as_os_str().as_encoded_bytes())
    ));
    std::fs::create_dir_all(&directory)?;
    Ok(directory)
}

pub(super) fn home_lock(home: &Path, exclusive: bool) -> Result<CacheLock> {
    CacheLock::acquire(&lock_directory(home)?.join("home.lock"), exclusive)
}

/// Coordinate `fetch --clean-cache` with live registry snapshots and cache writers.
pub fn clean_home(home: &Path) -> Result<()> {
    let _guard = home_lock(home, true)?;
    if home.exists() {
        std::fs::remove_dir_all(home).context("Failed to clean cache")?;
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn symlinked_home_uses_the_same_lock_identity() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().join("home");
        let alias = tmp.path().join("alias");
        std::fs::create_dir(&home).unwrap();
        std::os::unix::fs::symlink(&home, &alias).unwrap();
        assert_eq!(
            lock_directory(&home).unwrap(),
            lock_directory(&alias).unwrap()
        );
    }
}
