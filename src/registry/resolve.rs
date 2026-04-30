//! Pure version-resolution against pre-populated registry caches.
//!
//! `resolve_dep` is I/O-free at the registry-network level — it only reads
//! from local caches. Callers MUST `ensure_synced` each cache before passing
//! them in.

use anyhow::Result;

use super::cache::RegistryCache;
use super::index::VersionEntry;

/// A successful version resolution: the registry it came from, the package
/// name, the matched [`VersionEntry`], and the package's source git URL.
///
/// `package_repository` is propagated from `PackageEntry.repository` so the
/// fetch path can fall back to `git clone --branch <tag>` when the
/// `VersionEntry` has no `archive_url` (i.e. the publisher hasn't uploaded
/// a binary archive yet but the source git repo + tag are known).
#[derive(Debug, Clone)]
pub struct ResolvedRegistryDep {
    pub registry_name: String,
    pub registry_url: String,
    pub package_name: String,
    pub package_repository: String,
    pub version_entry: VersionEntry,
}

/// Walk registries in declaration order; return the first non-yanked
/// exact-version match.
///
/// `registries` is a slice of `(registry_name, RegistryCache)` pairs. The
/// caller controls ordering — typically iter-order from the consumer's
/// `[registries]` table. Each cache must already be synced; this function
/// performs no network I/O and does NOT call `ensure_synced` itself.
///
/// Returns `Ok(None)` when no registry has the package OR none has the
/// requested version. Returns `Err` only when `RegistryCache::lookup`
/// surfaces an I/O / JSON-parse failure.
pub fn resolve_dep(
    dep_name: &str,
    version_req: &str,
    registries: &[(String, RegistryCache)],
) -> Result<Option<ResolvedRegistryDep>> {
    for (name, cache) in registries {
        let entry = match cache.lookup(dep_name)? {
            Some(e) => e,
            None => continue,
        };
        for v in &entry.versions {
            if v.yanked {
                continue;
            }
            if v.version == version_req {
                return Ok(Some(ResolvedRegistryDep {
                    registry_name: name.clone(),
                    registry_url: cache.url().to_string(),
                    package_name: dep_name.to_string(),
                    package_repository: entry.repository.clone(),
                    version_entry: v.clone(),
                }));
            }
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::PackageIndex;

    /// Build a `RegistryCache` against a tempdir-backed `CCGO_HOME` and
    /// pre-populate the sharded leaf path with `package_json`. No git
    /// involved — `lookup()` only reads from `local_path()`.
    fn make_test_cache(
        name: &str,
        package_json: &str,
        package_name: &str,
    ) -> (tempfile::TempDir, RegistryCache) {
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());
        let cache = RegistryCache::new(name, "file:///mock");
        let rel = PackageIndex::package_index_path(package_name);
        let abs = cache.local_path().join(&rel);
        std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
        std::fs::write(&abs, package_json).unwrap();
        (tmp, cache)
    }

    fn leaf_pkg_json(version: &str, yanked: bool) -> String {
        let yanked_str = if yanked { ", \"yanked\": true" } else { "" };
        format!(
            r#"{{"name":"leaf","description":"x","repository":"x","license":"MIT","platforms":[],"versions":[{{"version":"{version}","tag":"v{version}"{yanked_str}}}]}}"#
        )
    }

    #[test]
    #[serial_test::serial]
    fn no_registries_declared_returns_none() {
        let result = resolve_dep("leaf", "1.0.0", &[]).unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn exact_version_match_returns_resolved() {
        let (_g, cache) = make_test_cache("test", &leaf_pkg_json("1.0.0", false), "leaf");
        let registries = vec![("test".to_string(), cache)];
        let r = resolve_dep("leaf", "1.0.0", &registries)
            .unwrap()
            .expect("should resolve");
        assert_eq!(r.registry_name, "test");
        assert_eq!(r.package_name, "leaf");
        assert_eq!(r.version_entry.version, "1.0.0");
        assert_eq!(r.registry_url, "file:///mock");
    }

    #[test]
    #[serial_test::serial]
    fn version_mismatch_returns_none() {
        let (_g, cache) = make_test_cache("test", &leaf_pkg_json("1.0.0", false), "leaf");
        let registries = vec![("test".to_string(), cache)];
        let r = resolve_dep("leaf", "2.0.0", &registries).unwrap();
        assert!(r.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn first_registry_with_match_wins() {
        // Both registries hold a `leaf` package with version 1.0.0. The
        // first declared registry must win.
        let tmp = tempfile::TempDir::new().unwrap();
        std::env::set_var("CCGO_HOME", tmp.path());

        let cache_a = RegistryCache::new("alpha", "file:///alpha");
        let cache_b = RegistryCache::new("beta", "file:///beta");

        let rel = PackageIndex::package_index_path("leaf");
        for cache in [&cache_a, &cache_b] {
            let abs = cache.local_path().join(&rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(&abs, leaf_pkg_json("1.0.0", false)).unwrap();
        }

        let registries = vec![
            ("alpha".to_string(), cache_a),
            ("beta".to_string(), cache_b),
        ];
        let r = resolve_dep("leaf", "1.0.0", &registries)
            .unwrap()
            .expect("should resolve");
        assert_eq!(r.registry_name, "alpha");
        assert_eq!(r.registry_url, "file:///alpha");
    }

    #[test]
    #[serial_test::serial]
    fn yanked_versions_are_skipped() {
        let (_g, cache) = make_test_cache("test", &leaf_pkg_json("1.0.0", true), "leaf");
        let registries = vec![("test".to_string(), cache)];
        let r = resolve_dep("leaf", "1.0.0", &registries).unwrap();
        assert!(
            r.is_none(),
            "yanked-only entry must not satisfy resolution"
        );
    }

    #[test]
    #[serial_test::serial]
    fn resolved_carries_package_repository_for_git_fallback() {
        // The fetch path falls back to `git clone --branch <tag>` when the
        // VersionEntry has no archive_url. To do that it needs the git URL,
        // which lives at the package level (PackageEntry.repository) — not
        // per-version. Pin that the resolver propagates it onto
        // ResolvedRegistryDep.package_repository so install_from_registry
        // can reach it without a second cache lookup.
        let pkg_json = r#"{
            "name": "leaf",
            "description": "x",
            "repository": "git@example.com:org/leaf.git",
            "license": "MIT",
            "platforms": [],
            "versions": [{"version":"1.0.0","tag":"v1.0.0"}]
        }"#;
        let (_g, cache) = make_test_cache("test", pkg_json, "leaf");
        let registries = vec![("test".to_string(), cache)];
        let r = resolve_dep("leaf", "1.0.0", &registries)
            .unwrap()
            .expect("should resolve");
        assert_eq!(r.package_repository, "git@example.com:org/leaf.git");
        assert_eq!(r.version_entry.tag, "v1.0.0");
        assert!(r.version_entry.archive_url.is_none());
    }
}
