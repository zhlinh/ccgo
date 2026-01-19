# CCGO Roadmap

> Version: v3.0.10 | Updated: 2026-01-19

## Project Status Overview

| Module | Progress | Status |
|--------|----------|--------|
| Python CLI | 100% | Feature complete, maintenance mode |
| Rust CLI | 85% | Core features migrated, in active development |
| Cross-Platform Builds | 100% | 8 platforms supported |
| Docker Builds | 100% | Universal cross-compilation |
| Dependency Management | 95% | Git, path, registry sources with lockfile |
| Publishing System | 100% | Maven, CocoaPods, SPM, OHPM, Conan |
| Template System | 100% | Copier-based project generation |
| CMake Integration | 100% | Centralized build scripts |
| Gradle Plugins | 100% | Android/KMP convention plugins |
| Documentation | 70% | MkDocs with i18n (this document!) |

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
**Status**: 85% Complete | **Target**: v3.1.0 (Q1 2026)

- [x] Core build commands (build, test, bench, doc) ✅
- [x] Dependency management (install with lockfile) ✅
- [x] Project creation (new, init) ✅
- [x] Version management (tag, package) ✅
- [ ] Vendor command implementation
- [ ] Update command for dependency updates
- [ ] Run command for examples/binaries
- [ ] CI command orchestration
- [ ] Complete migration from Python to Rust

**Rationale**: Rust provides better performance, type safety, and easier distribution (single binary).

### 2. Documentation Completion
**Status**: 70% Complete | **Target**: v3.1.0 (Q1 2026)

- [x] MkDocs setup with i18n ✅
- [x] Home page and getting started ✅
- [ ] Complete platform guides (Android, iOS, macOS, etc.)
- [ ] CLI reference documentation
- [ ] CCGO.toml configuration reference
- [ ] CMake integration guide
- [ ] Gradle plugins reference
- [ ] Migration guides (from Conan, vcpkg, etc.)

**Rationale**: Good documentation is critical for user adoption and reducing support burden.

### 3. Error Handling Enhancement
**Status**: 50% Complete | **Target**: v3.1.0 (Q1 2026)

- [x] Unified error types in Rust CLI ✅
- [ ] User-friendly error messages with actionable hints
- [ ] Graceful degradation when tools missing
- [ ] Configuration validation with helpful suggestions
- [ ] Better diagnostics for build failures

---

## P1 - High (v3.2-v3.3) 🚀

### 4. Package Registry Support
**Status**: 0% Complete | **Target**: v3.2.0 (Q2 2026)

- [ ] ccgo-registry server implementation
- [ ] Package publishing to ccgo-registry
- [ ] Package discovery and search
- [ ] Semantic versioning resolution
- [ ] Private registry support
- [ ] Integration with existing registries (Conan Center, vcpkg)

**Rationale**: Enable easier dependency sharing within organizations and community.

### 5. IDE Integration
**Status**: 10% Complete | **Target**: v3.2.0 (Q2 2026)

- [ ] VS Code extension
  - Syntax highlighting for CCGO.toml
  - Build tasks integration
  - Dependency tree visualization
- [ ] CLion/Android Studio plugin
- [ ] Xcode project generation improvements
- [ ] Visual Studio project generation

**Rationale**: Better IDE support improves developer experience.

### 6. Build Performance Optimization
**Status**: 40% Complete | **Target**: v3.3.0 (Q2 2026)

- [x] Parallel platform builds ✅
- [x] Docker layer caching ✅
- [ ] Incremental builds (only rebuild changed sources)
- [ ] Build cache sharing (ccache, sccache integration)
- [ ] Remote build execution (distcc, icecc)
- [ ] Build analytics and profiling

**Rationale**: Faster builds = happier developers.

### 7. Advanced Dependency Features
**Status**: 30% Complete | **Target**: v3.3.0 (Q2 2026)

- [x] Git dependencies with revision pinning ✅
- [x] Path dependencies ✅
- [x] Lockfile generation ✅
- [ ] Dependency override/patches
- [ ] Dependency vendoring improvements
- [ ] Transitive dependency resolution
- [ ] Version conflict resolution strategies
- [ ] Workspace dependencies (monorepo support)

---

## P2 - Medium (v3.4-v4.0) 📦

### 8. Testing Framework Enhancement
**Status**: 60% Complete | **Target**: v3.4.0 (Q3 2026)

- [x] Google Test integration ✅
- [x] Catch2 integration ✅
- [ ] Test discovery improvements
- [ ] Code coverage reporting
- [ ] Test result aggregation
- [ ] Benchmark result comparison
- [ ] Integration with CI services

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
- Email: ccgo@mojeter.com

---

*This roadmap is a living document and may change based on community feedback and project priorities.*
