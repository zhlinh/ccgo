#!/usr/bin/env python3
"""Characterize the real ccgo binary, including known failures; never use real remotes."""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import zipfile

from benchmark import ENV, commit, encode, git, meta, run, version


def check(condition, message):
    if not condition:
        raise AssertionError(message)


class Fixture:
    def __init__(self, root, binary):
        self.root, self.binary = root, binary
        root.mkdir(parents=True)
        self.project = root / "project"
        self.project.mkdir()
        self.remote = root / "remote.git"
        git(root, "init", "--bare", "-q", "--initial-branch=main", self.remote)
        seed = root / "seed"
        seed.mkdir()
        git(seed, "init", "-q", "-b", "main")
        (seed / "index.json").write_bytes(encode(dict(version=1, name="company", description="fixture",
                                                    package_count=0, updated_at="2020-01-01T00:00:00Z")))
        commit(seed, "seed")
        git(seed, "remote", "add", "origin", self.remote)
        git(seed, "push", "-q", "origin", "HEAD")
        (self.project / "CCGO.toml").write_text(
            f"[package]\nname='demo'\nversion='1.2.3'\n[registries]\ncompany='{self.remote.as_uri()}'\n")
        git(self.project, "init", "-q", "-b", "main")
        commit(self.project, "source")
        git(self.project, "remote", "add", "origin", "https://example.invalid/demo.git")
        for v in ("1.2.3", "1.2.4", "1.2.5"):
            git(self.project, "tag", f"v{v}")
        self.archive = root / "sdk.zip"
        with zipfile.ZipFile(self.archive, "w") as z:
            z.writestr("CCGO.toml", "[package]\nname='demo'\nversion='1.2.3'\n")
            z.writestr("payload.txt", "original")
        self.bin = root / "bin"
        self.bin.mkdir()
        gh = self.bin / "gh"
        # Stand-in for the external release service. Deliberately requires immutable assets.
        gh.write_text(f"#!{sys.executable}\n" + '''import os, pathlib, sys, time
args = sys.argv[1:]
if args[:2] == ['release', 'view']: sys.exit(0)
if args[:2] != ['release', 'upload']: sys.exit(2)
root = pathlib.Path(os.environ['FAULT_ROOT'])
who = os.environ.get('FAULT_WHO', 'one')
(root / ('ready-' + who)).write_text('ready')
if os.environ.get('FAULT_BARRIER') == '1':
    deadline = time.monotonic() + 30
    while not (root / 'gate').exists():
        if time.monotonic() > deadline: sys.exit(10)
        time.sleep(0.01)
asset = root / ('asset-' + args[2])
try:
    with asset.open('xb') as out:
        out.write(pathlib.Path(next(a for a in args if a.endswith('.zip'))).read_bytes())
except FileExistsError: sys.exit(4)
''')
        gh.chmod(0o755)

    def env(self, home="one", **extra):
        return dict(ENV, PATH=str(self.bin) + os.pathsep + ENV["PATH"],
                    CCGO_HOME=str(self.root / home), FAULT_ROOT=str(self.root), **extra)

    def clone(self, home="one"):
        target = self.root / home / "registry/publish/company"
        target.parent.mkdir(parents=True, exist_ok=True)
        git(self.root, "clone", "-q", self.remote.as_uri(), target)
        return target

    def publish(self, v="1.2.3", home="one", upload=False, **extra):
        args = [self.binary, "publish", "package", "--registry", "private", "--remote", "company",
                "--version", v, "--push"]
        if upload:
            args += ["--artifact-provider", "github", "--github-repo", "example/sdk", "--path", self.archive]
        p = subprocess.run([str(a) for a in args], cwd=self.project, env=self.env(home, **extra),
                           capture_output=True, text=True, timeout=60)
        return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr)

    def entries(self):
        p = subprocess.run(["git", "show", "HEAD:de/mo/demo.json"], cwd=self.remote, env=ENV, capture_output=True)
        return json.loads(p.stdout)["versions"] if p.returncode == 0 else []


def concurrency(root, binary, same):
    f = Fixture(root, binary)
    f.clone("a")
    f.clone("b")
    with ThreadPoolExecutor(2) as pool:
        a = pool.submit(f.publish, "1.2.3", "a", True, FAULT_WHO="a", FAULT_BARRIER="1")
        b = pool.submit(f.publish, "1.2.3" if same else "1.2.4", "b", True,
                        FAULT_WHO="b", FAULT_BARRIER="1")
        deadline = time.monotonic() + 30
        while not all((root / f"ready-{n}").exists() for n in ("a", "b")):
            check(time.monotonic() < deadline, "both publishers must reach upload barrier")
            time.sleep(0.01)
        (root / "gate").touch()
        outcomes = [a.result(), b.result()]
    check(sorted(p["code"] == 0 for p in outcomes) == [False, True], "exactly one publication succeeds")
    entries = f.entries()
    check(len(entries) == 1, "remote must contain exactly the acknowledged winner")
    result = dict(case="same_version_race" if same else "different_version_race", outcomes=outcomes,
                  remote_versions=[e["version"] for e in entries])
    if not same:
        loser = 0 if outcomes[0]["code"] else 1
        retry = f.publish("1.2.3" if loser == 0 else "1.2.4", "a" if loser == 0 else "b", True)
        result["retry"] = retry
        check(retry["code"] != 0, "current implementation requires manual conflict recovery")
        result["orphan_assets"] = len(list(root.glob("asset-*"))) - len(entries)
        check(result["orphan_assets"] == 1, "losing version uploaded but was not indexed")
    return result


def commit_failure(root, binary):
    f = Fixture(root, binary)
    clone = f.clone()
    hook = clone / ".git/hooks/pre-commit"
    hook.write_text("#!/bin/sh\nexit 1\n")
    hook.chmod(0o755)
    first = f.publish(upload=True)
    check(first["code"] != 0 and "Failed to commit" in first["stderr"], "commit must fail")
    check(not f.entries() and (root / "asset-v1.2.3").is_file(), "upload survives failed commit")
    hook.unlink()
    retry = f.publish(upload=True)
    check(retry["code"] != 0, "dirty clone prevents ordinary retry")
    commit(clone, "manual recovery of staged index")
    git(clone, "push", "-q", "origin", "HEAD")
    check(len(f.entries()) == 1, "preserved local state supports manual recovery")
    return dict(case="upload_then_commit_failure", first=first, retry=retry,
                manual_recovery="commit staged state and push succeeds")


def push_failure(root, binary):
    f = Fixture(root, binary)
    clone = f.clone()
    hook = f.remote / "hooks/pre-receive"
    hook.write_text("#!/bin/sh\nexit 1\n")
    hook.chmod(0o755)
    first = f.publish(upload=True)
    check(first["code"] != 0 and "Failed to push" in first["stderr"], "remote rejects push")
    check(not f.entries(), "rejected push must not publish metadata")
    hook.unlink()
    retry = f.publish(upload=True)
    check(retry["code"] != 0, "local committed duplicate prevents ordinary retry")
    git(clone, "push", "-q", "origin", "HEAD")
    check(len(f.entries()) == 1, "manual push recovers retained commit")
    duplicate = f.publish(upload=True)
    check(duplicate["code"] != 0 and "already in the index" in duplicate["stderr"], "duplicate rejected")
    return dict(case="push_rejected_and_retry", first=first, retry=retry,
                duplicate=duplicate, manual_recovery="push retained commit succeeds")


def fetch_case(root, binary, operation):
    f = Fixture(root, binary)
    seed = root / "seed"
    archive = root / "leaf.zip"
    def make_zip(payload):
        with zipfile.ZipFile(archive, "w") as z:
            z.writestr("CCGO.toml", "[package]\nname='leaf'\nversion='1.0.0'\n")
            z.writestr("payload.txt", payload)
        return "sha256:" + hashlib.sha256(archive.read_bytes()).hexdigest()
    checksum = make_zip("original")
    entry = dict(version="1.0.0", tag="v1.0.0", checksum=checksum, archive_url=archive.as_uri(),
                 archive_format="zip", released_at="2020-01-01T00:00:00Z", yanked=False)
    target = seed / "le/af/leaf.json"
    target.parent.mkdir(parents=True)
    target.write_bytes(encode(dict(meta("leaf"), versions=[entry]), True))
    commit(seed, "leaf")
    git(seed, "push", "-q", "origin", "HEAD")
    consumer = root / "consumer"
    consumer.mkdir()
    (consumer / "CCGO.toml").write_text(
        f"[package]\nname='consumer'\nversion='1.0.0'\n[registries]\ncompany='{f.remote.as_uri()}'\n"
        "[[dependencies]]\nname='leaf'\nversion='1.0.0'\nregistry='company'\n")
    def fetch(*args):
        p = subprocess.run([str(binary), "fetch", *args], cwd=consumer, env=f.env(),
                           capture_output=True, text=True, timeout=60)
        return dict(code=p.returncode, stdout=p.stdout, stderr=p.stderr)
    first = fetch()
    check(first["code"] == 0, f"initial fetch: {first}")
    before_lock = (consumer / "CCGO.lock").read_text()
    check(checksum in before_lock, "initial lock contains original checksum")
    preflight = fetch("--locked", "--force")
    check(preflight["code"] != 0 and "Dependencies have changed" in preflight["stderr"],
          "unmodified registry lock is currently rejected by source comparison")
    # Explicit diagnostic workaround to reach the code behind the earlier defect.
    # This is not the lockfile generated by production fetch.
    normalized_lock = before_lock.replace("registry+" + f.remote.as_uri(), "registry+leaf@1.0.0")
    check(normalized_lock != before_lock, "normalization must only replace the source identity")
    (consumer / "CCGO.lock").write_text(normalized_lock)
    if operation == "yank":
        p = subprocess.run([str(binary), "yank", "--package", "leaf", "--vers", "1.0.0",
                            "--reason", "experiment", "--index-repo", f.remote.as_uri(),
                            "--index-name", "yank-case", "--push"],
                           cwd=consumer, env=f.env(), capture_output=True, text=True, timeout=60)
        check(p.returncode == 0, p.stderr)
    elif operation == "replace_both":
        entry["checksum"] = make_zip("replacement")
        target.write_bytes(encode(dict(meta("leaf"), versions=[entry]), True))
        commit(seed, "rewrite checksum and archive")
        git(seed, "push", "-q", "origin", "HEAD")
    else:
        make_zip("replacement")
    second = fetch("--locked", "--force")
    if operation == "replace_both":
        check(second["code"] == 0, f"current locked path trusts updated index checksum: {second}")
        check((consumer / ".ccgo/deps/leaf/payload.txt").read_text() == "replacement", "new bytes accepted")
        check(entry["checksum"] in (consumer / "CCGO.lock").read_text(), "locked operation rewrites checksum")
    else:
        check(second["code"] != 0, f"{operation} must cause a failure in current implementation")
        if operation == "replace_bytes":
            check("checksum mismatch" in second["stderr"], "unchanged index detects changed bytes")
    return dict(case="locked_" + operation, initial=first, unmodified_locked_preflight=preflight,
                diagnostic_workaround="manually normalize lock source to registry+leaf@1.0.0 to reach later code",
                refetch=second,
                lock_changed=before_lock != (consumer / "CCGO.lock").read_text())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--binary", type=Path, default=Path("target/debug/ccgo"))
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    binary = args.binary.resolve()
    results = []
    with tempfile.TemporaryDirectory(prefix="ccgo-index-faults-") as tmp:
        work = Path(tmp)
        cases = [("different", lambda p: concurrency(p, binary, False)),
                 ("same", lambda p: concurrency(p, binary, True)),
                 ("commit", lambda p: commit_failure(p, binary)),
                 ("push", lambda p: push_failure(p, binary))]
        cases += [(operation, lambda p, op=operation: fetch_case(p, binary, op))
                  for operation in ("yank", "replace_both", "replace_bytes")]
        for name, case in cases:
            print(name, flush=True)
            results.append(case(work / name))
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(dict(
                binary=str(binary), binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                note="Characterization assertions pin current observed behavior, including defects; not all are desired invariants.",
                cases=results), indent=2) + "\n")


if __name__ == "__main__":
    main()
