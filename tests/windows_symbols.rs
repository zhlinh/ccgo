//! Verify Windows symbol packaging without requiring a Windows compiler.
#![cfg(unix)]

use assert_cmd::Command;
use std::{fs, io::Read, os::unix::fs::PermissionsExt, path::Path};

fn tool(path: &Path, script: &str) {
    fs::write(path, format!("#!/bin/sh\nset -eu\n{script}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn zip_entry(path: &Path, name: &str) -> Vec<u8> {
    let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut bytes = Vec::new();
    zip.by_name(name).unwrap().read_to_end(&mut bytes).unwrap();
    bytes
}

#[test]
fn mingw_build_saves_symbols_before_stripping_and_package_merges_them() {
    check_windows_packaging(None);
}

#[test]
fn mingw_variant_applies_to_sdk_and_symbols_archives() {
    check_windows_packaging(Some("ANO"));
}

fn check_windows_packaging(variant: Option<&str>) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(
        root.join("CCGO.toml"),
        "[package]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    fs::write(root.join("CMakeLists.txt"), "# fake build\n").unwrap();
    for compiler in ["gcc", "g++"] {
        tool(
            &bin.join(format!("x86_64-w64-mingw32-{compiler}")),
            "echo 'MinGW 14.0.0'",
        );
    }
    tool(
        &bin.join("x86_64-w64-mingw32-strip"),
        "printf stripped > \"$2\"",
    );
    tool(
        &bin.join("cmake"),
        r#"
case "$1" in
  --version) echo 'cmake version 3.25.0';;
  --build)
    mkdir -p "$2/out"
    printf unstripped > "$2/out/demo.dll"
    printf import > "$2/out/demo.dll.a";;
  --install) :;;
  *) printf '%s\n' "$@" >> "$WINDOWS_CMAKE_CAPTURE";;
esac
"#,
    );
    let paths = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    command
        .current_dir(root)
        .env("PATH", paths)
        .env("CCGO_HOME", root.join("ccgo-home"))
        .env("WINDOWS_CMAKE_CAPTURE", root.join("cmake-args"))
        .args([
            "build",
            "windows",
            "--arch",
            "x86_64",
            "--build-as",
            "shared",
            "--release",
        ]);
    if let Some(variant) = variant {
        command.args(["--variant", variant]);
    }
    command.assert().success();
    assert!(fs::read_to_string(root.join("cmake-args"))
        .unwrap()
        .contains("-DCMAKE_BUILD_TYPE=RelWithDebInfo"));
    let output = root.join("target/release/windows");
    let variant_tail = variant.map(|v| format!("-{v}")).unwrap_or_default();
    let archive_name = format!("DEMO_WINDOWS_SDK-1.2.3{variant_tail}");
    assert_eq!(
        zip_entry(
            &output.join(format!("{archive_name}.zip")),
            "lib/windows/shared/x86_64/mingw/demo.dll"
        ),
        b"stripped"
    );
    let symbol_path = "symbols/windows/obj/x86_64/mingw/shared/demo.dll";
    assert_eq!(
        zip_entry(
            &output.join(format!("{archive_name}-SYMBOLS.zip")),
            symbol_path
        ),
        b"unstripped"
    );
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(root)
        .env("CCGO_HOME", root.join("ccgo-home"))
        .args(["package", "--release"])
        .assert()
        .success();
    assert_eq!(
        zip_entry(
            &root.join("target/release/package/DEMO_CCGO_PACKAGE-1.2.3.zip"),
            symbol_path
        ),
        b"unstripped"
    );
}
