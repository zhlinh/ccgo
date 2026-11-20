//! Exercise the embedded adapter with a real compiler and installed headers.
#![cfg(unix)]
use std::{fs, path::Path, process::Command};

fn run(command: &mut Command) {
    let output = command.output().expect("command must start");
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture(root: &Path) {
    fs::create_dir_all(root.join("upstream source")).unwrap();
    fs::create_dir_all(root.join("adapter")).unwrap();
    for (name, contents) in [
        (
            "CMakeLists.txt",
            include_str!("../src/builder/upstream/CMakeLists.txt"),
        ),
        (
            "merge.cmake",
            include_str!("../src/builder/upstream/merge.cmake"),
        ),
        (
            "headers.cmake.in",
            include_str!("../src/builder/upstream/headers.cmake.in"),
        ),
    ] {
        fs::write(root.join("adapter").join(name), contents).unwrap();
    }
    let source = root.join("upstream source");
    fs::write(
        source.join("CMakeLists.txt"),
        r#"
cmake_minimum_required(VERSION 3.18)
project(upstream C)
if(NOT HOOK_LOADED)
    message(FATAL_ERROR "Hook must run before upstream configuration")
endif()
add_library(helper STATIC helper.c)
set_target_properties(helper PROPERTIES POSITION_INDEPENDENT_CODE ON)
add_library(upstream entry.c)
target_link_libraries(upstream PRIVATE helper)
add_library(unrelated EXCLUDE_FROM_ALL broken.c)
install(TARGETS upstream helper DESTINATION lib)
install(FILES api.h DESTINATION include/vendor)
"#,
    )
    .unwrap();
    fs::write(source.join("helper.c"), "int helper(void) { return 42; }").unwrap();
    fs::write(
        source.join("entry.c"),
        "int helper(void); int answer(void) { return helper(); }",
    )
    .unwrap();
    fs::write(
        source.join("broken.c"),
        "#error unrelated target must not build",
    )
    .unwrap();
    fs::write(source.join("api.h"), "int answer(void);").unwrap();
    fs::write(root.join("CCGO.cmake"), "set(HOOK_LOADED ON)").unwrap();
}

#[test]
fn upstream_static_shared_and_header_refresh() {
    let temp = tempfile::Builder::new()
        .prefix("ccgo upstream ")
        .tempdir()
        .unwrap();
    let root = temp.path();
    fixture(root);
    for shared in [false, true] {
        let build = root.join(if shared { "shared" } else { "static" });
        run(Command::new("cmake")
            .arg("-S")
            .arg(root.join("adapter"))
            .arg("-B")
            .arg(&build)
            .arg(format!(
                "-DCCGO_UPSTREAM_SOURCE={}",
                root.join("upstream source").display()
            ))
            .arg("-DCCGO_UPSTREAM_TARGET=upstream")
            .arg("-DCCGO_UPSTREAM_NAME=packaged")
            .arg("-DCCGO_UPSTREAM_INCLUDE=include/vendor")
            .arg(format!("-DCCGO_PROJECT_ROOT={}", root.display()))
            .arg(format!(
                "-DCCGO_HEADER_STAGE={}",
                root.join("headers").display()
            ))
            .arg(format!(
                "-DCMAKE_INSTALL_PREFIX={}",
                build.join("install").display()
            ))
            .arg(format!(
                "-DCCGO_BUILD_SHARED_LIBS={}",
                if shared { "ON" } else { "OFF" }
            )));
        run(Command::new("cmake")
            .arg("--build")
            .arg(&build)
            .args(["--target", "ccgo_upstream"]));
        run(Command::new("cmake").arg("--install").arg(&build));
        assert!(root.join("headers/api.h").exists());
        fs::write(root.join("headers/stale.h"), "stale").unwrap();
        run(Command::new("cmake").arg("--install").arg(&build));
        assert!(!root.join("headers/stale.h").exists());
        {
            fs::write(
                root.join("main.c"),
                "#include <api.h>\nint main(void) { return answer() != 42; }",
            )
            .unwrap();
            let exe = root.join("consumer");
            let library = if !shared {
                "libpackaged.a"
            } else if cfg!(target_os = "macos") {
                "libpackaged.dylib"
            } else {
                "libpackaged.so"
            };
            assert!(build.join("out").join(library).exists());
            run(Command::new("cc")
                .arg(root.join("main.c"))
                .arg("-I")
                .arg(root.join("headers"))
                .arg(build.join("out").join(library))
                .arg(format!("-Wl,-rpath,{}", build.join("out").display()))
                .arg("-o")
                .arg(&exe));
            run(&mut Command::new(exe));
        }
    }
    for (argument, expected) in [
        ("-DCCGO_UPSTREAM_TARGET=missing", "target does not exist"),
        ("-DCCGO_UPSTREAM_TARGET=helper", "requires a SHARED_LIBRARY"),
        ("-DCCGO_UPSTREAM_INCLUDE=../escape", "must stay inside"),
    ] {
        let output = Command::new("cmake")
            .arg("-S")
            .arg(root.join("adapter"))
            .arg("-B")
            .arg(root.join("shared"))
            .arg(argument)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains(expected));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn upstream_cli_packages_sdk_without_root_cmakelists() {
    let temp = tempfile::tempdir().unwrap();
    fixture(temp.path());
    fs::write(
        temp.path().join("CCGO.toml"),
        r#"
[package]
name = "packaged"
version = "1.0.0"
[build.cmake]
source_dir = "upstream source"
target = "unused_global_target"
install_include_dir = "include/vendor"
[platforms.macos.build.cmake]
target = "upstream"
"#,
    )
    .unwrap();
    run(Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(temp.path())
        .args([
            "build",
            "macos",
            "--arch",
            "arm64",
            "--release",
            "--build-as",
            "static",
            "--no-fetch",
        ]));
    assert!(temp
        .path()
        .join("target/release/macos/PACKAGED_MACOS_SDK-1.0.0.zip")
        .exists());
}
