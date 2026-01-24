# CCGO Roadmap

> Version: v3.0.13 | Updated: 2026-01-23

## Project Status Overview

| Module | Progress | Status |
|--------|----------|--------|
| Python CLI | 100% | Feature complete, maintenance mode |
| Rust CLI | 100% | Feature complete, zero Python dependencies ✅ |
| Cross-Platform Builds | 100% | 8 platforms supported |
| Docker Builds | 100% | Universal cross-compilation |
| Dependency Management | 100% | Git, path, patches, lockfile, transitive resolution ✅ |
| Publishing System | 100% | Maven, CocoaPods, SPM, OHPM, Conan |
| Template System | 100% | Copier-based project generation |
| CMake Integration | 100% | Centralized build scripts |
| Gradle Plugins | 100% | Android/KMP convention plugins |
| Documentation | 100% | MkDocs with i18n (this document!) |

**Supported Platforms**: Android, iOS, macOS, Windows, Linux, OpenHarmony, watchOS, tvOS, Kotlin Multiplatform

---

## Priority Definitions

- **P0 (Critical)**: Blocking core functionality or release
- **P1 (High)**: Important features or major improvements
- **P2 (Medium)**: Enhancements, valuable but not urgent
- **P3 (Low)**: Long-term planning, nice-to-have

---

## P0 - Critical (Current Release v3.1) 🔥

### 1. Rust CLI Feature Parity
**Status**: 100% Complete ✅ | **Target**: v3.1.0 (Q1 2026)

- [x] Core build commands (build, test, bench, doc) ✅
- [x] Dependency management (install with lockfile) ✅
- [x] Project creation (new, init) ✅
- [x] Version management (tag, package) ✅
- [x] Vendor command implementation ✅
- [x] Update command for dependency updates ✅
- [x] Run command for examples/binaries ✅
- [x] CI command orchestration (via command composition) ✅
- [x] Complete migration from Python to Rust ✅
- [x] Zero Python dependencies (direct Copier invocation) ✅

**Rationale**: Rust provides better performance, type safety, and easier distribution (single binary).

### 2. Documentation Completion
**Status**: 100% Complete | **Target**: v3.1.0 (Q1 2026) ✅

- [x] MkDocs setup with i18n ✅
- [x] Home page and getting started ✅
- [x] Complete platform guides (Android, iOS, macOS, Linux, Windows, OpenHarmony, KMP) ✅
- [x] CLI reference documentation ✅
- [x] CCGO.toml configuration reference ✅
- [x] CMake integration guide ✅
- [x] Gradle plugins reference ✅
- [x] Migration guides (from Conan, Python to Rust) ✅

**Rationale**: Good documentation is critical for user adoption and reducing support burden.

### 3. Error Handling Enhancement
**Status**: 100% Complete | **Target**: v3.1.0 (Q1 2026) ✅

- [x] Unified error types in Rust CLI ✅
- [x] Custom error types with contextual hints ✅
- [x] User-friendly error messages with actionable hints ✅
- [x] Graceful degradation when tools missing ✅
- [x] Comprehensive configuration validation ✅
- [x] Tool detection module with requirement levels ✅
- [x] Integration into build/publish commands ✅
- [x] Build failure diagnostics with common solutions ✅

---

## P1 - High (v3.2-v3.3) 🚀

### 4. Package Registry Support
**Status**: 100% Complete ✅ | **Target**: v3.2.0 (Q2 2026)

**Phase 1 - Git-based Enhancement (v3.1.1)** ✅
- [x] Git URL shorthand syntax (`github:user/repo`, `gh:`, `gl:`, `bb:`) ✅
- [x] Automatic version discovery from Git tags (`--latest` flag) ✅
- [x] Bare `owner/repo` syntax (assumes GitHub) ✅
- [x] Pre-release version support (`--prerelease` flag) ✅

**Phase 2 - Lightweight Index (v3.2.0)** ✅
- [x] Index repository format design ✅
- [x] Index parsing and caching ✅
- [x] Simplified version syntax (`fmt = "^10.1"`) ✅
- [x] Private index support (`[registries]` section) ✅
- [x] Package search via index (`ccgo search`, `ccgo registry search`) ✅
- [x] Registry management commands (`ccgo registry add/list/remove/update/info`) ✅

**Phase 3 - Publishing Tools (v3.2.1)** ✅
- [x] `ccgo publish index` command ✅
- [x] Auto-generate package metadata JSON from CCGO.toml ✅
- [x] Version discovery from Git tags ✅
- [x] SHA-256 checksum generation via `--checksum` flag ✅

**Design Decision**: Following SPM's Git-based approach instead of central registry server.
- No server maintenance required
- Leverages existing Git infrastructure
- Natural support for private packages
- Index repository is just a Git repo (like crates.io-index)

**Rationale**: Enable easier dependency sharing within organizations and community.

### 5. IDE Integration
**Status**: 100% Complete ✅ | **Target**: v3.2.0 (Q2 2026)

- [x] VS Code extension (`vscode-ccgo/`) ✅
  - Syntax highlighting for CCGO.toml (TextMate grammar)
  - JSON Schema validation with error hints
  - Build tasks integration (all platforms, debug/release)
  - Dependency tree visualization (via `ccgo tree --format json`)
  - Code snippets for common patterns
- [x] CLion/Android Studio plugin (`jetbrains-ccgo/`) ✅
  - CCGO.toml syntax highlighting (TOML plugin integration)
  - JSON Schema validation with real-time error hints
  - Run configurations for all platforms and commands
  - Dependency tree tool window
  - Live templates for common patterns
  - Settings UI for plugin configuration
- [x] Xcode project generation (`ccgo build ios/macos --ide-project`) ✅
- [x] Visual Studio project generation (`ccgo build windows --ide-project`) ✅
- [x] Linux IDE project generation (`ccgo build linux --ide-project`) ✅
  - CodeLite workspace + compile_commands.json for VS Code/clangd

**Rationale**: Better IDE support improves developer experience.

### 6. Build Performance Optimization
**Status**: 67% Complete | **Target**: v3.3.0 (Q2 2026)

- [x] Parallel platform builds ✅
- [x] Docker layer caching ✅
- [ ] Incremental builds (only rebuild changed sources)
- [x] Build cache sharing (ccache, sccache integration) ✅
- [ ] Remote build execution (distcc, icecc)
- [x] Build analytics and profiling ✅

**Rationale**: Faster builds = happier developers.

### 7. Advanced Dependency Features
**Status**: 100% Complete ✅ | **Target**: v3.3.0 (Q2 2026)

- [x] Git dependencies with revision pinning ✅
- [x] Path dependencies ✅
- [x] Lockfile generation ✅
- [x] Dependency override/patches ✅
- [x] Dependency vendoring improvements ✅
  - SHA-256 checksum verification for vendored archives
  - Build artifact exclusion rules (target/, cmake_build/, etc.)
- [x] Transitive dependency resolution ✅ ([docs](dependency-resolution.md))
- [x] Version conflict resolution strategies ✅
  - First (default), Highest, Lowest, Strict modes
  - `--conflict-strategy` CLI option in install command
- [x] Workspace dependencies (monorepo support) ✅
  - `--workspace` flag for build/install commands
  - `--package <name>` for targeting specific members

---

## P2 - Medium (v3.4-v4.0) 📦

### 8. Testing Framework Enhancement
**Status**: 100% Complete ✅ | **Target**: v3.4.0 (Q3 2026)

- [x] Google Test integration ✅
- [x] Catch2 integration ✅
- [x] Test discovery improvements ✅
  - GoogleTest, Catch2, and CTest discovery
  - Test filtering by name pattern
  - Suite-based organization
- [x] Code coverage reporting ✅
  - gcov, llvm-cov, lcov support
  - HTML, LCOV, JSON, Cobertura output formats
  - Threshold enforcement with --fail-under-coverage
- [x] Test result aggregation ✅
  - XML result parsing (GoogleTest format)
  - Cross-suite aggregation
  - JUnit XML export
- [x] Benchmark result comparison ✅
  - Google Benchmark JSON parsing
  - Baseline comparison with regression detection
  - Markdown/JSON export for reports
- [x] Integration with CI services ✅
  - GitHub Actions, GitLab CI, Azure DevOps, Jenkins, TeamCity
  - Auto-detection of CI environment
  - Native CI annotation formats

### 9. Code Generation Tools
**Status**: 0% Complete | **Target**: v3.5.0 (Q3 2026)

- [ ] Protocol Buffers support
- [ ] Flat Buffers support
- [ ] gRPC support
- [ ] GraphQL code generation
- [ ] OpenAPI client generation
- [ ] Custom code gen plugin system

### 10. Platform-Specific Features
**Status**: Various | **Target**: v3.6.0 (Q4 2026)

- [ ] **Android**
  - [ ] Jetpack Compose native interop
  - [ ] Android Studio plugin
  - [ ] R8/ProGuard configuration
- [ ] **iOS/macOS**
  - [ ] SwiftUI interop helpers
  - [ ] Xcode Cloud integration
  - [ ] App Clip support
- [ ] **OpenHarmony**
  - [ ] DevEco Studio integration
  - [ ] ArkTS interop
- [ ] **Windows**
  - [ ] UWP support
  - [ ] WinUI 3 integration

### 11. Security Features
**Status**: 20% Complete | **Target**: v3.7.0 (Q4 2026)

- [x] Basic checksum verification ✅
- [ ] GPG signature verification for dependencies
- [ ] Security audit reports
- [ ] CVE scanning for dependencies
- [ ] Supply chain security (SLSA compliance)
- [ ] Code signing automation

---

## P3 - Low (v4.0+) 🔮

### 12. WebAssembly Support
**Status**: 0% Complete | **Target**: v4.0.0 (2027)

- [ ] WASM target compilation
- [ ] Emscripten integration
- [ ] WASI support
- [ ] WebAssembly System Interface (WASI)

### 13. AI-Powered Features
**Status**: 0% Complete | **Target**: v4.1.0 (2027)

- [ ] Dependency suggestion based on project analysis
- [ ] Build configuration optimization recommendations
- [ ] Automatic migration from other build systems
- [ ] Code generation from natural language

### 14. Cloud Build Service
**Status**: 0% Complete | **Target**: v4.2.0 (2027)

- [ ] Hosted build service (ccgo-cloud)
- [ ] Distributed caching
- [ ] Build analytics dashboard
- [ ] Team collaboration features

### 15. Advanced Platform Support
**Status**: 0% Complete | **Target**: v4.x (2027+)

- [ ] FreeBSD support
- [ ] Haiku OS support
- [ ] RISC-V architecture support
- [ ] LoongArch architecture support
- [ ] PlayStation/Xbox platforms (if licensing permits)

---

## Recently Completed (v3.0) ✅

### IDE Integration (v3.0.12)
- [x] VS Code extension (`vscode-ccgo/`)
  - TextMate grammar for CCGO.toml syntax highlighting
  - JSON Schema validation with real-time error hints
  - Build task provider for all platforms (debug/release)
  - Dependency tree view using `ccgo tree --format json`
  - Code snippets for package, build, dependencies, platforms, publish sections
- [x] Platform IDE project generation (`--ide-project` flag)
  - iOS/macOS: Xcode project generation (CMake -G "Xcode")
  - Windows: Visual Studio 2022 project (CMake -G "Visual Studio 17 2022")
  - Windows (MinGW): CodeLite project (CMake -G "CodeLite - MinGW Makefiles")
  - Linux: CodeLite workspace + compile_commands.json for clangd/VS Code

### Build Performance Optimization (v3.0.12)
- [x] Build cache sharing (ccache, sccache integration)
  - Automatic detection of ccache/sccache in PATH
  - Preference order: sccache > ccache (sccache is faster, more features)
  - CMAKE_C_COMPILER_LAUNCHER and CMAKE_CXX_COMPILER_LAUNCHER configuration
  - `--cache` CLI option: auto (default), ccache, sccache, none
  - Integrated into all platform builders
- [x] Build analytics and profiling
  - Build timing per phase (configure, compile, link, package)
  - Cache hit/miss statistics from ccache/sccache
  - File count and artifact size tracking
  - Historical data storage at ~/.ccgo/analytics/
  - `--analytics` flag to display build metrics
  - `ccgo analytics` command for viewing build history
  - Comparison with historical averages

### Testing Framework Enhancement (v3.0.12)
- [x] Test discovery improvements
  - GoogleTest (`--gtest_list_tests`), Catch2 (`--list-tests`), CTest (`ctest -N`)
  - Test filtering by name pattern with regex support
  - Suite-based organization and listing
- [x] Code coverage reporting
  - Support for gcov, llvm-cov, lcov tools
  - Output formats: HTML, LCOV, JSON, Cobertura, Summary
  - Threshold enforcement with `--fail-under-coverage` flag
- [x] Test result aggregation
  - GoogleTest XML result parsing
  - Cross-suite aggregation with pass/fail/skip counts
  - JUnit XML export for CI integration
- [x] Benchmark result comparison
  - Google Benchmark JSON parsing
  - Baseline comparison with configurable threshold
  - Regression detection with `--fail-on-regression`
  - Markdown/JSON export for reports
- [x] CI service integration
  - GitHub Actions (workflow annotations)
  - GitLab CI (collapsible sections)
  - Azure DevOps (task commands)
  - Jenkins (console formatting)
  - TeamCity (service messages)
  - Auto-detection via environment variables

### Advanced Dependency Features (v3.0.12)
- [x] Dependency vendoring improvements
  - SHA-256 checksum verification for vendored archives
  - Build artifact exclusion rules (target/, cmake_build/, bin/, etc.)
  - Improved archive creation with proper directory handling
- [x] Version conflict resolution strategies
  - Four strategies: First (default), Highest, Lowest, Strict
  - `--conflict-strategy` CLI option for install command
  - Integrated into VersionResolver with strategy-aware resolution
- [x] Workspace dependencies (monorepo support)
  - `--workspace` flag for building/installing all workspace members
  - `--package <name>` for targeting specific workspace members
  - Topological order resolution for inter-member dependencies

### Transitive Dependency Resolution (v3.0.11)
- [x] Dependency graph with cycle detection (DFS algorithm)
- [x] Topological sorting for correct build order (Kahn's algorithm)
- [x] Dependency tree visualization with shared dependency detection
- [x] Version conflict warnings
- [x] Recursive CCGO.toml resolution with path handling
- [x] Max depth protection (50 levels)
- [x] Integration with `ccgo install` command
- [x] Comprehensive test suite (10 tests: 7 resolver + 3 graph)
- [x] Full documentation ([docs/dependency-resolution.md](dependency-resolution.md))

### Rust CLI Migration (Partial)
- [x] Project architecture redesign
- [x] Core commands implementation
- [x] Dependency management system
- [x] Build orchestration
- [x] Configuration parsing (CCGO.toml)

### Docker Build System
- [x] Docker-based universal cross-compilation
- [x] Pre-built Docker images for all platforms
- [x] Image caching and optimization
- [x] Multi-stage build support

### Unified Publishing
- [x] Maven (local, private, central) publishing
- [x] CocoaPods publishing
- [x] Swift Package Manager publishing
- [x] OHPM publishing
- [x] Conan publishing

### Git Integration
- [x] Automatic version tagging
- [x] Commit message generation
- [x] Git hooks (pre-commit) support
- [x] Git-based dependencies

---

## How to Contribute

We welcome contributions! Here's how you can help:

1. **Pick a Feature**: Choose an item from P1 or P2 priorities
2. **Discuss**: Open a GitHub Discussion or Issue to discuss your approach
3. **Implement**: Follow our [Contributing Guide](contributing.md)
4. **Test**: Ensure your changes work across platforms
5. **Document**: Update docs with new features
6. **Submit**: Create a pull request

See [Contributing Guide](contributing.md) for detailed guidelines.

---

## Feedback

Have ideas for CCGO's future? We'd love to hear from you!

- [GitHub Discussions](https://github.com/zhlinh/ccgo/discussions) - Feature requests and ideas
- [GitHub Issues](https://github.com/zhlinh/ccgo/issues) - Bug reports and tasks
- Email: zhlinhng@gmail.com

---

*This roadmap is a living document and may change based on community feedback and project priorities.*
