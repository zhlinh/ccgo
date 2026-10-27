#!/usr/bin/env python3
"""Reproducible format/Git experiments. Standard library only; no production writes."""
import argparse
import datetime as dt
import gzip
import hashlib
import json
import os
from pathlib import Path
import platform
import statistics
import subprocess
import sys
import tarfile
import tempfile
import time


FORMATS = ("array", "jsonl", "split")
ENV = dict(os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL="/dev/null",
           GIT_AUTHOR_NAME="Index experiment", GIT_AUTHOR_EMAIL="index@example.invalid",
           GIT_COMMITTER_NAME="Index experiment", GIT_COMMITTER_EMAIL="index@example.invalid",
           LC_ALL="C")


def run(args, cwd=None):
    p = subprocess.run([str(a) for a in args], cwd=cwd, env=ENV, capture_output=True)
    if p.returncode:
        raise RuntimeError(f"{args}: {p.stderr.decode(errors='replace')}")
    return p.stdout


def git(path, *args):
    return run(["git", "-c", "gc.auto=0", *args], path)


def stats(values):
    return {"samples_ms": values, "median_ms": statistics.median(values),
            "min_ms": min(values), "max_ms": max(values)}


def measure(fn, repeats):
    samples = []
    for _ in range(repeats):
        start = time.perf_counter_ns()
        fn()
        samples.append((time.perf_counter_ns() - start) / 1e6)
    return stats(samples)


def meta(name):
    return dict(name=name, description="Standard common library for cross-platform C++",
                repository=f"https://example.invalid/org/{name}.git", license="MIT",
                keywords=[], platforms=["android", "ios", "macos", "linux", "windows"])


def version(i, name="stdcomm", extra=0):
    v = f"0.0.{i}"
    result = dict(version=v, tag=f"v{v}",
                  checksum="sha256:" + hashlib.sha256(f"{name}/{v}".encode()).hexdigest(),
                  archive_url=f"https://mirrors.example.invalid/repository/maven/com/example/{name}-ccgo/{v}/{name}-ccgo-{v}.zip",
                  archive_format="zip", released_at=(dt.datetime(2020, 1, 1, tzinfo=dt.timezone.utc)
                  + dt.timedelta(minutes=i)).isoformat(), yanked=False)
    if extra:
        # Deterministic, less-compressible extension, not present in today's schema.
        result["future_metadata"] = "".join(hashlib.sha256(f"{name}/{i}/{j}".encode()).hexdigest()
                                              for j in range((extra + 63) // 64))[:extra]
    return result


def encode(obj, pretty=False):
    return json.dumps(obj, ensure_ascii=False, indent=2 if pretty else None,
                      separators=None if pretty else (",", ":")).encode()


def package_dir(root, name):
    return root / name[:2] / name[2:4]


def write_package(root, fmt, name, entries):
    base = package_dir(root, name)
    base.mkdir(parents=True, exist_ok=True)
    if fmt == "array":
        (base / f"{name}.json").write_bytes(encode(dict(meta(name), versions=entries), True))
    elif fmt == "jsonl":
        (base / f"{name}.meta.json").write_bytes(encode(meta(name)))
        (base / f"{name}.jsonl").write_bytes(b"".join(encode(v) + b"\n" for v in entries))
    else:
        base = base / name
        (base / "versions").mkdir(parents=True, exist_ok=True)
        (base / "package.json").write_bytes(encode(meta(name)))
        (base / "versions.json").write_bytes(encode([v["version"] for v in entries]))
        for v in entries:
            (base / "versions" / f'{v["version"]}.json').write_bytes(encode(v))


def lookup(root, fmt, name, wanted):
    base = package_dir(root, name)
    if fmt == "array":
        entries = json.loads((base / f"{name}.json").read_bytes())["versions"]
        return next(v for v in entries if v["version"] == wanted and not v["yanked"])
    if fmt == "jsonl":
        # Streaming, early-exit is part of this candidate design.
        with (base / f"{name}.jsonl").open("rb") as f:
            for line in f:
                v = json.loads(line)
                if v["version"] == wanted and not v["yanked"]:
                    return v
        raise KeyError(wanted)
    v = json.loads((base / name / "versions" / f"{wanted}.json").read_bytes())
    assert not v["yanked"]
    return v


def select_latest(root, fmt, name):
    base = package_dir(root, name)
    if fmt == "split":
        versions = json.loads((base / name / "versions.json").read_bytes())
        # Catalog lists all versions, including yanked; verify descending candidates.
        for v in sorted(versions, key=lambda s: tuple(map(int, s.split('.'))), reverse=True):
            entry = json.loads((base / name / "versions" / f"{v}.json").read_bytes())
            if not entry["yanked"]:
                return v
    else:
        if fmt == "array":
            entries = json.loads((base / f"{name}.json").read_bytes())["versions"]
        else:
            with (base / f"{name}.jsonl").open("rb") as f:
                entries = [json.loads(line) for line in f]
        return max((v["version"] for v in entries if not v["yanked"]),
                   key=lambda s: tuple(map(int, s.split('.'))))


def mutate(root, fmt, name, new=None, yank=None):
    base = package_dir(root, name)
    if fmt == "array":
        path = base / f"{name}.json"
        obj = json.loads(path.read_bytes())
        if new:
            assert not any(v["version"] == new["version"] for v in obj["versions"])
            obj["versions"].append(new)
            obj["versions"].sort(key=lambda v: v["version"], reverse=True)
        if yank:
            next(v for v in obj["versions"] if v["version"] == yank)["yanked"] = True
        path.write_bytes(encode(obj, True))
    elif fmt == "jsonl":
        path = base / f"{name}.jsonl"
        if new:
            # Duplicate prevention still scans existing entries; append is not a transaction.
            with path.open("rb") as f:
                assert not any(json.loads(line)["version"] == new["version"] for line in f)
            with path.open("ab") as f:
                f.write(encode(new) + b"\n")
        if yank:
            entries = [json.loads(line) for line in path.read_bytes().splitlines()]
            next(v for v in entries if v["version"] == yank)["yanked"] = True
            path.write_bytes(b"".join(encode(v) + b"\n" for v in entries))
    else:
        base = base / name
        if new:
            target = base / "versions" / f'{new["version"]}.json'
            with target.open("xb") as f:
                f.write(encode(new))
            catalog = base / "versions.json"
            versions = json.loads(catalog.read_bytes())
            versions.append(new["version"])
            catalog.write_bytes(encode(versions))
        if yank:
            path = base / "versions" / f"{yank}.json"
            obj = json.loads(path.read_bytes())
            obj["yanked"] = True
            path.write_bytes(encode(obj))


def commit(root, message):
    git(root, "add", "-A")
    git(root, "commit", "-q", "-m", message)


def disk(root):
    files = [p for p in root.rglob("*") if p.is_file() and ".git" not in p.relative_to(root).parts]
    return {"files": len(files), "logical_bytes": sum(p.stat().st_size for p in files),
            "allocated_bytes": sum(p.stat().st_blocks * 512 for p in files)}


def git_size(root):
    return {"git_bytes": sum(p.stat().st_size for p in (root / ".git").rglob("*") if p.is_file()),
            "count_objects": git(root, "count-objects", "-v").decode()}


def snapshot(work, fmt, count, packages, repeats, extra):
    root = work / f"{fmt}-{packages}x{count}-extra{extra}"
    root.mkdir()
    name = "stdcomm"
    for p in range(packages):
        n = name if not p else f"pkg{p:05d}"
        # Actual publisher sorts version strings lexically, not semantically.
        entries = sorted((version(i, n, extra) for i in range(count)),
                         key=lambda v: v["version"], reverse=True)
        write_package(root, fmt, n, entries)
    result = dict(format=fmt, versions_per_package=count, packages=packages, extra_bytes=extra,
                  disk=disk(root))
    git(root, "init", "-q", "-b", "main")
    commit(root, "baseline snapshot")
    git(root, "gc", "--quiet")
    result["git_snapshot"] = git_size(root)
    tar_path = work / f"{root.name}.tar.gz"
    with tarfile.open(tar_path, "w:gz") as archive:
        for path in sorted(root.rglob("*")):
            if path.is_file() and ".git" not in path.relative_to(root).parts:
                archive.add(path, arcname=str(path.relative_to(root)), recursive=False)
    result["tar_gzip_bytes"] = tar_path.stat().st_size
    wanted = f"0.0.{count // 2}"
    result["exact_warm"] = measure(lambda: lookup(root, fmt, name, wanted), repeats)
    # New Python process; no claim to flush OS page cache.
    result["exact_process_cold"] = measure(lambda: run([
        sys.executable, __file__, "lookup", str(root), fmt, name, wanted]), repeats)
    result["latest_proxy"] = measure(lambda: select_latest(root, fmt, name), repeats)
    assert select_latest(root, fmt, name) == f"0.0.{count - 1}"
    published = []
    result["publish_metadata"] = measure(
        lambda: mutate(root, fmt, name, new=version(count + len(published), name, extra))
        or published.append(1), repeats)
    result["yank_metadata"] = measure(lambda: mutate(root, fmt, name, yank=wanted), repeats)
    commit(root, "publish and yank sample")
    result["clone_full"] = measure(lambda: clone_once(root, work, False), repeats)
    result["clone_shallow"] = measure(lambda: clone_once(root, work, True), repeats)
    cache = work / f"{root.name}-incremental"
    git(work, "clone", "-q", root.as_uri(), cache)
    increment_samples, object_growth = [], []
    for i in range(repeats):
        before = sum(p.stat().st_size for p in (cache / ".git" / "objects").rglob("*") if p.is_file())
        mutate(root, fmt, name, new=version(count + repeats + i, name, extra))
        commit(root, f"incremental {i}")
        start = time.perf_counter_ns()
        git(cache, "fetch", "-q", "origin")
        git(cache, "reset", "--hard", "-q", "FETCH_HEAD")
        increment_samples.append((time.perf_counter_ns() - start) / 1e6)
        after = sum(p.stat().st_size for p in (cache / ".git" / "objects").rglob("*") if p.is_file())
        object_growth.append(after - before)
    result["incremental_fetch_reset"] = stats(increment_samples)
    result["incremental_object_bytes"] = object_growth
    base = package_dir(root, name)
    if fmt == "split":
        exact_files = [base / name / "package.json", base / name / "versions" / f"0.0.{count - 1}.json"]
        discover_files = exact_files + [base / name / "versions.json"]
    else:
        exact_files = [base / f"{name}.json"] if fmt == "array" else [base / f"{name}.meta.json", base / f"{name}.jsonl"]
        discover_files = exact_files
    result["hypothetical_http"] = {
        "exact_requests": len(exact_files),
        "exact_gzip_body_bytes": sum(len(gzip.compress(p.read_bytes(), mtime=0)) for p in exact_files),
        "discovery_requests": len(discover_files),
        "discovery_gzip_body_bytes": sum(len(gzip.compress(p.read_bytes(), mtime=0)) for p in discover_files),
        "note": "Body sizes only, no HTTP latency, headers, ETag or CDN measurements; not current ccgo transport."}
    return result


def clone_once(root, work, shallow):
    # Retain each clone until the complete experiment ends, avoiding timed deletion.
    dest = work / f"clone-{time.time_ns()}"
    args = ["clone", "-q"] + (["--depth", "1"] if shallow else [])
    git(work, *args, root.as_uri(), dest)


def history(work, fmt, count):
    root = work / f"history-{fmt}"
    root.mkdir()
    git(root, "init", "-q", "-b", "main")
    write_package(root, fmt, "stdcomm", [])
    commit(root, "empty package")
    timings = []
    for i in range(count):
        start = time.perf_counter_ns()
        mutate(root, fmt, "stdcomm", new=version(i))
        # All formats update the shared root metadata as ccgo does today.
        (root / "index.json").write_bytes(encode(dict(version=1, name="bench", package_count=1,
                                                     updated_at=version(i)["released_at"]), True))
        commit(root, f"publish {i}")
        timings.append((time.perf_counter_ns() - start) / 1e6)
    result = dict(format=fmt, sequential_releases=count, per_release_ms=stats(timings),
                  before_gc=git_size(root), disk=disk(root))
    git(root, "gc", "--quiet")
    result["after_gc"] = git_size(root)
    result["full_clone"] = measure(lambda: clone_once(root, work, False), 3)
    result["shallow_clone"] = measure(lambda: clone_once(root, work, True), 3)
    return result


def main():
    if len(sys.argv) > 1 and sys.argv[1] == "lookup":
        lookup(Path(sys.argv[2]), *sys.argv[3:6])
        return
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--repeats", type=int, default=5)
    ap.add_argument("--history", type=int, default=1000)
    ap.add_argument("--scales", type=int, nargs="+", default=[1000, 10000, 100000])
    ap.add_argument("--multi-packages", type=int, default=100)
    ap.add_argument("--multi-versions", type=int, default=100)
    args = ap.parse_args()
    result = dict(environment=dict(python=sys.version, platform=platform.platform(),
                                   machine=platform.machine(), git=run(["git", "--version"]).decode().strip(),
                                   captured_at=dt.datetime.now(dt.timezone.utc).isoformat()),
                  parameters=vars(args) | {"output": str(args.output)}, snapshots=[], histories=[])
    args.output.parent.mkdir(parents=True, exist_ok=True)
    def save():
        args.output.write_text(json.dumps(result, indent=2) + "\n")
    with tempfile.TemporaryDirectory(prefix="ccgo-index-benchmark-") as tmp:
        work = Path(tmp)
        scenarios = [(n, 1, 0) for n in args.scales]
        scenarios += [(args.multi_versions, args.multi_packages, 0), (1000, 1, 2048)]
        for count, packages, extra in scenarios:
            for fmt in FORMATS:
                print(f"snapshot {fmt} {packages}x{count} extra={extra}", flush=True)
                result["snapshots"].append(snapshot(work, fmt, count, packages, args.repeats, extra))
                save()
        for fmt in FORMATS:
            print(f"history {fmt} {args.history}", flush=True)
            result["histories"].append(history(work, fmt, args.history))
            save()


if __name__ == "__main__":
    main()
