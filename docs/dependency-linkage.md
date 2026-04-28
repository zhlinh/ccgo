# Dependency Linkage

Two orthogonal axes determine how a dependency ends up in your build product:

| Axis | CCGO.toml field | Values |
|---|---|---|
| What **you** produce | `[build].link_type` | `static`, `shared`, `both` |
| How a **dep** relates to you | `[[dependencies]].linkage` (with `[build].default_dep_linkage` as project-wide default) | `shared-external`, `static-embedded`, `static-external` |

## Linkage values

* **`shared-external`** — the dep stays as its own `.so`/`.dylib`/`.dll` and your binary records a runtime dependency (`DT_NEEDED` on ELF). Default for shared consumers when the dep can produce a shared artifact. Best for app-internal sharing across multiple consumers — eliminates the per-consumer copy of the dep that fat archives produce.
* **`static-embedded`** — the dep's `.a` is archived into your binary at link time. Your binary becomes self-contained; multiple consumers each carry their own copy of the dep code. Default fallback when the dep only ships a `.a`.
* **`static-external`** — the dep stays as a separate `.a` and your `.a` records the dependency without merging. Only valid for static consumers; the final executable's linker reconciles symbols at exe link time. The "thin chain" model.
* `shared-embedded` — **does not exist**. Trying to set it produces a parse error pointing at the two valid alternatives. A `.so` cannot be archived into another `.so`.

## Decision matrix

| Consumer | Dep provides | Hint | Result |
|---|---|---|---|
| `static` | any | (any) | `static-external` (or `shared-external` when only `.so` available) |
| `shared` | only `.a` | (any) | `static-embedded` (forced; `shared-external` hint is an error) |
| `shared` | only `.so` | (any) | `shared-external` (forced; `static-embedded` hint is an error) |
| `shared` | both / source | absent | `shared-external` |
| `shared` | both / source | `shared-external` | `shared-external` |
| `shared` | both / source | `static-embedded` | `static-embedded` |
| `shared` | any | `static-external` | **error** — leaves unresolved external static refs in the `.so` |

## When to override

Most projects don't need to set `linkage` at all. The default
(shared-external for shared consumers when the dep ships a `.so`) avoids
bloat across multiple sibling consumers without surprises.

Set `linkage = "static-embedded"` per-dep when:
* The dep is small and you want self-containment (one fewer `.so` to ship).
* You're publishing an SDK to external developers via Maven/CocoaPods who
  won't have a way to install transitive deps independently.
* The dep only ships `.a` and you want to silence the auto-fallback notice
  in build logs.

## Build-time logging

ccgo emits a `STATUS` line per dep at CMake configure time:

```
[ccgo] stdcomm:    linkage=shared-external (DT_NEEDED to dep.so)
[ccgo] tinyhelper: linkage=static-embedded (.a archived into target)
[ccgo] zstd:       linkage=static-embedded (auto, no .so available)
```

`(auto, ...)` indicates the resolution came from a fallback rather than an
explicit `linkage` field; `(auto, no .so available)` specifically means the
dep doesn't ship a shared form, so embedding is the only choice.

## Example

```toml
[package]
name = "logcomm"
version = "1.0.0"

[build]
link_type = "shared"                 # I produce a .so
default_dep_linkage = "shared-external"  # default for my deps

[[dependencies]]
name = "stdcomm"
version = "25.2.9519653"
# Default linkage applies → shared-external (libstdcomm.so stays separate)

[[dependencies]]
name = "tinyhelper"
version = "0.3.0"
linkage = "static-embedded"          # explicit: archive into liblogcomm.so
```

Reads as: "I'm a shared library. By default my deps are external. tinyhelper
is the exception — bake it into me." Three semantics, three settings, no
ambiguity.

## Source-only dependencies

When a dep ships only source code (the `.ccgo/deps/<name>/` dir contains
`src/` and `CCGO.toml` but no `lib/<platform>/` artifacts for the platform
being built), `ccgo build` automatically recurses: it spawns `ccgo build
<platform> --build-as <derived>` inside the dep before it resolves linkage,
then symlinks `dep/lib/<platform>/` to the new build output so the
consumer's `FindCCGODependencies.cmake` walks find the artifacts.

The `--build-as` value is derived from the consumer's resolved hint for
that dep:

| Consumer's hint for the dep | Recursive `--build-as` |
|---|---|
| `shared-external` (default) | `shared` |
| `static-embedded` / `static-external` | `static` |
| (no hint) | `both` |

A dep's own `[build].link_type` declaration **does not** dictate what gets
produced during a recursive materialize — the consumer's needs do. If you
set `--linkage stdcomm=static-embedded` on a build that has a source-only
`stdcomm`, you get a `.a`. If you flip to `--linkage stdcomm=shared-external`,
ccgo rebuilds `stdcomm` as a `.so` on the next build.

### Caching

The materialize step persists a per-platform, per-`--build-as` fingerprint at
`.ccgo/deps/<name>/.ccgo_materialize_<platform>_<build_as>.fingerprint`.
The fingerprint is a SHA-256 over (sorted source-tree mtimes + sizes + paths)
+ the `CCGO.toml` content + the requested `--build-as`. Subsequent builds
skip the recursive spawn when the fingerprint matches AND `lib/<platform>/`
still has artifacts. Splitting the sidecar by `build_as` prevents two
parallel builds of the same path-source dep from racing on a shared sidecar
when one wants `--build-as shared` and the other wants `--build-as static`.

Behavior matrix:

| State | Action |
|---|---|
| no `lib/<platform>/`, no fingerprint | Spawn build, write fingerprint |
| no `lib/<platform>/`, fingerprint exists | Spawn build (lib was deleted) |
| `lib/<platform>/` exists, fingerprint matches | Skip (cache hit) |
| `lib/<platform>/` exists, fingerprint mismatches | Spawn build (source changed) |
| `lib/<platform>/` exists, no fingerprint | Trust prebuilt, write fingerprint |

The "trust prebuilt" path matters for fixtures and projects that ship
hand-curated `lib/<platform>/` layouts (e.g. xcframework symlinks). Those
get a fingerprint stamped on the first invocation and then participate in
normal source-change invalidation from then on.

### What gets propagated to the recursive build

The recursive `ccgo build` call inherits:

* `--release` (from the parent's release flag)
* `--arch <csv>` (lowercased; matches the parent's `--arch`)
* `--build-as <variant>` (derived from hint as above)

It does **not** inherit:

* `--linkage` — the dep's own `[[dependencies]]` are decided by its own
  CCGO.toml. A consumer's per-dep linkage hint applies only to that
  consumer's relationship with the dep, not to the dep's relationship with
  its own deps.

### Failure mode

If the recursive build fails, the parent ccgo bails with the dep name and
a reproduction command:

```
recursive `ccgo build` for source-only dep 'stdcomm' (--build-as shared) failed
with exit code Some(1). The dep at .ccgo/deps/stdcomm could not be compiled —
check its CCGO.toml and try `ccgo build macos --build-as shared` inside that
directory to reproduce.
```

### Source vs binary precedence in the consumer's CMake template

When a path-source dep ships both `src/` and pre-built `lib/<platform>/`
artifacts (the latter being what the bridge populates after a successful
materialize spawn), the consumer's CMake template now skips the inline
source compilation if the resolved linkage is `shared-external` and the
bridge has placed a usable shared library at the expected depth. Concretely:

* `consumer/.ccgo/deps/<name>/lib/<platform>/shared/<name>.xcframework/`
  exists (Apple) or `lib/<platform>/shared/<arch>/lib<name>.so` exists
  (Android/OHOS/Linux) → the dep's `src/` is skipped in the
  `<consumer>-deps` aggregate. The consumer's main shared target picks up
  `libleaf.dylib` / `libleaf.so` via DT_NEEDED / LC_LOAD_DYLIB instead.
* `static-embedded` linkage continues to compile the dep's source into the
  consumer's archive (or link against the pre-built `.a`), as expected.

Falling back to inline source compilation only happens when no shared
artifacts are present and the linkage hint either asks for or ends up at
`static-embedded`. This is the contract the linkage matrix expects.

### Cross-platform bridge

The bridge step uses NTFS directory junctions (`mklink /J`) on Windows in
place of Unix `symlink`. Junctions don't require admin or Developer Mode
and work on the same volume as the dep — which the `cmake_build/` tree
always is. They behave identically to symlinks for the `EXISTS` /
`file(GLOB ...)` walks that `FindCCGODependencies.cmake` performs.

## See also

- [`dependency-resolution.md`](dependency-resolution.md) — how ccgo finds and
  fetches deps before linkage decisions get made.
- The Rust source: `src/build/linkage.rs` — pure decision matrix and
  filesystem scanner. The decision table above is mirrored exactly in the
  unit tests.
