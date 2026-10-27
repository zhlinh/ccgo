//! Clone / pull a Git-based registry index into ~/.ccgo/registries/<name>/.
//!
//! Owns the on-disk cache and exposes a typed `lookup` for package entries.
//! No version-resolution logic here — that lives in `resolve.rs`.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

use super::cache_access::{home_lock, lock_directory, CacheLock};

use super::index::{PackageEntry, PackageIndex};

/// Typed wrapper around `~/.ccgo/registries/<name>/`.
///
/// Owns the local clone of a registry index repository and provides a
/// typed `lookup` for reading per-package JSON entries. Version resolution
/// (`resolve_dep`) is layered on top of this in a separate module.
pub struct RegistryCache {
    name: String,
    url: String,
    home: PathBuf,
}

impl RegistryCache {
    /// Construct a cache handle. No I/O happens here; call
    /// [`Self::ensure_synced`] before [`Self::lookup`].
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
            home: ccgo_home(),
        }
    }

    /// The configured upstream URL for this registry.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Local clone path: `<CCGO_HOME>/registries/<name>/`.
    pub fn local_path(&self) -> PathBuf {
        self.home.join("registries").join(&self.name)
    }

    /// Clone or update under an interprocess lock.
    pub fn ensure_synced(&self, verbose: bool) -> Result<()> {
        let _home = home_lock(&self.home, false)?;
        let _cache = self.lock()?;
        self.sync_locked(verbose)
    }

    pub(super) fn snapshot(&self, verbose: bool) -> Result<RegistrySnapshot> {
        let home = home_lock(&self.home, false)?;
        let _cache = self.lock()?;
        self.sync_locked(verbose)?;
        let path = self.local_path();
        let revision = git_text(&path, &["rev-parse", "--verify", "HEAD"])?;
        let token = tempfile::NamedTempFile::new_in(lock_directory(&self.home)?)?;
        let reference = format!(
            "refs/ccgo/snapshots/{}",
            token
                .path()
                .file_name()
                .context("Missing snapshot name")?
                .to_string_lossy()
                .trim_start_matches('.')
        );
        git_text(&path, &["update-ref", &reference, &revision])?;
        Ok(RegistrySnapshot {
            path,
            revision,
            reference,
            cache_lock: self.lock_path()?,
            _home: home,
            _token: token,
        })
    }

    fn lock_path(&self) -> Result<PathBuf> {
        if self.name.is_empty()
            || matches!(self.name.as_str(), "." | "..")
            || !self
                .name
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        {
            bail!("Invalid registry cache name: {}", self.name);
        }
        Ok(lock_directory(&self.home)?.join(format!("registry-{}.lock", self.name)))
    }

    fn lock(&self) -> Result<CacheLock> {
        CacheLock::acquire(&self.lock_path()?, true)
    }

    fn verify_origin(&self, path: &Path) -> Result<()> {
        if git_text(path, &["remote", "get-url", "origin"])? != self.url {
            bail!("Registry cache {} has a different origin; use a different registry name or clear that cache", path.display());
        }
        Ok(())
    }

    fn sync_locked(&self, verbose: bool) -> Result<()> {
        let path = self.local_path();
        let parent = path.parent().context("Missing registry cache parent")?;
        std::fs::create_dir_all(parent)?;
        if path.join(".git").is_dir() {
            self.verify_origin(&path)?;
            self.run_git(&["fetch", "--quiet", "origin"], &path, verbose)?;
            self.run_git(
                &["reset", "--hard", "--quiet", "FETCH_HEAD"],
                &path,
                verbose,
            )
            .with_context(|| format!("failed to reset registry '{}' to FETCH_HEAD", self.name))
        } else {
            // Publish only a complete clone; a failed clone leaves no partial cache.
            let staging = tempfile::tempdir_in(parent)?;
            let clone = staging.path().join("index");
            self.run_git(
                &[
                    "clone",
                    "--quiet",
                    &self.url,
                    clone.to_str().context("Non UTF-8 registry path")?,
                ],
                Path::new("."),
                verbose,
            )
            .with_context(|| {
                format!("failed to clone registry '{}' from {}", self.name, self.url)
            })?;
            std::fs::rename(&clone, &path).context("Failed to install registry clone")?;
            Ok(())
        }
    }

    /// Read the package's JSON entry. Returns `Ok(None)` when the file is
    /// absent. Returns `Err` on read or JSON-parse failures.
    pub fn lookup(&self, package: &str) -> Result<Option<PackageEntry>> {
        let _home = home_lock(&self.home, false)?;
        let _cache = self.lock()?;
        let rel = PackageIndex::package_index_path(package);
        let abs = self.local_path().join(&rel);
        match std::fs::read_to_string(&abs) {
            Ok(s) => {
                let entry: PackageEntry = serde_json::from_str(&s)
                    .with_context(|| format!("failed to parse {}", abs.display()))?;
                Ok(Some(entry))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).with_context(|| format!("failed to read {}", abs.display())),
        }
    }

    fn run_git(&self, args: &[&str], cwd: &Path, verbose: bool) -> Result<()> {
        let mut cmd = std::process::Command::new("git");
        cmd.args(args).current_dir(cwd);
        if verbose {
            eprintln!("[registry:{}] git {}", self.name, args.join(" "));
        }
        let status = cmd
            .status()
            .with_context(|| format!("failed to spawn git in {}", cwd.display()))?;
        if !status.success() {
            anyhow::bail!(
                "git {} exited with code {:?}",
                args.join(" "),
                status.code()
            );
        }
        Ok(())
    }
}

/// An immutable Git commit, retained by a temporary ref until the session ends.
/// Cache writers can advance HEAD without changing this reader's package view.
pub struct RegistrySnapshot {
    path: PathBuf,
    revision: String,
    reference: String,
    cache_lock: PathBuf,
    _home: CacheLock,
    _token: tempfile::NamedTempFile,
}

impl RegistrySnapshot {
    pub fn lookup(&self, package: &str) -> Result<Option<PackageEntry>> {
        let file = PackageIndex::package_index_path(package)
            .to_string_lossy()
            .replace('\\', "/");
        let object = format!("{}:{file}", self.revision);
        let output = std::process::Command::new("git")
            .current_dir(&self.path)
            .args(["show", &object])
            .output()?;
        if output.status.success() {
            return Ok(Some(
                serde_json::from_slice(&output.stdout)
                    .with_context(|| format!("failed to parse {object}"))?,
            ));
        }
        // A missing package is distinct from an unreadable or corrupt Git object.
        let files = git_text(
            &self.path,
            &[
                "ls-tree",
                "--name-only",
                &self.revision,
                "--",
                &file,
            ],
        )?;
        if files.is_empty() {
            return Ok(None);
        }
        bail!(
            "Failed to read registry object {object}: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    }
}

impl Drop for RegistrySnapshot {
    fn drop(&mut self) {
        if let Ok(_guard) = CacheLock::acquire(&self.cache_lock, true) {
            let _ = git_text(&self.path, &["update-ref", "-d", &self.reference]);
        }
    }
}

fn git_text(path: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .current_dir(path)
        .args(args)
        .output()?;
    if !output.status.success() {
        bail!(
            "git {} failed in {}: {}",
            args.join(" "),
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

/// Resolve `~/.ccgo/`, honoring `CCGO_HOME` for tests / overrides.
fn ccgo_home() -> PathBuf {
    if let Ok(h) = std::env::var("CCGO_HOME") {
        return PathBuf::from(h);
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .expect("HOME or USERPROFILE must be set");
    PathBuf::from(home).join(".ccgo")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run_git(args: &[&str], cwd: &Path) {
        let status = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .status()
            .unwrap();
        assert!(status.success(), "git {} failed", args.join(" "));
    }

    /// Build a synthetic upstream index repo containing a single `leaf` package.
    /// Path layout follows `PackageIndex::package_index_path("leaf")` so this
    /// fixture stays in lockstep with the production sharding rule.
    fn make_synthetic_index(parent: &Path, package_json: Option<&str>) -> PathBuf {
        let index_dir = parent.join("upstream-index");
        let leaf_rel = PackageIndex::package_index_path("leaf");
        let leaf_abs = index_dir.join(&leaf_rel);
        std::fs::create_dir_all(
            leaf_abs
                .parent()
                .expect("package_index_path always has a parent dir"),
        )
        .unwrap();

        run_git(&["init", "-q"], &index_dir);
        run_git(&["config", "user.email", "test@example.com"], &index_dir);
        run_git(&["config", "user.name", "ccgo-test"], &index_dir);

        let json = package_json.unwrap_or(
            r#"{"name":"leaf","description":"x","repository":"x","license":"MIT","platforms":[],"versions":[{"version":"1.0.0","tag":"v1.0.0"}]}"#,
        );
        std::fs::write(&leaf_abs, json).unwrap();

        run_git(&["add", "-A"], &index_dir);
        run_git(&["commit", "-q", "-m", "init", "--no-verify"], &index_dir);
        index_dir
    }

    #[test]
    #[serial_test::serial]
    fn ensure_synced_clones_when_absent() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());
        let upstream = make_synthetic_index(tmp.path(), None);
        let url = format!("file://{}", upstream.display());
        let cache = RegistryCache::new("test", url);
        cache.ensure_synced(false).unwrap();

        assert!(cache.local_path().join(".git").is_dir());
        let leaf_rel = PackageIndex::package_index_path("leaf");
        assert!(cache.local_path().join(&leaf_rel).is_file());
    }

    #[test]
    #[serial_test::serial]
    fn ensure_synced_pulls_when_already_cloned() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());
        let upstream = make_synthetic_index(tmp.path(), None);
        let url = format!("file://{}", upstream.display());
        let cache = RegistryCache::new("test", url);
        cache.ensure_synced(false).unwrap();
        cache.ensure_synced(false).unwrap();
    }

    #[test]
    #[serial_test::serial]
    fn lookup_returns_some_for_existing_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());
        let upstream = make_synthetic_index(tmp.path(), None);
        let url = format!("file://{}", upstream.display());
        let cache = RegistryCache::new("test", url);
        cache.ensure_synced(false).unwrap();

        let entry = cache.lookup("leaf").unwrap().expect("expected leaf entry");
        assert_eq!(entry.name, "leaf");
        assert_eq!(entry.versions.len(), 1);
    }

    #[test]
    #[serial_test::serial]
    fn lookup_returns_none_for_missing_package() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());
        let upstream = make_synthetic_index(tmp.path(), None);
        let url = format!("file://{}", upstream.display());
        let cache = RegistryCache::new("test", url);
        cache.ensure_synced(false).unwrap();

        assert!(cache.lookup("never-existed").unwrap().is_none());
    }

    #[test]
    #[serial_test::serial]
    fn lookup_returns_none_when_cache_not_yet_synced() {
        // The local clone directory doesn't exist at all. Lookup must
        // return Ok(None), not Err — callers (Task 4's resolve_dep) skip
        // missing entries and try the next registry, but propagate I/O
        // errors. Test pins this contract: a never-synced cache is
        // observably "no entry", not "cannot tell".
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());
        let cache = RegistryCache::new("test", "file:///never-cloned");
        assert!(cache.lookup("leaf").unwrap().is_none());
    }
    #[test]
    #[serial_test::serial]
    fn session_keeps_a_pinned_commit_and_new_session_sees_updates() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path().join("home"));
        let upstream = make_synthetic_index(tmp.path(), None);
        let cache = RegistryCache::new("test", format!("file://{}", upstream.display()));
        let mut session = super::super::RegistrySession::default();
        let before = session.synced(&cache).unwrap().revision.clone();
        let file = upstream.join(PackageIndex::package_index_path("leaf"));
        let changed = std::fs::read_to_string(&file)
            .unwrap()
            .replace("1.0.0", "2.0.0");
        std::fs::write(file, changed).unwrap();
        run_git(&["add", "-A"], &upstream);
        run_git(
            &["commit", "--amend", "-qm", "rewrite", "--no-verify"],
            &upstream,
        );
        let mut next = super::super::RegistrySession::default();
        assert_eq!(
            next.synced(&cache)
                .unwrap()
                .lookup("leaf")
                .unwrap()
                .unwrap()
                .versions[0]
                .version,
            "2.0.0"
        );
        // Even forced history replacement and pruning cannot remove a live snapshot.
        run_git(
            &["reflog", "expire", "--expire=now", "--all"],
            &cache.local_path(),
        );
        run_git(&["gc", "--prune=now"], &cache.local_path());
        let snapshot = session.synced(&cache).unwrap();
        assert_eq!(snapshot.revision, before);
        assert_eq!(
            snapshot.lookup("leaf").unwrap().unwrap().versions[0].version,
            "1.0.0"
        );
        assert!(snapshot.lookup("missing").unwrap().is_none());
        drop(session);
        drop(next);
        assert!(git_text(
            &cache.local_path(),
            &["for-each-ref", "refs/ccgo/snapshots/"]
        )
        .unwrap()
        .is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn failed_sync_is_retryable_and_registry_identity_is_checked() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path().join("home"));
        let upstream = tmp.path().join("upstream-index");
        let cache = RegistryCache::new("test", format!("file://{}", upstream.display()));
        let mut session = super::super::RegistrySession::default();
        assert!(session.synced(&cache).is_err());
        assert!(!cache.local_path().exists());
        make_synthetic_index(tmp.path(), None);
        assert!(session
            .synced(&cache)
            .unwrap()
            .lookup("leaf")
            .unwrap()
            .is_some());
        let changed_origin = RegistryCache::new("test", "file:///another-index");
        assert!(session
            .synced(&changed_origin)
            .err()
            .unwrap()
            .to_string()
            .contains("different origin"));
    }
    #[test]
    #[serial_test::serial]
    fn failed_fetch_can_retry_without_memoizing_the_old_snapshot() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path().join("home"));
        let upstream = make_synthetic_index(tmp.path(), None);
        let cache = RegistryCache::new("home", format!("file://{}", upstream.display()));
        cache.ensure_synced(false).unwrap();
        let moved = tmp.path().join("unavailable-index");
        std::fs::rename(&upstream, &moved).unwrap();
        let mut session = super::super::RegistrySession::default();
        assert!(session.synced(&cache).is_err());
        std::fs::rename(moved, upstream).unwrap();
        assert!(session
            .synced(&cache)
            .unwrap()
            .lookup("leaf")
            .unwrap()
            .is_some());
    }
}
