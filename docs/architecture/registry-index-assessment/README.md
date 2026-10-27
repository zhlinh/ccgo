# Registry index architecture assessment

Decision date: 2026-10-27. Scope: the current working tree, including the previously
authorized `publish package` naming change. Exact source and evidence hashes are
recorded in [evidence/provenance.json](evidence/provenance.json).

## Decision

Retain the current per-package JSON array as the canonical Git index for now.
Do not migrate to JSON Lines or per-version files solely because a package may
have 10,000 versions. Fix publication recovery and locked-fetch correctness
before treating the registry as reproducible under failures or concurrent CI.

This is a conditional architectural decision, not a claim that the current
implementation is production-complete. Tests reproduced important correctness
defects that a storage-format migration would leave unresolved.

The strongest measured reason to retain the format is the actual access path:
`fetch` clones the entire Git registry, including history. With 100,000 versions,
splitting metadata improves an exact local lookup, but multiplies filesystem
objects and made a fresh clone/checkout about 16 times slower in this experiment.
An HTTP sparse reader could reverse that tradeoff; ccgo does not currently have
one. Changing files without changing transport would optimize the wrong path.

Full numerical tables, including sample ranges, are in
[MEASUREMENTS.md](MEASUREMENTS.md). Commands and limitations are in
[REPRODUCING.md](REPRODUCING.md).

## Findings that should drive implementation priorities

### 1. A generated registry lock is rejected by ordinary locked fetch

`install_from_registry_archive` writes `source = registry+<registry URL>`
([fetch.rs](../../../src/commands/fetch.rs#L809)), while
`Lockfile::check_outdated` builds `registry+<package name>@<configured version>`
and compares the strings ([lockfile.rs](../../../src/lockfile.rs#L301)).
Thus, even without changing configuration, index, or artifact,
`fetch --locked --force` rejects a lockfile produced by `fetch`.

Evidence: all three `locked_*` cases in [faults.json](evidence/faults.json)
record the unmodified locked preflight and its error. This result occurs before
the yank/checksum behaviors below. It is independent of JSON array layout.

Required invariant: a successful fetch followed by locked fetch, with unchanged
inputs and available artifacts, must succeed and preserve the lock.

### 2. The deeper registry path does not enforce the lock's content identity

To reach the code behind finding 1, experiments explicitly normalize only the
lock's source field to the string expected by the existing preflight. This is a
diagnostic workaround, not the original generated lock or an implementation fix.

After that workaround:

- Yanking the locked version prevents forced locked re-fetch: `resolve_dep`
  always skips yanked entries, regardless of lock state
  ([resolve.rs](../../../src/registry/resolve.rs#L50)). The promise at the top of
  [yank.rs](../../../src/commands/yank.rs#L10) is not implemented on this path.
- Changing only archive bytes fails SHA-256 verification, as intended.
- Changing both index checksum and archive bytes allows forced locked fetch to
  install replacement content and rewrite the lock checksum. Verification uses
  the freshly resolved index entry, not the old locked checksum
  ([fetch.rs](../../../src/commands/fetch.rs#L802),
  [fetch.rs](../../../src/commands/fetch.rs#L894)).

Evidence: `locked_yank`, `locked_replace_bytes`, and `locked_replace_both` in
[faults.json](evidence/faults.json), including the explicit workaround description.
The rewrite scenario models an administrator, compromised index, or external
publisher rewriting history; normal ccgo publication itself rejects duplicates.

The Git fallback also passes `None` as the locked revision when delegating to
`install_from_git` ([fetch.rs](../../../src/commands/fetch.rs#L842)). This is a
source finding; this assessment did not execute a moved-tag experiment.

Required invariant: locked registry dependencies use locked registry identity,
exact version, and checksum/commit. A fresh index may supply a locator, but must
not silently change the content anchor. Yank should affect new selection while
allowing the documented locked-version policy. Missing immutable content must
fail clearly, rather than select another source or update the lock.

### 3. Failure preserves local work but publication is not resumable

The sequence is: synchronize index -> reject duplicates -> upload immutable
asset -> write package and root metadata -> commit -> push
([publish.rs](../../../src/commands/publish.rs#L1207)).

Experiments with the real CLI, local Git remote and a modeled GitHub service:

| Trigger | Observed result | Ordinary retry | Recovery demonstrated |
|---|---|---|---|
| Asset succeeds, pre-commit hook fails | Asset exists; remote has no version; local worktree is dirty | `pull --rebase` fails on dirty state | Commit retained state and push |
| Remote pre-receive hook rejects push | Asset and local commit exist; remote has no version | Local duplicate check rejects already committed version | Push retained commit |
| Two different versions reach upload from the same base | Both upload; one push succeeds, one is rejected | Rebase conflicts | Manual conflict resolution is required; not automatically tested as a successful recovery |
| Same version races against an immutable asset service | One upload succeeds, one fails; remote has one version | Subsequent duplicate rejected | No remote version overwritten |

See [faults.json](evidence/faults.json). Failures are reported, not silently
acknowledged as success. Git rejects stale-head pushes, so the experiment does
not show silent loss of an already accepted remote commit. It does show a
missing index entry for the rejected publisher, one orphan artifact, and no
automatic recovery.

The asset-service stand-in enforces create-only storage and a deterministic
barrier. It is not a live GitHub or Maven concurrency test. Maven upload uses
ordinary HTTP PUT without a client-side create-only precondition
([publish_artifact.rs](../../../src/commands/publish_artifact.rs#L29)); whether
an existing asset can be overwritten depends on server policy. The GitHub path
uses `gh release upload` without `--clobber`.

Required invariants: a duplicate with different immutable content must fail;
retrying the same transaction must either resume or confirm identical published
content without another upload. Uploaded-but-unindexed state must be visible
and recoverable. Index success must never precede successful asset upload.

### 4. Changing formats does not remove shared write conflicts

All publications update `index.json.updated_at`. The writer also traverses the
index tree to recount JSON package files
([index_writer.rs](../../../src/registry/index_writer.rs#L111)).

Twelve isolated Git experiments compare three formats, same/different packages,
and presence/absence of shared root updates:

- A stale-head push is rejected in every layout.
- Different packages rebase cleanly when they do not update root metadata.
- Updating root metadata creates conflicts even for different packages in every
  layout in this fixture.
- Same-package releases conflict in the array, JSONL tail, or split design's
  version catalog even when root metadata is excluded.

See [format-conflicts.json](evidence/format-conflicts.json). These are layout/Git
model tests, separate from the real CLI concurrency test. They establish that
these conflicts are possible, not a measured production conflict probability.

Publication clones are shared by registry name with no process lock. Files use
`fs::write`, not atomic replacement, and Git stages `-A`
([index_writer.rs](../../../src/registry/index_writer.rs#L169)). Concurrent
processes using one `CCGO_HOME` can interfere with a shared worktree/index. This
is a code-level gap; the deterministic CLI race deliberately uses separate homes
to avoid conflating that issue with remote push conflicts.

### 5. Registry identity and metadata need explicit semantics

`RegistriesConfig` stores registrations in a `HashMap`
([ccgo_toml.rs](../../../src/config/ccgo_toml.rs#L172)), and `fetch` iterates it
when the dependency does not choose a registry
([fetch.rs](../../../src/commands/fetch.rs#L709)). Despite comments promising
declaration order, the container does not preserve TOML order. Two registries
containing the same name/version can therefore select different sources across
processes. This is a source finding; no statistical priority experiment is
claimed. Explicit `registry = ...` avoids that ambiguity.

The package's repository, license and platforms are rebuilt from today's
configuration on every publication
([publish.rs](../../../src/commands/publish.rs#L1124)). Historical entries have
no independent repository/platform/license snapshot. A repository change can
redirect old versions' Git fallback; tags alone are mutable references. The
model has optional checksums and archive URLs, so a metadata entry is not by
itself a guarantee of content immutability or availability.

The binary reader currently extracts ZIP regardless of `archive_format`
([fetch.rs](../../../src/commands/fetch.rs#L806)), while the data model documents
`tar.gz` as another value. All executed archive cases use ZIP. Broader format
support should be aligned before promising it to publishers.

## Current implementation: verified read/write paths

| Operation | Actual path and behavior | Important boundary |
|---|---|---|
| Data model | `PackageEntry.versions: Vec<VersionEntry>`; optional checksum, URL, format, date; `yanked_reason` also exists ([index.rs](../../../src/registry/index.rs#L91)) | The example is valid, but not every field is mandatory |
| Filename | Cargo-style name sharding, one `.json` per package ([index.rs](../../../src/registry/index.rs#L283)) | Sharding limits directory fanout, not per-package growth |
| Publish | SemVer validates the new version; checks source tag exists; reads prior entries, rejects exact duplicate, sorts version strings lexically, pretty-serializes whole entry ([publish.rs](../../../src/commands/publish.rs#L305)) | Append-only semantic operation, not append-only physical write; lexical order is not SemVer order |
| External asset | Provider mode freezes input bytes, derives URL/checksum, uploads before index commit; URL-template mode assumes external hosting | Template URLs are not verified for continued existence or matching remote content |
| Publication Git cache | `<CCGO_HOME>/registry/publish/<name>`; depth-1 first clone, then `pull --rebase`, commit, optional push ([index_writer.rs](../../../src/registry/index_writer.rs#L25)) | Preserves failed local changes; no semantic retry or transaction lock |
| Fetch Git cache | `<CCGO_HOME>/registries/<name>`; full clone, then fetch and reset to `FETCH_HEAD` ([cache.rs](../../../src/registry/cache.rs#L43)) | Every registry's complete current tree and history are obtained; no sparse HTTP |
| Fetch version choice | Read entire package into typed structs, linear exact string match, always skip yanked ([resolve.rs](../../../src/registry/resolve.rs#L40)) | This path does not implement SemVer range selection |
| Management APIs | `PackageIndex` uses another cache at `registry/index`, depth-1 clone/ff-only pull, has separate SemVer-oriented lookup APIs ([index.rs](../../../src/registry/index.rs#L404)) | Management lookup is not the same path as `fetch` |
| Artifact fetch | Download into memory, optional SHA-256 verification, ZIP extraction; otherwise Git/tag fallback ([fetch.rs](../../../src/commands/fetch.rs#L758)) | Already-installed dependencies can bypass re-verification; forced clean retrieval was tested |
| Yank | Read full JSON, change flag/reason, rewrite, refresh root metadata, commit/push ([yank.rs](../../../src/commands/yank.rs#L75)) | Version URL/checksum stay present, but locked fetch does not yet honor promised semantics |

`try_resolve_registry` synchronizes candidates on each dependency call, rather
than once for the entire fetch operation. There is no freshness TTL, offline
fallback on synchronization failure, or in-memory parsed package cache in this
path. A cache hit therefore does not mean a network-free command. The general
fetch home helper also ignores `CCGO_HOME`, unlike `RegistryCache`; the archive
experiments avoid global fallback data and use explicit local registry caches.

## Measurements and interpretation

### Representative version records

The generator uses distinct SemVer strings, tags, SHA-256 values and minute-by-
minute timestamps, with URLs shaped like the supplied Maven URL. Checksums are
deterministic hashes of package/version identities, not repeated constants. It
tests 1,000, 10,000 and 100,000 versions, plus 100 packages x 100 versions and a
hypothetical additional 2,048 bytes per version. Future metadata is sensitivity
analysis, not a field currently consumed by ccgo.

Layouts:

1. **Array:** today's pretty JSON, repeated whole-file rewrite and lexical sort.
2. **JSONL:** package metadata file plus compact version records, one JSON per
   line; publication scans duplicates before appending; yank rewrites the file.
   No claim is made that appending is atomic or that yank is append-only.
3. **Split:** package metadata, `versions.json` catalog, one compact version JSON
   per version. Exact requests address the version directly; latest discovery
   reads the catalog, sorts SemVer numbers, then checks candidate yank status.

Candidate publication operations are metadata-only Python models. The real
Rust exact resolver is measured separately by importing unchanged production
modules into a release build. Real CLI publish/yank are measured with the
existing debug binary and local file remotes; timings are not release-binary
performance claims.

Snapshot format comparisons contain package metadata files without the small
shared root document. History runs and native CLI operations include root
metadata updates. Numeric `0.0.N` versions do not exercise prerelease precedence,
build metadata, dependency graph resolution or mixed-version constraints.

### Storage at 10,000 and 100,000 versions

| Layout | 10k logical MB | 100k logical MB | 100k allocated MB | 100k packed snapshot `.git` MB |
|---|---:|---:|---:|---:|
| Array | 3.966 | 40.056 | 40.059 | 5.805 |
| JSONL | 3.306 | 33.456 | 33.460 | 5.730 |
| Split | 3.405 | 34.545 | 410.796 | 39.420 |

Compact JSONL saves about 16.5% of uncompressed bytes against the current pretty
array here, but snapshot Git compression nearly erases that difference. Split
records consume about 12 times their logical bytes in allocated blocks at 100k,
before counting directory metadata and clones. Tar/gzip and Git pack are
different compression formats; both are reported, not equated.

In the multi-package scenario, array and split logical totals are 3.935 MB and
3.347 MB, but fresh full clone/checkout medians are 73 ms and 1,067 ms. Total
versions alone therefore do not predict lookup or filesystem costs.

### Read/write cost on the actual implementation

| Versions in one package | Rust exact lookup median [min, max] ms | Resolver peak RSS range MB | CLI local publish median ms | CLI yank median ms |
|---:|---:|---:|---:|---:|
| 1,000 | 0.34 [0.33, 0.48] | 7.01-7.09 | 102 | 216 |
| 10,000 | 3.18 [3.04, 3.87] | 16.15-17.65 | 289 | 477 |
| 100,000 | 33.25 [32.35, 38.76] | 160.15-164.81 | 2,234 | 3,303 |

RSS is for the optimized resolver process, not complete artifact download or
CLI execution. The lookup cost includes full typed deserialization and scan.
Warm sample sets include the first lookup within each process; raw samples
permit separating them. Fresh-process measurements do not flush the OS page
cache and must not be called physical disk-cold reads.

Per-version files make exact local lookup effectively independent of version
count in the candidate implementation. However, current ccgo has no such
reader, and its current Git reader would still obtain all those files first.
The snapshot full-clone medians at 100k are 0.816 s (array), 0.719 s (JSONL),
and 13.331 s (split), with all sample ranges retained in MEASUREMENTS.md.

### Git history and transport

The history experiment actually performs 1,000 publications, one version per
commit, including shared root metadata changes. It does not synthesize a final
snapshot and call that historical growth.

| Layout | `.git` before explicit GC MB | `.git` after GC MB |
|---|---:|---:|
| Array | 34.314 | 1.397 |
| JSONL | 32.818 | 1.411 |
| Split | 14.518 | 1.593 |

This demonstrates useful Git delta compression, not a guarantee about 100k
historical commits, all Git configurations, or permanently retained remote
history. History is measured at 1,000 releases only; larger historical runs are
an explicit remaining limit. Snapshot scale reaches 100,000 versions.

Local incremental object growth can be much larger than received traffic:
Git may materialize a received delta as a complete loose blob. A separate
`GIT_TRACE_PACKFILE` experiment records actual received PACK stream sizes for
one-release updates at all three scales, with three repetitions. Consult the
PACK table in MEASUREMENTS.md, not `incremental_object_bytes`, for transport
payload. PACK excludes protocol framing, SSH/TLS, authentication and WAN
latency. The source is `file://`, deliberately avoiding Git's local hardlink
clone shortcut.

At 100k baseline versions, the array's three one-release PACK payloads are
1,560/1,564/1,566 bytes, JSONL's are 1,393/1,405/1,396 bytes, and split's are
950/934/942 bytes. Rewriting a 40 MB JSON file therefore does not imply sending
40 MB on each Git update. This observed delta behavior is a substantial reason
not to migrate based on file size alone. Actual server packing policy and
history can change those payloads.

### Hypothetical sparse HTTP

Current ccgo cannot use this result directly. For 100k versions, a hypothetical
exact HTTP request for the array needs about 5.685 MB of gzip body, whereas
split metadata plus one version needs about 422 bytes across two responses.
Version discovery still needs the split catalog, which grows with version
count. More candidate reads may be needed when versions are yanked.

These are body-size calculations, not HTTP load tests. There is no measured CDN
hit rate, ETag reuse, request latency or authenticated corporate mirror result.
A meaningful split-format proposal must include such a transport and cache
design rather than infer its benefits from local point lookup alone.

## Comparison with primary ecosystem sources

The downloaded original documents and content hashes are retained under
`evidence/`; URLs and snapshot filenames are in provenance.json. The following
describes verified mechanisms, not an assertion that ccgo should copy them.

| Ecosystem | Discovery | Version metadata/content | Relevant lesson for ccgo |
|---|---|---|---|
| SwiftPM Git mode | Git tags in each source repository | Manifest at selected revision; source obtained through Git | No central versions-array requirement, but moving tags/history/availability remain concerns |
| Swift Package Registry | List-releases endpoint | Per-release metadata, per-release manifest and ZIP endpoints; SHA-256 metadata | Separate discovery from immutable content; requires an HTTP registry contract |
| CocoaPods | Generated sharded pod/version lists | Per-version podspec JSON; podspec identifies source artifacts | Tiny point reads help because the CDN reader avoids cloning all Specs |
| Cargo | One index file per crate, one JSON record per version | Checksums and dependency metadata in the index; archive endpoint configured separately | Growing per-package files are a valid mature design; sparse HTTP and cache validation are independent of JSONL |
| Conan 2 | Remote/cache queries by references, versions, revisions and package IDs | Recipe revision and binary package revision identify different immutable contents | A plain version number is insufficient for its binary matrix; copying this model implies more schema and remote-query machinery |

Primary references:

- SwiftPM's [registry specification](https://github.com/swiftlang/swift-package-manager/blob/main/Documentation/PackageRegistry/Registry.md), sections 4.1-4.4, separates release listing, metadata, manifests, archive download and integrity verification. The [accepted SE-0292 proposal](https://github.com/swiftlang/swift-evolution/blob/main/proposals/0292-package-registry-service.md), Motivation and Git-to-HTTP mapping, documents the traditional tag-based Git mode and its tradeoffs.
- CocoaPods' [CDN implementation README](https://github.com/CocoaPods/cdn.cocoapods.org/blob/master/README.md) gives actual `all_pods_versions_<shard>.txt` and `Specs/<hash>/<pod>/<version>/<pod>.podspec.json` paths and explains avoiding a complete Specs clone. The [official CDN announcement](https://blog.cocoapods.org/CocoaPods-1.7.2/) provides historical context, not a substitute for the current implementation README.
- Cargo's [index format and protocols](https://doc.rust-lang.org/cargo/reference/registry-index.html) specifies one line per version, mutable yank status, Git versus sparse protocols, and ETag/Last-Modified caching. Its [yank command reference](https://doc.rust-lang.org/cargo/commands/cargo-yank.html) explicitly distinguishes new resolution from existing locked versions.
- Conan's [revisions documentation](https://docs.conan.io/2/tutorial/versioning/revisions.html) explains revision hashes, same-version recipe changes and binary revisions. Its [list command reference](https://docs.conan.io/2/reference/commands/list.html) shows reference/revision/package-ID queries against cache and remotes. This assessment does not benchmark a Conan server or assert a particular backend database layout.

Swift's proposals and current specification are distinguished. CocoaPods'
older announcement is not used as evidence of today's deployment cadence.
Two guessed SwiftPM documentation paths returned 404; successful primary
sources above replace them. No unavailable document is treated as evidence.

## Minimal changes recommended before a schema migration

1. **Repair lock semantics.** Give registry dependencies a stable identity;
   preserve the locked checksum/commit; permit documented locked-yank behavior;
   never rewrite a lock under `--locked`. Test generated-lock re-fetch first,
   then the deeper tampering and yank cases without any diagnostic workaround.
2. **Make publication resumable.** Persist transaction state including target
   registry, package/version, immutable content hash, upload URL and Git base.
   Before another upload, reconcile remote publication and retained local state.
   Identical already-published content can be idempotent success; different
   content is an error. Retained dirty files/commits need a supported resume
   path, not unconditional duplicate rejection.
3. **Serialize or isolate writers.** Initially use a registry-wide CI queue;
   add an interprocess lock for shared local clones. A scalable writer can use
   isolated worktrees plus bounded optimistic retries that re-read latest JSON
   and reapply the semantic operation. A plain text rebase is insufficient.
4. **Enforce immutable version fields at the trust boundary.** Validate pushes
   server-side, or through a protected CI writer, allowing only new versions
   and explicit yank state/reason changes. Require checksums for binary assets;
   require create-only/immutable asset policy or content-addressed URLs. Keep
   external-service orphan reconciliation explicit; never delete a competing
   publisher's object blindly.
5. **Make metadata and source rules explicit.** Use atomic file replacement;
   exclude `.git` from package scans; avoid updating derived root counters and
   timestamps as part of every independent package transaction where possible.
   Establish deterministic registry priority or reject ambiguity; use explicit
   registry identities in locks. Capture version-specific metadata when it
   affects build behavior, and align archive format promises with the reader.
6. **Optimize the existing read path before changing files.** Synchronize each
   registry once per fetch invocation, cache parsed entries for that invocation,
   and measure whether current-state/shallow or bounded-history caches can
   replace full-history clones. Preserve explicit refresh and offline semantics.
   Removing pretty whitespace is optional and does not address the main costs.

These are recommendations, not changes implemented by this assessment. The
fastest initial operational mitigation is one publishing queue per registry,
while preserving local transaction state for manual recovery.

## When to reconsider the format

Use service budgets and measured production traces, not a universal version
count limit. Suggested initial review triggers, to be calibrated on CI hardware:

- Exact metadata lookup p95 exceeds 50 ms, or resolver RSS exceeds the project's
  budget (100 MB would already be exceeded by the measured 100k array case).
- Cold registry acquisition p95 exceeds 5 seconds or registry synchronization
  becomes a material share of dependency-install time; establish network and
  history contributions before blaming the file format.
- Single-package metadata grows past roughly 10 MB and repeated deserialization
  is a demonstrated cost, especially with larger per-version dependency or
  platform matrices. This is an investigation trigger, not a hard capacity cap.
- Many consumers need a few exact versions from a large registry: prototype
  HTTP sparse access. The same per-package array can be served over HTTP first;
  JSONL is not required to obtain most of the whole-registry transport benefit.
- Persistent publish conflicts or orphan assets trigger writer/recovery work
  immediately, regardless of registry size. They do not alone justify split files.

Five timing repeats do not establish production p95. These thresholds are
proposed operational policy; the measurements in this report are medians/ranges.

## Compatible evolution if a later experiment justifies change

Prefer adding a read projection over replacing the canonical writer first:

1. Keep the v1 Git array schema and immutable-version validation authoritative.
2. Generate a separately addressed, versioned HTTP projection from a single
   committed Git snapshot. It may initially serve the existing per-package
   file, later a package document, discovery catalog and version records.
3. Publish the projection atomically or switch a snapshot pointer only after
   all referenced records exist. Validate checksum/yank equivalence against
   the source commit. Cache immutable version content separately from mutable
   yank/discovery status; a permanently cached `yanked: false` is incorrect.
4. Add explicit client capability/protocol selection, conditional requests and
   authentication/cache semantics. Keep v1 readers on the old URL. A root
   `version` field alone is insufficient because today's fetch does not gate
   lookup on that metadata.
5. Avoid independently writable v1 and v2 stores. Do not mix thousands of
   version JSON files into the current package-count/search layout: today's
   JSON-file counter would count them as packages.
6. Roll out against an equivalence suite, offer fallback to v1, and deprecate
   old writers/readers only after actual client adoption is measured.

JSONL alone would require reader/writer changes and compatibility handling for
modest compressed-size benefit here. Split files become compelling when
combined with on-demand HTTP or substantially larger version metadata, not
because they resemble another ecosystem's directory tree.

## Evidence boundaries

Measured: synthetic current-shape metadata at all requested snapshot scales;
100-package case; metadata expansion sensitivity; actual 1,000-commit history;
full/shallow clones and incremental updates; received PACK bytes; native Rust
lookup/RSS; real debug CLI publication/yank; seven CLI fault characterizations;
twelve layout conflict experiments. Each timing set retains raw samples.

Not measured: the user's corporate Git/Maven servers, production CI logs or
release cadence, real GitHub/Maven failure semantics, WAN/TLS/auth latency,
OS-cold disk reads, sustained load, 100k sequential historical commits,
simultaneous writers sharing one home, moved-tag locked retrieval, or a working
sparse HTTP implementation. CPU/filesystem activity was not isolated from other
experiments. No statistical tail latency or production conflict rate is claimed.

Existing relevant checks passed: 42 registry-related unit tests and 17 publishing
integration tests. Passing characterizations mean the observed defects were
reproduced; they do not mean those defects have been fixed. No production
schema migration or corrective implementation is included.
