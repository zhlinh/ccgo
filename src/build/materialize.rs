//! Materialize source-only dependencies during the build.
//!
//! When `ccgo build` encounters a dependency in `.ccgo/deps/<name>/` that
//! ships source code but no platform artifacts (lib/<platform>/), it
//! recursively spawns `ccgo build` inside the dep so the artifact form
//! that the consumer's resolved linkage needs gets produced. This module
//! is the driver for that step.

use std::path::{Path, PathBuf};

use anyhow::{ensure, Context, Result};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::build::linkage::{detect_dep_artifacts, DepArtifacts};
use crate::commands::build::LinkType;
use crate::config::Linkage;

/// Pick the `--build-as` value for a recursive `ccgo build` invocation.
///
/// The dep's CCGO.toml may declare its own `[build].link_type`, but the
/// consumer's hint takes precedence: the consumer KNOWS what artifact
/// it needs to link against, and we'd otherwise produce the wrong shape.
///
/// * `hint = Some(SharedExternal)` → `--build-as shared` (we need the dep's .so)
/// * `hint = Some(StaticEmbedded)` or `Some(StaticExternal)` → `--build-as static`
/// * `hint = None` → `--build-as both` (consumer hasn't decided; cover both axes)
///
/// `consumer` is currently unused but kept in the signature for symmetry
/// with `resolve_linkage`; future linkage modes (e.g. a pure-shared dep that
/// must remain shared regardless of consumer) may want to consult it.
pub fn build_as_for_hint(_consumer: LinkType, hint: Option<Linkage>) -> LinkType {
    match hint {
        Some(Linkage::SharedExternal) => LinkType::Shared,
        Some(Linkage::StaticEmbedded) | Some(Linkage::StaticExternal) => LinkType::Static,
        None => LinkType::Both,
    }
}

/// Compute a stable hash over (sorted src/ tree mtimes + sizes + paths) +
/// CCGO.toml content + the requested build-as. The mtime+size+path combo
/// is much cheaper than re-hashing every file's content while still
/// catching the cases ccgo cares about (file added, edited, removed,
/// renamed). For the build-as field we use the stable hand-written
/// `Display` impl ("static" / "shared" / "both") rather than `Debug`,
/// so future renames of the enum variants don't silently invalidate
/// every cached fingerprint.
///
/// Returns `Err` when `dep_root` does not exist — the materialize step
/// requires `ccgo fetch` to have populated the dep tree first, and a
/// silent fingerprint over an empty path would masquerade as a cache
/// hit on an unfetched dep.
pub fn compute_source_fingerprint(dep_root: &Path, build_as: LinkType) -> Result<String> {
    ensure!(
        dep_root.exists(),
        "dep root does not exist: {} (was `ccgo fetch` run?)",
        dep_root.display()
    );

    let mut hasher = Sha256::new();
    hasher.update(format!("build_as={build_as}\n").as_bytes());

    // CCGO.toml — full content so any TOML edit invalidates.
    let toml_path = dep_root.join("CCGO.toml");
    if toml_path.exists() {
        let content = std::fs::read(&toml_path)
            .with_context(|| format!("failed to read {}", toml_path.display()))?;
        hasher.update(b"toml=");
        hasher.update(&content);
        hasher.update(b"\n");
    }

    // src/ tree — sorted (path, mtime_nanos, size) tuples.
    let src = dep_root.join("src");
    if src.is_dir() {
        let mut entries: Vec<(PathBuf, u128, u64)> = WalkDir::new(&src)
            .follow_links(false)
            .into_iter()
            .flatten()
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let meta = e.metadata().ok()?;
                let mtime = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_nanos())
                    .unwrap_or(0);
                Some((e.path().to_path_buf(), mtime, meta.len()))
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, mtime, size) in entries {
            hasher.update(path.to_string_lossy().as_bytes());
            hasher.update(b"\0");
            hasher.update(mtime.to_le_bytes());
            hasher.update(size.to_le_bytes());
            hasher.update(b"\n");
        }
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Per-platform fingerprint sidecar path inside the dep root.
///
/// `platform` must be lowercase (matches `BuildTarget::to_string()` and the
/// `lib/<platform>/` layout). Mixing case will produce a different sidecar
/// path and silently invalidate caches.
pub fn fingerprint_path(dep_root: &Path, platform: &str) -> PathBuf {
    dep_root.join(format!(".ccgo_materialize_{platform}.fingerprint"))
}

/// Read a previously persisted fingerprint. `None` if the sidecar does
/// not exist yet; other I/O errors propagate as `Err`. The content is
/// trimmed so an external edit that adds a trailing newline (e.g. an
/// editor saving the file) doesn't break equality with the in-memory
/// digest.
pub fn read_fingerprint(path: &Path) -> Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(s) => Ok(Some(s.trim().to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e).with_context(|| format!("failed to read {}", path.display())),
    }
}

/// Persist the fingerprint, creating parent directories if needed.
pub fn write_fingerprint(path: &Path, value: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, value)
        .with_context(|| format!("failed to write {}", path.display()))
}

/// For each declared dep that ships source we can rebuild from, spawn
/// `ccgo build` inside the dep when its artifacts are missing OR the
/// source has changed since the last materialize. Skip when both
/// artifacts are present AND the fingerprint matches.
///
/// `dep_hints` is `(name, hint)` for every entry in `[[dependencies]]` —
/// the resolved hint after CLI / toml precedence merging. `None` means
/// "no hint, build both forms".
///
/// `ccgo_bin` is the path to the ccgo binary; tests resolve it manually
/// because `CARGO_BIN_EXE_ccgo` is only injected for integration tests
/// in `tests/`. Production passes `std::env::current_exe()?`.
///
/// `consumer_link_type` is fixed at `LinkType::Both` deliberately: we
/// build the worst-case artifact set so that regardless of whether the
/// ultimate consumer links statically or dynamically, the artifacts the
/// resolver needs are present. Threading the consumer's own link_type
/// here would couple `--build-as` for the *dep* to the *consumer*'s
/// preference, which is the wrong direction — the hint already encodes
/// what the consumer wants from the dep.
pub fn materialize_source_deps_inner(
    project_root: &Path,
    platform: &str,
    archs: &[String],
    release: bool,
    dep_hints: &[(String, Option<Linkage>)],
    ccgo_bin: &str,
) -> Result<()> {
    let platform_lc = platform.to_lowercase();
    let consumer_link_type = LinkType::Both;

    for (dep_name, hint) in dep_hints {
        let dep_root = project_root.join(".ccgo/deps").join(dep_name);
        if !dep_root.exists() {
            // ccgo fetch hasn't been run, or the dep is provided by some
            // other mechanism. Skip — resolve_linkage will surface the
            // missing-artifacts error downstream if it actually matters.
            continue;
        }

        // Only consider deps that ship source we can rebuild from. Binary
        // deps (no src/) or deps without a manifest are out of scope —
        // the consumer either trusts the prebuilt artifacts or fails at
        // resolve_linkage time.
        if !dep_root.join("src").is_dir() || !dep_root.join("CCGO.toml").is_file() {
            continue;
        }

        let build_as = build_as_for_hint(consumer_link_type.clone(), *hint);
        let fp_path = fingerprint_path(&dep_root, &platform_lc);
        let fp_now = compute_source_fingerprint(&dep_root, build_as.clone())?;
        let fp_prev = read_fingerprint(&fp_path)?;

        // Up-to-date iff the source fingerprint matches the persisted
        // one AND the dep currently has compiled artifacts on disk. The
        // artifact check defends against `rm -rf lib/` between runs:
        // even if the source hasn't changed, a missing lib/ must
        // re-trigger the build.
        let arts = detect_dep_artifacts(&dep_root, &platform_lc);
        let artifacts_present = matches!(
            arts,
            DepArtifacts::Both | DepArtifacts::OnlyStatic | DepArtifacts::OnlyShared
        );
        let fp_matches = Some(&fp_now) == fp_prev.as_ref();
        if artifacts_present && fp_matches {
            continue;
        }
        // First-run trust: if the dep already has compiled artifacts
        // (e.g. hand-committed xcframework symlinks like the linkage
        // fixtures, or a prior `ccgo install`) but no prior fingerprint
        // exists, assume the on-disk artifacts are correct for the
        // current source. Record the fingerprint so that future source
        // changes invalidate normally, and skip the spawn — rebuilding
        // from scratch would clobber the curated layout.
        if artifacts_present && fp_prev.is_none() {
            write_fingerprint(&fp_path, &fp_now)?;
            continue;
        }

        eprintln!(
            "📦 Materializing source-only dep '{}' for {} (--build-as {})",
            dep_name, platform_lc, build_as
        );

        let mut cmd = std::process::Command::new(ccgo_bin);
        cmd.arg("build").arg(&platform_lc).current_dir(&dep_root);
        if release {
            cmd.arg("--release");
        }
        if !archs.is_empty() {
            cmd.arg("--arch").arg(archs.join(","));
        }
        cmd.arg("--build-as").arg(build_as.to_string());

        let status = cmd.status().with_context(|| {
            format!(
                "failed to spawn `ccgo build` for source-only dep '{}'",
                dep_name
            )
        })?;
        if !status.success() {
            anyhow::bail!(
                "recursive `ccgo build` for source-only dep '{}' (--build-as {}) \
                 failed with exit code {:?}. The dep at {} could not be \
                 compiled — check its CCGO.toml and try `ccgo build {} \
                 --build-as {}` inside that directory to reproduce.",
                dep_name,
                build_as,
                status.code(),
                dep_root.display(),
                platform_lc,
                build_as,
            );
        }

        // Bridge ccgo's build-output layout into the lib/<platform>/ layout
        // that downstream FindCCGODependencies.cmake expects. ccgo build
        // writes into `cmake_build/<profile>/<platform>/{shared,static}/`,
        // but `detect_dep_artifacts` and the consumer-side CMake-Find both
        // read from `lib/<platform>/{shared,static}/`. A single symlink
        // covers every platform/arch/link-type permutation in one shot —
        // we don't need per-platform bridge logic.
        bridge_cmake_build_to_lib(&dep_root, &platform_lc, release)?;

        // Persist the new fingerprint so subsequent runs skip when nothing changed.
        write_fingerprint(&fp_path, &fp_now)?;
    }

    Ok(())
}

/// After `ccgo build` finishes inside a path-source dep, expose its
/// `cmake_build/<profile>/<platform>/` tree as `lib/<platform>/` so the
/// consumer's `FindCCGODependencies.cmake` walks find the artifacts.
///
/// Implemented as a single directory symlink (Unix) — no per-platform or
/// per-arch logic needed because the cmake_build subtree already mirrors
/// the `{shared,static}/<arch>/...` shape that CMake-Find walks. Pre-existing
/// `lib/<platform>/` content (e.g. a hand-committed xcframework symlink in
/// the linkage fixtures) is left alone; we only act when the directory is
/// absent. On Windows this is a no-op for now — Windows path-source deps
/// would need junction or copy support, deferred.
fn bridge_cmake_build_to_lib(dep_root: &Path, platform: &str, release: bool) -> Result<()> {
    let profile = if release { "release" } else { "debug" };
    let cmake_build_platform = dep_root.join("cmake_build").join(profile).join(platform);
    if !cmake_build_platform.is_dir() {
        // Build didn't produce the expected output tree. Most likely the
        // dep's CMake template doesn't write into cmake_build/<profile>/
        // (older fixture, custom CCGO.toml). Leave lib/ alone and let
        // resolve_linkage surface the missing-artifacts error downstream.
        return Ok(());
    }

    let lib_platform = dep_root.join("lib").join(platform);
    if lib_platform.exists() {
        // Pre-existing layout — committed symlinks (e.g. tests/fixtures/
        // linkage/leaf), or a previously-bridged dep. Don't disturb.
        //
        // Edge case: if the curated layout covers only one link type
        // (e.g. lib/<platform>/static/ exists but shared/ is missing),
        // the bridge skips entirely rather than filling in the missing
        // half. That's the safer default — touching the curated tree
        // could conflict with hand-rolled symlink/copy strategies. Deps
        // in this state need to commit both halves up front, or delete
        // the partial layout to let materialize own it.
        return Ok(());
    }

    std::fs::create_dir_all(dep_root.join("lib")).with_context(|| {
        format!(
            "failed to create lib/ in dep {}",
            dep_root.display()
        )
    })?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&cmake_build_platform, &lib_platform).with_context(|| {
            format!(
                "failed to symlink {} -> {}",
                lib_platform.display(),
                cmake_build_platform.display()
            )
        })?;
    }
    #[cfg(not(unix))]
    {
        // TODO: junction or copy fallback for Windows path-source deps.
        let _ = (cmake_build_platform, lib_platform);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every test sweeps all three consumer variants so that the day
    /// `_consumer` becomes load-bearing, any per-consumer divergence
    /// fires a test failure here instead of silently shipping.
    const ALL_CONSUMERS: [LinkType; 3] = [LinkType::Shared, LinkType::Static, LinkType::Both];

    #[test]
    fn no_hint_means_build_both_for_every_consumer() {
        for consumer in ALL_CONSUMERS {
            assert_eq!(
                build_as_for_hint(consumer.clone(), None),
                LinkType::Both,
                "consumer={consumer:?}"
            );
        }
    }

    #[test]
    fn shared_external_hint_picks_shared_for_every_consumer() {
        for consumer in ALL_CONSUMERS {
            assert_eq!(
                build_as_for_hint(consumer.clone(), Some(Linkage::SharedExternal)),
                LinkType::Shared,
                "consumer={consumer:?}"
            );
        }
    }

    #[test]
    fn static_embedded_hint_picks_static_for_every_consumer() {
        for consumer in ALL_CONSUMERS {
            assert_eq!(
                build_as_for_hint(consumer.clone(), Some(Linkage::StaticEmbedded)),
                LinkType::Static,
                "consumer={consumer:?}"
            );
        }
    }

    #[test]
    fn static_external_hint_picks_static_for_every_consumer() {
        for consumer in ALL_CONSUMERS {
            assert_eq!(
                build_as_for_hint(consumer.clone(), Some(Linkage::StaticExternal)),
                LinkType::Static,
                "consumer={consumer:?}"
            );
        }
    }

    #[test]
    fn fingerprint_changes_when_source_file_is_modified() {
        use std::fs;
        let tmp = tempfile::TempDir::new().unwrap();
        let dep = tmp.path().join("leaf");
        fs::create_dir_all(dep.join("src")).unwrap();
        fs::write(dep.join("CCGO.toml"), "[project]\nname=\"leaf\"\n").unwrap();
        fs::write(dep.join("src/leaf.cc"), "int x() { return 1; }\n").unwrap();

        let fp1 = compute_source_fingerprint(&dep, LinkType::Both).unwrap();

        // Modify the source. Use a different byte length so the size axis
        // alone discriminates the fingerprint — that keeps the test robust
        // on filesystems with coarse mtime resolution (FAT32, some network
        // mounts) where a quick rewrite could otherwise share an mtime.
        fs::write(dep.join("src/leaf.cc"), "int x() { return 42; }\n").unwrap();

        let fp2 = compute_source_fingerprint(&dep, LinkType::Both).unwrap();
        assert_ne!(fp1, fp2, "fingerprint should change when src content changes");
    }

    #[test]
    fn fingerprint_changes_when_build_as_differs() {
        use std::fs;
        let tmp = tempfile::TempDir::new().unwrap();
        let dep = tmp.path().join("leaf");
        fs::create_dir_all(dep.join("src")).unwrap();
        fs::write(dep.join("CCGO.toml"), "[project]\nname=\"leaf\"\n").unwrap();
        fs::write(dep.join("src/leaf.cc"), "int x() {}\n").unwrap();

        let fp_static = compute_source_fingerprint(&dep, LinkType::Static).unwrap();
        let fp_shared = compute_source_fingerprint(&dep, LinkType::Shared).unwrap();
        assert_ne!(
            fp_static, fp_shared,
            "different --build-as must yield different fingerprints"
        );
    }

    #[test]
    fn fingerprint_errors_when_dep_root_does_not_exist() {
        let tmp = tempfile::TempDir::new().unwrap();
        let nonexistent = tmp.path().join("never-fetched");
        let err = compute_source_fingerprint(&nonexistent, LinkType::Both).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("does not exist"),
            "expected `does not exist` in error, got: {msg}"
        );
        assert!(
            msg.contains("ccgo fetch"),
            "expected pointer to `ccgo fetch` in error, got: {msg}"
        );
    }

    #[test]
    fn fingerprint_persists_to_disk_and_reads_back() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join(".ccgo_materialize_macos.fingerprint");
        write_fingerprint(&path, "abc123").unwrap();
        let read = read_fingerprint(&path).unwrap();
        assert_eq!(read, Some("abc123".to_string()));
    }

    #[test]
    fn missing_fingerprint_file_reads_as_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.fingerprint");
        assert_eq!(read_fingerprint(&path).unwrap(), None);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn materialize_rebuilds_source_only_dep() {
        // First-build path: a path-source dep with no prior artifacts must
        // get a successful recursive `ccgo build` and the fingerprint
        // sidecar persisted. The cache-hit and cache-miss paths are
        // exercised separately by the synthetic tests below — they don't
        // rely on the real ccgo build pipeline producing the
        // `lib/<platform>/` layout that `detect_dep_artifacts` looks at.
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let consumer = manifest_dir.join("tests/fixtures/source_only/consumer");
        let leaf = manifest_dir.join("tests/fixtures/source_only/leaf");

        let _ = std::fs::remove_dir_all(consumer.join(".ccgo"));
        let _ = std::fs::remove_dir_all(leaf.join("cmake_build"));
        let _ = std::fs::remove_dir_all(leaf.join("lib"));
        let _ = std::fs::remove_file(leaf.join(".ccgo_materialize_macos.fingerprint"));

        let deps_dir = consumer.join(".ccgo/deps");
        std::fs::create_dir_all(&deps_dir).unwrap();
        std::os::unix::fs::symlink(&leaf, deps_dir.join("leaf")).unwrap();

        // CARGO_BIN_EXE_<name> is only injected for integration tests in
        // tests/, not for in-source #[cfg(test)] modules. Resolve under
        // target/<profile>/ — `cargo test` builds the binary into the
        // same profile dir as the test runner.
        let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
        let ccgo_bin = manifest_dir.join("target").join(profile).join("ccgo");
        assert!(
            ccgo_bin.exists(),
            "ccgo binary not found at {} — run `cargo build --bin ccgo`",
            ccgo_bin.display()
        );

        let result = materialize_source_deps_inner(
            &consumer,
            "macos",
            &[],
            false,
            &[("leaf".to_string(), None)],
            ccgo_bin.to_str().unwrap(),
        );
        assert!(
            result.is_ok(),
            "materialize_source_deps_inner failed: {result:?}"
        );

        // After the spawn, leaf must have build output AND the fingerprint
        // sidecar. (cmake_build/ is what ccgo build itself produces; the
        // lib/<platform>/ layout that the consumer's CMake-Find expects
        // is materialized later by Task 7's wiring — out of scope here.)
        assert!(
            leaf.join("cmake_build").exists(),
            "leaf should have produced cmake_build/ after materialize"
        );
        assert!(
            leaf.join(".ccgo_materialize_macos.fingerprint").exists(),
            "fingerprint sidecar should be persisted after a successful spawn"
        );
    }

    /// Build a synthetic dep tree at `dep_root` with src/, CCGO.toml, and a
    /// minimal `lib/macos/static/<name>.a` so `detect_dep_artifacts` reports
    /// `OnlyStatic`. Returns the path to the dep root.
    #[cfg(test)]
    fn make_synthetic_dep(parent: &Path, name: &str) -> PathBuf {
        let dep = parent.join(".ccgo/deps").join(name);
        std::fs::create_dir_all(dep.join("src")).unwrap();
        std::fs::create_dir_all(dep.join("lib/macos/static")).unwrap();
        std::fs::write(dep.join("CCGO.toml"), "[project]\nname=\"x\"\n").unwrap();
        std::fs::write(dep.join("src/x.cc"), "int x() { return 0; }\n").unwrap();
        std::fs::write(
            dep.join(format!("lib/macos/static/lib{name}.a")),
            b"!<arch>\n",
        )
        .unwrap();
        dep
    }

    #[test]
    fn cache_hit_skips_spawn_when_artifacts_present_and_fingerprint_matches() {
        // Wires up a synthetic dep where lib/ is already populated and the
        // fingerprint sidecar matches the current source. The driver MUST
        // skip the spawn entirely; we prove that by passing a non-existent
        // binary path that would error if spawned.
        let tmp = tempfile::TempDir::new().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let dep_root = make_synthetic_dep(&project, "leaf");

        let build_as = build_as_for_hint(LinkType::Both, None);
        let fp = compute_source_fingerprint(&dep_root, build_as.clone()).unwrap();
        write_fingerprint(&fingerprint_path(&dep_root, "macos"), &fp).unwrap();

        let sentinel = tmp.path().join("does-not-exist-and-must-not-be-spawned");
        let result = materialize_source_deps_inner(
            &project,
            "macos",
            &[],
            false,
            &[("leaf".to_string(), None)],
            sentinel.to_str().unwrap(),
        );
        assert!(
            result.is_ok(),
            "cache hit should skip spawn entirely; got error: {result:?}"
        );
    }

    #[test]
    fn cache_miss_attempts_spawn_when_source_changed() {
        // Same shape as cache_hit, but with a stale fingerprint. The driver
        // MUST try to spawn; the spawn fails (sentinel binary missing),
        // which the test asserts as proof the spawn was attempted.
        let tmp = tempfile::TempDir::new().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let dep_root = make_synthetic_dep(&project, "leaf");

        // Persist a fingerprint that won't match the current source.
        write_fingerprint(
            &fingerprint_path(&dep_root, "macos"),
            "stale-fingerprint-0000",
        )
        .unwrap();

        let sentinel = tmp.path().join("does-not-exist-and-spawn-must-error");
        let result = materialize_source_deps_inner(
            &project,
            "macos",
            &[],
            false,
            &[("leaf".to_string(), None)],
            sentinel.to_str().unwrap(),
        );
        assert!(
            result.is_err(),
            "cache miss must attempt spawn (which fails on the sentinel)"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("failed to spawn") || msg.contains("'leaf'"),
            "expected spawn-attempt error mentioning the dep, got: {msg}"
        );
    }

    #[test]
    fn first_run_with_prebuilt_artifacts_trusts_them_and_records_fingerprint() {
        // Dep ships pre-built artifacts (e.g. hand-committed xcframework
        // symlinks like tests/fixtures/linkage/leaf) and has never been
        // through materialize. The driver MUST trust the artifacts,
        // record a fingerprint for future invalidation, and NOT spawn —
        // rebuilding from scratch would clobber the curated layout.
        let tmp = tempfile::TempDir::new().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let dep_root = make_synthetic_dep(&project, "leaf");

        let fp_path = fingerprint_path(&dep_root, "macos");
        assert!(
            !fp_path.exists(),
            "test setup invariant: no fingerprint sidecar yet"
        );

        let sentinel = tmp.path().join("does-not-exist-and-must-not-be-spawned");
        let result = materialize_source_deps_inner(
            &project,
            "macos",
            &[],
            false,
            &[("leaf".to_string(), None)],
            sentinel.to_str().unwrap(),
        );
        assert!(
            result.is_ok(),
            "first run with prebuilt artifacts should skip spawn; got: {result:?}"
        );
        assert!(
            fp_path.exists(),
            "fingerprint sidecar should be persisted on first-run trust path"
        );
    }

    #[test]
    fn cache_miss_when_artifacts_deleted_even_if_fingerprint_matches() {
        // Source unchanged → fingerprint matches; but lib/ has been wiped
        // (e.g. user `rm -rf lib/`). The driver must still spawn.
        let tmp = tempfile::TempDir::new().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&project).unwrap();
        let dep_root = make_synthetic_dep(&project, "leaf");

        let build_as = build_as_for_hint(LinkType::Both, None);
        let fp = compute_source_fingerprint(&dep_root, build_as.clone()).unwrap();
        write_fingerprint(&fingerprint_path(&dep_root, "macos"), &fp).unwrap();

        // Now wipe lib/ to simulate the user nuking build artifacts.
        std::fs::remove_dir_all(dep_root.join("lib")).unwrap();

        let sentinel = tmp.path().join("does-not-exist");
        let result = materialize_source_deps_inner(
            &project,
            "macos",
            &[],
            false,
            &[("leaf".to_string(), None)],
            sentinel.to_str().unwrap(),
        );
        assert!(
            result.is_err(),
            "missing artifacts must trigger spawn even when fingerprint matches"
        );
    }

    #[test]
    fn binary_only_dep_is_skipped() {
        // A dep that has no src/ + CCGO.toml is binary-only and must not
        // be a candidate for materialize. Verify by making the spawn fail
        // if the driver were to try.
        let tmp = tempfile::TempDir::new().unwrap();
        let project = tmp.path().join("project");
        let dep = project.join(".ccgo/deps/binarydep");
        std::fs::create_dir_all(dep.join("lib/macos/shared")).unwrap();
        std::fs::write(
            dep.join("lib/macos/shared/libbinarydep.dylib"),
            b"fake-dylib",
        )
        .unwrap();

        let sentinel = tmp.path().join("does-not-exist");
        let result = materialize_source_deps_inner(
            &project,
            "macos",
            &[],
            false,
            &[("binarydep".to_string(), None)],
            sentinel.to_str().unwrap(),
        );
        assert!(
            result.is_ok(),
            "binary-only deps must be skipped without attempting spawn; got: {result:?}"
        );
    }

    #[test]
    fn missing_dep_root_is_skipped() {
        // A dep listed in dep_hints but never fetched (no .ccgo/deps/<name>/
        // dir) must be silently skipped — resolve_linkage will surface the
        // missing-artifacts error if it actually matters at link time.
        let tmp = tempfile::TempDir::new().unwrap();
        let project = tmp.path().join("project");
        std::fs::create_dir_all(project.join(".ccgo/deps")).unwrap();

        let sentinel = tmp.path().join("does-not-exist");
        let result = materialize_source_deps_inner(
            &project,
            "macos",
            &[],
            false,
            &[("ghost".to_string(), None)],
            sentinel.to_str().unwrap(),
        );
        assert!(
            result.is_ok(),
            "missing dep_root must be skipped without spawn; got: {result:?}"
        );
    }
}
