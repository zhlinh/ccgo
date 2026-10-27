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

## Prepared installation replacement

Archive extraction and directory linking/copying prepare a candidate under a
temporary `.ccgo-install-*` directory beside the destination. Registry downloads
are checksum-verified before extraction. Download, validation, extraction and
copy errors leave the old installation untouched. This applies to registry and
direct archives, Git, local paths, the packages cache, and vendor retrieval.

Once preparation succeeds, the old directory, file or symlink moves to a backup
inside the staging directory, then the candidate moves into place. If the second
rename fails, the old installation is restored. If restoration also fails, the
staging directory is retained and the error names the recovery location; cleanup
must not destroy the backup. Successful replacement removes the old backup and
stale files. A dangling old symlink is moved as a symlink without following it.

These are same-filesystem renames under the project lock, with a brief interval
between them when the destination is absent. External readers do not share the
lock. Replacement is per dependency, not a transaction for the dependency batch and
CCGO.lock; subsequent patching and lockfile-save failures are outside this
preparation guarantee. The old copy and candidate can temporarily require roughly
twice the installation disk space.

### Recovery after process interruption

Each new staging directory contains a schema-1 `transaction.json` identifying a
single destination basename. The receipt is synced and atomically published
before preparation; no old installation moves until preparation succeeds. Once
replacement starts, automatic temporary-directory destruction is disabled.

After acquiring the project lock, standalone and workspace member fetch recover
recognized transactions before configuration, lockfile or installed-tree reads:

- If the destination is absent and `previous` exists, restore `previous`.
- If the destination exists, retain it: it is either the untouched old tree or
  the complete published replacement.
- Remove the recognized staging directory after recovery succeeds. A failed
  recovery retains its state and stops fetch, naming the recovery location.

Recovery is repeatable after interruption, including an interruption immediately
after the backup has been restored. Receipt schema and destination components
are validated before recovery mutates anything. Invalid recognized records stop
fetch for inspection. Directories without receipts, including earlier temporary
directories and interruption before receipt publication, are left untouched;
they cannot contain a backup created by this protocol.

This handles process termination on a functioning filesystem, not sudden power
loss: directory entries and extracted file contents are not recursively fsynced.
It assumes cooperating writers use the project lock. A newly published dependency
can survive termination before its new lockfile is saved. Recovery does not roll
back a successfully published dependency based on lockfile state, and cannot make
the full batch, patch application and lockfile save atomic.

### Atomic lockfile publication

`Lockfile::save_to` writes complete content to a `.ccgo-lock-*` temporary file in
the destination directory, syncs the file, and atomically replaces the target.
Readers see either the previous complete lockfile or the complete replacement.
An interrupted first write leaves the target absent until publication. Failed
publication leaves the original target intact; a subsequent save can retry.
All callers of the lockfile save API use this protocol.

Existing file permissions are retained. On Unix a new file uses normal 0666
creation permissions restricted by the process umask. A live symlink is resolved
and its target replaced, preserving the link; a dangling symlink fails without
mutation. Replacement changes the target inode: hard-link identity, ownership,
ACLs and extended attributes are not guaranteed to survive. Filesystem directory
write permission is required for atomic replacement.

This guarantees publication atomicity on filesystems supporting atomic rename;
it does not serialize callers outside fetch or merge concurrent lockfile changes.
It also does not commit installations and the lockfile in one transaction.
Power-loss durability is not claimed because the parent directory is not fsynced.
Process termination before publication can leave an unused temporary file; later
saves ignore it. Unit subprocess tests in `src/lockfile_write.rs` exit before and
after publication, verify exact destination bytes, and retry. They also cover
relative filenames, failed publication, Unix file modes and live symlinks. The
existing lockfile roundtrip and locked-fetch regressions verify content format
and byte-for-byte preservation of unchanged locks.

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
- Missing downloads and partially extracted invalid archives preserve the old
  package and lockfile; a corrected registry or direct archive can then retry.

Unit tests in `src/commands/fetch_install.rs` also check partial preparation,
publication rollback, dangling symlinks and removal of stale installed files.
Child test processes exit without destructors after preparation, backup rename,
and publication; the parent checks recovery and a second recovery pass. Crash
checkpoints exist only in the unit-test binary. CLI integration tests simulate an
interrupted backup and verify recovery before installed-tree reuse in standalone
and workspace commands, even after the source archive has been removed.

The existing registry Git regression also checks moved/deleted tags, dirty locked
checkouts and byte-for-byte preservation of CCGO.lock. Tests use local Git and ZIP
fixtures. The shell-based interleaving tests have not been run on Windows; the
production code uses the same fs2 locking and tempfile replacement primitives on
supported platforms.
