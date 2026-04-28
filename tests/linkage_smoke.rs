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
