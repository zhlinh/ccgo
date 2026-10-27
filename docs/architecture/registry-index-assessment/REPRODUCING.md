# Reproducing the assessment

Run commands from the ccgo repository root. Production index files are never
migrated by these scripts. Git repositories, assets, caches and temporary probe
builds live in newly created temporary directories and are cleaned by Python.
Only the selected evidence output files are retained in this repository.

## Requirements and recorded environment

- Python 3.9+ standard library; recorded Python 3.9.6.
- Git; recorded Apple Git 2.50.1.
- Cargo/Rust and the repository's cached dependencies for `--offline` builds.
  The exact Rust compiler and temporary probe dependency lock are retained in
  `evidence/native.json`; root Cargo.lock is hashed in provenance.json.
- Recorded platform: macOS 26.2, arm64. Native RSS collection uses macOS
  `/usr/bin/time -l` and is not a portable Linux memory benchmark.
- A current `target/debug/ccgo`, or pass a different binary to `native.py` and
  `faults.py`. Recorded binary content hashes are in both output files.
- Several GB of temporary disk headroom. The split 100k scenario creates many
  files and clone copies; cleanup itself can take time.

The full experiment does not contact real package services. Local `file://`
Git remotes invoke upload-pack rather than the hardlink-based local clone
shortcut. No real credentials are read by the asset stand-in. Git global and
system configuration are disabled for the experiment helper commands. The
stand-in for GitHub implements only release-view and create-only asset-upload
behavior, not the whole GitHub API.

`fetch` prints the ordinary user home because its general home helper currently
ignores `CCGO_HOME`; the registry cache honors the fixture override. Archive
fixtures do not invoke the Git/local-package installation path that writes to
that general home. Do not add `--clean-cache` to these experiments.

## Build and execute

```sh
cargo build --offline --bin ccgo

python3 scripts/registry-index-assessment/benchmark.py \
  --output docs/architecture/registry-index-assessment/evidence/benchmark.json

python3 scripts/registry-index-assessment/transport.py \
  --output docs/architecture/registry-index-assessment/evidence/transport.json

python3 scripts/registry-index-assessment/native.py \
  --output docs/architecture/registry-index-assessment/evidence/native.json

python3 scripts/registry-index-assessment/faults.py \
  --output docs/architecture/registry-index-assessment/evidence/faults.json

python3 scripts/registry-index-assessment/format_conflicts.py \
  --output docs/architecture/registry-index-assessment/evidence/format-conflicts.json

python3 scripts/registry-index-assessment/summarize.py
```

`native.py` builds a small release-optimized temporary crate that imports the
repository's production registry modules and version parser unchanged. It
copies the root Cargo.lock before offline resolution, and records the resulting
probe lock for auditing. The temporary crate's compiler profile differs from
the debug CLI: compare like with like.

On a restricted macOS sandbox, `/usr/bin/time -l` can fail while reading resource
statistics. Permit this experiment to run outside that sandbox or adapt memory
collection to the local platform, documenting the change. It does not require
real registry access.

Run timing experiments sequentially on an otherwise idle machine for stronger
comparability. The recorded run partly overlapped independent experiments and
was not CPU-pinned. Record that distinction when comparing absolute numbers.
Each metadata/clone timing scenario has five samples; history publication has
1,000 samples, history clones have three, PACK measurements have three, and
native resolution has five processes with five lookups per process.

The native CLI yank series clones on its first sample and reuses that clone
later, which is visible in raw timings. Python metadata yank repeatedly writes
the same desired flag as a rewrite-cost model; the real CLI rejects repeated
yanks, so the native series yanks a newly published version each time.

The Python process-cold benchmark includes interpreter startup and receives no
OS cache flush. Clone-cold means a fresh Git destination, not a cold page cache.
True cold-storage and real-network benchmarks remain separate work.

For a quick harness check, use smaller `benchmark.py --scales ... --history ...`
values and a different output filename. Such a run does not reproduce the
retained full-scale evidence or satisfy the completeness verifier.

## Correctness checks and interpreting failures

```sh
cargo test --offline --bin ccgo registry::
cargo test --offline --test publish_registry
```

The recorded checks passed 42 and 17 tests respectively. The scripts add seven
real-CLI characterization scenarios and twelve layout/Git conflict scenarios.
They raise assertions if the characterized outcome changes unexpectedly.

Several assertions deliberately expect a defect, including failed ordinary
retry and rejected generated locks. Script success means reproduction, not
correct production behavior. When implementation fixes land, replace those
characterizations with tests of the desired invariants rather than maintaining
the faulty outcome.

For the deeper yank and checksum tests, `faults.py` first proves the unmodified
generated lock fails. It then explicitly rewrites only the fixture's source
identity to bypass that preflight defect and exercise later code. Each result
stores the workaround and both command outputs. Do not omit this qualification
when citing the deeper outcomes.

The different-version race uses independent homes and an upload barrier so both
publishers have read the same index base before uploading. It does not depend
on a sleep to force a particular winner. Git hooks inject commit/push failures;
recovery assertions confirm retained state can be committed/pushed manually.

## Primary-source snapshots

The eight original sources are retained as `evidence/*.source`. Their URLs and
SHA-256 hashes are in provenance.json. HTML pages contain navigation as well as
body content; the report cites the substantive sections. GitHub URLs use
upstream branch names, so a future fetch can differ: the retained bytes and
hashes identify what this assessment actually read.

To refresh a source deliberately, use its URL from provenance.json, for example:

```sh
curl -sSL --fail --max-time 45 \
  https://doc.rust-lang.org/cargo/reference/registry-index.html \
  -o docs/architecture/registry-index-assessment/evidence/cargo-index.source
```

External reads require network permission in a restricted environment. Experiments
themselves run without external network after Cargo dependencies are available.
The SwiftPM Git behavior is sourced from accepted SE-0292 plus the current
registry specification; guessed Usage/PackageDependencies paths returned 404
and were not retained as supporting documents.

## Independent verification

To verify the retained artifact without changing its manifest:

```sh
python3 scripts/registry-index-assessment/provenance.py --verify
```

The verifier checks evidence/source/script/implementation hashes, expected
scenario counts and timing sample completeness. It cannot prove an architecture
decision or authenticate an upstream publisher by itself. An independent
reviewer should also rerun the experiments, inspect code locations, and assess
whether the synthetic assumptions match actual production traffic.

After an intentional new assessment, regenerate measurement tables, update
the narrative if needed, then record and verify new provenance:

```sh
python3 scripts/registry-index-assessment/summarize.py
python3 scripts/registry-index-assessment/provenance.py
python3 scripts/registry-index-assessment/provenance.py --verify
```

The existing source tree already contains uncommitted naming changes from the
prior task. Provenance hashes the actual assessed files rather than assuming
Git HEAD describes that working tree. This assessment adds scripts and reports;
it does not change production index schema or implement the recommended fixes.
