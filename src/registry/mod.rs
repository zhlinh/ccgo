//! Package registry and Git shorthand support
//!
//! This module provides:
//! - Git URL shorthand syntax (e.g., `github:user/repo`)
//! - Automatic version discovery from Git tags
//! - Lightweight index repository support
//!
//! # Overview
//!
//! CCGO supports three ways to specify dependencies:
//!
//! 1. **Git shorthand** - Simplified syntax for common hosts
//!    ```toml
//!    # In CCGO.toml or via ccgo add
//!    ccgo add github:fmtlib/fmt
//!    ccgo add gh:nlohmann/json@v3.11.0
//!    ```
//!
//! 2. **Full Git URL** - Traditional explicit URLs
//!    ```toml
//!    [[dependencies]]
//!    name = "fmt"
//!    git = "https://github.com/fmtlib/fmt.git"
//!    branch = "v10.1.1"
//!    ```
//!
//! 3. **Package index** - Simplified version syntax (via index)
//!    ```toml
//!    [dependencies]
//!    fmt = "^10.1"
//!    spdlog = "1.12.0"
//!    ```

mod index;
mod shorthand;
mod version_discovery;

pub use index::{
    generate_package_entry, IndexMetadata, PackageEntry, PackageIndex, RegistryConfig,
    ResolvedPackage, UpdateResult, VersionEntry, DEFAULT_REGISTRY, DEFAULT_REGISTRY_URL,
};
pub use shorthand::{expand_git_shorthand, GitProvider, ShorthandSpec};
pub use version_discovery::{discover_latest_version, GitTagInfo, SemVer, VersionDiscovery};
