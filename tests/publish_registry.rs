//! Exercise registry selection without contacting external package services.
#![cfg(unix)]
use assert_cmd::Command;
use predicates::str::contains;
use std::{fs, os::unix::fs::PermissionsExt, path::Path};

fn tool(path: &Path, body: &str) {
    fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
fn command(root: &Path, bin: &Path) -> Command {
    let paths = std::env::join_paths(
        std::iter::once(bin.to_owned())
            .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    cmd.current_dir(root)
        .env("PATH", paths)
        .env("CCGO_HOME", root.join("ccgo-home"))
        .env("PUBLISH_CAPTURE", root.join("commands"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "CCGO Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "CCGO Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid");
    cmd
}
fn git(root: &Path, args: &[&str]) -> String {
    let output = std::process::Command::new("git")
        .current_dir(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "CCGO Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "CCGO Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}
fn project(root: &Path) -> std::path::PathBuf {
    let project = root.join("project");
    fs::create_dir(&project).unwrap();
    fs::write(
        project.join("CCGO.toml"),
        "[package]\nname='demo'\nversion='1.2.3'\n",
    )
    .unwrap();
    git(&project, &["init"]);
    git(&project, &["add", "CCGO.toml"]);
    git(&project, &["commit", "-m", "initial"]);
    git(&project, &["tag", "v1.2.3"]);
    git(
        &project,
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/demo.git",
        ],
    );
    project
}

#[test]
fn ohpm_local_validates_and_remote_destinations_are_explicit() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(
        &bin.join("ohpm"),
        "printf '%s\\n' \"$@\" > \"$PUBLISH_CAPTURE\"",
    );
    fs::write(root.join("demo.har"), "fixture").unwrap();
    for (registry, url) in [
        ("local", None),
        ("official", None),
        ("private", Some("https://ohpm.example/private/")),
    ] {
        let mut cmd = command(root, &bin);
        cmd.args([
            "publish",
            "ohos",
            "--path",
            "demo.har",
            "--registry",
            registry,
        ]);
        if let Some(url) = url {
            cmd.args(["--remote", url]);
        }
        cmd.assert().success();
        let actual = fs::read_to_string(root.join("commands")).unwrap();
        if registry == "local" {
            assert!(actual.starts_with("prepublish\n"));
            assert!(!actual.contains("--publish_registry"));
        } else {
            assert!(actual.starts_with("publish\n--publish_registry\n"));
            assert!(actual.contains(url.unwrap_or("https://ohpm.openharmony.cn/ohpm/")));
        }
    }
    command(root, &bin)
        .args([
            "publish",
            "ohos",
            "--path",
            "demo.har",
            "--registry",
            "private",
        ])
        .assert()
        .failure()
        .stderr(contains("requires --remote"));
}

#[test]
fn conan_official_never_falls_back_and_private_uploads_exact_recipe() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(&bin.join("conan"), "printf '%s\\n' \"$@\" >> \"$PUBLISH_CAPTURE\"\nif [ \"$1\" = export ]; then echo '{\"reference\":\"demo/1.2.3@ccgo/stable#revision\"}'; fi");
    fs::write(root.join("conanfile.py"), "fixture").unwrap();
    command(root, &bin)
        .args([
            "publish",
            "conan",
            "--skip-build",
            "--path",
            "conanfile.py",
            "--registry",
            "official",
            "--remote",
            "company",
        ])
        .assert()
        .failure()
        .stderr(contains("ConanCenter does not accept direct CLI uploads"));
    assert!(!root.join("commands").exists());
    command(root, &bin)
        .args([
            "publish",
            "conan",
            "--skip-build",
            "--path",
            "conanfile.py",
            "--registry",
            "private",
        ])
        .assert()
        .failure()
        .stderr(contains("requires --remote"));
    assert!(!root.join("commands").exists());
    command(root, &bin)
        .args([
            "publish",
            "conan",
            "--skip-build",
            "--path",
            "conanfile.py",
            "--registry",
            "private",
            "--remote",
            "company",
        ])
        .assert()
        .success();
    let actual = fs::read_to_string(root.join("commands")).unwrap();
    assert!(actual.contains(
        "upload\ndemo/1.2.3@ccgo/stable#revision\n--only-recipe\n--remote\ncompany\n--confirm"
    ));
    assert!(!actual.lines().any(|line| line == "*"));
}

#[test]
fn spm_local_validates_and_private_pushes_only_selected_remote() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(
        &bin.join("swift"),
        "printf '%s\\n' \"$@\" > \"$PUBLISH_CAPTURE\"",
    );
    let source = project(root);
    fs::write(source.join("Package.swift"), "// test manifest").unwrap();
    git(&source, &["add", "Package.swift"]);
    git(&source, &["commit", "-m", "wrapper"]);
    git(&source, &["tag", "-d", "v1.2.3"]);
    let remote = root.join("spm.git");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    git(
        &source,
        &["remote", "add", "company", remote.to_str().unwrap()],
    );
    command(&source, &bin)
        .args(["publish", "apple", "--manager", "spm"])
        .assert()
        .success();
    assert!(fs::read_to_string(source.join("commands"))
        .unwrap()
        .starts_with("package\ndump-package"));
    // Keep test capture outside the wrapper, which must be fully committed.
    for args in [
        vec!["--registry", "official"],
        vec!["--registry", "private"],
    ] {
        command(&source, &bin)
            .args(["publish", "apple", "--manager", "spm"])
            .args(args)
            .env("PUBLISH_CAPTURE", root.join("commands"))
            .assert()
            .failure();
    }
    fs::remove_file(source.join("commands")).unwrap();
    command(&source, &bin)
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--registry",
            "private",
            "--remote",
            "company",
            "--push",
        ])
        .env("PUBLISH_CAPTURE", root.join("commands"))
        .env("CCGO_HOME", root.join("ccgo-home"))
        .assert()
        .success();
    let head = git(&source, &["rev-parse", "HEAD"]);
    assert_eq!(git(&remote, &["rev-parse", "refs/tags/v1.2.3"]), head);
    command(&source, &bin)
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--registry",
            "private",
            "--remote",
            "company",
            "--push",
        ])
        .env("PUBLISH_CAPTURE", root.join("commands"))
        .env("CCGO_HOME", root.join("ccgo-home"))
        .assert()
        .success();
    assert_eq!(git(&source, &["rev-parse", "refs/tags/v1.2.3"]), head);
    assert_eq!(git(&remote, &["rev-parse", "refs/tags/v1.2.3"]), head);
    // Existing tags on a different commit must not be silently pushed.
    git(&source, &["commit", "--allow-empty", "-m", "new head"]);
    command(&source, &bin)
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--registry",
            "private",
            "--remote",
            "company",
            "--push",
        ])
        .env("PUBLISH_CAPTURE", root.join("commands"))
        .assert()
        .failure()
        .stderr(contains("different commit"));
    assert_eq!(git(&remote, &["rev-parse", "refs/tags/v1.2.3"]), head);
}

#[test]
fn index_local_is_offline_and_private_resolves_named_registry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let source = project(root);
    let ccgo_home = root.join("ccgo-home");
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args(["publish", "index"])
        .assert()
        .success()
        .stdout(contains("No index files, commits, or remotes changed"));
    let local = ccgo_home.join("registry/publish/local");
    assert!(!local.exists());
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args(["publish", "index", "--push"])
        .assert()
        .success()
        .stdout(contains("no remote was contacted"));
    assert!(local.join("de/mo/demo.json").is_file());
    let entry: serde_json::Value =
        serde_json::from_slice(&fs::read(local.join("de/mo/demo.json")).unwrap()).unwrap();
    assert_eq!(entry["versions"][0]["version"], "1.2.3");
    assert_eq!(entry["versions"][0]["tag"], "v1.2.3");
    assert_eq!(git(&local, &["remote"]), "");
    command(&source, &bin)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.3",
            "--registry",
            "private",
        ])
        .assert()
        .failure()
        .stderr(contains("requires --remote"));
    let remote = root.join("index.git");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    let config = format!(
        "[package]\nname='demo'\nversion='1.2.3'\n[registries]\ncompany='{}'\n",
        remote.display()
    );
    fs::write(source.join("CCGO.toml"), config).unwrap();
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.3",
            "--registry",
            "private",
            "--remote",
            "company",
            "--push",
        ])
        .assert()
        .success();
    assert!(git(&remote, &["show", "HEAD:de/mo/demo.json"]).contains("1.2.3"));
    // The old switch is an alias for the same push flag.
    git(&source, &["tag", "v1.2.4"]);
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.4",
            "--registry",
            "private",
            "--remote",
            "company",
            "--index-push",
        ])
        .assert()
        .success();
    let versions: serde_json::Value =
        serde_json::from_str(&git(&remote, &["show", "HEAD:de/mo/demo.json"])).unwrap();
    assert_eq!(versions["versions"].as_array().unwrap().len(), 2);
}

#[test]
fn index_official_rejects_overrides_before_remote_access() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let source = project(root);
    command(&source, &bin)
        .args([
            "publish",
            "index",
            "--registry",
            "official",
            "--remote",
            "company",
            "--index-version",
            "1.2.3",
        ])
        .assert()
        .failure()
        .stderr(contains("Official index is fixed to"));
    command(&source, &bin)
        .args([
            "publish",
            "index",
            "--registry",
            "private",
            "--remote",
            "../outside",
            "--index-version",
            "1.2.3",
        ])
        .assert()
        .failure()
        .stderr(contains("Invalid index registry name"));
}

#[test]
fn index_private_reads_global_registration_and_retains_failed_pull_clone() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let source = project(root);
    let ccgo_home = root.join("ccgo-home");
    let registry_dir = ccgo_home.join("registry");
    fs::create_dir_all(&registry_dir).unwrap();
    let remote = root.join("index.git");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    let configs = serde_json::json!([{"name":"global-company","url":remote.to_str().unwrap(),"enabled":true}]);
    fs::write(
        registry_dir.join("registries.json"),
        serde_json::to_vec(&configs).unwrap(),
    )
    .unwrap();
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.3",
            "--registry",
            "private",
            "--remote",
            "global-company",
            "--push",
        ])
        .assert()
        .success();
    let clone = registry_dir.join("publish/global-company");
    fs::write(clone.join("keep.txt"), "unpublished local work").unwrap();
    git(&clone, &["add", "keep.txt"]);
    git(&clone, &["commit", "-m", "local only"]);
    let local_head = git(&clone, &["rev-parse", "HEAD"]);
    // Make the remote unreachable without changing the selected destination URL.
    fs::rename(&remote, root.join("index-offline.git")).unwrap();
    git(&source, &["tag", "v1.2.4"]);
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.4",
            "--registry",
            "private",
            "--remote",
            "global-company",
            "--push",
        ])
        .assert()
        .failure()
        .stderr(contains("Local changes are preserved"));
    assert_eq!(git(&clone, &["rev-parse", "HEAD"]), local_head);
    assert_eq!(
        fs::read_to_string(clone.join("keep.txt")).unwrap(),
        "unpublished local work"
    );
}

#[test]
fn index_commit_failure_stops_before_remote_push() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let source = project(root);
    let remote = root.join("index.git");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    let mut config = fs::read_to_string(source.join("CCGO.toml")).unwrap();
    config.push_str(&format!("[registries]\ncompany='{}'\n", remote.display()));
    fs::write(source.join("CCGO.toml"), config).unwrap();
    let ccgo_home = root.join("ccgo-home");
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.3",
            "--registry",
            "private",
            "--remote",
            "company",
            "--push",
        ])
        .assert()
        .success();
    let before = git(&remote, &["rev-parse", "HEAD"]);
    let hook = ccgo_home.join("registry/publish/company/.git/hooks/pre-commit");
    tool(&hook, "exit 1");
    git(&source, &["tag", "v1.2.4"]);
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--index-version",
            "1.2.4",
            "--registry",
            "private",
            "--remote",
            "company",
            "--push",
        ])
        .assert()
        .failure()
        .stderr(contains("Failed to commit index changes"));
    assert_eq!(git(&remote, &["rev-parse", "HEAD"]), before);
}

#[test]
fn maven_remote_overrides_environment_for_android_and_kmp() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    for target in ["android", "kmp"] {
        let project = root.join(target);
        fs::create_dir(&project).unwrap();
        tool(
            &project.join("gradlew"),
            "printf '%s' \"$CCGO_MAVEN_URL\" > \"$PUBLISH_CAPTURE\"",
        );
        for flag in ["--remote", "--url", "--remote-name"] {
            command(root, &bin)
                .env("CCGO_MAVEN_URL", "https://old.example/maven/")
                .args([
                    "publish",
                    target,
                    "--registry",
                    "private",
                    flag,
                    "https://new.example/maven/",
                    "--path",
                ])
                .arg(&project)
                .assert()
                .success();
            assert_eq!(
                fs::read_to_string(root.join("commands")).unwrap(),
                "https://new.example/maven/"
            );
        }
    }
}

#[test]
fn spm_generic_version_tag_validate_without_changes_and_publish_locally() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(
        &bin.join("swift"),
        "printf '%s\\n' \"$@\" > \"$PUBLISH_CAPTURE\"",
    );
    let source = project(root);
    fs::write(source.join("Package.swift"), "// test manifest").unwrap();
    git(&source, &["add", "Package.swift"]);
    git(&source, &["commit", "-m", "wrapper"]);
    let head = git(&source, &["rev-parse", "HEAD"]);
    let tags = git(&source, &["tag"]);
    for flags in [
        vec!["--version", "2.0.0", "--tag", "2.0.0"],
        vec!["--index-version", "2.0.0", "--index-tag", "2.0.0"],
    ] {
        command(&source, &bin)
            .env("PUBLISH_CAPTURE", root.join("commands"))
            .args(["publish", "apple", "--manager", "spm"])
            .args(flags)
            .assert()
            .success()
            .stdout(contains("version 2.0.0, tag 2.0.0"));
        assert_eq!(git(&source, &["rev-parse", "HEAD"]), head);
        assert_eq!(git(&source, &["tag"]), tags);
    }
    command(&source, &bin)
        .env("PUBLISH_CAPTURE", root.join("commands"))
        .args([
            "publish",
            "apple",
            "--manager",
            "spm",
            "--version",
            "2.0.0",
            "--tag",
            "2.0.0",
            "--push",
        ])
        .assert()
        .success()
        .stdout(contains("no remote was contacted"));
    assert_eq!(git(&source, &["rev-parse", "refs/tags/2.0.0"]), head);
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .arg("--version")
        .assert()
        .success()
        .stdout(contains("ccgo "));
    Command::new(env!("CARGO_BIN_EXE_ccgo"))
        .args([
            "publish",
            "index",
            "--version",
            "1.0.0",
            "--index-version",
            "2.0.0",
        ])
        .assert()
        .failure()
        .stderr(contains("cannot be used multiple times"));
}

#[test]
fn index_private_validation_does_not_clone_commit_or_create_output() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let source = project(root);
    let mut config = fs::read_to_string(source.join("CCGO.toml")).unwrap();
    config.push_str("[registries]\ncompany='https://example.invalid/index.git'\n");
    fs::write(source.join("CCGO.toml"), config).unwrap();
    let ccgo_home = root.join("ccgo-home");
    let head = git(&source, &["rev-parse", "HEAD"]);
    command(&source, &bin)
        .env("CCGO_HOME", &ccgo_home)
        .args([
            "publish",
            "index",
            "--registry",
            "private",
            "--remote",
            "company",
            "--version",
            "1.2.3",
            "--tag",
            "v1.2.3",
        ])
        .assert()
        .success()
        .stdout(contains("No index files, commits, or remotes changed"));
    assert!(!ccgo_home.join("registry/publish").exists());
    assert_eq!(git(&source, &["rev-parse", "HEAD"]), head);
}

#[test]
fn apple_all_without_push_does_not_publish_private_specs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(&bin.join("swift"), "exit 0");
    tool(
        &bin.join("pod"),
        "printf '%s\\n' \"$@\" > \"$PUBLISH_CAPTURE\"",
    );
    let source = project(root);
    fs::write(source.join("Package.swift"), "// fixture").unwrap();
    fs::write(source.join("Demo.podspec"), "fixture").unwrap();
    command(&source, &bin)
        .env("PUBLISH_CAPTURE", root.join("commands"))
        .args([
            "publish",
            "apple",
            "--manager",
            "all",
            "--registry",
            "private",
            "--remote",
            "origin",
        ])
        .assert()
        .success();
    let actual = fs::read_to_string(root.join("commands")).unwrap();
    assert!(actual.starts_with("lib\nlint\n"));
    assert!(!actual.lines().any(|line| line == "push"));
}

fn package_zip(path: &Path) {
    use std::io::Write;
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut zip = zip::ZipWriter::new(fs::File::create(path).unwrap());
    zip.start_file("CCGO.toml", zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"[package]\nname='demo'\nversion='1.2.3'\n")
        .unwrap();
    zip.finish().unwrap();
}

#[test]
fn index_github_upload_populates_private_index_and_rejects_duplicates_before_upload() {
    use sha2::{Digest, Sha256};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(&bin.join("gh"), "printf '%s\\n' \"$@\" >> \"$PUBLISH_CAPTURE\"\nif [ \"$2\" = upload ]; then\nfor arg in \"$@\"; do case \"$arg\" in *.zip) cp \"$arg\" \"$PUBLISH_ARCHIVE\"; printf '%s' \"$arg\" > \"$PUBLISH_TEMP\";; esac; done\nfi");
    let source = project(root);
    let archive = source.join("target/release/package/DEMO_CCGO_PACKAGE-1.2.3.zip");
    package_zip(&archive);
    let original = fs::read(&archive).unwrap();
    let remote = root.join("index.git");
    git(root, &["init", "--bare", remote.to_str().unwrap()]);
    let mut config = fs::read_to_string(source.join("CCGO.toml")).unwrap();
    config.push_str(&format!("[registries]\ncompany='{}'\n", remote.display()));
    fs::write(source.join("CCGO.toml"), config).unwrap();
    let home = root.join("ccgo-home");
    let uploaded = root.join("uploaded.zip");
    let temp_path = root.join("temporary-path");
    let args = [
        "publish",
        "index",
        "--registry",
        "private",
        "--remote",
        "company",
        "--artifact-provider",
        "github",
        "--github-repo",
        "company/sdk",
        "--push",
    ];
    command(&source, &bin)
        .env("CCGO_HOME", &home)
        .env("PUBLISH_CAPTURE", root.join("gh-args"))
        .env("PUBLISH_ARCHIVE", &uploaded)
        .env("PUBLISH_TEMP", &temp_path)
        .args(args)
        .assert()
        .success();
    let entry: serde_json::Value =
        serde_json::from_str(&git(&remote, &["show", "HEAD:de/mo/demo.json"])).unwrap();
    let version = &entry["versions"][0];
    assert_eq!(
        version["archive_url"],
        "https://github.com/company/sdk/releases/download/v1.2.3/demo-ccgo-1.2.3.zip"
    );
    assert_eq!(version["archive_format"], "zip");
    assert_eq!(
        version["checksum"],
        format!("sha256:{:x}", Sha256::digest(&original))
    );
    assert_eq!(fs::read(uploaded).unwrap(), original);
    assert_eq!(fs::read(archive).unwrap(), original);
    assert!(!Path::new(&fs::read_to_string(temp_path).unwrap()).exists());
    let before = fs::read(root.join("gh-args")).unwrap();
    command(&source, &bin)
        .env("CCGO_HOME", &home)
        .env("PUBLISH_CAPTURE", root.join("gh-args"))
        .args(args)
        .assert()
        .failure()
        .stderr(contains("already in the index"));
    assert_eq!(fs::read(root.join("gh-args")).unwrap(), before);
}

#[test]
fn index_provider_validation_and_missing_tags_never_upload() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(
        &bin.join("gh"),
        "printf called > \"$PUBLISH_CAPTURE\"\nexit 9",
    );
    let source = project(root);
    let archive = root.join("SDK.zip");
    package_zip(&archive);
    let home = root.join("ccgo-home");
    let args = [
        "publish",
        "index",
        "--artifact-provider",
        "github",
        "--github-repo",
        "company/sdk",
        "--path",
    ];
    command(&source, &bin)
        .env("CCGO_HOME", &home)
        .env("PUBLISH_CAPTURE", root.join("gh-args"))
        .args(args)
        .arg(&archive)
        .assert()
        .success();
    assert!(!home.join("registry/publish").exists());
    assert!(!root.join("gh-args").exists());
    git(&source, &["tag", "-d", "v1.2.3"]);
    command(&source, &bin)
        .env("CCGO_HOME", &home)
        .env("PUBLISH_CAPTURE", root.join("gh-args"))
        .args(args)
        .arg(&archive)
        .arg("--push")
        .assert()
        .failure()
        .stderr(contains("tag 'v1.2.3' not found"));
    assert!(!root.join("gh-args").exists());
    assert!(!home.join("registry/publish").exists());
}

#[test]
fn index_provider_upload_failure_does_not_write_a_version_entry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    tool(&bin.join("gh"), "if [ \"$2\" = upload ]; then\nfor arg in \"$@\"; do case \"$arg\" in *.zip) printf '%s' \"$arg\" > \"$PUBLISH_TEMP\";; esac; done\nexit 1\nfi");
    let source = project(root);
    let archive = root.join("SDK.zip");
    package_zip(&archive);
    let home = root.join("ccgo-home");
    let temporary = root.join("temporary-path");
    command(&source, &bin)
        .env("CCGO_HOME", &home)
        .env("PUBLISH_TEMP", &temporary)
        .args([
            "publish",
            "index",
            "--artifact-provider",
            "github",
            "--github-repo",
            "company/sdk",
            "--path",
        ])
        .arg(&archive)
        .arg("--push")
        .assert()
        .failure()
        .stderr(contains("asset upload failed"));
    assert!(!home.join("registry/publish/local/de/mo/demo.json").exists());
    assert!(!Path::new(&fs::read_to_string(temporary).unwrap()).exists());
    assert!(archive.exists());
}

#[test]
fn index_maven_upload_uses_zip_coordinates_and_automatic_checksum() {
    use sha2::{Digest, Sha256};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    let source = project(root);
    let archive = root.join("SDK.zip");
    package_zip(&archive);
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
            let mut buffer = [0; 4096];
            loop {
                let read = stream.read(&mut buffer).unwrap();
                assert!(read > 0);
                data.extend_from_slice(&buffer[..read]);
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
    let home = root.join("ccgo-home");
    command(&source, &bin)
        .env("CCGO_HOME", &home)
        .env("CCGO_MAVEN_URL", &base)
        .env("CCGO_MAVEN_USERNAME", "publisher")
        .env("CCGO_MAVEN_PASSWORD", "secret")
        .args([
            "publish",
            "index",
            "--artifact-provider",
            "maven",
            "--artifact-group",
            "com.example",
            "--path",
        ])
        .arg(&archive)
        .arg("--push")
        .assert()
        .success();
    let requests = server.join().unwrap();
    assert!(String::from_utf8_lossy(&requests[0])
        .starts_with("PUT /releases/com/example/demo-ccgo/1.2.3/demo-ccgo-1.2.3.zip HTTP/1.1"));
    assert!(requests[0].ends_with(&original));
    assert!(String::from_utf8_lossy(&requests[0])
        .to_lowercase()
        .contains("authorization: basic "));
    assert!(String::from_utf8_lossy(&requests[2]).contains("<artifactId>demo-ccgo</artifactId>"));
    let entry: serde_json::Value = serde_json::from_slice(
        &fs::read(home.join("registry/publish/local/de/mo/demo.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        entry["versions"][0]["archive_url"],
        format!("{base}/com/example/demo-ccgo/1.2.3/demo-ccgo-1.2.3.zip")
    );
    assert_eq!(
        entry["versions"][0]["checksum"],
        format!("sha256:{:x}", Sha256::digest(&original))
    );
    assert!(!entry.to_string().contains("secret"));
    assert_eq!(fs::read(archive).unwrap(), original);
}

#[test]
fn conan_binary_matrix_uploads_exact_packages_after_all_creates() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(root.join("conanfile.py"), "fixture").unwrap();
    tool(
        &bin.join("conan"),
        r#"
printf '%s\n' "$@" >> "$PUBLISH_CAPTURE"
case "$1" in
export) echo '{"reference":"demo/1.2.3@ccgo/stable#revision"}' ;;
create)
    if [ "$FAIL_CREATE" = 1 ]; then exit 3; fi
    package=$(printf '%s' "$*" | cksum | cut -d ' ' -f 1)
    printf '{"graph":{"nodes":{"0":{"ref":"dependency/1#r","package_id":"unrelated","prev":"r"},"1":{"ref":"demo/1.2.3@ccgo/stable#revision","package_id":"%s","prev":"p"}}}}' "$package"
    ;;
upload)
    shift
    test "$1" = --list || exit 4
    cp "$2" "$PACKAGE_CAPTURE"
    ;;
esac
"#,
    );
    for fail in [true, false] {
        let mut cmd = command(root, &bin);
        cmd.env("PACKAGE_CAPTURE", root.join("packages.json"))
            .env("FAIL_CREATE", if fail { "1" } else { "0" })
            .args([
                "publish",
                "conan",
                "--path",
                "conanfile.py",
                "--registry",
                "private",
                "--remote",
                "company",
                "--profile",
                "linux,windows",
                "--build-profile",
                "builder",
                "--conan-build-types",
                "Debug,Release",
                "--link-type",
                "both",
                "--conan-setting",
                "compiler.cppstd=17",
                "--conan-option",
                "*:fPIC=True",
            ]);
        if fail {
            cmd.assert().failure();
            assert!(!root.join("packages.json").exists());
            assert!(!fs::read_to_string(root.join("commands"))
                .unwrap()
                .lines()
                .any(|l| l == "upload"));
            fs::remove_file(root.join("commands")).unwrap();
        } else {
            cmd.assert().success();
        }
    }
    let args = fs::read_to_string(root.join("commands")).unwrap();
    assert_eq!(args.lines().filter(|l| *l == "create").count(), 8);
    assert_eq!(args.lines().filter(|l| *l == "upload").count(), 1);
    assert!(args.contains("-pr:b=builder\n"));
    assert!(args.contains("-s:h=compiler.cppstd=17\n"));
    let packages: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("packages.json")).unwrap()).unwrap();
    let cache = packages["Local Cache"].as_object().unwrap();
    assert_eq!(cache.len(), 1);
    assert_eq!(
        cache["demo/1.2.3@ccgo/stable"]["revisions"]["revision"]["packages"]
            .as_object()
            .unwrap()
            .len(),
        8
    );
}

#[test]
fn conan_rejects_invalid_matrix_before_export() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let bin = root.join("bin");
    fs::create_dir(&bin).unwrap();
    fs::write(root.join("conanfile.py"), "fixture").unwrap();
    tool(&bin.join("conan"), "echo invoked >> \"$PUBLISH_CAPTURE\"");
    for args in [
        vec!["--profile", "default,,other"],
        vec!["--build-profile", " "],
        vec!["--conan-build-types", "Invalid"],
        vec!["--conan-setting", "build_type=Release"],
        vec!["--conan-option", "*:shared=True"],
        vec!["--link-type", "invalid"],
        vec!["--registry", "private", "--remote", " "],
    ] {
        command(root, &bin)
            .args(["publish", "conan", "--path", "conanfile.py"])
            .args(args)
            .assert()
            .failure();
        assert!(!root.join("commands").exists());
    }
}
