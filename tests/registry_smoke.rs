//! Smoke test: a consumer that depends on `leaf` via [registries] resolution.
//!
//! `ccgo fetch` should clone the index repo, look up `leaf` 1.0.0, find its
//! `archive_url`, download/extract the zip, and verify the checksum.
//!
//! This test currently FAILS by design — registry-driven resolution is not
//! yet implemented in `src/commands/fetch.rs`. It will start passing once
//! Task 5 of the index-driven dep resolution plan lands.

#[cfg(target_os = "macos")]
#[test]
fn registry_resolves_version_only_dep_via_index() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture = manifest_dir.join("tests/fixtures/registry/consumer");

    // Bootstrap the index + archive if absent (idempotent).
    let bootstrap = manifest_dir.join("tests/fixtures/registry/bootstrap.sh");
    let bootstrap_status = std::process::Command::new("sh")
        .arg(&bootstrap)
        .status()
        .expect("bootstrap.sh failed to spawn");
    assert!(
        bootstrap_status.success(),
        "registry bootstrap.sh failed; fixture is unusable"
    );

    // Patch the consumer CCGO.toml's `ABS_PATH_TO_INDEX` placeholder with a
    // real absolute path. The committed file is the .template; the .toml is
    // generated per-test-run and gitignored.
    let template = std::fs::read_to_string(fixture.join("CCGO.toml.template"))
        .expect("CCGO.toml.template should be present in the fixture");
    let index_path = manifest_dir.join("tests/fixtures/registry/index");
    let index_path_str = index_path
        .to_str()
        .expect("registry fixture path is not valid UTF-8");
    let toml = template.replace("ABS_PATH_TO_INDEX", index_path_str);
    std::fs::write(fixture.join("CCGO.toml"), toml).expect("failed to write generated CCGO.toml");

    // Clean prior state from previous runs.
    let _ = std::fs::remove_dir_all(fixture.join(".ccgo"));
    let _ = std::fs::remove_dir_all(fixture.join("cmake_build"));
    let _ = std::fs::remove_file(fixture.join("CCGO.lock"));

    let fetch = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args(["fetch"])
        .current_dir(&fixture)
        .status()
        .expect("ccgo fetch failed to spawn");
    assert!(
        fetch.success(),
        "ccgo fetch should resolve `leaf` 1.0.0 through the test registry"
    );

    // After fetch, .ccgo/deps/leaf/ must contain the unzipped archive.
    let leaf_root = fixture.join(".ccgo/deps/leaf");
    assert!(
        leaf_root.join("CCGO.toml").is_file(),
        "leaf/CCGO.toml should exist after registry-resolved fetch"
    );
    let leaf_lib_macos = leaf_root.join("lib/macos");
    assert!(
        leaf_lib_macos.is_dir(),
        "leaf/lib/macos/ should exist after registry-resolved fetch"
    );

    // Stronger: the bridge should leave at least one .a or .dylib visible
    // somewhere under lib/macos/. A bare lib/macos/ dir without artifacts
    // would mean the archive extracted but the fixture is wrong.
    let has_artifact = walk_for_extension(&leaf_lib_macos, &["a", "dylib", "xcframework"]);
    assert!(
        has_artifact,
        "leaf/lib/macos/ should contain at least one .a/.dylib/.xcframework after \
         registry-resolved fetch — got an empty layout, likely a fixture/extract bug"
    );

    // Lockfile must record the registry+ source and the checksum copied
    // from the index — registry-resolved deps are byte-deterministic on
    // re-fetch.
    let lock_path = fixture.join("CCGO.lock");
    let lock_text = std::fs::read_to_string(&lock_path)
        .expect("CCGO.lock should be written by ccgo fetch");
    assert!(
        lock_text.contains("source = \"registry+"),
        "lockfile should record `registry+<url>` source for the resolved dep:\n{lock_text}"
    );
    assert!(
        lock_text.contains("checksum = \"sha256:"),
        "lockfile should record sha256 checksum for the resolved dep:\n{lock_text}"
    );
}

#[cfg(target_os = "macos")]
fn walk_for_extension(root: &std::path::Path, exts: &[&str]) -> bool {
    let Ok(entries) = std::fs::read_dir(root) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                return true;
            }
        }
        if path.is_dir() && walk_for_extension(&path, exts) {
            return true;
        }
    }
    false
}
