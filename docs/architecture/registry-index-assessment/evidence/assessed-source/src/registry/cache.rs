//! Clone / pull a Git-based registry index into ~/.ccgo/registries/<name>/.
//!
//! Owns the on-disk cache and exposes a typed `lookup` for package entries.
//! No version-resolution logic here — that lives in `resolve.rs`.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::index::{PackageEntry, PackageIndex};

/// Typed wrapper around `~/.ccgo/registries/<name>/`.
///
/// Owns the local clone of a registry index repository and provides a
/// typed `lookup` for reading per-package JSON entries. Version resolution
/// (`resolve_dep`) is layered on top of this in a separate module.
pub struct RegistryCache {
    name: String,
    url: String,
}

impl RegistryCache {
    /// Construct a cache handle. No I/O happens here; call
    /// [`Self::ensure_synced`] before [`Self::lookup`].
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            url: url.into(),
        }
    }

    /// The configured upstream URL for this registry.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Local clone path: `<CCGO_HOME>/registries/<name>/`.
    pub fn local_path(&self) -> PathBuf {
        ccgo_home().join("registries").join(&self.name)
    }

    /// Clone if absent, fetch + hard-reset if present. Idempotent.
    pub fn ensure_synced(&self, verbose: bool) -> Result<()> {
        let path = self.local_path();
        std::fs::create_dir_all(path.parent().expect("registries path always has a parent"))
            .with_context(|| format!("failed to create registries dir for '{}'", self.name))?;

        if path.join(".git").is_dir() {
            self.run_git(&["fetch", "--quiet", "origin"], &path, verbose)?;
            self.run_git(
                &["reset", "--hard", "--quiet", "FETCH_HEAD"],
                &path,
                verbose,
            )
            .with_context(|| format!("failed to reset registry '{}' to FETCH_HEAD", self.name))
        } else {
            self.run_git(
                &[
                    "clone",
                    "--quiet",
                    &self.url,
                    path.to_str().expect("local registry path is UTF-8"),
                ],
                Path::new("."),
                verbose,
            )
            .with_context(|| format!("failed to clone registry '{}' from {}", self.name, self.url))
        }
    }

    /// Read the package's JSON entry. Returns `Ok(None)` when the file is
    /// absent. Returns `Err` on read or JSON-parse failures.
    pub fn lookup(&self, package: &str) -> Result<Option<PackageEntry>> {
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
}
