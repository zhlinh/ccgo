#!/usr/bin/env python3
"""Measure actual received Git PACK bytes, distinct from loose-object disk growth."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from benchmark import ENV, FORMATS, commit, git, mutate, version, write_package


def traced(args, cwd, trace):
    p = subprocess.run(["git", *map(str, args)], cwd=cwd,
                       env=dict(ENV, GIT_TRACE_PACKFILE=str(trace)), capture_output=True)
    assert p.returncode == 0, p.stderr.decode()
    body = trace.read_bytes()
    assert body[:4] == b"PACK"
    return len(body)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--repeats", type=int, default=3)
    args = ap.parse_args()
    results = []
    with tempfile.TemporaryDirectory(prefix="ccgo-index-pack-") as tmp:
        root = Path(tmp)
        for count in (1000, 10000, 100000):
            for fmt in FORMATS:
                print(f"transport {fmt} {count}", flush=True)
                upstream = root / f"{fmt}-{count}"
                upstream.mkdir()
                write_package(upstream, fmt, "stdcomm", sorted((version(i) for i in range(count)),
                              key=lambda v: v["version"], reverse=True))
                git(upstream, "init", "-q", "-b", "main")
                commit(upstream, "snapshot")
                clones, full, shallow = [], [], []
                for i in range(args.repeats):
                    dest = root / f"{fmt}-{count}-full-{i}"
                    trace = root / f"{fmt}-{count}-full-{i}.pack"
                    full.append(traced(["clone", "-q", "--no-checkout", upstream.as_uri(), dest], root, trace))
                    clones.append(dest)
                    shallow_dest = root / f"{fmt}-{count}-shallow-{i}"
                    trace = root / f"{fmt}-{count}-shallow-{i}.pack"
                    shallow.append(traced(["clone", "-q", "--no-checkout", "--depth", "1",
                                           upstream.as_uri(), shallow_dest], root, trace))
                increments = []
                for i in range(args.repeats):
                    # Reuse one up-to-date object store: each fetch receives one new release.
                    mutate(upstream, fmt, "stdcomm", new=version(count + i))
                    commit(upstream, "one release")
                    trace = root / f"{fmt}-{count}-increment-{i}.pack"
                    increments.append(traced(["fetch", "-q", "origin"], clones[0], trace))
                results.append(dict(format=fmt, versions=count, full_clone_pack_bytes=full,
                                    shallow_clone_pack_bytes=shallow, one_release_fetch_pack_bytes=increments))
                args.output.write_text(json.dumps(dict(
                    note="Received Git PACK stream over file:// upload-pack. Excludes protocol framing, SSH/TLS, latency and checkout. Snapshot contains one baseline commit; history costs measured separately.",
                    scenarios=results), indent=2) + "\n")


if __name__ == "__main__":
    main()
