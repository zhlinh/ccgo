#!/usr/bin/env python3
"""Record/verify source snapshots, implementation hashes and experiment artifacts."""
import argparse
import datetime as dt
import hashlib
import json
from pathlib import Path
import subprocess


SOURCES = {
    "cargo-index": "https://doc.rust-lang.org/cargo/reference/registry-index.html",
    "cargo-yank": "https://doc.rust-lang.org/cargo/commands/cargo-yank.html",
    "swift-registry": "https://raw.githubusercontent.com/swiftlang/swift-package-manager/main/Documentation/PackageRegistry/Registry.md",
    "swift-registry-spec": "https://raw.githubusercontent.com/swiftlang/swift-evolution/main/proposals/0292-package-registry-service.md",
    "cocoapods-cdn": "https://raw.githubusercontent.com/CocoaPods/cdn.cocoapods.org/master/README.md",
    "cocoapods-cdn-design": "https://blog.cocoapods.org/CocoaPods-1.7.2/",
    "conan-revisions": "https://docs.conan.io/2/tutorial/versioning/revisions.html",
    "conan-api": "https://docs.conan.io/2/reference/commands/list.html",
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--verify", action="store_true")
    args = ap.parse_args()
    root = Path(__file__).resolve().parents[2]
    report = root / "docs/architecture/registry-index-assessment"
    evidence = report / "evidence"
    manifest = evidence / "provenance.json"
    if args.verify:
        data = json.loads(manifest.read_text())
        for item in data["files"]:
            assert sha(root / item["path"]) == item["sha256"], f"changed: {item['path']}"
        b = json.loads((evidence / "benchmark.json").read_text())
        assert len(b["snapshots"]) == 15 and len(b["histories"]) == 3
        assert all(h["sequential_releases"] == 1000 for h in b["histories"])
        for s in b["snapshots"]:
            for key in ("exact_warm", "exact_process_cold", "latest_proxy", "publish_metadata",
                        "yank_metadata", "clone_full", "clone_shallow", "incremental_fetch_reset"):
                assert len(s[key]["samples_ms"]) == 5
        assert len(json.loads((evidence / "faults.json").read_text())["cases"]) == 7
        assert len(json.loads((evidence / "format-conflicts.json").read_text())["cases"]) == 12
        assert len(json.loads((evidence / "native.json").read_text())["scenarios"]) == 3
        t = json.loads((evidence / "transport.json").read_text())
        assert len(t["scenarios"]) == 9
        assert all(len(s["one_release_fetch_pack_bytes"]) == 3 for s in t["scenarios"])
        print(f"Verified {len(data['files'])} file hashes and experiment completeness.")
        return
    paths = set(report.rglob("*"))
    paths |= set((root / "scripts/registry-index-assessment").glob("*.py"))
    paths.add(root / "scripts/registry-index-assessment/native.rs")
    paths |= set((root / "src/registry").glob("*.rs"))
    paths |= {root / p for p in ("src/commands/publish.rs", "src/commands/fetch.rs", "src/commands/yank.rs",
              "src/commands/publish_artifact.rs", "src/commands/publish_index_destination.rs", "src/lockfile.rs",
              "src/config/ccgo_toml.rs", "Cargo.toml", "Cargo.lock")}
    files = [dict(path=str(p.relative_to(root)), bytes=p.stat().st_size, sha256=sha(p))
             for p in sorted(paths) if p.is_file() and p != manifest]
    data = dict(recorded_at=dt.datetime.now(dt.timezone.utc).isoformat(),
                git_head=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
                note="Includes the previously authorized, uncommitted publish package naming changes. Content hashes identify the assessed code; upstream HEAD alone does not.",
                sources=[dict(id=k, url=v, snapshot=f"evidence/{k}.source") for k, v in SOURCES.items()],
                files=files)
    manifest.write_text(json.dumps(data, indent=2) + "\n")
    print(f"Recorded {len(files)} file hashes.")


if __name__ == "__main__":
    main()
