//! Install the current project into the global CCGO package cache.
//!
//! This is the analog of `cargo install --path .` / `mvn install` / `npm link`
//! for C/C++ SDK libraries built with ccgo. After running `ccgo install` in a
//! project directory, the package is available to any other ccgo project on
//! the machine via a plain `name + version` dependency declaration — no git
//! URL, no relative path, no external registry required.
//!
//! Semantics (new in v3.5):
//! * `ccgo fetch`   — installs the *current project's dependencies* into
//!   `.ccgo/deps/` (what `ccgo install` used to do).
//! * `ccgo install` — installs the *current project itself* into
//!   `$CCGO_HOME/packages/<name>/<version>/`.
//!
//! The install location honors `$CCGO_HOME` (falling back to `~/.ccgo`).
//!
//! Example:
//! ```bash
//! cd mna-stdcomm/stdcomm
//! ccgo build all --release
//! ccgo install          # → ~/.ccgo/packages/stdcomm/25.2.9519653/
//!
//! # logcomm/CCGO.toml:
//! #   [[dependencies]]
//! #   name = "stdcomm"
//! #   version = "25.2.9519653"
//! cd mna-logcomm/logcomm && ccgo fetch
//! ```

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use clap::Args;
use console::style;

use crate::commands::package::{install_to_local_cache, PackageCommand};
use crate::commands::run::RunCommand;
use crate::config::{BinConfig, CcgoConfig};

/// Install the current project into the global CCGO package cache.
#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
pub struct InstallCommand {
    /// Install the debug build. Default is release — matches
    /// `cargo install`, which also defaults to release since the output is
    /// intended for actual use rather than iterative development.
    #[arg(long)]
    pub debug: bool,

    /// List installed packages under $CCGO_HOME/packages/ and bins under
    /// $CCGO_HOME/bin/ and exit. Analogous to `cargo install --list`.
    #[arg(long)]
    pub list: bool,

    /// Explicit version to install as. Defaults to the version auto-detected
    /// by `ccgo package` (git describe / CCGO.toml).
    #[arg(long)]
    pub version: Option<String>,

    /// Force reinstall even if the target version is already present.
    #[arg(long)]
    pub force: bool,

    /// If `target/<mode>/package/` has no output zip yet, automatically run
    /// `ccgo package` to produce one instead of failing. Enabled by default;
    /// pass `--no-auto-package` to disable.
    #[arg(long = "no-auto-package", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub auto_package: bool,

    /// Comma-separated platforms to include when auto-packaging.
    #[arg(long)]
    pub platforms: Option<String>,

    /// Install only the named binary (from `[[bin]]`). Repeatable.
    /// When any `--bin` is given, only those bins are installed and lib
    /// installation is skipped unless `--lib` is also passed.
    #[arg(long, value_name = "NAME")]
    pub bin: Vec<String>,

    /// Install every `[[bin]]` entry; skip the lib unless `--lib` is also set.
    #[arg(long, conflicts_with = "bin")]
    pub bins: bool,

    /// Install the library portion (include/, lib/). Default when no `--bin`
    /// or `--bins` flag is given; required to include lib alongside bins.
    #[arg(long)]
    pub lib: bool,
}

impl InstallCommand {
    pub fn execute(self, verbose: bool) -> Result<()> {
        // --list short-circuits everything else.
        if self.list {
            return list_installed_packages();
        }

        println!("{}", "=".repeat(80));
        println!("CCGO Install - Current project → global package cache");
        println!("{}", "=".repeat(80));

        let ctx = InstallContext::load()?;
        let release = !self.debug;
        let package_output = ctx
            .project_root
            .join("target")
            .join(if release { "release" } else { "debug" })
            .join("package");

        self.ensure_packaged(&ctx, &package_output, release, verbose)?;

        let zip_prefix = format!("{}_CCGO_PACKAGE-", ctx.project_name.to_uppercase());
        let version_clean =
            self.resolve_version(&package_output, &zip_prefix, &ctx.package_version);

        let plan = self.plan_install(&ctx.bins);
        self.install_lib_part(&ctx, &package_output, &version_clean, &plan)?;
        self.install_bins_part(&ctx, &version_clean, &plan, release, verbose)?;

        Ok(())
    }

    /// Guarantee `target/<mode>/package/<NAME>_CCGO_PACKAGE-*.zip` exists,
    /// invoking `ccgo package` when absent and the caller allows auto-packaging.
    fn ensure_packaged(
        &self,
        ctx: &InstallContext,
        package_output: &Path,
        release: bool,
        verbose: bool,
    ) -> Result<()> {
        let zip_prefix = format!("{}_CCGO_PACKAGE-", ctx.project_name.to_uppercase());
        if find_sdk_zip(package_output, &zip_prefix).is_some() {
            return Ok(());
        }

        let pkg_flag = if release { " --release" } else { "" };
        if !self.auto_package {
            return Err(anyhow!(
                "No packaged SDK found at {}.\n\
                 Run `ccgo package{}` first, or re-run with --auto-package.",
                package_output.display(),
                pkg_flag
            ));
        }
        println!("📦 No packaged SDK found, running `ccgo package{pkg_flag}` first…");
        PackageCommand {
            version: self.version.clone(),
            output: None,
            platforms: self.platforms.clone(),
            no_merge: false,
            release,
            dist_branch: None,
            dist: false,
            dist_push: false,
        }
        .execute(verbose)
    }

    /// Prefer explicit `--version`, else parse the produced zip filename, else
    /// fall back to CCGO.toml's `[package].version`. Strips a leading `v`.
    fn resolve_version(
        &self,
        package_output: &Path,
        zip_prefix: &str,
        toml_version: &str,
    ) -> String {
        let version = self
            .version
            .clone()
            .or_else(|| infer_version_from_zip(package_output, zip_prefix))
            .unwrap_or_else(|| toml_version.to_string());
        version.strip_prefix('v').unwrap_or(&version).to_string()
    }

    fn plan_install(&self, bins: &[BinConfig]) -> InstallPlan {
        let has_bin_config = !bins.is_empty();
        let user_selected_bin = !self.bin.is_empty() || self.bins;
        let install_lib = if self.lib { true } else { !user_selected_bin };
        let install_bins = if self.lib && !user_selected_bin {
            false
        } else if user_selected_bin {
            true
        } else {
            has_bin_config
        };
        InstallPlan {
            install_lib,
            install_bins,
            has_bin_config,
        }
    }

    fn install_lib_part(
        &self,
        ctx: &InstallContext,
        package_output: &Path,
        version_clean: &str,
        plan: &InstallPlan,
    ) -> Result<()> {
        if !plan.install_lib {
            return Ok(());
        }
        let dest = cache_path(&ctx.project_name, version_clean)?;
        if dest.exists() && !self.force {
            println!(
                "\n{}",
                style(format!(
                    "ℹ️  Lib already installed: {} {} -> {}",
                    ctx.project_name,
                    version_clean,
                    dest.display()
                ))
                .yellow()
            );
            println!("   Re-run with --force to reinstall.");
            return Ok(());
        }
        install_to_local_cache(
            &ctx.project_root,
            package_output,
            &ctx.project_name,
            version_clean,
        )
    }

    fn install_bins_part(
        &self,
        ctx: &InstallContext,
        version_clean: &str,
        plan: &InstallPlan,
        release: bool,
        verbose: bool,
    ) -> Result<()> {
        if !plan.install_bins {
            return Ok(());
        }
        if !plan.has_bin_config {
            println!(
                "\n{}",
                style("ℹ️  No [[bin]] targets in CCGO.toml — skipping bin install.").yellow()
            );
            return Ok(());
        }
        let selected: Vec<String> = if self.bin.is_empty() {
            ctx.bins.iter().map(|b| b.name.clone()).collect()
        } else {
            self.bin.clone()
        };
        install_bin_targets(
            &ctx.project_root,
            &ctx.project_name,
            version_clean,
            &ctx.bins,
            &selected,
            release,
            self.force,
            verbose,
        )
    }
}

struct InstallContext {
    project_root: PathBuf,
    project_name: String,
    package_version: String,
    bins: Vec<BinConfig>,
}

impl InstallContext {
    fn load() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;
        let config_path = find_ccgo_toml(&cwd)?;
        let project_root = config_path
            .parent()
            .ok_or_else(|| anyhow!("Invalid CCGO.toml path: {}", config_path.display()))?
            .to_path_buf();
        let config = CcgoConfig::load_from_path(&config_path)
            .with_context(|| format!("Failed to load {}", config_path.display()))?;
        let package = config
            .package
            .as_ref()
            .ok_or_else(|| anyhow!("CCGO.toml is missing a [package] section"))?;
        Ok(InstallContext {
            project_root,
            project_name: package.name.clone(),
            package_version: package.version.clone(),
            bins: config.bins.clone(),
        })
    }
}

struct InstallPlan {
    install_lib: bool,
    install_bins: bool,
    has_bin_config: bool,
}

/// Return the first unified SDK zip (not SYMBOLS / ARCHIVE variants) in dir.
fn find_sdk_zip(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let fname = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if fname.starts_with(prefix)
            && fname.ends_with(".zip")
            && !fname.contains("SYMBOLS")
            && !fname.contains("ARCHIVE")
        {
            return Some(path);
        }
    }
    None
}

/// Walk up from `start_dir` looking for a CCGO.toml. Returns its absolute path.
fn find_ccgo_toml(start_dir: &Path) -> Result<PathBuf> {
    let mut cur = start_dir.to_path_buf();
    loop {
        let candidate = cur.join("CCGO.toml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    Err(anyhow!(
        "CCGO.toml not found in current directory or any parent"
    ))
}

fn ccgo_home_dir() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("CCGO_HOME") {
        return Ok(PathBuf::from(custom));
    }
    let home = std::env::var("HOME")
        .map_err(|_| anyhow!("HOME env not set; cannot determine global cache path"))?;
    Ok(PathBuf::from(home).join(".ccgo"))
}

fn cache_path(project_name: &str, version: &str) -> Result<PathBuf> {
    Ok(ccgo_home_dir()?
        .join("packages")
        .join(project_name.to_lowercase())
        .join(version))
}

/// List installed packages under $CCGO_HOME/packages/<name>/<version>/
fn list_installed_libs(packages_root: &Path, any_output: &mut bool) -> Result<()> {
    if !packages_root.is_dir() {
        return Ok(());
    }

    let mut packages: Vec<_> = std::fs::read_dir(packages_root)
        .with_context(|| format!("Failed to read {}", packages_root.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    packages.sort_by_key(|e| e.file_name());

    for pkg_entry in packages {
        let name = pkg_entry.file_name().to_string_lossy().to_string();
        let mut versions: Vec<_> = match std::fs::read_dir(pkg_entry.path()) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .collect(),
            Err(_) => continue,
        };
        versions.sort_by_key(|e| e.file_name());

        for ver_entry in versions {
            let version = ver_entry.file_name().to_string_lossy().to_string();
            let path = ver_entry.path();
            println!(
                "{} {} ({})",
                style(&name).green().bold(),
                style(&version).cyan(),
                path.display()
            );
            describe_package_contents(&path);
            *any_output = true;
        }
    }
    Ok(())
}

/// List stray bins under $CCGO_HOME/bin/
fn list_stray_bins(bin_root: &Path, any_output: &mut bool) -> Result<()> {
    if !bin_root.is_dir() {
        return Ok(());
    }

    let stray_bins: Vec<_> = std::fs::read_dir(bin_root)
        .with_context(|| format!("Failed to read {}", bin_root.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file() || e.path().is_symlink())
        .collect();

    if stray_bins.is_empty() {
        return Ok(());
    }

    println!("\nBins in {}:", bin_root.display());
    for b in stray_bins {
        let name = b.file_name().to_string_lossy().to_string();
        let p = b.path();
        if p.is_symlink() {
            match std::fs::read_link(&p) {
                Ok(target) => println!("    {} → {}", name, target.display()),
                Err(_) => println!("    {}", name),
            }
        } else {
            println!("    {}", name);
        }
        *any_output = true;
    }
    Ok(())
}

/// `ccgo install --list`: enumerate installed packages under
/// $CCGO_HOME/packages/<name>/<version>/ and binaries under $CCGO_HOME/bin/.
fn list_installed_packages() -> Result<()> {
    let home = ccgo_home_dir()?;
    let packages_root = home.join("packages");
    let bin_root = home.join("bin");

    let mut any_output = false;

    list_installed_libs(&packages_root, &mut any_output)?;
    list_stray_bins(&bin_root, &mut any_output)?;

    if !any_output {
        println!("No packages installed under {}.", home.display());
        println!("Run `ccgo install` inside a project to populate this cache.");
    }

    Ok(())
}

/// Print one-line summary of what a package directory contains:
/// "    lib: include/, lib/{android,ios}/" / "    bin: foo, bar".
fn describe_package_contents(pkg_dir: &Path) {
    let mut parts: Vec<String> = Vec::new();

    // Lib portion
    let mut lib_bits: Vec<&str> = Vec::new();
    if pkg_dir.join("include").is_dir() {
        lib_bits.push("include/");
    }
    let lib_dir = pkg_dir.join("lib");
    if lib_dir.is_dir() {
        let platforms: Vec<String> = std::fs::read_dir(&lib_dir)
            .ok()
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .collect()
            })
            .unwrap_or_default();
        if !platforms.is_empty() {
            // Keep output short but informative.
            let mut platforms = platforms;
            platforms.sort();
            lib_bits.push("lib/{");
            let joined = format!("lib/{{{}}}/", platforms.join(","));
            // Replace the placeholder we just pushed.
            lib_bits.pop();
            parts.push(format!("lib: include/, {}", joined));
        } else {
            parts.push(format!("lib: {}", lib_bits.join(", ")));
        }
    } else if !lib_bits.is_empty() {
        parts.push(format!("lib: {}", lib_bits.join(", ")));
    }

    // Bin portion
    let bin_dir = pkg_dir.join("bin");
    if bin_dir.is_dir() {
        if let Ok(rd) = std::fs::read_dir(&bin_dir) {
            let names: Vec<String> = rd
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file() || e.path().is_symlink())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            if !names.is_empty() {
                parts.push(format!("bin: {}", names.join(", ")));
            }
        }
    }

    for p in parts {
        println!("    {}", p);
    }
}

/// Best-effort: read the zip filename to infer the version used by the
/// packaging step. Filename format: `<NAME>_CCGO_PACKAGE-<version>-<suffix>.zip`.
fn infer_version_from_zip(dir: &Path, prefix: &str) -> Option<String> {
    let entries = std::fs::read_dir(dir).ok()?;
    for entry in entries.flatten() {
        let fname = entry.file_name().to_string_lossy().to_string();
        if fname.starts_with(prefix)
            && fname.ends_with(".zip")
            && !fname.contains("SYMBOLS")
            && !fname.contains("ARCHIVE")
        {
            // Strip `<PREFIX>` and `.zip`.
            let tail = &fname[prefix.len()..fname.len() - 4];
            return Some(tail.to_string());
        }
    }
    None
}

/// Compile each requested `[[bin]]` target and install it:
///   1. Binary copied to `$CCGO_HOME/packages/<pkg>/<ver>/bin/<bin-name>`
///   2. Convenience symlink at `$CCGO_HOME/bin/<bin-name>` → (1)
#[allow(clippy::too_many_arguments)]
fn install_bin_targets(
    project_root: &Path,
    pkg_name: &str,
    version: &str,
    all_bins: &[BinConfig],
    selected: &[String],
    release: bool,
    force: bool,
    verbose: bool,
) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Installing bin targets");
    println!("{}", "=".repeat(80));

    let bin_dir_pkg = cache_path(pkg_name, version)?.join("bin");
    std::fs::create_dir_all(&bin_dir_pkg)
        .with_context(|| format!("Failed to create {}", bin_dir_pkg.display()))?;
    let bin_dir_global = ccgo_home_dir()?.join("bin");
    std::fs::create_dir_all(&bin_dir_global)
        .with_context(|| format!("Failed to create {}", bin_dir_global.display()))?;

    for bin_name in selected {
        // Validate the bin is actually declared.
        if !all_bins.iter().any(|b| &b.name == bin_name) {
            return Err(anyhow!(
                "Binary '{}' is not declared in CCGO.toml [[bin]] entries.\n\
                 Available: {}",
                bin_name,
                all_bins
                    .iter()
                    .map(|b| b.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        println!("\n🔨 Building bin: {}", style(bin_name).cyan().bold());

        // Delegate to RunCommand in build-only mode.
        let run_cmd = RunCommand {
            example: None,
            bin: Some(bin_name.clone()),
            release,
            build_only: true,
            jobs: None,
            features: Vec::new(),
            no_default_features: false,
            all_features: false,
            args: Vec::new(),
        };
        // Ensure the build runs from project root so relative paths resolve.
        std::env::set_current_dir(project_root)?;
        run_cmd.execute(verbose)?;

        // Locate the produced executable.
        let build_dir = project_root.join("target").join("run").join(bin_name);
        let produced = find_built_executable(&build_dir, bin_name).ok_or_else(|| {
            anyhow!(
                "Built binary not found under {} (target name: {})",
                build_dir.display(),
                bin_name
            )
        })?;

        // Copy into package cache
        let pkg_target = bin_dir_pkg.join(with_exe_suffix(bin_name));
        if pkg_target.exists() && !force {
            println!(
                "   {} bin '{}' already exists, skipping (use --force to overwrite)",
                style("ℹ️").yellow(),
                bin_name
            );
            continue;
        }
        if pkg_target.exists() {
            std::fs::remove_file(&pkg_target)?;
        }
        std::fs::copy(&produced, &pkg_target).with_context(|| {
            format!(
                "Failed to copy {} → {}",
                produced.display(),
                pkg_target.display()
            )
        })?;
        make_executable(&pkg_target)?;
        println!("   📦 package: {}", pkg_target.display());

        // Symlink into the global bin dir
        let link = bin_dir_global.join(with_exe_suffix(bin_name));
        if link.exists() || link.symlink_metadata().is_ok() {
            std::fs::remove_file(&link).ok();
        }
        make_symlink(&pkg_target, &link)?;
        println!(
            "   🔗 symlink: {} → {}",
            link.display(),
            pkg_target.display()
        );
    }

    // PATH hint (mirroring Cargo's post-install message).
    let bin_dir_display = ccgo_home_dir()?.join("bin").display().to_string();
    println!(
        "\n💡 Make sure the following directory is on your PATH:\n    export PATH=\"{}:$PATH\"",
        bin_dir_display
    );

    Ok(())
}

fn find_built_executable(build_dir: &Path, name: &str) -> Option<PathBuf> {
    let candidates = [
        build_dir.join(name),
        build_dir.join(format!("{}.exe", name)),
        build_dir.join("Release").join(name),
        build_dir.join("Release").join(format!("{}.exe", name)),
        build_dir.join("Debug").join(name),
        build_dir.join("Debug").join(format!("{}.exe", name)),
    ];
    candidates.into_iter().find(|p| p.exists())
}

fn with_exe_suffix(name: &str) -> String {
    if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    }
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(path)?.permissions();
        perms.set_mode(perms.mode() | 0o111);
        std::fs::set_permissions(path, perms)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path; // Windows: permissions implicit.
    }
    Ok(())
}

fn make_symlink(target: &Path, link: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).with_context(|| {
            format!(
                "Failed to symlink {} → {}",
                link.display(),
                target.display()
            )
        })?;
    }
    #[cfg(windows)]
    {
        // File symlink; falls back to copy on restricted Windows installs.
        if std::os::windows::fs::symlink_file(target, link).is_err() {
            std::fs::copy(target, link).with_context(|| {
                format!(
                    "Failed to create symlink or copy {} → {}",
                    target.display(),
                    link.display()
                )
            })?;
        }
    }
    Ok(())
}
