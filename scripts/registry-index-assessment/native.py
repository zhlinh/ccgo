#!/usr/bin/env python3
"""Measure actual Rust resolver and CLI publishing, separate from Python format models."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

from benchmark import ENV, commit, encode, git, meta, run, stats, version, write_package


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--output", required=True, type=Path)
    ap.add_argument("--binary", type=Path, default=Path("target/debug/ccgo"))
    ap.add_argument("--repeats", type=int, default=5)
    args = ap.parse_args()
    repo = Path(__file__).resolve().parents[2]
    binary = args.binary.resolve()
    evidence = dict(binary=str(binary), binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                    rustc=run(["rustc", "--version"]).decode().strip(),
                    note="Resolver: release-optimized temporary crate importing production modules. CLI: supplied binary (debug in recorded run). Cold means fresh process, OS page cache not flushed.",
                    scenarios=[])
    with tempfile.TemporaryDirectory(prefix="ccgo-index-native-") as tmp:
        root = Path(tmp)
        crate = root / "probe"
        (crate / "src").mkdir(parents=True)
        (crate / "src/main.rs").write_text((repo / "scripts/registry-index-assessment/native.rs").read_text().replace("__ROOT__", str(repo)))
        (crate / "Cargo.toml").write_text('''[package]
name = "ccgo-index-native"
version = "0.1.0"
edition = "2021"
[dependencies]
anyhow = "1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
walkdir = "2.5"
''')
        # Reuse the checked-in dependency versions where possible; Cargo updates root package only.
        (crate / "Cargo.lock").write_bytes((repo / "Cargo.lock").read_bytes())
        run(["cargo", "build", "--release", "--offline", "--manifest-path", crate / "Cargo.toml"])
        probe = crate / "target/release/ccgo-index-native"
        evidence["probe_cargo_lock"] = (crate / "Cargo.lock").read_text()
        for count in (1000, 10000, 100000):
            print(f"native {count}", flush=True)
            home = root / f"home-{count}"
            index = home / "registries/bench"
            entries = sorted((version(i) for i in range(count)), key=lambda v: v["version"], reverse=True)
            write_package(index, "array", "stdcomm", entries)
            env = dict(ENV, CCGO_HOME=str(home))
            cold, warm, memory = [], [], []
            for _ in range(args.repeats):
                start = time.perf_counter_ns()
                timed = subprocess.run(["/usr/bin/time", "-l", str(probe), f"0.0.{count // 2}", str(args.repeats)],
                                       env=env, capture_output=True, text=True, check=True)
                cold.append((time.perf_counter_ns() - start) / 1e6)
                warm.extend(json.loads(timed.stdout))
                match = re.search(r"(\d+)\s+maximum resident set size", timed.stderr)
                memory.append(int(match.group(1)) if match else None)
            source = root / f"source-{count}"
            source.mkdir()
            (source / "CCGO.toml").write_text("[package]\nname='stdcomm'\nversion='0.0.0'\n")
            git(source, "init", "-q", "-b", "main")
            commit(source, "source")
            git(source, "remote", "add", "origin", "https://example.invalid/stdcomm.git")
            # Local publication isolates real read/duplicate/sort/serialize/metadata/commit costs.
            git(index, "init", "-q", "-b", "main")
            git(index, "config", "receive.denyCurrentBranch", "updateInstead")
            commit(index, "index baseline")
            publish_samples, yank_samples = [], []
            for i in range(args.repeats):
                v = f"0.0.{count + i}"
                git(source, "tag", f"v{v}")
                start = time.perf_counter_ns()
                p = subprocess.run([str(binary), "publish", "package", "--registry", "local",
                                    "--index-repo", str(index), "--version", v, "--push"],
                                   cwd=source, env=env, capture_output=True, text=True, check=True)
                publish_samples.append((time.perf_counter_ns() - start) / 1e6)
                start = time.perf_counter_ns()
                subprocess.run([str(binary), "yank", "--package", "stdcomm", "--vers", v,
                                "--reason", "benchmark", "--index-repo", index.as_uri(),
                                "--index-name", "yank-bench", "--push"], cwd=source,
                               env=env, capture_output=True, text=True, check=True)
                yank_samples.append((time.perf_counter_ns() - start) / 1e6)
                # Synchronize local publication destination with yank's pushed commit.
                # (The destination itself is the origin, so push already updates it.)
            evidence["scenarios"].append(dict(versions=count,
                resolver_in_process=stats(warm), fresh_process_five_lookups=stats(cold),
                max_rss_bytes_macos=memory, cli_publish_local=stats(publish_samples),
                cli_yank_remote_file=stats(yank_samples)))
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(evidence, indent=2) + "\n")


if __name__ == "__main__":
    main()
