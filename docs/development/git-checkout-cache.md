# Git checkout cache generations

Fetch uses `src/registry/git_cache.rs` for both direct Git dependencies and the
Git fallback of registry dependencies. The cache key contains the package name,
source identity and branch/tag for an unpinned request, or the locked commit for
a pinned request. Pinned identity does not depend on a mutable branch/tag name.
Relative filesystem Git sources are normalized to absolute identities.

## Publication and concurrency

A writer acquires a shared CCGO_HOME lifecycle lock and an exclusive lock for its
cache key. The lock files share the external lock directory used by registry
indexes, with a distinct `git-` prefix. Readers and writers cannot race explicit
`fetch --clean-cache`. Fetch holds its home lock across dependency resolution,
installation and lockfile saving; the dependency resolver also takes a home lock
when used independently.

A cold or forced request clones into a temporary directory under
`CCGO_HOME/git/checkouts`. A locked request clones without selecting a mutable
branch, then checks out the locked commit. Before publication, the checkout must
have the expected origin and, for pinned requests, the expected HEAD and a clean
Git status. Errors do not replace the old pointer or installation.

A complete checkout moves into a unique directory:

```text
git/checkouts/<key>-<generation>/
git/checkouts/.ccgo-pointers/<key>.json
```

The pointer contains only the generation basename. It is validated as a safe path
component and written through a synced temporary file with atomic replacement.
The generation is ready before the pointer changes. The installation links or
copies that generation. When reading transitive metadata, the resolver first
uses the project's installed tree if its lock, source and branch match and its
checkout passes validation. Otherwise it follows the cache pointer. Refreshing
another project's pointer cannot change the original project's locked graph. The writer lock remains held through
installation/copying. Returned source paths are absolute, including when
`CCGO_HOME` is relative.

A future force fetch creates another generation; it never deletes, checks out or
resets a published tree. Existing project symlinks keep pointing to their old
generation. This removes the need to keep a cross-process read lock for the entire
lifetime of every project symlink.

## Compatibility and maintenance

Earlier `git/checkouts/<md5>` and flat `registry/<name>-<short md5>` entries remain
readable when their identity is unambiguous. An unpinned request with an explicit
branch/tag does not reuse an old entry whose key omitted that selector. Pinned
legacy entries must pass origin, HEAD and dirty-tree validation; an invalid entry
requires `--force` and is preserved rather than reset in place. Existing lockfile
and manifest formats do not change.

Published generations are retained because other projects may still link them.
`fetch --clean-cache` explicitly removes the whole cache and can invalidate those
older installations, as before. There is no automatic generation garbage
collector. A crash after the generation rename but before pointer publication can
leave an unused complete generation, which cache cleanup removes. Concurrent
external edits to a shared checkout are not prevented; pinned retrieval checks
tracked/untracked Git status, following the existing dirty-check contract.

The older `vendor` command still owns its separate flat-cache write path. This
protocol covers fetch's checkouts and resolver reads; it does not claim to
serialize older CLI versions or every cache consumer.

## Project installation lock

Fetch holds an exclusive `.ccgo/fetch.lock` in each canonical project directory
before reading its configuration or lockfile and until installation and lockfile
saving finish. This identity is independent of CCGO_HOME and is shared by project
path aliases. The persistent lock file must not be removed while a fetch is live:
removing it could let another process lock a different inode. The current clean
command does not remove this directory. OS locks are released on exit or failure.

The lock order is home lifecycle lock, project lock, then individual cache locks.
Cache cleaning happens before taking these locks. Workspace fetch holds only one
member project lock at a time, using the same protocol as standalone fetch.
A waiting fetch reads the lockfile only after the preceding writer has finished.
Different projects can still fetch concurrently.

This serializes cooperating fetch processes; it does not make installation plus
lockfile saving a crash-atomic transaction, protect against manual deletion or
coordinate older clients, vendor, or external project writers. The guarantee
requires a filesystem supporting the fs2 OS lock primitives.

## Executable evidence

```sh
cargo clippy --offline --bin ccgo --tests -- -D warnings
cargo test --offline --bin ccgo
cargo test --offline --test registry_locked
```

The Unix subprocess tests in `tests/support/registry_fetch_session.rs` cover:

- Concurrent first fetches of one source/branch produce one clone and a complete
  generation usable by both projects.
- Different branches publish independently and retain different package bytes.
- Failed force cloning preserves the installed tree, link and lockfile; retry
  succeeds while the old generation remains readable.
- Transitive dependency metadata is found through the new generation pointer,
  while an already locked project keeps its own graph after another project
  refreshes to a different commit.
- An old checkout remains usable and is not deleted by a force fetch.
- Relative Git source paths and a relative CCGO_HOME still install correctly.
- A locked tag can disappear and a force refetch still retrieves its commit.
- A second fetch of the same project waits before reading its lockfile, including
  across independent cache homes, a symlink alias, and workspace/member commands.
- A failed fetch releases the project lock so a corrected retry can finish.

The existing registry Git regression also checks moved/deleted tags, dirty locked
checkouts and byte-for-byte preservation of CCGO.lock. Tests use local Git and ZIP
fixtures. The shell-based interleaving tests have not been run on Windows; the
production code uses the same fs2 locking and tempfile replacement primitives on
supported platforms.
