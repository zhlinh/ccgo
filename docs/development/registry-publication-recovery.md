# Registry publication concurrency and recovery

This stage retains the existing index schema and adds protection around the
publish/edit/commit/push workflow. It applies to `ccgo publish package`; the
legacy `index` alias uses the same implementation.

## Local writers

Publication and yank take an OS file lock before touching their shared index
working directory. A competing operation exits with a retry instruction; it
does not wait while holding Git state. Locks are released when the process exits,
including abnormal termination. Lock files remain outside the index repository
to avoid inode races and accidental commits. Registry aliases and different
CCGO_HOME directories have separate clones and coordinate through Git pushes.

Existing clones verify their origin before synchronizing. A clone failure is an
error, not a reason to initialize an unrelated local index. The remote repository
must exist, but it may be empty for its first publication.

## Concurrent remote updates

Pushes never force-update the remote branch. After rejection, the writer fetches
and reconciles changes, with at most three push attempts in one invocation.
The merge uses the common ancestor, local intent and remote package records:

- Added versions from different writers are retained together.
- Identical concurrent version records may converge successfully. Sequential
  duplicate publication remains an error after the original operation finishes.
- Different records for the same new version are rejected. Changes to existing
  immutable version fields and conflicting yank changes are rejected.
- Compatible changes to different versions, including yank and publication, can
  merge. Conflicting package metadata requires manual resolution.
- Other writers' package files and fields are preserved; package counts are
  recomputed without inspecting `.git` recovery records.

Successful reconciliation makes a normal merge commit retaining both histories.
Unrelated dirty files prevent synchronization. Authentication failures, hook
rejections and repeated contention return errors and retain local work.

## Interrupted publication

The publishing clone records its pending intent at `.git/ccgo-publication.json`.
The record contains package/source identity, the original version metadata, and
whether artifact upload completed. Updates use a temporary file and atomic
replacement. The record is removed after publication succeeds.

Retry the original command with the same source, version and artifact bytes.
Recovery preserves the original timestamp and version record, commits a pending
index edit if necessary, and retries the push. A confirmed upload is not repeated.
The record blocks a different publication or yank in that clone until the
pending operation is completed. It is local to that clone, not a distributed
transaction log.

For an upload whose outcome was uncertain, artifact handling is create-only:

- Maven reads each ZIP/POM/sidecar before uploading. Identical existing bytes
  are reused; different bytes fail. Creation sends `If-None-Match: *` and reads
  the object back for verification. The hosted service must honor conditional
  creation or enforce immutable release coordinates to prevent a race between
  independent clients. A server ignoring both protections cannot provide this
  guarantee through a client-side check alone.
- GitHub downloads a matching asset and compares bytes when available. Upload
  never uses `--clobber`. If creation loses a race, a second download can verify
  identical bytes. A failed lookup never authorizes overwriting an existing asset.

These rules are scoped to registry package publication. Other Apple/Doc artifact
workflows retain their existing upload policy.

## Recovery boundaries

There is no atomic transaction between external artifact storage and Git. An
uploaded artifact can remain unindexed; it is not deleted automatically. Stored
upload confirmation cannot prevent an administrator from later deleting or
replacing that remote object. Source-only index entries still use Git tags;
existing consumer locks pin commits, but index publication does not make tags
immutable on the source server.

Unrelated local edits, malformed/partially written JSON, conflicting same-version
history, or incompatible package metadata stop automatic recovery. Preserve the
clone and recovery record before manual repair. If intentionally abandoning an
operation, inspect remote artifacts and pending Git changes before moving the
recovery record aside; removing a record does not undo those effects.

Yank and unyank use `.git/ccgo-yank.json` to record the target version, intended
flag/reason, repository, push requirement, and hashes of the package before and
after the edit. The package edit and receipt use atomic file replacement. Retry
the original command after fixing a commit hook or remote failure. A pending
remote push cannot be completed by omitting `--push` on the retry. Publish and
yank block each other while either receipt is pending.

Recovery accepts the recorded package snapshot; unrelated dirty files or package
edits are rejected. If a semantic merge was prepared but its commit failed, the
package is checked against the expected reconciliation of both Git parents before
committing. Thus a new version received from another publisher survives both the
merge and a subsequent retry. Same-version conflicting history still fails.
Completed operations remove their receipt; repeating a completed yank/unyank
continues to report the existing no-op error.

Malformed JSON, unknown local modifications and partially repaired Git merges
can still require manual repair. Fetch cache synchronization and atomic dependency
installation are separate concerns and are not serialized by this writer lock.

## Verification

```sh
cargo test --offline --test publish_registry --test registry_locked
cargo test --offline --bin ccgo
cargo clippy --offline --bin ccgo --tests
python3 scripts/registry-index-assessment/provenance.py --verify
git diff --check
```

The publication tests use real temporary Git remotes and a loopback HTTP storage
server; GitHub is represented by a local `gh` test double. Barrier-controlled
subprocesses force push races and hold an upload open to check local exclusion.
Tests check version retention, conflicting duplicates, rejected pushes, upload
followed by commit failure, upload reuse, and refusal to overwrite stored bytes.
Yank tests also cover commit failures in both directions, rejected pushes followed
by another publisher's new version, a second failure while committing that merge,
changed operation parameters, and unrelated dirty file/package edits.
The HTTP tests need permission to listen on localhost. No real Maven/GitHub
service, repository authorization policy, or network filesystem lock semantics
has been validated. The original architecture assessment remains frozen baseline
evidence; these tests verify the new implementation.
