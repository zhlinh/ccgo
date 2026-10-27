//! Verify publish command wiring without contacting package repositories.
#![cfg(unix)]

use assert_cmd::Command;
use predicates::prelude::PredicateBooleanExt;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn test_signing_key() -> String {
    use pgp::composed::{KeyType, SecretKeyParamsBuilder};
    let params = SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519)
        .can_certify(true)
        .can_sign(true)
        .primary_user_id("Tester <test@example.com>".into())
        .build()
        .unwrap();
    let secret = params.generate(rand::thread_rng()).unwrap();
    let signed = secret.sign(rand::thread_rng(), String::new).unwrap();
    signed.to_armored_string(None.into()).unwrap()
}

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
    recorder(&bin.join("ohpm"));
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\ngroup_id='com.example'\n\
         [publish.android.maven]\nartifact_id='demo'\n",
    )
    .unwrap();
    let local = tmp.path().join("local-maven");
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
            let mut command = command_with_tools(tmp.path(), &bin, &capture);
            command.args(["publish", target]);
            if target == "android" {
                command
                    .args(["--registry", "local"])
                    .env("MAVEN_LOCAL_PATH", &local);
            }
            command.arg("--path").arg(input).assert().success();
            if target == "android" {
                // No android/ project exists: a successful run proves the AAR
                // was uploaded directly instead of being handed to Gradle.
                assert_eq!(
                    fs::read(local.join("com/example/demo/1.2.3/demo-1.2.3.aar")).unwrap(),
                    b"prebuilt"
                );
            } else {
                let args = fs::read_to_string(&capture).unwrap();
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
            }
            assert_eq!(fs::read(&zip_path).unwrap(), original_zip);
            assert!(artifact.is_file());
        }
    }
}

#[test]
fn android_aar_publishes_directly_to_local_maven_without_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='stdcomm'\nversion='0.0.13'\ngroup_id='com.cyber.mna'\n\
         [publish.android.maven]\nartifact_id='stdcomm'\nchannel_desc='beta'\n\
         dependencies=['org.example:dep:1.2.3']\n\
         [android]\nstl='c++_shared'\n",
    )
    .unwrap();
    let aar = tmp.path().join("stdcomm-release.aar");
    fs::write(&aar, b"prebuilt-aar").unwrap();
    let local = tmp.path().join("local-maven");

    // --stl wins over CCGO.toml and selects the -stdembed artifact name, just
    // like the Gradle plugin.
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("MAVEN_LOCAL_PATH", &local)
        .args([
            "publish",
            "android",
            "--registry",
            "local",
            "--stl",
            "c++_static",
            "--path",
        ])
        .arg(&aar)
        .assert()
        .success();

    // No android/ directory or gradlew exists: the old implementation failed
    // before it could publish anything.
    let coordinate = local.join("com/cyber/mna/stdcomm-beta-stdembed/0.0.13");
    assert_eq!(
        fs::read(coordinate.join("stdcomm-beta-stdembed-0.0.13.aar")).unwrap(),
        b"prebuilt-aar"
    );
    let pom = fs::read_to_string(coordinate.join("stdcomm-beta-stdembed-0.0.13.pom")).unwrap();
    assert!(pom.contains("<packaging>aar</packaging>"));
    assert!(pom.contains("<name>stdcomm</name>"));
    assert!(pom.contains("<description>The stdcomm SDK</description>"));
    assert!(pom.contains("<artifactId>dep</artifactId>"));
    assert!(!pom.contains("<scope>"));
    assert_eq!(
        fs::read_to_string(coordinate.join("stdcomm-beta-stdembed-0.0.13.aar.sha256"))
            .unwrap()
            .len(),
        64
    );
    assert!(!tmp.path().join("android").exists());
}

#[test]
fn android_official_publishes_a_signed_bundle_to_the_central_portal() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\ngroup_id='com.example'\n\
         description='Demo SDK'\nrepository='https://example.com/demo'\nlicense='MIT'\n\
         authors=['Tester <test@example.com>']\n\
         [publish.android.maven]\nartifact_id='demo'\n",
    )
    .unwrap();
    let aar = tmp.path().join("demo.aar");
    fs::write(&aar, b"prebuilt-aar").unwrap();

    let key = test_signing_key();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                assert!(size > 0);
                data.extend_from_slice(&buffer[..size]);
                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap_or("0")
                        .parse()
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = if String::from_utf8_lossy(&data)
                .starts_with("POST /api/v1/publisher/upload")
            {
                "HTTP/1.1 201 Created\r\nContent-Length: 5\r\nConnection: close\r\n\r\ndep-1"
                    .to_owned()
            } else {
                let body = r#"{"deploymentId":"dep-1","deploymentState":"PUBLISHED"}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            stream.write_all(response.as_bytes()).unwrap();
            requests.push(data);
        }
        requests
    });

    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("CCGO_CENTRAL_URL", &base)
        .env("MAVEN_CENTRAL_USERNAME", "user")
        .env("MAVEN_CENTRAL_PASSWORD", "pass")
        .env("SIGNING_IN_MEMORY_KEY", &key)
        .env("SIGNING_IN_MEMORY_KEY_PASSWORD", "")
        .args(["publish", "android", "--registry", "official", "--path"])
        .arg(&aar)
        .assert()
        .success();

    let requests = server.join().unwrap();
    let upload = String::from_utf8_lossy(&requests[0]);
    assert!(upload.starts_with(
        "POST /api/v1/publisher/upload?name=com.example%3Ademo%3A1.2.3&publishingType=AUTOMATIC"
    ));
    assert!(upload
        .to_lowercase()
        .contains("authorization: bearer dxnlcjpwyxnz"));
    assert!(upload.contains("name=\"bundle\"; filename=\"bundle.zip\""));
    assert!(upload.contains("demo-1.2.3.aar"));
    assert!(upload.contains("demo-1.2.3.pom"));
    assert!(upload.contains("demo-1.2.3.aar.asc"));
    assert!(
        String::from_utf8_lossy(&requests[1]).starts_with("POST /api/v1/publisher/status?id=dep-1")
    );
}

#[test]
fn android_official_reports_missing_configuration_before_building() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\ngroup_id='com.example'\n\
         repository='https://example.com/demo'\n",
    )
    .unwrap();

    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("PATH", &bin)
        .env_remove("MAVEN_CENTRAL_USERNAME")
        .env_remove("MAVEN_CENTRAL_PASSWORD")
        .env_remove("SONATYPE_USERNAME")
        .env_remove("SONATYPE_PASSWORD")
        .env_remove("SIGNING_IN_MEMORY_KEY")
        .env_remove("SIGNING_IN_MEMORY_KEY_PASSWORD")
        .args(["publish", "android", "--registry", "official"])
        .assert()
        .failure()
        .stdout(predicates::str::contains("Building the release AAR").not())
        .stderr(predicates::str::contains(
            "Maven Central publishing is not configured yet",
        ))
        .stderr(predicates::str::contains("Central Portal credentials"))
        .stderr(predicates::str::contains("MAVEN_CENTRAL_USERNAME"))
        .stderr(predicates::str::contains("PGP signatures"))
        .stderr(predicates::str::contains("SIGNING_IN_MEMORY_KEY"));
}

#[test]
fn android_skip_build_without_path_requires_an_explicit_artifact() {
    let tmp = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .args(["publish", "android", "--registry", "local", "--skip-build"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("pass --path"));
}

#[test]
fn explicit_project_directories_select_android_and_ohos_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    recorder(&bin.join("ohpm"));

    let ohos = tmp.path().join("custom-ohos");
    fs::create_dir(&ohos).unwrap();
    fs::write(ohos.join("oh-package.json5"), b"{}").unwrap();
    fs::write(ohos.join("fixture.har"), b"fixture").unwrap();
    let capture = tmp.path().join("args");
    command_with_tools(tmp.path(), &bin, &capture)
        .args(["publish", "ohos", "--path"])
        .arg(&ohos)
        .assert()
        .success();
    let args = fs::read_to_string(capture).unwrap();
    assert_eq!(
        Path::new(args.lines().next().unwrap()),
        fs::canonicalize(&ohos).unwrap()
    );
    assert!(args.contains("prepublish\n"));

    // Android project directories are accepted as build inputs: ccgo resolves
    // the root, then builds with ccgo instead of invoking this wrapper.
    let android = tmp.path().join("custom-android");
    fs::create_dir(&android).unwrap();
    let android_capture = tmp.path().join("android-args");
    let gradlew = android.join("gradlew");
    fs::write(
        &gradlew,
        format!(
            "#!/bin/sh\n: > \"{}\"\nexit 99\n",
            android_capture.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&gradlew, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(
        android.join("CCGO.toml"),
        "[project]\nname = \"sample\"\nversion = \"1.0.0\"\n",
    )
    .unwrap();
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .args(["publish", "android", "--path"])
        .arg(&android)
        .assert()
        .failure();
    assert!(
        !android_capture.exists(),
        "ccgo publish must not invoke the project's gradlew"
    );
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
fn android_publishes_an_absolute_aar_without_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.2.3'\ngroup_id='com.example'\n",
    )
    .unwrap();
    fs::write(tmp.path().join("custom library.aar"), b"aar").unwrap();
    let local = tmp.path().join("local-maven");
    for extra in [Vec::new(), vec!["--skip-build"]] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
        command
            .current_dir(tmp.path())
            .env("MAVEN_LOCAL_PATH", &local)
            .args(["publish", "android", "--registry", "local"])
            .args(extra)
            .args(["--path", "custom library.aar"])
            .assert()
            .success();
        assert_eq!(
            fs::read(local.join("com/example/demo/1.2.3/demo-1.2.3.aar")).unwrap(),
            b"aar"
        );
    }
    assert!(!tmp.path().join("android").exists());
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
fn kmp_project_directory_builds_with_ccgo_without_gradle_publish() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("custom-kmp");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("CCGO.toml"),
        "[project]\nname='demo'\nversion='1.0.0'\n",
    )
    .unwrap();
    let capture = tmp.path().join("kmp-gradle-args");
    let gradlew = project.join("gradlew");
    fs::write(
        &gradlew,
        format!("#!/bin/sh\n: > \"{}\"\nexit 99\n", capture.display()),
    )
    .unwrap();
    fs::set_permissions(&gradlew, fs::Permissions::from_mode(0o755)).unwrap();
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .args(["publish", "kmp", "--path", "custom-kmp"])
        .assert()
        .failure();
    assert!(
        !capture.exists(),
        "ccgo publish must not invoke the KMP project's gradlew"
    );
}

#[test]
fn kmp_sdk_zip_publishes_a_signed_bundle_to_the_central_portal() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("CCGO.toml"), kmp_config()).unwrap();
    let archive = tmp.path().join("SDK.zip");
    write_zip(
        &archive,
        &[
            (
                "meta/kmp/build_info.json",
                br#"{"project":"stdcomm","version":"0.0.5"}"#,
            ),
            ("lib/kmp/android/library.aar", b"prebuilt-aar"),
            ("lib/kmp/desktop/library.jar", b"prebuilt-jar"),
            (
                "lib/kmp/native/iosArm64/klib/stdcomm.klib",
                b"prebuilt-klib",
            ),
            (
                "lib/kmp/native/iosArm64/cinterop/stdcomm-cinterop.klib",
                b"prebuilt-cinterop",
            ),
        ],
    );

    let key = test_signing_key();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                assert!(size > 0);
                data.extend_from_slice(&buffer[..size]);
                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .unwrap_or("0")
                        .parse()
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = if String::from_utf8_lossy(&data)
                .starts_with("POST /api/v1/publisher/upload")
            {
                "HTTP/1.1 201 Created\r\nContent-Length: 5\r\nConnection: close\r\n\r\ndep-2"
                    .to_owned()
            } else {
                let body = r#"{"deploymentId":"dep-2","deploymentState":"PUBLISHED"}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            stream.write_all(response.as_bytes()).unwrap();
            requests.push(data);
        }
        requests
    });

    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("CCGO_CENTRAL_URL", &base)
        .env("MAVEN_CENTRAL_USERNAME", "user")
        .env("MAVEN_CENTRAL_PASSWORD", "pass")
        .env("SIGNING_IN_MEMORY_KEY", &key)
        .env("SIGNING_IN_MEMORY_KEY_PASSWORD", "")
        .args(["publish", "kmp", "--registry", "official", "--path"])
        .arg(&archive)
        .assert()
        .success();

    let requests = server.join().unwrap();
    let upload = String::from_utf8_lossy(&requests[0]);
    assert!(upload.starts_with(
        "POST /api/v1/publisher/upload?name=com.cyber.mna%3Astdcomm-kmp%3A0.0.5&publishingType=AUTOMATIC"
    ));
    assert!(upload
        .to_lowercase()
        .contains("authorization: bearer dxnlcjpwyxnz"));
    for name in [
        "stdcomm-kmp-android-0.0.5.aar",
        "stdcomm-kmp-desktop-0.0.5.jar",
        "stdcomm-kmp-iosarm64-0.0.5.klib",
        "stdcomm-kmp-iosarm64-0.0.5-stdcomm-cinterop.klib",
        "stdcomm-kmp-android-0.0.5.pom",
    ] {
        assert!(upload.contains(name), "missing bundle member {name}");
    }
}

fn kmp_config() -> &'static str {
    "[project]\nname='stdcomm'\nversion='0.0.5'\ngroup_id='com.cyber.mna'\n\
     description='Demo KMP SDK'\nrepository='https://example.com/stdcomm'\n\
     license='MIT'\nauthors=['Tester <test@example.com>']\n\
     [publish.kmp.maven]\nartifact_id='stdcomm-kmp'\n\
     dependencies = [{ group = 'org.example', artifact = 'dep', version = '1.2.3' }]\n"
}

fn kmp_sdk_zip(path: &Path) {
    write_zip(
        path,
        &[
            (
                "meta/kmp/build_info.json",
                br#"{"project":"stdcomm","version":"0.0.5"}"#,
            ),
            ("lib/kmp/android/library.aar", b"prebuilt-aar"),
            ("lib/kmp/desktop/library.jar", b"prebuilt-jar"),
        ],
    );
}

#[test]
fn kmp_sdk_zip_publishes_directly_to_local_maven_without_gradle() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("CCGO.toml"), kmp_config()).unwrap();
    let archive = tmp.path().join("SDK.zip");
    kmp_sdk_zip(&archive);
    let local = tmp.path().join("local-maven");

    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("MAVEN_LOCAL_PATH", &local)
        .args(["publish", "kmp", "--registry", "local", "--path"])
        .arg(&archive)
        .assert()
        .success();

    // No kmp/ project or gradlew exists: the old implementation failed before
    // it could publish anything.
    let android = local.join("com/cyber/mna/stdcomm-kmp-android/0.0.5");
    assert_eq!(
        fs::read(android.join("stdcomm-kmp-android-0.0.5.aar")).unwrap(),
        b"prebuilt-aar"
    );
    let pom = fs::read_to_string(android.join("stdcomm-kmp-android-0.0.5.pom")).unwrap();
    assert!(pom.contains("<packaging>aar</packaging>"));
    assert!(pom.contains("<groupId>org.example</groupId>"));
    assert!(pom.contains("<artifactId>dep</artifactId>"));
    let checksum =
        fs::read_to_string(android.join("stdcomm-kmp-android-0.0.5.aar.sha256")).unwrap();
    assert_eq!(checksum.len(), 64);
    assert_eq!(
        fs::read(
            local.join("com/cyber/mna/stdcomm-kmp-desktop/0.0.5/stdcomm-kmp-desktop-0.0.5.jar")
        )
        .unwrap(),
        b"prebuilt-jar"
    );
    assert!(!tmp.path().join("kmp").exists());
}

#[test]
fn kmp_sdk_zip_private_uploads_directly_without_gradle() {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("CCGO.toml"), kmp_config()).unwrap();
    let archive = tmp.path().join("SDK.zip");
    kmp_sdk_zip(&archive);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}/releases", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..8 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut data = Vec::new();
            let mut buffer = [0; 4096];
            loop {
                let size = stream.read(&mut buffer).unwrap();
                assert!(size > 0);
                data.extend_from_slice(&buffer[..size]);
                if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                    let length: usize = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
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

    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .env("CCGO_MAVEN_USERNAME", "publisher")
        .env("CCGO_MAVEN_PASSWORD", "secret")
        .args([
            "publish",
            "kmp",
            "--registry",
            "private",
            "--remote",
            &base,
            "--path",
        ])
        .arg(&archive)
        .assert()
        .success();

    let requests = server.join().unwrap();
    let text: Vec<_> = requests
        .iter()
        .map(|request| String::from_utf8_lossy(request).into_owned())
        .collect();
    assert!(text.iter().any(|request| request.starts_with(
        "PUT /releases/com/cyber/mna/stdcomm-kmp-android/0.0.5/stdcomm-kmp-android-0.0.5.aar HTTP/1.1"
    )));
    assert!(text.iter().any(|request| request.starts_with(
        "PUT /releases/com/cyber/mna/stdcomm-kmp-desktop/0.0.5/stdcomm-kmp-desktop-0.0.5.jar HTTP/1.1"
    )));
    assert!(text
        .iter()
        .any(|request| request.contains("<packaging>aar</packaging>")
            && request.contains("<artifactId>dep</artifactId>")));
    assert!(text
        .iter()
        .any(|request| request.to_lowercase().contains("authorization: basic ")));
    assert!(requests
        .iter()
        .any(|request| request.ends_with(b"prebuilt-aar")));
    assert!(!tmp.path().join("kmp").exists());
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
fn apple_warns_that_registry_does_not_select_the_maven_destination() {
    let tmp = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .current_dir(tmp.path())
        .args([
            "publish",
            "apple",
            "--artifact-provider",
            "maven",
            "--registry",
            "official",
            "--artifact-group",
            "com.example",
            "--path",
            "missing.zip",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("Use --maven-registry official"));
}

#[test]
fn doc_rejects_registry_instead_of_silently_ignoring_it() {
    let tmp = tempfile::tempdir().unwrap();
    for args in [
        vec![
            "publish",
            "doc",
            "--artifact-provider",
            "maven",
            "--maven-registry",
            "local",
            "--registry",
            "official",
        ],
        vec![
            "publish",
            "doc",
            "--registry",
            "private",
            "--remote",
            "docs",
        ],
    ] {
        Command::new(env!("CARGO_BIN_EXE_ccgo"))
            .current_dir(tmp.path())
            .args(args)
            .assert()
            .failure()
            .stderr(predicates::str::contains("does not use --registry"));
    }
}

#[test]
fn doc_central_uploads_a_signed_bundle() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let tmp = tempfile::tempdir().unwrap();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(
        tmp.path().join("CCGO.toml"),
        "[project]\nname = 'demo'\nversion = '1.2.3'\n\
         repository = 'https://example.com/demo'\nlicense = 'MIT'\n\
         authors = ['Tester <test@example.com>']\n",
    )
    .unwrap();
    fs::write(tmp.path().join("mkdocs.yml"), "site_name: Demo").unwrap();
    let tool = bin.join("mkdocs");
    fs::write(&tool, "#!/bin/sh\nwhile [ \"$#\" -gt 0 ]; do\nif [ \"$1\" = --site-dir ]; then shift; mkdir -p \"$1\"; printf fresh > \"$1/index.html\"; fi\nshift\ndone\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let key = test_signing_key();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
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
                        .unwrap_or("0")
                        .parse()
                        .unwrap();
                    if data.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            let response = if String::from_utf8_lossy(&data)
                .starts_with("POST /api/v1/publisher/upload")
            {
                "HTTP/1.1 201 Created\r\nContent-Length: 5\r\nConnection: close\r\n\r\ndep-3"
                    .to_owned()
            } else {
                let body = r#"{"deploymentId":"dep-3","deploymentState":"PUBLISHED"}"#;
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            stream.write_all(response.as_bytes()).unwrap();
            requests.push(data);
        }
        requests
    });

    let capture = tmp.path().join("args");
    command_with_tools(tmp.path(), &bin, &capture)
        .env("CCGO_CENTRAL_URL", &base)
        .env("MAVEN_CENTRAL_USERNAME", "user")
        .env("MAVEN_CENTRAL_PASSWORD", "pass")
        .env("SIGNING_IN_MEMORY_KEY", &key)
        .env("SIGNING_IN_MEMORY_KEY_PASSWORD", "")
        .args([
            "publish",
            "doc",
            "--artifact-provider",
            "maven",
            "--maven-registry",
            "official",
            "--artifact-group",
            "com.example",
            "--push",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "https://repo1.maven.org/maven2/com/example/demo-doc/1.2.3/demo-doc-1.2.3.zip",
        ));

    let requests = server.join().unwrap();
    let upload = String::from_utf8_lossy(&requests[0]);
    assert!(upload.starts_with(
        "POST /api/v1/publisher/upload?name=com.example%3Ademo-doc%3A1.2.3&publishingType=AUTOMATIC"
    ));
    assert!(upload.contains("demo-doc-1.2.3.zip"));
    assert!(upload.contains("demo-doc-1.2.3.pom"));
    assert!(upload.contains("demo-doc-1.2.3.zip.asc"));
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
