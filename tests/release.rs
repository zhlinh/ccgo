//! `ccgo release` commit/tag modes.
#![cfg(unix)]

use assert_cmd::Command;
use std::fs;
use std::path::Path;
use std::process::Command as ProcessCommand;

fn git(repo: &Path, args: &[&str]) -> String {
    let output = ProcessCommand::new("git")
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}

fn project() -> tempfile::TempDir {
    let repo = tempfile::tempdir().unwrap();
    fs::write(
        repo.path().join("CCGO.toml"),
        "[project]\nname=\"demo\"\nversion=\"1.0.0\"\n",
    )
    .unwrap();
    git(repo.path(), &["init", "-q"]);
    git(repo.path(), &["config", "user.name", "Release Test"]);
    git(
        repo.path(),
        &["config", "user.email", "release@test.invalid"],
    );
    git(repo.path(), &["add", "-A"]);
    git(repo.path(), &["commit", "-q", "-m", "initial"]);
    repo
}

fn ccgo(repo: &Path, args: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ccgo"));
    command
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_AUTHOR_NAME", "Release Test")
        .env("GIT_AUTHOR_EMAIL", "release@test.invalid")
        .env("GIT_COMMITTER_NAME", "Release Test")
        .env("GIT_COMMITTER_EMAIL", "release@test.invalid")
        .args(args);
    command
}

#[test]
fn no_tag_commits_the_bump_without_creating_a_tag() {
    let repo = project();
    ccgo(
        repo.path(),
        &["release", "patch", "--no-tag", "--no-changelog"],
    )
    .assert()
    .success();

    assert_eq!(git(repo.path(), &["tag", "--list"]), "");
    assert_eq!(
        git(repo.path(), &["log", "-1", "--pretty=%s"]),
        "chore: release v1.0.1"
    );
    assert!(fs::read_to_string(repo.path().join("CCGO.toml"))
        .unwrap()
        .contains("version=\"1.0.1\""));
    assert_eq!(git(repo.path(), &["status", "--porcelain"]), "");
}

#[test]
fn no_commit_still_skips_both_commit_and_tag() {
    let repo = project();
    ccgo(
        repo.path(),
        &["release", "patch", "--no-commit", "--no-changelog"],
    )
    .assert()
    .success();

    assert_eq!(git(repo.path(), &["rev-list", "--count", "HEAD"]), "1");
    assert_eq!(git(repo.path(), &["tag", "--list"]), "");
    assert!(fs::read_to_string(repo.path().join("CCGO.toml"))
        .unwrap()
        .contains("version=\"1.0.1\""));
    assert!(git(repo.path(), &["status", "--porcelain"]).contains("CCGO.toml"));
}
