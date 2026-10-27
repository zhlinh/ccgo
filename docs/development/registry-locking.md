# Registry locking: first optimization stage

The per-package JSON index format remains unchanged. This stage addresses
registry identity and reproducible retrieval before changing storage layout.

## Implemented behavior

- Fresh resolution requires an explicit registry when multiple registries match.
  A selected registry with no match fails instead of falling back to local cache.
- New archive locks always contain the SHA-256 of downloaded bytes, even when
  the index omits a checksum. Git locks retain the resolved commit.
- `ccgo fetch --locked` selects the registry URL and exact version recorded in
  the lock. It verifies the locked checksum or full Git commit and never rewrites
  the lockfile. Index checksum changes cannot replace a locked checksum.
- Yanked versions remain retrievable with an existing lock but are excluded
  from fresh resolution. Missing locked versions fail without registry fallback.
- Legacy source spelling is accepted only when registry identity is unique.
  Locks without a content anchor require `ccgo fetch --force` without `--locked`.
- Git checkouts are isolated by commit when pinned. Dirty pinned checkouts fail;
  `--force` retrieves the commit again. Registry cache origins are checked before
  reuse. Workspace member failures propagate to the command exit status.

Locked registry retrieval currently synchronizes the index and revalidates
content, including already installed dependencies. It requires access to the
index and archive or source repository; this is not an offline mode. A moved
archive URL is allowed when it serves identical bytes. Git reproducibility
depends on the origin retaining the locked commit. Atomic installation and
cross-process cache coordination are not introduced by this stage.

## Reproduction

Run from the repository root with the existing Rust dependencies available:

```sh
cargo test --offline --test registry_locked --test publish_registry
cargo test --offline --bin ccgo
python3 scripts/registry-index-assessment/provenance.py --verify
git diff --check
```

`tests/registry_locked.rs` exercises the real CLI using temporary local Git
repositories and ZIP archives. It covers checksum and locator changes, missing
anchors and versions, yank, registry ambiguity and identity, legacy locks,
workspace failures, moved and deleted tags, dirty Git caches, unsupported archive
formats, and byte-for-byte lockfile preservation. No external registry is needed.

## Relationship to the architecture assessment

The assessment under `docs/architecture/registry-index-assessment/` describes the
implementation before these fixes. Its original measurements and fault results
remain historical evidence. Assessed source files are frozen under
`evidence/assessed-source/`; `evidence/provenance.json` records their hashes and
original paths. The provenance verifier validates those snapshots rather than
requiring today's implementation to retain the assessed bugs.

Historical fault scripts characterize the baseline and may intentionally fail
against the fixed CLI. Use the integration tests above for current invariants.

Publication serialization and interrupted-publication recovery are described in
[registry-publication-recovery.md](registry-publication-recovery.md). Transport
improvements remain subsequent work. Registry synchronization and concurrent
cache access are described in [registry-fetch-cache.md](registry-fetch-cache.md).
These changes do not claim a distributed transaction or migrate existing indexes
or clients.
