#![allow(dead_code)] // Allow unused code during development

//! Native Rust build orchestration module
//!
//! This module provides platform-specific build logic that replaces the Python
//! build scripts with native Rust implementations. The goal is ZERO Python
//! dependency - all build logic is implemented in Rust.
//!
//! ## Architecture
//!
//! ```text
//! Rust CLI → build/mod.rs → platforms/<platform>.rs → CMake/Gradle/Hvigor
//! ```
//!
//! ## Modules
//!
//! - `platforms` - Platform-specific builders (linux, macos, windows, ios, android, ohos, etc.)
//! - `toolchains` - Toolchain detection (gcc, clang, xcode, ndk, msvc, mingw, etc.)
//! - `cmake` - CMake configuration and execution
//! - `cache` - Compiler cache support (ccache, sccache)
//! - `analytics` - Build performance metrics and analytics
//! - `incremental` - Incremental build support with smart rebuild detection
//! - `docker` - Docker-based cross-platform builds
//! - `archive` - ZIP archive creation with build_info.json

pub mod analytics;
pub mod archive;
pub mod cache;
pub mod cmake;
pub mod cmake_templates;
pub mod docker;
pub mod elf;
pub mod incremental;
pub mod linkage;
pub mod materialize;
pub mod platforms;
pub mod toolchains;
pub mod verinfo;
pub mod profile;

use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::commands::build::{BuildTarget, LinkType, WindowsToolchain};
use crate::config::CcgoConfig;

/// Build options passed to platform builders
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Target platform
    pub target: BuildTarget,
    /// Architectures to build (platform-specific)
    pub architectures: Vec<String>,
    /// Link type (static, shared, or both)
    pub link_type: LinkType,
    /// Use Docker for building
    pub use_docker: bool,
    /// Automatically use Docker when native build is not possible
    pub auto_docker: bool,
    /// Number of parallel jobs
    pub jobs: Option<usize>,
    /// Generate IDE project files
    pub ide_project: bool,
    /// Build in release mode
    pub release: bool,
    /// Build only native libraries without packaging (AAR/HAR)
    pub native_only: bool,
    /// Windows toolchain (msvc, mingw, auto)
    pub toolchain: WindowsToolchain,
    /// Verbose output
    pub verbose: bool,
    /// Development mode: use pre-built ccgo binary from GitHub releases in Docker
    pub dev: bool,
    /// Features to enable (resolved from command line)
    pub features: Vec<String>,
    /// Whether to use default features
    pub use_default_features: bool,
    /// Enable all available features
    pub all_features: bool,
    /// Compiler cache type (ccache, sccache, auto, none)
    pub cache: Option<String>,
    /// Show build analytics summary
    pub analytics: bool,
    /// CLI override for project-wide default linkage (`--linkage <value>`).
    /// Falls through to `[build].default_dep_linkage` when `None`.
    pub linkage_default: Option<crate::config::Linkage>,
    /// CLI per-dependency linkage overrides (`--linkage <name>=<value>`).
    /// Highest priority — wins over every CCGO.toml setting.
    pub linkage_overrides: std::collections::HashMap<String, crate::config::Linkage>,
    /// CLI override when the consumer builds as **shared** (`--linkage-on-shared <value>`).
    /// Mirrors `linkage` semantics but only applies when `build_as = shared`.
    pub linkage_on_shared_default: Option<crate::config::Linkage>,
    /// CLI per-dep overrides for shared consumers (`--linkage-on-shared <name>=<value>`).
    pub linkage_on_shared_overrides: std::collections::HashMap<String, crate::config::Linkage>,
    /// CLI override when the consumer builds as **static** (`--linkage-on-static <value>`).
    pub linkage_on_static_default: Option<crate::config::Linkage>,
    /// CLI per-dep overrides for static consumers (`--linkage-on-static <name>=<value>`).
    pub linkage_on_static_overrides: std::collections::HashMap<String, crate::config::Linkage>,
}

impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            target: BuildTarget::Linux,
            architectures: Vec::new(),
            link_type: LinkType::Both,
            use_docker: false,
            auto_docker: false,
            jobs: None,
            ide_project: false,
            release: true,
            native_only: false,
            toolchain: WindowsToolchain::Auto,
            verbose: false,
            dev: false,
            features: Vec::new(),
            use_default_features: true,
            all_features: false,
            cache: None,
            analytics: false,
            linkage_default: None,
            linkage_overrides: std::collections::HashMap::new(),
            linkage_on_shared_default: None,
            linkage_on_shared_overrides: std::collections::HashMap::new(),
            linkage_on_static_default: None,
            linkage_on_static_overrides: std::collections::HashMap::new(),
        }
    }
}

/// Build context containing project configuration and build options
#[derive(Debug)]
pub struct BuildContext {
    /// Project root directory (where CCGO.toml is located)
    pub project_root: PathBuf,
    /// Loaded project configuration
    pub config: CcgoConfig,
    /// Build options
    pub options: BuildOptions,
    /// CMake build directory (cmake_build/{debug|release}/<platform>)
    pub cmake_build_dir: PathBuf,
    /// Output directory for final artifacts (target/<platform>)
    pub output_dir: PathBuf,
    /// Git version information
    pub git_version: Option<crate::utils::git_version::GitVersion>,
}

impl BuildContext {
    /// Create a new build context
    pub fn new(project_root: PathBuf, config: CcgoConfig, options: BuildOptions) -> Self {
        // Convert platform name to lowercase for consistent directory structure
        let platform_name = options.target.to_string().to_lowercase();

        // Both cmake_build and target use release/debug subdirectory for consistency:
        // cmake_build/release/android/ or cmake_build/debug/android/
        // target/release/android/ or target/debug/android/
        let release_subdir = if options.release { "release" } else { "debug" };
        let cmake_build_dir = project_root
            .join("cmake_build")
            .join(release_subdir)
            .join(&platform_name);
        let output_dir = project_root
            .join("target")
            .join(release_subdir)
            .join(&platform_name);

        // Get package info (required for builds)
        let package = config
            .package
            .as_ref()
            .expect("Build requires [package] section in CCGO.toml");

        // Calculate git version information (ignore errors - continue without git info)
        let git_version = crate::utils::git_version::GitVersion::from_project_root(
            &project_root,
            &package.version,
        )
        .ok();

        // If [build].verinfo_path is set, regenerate the ccgo verinfo
        // translation unit under cmake_build/ccgo_generated/ with the
        // current build identity before the C/C++ build starts. Best-
        // effort — skipping on failure so verinfo trouble can't block builds.
        if let Some(header_rel) = config
            .build
            .as_ref()
            .and_then(|b| b.verinfo_path.as_deref())
        {
            if let Some(gv) = git_version.as_ref() {
                let identity = gv.veridentity(&package.version);
                let _ = verinfo::generate(&project_root, header_rel, &package.name, &identity);
            }
        }

        Self {
            project_root,
            config,
            options,
            cmake_build_dir,
            output_dir,
            git_version,
        }
    }

    /// Get the library name from config
    pub fn lib_name(&self) -> &str {
        &self
            .config
            .package
            .as_ref()
            .expect("Build requires [package] section")
            .name
    }

    /// Get the include source directory for SDK packaging.
    /// Uses `[include].src` from CCGO.toml if configured; otherwise falls back to `include/`.
    pub fn include_source_dir(&self) -> PathBuf {
        if let Some(src) = self.config.include.as_ref().and_then(|c| c.src.as_deref()) {
            self.project_root.join(src)
        } else {
            self.project_root.join("include")
        }
    }

    /// Get the version string
    pub fn version(&self) -> &str {
        &self
            .config
            .package
            .as_ref()
            .expect("Build requires [package] section")
            .version
    }

    /// Publish suffix (e.g., "beta.18-dirty").
    ///
    /// Plan A (Cargo-aligned): always empty. The version comes solely from
    /// `[package].version` in CCGO.toml; git state is not folded into it.
    /// Retained as a `&str` API so filename templates can pass it through
    /// unchanged — ArchiveBuilder & friends already short-circuit on empty.
    pub fn publish_suffix(&self) -> &str {
        ""
    }

    /// Get the full version (same as `version()` under Plan A — no suffix
    /// is ever appended). Name retained for API compatibility.
    pub fn version_with_suffix(&self) -> &str {
        self.version()
    }

    /// Get the number of parallel jobs (default to CPU count)
    pub fn jobs(&self) -> usize {
        self.options.jobs.unwrap_or_else(num_cpus)
    }

    /// Get the CCGO_CONFIG_DEPS_MAP string for CMake
    ///
    /// Format: "module1;dep1,dep2;module2;dep3"
    /// This tells CMake which submodules depend on which other submodules
    /// for proper shared library linking.
    pub fn deps_map(&self) -> Option<String> {
        let submodule_deps = self
            .config
            .build
            .as_ref()
            .map(|b| &b.submodule_deps)
            .filter(|deps| !deps.is_empty())?;

        let mut deps_list = Vec::new();
        for (module, deps) in submodule_deps {
            if !deps.is_empty() {
                deps_list.push(module.clone());
                deps_list.push(deps.join(","));
            }
        }

        if deps_list.is_empty() {
            None
        } else {
            Some(deps_list.join(";"))
        }
    }

    /// Get the symbol visibility setting (0 = hidden, 1 = default)
    pub fn symbol_visibility(&self) -> u8 {
        if self
            .config
            .build
            .as_ref()
            .map(|b| b.symbol_visibility)
            .unwrap_or(false)
        {
            1
        } else {
            0
        }
    }

    /// Extract the three global-platform linkage fields from the platform-specific
    /// config section (e.g. `[platforms.android]`). Returns `(build_as_hint,
    /// default_hint)` where `build_as_hint` is the consumer-specific field and
    /// `default_hint` is `default_dep_linkage`.
    fn global_platform_linkage(
        &self,
        platform: &str,
        consumer: &crate::commands::build::LinkType,
    ) -> (Option<crate::config::Linkage>, Option<crate::config::Linkage>) {
        use crate::commands::build::LinkType;
        let p = self.config.platforms.as_ref();
        macro_rules! extract {
            ($cfg:expr) => {{
                let build_as = $cfg.and_then(|c| match consumer {
                    LinkType::Shared => c.dep_linkage_on_shared,
                    LinkType::Static => c.dep_linkage_on_static,
                    _ => None,
                });
                (build_as, $cfg.and_then(|c| c.default_dep_linkage))
            }};
        }
        match platform {
            "android" => extract!(p.and_then(|p| p.android.as_ref())),
            "ios" => extract!(p.and_then(|p| p.ios.as_ref())),
            "macos" => extract!(p.and_then(|p| p.macos.as_ref())),
            "ohos" => extract!(p.and_then(|p| p.ohos.as_ref())),
            "linux" => extract!(p.and_then(|p| p.linux.as_ref())),
            "windows" => extract!(p.and_then(|p| p.windows.as_ref())),
            _ => (None, None),
        }
    }

    /// Resolve and get enabled features based on build options
    ///
    /// Returns the full set of enabled features after resolving:
    /// - Default features (unless --no-default-features)
    /// - Explicitly requested features (--features)
    /// - All features (if --all-features)
    pub fn resolved_features(&self) -> Result<HashSet<String>> {
        let features_config = &self.config.features;

        if self.options.all_features {
            // Enable all available features
            let mut all = HashSet::new();
            for name in features_config.feature_names() {
                features_config.resolve_feature(name, &mut all)?;
            }
            // Also include default features
            features_config.resolve_feature("default", &mut all)?;
            Ok(all)
        } else {
            // Resolve requested features with/without defaults
            features_config
                .resolve_features(&self.options.features, self.options.use_default_features)
        }
    }

    /// Get CMake feature definitions as a semicolon-separated list
    ///
    /// Returns a string like "CCGO_FEATURE_NETWORKING;CCGO_FEATURE_ADVANCED"
    /// that can be passed to CMake as compile definitions.
    pub fn cmake_feature_defines(&self) -> Result<String> {
        let resolved = self.resolved_features()?;
        let defines: Vec<String> = resolved
            .iter()
            .filter(|f| !f.contains('/')) // Skip dep/feature syntax
            .map(|f| format!("CCGO_FEATURE_{}", f.to_uppercase().replace('-', "_")))
            .collect();
        Ok(defines.join(";"))
    }

    /// Get enabled dependencies (non-optional + enabled optional deps)
    pub fn enabled_dependencies(&self) -> Result<Vec<&crate::config::DependencyConfig>> {
        let resolved = self.resolved_features()?;
        Ok(self
            .config
            .features
            .get_enabled_optional_deps(&resolved, &self.config.dependencies))
    }

    /// Get the CCGO cmake directory path
    ///
    /// Searches in the following order:
    /// 1. Embedded CMake templates (extracted to ~/.ccgo/cmake/)
    /// 2. CCGO_CMAKE_DIR environment variable
    /// 3. Relative to ccgo-rs binary (for development)
    /// 4. Installed ccgo package location
    pub fn ccgo_cmake_dir(&self) -> Option<PathBuf> {
        // 1. Use embedded CMake templates (primary method)
        if let Ok(cmake_dir) = cmake_templates::get_cmake_dir() {
            return Some(cmake_dir);
        }

        // 2. Check environment variable (fallback)
        if let Ok(dir) = std::env::var("CCGO_CMAKE_DIR") {
            let path = PathBuf::from(dir);
            if path.exists() {
                return Some(path);
            }
        }

        // 3. Check relative to current executable (development mode)
        if let Ok(exe) = std::env::current_exe() {
            // Go up from ccgo-rs/target/debug/ccgo to ccgo-rs/../ccgo/ccgo/build_scripts/cmake
            if let Some(ccgo_rs_root) = exe
                .parent()
                .and_then(|p| p.parent())
                .and_then(|p| p.parent())
            {
                let cmake_dir = ccgo_rs_root
                    .parent()
                    .map(|p| p.join("ccgo/ccgo/build_scripts/cmake"));
                if let Some(ref dir) = cmake_dir {
                    if dir.exists() {
                        return cmake_dir;
                    }
                }
            }
        }

        // 4. Try to find installed ccgo package via pip (last resort)
        if let Ok(output) = std::process::Command::new("pip3")
            .args(["show", "-f", "ccgo"])
            .output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Parse "Location: /path/to/site-packages"
                for line in stdout.lines() {
                    if let Some(location) = line.strip_prefix("Location: ") {
                        let cmake_dir =
                            PathBuf::from(location.trim()).join("ccgo/build_scripts/cmake");
                        if cmake_dir.exists() {
                            return Some(cmake_dir);
                        }
                    }
                }
            }
        }

        None
    }

    /// Get compiler cache configuration based on build options
    ///
    /// Returns None if caching is disabled or not available
    pub fn compiler_cache(&self) -> Option<cache::CacheConfig> {
        let cache_option = self.options.cache.as_ref()?;

        match cache_option.as_str() {
            "none" | "disabled" | "off" => None,
            "auto" => cache::CacheConfig::auto().ok(),
            "ccache" => cache::CacheConfig::with_type(cache::CacheType::CCache).ok(),
            "sccache" => cache::CacheConfig::with_type(cache::CacheType::SCache).ok(),
            _ => {
                eprintln!(
                    "⚠️  Unknown cache type '{}', using auto-detection",
                    cache_option
                );
                cache::CacheConfig::auto().ok()
            }
        }
    }

    /// Get CCGO.toml configuration hash
    pub fn config_hash(&self) -> Result<String> {
        let config_path = self.project_root.join("CCGO.toml");
        incremental::BuildState::hash_config(&config_path)
    }

    /// Get build options hash
    pub fn options_hash(&self) -> String {
        // Create a canonical string representation of build options
        let options_str = format!(
            "target={:?},arch={:?},link={:?},release={},jobs={:?},features={:?},cache={:?}",
            self.options.target,
            self.options.architectures,
            self.options.link_type,
            self.options.release,
            self.options.jobs,
            self.options.features,
            self.options.cache
        );
        incremental::BuildState::hash_options(&options_str)
    }

    /// Resolve the linkage for every entry in `[[dependencies]]` against the
    /// dep's already-fetched artifacts in `.ccgo/deps/<name>/`. Returns
    /// `(name, ResolvedLinkage)` pairs for CMake to consume.
    pub fn resolved_dep_linkages(
        &self,
        platform: &str,
    ) -> anyhow::Result<Vec<(String, crate::build::linkage::ResolvedLinkage)>> {
        use crate::build::linkage::{detect_dep_artifacts, resolve_linkage};

        // Normalize platform to lowercase: artifact paths (.ccgo/deps/<name>/lib/<platform>/)
        // are always lowercase, but PlatformBuilder::platform_name() returns
        // "Android" (capital A) for historical reasons. Lowercase here so the
        // helper is resilient to whatever the caller passes.
        let platform = platform.to_lowercase();

        // Warn about CLI per-dep overrides that reference a dep that isn't
        // declared in [[dependencies]] — likely a user typo. Don't fail; the
        // override just won't apply, same as cargo's behavior on unknown
        // --features.
        let declared: std::collections::HashSet<&str> = self
            .config
            .dependencies
            .iter()
            .map(|d| d.name.as_str())
            .collect();
        for name in self.options.linkage_overrides.keys() {
            if !declared.contains(name.as_str()) {
                eprintln!(
                    "warning: --linkage {name}=<...> referenced an unknown dependency. \
                     '{name}' is not declared in [[dependencies]]; the override is ignored. \
                     Declared deps: {}",
                    if declared.is_empty() {
                        "(none)".to_string()
                    } else {
                        let mut names: Vec<&&str> = declared.iter().collect();
                        names.sort();
                        names
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                );
            }
        }

        let consumer = self.options.link_type.preferred_single();
        let mut out = Vec::new();
        for dep in &self.config.dependencies {
            let dep_root = self.project_root.join(".ccgo/deps").join(&dep.name);
            let artifacts = detect_dep_artifacts(&dep_root, &platform);
            let hint = self.resolved_linkage_hint(dep, &platform, &consumer);
            let resolved = resolve_linkage(consumer.clone(), artifacts, hint, &dep.name)?;
            out.push((dep.name.clone(), resolved));
        }
        Ok(out)
    }

    /// Resolve the linkage hint for a single dep using the canonical 8-tier
    /// precedence chain. Centralises the rule so [`Self::resolved_dep_linkages`]
    /// (which feeds the resolver) and [`Self::materialize_source_deps`] (which
    /// feeds the recursive build) cannot drift.
    ///
    /// Precedence (highest to lowest, after CLI overrides):
    ///   1. CLI per-dep (`--linkage <name>=<value>`)
    ///   2. CLI default (`--linkage <value>`)
    ///   3. dep.{platform}.linkage_on_{shared|static}  (platform + build_as + dep)
    ///   4. dep.linkage_on_{shared|static}              (build_as + dep)
    ///   5. dep.{platform}.linkage                      (platform + dep)
    ///   6. dep.linkage                                 (dep only)
    ///   7. [platforms.X].dep_linkage_on_{shared|static} (global: platform + build_as)
    ///   8. [build].dep_linkage_on_{shared|static}       (global: build_as)
    ///   9. [platforms.X].default_dep_linkage             (global: platform)
    ///  10. [build].default_dep_linkage                   (global default)
    ///  11. None → resolver picks based on artifacts
    ///
    /// CLI `--linkage-on-shared/static` flags mirror the TOML `linkage_on_shared/static`
    /// fields but sit above their TOML counterparts in priority:
    ///   - `--linkage-on-shared <name>=<value>` beats tier 3
    ///   - `--linkage-on-shared <value>`        beats tier 4
    fn resolved_linkage_hint(
        &self,
        dep: &crate::config::DependencyConfig,
        platform: &str,
        consumer: &crate::commands::build::LinkType,
    ) -> Option<crate::config::Linkage> {
        use crate::commands::build::LinkType;

        let plat_dep = dep.platform_config(platform);
        let (global_plat_build_as, global_plat_default) =
            self.global_platform_linkage(platform, consumer);
        let build_cfg = self.config.build.as_ref();

        let global_build_as = build_cfg.and_then(|b| match consumer {
            LinkType::Shared => b.dep_linkage_on_shared,
            LinkType::Static => b.dep_linkage_on_static,
            _ => None,
        });
        let global_default = build_cfg.and_then(|b| b.default_dep_linkage);

        let plat_dep_build_as = plat_dep.and_then(|p| match consumer {
            LinkType::Shared => p.linkage_on_shared,
            LinkType::Static => p.linkage_on_static,
            _ => None,
        });
        let dep_build_as = match consumer {
            LinkType::Shared => dep.linkage_on_shared,
            LinkType::Static => dep.linkage_on_static,
            _ => None,
        };

        // CLI build-as-specific overrides (above toml per-dep)
        let (cli_build_as_override, cli_build_as_default) = match consumer {
            LinkType::Shared => (
                self.options
                    .linkage_on_shared_overrides
                    .get(&dep.name)
                    .copied(),
                self.options.linkage_on_shared_default,
            ),
            LinkType::Static => (
                self.options
                    .linkage_on_static_overrides
                    .get(&dep.name)
                    .copied(),
                self.options.linkage_on_static_default,
            ),
            _ => (None, None),
        };

        self.options
            .linkage_overrides
            .get(&dep.name)
            .copied()
            .or(self.options.linkage_default)
            .or(cli_build_as_override)
            .or(cli_build_as_default)
            .or(plat_dep_build_as)
            .or(dep_build_as)
            .or_else(|| plat_dep.and_then(|p| p.linkage))
            .or(dep.linkage)
            .or(global_plat_build_as)
            .or(global_build_as)
            .or(global_plat_default)
            .or(global_default)
    }

    /// Resolve the linkage hint for materialize (pre-build of source deps).
    ///
    /// `materialize_source_deps` builds dep artifacts without knowing which
    /// build-as the final consumer will use. If both Shared and Static consumers
    /// would select the same hint, use it (avoids building artifacts we won't
    /// need). If they differ, return `None` so the materializer builds both
    /// artifact shapes.
    fn resolved_materialize_hint(
        &self,
        dep: &crate::config::DependencyConfig,
        platform: &str,
    ) -> Option<crate::config::Linkage> {
        use crate::commands::build::LinkType;
        let shared_hint = self.resolved_linkage_hint(dep, platform, &LinkType::Shared);
        let static_hint = self.resolved_linkage_hint(dep, platform, &LinkType::Static);
        if shared_hint == static_hint {
            shared_hint
        } else {
            None
        }
    }

    /// Run the source-only-dep materialize pass: for any dep that ships
    /// source code we can rebuild from, spawn `ccgo build` inside it when
    /// its artifacts are missing OR the source has changed since the last
    /// successful materialize. Skips deps whose source fingerprint matches
    /// the persisted sidecar AND whose `lib/<platform>/` already has
    /// artifacts on disk.
    pub fn materialize_source_deps(&self, platform: &str) -> anyhow::Result<()> {
        use crate::build::materialize::{global_fingerprint_cache, materialize_source_deps_inner};

        let dep_hints: Vec<(String, Option<crate::config::Linkage>)> = self
            .config
            .dependencies
            .iter()
            .map(|d| (d.name.clone(), self.resolved_materialize_hint(d, platform)))
            .collect();

        // `BuildOptions::architectures` is already lowercased by
        // `parse_arch_arg` at parse time, but `materialize_source_deps_inner`
        // requires lowercase for fingerprint-key consistency. Re-applying
        // is cheap and pins the invariant locally so a future change to
        // `parse_arch_arg` can't silently corrupt cache keys.
        let archs: Vec<String> = self
            .options
            .architectures
            .iter()
            .map(|a| a.to_lowercase())
            .collect();

        let ccgo_bin =
            std::env::current_exe().context("failed to resolve current ccgo binary path")?;

        materialize_source_deps_inner(
            &self.project_root,
            platform,
            &archs,
            self.options.release,
            &dep_hints,
            ccgo_bin
                .to_str()
                .context("ccgo binary path is not UTF-8")?,
            &global_fingerprint_cache(),
        )
    }

    /// Return merged cmake user config: global `[build.cmake]` + platform
    /// `[platforms.X.build.cmake]`. All three lists are concatenated (global first).
    pub fn cmake_user_config(&self, platform: &str) -> crate::config::CmakeUserConfig {
        use crate::config::CmakeUserConfig;

        let global = self
            .config
            .build
            .as_ref()
            .and_then(|b| b.cmake.clone())
            .unwrap_or_default();

        let p = self.config.platforms.as_ref();
        let platform_cfg: CmakeUserConfig = match platform.to_lowercase().as_str() {
            "android" => p
                .and_then(|p| p.android.as_ref())
                .and_then(|a| a.build.as_ref())
                .and_then(|b| b.cmake.clone())
                .unwrap_or_default(),
            "ios" => p
                .and_then(|p| p.ios.as_ref())
                .and_then(|a| a.build.as_ref())
                .and_then(|b| b.cmake.clone())
                .unwrap_or_default(),
            "macos" => p
                .and_then(|p| p.macos.as_ref())
                .and_then(|a| a.build.as_ref())
                .and_then(|b| b.cmake.clone())
                .unwrap_or_default(),
            "windows" => p
                .and_then(|p| p.windows.as_ref())
                .and_then(|a| a.build.as_ref())
                .and_then(|b| b.cmake.clone())
                .unwrap_or_default(),
            "linux" => p
                .and_then(|p| p.linux.as_ref())
                .and_then(|a| a.build.as_ref())
                .and_then(|b| b.cmake.clone())
                .unwrap_or_default(),
            "ohos" => p
                .and_then(|p| p.ohos.as_ref())
                .and_then(|a| a.build.as_ref())
                .and_then(|b| b.cmake.clone())
                .unwrap_or_default(),
            _ => CmakeUserConfig::default(),
        };

        CmakeUserConfig {
            arguments: [global.arguments, platform_cfg.arguments].concat(),
            c_flags: [global.c_flags, platform_cfg.c_flags].concat(),
            cpp_flags: [global.cpp_flags, platform_cfg.cpp_flags].concat(),
        }
    }

    /// Create incremental build analyzer for a specific link type
    pub fn create_incremental_analyzer(
        &self,
        link_type: &str,
    ) -> Result<incremental::IncrementalAnalyzer> {
        let config_hash = self.config_hash()?;
        let options_hash = self.options_hash();

        incremental::IncrementalAnalyzer::new(
            &self.cmake_build_dir,
            self.lib_name().to_string(),
            self.options.target.to_string(),
            link_type.to_string(),
            config_hash,
            options_hash,
        )
    }
}

/// Get number of CPUs for parallel builds
fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(4)
}

/// Build information stored in build_info.json
///
/// Field names are aligned with Python ccgo for compatibility:
/// - `project` (not `name`) - Project name (lowercase)
/// - `build_time` (not `timestamp`) - ISO 8601 with microseconds, local time
/// - `build_host` - Build host OS (Darwin, Linux, Windows)
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildInfo {
    /// Project name (lowercase) - renamed from 'name' to match Python ccgo
    pub project: String,
    /// Platform name (linux, windows, macos, ios, android, ohos, etc.)
    pub platform: String,
    /// Version string
    pub version: String,
    /// Link type (static, shared, both)
    pub link_type: String,
    /// Build timestamp (ISO 8601 with microseconds, local time) - renamed from 'timestamp'
    pub build_time: String,
    /// Build host OS (Darwin, Linux, Windows, etc.)
    pub build_host: String,
    /// Architectures built
    pub architectures: Vec<String>,
    /// Toolchain used (optional, e.g., "msvc", "mingw")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolchain: Option<String>,
    /// Git commit hash (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_commit: Option<String>,
    /// Git branch (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

/// Comprehensive build information matching pyccgo format
///
/// This is the full JSON structure that matches Python ccgo's build_info output:
/// - build_metadata: version, generated_at, generator
/// - project: name, version
/// - git: branch, revision, revision_full, tag, is_dirty, remote_url
/// - build: time, timestamp, platform, and platform-specific info
/// - environment: os, os_version, python_version, ccgo_version
#[derive(Debug, Serialize, Deserialize)]
pub struct BuildInfoFull {
    pub build_metadata: BuildMetadata,
    pub project: ProjectInfo,
    pub git: GitInfo,
    pub build: BuildDetails,
    pub environment: EnvironmentInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildMetadata {
    pub version: String,
    pub generated_at: String,
    pub generator: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GitInfo {
    pub branch: String,
    pub revision: String,
    pub revision_full: String,
    pub tag: String,
    pub is_dirty: bool,
    pub remote_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildDetails {
    pub time: String,
    pub timestamp: i64,
    pub platform: String,
    /// Platform-specific build info (ios, android, etc.)
    #[serde(flatten)]
    pub platform_info: Option<PlatformBuildInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlatformBuildInfo {
    Ios { ios: IosBuildInfo },
    Android { android: AndroidBuildInfo },
    Macos { macos: MacosBuildInfo },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IosBuildInfo {
    pub xcode_version: String,
    pub xcode_build: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AndroidBuildInfo {
    pub ndk_version: String,
    pub stl: String,
    pub min_sdk_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MacosBuildInfo {
    pub xcode_version: String,
    pub xcode_build: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub os: String,
    pub os_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub python_version: Option<String>,
    pub ccgo_version: String,
}

/// Trait for platform-specific builders
pub trait PlatformBuilder {
    /// Get the platform name
    fn platform_name(&self) -> &str;

    /// Get default architectures for this platform
    fn default_architectures(&self) -> Vec<String>;

    /// Validate build prerequisites (toolchains, SDKs, etc.)
    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()>;

    /// Execute the build
    fn build(&self, ctx: &BuildContext) -> Result<BuildResult>;

    /// Clean build artifacts
    fn clean(&self, ctx: &BuildContext) -> Result<()>;
}

/// Result of a successful build
#[derive(Debug)]
pub struct BuildResult {
    /// Path to the main SDK archive
    pub sdk_archive: PathBuf,
    /// Path to the symbols archive (if generated)
    pub symbols_archive: Option<PathBuf>,
    /// Path to the AAR archive (Android only, if generated)
    pub aar_archive: Option<PathBuf>,
    /// Build duration in seconds
    pub duration_secs: f64,
    /// Architectures that were built
    pub architectures: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::build::LinkType;
    use crate::config::{BuildConfig, CcgoConfig, DependencyConfig, Linkage, PlatformLinkageConfig};

    fn make_ctx(options: BuildOptions, config: CcgoConfig) -> BuildContext {
        BuildContext {
            project_root: PathBuf::from("/tmp/test"),
            config,
            options,
            cmake_build_dir: PathBuf::from("/tmp/test/cmake_build"),
            output_dir: PathBuf::from("/tmp/test/target"),
            git_version: None,
        }
    }

    fn bare_dep(name: &str) -> DependencyConfig {
        DependencyConfig {
            name: name.to_string(),
            ..Default::default()
        }
    }

    fn bare_config() -> CcgoConfig {
        toml::from_str("").expect("empty toml should parse")
    }

    fn bare_options() -> BuildOptions {
        BuildOptions::default()
    }

    // --- tier 6: dep.linkage (dep only) ---

    #[test]
    fn dep_linkage_field_used_when_no_platform_or_build_as() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::StaticEmbedded);
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::StaticEmbedded)
        );
    }

    // --- tier 4: dep.linkage_on_shared / dep.linkage_on_static ---

    #[test]
    fn dep_linkage_on_shared_beats_dep_linkage() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::SharedExternal);
        dep.linkage_on_shared = Some(Linkage::StaticEmbedded);
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::StaticEmbedded)
        );
        // Static consumer still falls back to dep.linkage
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Static),
            Some(Linkage::SharedExternal)
        );
    }

    #[test]
    fn dep_linkage_on_static_beats_dep_linkage() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::SharedExternal);
        dep.linkage_on_static = Some(Linkage::StaticExternal);
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Static),
            Some(Linkage::StaticExternal)
        );
    }

    // --- tier 5 & 3: dep.{platform}.linkage / dep.{platform}.linkage_on_X ---

    #[test]
    fn platform_dep_linkage_beats_bare_dep_linkage() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::SharedExternal);
        dep.android = Some(PlatformLinkageConfig {
            linkage: Some(Linkage::StaticEmbedded),
            linkage_on_shared: None,
            linkage_on_static: None,
        });
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "android", &LinkType::Shared),
            Some(Linkage::StaticEmbedded),
            "android.linkage should win over dep.linkage for android platform"
        );
        // Non-android platform falls back to dep.linkage
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "ios", &LinkType::Shared),
            Some(Linkage::SharedExternal),
        );
    }

    #[test]
    fn platform_dep_build_as_beats_platform_dep_linkage() {
        let mut dep = bare_dep("leaf");
        dep.android = Some(PlatformLinkageConfig {
            linkage: Some(Linkage::SharedExternal),
            linkage_on_shared: Some(Linkage::StaticEmbedded),
            linkage_on_static: None,
        });
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "android", &LinkType::Shared),
            Some(Linkage::StaticEmbedded),
            "android.linkage_on_shared beats android.linkage"
        );
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "android", &LinkType::Static),
            Some(Linkage::SharedExternal),
            "android.linkage used when linkage_on_static is absent"
        );
    }

    // --- tier 10: [build].default_dep_linkage (global default) ---

    #[test]
    fn global_build_default_used_when_nothing_else_set() {
        let dep = bare_dep("leaf");
        let mut config = bare_config();
        config.build = Some(BuildConfig {
            default_dep_linkage: Some(Linkage::StaticExternal),
            ..Default::default()
        });
        let ctx = make_ctx(bare_options(), config);
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::StaticExternal)
        );
    }

    // --- tier 8: [build].dep_linkage_on_shared / dep_linkage_on_static ---

    #[test]
    fn global_build_build_as_beats_global_build_default() {
        let dep = bare_dep("leaf");
        let mut config = bare_config();
        config.build = Some(BuildConfig {
            default_dep_linkage: Some(Linkage::SharedExternal),
            dep_linkage_on_shared: Some(Linkage::StaticEmbedded),
            dep_linkage_on_static: None,
            ..Default::default()
        });
        let ctx = make_ctx(bare_options(), config);
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::StaticEmbedded)
        );
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Static),
            Some(Linkage::SharedExternal)
        );
    }

    // --- CLI overrides beat everything ---

    #[test]
    fn cli_per_dep_override_beats_all_toml() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::SharedExternal);
        dep.android = Some(PlatformLinkageConfig {
            linkage: Some(Linkage::StaticEmbedded),
            linkage_on_shared: Some(Linkage::StaticEmbedded),
            linkage_on_static: Some(Linkage::StaticEmbedded),
        });
        let mut options = bare_options();
        options
            .linkage_overrides
            .insert("leaf".to_string(), Linkage::StaticExternal);
        let ctx = make_ctx(options, bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "android", &LinkType::Shared),
            Some(Linkage::StaticExternal)
        );
    }

    // --- CLI --linkage-on-shared / --linkage-on-static ---

    #[test]
    fn cli_linkage_on_shared_default_beats_toml_dep_linkage_for_shared_consumer() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::SharedExternal);
        let mut options = bare_options();
        options.linkage_on_shared_default = Some(Linkage::StaticEmbedded);
        let ctx = make_ctx(options, bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::StaticEmbedded),
            "--linkage-on-shared default should beat dep.linkage for shared consumer"
        );
        // Static consumer unaffected — falls through to dep.linkage
        let ctx2 = make_ctx(
            {
                let mut o = bare_options();
                o.linkage_on_shared_default = Some(Linkage::StaticEmbedded);
                o
            },
            bare_config(),
        );
        assert_eq!(
            ctx2.resolved_linkage_hint(&dep, "linux", &LinkType::Static),
            Some(Linkage::SharedExternal),
        );
    }

    #[test]
    fn cli_linkage_on_shared_per_dep_beats_default() {
        let dep = bare_dep("leaf");
        let mut options = bare_options();
        options.linkage_on_shared_default = Some(Linkage::SharedExternal);
        options
            .linkage_on_shared_overrides
            .insert("leaf".to_string(), Linkage::StaticEmbedded);
        let ctx = make_ctx(options, bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::StaticEmbedded),
            "--linkage-on-shared leaf=X should beat --linkage-on-shared default"
        );
    }

    #[test]
    fn cli_linkage_on_static_default_beats_toml_dep_linkage_for_static_consumer() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::SharedExternal);
        let mut options = bare_options();
        options.linkage_on_static_default = Some(Linkage::StaticExternal);
        let ctx = make_ctx(options, bare_config());
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Static),
            Some(Linkage::StaticExternal)
        );
    }

    #[test]
    fn cli_bare_linkage_still_beats_cli_build_as_linkage() {
        let dep = bare_dep("leaf");
        let mut options = bare_options();
        options.linkage_default = Some(Linkage::SharedExternal);
        options.linkage_on_shared_default = Some(Linkage::StaticEmbedded);
        let ctx = make_ctx(options, bare_config());
        // --linkage (bare) sits above --linkage-on-shared in priority
        assert_eq!(
            ctx.resolved_linkage_hint(&dep, "linux", &LinkType::Shared),
            Some(Linkage::SharedExternal),
        );
    }

    // --- resolved_materialize_hint ---

    #[test]
    fn materialize_hint_returns_hint_when_shared_static_agree() {
        let mut dep = bare_dep("leaf");
        dep.linkage = Some(Linkage::StaticEmbedded);
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(
            ctx.resolved_materialize_hint(&dep, "linux"),
            Some(Linkage::StaticEmbedded)
        );
    }

    #[test]
    fn materialize_hint_returns_none_when_shared_static_differ() {
        let mut dep = bare_dep("leaf");
        dep.linkage_on_shared = Some(Linkage::SharedExternal);
        dep.linkage_on_static = Some(Linkage::StaticEmbedded);
        let ctx = make_ctx(bare_options(), bare_config());
        assert_eq!(ctx.resolved_materialize_hint(&dep, "linux"), None);
    }
}
