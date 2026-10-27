//! Unix Git barriers exercise independent CLI processes sharing one cache.
use super::*;
use std::{
    os::unix::fs::PermissionsExt,
    process::Stdio,
    time::{Duration, Instant},
};

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

struct GitTrace {
    bin: PathBuf,
    log: PathBuf,
    marker: PathBuf,
    release: PathBuf,
}

impl GitTrace {
    fn new(fixture: &Fixture) -> Self {
        let bin = fixture.tmp.path().join("git-wrapper");
        fs::create_dir(&bin).unwrap();
        let real = success(
            Command::new("sh")
                .args(["-c", "command -v git"])
                .output()
                .unwrap(),
        );
        let real = String::from_utf8(real.stdout).unwrap();
        let script = format!(
            r#"#!/bin/sh
kind=
for arg in "$@"; do
    case "$arg" in clone|fetch|show) kind=$arg; break;; esac
done
if [ -n "$kind" ]; then printf '%s\n' "$kind" >> "$CCGO_TEST_GIT_TRACE"; fi
if [ -n "$CCGO_TEST_GIT_PAUSE" ] && [ "$kind" = "$CCGO_TEST_GIT_PAUSE" ]; then
    : > "$CCGO_TEST_GIT_MARKER"
    attempts=0
    while [ ! -f "$CCGO_TEST_GIT_RELEASE" ]; do
        attempts=$((attempts + 1))
        if [ "$attempts" -ge 1000 ]; then echo 'Git barrier timed out' >&2; exit 75; fi
        sleep 0.02
    done
fi
exec {} "$@"
"#,
            quote(real.trim())
        );
        fs::write(bin.join("git"), script).unwrap();
        fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
        Self {
            bin,
            log: fixture.tmp.path().join("git-trace"),
            marker: fixture.tmp.path().join("paused"),
            release: fixture.tmp.path().join("release"),
        }
    }

    fn command(&self, fixture: &Fixture, cwd: &Path, args: &[&str], pause: &str) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ccgo"));
        isolated(&mut cmd);
        let mut paths = vec![self.bin.clone()];
        paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
        cmd.current_dir(cwd)
            .env("PATH", std::env::join_paths(paths).unwrap())
            .env("CCGO_HOME", fixture.tmp.path().join("home"))
            .env("CCGO_TEST_GIT_TRACE", &self.log)
            .env("CCGO_TEST_GIT_PAUSE", pause)
            .env("CCGO_TEST_GIT_MARKER", &self.marker)
            .env("CCGO_TEST_GIT_RELEASE", &self.release)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd
    }

    fn wait_until_paused(&self) {
        let start = Instant::now();
        while !self.marker.exists() {
            assert!(
                start.elapsed() < Duration::from_secs(20),
                "Git barrier timed out"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn count(&self, kind: &str) -> usize {
        fs::read_to_string(&self.log)
            .unwrap()
            .lines()
            .filter(|line| *line == kind)
            .count()
    }

    fn release(&self) {
        fs::write(&self.release, "go").unwrap();
    }
}

impl Drop for GitTrace {
    fn drop(&mut self) {
        let _ = fs::write(&self.release, "go");
    }
}

fn add_second_dependency(fixture: &Fixture) {
    let mut entry = fixture.entry.clone();
    entry["name"] = "twig".into();
    let file = fixture.seed.join("tw/ig/twig.json");
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(file, serde_json::to_vec(&entry).unwrap()).unwrap();
    fixture.publish_index();
    let mut manifest = fs::OpenOptions::new()
        .append(true)
        .open(fixture.consumer.join("CCGO.toml"))
        .unwrap();
    writeln!(
        manifest,
        "[[dependencies]]\nname='twig'\nversion='1.0.0'\nregistry='company'"
    )
    .unwrap();
}

#[test]
fn one_sync_per_registry_for_multiple_dependencies_and_locked_fetch() {
    let fixture = Fixture::new();
    add_second_dependency(&fixture);
    let trace = GitTrace::new(&fixture);
    success(
        trace
            .command(&fixture, &fixture.consumer, &["fetch"], "")
            .output()
            .unwrap(),
    );
    assert_eq!(trace.count("clone"), 1);
    assert_eq!(trace.count("fetch"), 0);
    success(
        trace
            .command(&fixture, &fixture.consumer, &["fetch", "--locked"], "")
            .output()
            .unwrap(),
    );
    assert_eq!(trace.count("fetch"), 1);
    assert_eq!(fixture.payload(), "original");
}

#[test]
fn workspace_members_share_one_sync_but_new_invocation_refreshes() {
    let fixture = Fixture::new();
    let workspace = fixture.tmp.path().join("workspace-session");
    fs::create_dir(&workspace).unwrap();
    fs::write(
        workspace.join("CCGO.toml"),
        "[workspace]\nmembers=['a','b']\n",
    )
    .unwrap();
    let manifest = fs::read_to_string(fixture.consumer.join("CCGO.toml")).unwrap();
    for name in ["a", "b"] {
        fs::create_dir(workspace.join(name)).unwrap();
        fs::write(
            workspace.join(name).join("CCGO.toml"),
            manifest.replace("name='consumer'", &format!("name='{name}'")),
        )
        .unwrap();
    }
    let trace = GitTrace::new(&fixture);
    success(
        trace
            .command(&fixture, &workspace, &["fetch", "--workspace"], "")
            .output()
            .unwrap(),
    );
    assert_eq!(trace.count("clone"), 1);
    assert_eq!(trace.count("fetch"), 0);
    success(
        trace
            .command(
                &fixture,
                &workspace,
                &["fetch", "--workspace", "--locked"],
                "",
            )
            .output()
            .unwrap(),
    );
    assert_eq!(trace.count("fetch"), 1);
}

#[test]
fn concurrent_first_fetches_share_one_complete_clone() {
    let fixture = Fixture::new();
    let second = fixture.tmp.path().join("other-consumer");
    fs::create_dir(&second).unwrap();
    fs::copy(fixture.consumer.join("CCGO.toml"), second.join("CCGO.toml")).unwrap();
    let trace = GitTrace::new(&fixture);
    let first = trace
        .command(&fixture, &fixture.consumer, &["fetch"], "clone")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    let mut next = trace
        .command(&fixture, &second, &["fetch"], "")
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(next.try_wait().unwrap().is_none());
    trace.release();
    success(first.wait_with_output().unwrap());
    success(next.wait_with_output().unwrap());
    assert_eq!(trace.count("clone"), 1);
    assert_eq!(trace.count("fetch"), 1);
}

#[test]
fn live_snapshot_survives_another_fetch_reset_and_yank() {
    let mut fixture = Fixture::new();
    let second = fixture.tmp.path().join("other-consumer");
    fs::create_dir(&second).unwrap();
    fs::copy(fixture.consumer.join("CCGO.toml"), second.join("CCGO.toml")).unwrap();
    let trace = GitTrace::new(&fixture);
    let first = trace
        .command(&fixture, &fixture.consumer, &["fetch"], "show")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    fixture.entry["versions"][0]["yanked"] = true.into();
    fixture.publish_index();
    failure(
        trace
            .command(&fixture, &second, &["fetch"], "")
            .output()
            .unwrap(),
        "no non-yanked match",
    );
    trace.release();
    success(first.wait_with_output().unwrap());
    assert_eq!(fixture.payload(), "original");
    failure(fixture.fetch(&["--force"]), "no non-yanked match");
}

#[test]
fn clean_cache_waits_for_live_snapshot_then_rebuilds() {
    let fixture = Fixture::new();
    let second = fixture.tmp.path().join("clean-consumer");
    fs::create_dir(&second).unwrap();
    fs::copy(fixture.consumer.join("CCGO.toml"), second.join("CCGO.toml")).unwrap();
    let trace = GitTrace::new(&fixture);
    let first = trace
        .command(&fixture, &fixture.consumer, &["fetch"], "show")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    let mut cleaner = trace
        .command(&fixture, &second, &["fetch", "--clean-cache"], "")
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(cleaner.try_wait().unwrap().is_none());
    trace.release();
    success(first.wait_with_output().unwrap());
    success(cleaner.wait_with_output().unwrap());
    assert_eq!(trace.count("clone"), 2);
}

fn git_source(fixture: &Fixture) -> PathBuf {
    let source = fixture.tmp.path().join("git-source");
    fs::create_dir(&source).unwrap();
    git(&source, &["init", "-b", "main"]);
    fs::write(
        source.join("CCGO.toml"),
        "[package]\nname='leaf'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(source.join("payload.txt"), "original").unwrap();
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-m", "source"]);
    source
}

fn git_consumer(directory: &Path, source: &Path, branch: &str) {
    fs::create_dir_all(directory).unwrap();
    fs::write(directory.join("CCGO.toml"), format!("[package]\nname='consumer'\nversion='1.0.0'\n[[dependencies]]\nname='leaf'\nversion='1.0.0'\ngit='{}'\nbranch='{branch}'\n", file_url(source))).unwrap();
}

#[test]
fn concurrent_git_checkouts_publish_one_complete_generation() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git_consumer(&fixture.consumer, &source, "main");
    let second = fixture.tmp.path().join("git-consumer-two");
    git_consumer(&second, &source, "main");
    let trace = GitTrace::new(&fixture);
    let first = trace
        .command(&fixture, &fixture.consumer, &["fetch"], "clone")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    let mut next = trace
        .command(&fixture, &second, &["fetch"], "")
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(next.try_wait().unwrap().is_none());
    trace.release();
    success(first.wait_with_output().unwrap());
    success(next.wait_with_output().unwrap());
    assert_eq!(trace.count("clone"), 1);
    assert_eq!(fixture.payload(), "original");
    assert_eq!(
        fs::read_to_string(second.join(".ccgo/deps/leaf/payload.txt")).unwrap(),
        "original"
    );
}

#[test]
fn force_git_refresh_preserves_old_generation_and_failed_refresh_preserves_installation() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git_consumer(&fixture.consumer, &source, "main");
    success(fixture.fetch(&[]));
    let original_tree = fs::read_link(fixture.consumer.join(".ccgo/deps/leaf")).unwrap();
    let lock = fixture.lock();
    let moved = fixture.tmp.path().join("git-source-unavailable");
    fs::rename(&source, &moved).unwrap();
    failure(fixture.fetch(&["--force"]), "Git clone failed");
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), lock);
    assert_eq!(
        fs::read_link(fixture.consumer.join(".ccgo/deps/leaf")).unwrap(),
        original_tree
    );
    fs::rename(moved, &source).unwrap();
    success(fixture.fetch(&["--force"]));
    let replacement = fs::read_link(fixture.consumer.join(".ccgo/deps/leaf")).unwrap();
    assert_ne!(replacement, original_tree);
    assert_eq!(
        fs::read_to_string(original_tree.join("payload.txt")).unwrap(),
        "original"
    );
}

#[test]
fn concurrent_branches_have_separate_cache_identities() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git(&source, &["checkout", "-b", "other"]);
    fs::write(source.join("payload.txt"), "other").unwrap();
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-m", "other branch"]);
    git_consumer(&fixture.consumer, &source, "main");
    let second = fixture.tmp.path().join("other-branch-consumer");
    git_consumer(&second, &source, "other");
    let trace = GitTrace::new(&fixture);
    let first = trace
        .command(&fixture, &fixture.consumer, &["fetch"], "clone")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    success(
        trace
            .command(&fixture, &second, &["fetch"], "")
            .output()
            .unwrap(),
    );
    trace.release();
    success(first.wait_with_output().unwrap());
    assert_eq!(fixture.payload(), "original");
    assert_eq!(
        fs::read_to_string(second.join(".ccgo/deps/leaf/payload.txt")).unwrap(),
        "other"
    );
    assert_eq!(trace.count("clone"), 2);
}

#[test]
fn dependency_graph_reads_transitive_dependencies_from_generation_pointer() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    fs::create_dir(source.join("child")).unwrap();
    fs::write(
        source.join("child/CCGO.toml"),
        "[package]\nname='child'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(source.join("CCGO.toml"), "[package]\nname='leaf'\nversion='1.0.0'\n[[dependencies]]\nname='child'\nversion='1.0.0'\npath='child'\n").unwrap();
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-m", "transitive dependency"]);
    git_consumer(&fixture.consumer, &source, "main");
    success(fixture.fetch(&[]));
    let second = fixture.tmp.path().join("transitive-consumer");
    git_consumer(&second, &source, "main");
    success(fixture.ccgo_at(&second, &["fetch"]));
    assert!(second.join(".ccgo/deps/child/CCGO.toml").is_file());
    assert!(fs::read_to_string(second.join("CCGO.lock"))
        .unwrap()
        .contains("name = \"child\""));
}

#[test]
fn legacy_checkout_remains_readable_and_force_never_deletes_it() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git_consumer(&fixture.consumer, &source, "main");
    let manifest = fixture.consumer.join("CCGO.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace("branch='main'\n", ""),
    )
    .unwrap();
    let key = format!("{:x}", md5::compute(format!("leaf:{}", file_url(&source))));
    let old = fixture.tmp.path().join("home/git/checkouts").join(key);
    fs::create_dir_all(old.parent().unwrap()).unwrap();
    git(
        fixture.tmp.path(),
        &["clone", &file_url(&source), old.to_str().unwrap()],
    );
    success(fixture.fetch(&[]));
    assert_eq!(
        fs::read_link(fixture.consumer.join(".ccgo/deps/leaf"))
            .unwrap()
            .canonicalize()
            .unwrap(),
        old.canonicalize().unwrap()
    );
    success(fixture.fetch(&["--force"]));
    assert_eq!(
        fs::read_to_string(old.join("payload.txt")).unwrap(),
        "original"
    );
    assert_ne!(
        fs::read_link(fixture.consumer.join(".ccgo/deps/leaf")).unwrap(),
        old
    );
}

#[test]
fn relative_git_source_and_cache_home_remain_supported() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git_consumer(&fixture.consumer, &source, "main");
    let manifest = fixture.consumer.join("CCGO.toml");
    fs::write(
        &manifest,
        fs::read_to_string(&manifest)
            .unwrap()
            .replace(&file_url(&source), "../git-source"),
    )
    .unwrap();
    let trace = GitTrace::new(&fixture);
    success(
        trace
            .command(&fixture, &fixture.consumer, &["fetch"], "")
            .env("CCGO_HOME", "relative-cache")
            .output()
            .unwrap(),
    );
    assert_eq!(fixture.payload(), "original");
}

#[test]
fn locked_git_tag_can_disappear_without_breaking_force_refetch() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git(&source, &["tag", "v1.0.0"]);
    git_consumer(&fixture.consumer, &source, "v1.0.0");
    success(fixture.fetch(&[]));
    let lock = fixture.lock();
    git(&source, &["tag", "-d", "v1.0.0"]);
    success(fixture.fetch(&["--locked", "--force"]));
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), lock);
}

#[test]
fn another_projects_force_refresh_does_not_change_locked_dependency_graph() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git_consumer(&fixture.consumer, &source, "main");
    success(fixture.fetch(&[]));
    let before = fixture.lock();
    fs::create_dir(source.join("new-child")).unwrap();
    fs::write(
        source.join("new-child/CCGO.toml"),
        "[package]\nname='new-child'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(source.join("CCGO.toml"), "[package]\nname='leaf'\nversion='1.0.0'\n[[dependencies]]\nname='new-child'\nversion='1.0.0'\npath='new-child'\n").unwrap();
    fs::write(source.join("payload.txt"), "replacement").unwrap();
    git(&source, &["add", "-A"]);
    git(&source, &["commit", "-m", "new dependency"]);
    let second = fixture.tmp.path().join("refresh-consumer");
    git_consumer(&second, &source, "main");
    success(fixture.ccgo_at(&second, &["fetch", "--force"]));
    assert_eq!(
        fs::read_to_string(second.join(".ccgo/deps/leaf/payload.txt")).unwrap(),
        "replacement"
    );
    success(fixture.fetch(&["--locked"]));
    assert_eq!(fixture.payload(), "original");
    assert_eq!(fixture.lock(), before);
    assert!(!fixture.consumer.join(".ccgo/deps/new-child").exists());
}

fn project_fetch_waits_for_writer(alias: bool, workspace: bool) {
    let fixture = Fixture::new();
    let trace = GitTrace::new(&fixture);
    let (cwd, args) = if workspace {
        fs::write(
            fixture.tmp.path().join("CCGO.toml"),
            "[workspace]\nmembers=['consumer']\n",
        )
        .unwrap();
        (fixture.tmp.path(), vec!["fetch", "--workspace"])
    } else {
        (fixture.consumer.as_path(), vec!["fetch"])
    };
    let first = trace
        .command(&fixture, cwd, &args, "clone")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    let second_cwd = if alias {
        let path = fixture.tmp.path().join("consumer-alias");
        std::os::unix::fs::symlink(&fixture.consumer, &path).unwrap();
        path
    } else {
        fixture.consumer.clone()
    };
    let mut second = trace
        .command(&fixture, &second_cwd, &["fetch", "--locked"], "")
        .env("CCGO_HOME", fixture.tmp.path().join("independent-home"))
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(150));
    assert!(
        second.try_wait().unwrap().is_none(),
        "fetch must wait before reading the missing lockfile"
    );
    trace.release();
    success(first.wait_with_output().unwrap());
    let lock = fixture.lock();
    success(second.wait_with_output().unwrap());
    assert_eq!(fixture.lock(), lock);
    assert_eq!(fixture.payload(), "original");
}

#[test]
fn same_project_fetch_serializes_across_cache_homes() {
    project_fetch_waits_for_writer(false, false);
}

#[test]
fn project_alias_fetch_uses_the_same_installation_lock() {
    project_fetch_waits_for_writer(true, false);
}

#[test]
fn workspace_and_member_fetch_share_the_installation_lock() {
    project_fetch_waits_for_writer(false, true);
}

#[test]
fn failed_fetch_releases_project_lock_for_retry() {
    let fixture = Fixture::new();
    let source = git_source(&fixture);
    git_consumer(&fixture.consumer, &source, "missing-branch");
    assert!(!fixture.fetch(&[]).status.success());
    assert!(!fixture.consumer.join("CCGO.lock").exists());
    git_consumer(&fixture.consumer, &source, "main");
    success(fixture.fetch(&[]));
    assert_eq!(fixture.payload(), "original");
}

#[test]
fn lock_save_failure_forces_validated_retry_instead_of_reusing_unrecorded_tree() {
    let fixture = Fixture::new();
    let trace = GitTrace::new(&fixture);
    let first = trace
        .command(&fixture, &fixture.consumer, &["fetch"], "clone")
        .spawn()
        .unwrap();
    trace.wait_until_paused();
    // Lock loading has finished. A directory makes atomic lockfile publication fail.
    fs::create_dir(fixture.consumer.join("CCGO.lock")).unwrap();
    trace.release();
    failure(
        first.wait_with_output().unwrap(),
        "Failed to write lockfile",
    );
    assert_eq!(fixture.payload(), "original");
    let marker = fixture
        .consumer
        .join(".ccgo/pending-installs")
        .join(format!("{:x}", md5::compute("leaf")));
    assert!(marker.is_file());
    fs::remove_dir(fixture.consumer.join("CCGO.lock")).unwrap();
    fs::write(
        fixture.consumer.join(".ccgo/deps/leaf/payload.txt"),
        "unrecorded",
    )
    .unwrap();
    success(fixture.fetch(&[]));
    assert_eq!(fixture.payload(), "original");
    let lock = String::from_utf8(fixture.lock()).unwrap();
    assert!(lock.contains("sha256:"));
    assert!(lock.contains(&format!("registry+{}", file_url(&fixture.remote))));
    assert!(!marker.exists());
    // Once saved, a clean retry can reuse the installation without downloading.
    fs::remove_file(&fixture.archive).unwrap();
    success(fixture.fetch(&[]));
}

#[test]
fn unrecorded_existing_tree_is_revalidated_after_lock_save_failure() {
    let fixture = Fixture::new();
    // Simulate a tree left behind by an older fetch whose lock was never saved.
    let leaf = fixture.consumer.join(".ccgo/deps/leaf");
    fs::create_dir_all(&leaf).unwrap();
    fs::write(
        leaf.join("CCGO.toml"),
        "[package]\nname='leaf'\nversion='1.0.0'\n",
    )
    .unwrap();
    fs::write(leaf.join("payload.txt"), "stale").unwrap();
    // A dangling lockfile symlink is invisible to `exists()` and fails on save,
    // without blocking the earlier lockfile load.
    std::os::unix::fs::symlink("missing-lock-target", fixture.consumer.join("CCGO.lock")).unwrap();

    failure(fixture.fetch(&[]), "Failed to write lockfile");
    let marker = fixture
        .consumer
        .join(".ccgo/pending-installs")
        .join(format!("{:x}", md5::compute("leaf")));
    assert!(marker.is_file());

    fs::remove_file(fixture.consumer.join("CCGO.lock")).unwrap();
    success(fixture.fetch(&[]));
    assert_eq!(fixture.payload(), "original");
    assert!(String::from_utf8(fixture.lock()).unwrap().contains("sha256:"));
    assert!(!marker.exists());
}
