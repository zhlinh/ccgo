//! Smoke test: a consumer that depends on a source-only leaf dep
//! must build cleanly without the user pre-running `ccgo build` in leaf/.

#[cfg(target_os = "macos")]
#[test]
fn source_only_dep_is_built_automatically() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/source_only/consumer");

    // Clean prior outputs in BOTH consumer and leaf so the test is
    // order-independent and exercises the materialize step from a
    // clean slate.
    let _ = std::fs::remove_dir_all(fixture.join("cmake_build"));
    let _ = std::fs::remove_dir_all(fixture.join("target"));
    let leaf = fixture.parent().unwrap().join("leaf");
    let _ = std::fs::remove_dir_all(leaf.join("cmake_build"));
    let _ = std::fs::remove_dir_all(leaf.join("lib"));

    let fetch = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args(["fetch"])
        .current_dir(&fixture)
        .status()
        .expect("ccgo fetch failed to spawn");
    assert!(fetch.success(), "ccgo fetch did not succeed");

    let status = std::process::Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args(["build", "macos", "--release"])
        .current_dir(&fixture)
        .status()
        .expect("ccgo build failed to spawn");
    assert!(
        status.success(),
        "build did not succeed for source-only dep — \
         materialize_source_deps should have rebuilt leaf"
    );

    // Consumer dylib must list libleaf.dylib (default linkage = shared-external).
    let dylib = fixture.join("cmake_build/release/macos/shared/arm64/libconsumer.dylib");
    let out = std::process::Command::new("otool")
        .args(["-L"])
        .arg(&dylib)
        .output()
        .expect("otool not found");
    let load_cmds = String::from_utf8_lossy(&out.stdout);
    assert!(
        load_cmds.contains("libleaf.dylib"),
        "consumer.dylib should reference libleaf.dylib produced by the \
         materialize step. otool -L:\n{load_cmds}"
    );
}
