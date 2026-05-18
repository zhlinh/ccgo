# Build Profiles

Named build profiles let you define reusable configuration slices and select them with a single flag instead of repeating long command-line arguments.

## Overview

A profile is a named set of build settings — release mode, link type, features, CMake flags, dependency linkage, and output package name — stored in `CCGO.toml` under `[profile.<name>]`. When you pass `--profile <name>` to `ccgo build`, all profile settings are applied on top of the global config.

```bash
ccgo build android --profile sanitize
ccgo build ios --profile release-shared
ccgo build macos --profile fat-static
```

## Built-in Profiles

Two profiles are always available without any declaration:

| Name | Effect |
|------|--------|
| `debug` | `release = false` — debug symbols, no optimization |
| `release` | `release = true` — same as `--release` |

```bash
ccgo build android --profile release   # equivalent to: ccgo build android --release
ccgo build android --profile debug     # explicit debug build
```

## Defining a Custom Profile

Add a `[profile.<name>]` table to your `CCGO.toml`:

```toml
[profile.sanitize]
release = false
link_type = "both"

[profile.sanitize.cmake]
c_flags  = ["-fsanitize=address", "-fno-omit-frame-pointer"]
cpp_flags = ["-fsanitize=address", "-fno-omit-frame-pointer"]
```

Then build with it:

```bash
ccgo build macos --profile sanitize
```

## Profile Fields

### Scalar Fields

| Field | Type | Description | Default |
|-------|------|-------------|---------|
| `inherits` | string | Parent profile to inherit from | — |
| `name` | string | Override the output package name | package name |
| `release` | bool | `true` = release, `false` = debug | — |
| `link_type` | string | `"static"` \| `"shared"` \| `"both"` | — |
| `jobs` | integer | Parallel build jobs | auto-detected |

### [profile.\<name\>.cmake]

Extra CMake flags applied to all platforms when this profile is active.

| Field | Type | Description |
|-------|------|-------------|
| `merge` | string | `"replace"` (default) or `"extend"` — see [Merge Strategy](#merge-strategy) |
| `arguments` | array | Raw arguments passed verbatim to cmake configure |
| `c_flags` | array | Flags appended to `CMAKE_C_FLAGS` |
| `cpp_flags` | array | Flags appended to `CMAKE_CXX_FLAGS` |

### [profile.\<name\>.features]

Features to enable when this profile is active.

| Field | Type | Description |
|-------|------|-------------|
| `merge` | string | `"replace"` (default) or `"extend"` |
| `list` | array | Feature names to enable |

### [profile.\<name\>.dep_linkage]

Default dependency linkage when this profile is active.

| Field | Type | Description |
|-------|------|-------------|
| `default` | string | Linkage for all build types |
| `on_shared` | string | Override when consumer builds a shared library |
| `on_static` | string | Override when consumer builds a static library |

**Linkage values:** `"shared-external"` \| `"static-embedded"` \| `"static-external"`

### Per-Platform Overrides

Override CMake flags or dep_linkage for a specific platform within a profile:

```toml
[profile.sanitize.platforms.android.build.cmake]
merge     = "extend"
cpp_flags = ["-fsanitize=address"]

[profile.sanitize.platforms.android.build.dep_linkage]
default = "static-embedded"
```

Supported platforms: `android`, `ios`, `macos`, `windows`, `linux`, `ohos`

## Merge Strategy

List fields (`cmake.arguments`, `cmake.c_flags`, `cmake.cpp_flags`, `features.list`) carry a `merge` field that controls how they combine with an inherited parent's accumulated list:

| Value | Behavior |
|-------|----------|
| `"replace"` | Discard the parent's list; use only this profile's list (default) |
| `"extend"` | Append this profile's list after the parent's accumulated list |

```toml
[profile.base]
[profile.base.cmake]
cpp_flags = ["-Wall", "-Wextra"]

[profile.strict]
inherits = "base"
[profile.strict.cmake]
merge     = "extend"          # append after base's ["-Wall", "-Wextra"]
cpp_flags = ["-Werror"]       # result: ["-Wall", "-Wextra", "-Werror"]
```

Without `merge = "extend"`, the `strict` profile would replace `base`'s flags entirely.

## Inheritance

Profiles support single inheritance with `inherits`:

```toml
[profile.base]
release = false
[profile.base.cmake]
cpp_flags = ["-Wall"]

[profile.sanitize]
inherits = "base"       # inherits release=false and cpp_flags=["-Wall"]
[profile.sanitize.cmake]
merge     = "extend"
cpp_flags = ["-fsanitize=address"]   # final: ["-Wall", "-fsanitize=address"]
```

**Rules:**
- Only single inheritance (`inherits` accepts one name, not a list)
- Chains are allowed: `sanitize` inherits `base` which inherits `debug`
- Circular references are detected and reported as an error
- The built-in profiles `debug` and `release` are always valid `inherits` targets

## Priority Order

Settings are applied from lowest to highest priority. Later entries win:

1. Hardcoded defaults (e.g., release = false)
2. Global `CCGO.toml` settings (`[build]`, `[build.cmake]`, `[platforms.X.build.cmake]`)
3. Inherited profile chain (oldest ancestor first, newest ancestor last)
4. Active profile's own settings
5. CLI flags (always win — `--release`, `--link-type`, `--features`, `--jobs`)

For example, if the profile sets `release = true` but you also pass `--release` on the CLI, the result is still `release = true` (they agree). If the profile sets `release = false` but you pass `--release`, the CLI wins.

## CMake Flag Merge Order

CMake flags accumulate through four layers (earlier = lower priority):

```
[build.cmake]                              global CCGO.toml cmake flags
  + [platforms.android.build.cmake]        global platform-level cmake flags
  + [profile.X.cmake]                      profile global cmake flags
  + [profile.X.platforms.android.build.cmake]   profile platform cmake flags
```

All four layers are concatenated (not replaced). This ensures that flags set in `[build.cmake]` and platform overrides remain effective alongside profile-specific flags.

## Overriding the Package Name

The `name` field in a profile overrides the package name used for build artifacts and SDK archives. This is useful when publishing platform-specific or configuration-specific builds under different names:

```toml
[profile.release-debug-symbols]
inherits = "release"
name = "mylib-with-symbols"
[profile.release-debug-symbols.cmake]
cpp_flags = ["-g"]
```

## Example: Common Profiles

### Debug + ASan

```toml
[profile.asan]
inherits  = "debug"
link_type = "both"

[profile.asan.cmake]
c_flags   = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
cpp_flags = ["-fsanitize=address,undefined", "-fno-omit-frame-pointer"]
```

### Release with hidden visibility

```toml
[profile.release-hidden]
inherits  = "release"
link_type = "shared"

[profile.release-hidden.cmake]
cpp_flags = ["-fvisibility=hidden", "-fvisibility-inlines-hidden"]
```

### Fat static (no dependencies dynamically linked)

```toml
[profile.fat-static]
inherits  = "release"
link_type = "static"

[profile.fat-static.dep_linkage]
default = "static-embedded"
```

### Platform-specific overrides

```toml
[profile.neon]
inherits = "release"

[profile.neon.platforms.android.build.cmake]
merge     = "extend"
arguments = ["-DANDROID_ARM_NEON=TRUE"]
```

## See Also

- [CCGO.toml Reference — \[profile.\<name\>\]](../reference/ccgo-toml.md#profilename)
- [CCGO.toml.example](../../CCGO.toml.example) — complete annotated template
- [Build System](build-system.md)
