//! Verify publish command wiring without contacting package repositories.
#![cfg(unix)]

use assert_cmd::Command;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn recorder(path: &Path) {
    fs::write(
        path,
        "#!/bin/sh\npwd > \"$PUBLISH_CAPTURE\"\nprintf '%s\\n' \"$@\" >> \"$PUBLISH_CAPTURE\"\n",
    )
    .unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn command_with_tools(root: &Path, bin: &Path, capture: &Path) -> Command {
    let paths = std::env::join_paths(
        std::iter::once(bin.to_path_buf())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    command
        .current_dir(root)
        .env("PATH", paths)
        .env("PUBLISH_CAPTURE", capture);
    command
}

fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
    use std::io::Write;
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    for (name, contents) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(contents).unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn doc_and_conan_accept_zipped_projects_and_preserve_original_archives() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for tool in ["mkdocs", "conan"] {
        let executable = bin.join(tool);
        recorder(&executable);
        let mut script = fs::read_to_string(&executable).unwrap();
        script.push_str("for arg in \"$@\"; do\ncase \"$arg\" in\n*.py|*.yml|*.yaml) test -f \"$arg\" || exit 8; dir=${arg%/*}; cp \"$dir/assets.txt\" \"$PUBLISH_ASSETS\" || exit 9;;\nesac\ndone\nexit \"${PUBLISH_EXIT:-0}\"\n");
        fs::write(executable, script).unwrap();
    }
    for (target, filename) in [
        ("doc", "mkdocs.yml"),
        ("doc", "mkdocs.yaml"),
        ("conan", "conanfile.py"),
    ] {
        for prefix in ["", "bundle/project/"] {
            for exit_code in ["0", "1"] {
                let archive = tmp.path().join(format!("{target}.zip"));
                let descriptor = format!("{prefix}{filename}");
                let assets = format!("{prefix}assets.txt");
                write_zip(
                    &archive,
                    &[
                        (descriptor.as_str(), b"fixture"),
                        (assets.as_str(), b"original-assets"),
                    ],
                );
                let original = fs::read(&archive).unwrap();
                let capture = tmp.path().join("args");
                let copied_assets = tmp.path().join("assets");
                let result = command_with_tools(tmp.path(), &bin, &capture)
                    .env("PUBLISH_ASSETS", &copied_assets)
                    .env("PUBLISH_EXIT", exit_code)
                    .args(["publish", target, "--path"])
                    .arg(&archive)
                    .args(if target == "conan" { vec!["--skip-build"] } else { vec![] })
                    .assert();
                if exit_code == "0" {
                    result.success();
                } else {
                    result.failure();
                }
                let args = fs::read_to_string(&capture).unwrap();
                let descriptor =
                    Path::new(args.lines().find(|line| line.ends_with(filename)).unwrap());
                assert!(
                    !descriptor.exists(),
                    "temporary extraction must be removed, including after failure"
                );
                assert_eq!(fs::read(&archive).unwrap(), original);
                assert_eq!(fs::read(&copied_assets).unwrap(), b"original-assets");
                if target == "doc" {
                    assert_eq!(
                        Path::new(args.lines().next().unwrap()),
                        fs::canonicalize(tmp.path()).unwrap()
                    );
                    assert!(args.contains("gh-deploy\n"));
                    assert!(args.contains("--config-file\n"));
                } else {
                    assert_eq!(
                        Path::new(args.lines().next().unwrap()),
                        descriptor.parent().unwrap()
                    );
                    assert!(args.contains("export\n"));
                }
            }
        }
    }
}

#[test]
fn doc_and_conan_reject_archives_with_missing_or_ambiguous_descriptors() {
    let tmp = tempfile::tempdir().unwrap();
    for (target, filename) in [("doc", "mkdocs.yml"), ("conan", "conanfile.py")] {
        let archive = tmp.path().join("project.zip");
        for ambiguous in [false, true] {
            if ambiguous {
                write_zip(
                    &archive,
                    &[
                        (filename, b"fixture"),
                        (&format!("nested/{filename}"), b"fixture"),
                    ],
                );
            } else {
                write_zip(&archive, &[("README.md", b"fixture")]);
            }
            let original = fs::read(&archive).unwrap();
            Command::new(env!("CARGO_BIN_EXE_ccgo"))
                .current_dir(tmp.path())
                .args(["publish", target, "--path"])
                .arg(&archive)
                .assert()
                .failure()
                .stderr(predicates::str::contains(if ambiguous {
                    "ZIP contains multiple"
                } else {
                    "ZIP must contain one"
                }));
            assert_eq!(fs::read(&archive).unwrap(), original);
        }
    }
}

#[test]
fn android_and_ohos_accept_artifact_directories_and_sdk_zips() {
    use std::io::Write;
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    fs::create_dir(tmp.path().join("android")).unwrap();
    recorder(&bin.join("ohpm"));
    recorder(&tmp.path().join("android/gradlew"));
    for (target, extension) in [("android", "aar"), ("ohos", "har")] {
        let sdk = tmp.path().join(format!("{target}-sdk"));
        fs::create_dir_all(sdk.join("lib")).unwrap();
        let artifact = sdk.join(format!("lib/library.{extension}"));
        fs::write(&artifact, b"prebuilt").unwrap();
        let zip_path = tmp.path().join(format!("{target}-sdk.zip"));
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        zip.start_file(
            format!("sdk/lib/library.{extension}"),
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"prebuilt").unwrap();
        // A ZIP is always a prebuilt SDK input, even if it includes project files.
        zip.start_file(
            if target == "android" {
                "gradlew"
            } else {
                "oh-package.json5"
            },
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"fixture").unwrap();
        zip.finish().unwrap();
        let original_zip = fs::read(&zip_path).unwrap();
        for input in [&sdk, &zip_path] {
            let capture = tmp.path().join("args");
            command_with_tools(tmp.path(), &bin, &capture)
                .args(["publish", target, "--path"])
                .arg(input)
                .assert()
                .success();
            let args = fs::read_to_string(&capture).unwrap();
            if target == "android" {
                assert!(args.contains("-PccgoPublishPath="));
                assert!(args.contains("-PccgoSkipPublishBuild=true"));
            }
            let published = args
                .lines()
                .find_map(|line| {
                    let path = line.strip_prefix("-PccgoPublishPath=").unwrap_or(line);
                    path.ends_with(&format!(".{extension}"))
                        .then_some(Path::new(path))
                })
                .unwrap();
            if input == &zip_path {
                assert!(!published.exists());
            } else {
                assert_eq!(fs::read(published).unwrap(), b"prebuilt");
            }
            assert_eq!(fs::read(&zip_path).unwrap(), original_zip);
            assert!(artifact.is_file());
        }
    }
}

#[test]
fn explicit_project_directories_select_android_and_ohos_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    recorder(&bin.join("ohpm"));
    for target in ["android", "ohos"] {
        let project = tmp.path().join(format!("custom-{target}"));
        fs::create_dir(&project).unwrap();
        if target == "android" {
            recorder(&project.join("gradlew"));
            fs::write(
                project.join("CCGO.toml"),
                "[project]\nname = \"sample\"\nversion = \"1.0.0\"\n[android]\nstl = \"c++_static\"\n",
            )
            .unwrap();
            let wrapper = project.join("gradlew");
            let mut script = fs::read_to_string(&wrapper).unwrap();
            script.push_str("printf '%s\\n' \"$CCGO_ANDROID_STL\" >> \"$PUBLISH_CAPTURE\"\n");
            fs::write(wrapper, script).unwrap();
        } else {
            fs::write(project.join("oh-package.json5"), b"{}").unwrap();
            fs::write(project.join("fixture.har"), b"fixture").unwrap();
        }
        let capture = tmp.path().join("args");
        command_with_tools(tmp.path(), &bin, &capture)
            .args(["publish", target, "--path"])
            .arg(&project)
            .assert()
            .success();
        let args = fs::read_to_string(capture).unwrap();
        assert_eq!(
            Path::new(args.lines().next().unwrap()),
            fs::canonicalize(project).unwrap()
        );
        assert!(!args.contains("ccgoPublishPath="));
        assert!(!args.contains("ccgoSkipPublishBuild=true"));
        if target == "ohos" {
            assert!(args.contains("prepublish\n"));
        }
        if target == "android" {
            assert!(args.lines().any(|line| line == "c++_static"));
        }
    }
}

#[test]
fn apple_conan_and_doc_accept_directory_or_descriptor() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    for tool in ["pod", "conan", "mkdocs", "swift"] {
        recorder(&bin.join(tool));
    }
    for (target, filename, extra) in [
        ("apple", "Library.podspec", vec!["--manager", "cocoapods"]),
        ("conan", "conanfile.py", vec!["--skip-build"]),
        ("doc", "mkdocs.yml", vec![]),
    ] {
        let project = tmp.path().join(format!("custom-{target}"));
        fs::create_dir(&project).unwrap();
        let descriptor = project.join(filename);
        fs::write(&descriptor, b"fixture").unwrap();
        for input in [&project, &descriptor] {
            let capture = tmp.path().join("args");
            command_with_tools(tmp.path(), &bin, &capture)
                .args(["publish", target, "--path"])
                .arg(input)
                .args(&extra)
                .assert()
                .success();
            let args = fs::read_to_string(capture).unwrap();
            assert_eq!(
                Path::new(args.lines().next().unwrap()),
                fs::canonicalize(&project).unwrap()
            );
            assert!(args
                .lines()
                .any(|line| Path::new(line) == fs::canonicalize(&descriptor).unwrap()));
            if target == "doc" {
                assert!(args.contains("gh-deploy\n"));
                assert!(args.contains("--config-file\n"));
            }
        }
    }
    let spm = tmp.path().join("spm");
    fs::create_dir(&spm).unwrap();
    fs::write(spm.join("Package.swift"), b"fixture").unwrap();
    fs::write(
        spm.join("CCGO.toml"),
        "[package]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    for input in [&spm, &spm.join("Package.swift")] {
        command_with_tools(tmp.path(), &bin, &tmp.path().join("args"))
            .args(["publish", "apple", "--manager", "spm", "--path"])
            .arg(input)
            .assert()
            .success();
    }
}

#[test]
fn ambiguous_artifact_directory_fails_before_publishing() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("debug.aar"), b"debug").unwrap();
    fs::write(tmp.path().join("release.aar"), b"release").unwrap();
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .args(["publish", "android", "--path", "."])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Multiple .aar files"));
}

#[test]
fn android_passes_absolute_aar_and_skips_build() {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir(tmp.path().join("android")).unwrap();
    recorder(&tmp.path().join("android/gradlew"));
    fs::write(tmp.path().join("custom library.aar"), b"aar").unwrap();
    let capture = tmp.path().join("args");
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("PUBLISH_CAPTURE", &capture)
        .args(["publish", "android", "--path", "custom library.aar"])
        .assert()
        .success();
    let args = fs::read_to_string(&capture).unwrap();
    let aar = fs::canonicalize(tmp.path().join("custom library.aar")).unwrap();
    assert!(args
        .lines()
        .any(|line| line == format!("-PccgoPublishPath={}", aar.display())));
    assert!(args.contains("-PccgoSkipPublishBuild=true\n"));
    assert!(args.contains("ccgoPublishToMavenLocal\n"));

    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("PUBLISH_CAPTURE", &capture)
        .args(["publish", "android", "--skip-build"])
        .assert()
        .success();
    let args = fs::read_to_string(capture).unwrap();
    assert!(args.contains("-PccgoSkipPublishBuild=true\n"));
    assert!(!args.contains("-PccgoPublishPath="));
}

#[test]
fn ohos_and_cocoapods_accept_files_outside_default_directories() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    recorder(&bin.join("ohpm"));
    recorder(&bin.join("pod"));
    let paths = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    fs::create_dir(tmp.path().join("artifacts")).unwrap();
    for (target, file, extra) in [
        ("ohos", "library.har", vec![]),
        ("apple", "Library.podspec", vec!["--manager", "cocoapods"]),
    ] {
        let artifact = tmp.path().join("artifacts").join(file);
        fs::write(&artifact, b"fixture").unwrap();
        let capture = tmp.path().join("args");
        Command::new(env!("CARGO_BIN_EXE_ccgo"))
            .current_dir(tmp.path())
            .env("PATH", &paths)
            .env("PUBLISH_CAPTURE", &capture)
            .args(["publish", target, "--path"])
            .arg(Path::new("artifacts").join(file))
            .args(extra)
            .assert()
            .success();
        let args = fs::read_to_string(capture).unwrap();
        assert!(args
            .lines()
            .any(|line| line == fs::canonicalize(&artifact).unwrap().to_str().unwrap()));
        if target == "apple" {
            assert_eq!(
                args.lines().next().unwrap(),
                fs::canonicalize(artifact.parent().unwrap())
                    .unwrap()
                    .to_str()
                    .unwrap()
            );
        }
    }
}

#[test]
fn kmp_uses_explicit_project_directory() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("custom-kmp");
    fs::create_dir(&project).unwrap();
    recorder(&project.join("gradlew"));
    let capture = tmp.path().join("args");
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("PUBLISH_CAPTURE", &capture)
        .args(["publish", "kmp", "--path", "custom-kmp"])
        .assert()
        .success();
    let args = fs::read_to_string(capture).unwrap();
    assert_eq!(
        args.lines().next().unwrap(),
        fs::canonicalize(project).unwrap().to_str().unwrap()
    );
    assert!(args.contains("publishToMavenLocal\n"));
}

#[test]
fn kmp_sdk_zip_and_directory_publish_prebuilt_files() {
    use std::io::Write;
    let tmp = tempfile::tempdir().unwrap();
    let sdk = tmp.path().join("sdk");
    fs::create_dir_all(sdk.join("lib/kmp/android")).unwrap();
    fs::create_dir_all(sdk.join("meta/kmp")).unwrap();
    let metadata = br#"{"project":"stdcomm","version":"0.0.5"}"#;
    fs::write(sdk.join("meta/kmp/build_info.json"), metadata).unwrap();
    fs::write(sdk.join("lib/kmp/android/library.aar"), b"prebuilt-aar").unwrap();
    let zip_path = tmp.path().join("SDK.zip");
    let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
    for (path, bytes) in [
        ("meta/kmp/build_info.json", metadata.as_slice()),
        ("lib/kmp/android/library.aar", b"prebuilt-aar".as_slice()),
    ] {
        zip.start_file(path, zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap();
    fs::create_dir(tmp.path().join("kmp")).unwrap();
    let wrapper = tmp.path().join("kmp/gradlew");
    recorder(&wrapper);
    // Read the temporary manifest/artifacts while the child process is alive.
    fs::write(&wrapper, "#!/bin/sh\nfor arg in \"$@\"; do\ncase \"$arg\" in\n-PccgoKmpSdkManifest=*) cp \"${arg#*=}\" \"$PUBLISH_CAPTURE\";;\nesac\ndone\nprintf '%s\\n' \"$@\" > \"$PUBLISH_ARGS\"\n").unwrap();
    let capture = tmp.path().join("manifest.json");
    let args_path = tmp.path().join("args");
    for input in [&zip_path, &sdk] {
        Command::new(env!("CARGO_BIN_EXE_ccgo"))
            .current_dir(tmp.path())
            .env("PUBLISH_CAPTURE", &capture)
            .env("PUBLISH_ARGS", &args_path)
            .args(["publish", "kmp", "--path"])
            .arg(input)
            .assert()
            .success();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&capture).unwrap()).unwrap();
        assert_eq!(manifest["project"], "stdcomm");
        assert_eq!(manifest["version"], "0.0.5");
        assert_eq!(manifest["artifacts"][0]["target"], "android");
        let args = fs::read_to_string(&args_path).unwrap();
        assert!(args.contains("ccgoPublishToMavenLocal\n"));
        assert!(args.contains("--init-script\n"));
    }
}

#[test]
fn apple_maven_upload_and_private_specs_are_independent() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    fs::write(tmp.path().join("demo.podspec"), "fixture").unwrap();
    fs::write(bin.join("pod"), "#!/bin/sh\nif [ \"$1\" = ipc ]; then\necho '{\"name\":\"Demo\",\"version\":\"1.2.3\",\"vendored_frameworks\":\"Demo.xcframework\"}'\nelse\nprintf '%s\\n' \"$@\" > \"$PUBLISH_CAPTURE\"\nfi\n").unwrap();
    fs::set_permissions(bin.join("pod"), fs::Permissions::from_mode(0o755)).unwrap();
    let archive = tmp.path().join("SDK.zip");
    write_zip(
        &archive,
        &[("Demo.xcframework/Info.plist", b"framework-fixture")],
    );
    let original = fs::read(&archive).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/releases", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut data = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let size = stream.read(&mut buf).unwrap();
                assert!(size > 0);
                data.extend_from_slice(&buf[..size]);
                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            requests.push(data);
        }
        requests
    });
    let capture = tmp.path().join("pod-args");
    command_with_tools(tmp.path(), &bin, &capture)
        .env("CCGO_MAVEN_URL", &base)
        .env("CCGO_MAVEN_USERNAME", "publisher")
        .env("CCGO_MAVEN_PASSWORD", "secret")
        .args([
            "publish",
            "apple",
            "--manager",
            "cocoapods",
            "--path",
            "SDK.zip",
            "--artifact-provider",
            "maven",
            "--artifact-group",
            "com.example",
            "--registry",
            "private",
            "--remote",
            "company-specs",
        ])
        .assert()
        .success();
    let requests = server.join().unwrap();
    assert!(String::from_utf8_lossy(&requests[0])
        .starts_with("PUT /releases/com/example/demo-apple/1.2.3/demo-apple-1.2.3.zip HTTP/1.1"));
    assert!(String::from_utf8_lossy(&requests[0])
        .to_lowercase()
        .contains("authorization: basic "));
    assert!(requests[0].ends_with(&original));
    assert!(String::from_utf8_lossy(&requests[2]).contains("<packaging>zip</packaging>"));
    let spec: serde_json::Value = serde_json::from_slice(
        &fs::read(
            tmp.path()
                .join("target/ccgo-publish/apple/1.2.3/demo-apple.podspec.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        spec["source"]["http"],
        format!("{base}/com/example/demo-apple/1.2.3/demo-apple-1.2.3.zip")
    );
    assert_eq!(spec["source"]["sha256"].as_str().unwrap().len(), 64);
    assert!(!spec.to_string().contains("secret"));
    let args = fs::read_to_string(capture).unwrap();
    assert!(args.starts_with("repo\npush\ncompany-specs\n"));
    assert_eq!(fs::read(archive).unwrap(), original);
}

#[test]
fn apple_github_generates_spm_wrapper_and_uploads_named_zip() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    let gh = bin.join("gh");
    fs::write(&gh, "#!/bin/sh\nprintf '%s\\n' \"$@\" >> \"$PUBLISH_CAPTURE\"\nif [ \"$2\" = upload ]; then\nfor arg in \"$@\"; do\ncase \"$arg\" in\n*.zip) cp \"$arg\" \"$PUBLISH_ARCHIVE\"; printf '%s' \"$arg\" > \"$PUBLISH_TEMP\";;\nesac\ndone\nfi\n").unwrap();
    fs::set_permissions(gh, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    let swift = "// swift-tools-version:5.9\nimport PackageDescription\nlet package = Package(name: \"Demo\", targets: [.binaryTarget(name: \"Demo\", url: \"https://old/SDK.zip\", checksum: \"old\")])\n";
    fs::write(tmp.path().join("Package.swift"), swift).unwrap();
    let framework = tmp.path().join("Demo.xcframework");
    fs::create_dir(&framework).unwrap();
    fs::write(framework.join("Info.plist"), "fixture").unwrap();
    let capture = tmp.path().join("args");
    let archive = tmp.path().join("uploaded.zip");
    let temporary = tmp.path().join("temporary-path");
    command_with_tools(tmp.path(), &bin, &capture)
        .env("PUBLISH_ARCHIVE", &archive)
        .env("PUBLISH_TEMP", &temporary)
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--push",
            "--artifact-provider",
            "github",
            "--github-repo",
            "company/sdk",
            "--path",
            "Demo.xcframework",
        ])
        .assert()
        .success();
    let updated = fs::read_to_string(
        tmp.path()
            .join("target/ccgo-publish/apple/1.2.3/Package.swift"),
    )
    .unwrap();
    assert!(updated
        .contains("https://github.com/company/sdk/releases/download/v1.2.3/demo-apple-1.2.3.zip"));
    assert!(!updated.contains("checksum: \"old\""));
    assert_eq!(
        fs::read_to_string(tmp.path().join("Package.swift")).unwrap(),
        swift
    );
    let zip = zip::ZipArchive::new(fs::File::open(archive).unwrap()).unwrap();
    assert!(zip
        .file_names()
        .any(|name| name == "Demo.xcframework/Info.plist"));
    assert!(!Path::new(&fs::read_to_string(temporary).unwrap()).exists());
    assert!(fs::read_to_string(capture)
        .unwrap()
        .contains("release\nupload\nv1.2.3\n--repo\ncompany/sdk"));
}

#[test]
fn apple_invalid_spm_archive_fails_before_upload() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    recorder(&bin.join("gh"));
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    fs::write(
        tmp.path().join("Package.swift"),
        ".binaryTarget(name: \"Demo\", url: \"https://old/sdk.zip\", checksum: \"old\")",
    )
    .unwrap();
    let archive = tmp.path().join("SDK.zip");
    write_zip(
        &archive,
        &[("lib/ios/Demo.xcframework/Info.plist", b"fixture")],
    );
    let capture = tmp.path().join("args");
    command_with_tools(tmp.path(), &bin, &capture)
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--artifact-provider",
            "github",
            "--github-repo",
            "company/sdk",
            "--path",
            "SDK.zip",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("ZIP root"));
    assert!(!capture.exists());
    assert!(archive.exists());
}

#[test]
fn spm_binary_without_push_does_not_upload_or_generate_manifests() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    recorder(&bin.join("gh"));
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[package]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    let manifest = ".binaryTarget(name: \"Demo\", url: \"https://old/sdk.zip\", checksum: \"old\")";
    fs::write(tmp.path().join("Package.swift"), manifest).unwrap();
    let archive = tmp.path().join("SDK.zip");
    write_zip(&archive, &[("Demo.xcframework/Info.plist", b"fixture")]);
    let original = fs::read(&archive).unwrap();
    let capture = tmp.path().join("gh-args");
    command_with_tools(tmp.path(), &bin, &capture)
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--artifact-provider",
            "github",
            "--github-repo",
            "company/sdk",
            "--path",
            "SDK.zip",
            "--version",
            "2.0.0",
            "--tag",
            "v2.0.0",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "No files generated or artifacts uploaded",
        ));
    assert!(!capture.exists());
    assert!(!tmp.path().join("target/ccgo-publish").exists());
    assert_eq!(fs::read(archive).unwrap(), original);
    assert_eq!(
        fs::read_to_string(tmp.path().join("Package.swift")).unwrap(),
        manifest
    );
}

#[test]
fn doc_maven_builds_fresh_site_and_uploads_only_with_push() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname = 'demo'\nversion = '1.2.3'\n",
    )
    .unwrap();
    fs::write(tmp.path().join("mkdocs.yml"), "site_name: Demo").unwrap();
    let tool = bin.join("mkdocs");
    fs::write(&tool, "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$PUBLISH_CAPTURE\"\nwhile [ \"$#\" -gt 0 ]; do\nif [ \"$1\" = --site-dir ]; then shift; mkdir -p \"$1\"; printf fresh > \"$1/index.html\"; fi\nshift\ndone\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/releases", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..4 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut data = Vec::new();
            let mut buf = [0; 4096];
            loop {
                let size = stream.read(&mut buf).unwrap();
                assert!(size > 0);
                data.extend_from_slice(&buf[..size]);
                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length: "))
                        .unwrap()
                        .parse()
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            stream
                .write_all(
                    b"HTTP/1.1 201 Created\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();
            requests.push(data);
        }
        requests
    });

    let capture = tmp.path().join("args");
    for push in [false, true] {
        let mut command = command_with_tools(tmp.path(), &bin, &capture);
        command.env("CCGO_MAVEN_URL", &base).args([
            "publish",
            "doc",
            "--artifact-provider",
            "maven",
            "--artifact-group",
            "com.example",
        ]);
        if push {
            command.arg("--push");
        }
        command.assert().success();
        let args = fs::read_to_string(&capture).unwrap();
        assert!(args.starts_with("build\n"));
        assert!(!args.contains("gh-deploy"));
        let site = args
            .lines()
            .skip_while(|line| *line != "--site-dir")
            .nth(1)
            .unwrap();
        assert!(!Path::new(site).exists());
    }
    let requests = server.join().unwrap();
    assert!(String::from_utf8_lossy(&requests[0])
        .starts_with("PUT /releases/com/example/demo-doc/1.2.3/demo-doc-1.2.3.zip HTTP/1.1"));
    let data = &requests[0];
    let end = data.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&data[end..])).unwrap();
    let mut html = String::new();
    zip.by_name("index.html")
        .unwrap()
        .read_to_string(&mut html)
        .unwrap();
    assert_eq!(html, "fresh");
    assert!(String::from_utf8_lossy(&requests[2]).contains("<packaging>zip</packaging>"));
}
