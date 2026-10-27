//! Real CLI regressions for registry identity, locked content and yank semantics.
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn isolated(command: &mut Command) -> &mut Command {
    command
        .env("RUST_BACKTRACE", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env(
            "GIT_CONFIG_GLOBAL",
            if cfg!(windows) { "NUL" } else { "/dev/null" },
        )
        .env("GIT_AUTHOR_NAME", "Registry Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Registry Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn failure(output: Output, expected: &str) {
    assert!(!output.status.success(), "command unexpectedly succeeded");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(expected),
        "expected {expected:?}, stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = success(
        isolated(Command::new("git").current_dir(root))
            .args(args)
            .output()
            .unwrap(),
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn file_url(path: &Path) -> String {
    reqwest::Url::from_file_path(path).unwrap().to_string()
}

struct Fixture {
    tmp: tempfile::TempDir,
    seed: PathBuf,
    remote: PathBuf,
    consumer: PathBuf,
    archive: PathBuf,
    entry: serde_json::Value,
}

impl Fixture {
    fn new() -> Self {
        let tmp = tempfile::tempdir().unwrap();
        let seed = tmp.path().join("seed");
        let remote = tmp.path().join("index.git");
        let consumer = tmp.path().join("consumer");
        fs::create_dir(&seed).unwrap();
        fs::create_dir(&consumer).unwrap();
        git(
            tmp.path(),
            &[
                "init",
                "--bare",
                "--initial-branch=main",
                remote.to_str().unwrap(),
            ],
        );
        git(&seed, &["init", "-b", "main"]);
        git(&seed, &["remote", "add", "origin", &file_url(&remote)]);
        let entry = serde_json::json!({
            "name": "leaf", "description": "test", "repository": "", "license": "MIT",
            "keywords": [], "platforms": [], "versions": [{
                "version": "1.0.0", "tag": "v1.0.0", "archive_format": "zip", "yanked": false
            }]
        });
        let mut fixture = Self {
            archive: tmp.path().join("leaf.zip"),
            tmp,
            seed,
            remote,
            consumer,
            entry,
        };
        let checksum = fixture.zip("original");
        fixture.entry["versions"][0]["checksum"] = checksum.into();
        fixture.entry["versions"][0]["archive_url"] = file_url(&fixture.archive).into();
        fixture.publish_index();
        fixture.manifest(&[("company", file_url(&fixture.remote))], Some("company"));
        fixture
    }

    fn zip(&self, payload: &str) -> String {
        let mut zip = zip::ZipWriter::new(fs::File::create(&self.archive).unwrap());
        for (name, contents) in [
            ("CCGO.toml", "[package]\nname='leaf'\nversion='1.0.0'\n"),
            ("payload.txt", payload),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        format!(
            "sha256:{:x}",
            Sha256::digest(fs::read(&self.archive).unwrap())
        )
    }

    fn publish_index(&self) {
        let path = self.seed.join("le/af/leaf.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(&self.entry).unwrap()).unwrap();
        git(&self.seed, &["add", "-A"]);
        git(&self.seed, &["commit", "-m", "index fixture"]);
        git(&self.seed, &["push", "origin", "HEAD"]);
    }

    fn manifest(&self, registries: &[(&str, String)], selected: Option<&str>) {
        let mut manifest = "[package]\nname='consumer'\nversion='1.0.0'\n[registries]\n".to_owned();
        for (name, url) in registries {
            manifest.push_str(&format!("{name}='{url}'\n"));
        }
        manifest.push_str("[[dependencies]]\nname='leaf'\nversion='1.0.0'\n");
        if let Some(name) = selected {
            manifest.push_str(&format!("registry='{name}'\n"));
        }
        fs::write(self.consumer.join("CCGO.toml"), manifest).unwrap();
    }

    fn ccgo_at(&self, cwd: &Path, args: &[&str]) -> Output {
        isolated(Command::new(env!("CARGO_BIN_EXE_ccgo")).current_dir(cwd))
            .env("CCGO_HOME", self.tmp.path().join("home"))
            .args(args)
            .output()
            .unwrap()
    }

    fn fetch(&self, args: &[&str]) -> Output {
        let mut command = vec!["fetch"];
        command.extend_from_slice(args);
        self.ccgo_at(&self.consumer, &command)
    }

    fn lock(&self) -> Vec<u8> {
        fs::read(self.consumer.join("CCGO.lock")).unwrap()
    }

    fn payload(&self) -> String {
        fs::read_to_string(self.consumer.join(".ccgo/deps/leaf/payload.txt")).unwrap()
    }

    fn yank(&self) {
        success(self.ccgo_at(
            &self.consumer,
            &[
                "yank",
                "--package",
                "leaf",
                "--vers",
                "1.0.0",
                "--reason",
                "test",
                "--index-repo",
                &file_url(&self.remote),
                "--index-name",
                "yank",
                "--push",
            ],
        ));
    }
}

#[test]
fn generated_registry_lock_can_be_refetched_without_any_lock_changes() {
    let fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    for args in [vec!["--locked"], vec!["--locked", "--force"]] {
        success(fixture.fetch(&args));
        assert_eq!(fixture.payload(), "original");
        assert_eq!(fixture.lock(), before);
    }
}

#[test]
fn yank_excludes_new_resolution_but_preserves_locked_retrieval() {
    let fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fixture.yank();
    failure(fixture.fetch(&["--force"]), "no non-yanked match");
    success(fixture.fetch(&["--locked", "--force"]));
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), before);
}

#[test]
fn changed_archive_bytes_fail_and_preserve_installed_content_and_lock() {
    let fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fixture.zip("replacement");
    for args in [vec!["--locked"], vec!["--locked", "--force"]] {
        failure(fixture.fetch(&args), "checksum mismatch");
        assert_eq!(fixture.payload(), "original");
        assert_eq!(fixture.lock(), before);
    }
}

#[test]
fn rewriting_index_and_archive_cannot_replace_the_locked_checksum() {
    let mut fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fixture.entry["versions"][0]["checksum"] = fixture.zip("replacement").into();
    fixture.publish_index();
    failure(
        fixture.fetch(&["--locked", "--force"]),
        "Index checksum changed",
    );
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), before);
}

#[test]
fn absent_index_checksum_still_produces_and_enforces_a_content_lock() {
    let mut fixture = Fixture::new();
    fixture.entry["versions"][0]
        .as_object_mut()
        .unwrap()
        .remove("checksum");
    fixture.publish_index();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    assert!(String::from_utf8_lossy(&before).contains("sha256:"));
    success(fixture.fetch(&["--locked", "--force"]));
    fixture.zip("replacement");
    failure(fixture.fetch(&["--locked", "--force"]), "checksum mismatch");
    assert_eq!(fixture.lock(), before);
}

#[test]
fn a_different_download_locator_can_serve_the_same_locked_bytes() {
    let mut fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    let mirror = fixture.tmp.path().join("mirror.zip");
    fs::copy(&fixture.archive, &mirror).unwrap();
    fixture.entry["versions"][0]["archive_url"] = file_url(&mirror).into();
    fixture.publish_index();
    success(fixture.fetch(&["--locked", "--force"]));
    assert_eq!(fixture.lock(), before);
}

#[test]
fn changing_registry_url_is_rejected_even_with_an_existing_installation() {
    let fixture = Fixture::new();
    let other = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fixture.manifest(&[("company", file_url(&other.remote))], Some("company"));
    failure(fixture.fetch(&["--locked"]), "Registry source changed");
    assert_eq!(fixture.lock(), before);
    assert_eq!(fixture.payload(), "original");
    failure(fixture.fetch(&["--force"]), "different origin");
}

#[test]
fn missing_locked_version_never_falls_back_to_another_registry() {
    let mut fixture = Fixture::new();
    let other = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fixture.entry["versions"] = serde_json::json!([]);
    fixture.publish_index();
    fixture.manifest(
        &[
            ("company", file_url(&fixture.remote)),
            ("other", file_url(&other.remote)),
        ],
        None,
    );
    failure(
        fixture.fetch(&["--locked", "--force"]),
        "missing from its registry",
    );
    assert_eq!(fixture.lock(), before);
}

#[test]
fn legacy_registry_source_spelling_is_preserved_when_identity_is_unique() {
    let fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let legacy = String::from_utf8(fixture.lock()).unwrap().replace(
        &format!("registry+{}", file_url(&fixture.remote)),
        "registry+leaf@1.0.0",
    );
    fs::write(fixture.consumer.join("CCGO.lock"), &legacy).unwrap();
    success(fixture.fetch(&["--locked", "--force"]));
    assert_eq!(fixture.lock(), legacy.as_bytes());
    let other = Fixture::new();
    fixture.manifest(
        &[
            ("company", file_url(&fixture.remote)),
            ("other", file_url(&other.remote)),
        ],
        None,
    );
    failure(fixture.fetch(&["--locked"]), "no unique registry identity");
    assert_eq!(fixture.lock(), legacy.as_bytes());
}

#[test]
fn missing_content_anchor_requires_regenerating_the_legacy_lock() {
    let fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = String::from_utf8(fixture.lock())
        .unwrap()
        .lines()
        .filter(|line| !line.starts_with("checksum ="))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(fixture.consumer.join("CCGO.lock"), &before).unwrap();
    failure(fixture.fetch(&["--locked"]), "no SHA-256 checksum");
    assert_eq!(fixture.lock(), before.as_bytes());
}

#[test]
fn fresh_resolution_requires_an_explicit_registry_for_ambiguous_matches() {
    let fixture = Fixture::new();
    let other = Fixture::new();
    let registries = [
        ("company", file_url(&fixture.remote)),
        ("other", file_url(&other.remote)),
    ];
    fixture.manifest(&registries, None);
    failure(fixture.fetch(&[]), "matches multiple registries");
    assert!(!fixture.consumer.join("CCGO.lock").exists());
    fixture.manifest(&registries, Some("company"));
    success(fixture.fetch(&[]));
}

#[test]
fn locked_git_uses_the_commit_after_the_tag_moves_or_disappears() {
    let mut fixture = Fixture::new();
    let source = fixture.tmp.path().join("source");
    fs::create_dir(&source).unwrap();
    git(&source, &["init", "-b", "main"]);
    fs::write(
        source.join("CCGO.toml"),
        "[package]\nname='leaf'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(source.join("payload.txt"), "original").unwrap();
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-m", "original"]);
    git(&source, &["tag", "v1.0.0"]);
    let original = git(&source, &["rev-parse", "HEAD"]);
    fixture.entry["repository"] = file_url(&source).into();
    for key in ["archive_url", "checksum"] {
        fixture.entry["versions"][0]
            .as_object_mut()
            .unwrap()
            .remove(key);
    }
    fixture.publish_index();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    assert!(String::from_utf8_lossy(&before).contains(&original));
    fs::write(source.join("payload.txt"), "replacement").unwrap();
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-m", "replacement"]);
    git(&source, &["tag", "-f", "v1.0.0"]);
    success(fixture.fetch(&["--locked"]));
    assert_eq!(fixture.payload(), "original");
    fs::write(
        fixture.consumer.join(".ccgo/deps/leaf/payload.txt"),
        "dirty",
    )
    .unwrap();
    failure(fixture.fetch(&["--locked"]), "Locked Git checkout is dirty");
    assert_eq!(fixture.lock(), before);
    git(&source, &["tag", "-d", "v1.0.0"]);
    success(fixture.fetch(&["--locked", "--force"]));
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), before);
}

#[test]
fn changed_version_requirement_is_rejected_without_rewriting_the_lock() {
    let fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    let manifest = fixture.consumer.join("CCGO.toml");
    let content = fs::read_to_string(&manifest).unwrap();
    fs::write(
        manifest,
        content.replace("version='1.0.0'", "version='1.0.1'"),
    )
    .unwrap();
    failure(fixture.fetch(&["--locked"]), "Dependencies have changed");
    assert_eq!(fixture.lock(), before);
}

#[test]
fn registry_archive_format_is_checked_before_replacing_an_installation() {
    let mut fixture = Fixture::new();
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fixture.entry["versions"][0]["archive_format"] = "tar.gz".into();
    fixture.publish_index();
    failure(
        fixture.fetch(&["--locked", "--force"]),
        "not supported; expected zip",
    );
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), before);
}

#[test]
fn workspace_locked_fetch_uses_member_config_and_propagates_failures() {
    let fixture = Fixture::new();
    fs::write(
        fixture.tmp.path().join("CCGO.toml"),
        "[workspace]\nmembers=['consumer']\n",
    )
    .unwrap();
    success(fixture.ccgo_at(fixture.tmp.path(), &["fetch", "--workspace"]));
    let before = fixture.lock();
    fixture.yank();
    success(fixture.ccgo_at(
        fixture.tmp.path(),
        &["fetch", "--workspace", "--locked", "--force"],
    ));
    assert_eq!(fixture.lock(), before);
    fixture.zip("replacement");
    failure(
        fixture.ccgo_at(
            fixture.tmp.path(),
            &["fetch", "--workspace", "--locked", "--force"],
        ),
        "checksum mismatch",
    );
    assert_eq!(fixture.lock(), before);
}
