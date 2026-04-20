//! Self-update logic for `ccgo update self`
//!
//! Detects how ccgo was installed (brew/cargo/pipx/uv/pip) and runs
//! the appropriate package-manager upgrade command.

use anyhow::{bail, Context, Result};
use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
enum InstallMethod {
    Brew,
    Cargo,
    Pipx,
    Uv,
    Pip,
}

impl InstallMethod {
    fn label(&self) -> &'static str {
        match self {
            InstallMethod::Brew => "Homebrew",
            InstallMethod::Cargo => "Cargo",
            InstallMethod::Pipx => "pipx",
            InstallMethod::Uv => "uv",
            InstallMethod::Pip => "pip",
        }
    }
}

pub fn execute_self_update(_verbose: bool) -> Result<()> {
    println!("{}", "=".repeat(80));
    println!("CCGO Self-Update");
    println!("{}", "=".repeat(80));

    println!("\n🔍 Detecting installation method...");

    match detect_install_method() {
        Some(method) => {
            println!("   Detected: {}", method.label());
            run_update(&method)
        }
        None => bail!(
            "Could not detect how ccgo was installed.\n\
             Update manually with one of:\n\
               brew upgrade ccgo\n\
               cargo install ccgo --force\n\
               pipx upgrade ccgo\n\
               uv tool upgrade ccgo\n\
               pip install --upgrade ccgo"
        ),
    }
}

fn detect_install_method() -> Option<InstallMethod> {
    detect_from_exe_path().or_else(detect_from_package_managers)
}

fn detect_from_exe_path() -> Option<InstallMethod> {
    let exe = std::env::current_exe().ok()?;
    let resolved = std::fs::canonicalize(&exe).unwrap_or(exe);
    let path = resolved.to_string_lossy();

    if path.contains("/Cellar/") {
        return Some(InstallMethod::Brew);
    }
    if path.contains("/pipx/venvs/") || path.contains(".local/pipx") {
        return Some(InstallMethod::Pipx);
    }
    if path.contains("/uv/tools/") {
        return Some(InstallMethod::Uv);
    }
    if path.contains("/.cargo/bin/") {
        return Some(InstallMethod::Cargo);
    }
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        if path.starts_with(cargo_home.trim_end_matches('/')) {
            return Some(InstallMethod::Cargo);
        }
    }

    None
}

fn detect_from_package_managers() -> Option<InstallMethod> {
    if run_silent("pipx", &["list", "--short"])
        .map(|o| o.contains("ccgo"))
        .unwrap_or(false)
    {
        return Some(InstallMethod::Pipx);
    }

    if run_silent("uv", &["tool", "list"])
        .map(|o| o.contains("ccgo"))
        .unwrap_or(false)
    {
        return Some(InstallMethod::Uv);
    }

    if run_silent("brew", &["list", "--formula"])
        .map(|o| o.split_whitespace().any(|s| s == "ccgo"))
        .unwrap_or(false)
    {
        return Some(InstallMethod::Brew);
    }

    for pip in &["pip3", "pip"] {
        if run_silent(pip, &["show", "ccgo"])
            .map(|o| o.to_ascii_lowercase().contains("name: ccgo"))
            .unwrap_or(false)
        {
            return Some(InstallMethod::Pip);
        }
    }

    if run_silent("cargo", &["install", "--list"])
        .map(|o| o.contains("ccgo"))
        .unwrap_or(false)
    {
        return Some(InstallMethod::Cargo);
    }

    None
}

fn run_silent(program: &str, args: &[&str]) -> Option<String> {
    Command::new(program)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

fn run_update(method: &InstallMethod) -> Result<()> {
    match method {
        InstallMethod::Brew => run_command("brew", &["upgrade", "ccgo"]),
        InstallMethod::Cargo => run_command("cargo", &["install", "ccgo", "--force"]),
        InstallMethod::Pipx => run_command("pipx", &["upgrade", "ccgo"]),
        InstallMethod::Uv => run_command("uv", &["tool", "upgrade", "ccgo"]),
        InstallMethod::Pip => {
            let pip = if which::which("pip3").is_ok() { "pip3" } else { "pip" };
            run_command(pip, &["install", "--upgrade", "ccgo"])
        }
    }
}

fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let cmd_str = format!("{} {}", program, args.join(" "));
    println!("\n🚀 Running: {}\n", cmd_str);

    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("Failed to run: {}", cmd_str))?;

    if !status.success() {
        bail!("{} failed with non-zero exit code", cmd_str);
    }

    println!("\n✓ ccgo updated successfully via {}", program);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_method_label() {
        assert_eq!(InstallMethod::Brew.label(), "Homebrew");
        assert_eq!(InstallMethod::Cargo.label(), "Cargo");
        assert_eq!(InstallMethod::Pipx.label(), "pipx");
        assert_eq!(InstallMethod::Uv.label(), "uv");
        assert_eq!(InstallMethod::Pip.label(), "pip");
    }
}
