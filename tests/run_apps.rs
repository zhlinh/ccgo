//! Application dispatch and native CMake projects without opening GUI windows.
#![cfg(unix)]

use assert_cmd::Command;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn config(root: &Path, apps: &str) {
    fs::write(
        root.join("CCGO.toml"),
        format!("[package]\nname='demo'\nversion='1.0.0'\n{apps}"),
    )
    .unwrap();
}

fn run(root: &Path, name: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    command
        .current_dir(root)
        .env("CCGO_HOME", root.join("ccgo-home"))
        .args(["run", "--app", name]);
    command
}

fn cmake_app(root: &Path, source: &str) {
    let app = root.join("app/native");
    fs::create_dir_all(&app).unwrap();
    fs::write(
        app.join("CMakeLists.txt"),
        r#"
cmake_minimum_required(VERSION 3.16)
project(native_demo LANGUAGES CXX)
add_executable(native_demo main.cpp extra.cpp)
set_target_properties(native_demo PROPERTIES RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin")
"#,
    )
    .unwrap();
    fs::write(app.join("main.cpp"), source).unwrap();
    fs::write(app.join("extra.cpp"), "int value() { return 7; }\n").unwrap();
    fs::write(app.join("asset.txt"), "app-asset").unwrap();
}

#[test]
fn cmake_app_builds_multiple_sources_and_runs_with_assets_and_arguments() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(root, "[[app]]\nname='native'\ntarget='native_demo'\nexecutable='bin/native_demo'\nrun_args=['default']\n");
    cmake_app(
        root,
        r#"
#include <fstream>
#include <iostream>
#include <string>
extern int value();
int main(int argc, char** argv) {
    std::ifstream asset("asset.txt");
    std::string text;
    asset >> text;
    if (value() != 7 || text != "app-asset" || argc != 3) return 1;
    std::cout << "APP:" << argv[1] << ":" << argv[2];
}
"#,
    );
    let result = run(root, "native")
        .args(["--release", "--", "hello world"])
        .assert()
        .success();
    assert!(result
        .get_output()
        .stdout
        .ends_with(b"APP:default:hello world"));
    assert!(root
        .join("target/run/app/native/release/bin/native_demo")
        .is_file());
}

#[test]
fn cmake_build_only_does_not_execute_app() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(root, "[[app]]\nname='native'\nframework='cmake'\ntarget='native_demo'\nexecutable='bin/native_demo'\n");
    cmake_app(root, "int main() { return 42; }\n");
    run(root, "native").arg("--build-only").assert().success();
    let result = run(root, "native").assert().failure();
    assert!(String::from_utf8_lossy(&result.get_output().stderr).contains("App command failed"));
}

fn fake_tool(root: &Path, tool: &str) -> std::ffi::OsString {
    let bin = root.join("tools");
    fs::create_dir_all(&bin).unwrap();
    let path = bin.join(tool);
    fs::write(
        &path,
        r#"#!/bin/sh
set -eu
pwd > "$APP_CAPTURE"
printf '%s\n' "$@" >> "$APP_CAPTURE"
exit "${APP_EXIT:-0}"
"#,
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap()
}

#[test]
fn flutter_dispatches_run_and_build_and_preserves_argument_boundaries() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(root, "[[app]]\nname='ui'\npath='app/flutter ui'\nplatform='linux'\ndevice='device-123'\nrun_args=['--dart-define=MODE=demo']\nbuild_args=['--split-debug-info=symbols']\n");
    let app = root.join("app/flutter ui");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("pubspec.yaml"), "name: demo\n").unwrap();
    let paths = fake_tool(root, "flutter");
    let capture = root.join("capture");
    run(root, "ui")
        .env("PATH", &paths)
        .env("APP_CAPTURE", &capture)
        .args(["--release", "--", "--dart-define=LABEL=hello world"])
        .assert()
        .success();
    let text = fs::read_to_string(&capture).unwrap();
    assert!(text.ends_with("run\n--device-id\ndevice-123\n--release\n--dart-define=MODE=demo\n--dart-define=LABEL=hello world\n"));
    assert_eq!(
        Path::new(text.lines().next().unwrap())
            .canonicalize()
            .unwrap(),
        app.canonicalize().unwrap()
    );
    run(root, "ui")
        .env("PATH", &paths)
        .env("APP_CAPTURE", &capture)
        .arg("--build-only")
        .assert()
        .success();
    assert!(fs::read_to_string(capture)
        .unwrap()
        .ends_with("build\nlinux\n--debug\n--split-debug-info=symbols\n"));
}

#[test]
fn flutter_mobile_without_device_uses_flutter_device_selection() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(
        root,
        "[[app]]\nname='ui'\nframework='flutter'\nplatform='android'\n",
    );
    fs::create_dir_all(root.join("app/ui")).unwrap();
    fs::write(root.join("app/ui/pubspec.yaml"), "name: demo\n").unwrap();
    let paths = fake_tool(root, "flutter");
    let capture = root.join("capture");
    run(root, "ui")
        .env("PATH", paths)
        .env("APP_CAPTURE", &capture)
        .assert()
        .success();
    assert!(fs::read_to_string(capture)
        .unwrap()
        .ends_with("run\n--debug\n"));
}

#[test]
fn tauri_is_discovered_and_tool_failures_propagate() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(root, "");
    let app = root.join("app/ui/src-tauri");
    fs::create_dir_all(&app).unwrap();
    fs::write(app.join("tauri.conf.json"), "{}").unwrap();
    let paths = fake_tool(root, "cargo");
    let capture = root.join("capture");
    for (flags, expected) in [
        (vec![], "tauri\ndev\n"),
        (vec!["--release"], "tauri\ndev\n--release\n"),
        (vec!["--build-only"], "tauri\nbuild\n--debug\n"),
        (vec!["--build-only", "--release"], "tauri\nbuild\n"),
    ] {
        run(root, "ui")
            .env("PATH", &paths)
            .env("APP_CAPTURE", &capture)
            .args(flags)
            .assert()
            .success();
        assert!(fs::read_to_string(&capture).unwrap().ends_with(expected));
    }
    run(root, "ui")
        .env("PATH", &paths)
        .env("APP_CAPTURE", &capture)
        .env("APP_EXIT", "17")
        .assert()
        .failure();
}

#[test]
fn rejects_ambiguous_manifests_and_conflicting_target_flags() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(root, "");
    cmake_app(root, "int main() {}\n");
    fs::write(root.join("app/native/pubspec.yaml"), "name: demo\n").unwrap();
    let result = run(root, "native").assert().failure();
    assert!(
        String::from_utf8_lossy(&result.get_output().stderr).contains("Multiple app frameworks")
    );
    run(root, "native")
        .args(["--example", "basic"])
        .assert()
        .code(2);
    run(root, "native").args(["--bin", "tool"]).assert().code(2);
    run(root, "../native").assert().failure();
}

#[test]
fn clean_app_removes_build_caches_and_preserves_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    config(root, "");
    cmake_app(root, "int main() {}\n");
    fs::create_dir_all(root.join("target/run/app/native/debug")).unwrap();
    fs::create_dir_all(root.join("app/native/build")).unwrap();
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(root)
        .args(["clean", "app", "--yes"])
        .assert()
        .success();
    assert!(!root.join("target/run/app").exists());
    assert!(!root.join("app/native/build").exists());
    assert!(root.join("app/native/main.cpp").is_file());
}
