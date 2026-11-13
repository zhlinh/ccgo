//! Host C++ example/bin builds, including linking a prebuilt project library.
#![cfg(unix)]

use assert_cmd::Command;
use std::{fs, path::Path};

fn project(root: &Path, targets: &str) {
    fs::write(
        root.join("CCGO.toml"),
        format!("[package]\nname='demo'\nversion='1.0.0'\n{targets}"),
    )
    .unwrap();
}

fn run(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    command
        .current_dir(root)
        .env("CCGO_HOME", root.join("ccgo-home"))
        .arg("run");
    command
}

#[test]
fn discovers_and_runs_flat_and_directory_cxx_examples() {
    for relative in ["app/basic.cxx", "app/basic/main.cc", "app/basic/main.cxx"] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        project(root, "");
        let source = root.join(relative);
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(
            source,
            "#include <iostream>\nint main(int argc, char** argv) { std::cout << argv[1]; }\n",
        )
        .unwrap();
        let result = run(root).args(["--", "example-ok"]).assert().success();
        assert!(result.get_output().stdout.ends_with(b"example-ok"));
    }
}

#[test]
fn bin_build_only_compiles_without_running() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(root, "[[bin]]\nname='tool'\npath='tool.cpp'\n");
    fs::write(root.join("tool.cpp"), "int main() { return 42; }\n").unwrap();
    run(root)
        .args(["--bin", "tool", "--release", "--build-only"])
        .assert()
        .success();
    assert!(root.join("target/run/tool/tool").is_file());
    run(root)
        .args(["--bin", "tool", "--release"])
        .assert()
        .failure();
}

#[test]
fn configured_example_links_project_static_library() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    project(
        root,
        "[[app]]\nname='linked'\npath='demo.cpp'\nframework='cpp'\n",
    );
    fs::write(root.join("library.cpp"), "int demo_value() { return 7; }\n").unwrap();
    fs::write(
        root.join("demo.cpp"),
        "extern int demo_value();\nint main() { return demo_value() == 7 ? 0 : 1; }\n",
    )
    .unwrap();
    fs::create_dir_all(root.join("target/run/lib")).unwrap();
    assert!(std::process::Command::new("c++")
        .current_dir(root)
        .args(["-c", "library.cpp", "-o", "library.o"])
        .status()
        .unwrap()
        .success());
    assert!(std::process::Command::new("ar")
        .current_dir(root)
        .args(["rcs", "target/run/lib/libdemo.a", "library.o"])
        .status()
        .unwrap()
        .success());
    run(root).args(["--app", "linked"]).assert().success();
}
