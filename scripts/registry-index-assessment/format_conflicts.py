#!/usr/bin/env python3
"""Isolate Git conflicts caused by format versus shared root metadata."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from benchmark import ENV, FORMATS, commit, encode, git, mutate, version, write_package


def experiment(work, fmt, shared_root, different_packages):
    root = work / f"{fmt}-{shared_root}-{different_packages}"
    root.mkdir()
    upstream = root / "remote.git"
    git(root, "init", "-q", "--bare", "--initial-branch=main", upstream)
    seed = root / "seed"
    seed.mkdir()
    git(seed, "init", "-q", "-b", "main")
    for name in ("stdcomm", "another"):
        write_package(seed, fmt, name, [version(0, name)])
    (seed / "index.json").write_bytes(encode(dict(package_count=2, updated_at="initial"), True))
    commit(seed, "initial")
    git(seed, "remote", "add", "origin", upstream)
    git(seed, "push", "-q", "origin", "HEAD")
    clones = []
    for who in ("a", "b"):
        clone = root / who
        git(root, "clone", "-q", upstream.as_uri(), clone)
        name = "another" if different_packages and who == "b" else "stdcomm"
        mutate(clone, fmt, name, new=version(1 if who == "a" else 2, name))
        if shared_root:
            (clone / "index.json").write_bytes(encode(dict(package_count=2, updated_at=who), True))
        commit(clone, f"publish {who}")
        clones.append(clone)
    git(clones[0], "push", "-q", "origin", "HEAD")
    rejected = subprocess.run(["git", "push", "origin", "HEAD"], cwd=clones[1], env=ENV, capture_output=True)
    assert rejected.returncode != 0, "Git must reject stale-head push for every layout"
    rebase = subprocess.run(["git", "pull", "--rebase"], cwd=clones[1], env=ENV, capture_output=True)
    conflicts = git(clones[1], "diff", "--name-only", "--diff-filter=U").decode().splitlines()
    if shared_root:
        assert "index.json" in conflicts, "shared timestamp conflicts regardless of package layout"
    elif different_packages:
        assert rebase.returncode == 0, "unrelated package changes rebase without shared root"
    else:
        assert rebase.returncode != 0, "array, JSONL tail and split catalog all conflict in this fixture"
    return dict(format=fmt, shared_root=shared_root, different_packages=different_packages,
                initial_push_rejected=True, rebase_code=rebase.returncode, conflicts=conflicts,
                stderr=rebase.stderr.decode())


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    results = []
    with tempfile.TemporaryDirectory(prefix="ccgo-index-layout-conflicts-") as tmp:
        for fmt in FORMATS:
            for shared in (False, True):
                for different in (False, True):
                    results.append(experiment(Path(tmp), fmt, shared, different))
    args.output.write_text(json.dumps(dict(note="Git/layout model, separate from real CLI fault tests", cases=results), indent=2) + "\n")


if __name__ == "__main__":
    main()
