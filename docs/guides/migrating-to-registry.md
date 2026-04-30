# Migrating to a Registry-Based Dependency Setup

This guide walks an organization that maintains multiple library projects
sharing a single internal package collection through replacing
`git`+`branch` and `zip`+`local-path` dependency declarations with
`version` + `registry` resolution backed by a shared index repository.

The running example is a hypothetical "comm-group" organization: a
collection of internal C++ libraries (`stdcomm`, `foundrycomm`, `logcomm`,
several leaf consumers) that today depend on each other through pinned
Git branches. The same shape applies to any setup where a dozen sibling
projects need to coordinate releases without spinning up a hosted
package server.

## What you get

**Before**: every consumer's `CCGO.toml` carries the Git URL and the
"distribution branch" of every dep, so a new release of `stdcomm`
triggers a coordinated branch update across every downstream
`CCGO.toml`. Branches like `dist-v1.0.0` move under your feet whenever
the upstream tagger force-updates them.

```toml
[[dependencies]]
name = "stdcomm"
git = "git@git.example.com:org/stdcomm.git"
branch = "dist-v1.0.0"
version = "1.0.0"

[[dependencies]]
name = "logcomm"
git = "git@git.example.com:org/logcomm.git"
branch = "dist-v1.0.0"
version = "1.0.0"
```

**After**: every consumer points at one shared index repo, then names
each dep by exact version. The index records the artifact archive URL
and a SHA-256 checksum per version.

```toml
[registries]
org = "git@git.example.com:org/ccgo-index.git"

[[dependencies]]
name = "stdcomm"
version = "25.2.9519653"
registry = "org"

[[dependencies]]
name = "logcomm"
version = "25.2.9519653"
registry = "org"
```

* Pinned by exact version. No moving `dist-v...` branches.
* One shared index lists every published version with its checksum.
* Consumers download a single archive zip from a CDN. No git history
  transfer, no post-clone build step on the consumer side.
* Adding a new sibling library requires zero changes to existing
  consumers — only a new `ccgo publish index` run from the new project.

## Step 1 — Bootstrap the index repo

Create one Git repository to hold the package index. It starts empty
and gets populated by every project's `ccgo publish index` runs.

```bash
# On your Git host: create a fresh repo, e.g. org/ccgo-index.git
# Locally, give it one initial commit so `git clone` works:
git clone git@git.example.com:org/ccgo-index.git
cd ccgo-index

cat > index.json <<'EOF'
{
  "name": "org-index",
  "description": "Internal package index for org libraries",
  "version": "1",
  "package_count": 0
}
EOF

git add index.json
git commit -m "init: empty index"
git push origin master
```

That's it for bootstrap. The directory layout (one JSON per package,
sharded by name) is created lazily by each `ccgo publish index` call.

## Step 2 — Per-project CI workflow

For every library in the collection, replace the existing release step
(probably "tag, push, branch update") with a build → package → upload →
publish-index sequence. The shape:

```bash
# In the library's repo:

# 1. Build all platforms.
ccgo build all --release

# 2. Package the build outputs into a zip whose layout matches what
#    `ccgo fetch` extracts into .ccgo/deps/<name>/.
ccgo package --release
# Produces: target/release/<plat>/<NAME>_CCGO_PACKAGE-<version>.zip

# 3. Upload the zip to your CDN / artifactory.
#    (Plug in your existing release upload script. The destination URL
#    must match the template you'll pass to `ccgo publish index`.)
your-upload-script target/release/macos/STDCOMM_CCGO_PACKAGE-25.2.9519653.zip \
  https://artifacts.example.com/stdcomm/

# 4. Append the new version to the shared index, with archive URL +
#    SHA-256 baked into the VersionEntry.
ccgo publish index \
  --index-repo git@git.example.com:org/ccgo-index.git \
  --index-name org-index \
  --archive-url-template "https://artifacts.example.com/{name}/{name}_CCGO_PACKAGE-{version}.zip" \
  --checksum \
  --index-push
```

The `{name}`, `{version}`, and `{tag}` placeholders are substituted
per-version. Run this same command from every library in the collection;
the index repo accumulates entries naturally.

## Step 3 — Migrate consumer CCGO.toml

In each consumer project, replace existing `git`/`branch` (or
`zip`/`local-path`) dep declarations with `version` + `registry`:

```diff
+[registries]
+org = "git@git.example.com:org/ccgo-index.git"
+
 [[dependencies]]
 name = "stdcomm"
-git = "git@git.example.com:org/stdcomm.git"
-branch = "dist-v1.0.0"
-version = "1.0.0"
+version = "25.2.9519653"
+registry = "org"
```

Then run `ccgo fetch` to confirm the dep resolves through the index. The
generated `CCGO.lock` will record `source = "registry+git@..."` plus the
checksum so the next `ccgo fetch --locked` reproduces the exact bytes.

If a single consumer pulls deps from more than one collection, declare
both registries in `[registries]` and let each `[[dependencies]]` entry
pin itself with its own `registry = "..."`. Per-dep selectors override
TOML iteration order.

## Step 4 — Roll out (deepest dep first)

Migrate libraries in dependency-graph order, starting with the leaves.
That way, by the time you migrate a library that depends on
`stdcomm`, `stdcomm` is already publishable through the index — the
two changes never have to land in the same PR.

A typical rollout:

1. **`stdcomm`** (no internal deps). Adopt the publish-to-index CI step.
   Existing consumers keep using `git`/`branch` for now — `stdcomm`'s
   own CCGO.toml has no consumer-facing changes.
2. **`foundrycomm`, `logcomm`** (depend on `stdcomm`). First, migrate
   their consumer side: their CCGO.toml now references `stdcomm` by
   `version` + `registry`. Confirm with `ccgo fetch` + `ccgo build`.
   Then adopt the publish-to-index CI step so they themselves become
   resolvable through the index.
3. **Leaf consumers** (depend on `foundrycomm` / `logcomm`). Migrate
   their CCGO.toml to point at the index. By this point, every dep
   they need is in the index.

You don't have to do the whole graph in one pass. Mixing old-style
`git`/`branch` and new-style `version`/`registry` in the same CCGO.toml
is supported — they're independent branches in the
[resolution priority list](../dependency-resolution.md#resolution-priority).

## Rollback

Legacy `git`/`branch` and `zip` declarations keep working unchanged.
The registry tier is opt-in and only fires when (a) `[registries]` is
non-empty AND (b) a dependency has no explicit `path`/`git`/`zip` source.
If a project hits trouble — index repo unreachable, archive URL down,
checksum mismatch — reverting that project's CCGO.toml back to the old
`git = "..." / branch = "..."` form is a one-line revert. No
infrastructure unwind required.

You can also migrate one dependency at a time within a single CCGO.toml.
Keep most deps on `git` / `branch`, switch the riskiest one to
`version` + `registry`, and confirm that branch in CI before moving the
next one over.

## See also

* [Package Registry feature reference](../features/registry.md) — index
  JSON schema, CLI, the `--archive-url-template` substitution rules.
* [Dependency Resolution](../dependency-resolution.md) — full resolution
  priority list across all source kinds.
* [Configuration Guide](../getting-started/configuration.md#registry-dependencies)
  — the `[registries]` and `[[dependencies]].registry` reference.
