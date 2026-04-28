# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Features

* **Linkage strategy** (`[[dependencies]].linkage` / `[build].default_dep_linkage`): consumers building shared libraries can now keep dependencies external (`shared-external`, default) instead of archiving them in. Eliminates the per-consumer copy of every transitive dep that previously bloated APKs containing multiple sibling libraries. **OHOS** static consumers now produce thin `.a` files (skip the third-party merge); other platforms' static-build behavior is unchanged in this release.

  Migration:
  * Existing **shared-consumer** projects need no change unless they relied on shared consumers being self-contained (e.g. shipping a single `.so` to external app developers). Those projects should set `linkage = "static-embedded"` on the relevant deps to keep the old behavior.
  * Existing **OHOS static-consumer** projects: if you shipped a `.a` that external code links directly and relied on third-party dep symbols being merged in, set `linkage = "static-embedded"` on those deps, or the resulting archive will no longer contain those symbols.

  See [`docs/dependency-linkage.md`](docs/dependency-linkage.md) for the full decision matrix.

* **Source-only dependency auto-materialize**: `ccgo build` now automatically
  (re)compiles any path-source dependency whose `lib/<platform>/` artifacts
  are missing or stale relative to the dep's source tree. The materialize
  step spawns a recursive `ccgo build` inside `.ccgo/deps/<name>/` with a
  `--build-as` derived from the consumer's resolved linkage hint, caches
  via per-platform fingerprint at `.ccgo/deps/<name>/.ccgo_materialize_<platform>.fingerprint`,
  and exposes the build output to `FindCCGODependencies.cmake` via a
  `lib/<platform>/` → `cmake_build/<profile>/<platform>/` symlink. See
  [`docs/dependency-linkage.md`](docs/dependency-linkage.md#source-only-dependencies)
  for the full behavior matrix.

  Limitations in this release:
  * The consumer-side CMake template's source-precedence behavior means
    a source-shipped dep linked as `shared-external` still ends up
    statically embedded in practice. Tracked as a follow-up.
  * Windows path-source deps: the `lib/<platform>/` bridge uses Unix
    `symlink`; junction/copy fallback is deferred.

### Changed

* `crate::build::linkage::resolve_linkage` now returns an explicit error
  when given `DepArtifacts::SourceOnly` instead of silently coalescing
  with `Both`. External callers must run `BuildContext::materialize_source_deps`
  (or the standalone `materialize_source_deps_inner`) before resolving
  linkage. This prevents a class of bugs where the materialize step is
  skipped and the resolver silently produces an incorrect link line.

## [3.4.4] - 2026-04-20

### Added

- add `ccgo self update` command

### Documentation

- reorder installation methods and add uv tool install

## [3.4.3] - 2026-04-17

### Added

- CCGO.toml-sourced versions + embedded VERIDENTITY
- mvn-install-style local package cache + bin/lib targets

### Fixed

- zero out remaining clippy errors
- address clippy errors from last result
- use relative CHANGELOG link in release notes

### Changed

- extract helpers to reduce cyclomatic complexity

## [3.4.2] - 2026-04-13

### Added

- add `ccgo release` command for user project releases
- sync CCGO.toml version into android/ohos manifests

### Documentation

- regenerate CHANGELOG.md from git history

### Added

- add `ccgo release` command for user project releases
- sync CCGO.toml version into android/ohos manifests

## [3.4.1] - 2026-04-11

### Added

- add Doxygen support alongside MkDocs

### Fixed

- remove -fsyntax-only flag that breaks CMake compiler detection

## [3.4.0] - 2026-03-16

### Added

- add tar.gz archive support, xcframework resolution, and dist branch publishing
- support tar.gz archives alongside zip for prebuilt SDK deps
- embed CCGO.toml with transitive zip deps into merged SDK ZIP
- add zip source support for prebuilt SDK deps
- add package ZIP path search and dedup guard to ccgo_link_dependency
- add zip field to DependencyConfig for prebuilt SDK deps

### Fixed

- make AAR build failure fatal instead of warning
- use ~/.ccgo/cmake/ for MSVC toolchain file like other platforms
- simplify OHOS detection to if(OHOS) for consistency
- handle zip deps in resolver, lockfile SourceType, and WorkspaceDependency
- reject zip slip paths and check HTTP status in download_zip
- add zip branch to build_source_string in resolver and lockfile
- fix variable naming and improve Apple platform support

### Changed

- split check into check (compilation) and doctor (diagnostics)

### Documentation

- add publishing guide for JetBrains plugin
- add publishing guide for VS Code extension

## [3.3.0] - 2026-01-25

### Added

- add missing commands (doc, check, publish, tag, package, bench, tree)
- add missing commands (doc, check, publish, tag, package, bench, tree)
- disable notifications by default
- add Neovim plugin (nvim-ccgo)

### Fixed

- skip terminal buffers when finding CCGO.toml, try recent file buffers
- pass all architectures as comma-separated list when 'all' is selected
- add command stubs for lazy loading support
- rename commands from CcgoXxx to ccgoXxx format

### Documentation

- change icons
- reorder installation sections, clarify file paths
- improve installation instructions for local and remote setup
- update installation for GitHub subdirectory
- clarify installation file paths
- add Neovim plugin to IDE Integration
- update to Rust-only implementation

## [3.2.0] - 2026-01-25

### Added

- add ccgo publish index command
- add package registry and simplified dependency syntax
- add JetBrains plugin

### Fixed

- use xcrun to find compilers for IDE project generation
- simplify IDE project generation to let Xcode auto-detect SDK
- handle IDE project output correctly in build result display
- use explicit remote and branch in git push

## [3.1.0] - 2026-01-22

### Added

- add VS Code extension and platform IDE project generation
- add ccache/sccache integration and build analytics
- complete testing framework enhancement

### Fixed

- resolve merge conflict in roadmap.md

## [3.0.11] - 2026-01-22

### Added

- update MkDocs theme
- implement workspace dependencies for monorepo support

### Documentation

- update documentation completion - mark CMake guide, Gradle plugins, and migration guides as complete
- add comprehensive Conan to CCGO migration guide
- add comprehensive Gradle plugins reference
- add comprehensive CMake integration guide
- update documentation completion status
- add comprehensive CCGO.toml configuration reference
- add comprehensive Chinese CLI reference documentation
- add comprehensive tree, search, collection commands documentation
- update contact email from ccgo@mojeter.com to zhlinhng@gmail.com
- fix markdown list formatting in dependency-management and gradle-plugins
- fix markdown list formatting in project-structure docs
- fix markdown list formatting in all platform docs
- fix markdown list formatting in configuration docs
- fix markdown list formatting in build-system docs
- complete missing documentation and update navigation
- add logo and favicon SVG assets for documentation site
- add comprehensive bilingual MkDocs documentation

## [3.0.10] - 2026-01-18

### Added

- add ccgo vendor command
- add ccgo run command to build and execute examples/binaries
- add ccgo run command
- add workspace support for multi-package management
- add features system for conditional compilation
- add HTTP(S) support for remote collections
- add package collection and search commands
- add ccgo tree command for dependency visualization
- add comprehensive dependency management commands

### Fixed

- enhance get pip command
- fix build bench and test

### Documentation

- add features system documentation

## [3.0.9] - 2026-01-15

### Added

- enable MSVC cross-compilation with xwin and clang-cl

### Fixed

- update OHOS SDK URL format and version to 5.0.1

## [3.0.8] - 2026-01-11

### Added

- add auto-docker flag and subprocess isolation for multi-platform builds

### Fixed

- print build info JSON before success message

### Documentation

- update README

## [3.0.7] - 2026-01-04

### Fixed

- merge static library modules for Linux and Windows

### Documentation

- add CHANGELOG

## [3.0.5] - 2026-01-04

### Fixed

- pass toolchain argument for Windows builds in Docker

### Documentation

- use consistent badge style for PyPI version

## [3.0.4] - 2026-01-03

### Added

- add ELF parsing and unify include directory handling across platforms

### Fixed

- prioritize out/ directory for merged libraries and rewrite builders

## [3.0.3] - 2026-01-02

### Added

- add OHOS Docker support and fix Linux SDK archive
- update lock file
- rewrite ccgo CLI in Rust with Docker build support
- use MkDocs + MkDoxy for documentation instead of CMake
- add --registry local support for apple and fix conan config loading
- add tar.gz format support for OHOS HAR file preview
- bump to v2.5.0 and add buildHAR fallback to assembleHar
- add ConanConfig module for CCGO.toml integration
- unify Maven registry options across Android and KMP targets
- add Conan registry selection and improve remote configuration
- add OHPM registry selection and improve package metadata handling
- add dependency support for Apple, Maven, and OHPM platforms
- add Apple platform publishing support (CocoaPods & SPM)
- add --artifact-id option for Android/KMP publishing
- separate debug/release artifacts into distinct directories
- standardize archive structure with meta directory
- add Apple static library build and improve Docker error handling
- add source

### Fixed

- support dual registry publishing and fix unreachable code warnings
- only add AAR to haars/android/ in SDK archive
- embed Dockerfiles in binary at compile time
- clean up target directory and preserve HAR for publish
- clean stale HAR files and check buildHAR task before execution
- fix target directory path in print_build_results
- use correct target subdir in print_build_results
- map macOS to 'osx' for CocoaPods podspec generation
- improve CocoaPods podspec defaults and git URL handling
- use correct target subdir in tvOS/watchOS build results
- rename Gradle tasks and enable real-time output
- prevent KMP from overwriting Android AAR in parallel builds

### Changed

- use pre-installed ccgo from PyPI in Docker images
- change cmake_build to cmake_build/{release|debug}/<platform> structure
- unify output directory structure across all platforms
- clean up redundant artifacts and simplify archive naming
- separate local build from publish workflow
- simplify package command to copy ZIP artifacts directly
- merge ci command into build all and improve KMP packaging

### Documentation

- added crates.io version badge to README header
- add README


