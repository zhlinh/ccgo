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
