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

## See also

- [`dependency-resolution.md`](dependency-resolution.md) — how ccgo finds and
  fetches deps before linkage decisions get made.
- The Rust source: `src/build/linkage.rs` — pure decision matrix and
  filesystem scanner. The decision table above is mirrored exactly in the
  unit tests.
