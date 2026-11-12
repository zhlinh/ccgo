//! Verify C/C++ SDK import and the Gradle handoff without requiring an NDK.
#![cfg(unix)]

use assert_cmd::Command;
use std::{
    fs,
    io::{Read, Write},
    os::unix::fs::PermissionsExt,
    path::Path,
};

fn sdk(path: &Path, version: &str, unsafe_entry: bool) {
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        (
            "meta/android/build_info.json",
            format!(
                r#"{{"project":{{"name":"DEMO","version":"{version}"}},"build":{{"platform":"Android"}}}}"#
            ),
        ),
        ("include/demo/api/native/demo.h", "void demo(void);".into()),
        (
            "lib/android/shared/arm64-v8a/libdemo.so",
            "native-library".into(),
        ),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    if unsafe_entry {
        zip.start_file("../escaped", opts).unwrap();
        zip.write_all(b"unsafe").unwrap();
    }
    zip.finish().unwrap();
}

fn project(root: &Path) {
    fs::write(
        root.join("CCGO.toml"),
        r#"
[package]
name = "demo"
version = "1.2.3"
[kmp]
targets = ["android"]
[kmp.native_sdks]
android = "android-sdk.zip"
"#,
    )
    .unwrap();
    let kmp = root.join("kmp");
    fs::create_dir(&kmp).unwrap();
    fs::write(kmp.join("build.gradle.kts"), "// mocked Gradle project").unwrap();
    let wrapper = kmp.join("gradlew");
    fs::write(
        &wrapper,
        r#"#!/bin/sh
set -eu
sdk=''
for arg in "$@"; do
    case "$arg" in -PccgoNativeSdk.android=*) sdk="${arg#*=}";; esac
    case "$arg" in -PccgoNativeSdkReceipt=*) receipt="${arg#*=}";; esac
done
test -n "$sdk"
test "$(cat "$sdk/lib/android/shared/arm64-v8a/libdemo.so")" = native-library
test -f "$sdk/include/demo/api/native/demo.h"
printf '%s\n' "$sdk" > ../sdk-root
printf ccgo-native-sdk-v1 > "$receipt"
mkdir -p build/outputs/aar
printf kotlin-aar > build/outputs/aar/demo-release.aar
"#,
    )
    .unwrap();
    fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755)).unwrap();
}

fn build(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    command
        .current_dir(root)
        .env("CCGO_HOME", root.join("ccgo-home"))
        .args([
            "build",
            "kmp",
            "--release",
            "--variant",
            "ANO",
            "--no-fetch",
        ]);
    command
}

#[test]
fn imports_native_sdk_and_packages_gradle_outputs_without_native_compilation() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(root);
    sdk(&root.join("android-sdk.zip"), "1.2.3", false);
    let result = build(root).assert().success();
    let stderr = String::from_utf8_lossy(&result.get_output().stderr);
    assert!(!stderr.contains("--- Building android native libraries ---"));
    let sdk_root = fs::read_to_string(root.join("sdk-root")).unwrap();
    assert!(
        !Path::new(sdk_root.trim()).exists(),
        "temporary SDK must be cleaned up"
    );
    let archive = root.join("target/release/kmp/DEMO_KMP_SDK-1.2.3-ANO.zip");
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).unwrap()).unwrap();
    let mut content = String::new();
    zip.by_name("lib/kmp/android/demo-release.aar")
        .unwrap()
        .read_to_string(&mut content)
        .unwrap();
    assert_eq!(content, "kotlin-aar");
}

#[test]
fn rejects_sdk_version_mismatch_before_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    project(tmp.path());
    sdk(&tmp.path().join("android-sdk.zip"), "9.9.9", false);
    let result = build(tmp.path()).assert().failure();
    assert!(String::from_utf8_lossy(&result.get_output().stderr)
        .contains("Native SDK metadata must match"));
    assert!(!tmp.path().join("sdk-root").exists());
}

#[test]
fn cli_native_sdk_works_without_toml_sdk_inputs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(root);
    let config = root.join("CCGO.toml");
    let text = fs::read_to_string(&config).unwrap();
    fs::write(config, text.split("[kmp.native_sdks]").next().unwrap()).unwrap();
    sdk(&root.join("renamed sdk=ANO.zip"), "1.2.3", false);
    build(root)
        .args(["--native-sdk", "Android=renamed sdk=ANO.zip"])
        .assert()
        .success();
}

#[test]
fn cli_native_sdk_overrides_toml_and_last_cli_value_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(root);
    // The configured android-sdk.zip does not exist; only the override is valid.
    sdk(&root.join("new-sdk.zip"), "1.2.3", false);
    build(root)
        .args([
            "--native-sdk",
            "android=missing.zip",
            "--native-sdk",
            "android=new-sdk.zip",
        ])
        .assert()
        .success();
}

#[test]
fn cli_native_sdk_preserves_other_toml_platforms() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(root);
    let config = root.join("CCGO.toml");
    let text = fs::read_to_string(&config).unwrap() + "linux = 'missing-linux.zip'\n";
    fs::write(config, text).unwrap();
    sdk(&root.join("new-sdk.zip"), "1.2.3", false);
    let result = build(root)
        .args(["--native-sdk", "android=new-sdk.zip"])
        .assert()
        .failure();
    assert!(
        String::from_utf8_lossy(&result.get_output().stderr).contains("Failed to import linux SDK")
    );
    assert!(!root.join("sdk-root").exists());
}

#[test]
fn invalid_cli_native_sdk_inputs_are_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    for input in ["android", "android=", "ohos=sdk.zip", "=sdk.zip"] {
        build(tmp.path())
            .args(["--native-sdk", input])
            .assert()
            .code(2);
    }
}

#[test]
fn rejects_unsafe_sdk_paths_before_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    project(tmp.path());
    sdk(&tmp.path().join("android-sdk.zip"), "1.2.3", true);
    let result = build(tmp.path()).assert().failure();
    assert!(
        String::from_utf8_lossy(&result.get_output().stderr).contains("Unsafe native SDK ZIP path")
    );
    assert!(!tmp.path().join("sdk-root").exists());
}

#[test]
fn refuses_to_package_when_gradle_ignores_native_sdks() {
    let tmp = tempfile::tempdir().unwrap();
    project(tmp.path());
    sdk(&tmp.path().join("android-sdk.zip"), "1.2.3", false);
    let wrapper = tmp.path().join("kmp/gradlew");
    let script = fs::read_to_string(&wrapper).unwrap();
    fs::write(
        &wrapper,
        script.replace("printf ccgo-native-sdk-v1 > \"$receipt\"", ":"),
    )
    .unwrap();
    let result = build(tmp.path()).assert().failure();
    assert!(String::from_utf8_lossy(&result.get_output().stderr)
        .contains("did not acknowledge imported native SDKs"));
    assert!(!tmp
        .path()
        .join("target/release/kmp/DEMO_KMP_SDK-1.2.3-ANO.zip")
        .exists());
}

#[test]
fn desktop_can_consume_a_windows_sdk_on_any_host_without_building_host_native_code() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(root);
    let config = root.join("CCGO.toml");
    let text = fs::read_to_string(&config)
        .unwrap()
        .replace("targets = [\"android\"]", "targets = [\"desktop\"]")
        .replace(
            "android = \"android-sdk.zip\"",
            "windows = \"windows-sdk.zip\"",
        );
    fs::write(config, text).unwrap();
    let mut zip = zip::ZipWriter::new(fs::File::create(root.join("windows-sdk.zip")).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    for (name, bytes) in [
        (
            "meta/windows/build_info.json",
            r#"{"project":"demo","version":"1.2.3","platform":"windows"}"#,
        ),
        ("include/demo/demo.h", "void demo(void);"),
        ("lib/windows/shared/x86_64/msvc/demo.dll", "windows-library"),
    ] {
        zip.start_file(name, opts).unwrap();
        zip.write_all(bytes.as_bytes()).unwrap();
    }
    zip.finish().unwrap();
    fs::write(
        root.join("kmp/gradlew"),
        r#"#!/bin/sh
set -eu
for arg in "$@"; do
    case "$arg" in
        -PccgoNativeSdk.windows=*) sdk="${arg#*=}";;
        -PccgoNativeSdkReceipt=*) receipt="${arg#*=}";;
    esac
done
test "$(cat "$sdk/lib/windows/shared/x86_64/msvc/demo.dll")" = windows-library
printf ready > "$receipt"
mkdir -p build/libs
printf desktop-jar > build/libs/demo.jar
"#,
    )
    .unwrap();
    let result = build(root).assert().success();
    assert!(!String::from_utf8_lossy(&result.get_output().stderr).contains("--- Building"));
    assert!(root
        .join("target/release/kmp/DEMO_KMP_SDK-1.2.3-ANO.zip")
        .is_file());
}
