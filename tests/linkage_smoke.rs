//! Smoke tests: build the consumer fixtures, inspect the resulting binaries
//! to verify that the linkage mode (shared-external vs static-embedded) is
//! correctly applied end-to-end.
//!
//! shared-external: the consumer's dylib lists libleaf.dylib as a DT_NEEDED
//! load command (otool -L) and does NOT compile leaf symbols into itself.
//!
//! static-embedded: the consumer's static archive (.a) contains the leaf
//! symbols, and the dylib does NOT list libleaf.dylib as a load command.

#[cfg(target_os = "macos")]
#[test]
fn shared_external_does_not_embed_dep_symbols() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/linkage/consumer-shared-external");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args(["build", "macos", "--release"])
        .current_dir(&fixture)
        .status()
        .expect("ccgo build failed to spawn");
    assert!(status.success(), "build did not succeed");

    // For shared-external linkage, the consumer dylib must list libleaf.dylib
    // as an external load command (DT_NEEDED / LC_LOAD_DYLIB).
    let dylib = fixture.join("cmake_build/release/macos/shared/arm64/libconsumer.dylib");
    let out = std::process::Command::new("otool")
        .args(["-L"])
        .arg(&dylib)
        .output()
        .expect("otool not found");
    let load_cmds = String::from_utf8_lossy(&out.stdout);
    assert!(
        load_cmds.contains("libleaf.dylib"),
        "consumer.dylib should list libleaf.dylib as an external load command \
         when linkage = shared-external. otool -L output:\n{load_cmds}"
    );

    // Also verify that the leaf symbol is NOT exported by the consumer dylib
    // (it lives in libleaf.dylib, not duplicated in libconsumer.dylib).
    let nm_out = std::process::Command::new("nm")
        .args(["-gU"])
        .arg(&dylib)
        .output()
        .expect("nm not found");
    let symbols = String::from_utf8_lossy(&nm_out.stdout);
    assert!(
        !symbols.contains("_leaf_version_marker"),
        "consumer.dylib should not export leaf symbols when \
         linkage = shared-external. Found:\n{symbols}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn static_embedded_does_embed_dep_symbols() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/linkage/consumer-static-embedded");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args(["build", "macos", "--release"])
        .current_dir(&fixture)
        .status()
        .expect("ccgo build failed to spawn");
    assert!(status.success(), "build did not succeed");

    // For static-embedded linkage, the universal static archive must contain
    // the leaf symbol compiled in (not referenced as undefined).
    let static_archive =
        fixture.join("cmake_build/release/macos/static/universal/lib/libconsumer.a");
    let out = std::process::Command::new("nm")
        .arg(&static_archive)
        .output()
        .expect("nm not found");
    let symbols = String::from_utf8_lossy(&out.stdout);
    assert!(
        symbols.contains("T _leaf_version_marker"),
        "consumer.a should contain a defined _leaf_version_marker (T) when \
         linkage = static-embedded. nm output:\n{symbols}"
    );

    // Also verify that the dylib does NOT list libleaf.dylib as an external
    // load command — the dep is embedded, not a DT_NEEDED.
    let dylib = fixture.join("cmake_build/release/macos/shared/arm64/libconsumer.dylib");
    let otool_out = std::process::Command::new("otool")
        .args(["-L"])
        .arg(&dylib)
        .output()
        .expect("otool not found");
    let load_cmds = String::from_utf8_lossy(&otool_out.stdout);
    assert!(
        !load_cmds.contains("libleaf.dylib"),
        "consumer.dylib should NOT list libleaf.dylib as an external load \
         command when linkage = static-embedded. otool -L output:\n{load_cmds}"
    );
}

/// CLI `--linkage <value>` must override `[build].default_dep_linkage` from
/// CCGO.toml. The consumer-static-embedded fixture has
/// `default_dep_linkage = "static-embedded"` in its CCGO.toml; running
/// with `--linkage shared-external` should flip the resolution so the
/// leaf symbol is no longer archived in.
///
/// This is the central acceptance test for the size-comparison use case:
/// the same project, the same CCGO.toml, but two different linkages
/// chosen at build time.
///
/// Uses a dedicated `consumer-cli-override` fixture (a copy of
/// consumer-static-embedded) so this test's `cmake_build/` doesn't collide
/// in parallel with the sibling tests that target the shared-external and
/// static-embedded fixtures. Cargo runs integration tests in parallel by
/// default; one fixture per test gives clean isolation.
#[cfg(target_os = "macos")]
#[test]
fn cli_linkage_default_overrides_toml_default() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/linkage/consumer-cli-override");

    // Clean prior outputs so this test is order-independent.
    let _ = std::fs::remove_dir_all(fixture.join("cmake_build"));
    let _ = std::fs::remove_dir_all(fixture.join("target"));

    // The fixture has `path = "../leaf"`, so ccgo fetch resolves the dep
    // by symlinking ../leaf into .ccgo/deps/leaf. Run fetch first so the
    // build sees the dep tree, regardless of fresh-clone state.
    let fetch = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args(["fetch"])
        .current_dir(&fixture)
        .status()
        .expect("ccgo fetch failed to spawn");
    assert!(fetch.success(), "ccgo fetch did not succeed");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args([
            "build",
            "macos",
            "--release",
            "--linkage",
            "shared-external",
        ])
        .current_dir(&fixture)
        .status()
        .expect("ccgo build failed to spawn");
    assert!(
        status.success(),
        "build with --linkage shared-external did not succeed"
    );

    // After the override, the consumer dylib should record libleaf.dylib as
    // an external load command — proof the CLI flag took effect even though
    // CCGO.toml says static-embedded.
    let dylib = fixture.join("cmake_build/release/macos/shared/arm64/libconsumer.dylib");
    let otool_out = std::process::Command::new("otool")
        .args(["-L"])
        .arg(&dylib)
        .output()
        .expect("otool not found");
    let load_cmds = String::from_utf8_lossy(&otool_out.stdout);
    assert!(
        load_cmds.contains("libleaf.dylib"),
        "with --linkage shared-external, consumer.dylib should list \
         libleaf.dylib as a load command even though CCGO.toml says \
         static-embedded. otool -L output:\n{load_cmds}"
    );

    // And the leaf symbol should NOT be defined in the consumer dylib.
    let nm_out = std::process::Command::new("nm")
        .args(["-gU"])
        .arg(&dylib)
        .output()
        .expect("nm not found");
    let symbols = String::from_utf8_lossy(&nm_out.stdout);
    assert!(
        !symbols.contains("_leaf_version_marker"),
        "with --linkage shared-external, consumer.dylib should not export \
         leaf symbols. nm -gU output:\n{symbols}"
    );
}
