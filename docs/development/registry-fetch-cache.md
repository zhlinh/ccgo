# Registry synchronization and concurrent fetches

`ccgo fetch` owns a `RegistrySession` for the invocation. Workspace members share
that session, including `--locked` retrieval. Each cache path and configured URL
is synchronized once after successful synchronization. A failed attempt is not
memoized. A new invocation synchronizes again; there is no TTL or process-global
success cache. Dependencies that reuse an existing installation still follow the
existing fast path and need not consult an index.

## Cache protocol

1. Acquire a shared home lifecycle lock, then an exclusive lock for the named
   registry. Lock files are in a sibling `.ccgo-registry-locks/<home hash>/`
   directory, outside `CCGO_HOME`. Canonical home paths make symlink aliases use
   the same locks. Existing origin verification remains mandatory.
2. For a cold cache, clone into a temporary directory beside the destination,
   then rename the complete clone into place. Failure does not expose a partial
   clone. Existing caches fetch and reset while holding the registry lock.
3. Capture HEAD and create a unique `refs/ccgo/snapshots/<token>` reference before
   releasing the registry lock. This reference retains the commit even if another
   process resets the cache to unrelated history and Git prunes old objects.
4. Read package JSON with `git show <commit>:<package path>`. The session always
   reads its captured commit; mutable checkout files are not its data source.
   A missing path returns no match; malformed JSON and object access errors fail.
5. Keep the shared home lock for the snapshot lifetime. Drop removes the temporary
   ref under the registry lock, then releases the home lock. Artifact downloads
   do not hold the exclusive registry lock, so another fetch can sync concurrently.

`fetch --clean-cache` takes the exclusive home lock before deletion. It waits for
live registry readers and writers, then removes the cache. Lock files are never
removed while another process might use them. The low-level live-cache `lookup`
also takes these locks so it cannot read during a checkout reset.

A configured URL change under an existing name fails the origin check. The
session key includes both path and URL, so a workspace cannot silently reuse the
old snapshot for a new source. Names must be single safe path components.

## Evidence and reproduction

```sh
cargo clippy --offline --bin ccgo --tests -- -D warnings
cargo test --offline --bin ccgo registry::cache::tests
cargo test --offline --test registry_locked
```

`tests/support/registry_fetch_session.rs` runs actual CLI processes against local
Git repositories and ZIPs. A Git wrapper counts clone/fetch calls and pauses
specific operations for interleavings. These barriers test:

- Two dependencies: one cold clone and zero redundant fetches; the following
  locked invocation performs exactly one fetch.
- Two workspace members: one shared clone; the following locked workspace
  invocation performs exactly one fetch.
- Concurrent first fetches: one complete clone followed by one serialized update.
- A paused package read while another process fetches a yank: the original
  invocation reads its pinned version; the new invocation rejects the yanked
  version. The writer finishes before the paused reader resumes.
- A paused reader and `--clean-cache`: cleanup waits for the snapshot to finish,
  then successfully rebuilds the cache.

Unit tests cover retry after clone failure, rejection of an origin change, snapshot
stability after forced history replacement and `git gc --prune=now`, and reference
cleanup on normal drop. The subprocess/barrier tests are Unix-only; Windows uses
fs2 locks and the same production protocol but has not been exercised by these
shell-wrapper tests. These are operation-count and correctness tests, not a
wall-clock performance claim.

## Boundaries

The snapshot is per registry, not a transaction spanning multiple registries.
Two concurrent commands each perform their own synchronization; they serialize
cache mutation but do not share a freshness TTL. Direct Git source checkout
caches, concurrent writes to the same project's installation directory and
external tools deleting cache directories are outside this registry-index
protocol. Cache cleanup coordinates registry users, not every other CCGO cache
consumer.

OS locks release when a process exits. A forced termination can leave a snapshot
ref, its token file or a clone staging directory; those do not block subsequent
fetches. A normal drop removes its ref and token. `fetch --clean-cache` removes the
registry cache and retained Git refs; stale token files in the external lock
folder may be removed during maintenance when no fetch process is running.
Blocking synchronization waits for the current Git operation; no timeout or stale
network fallback is introduced.
