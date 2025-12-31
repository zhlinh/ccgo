//! Documentation command implementation

use anyhow::{anyhow, Context, Result};
use clap::Args;
use console::style;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if a command is available in PATH
fn check_command_installed(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Check if a Python package is installed
fn check_python_package_installed(package: &str) -> bool {
    Command::new("python3")
        .args(["-c", &format!("import {}", package)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install Python dependencies from requirements.txt
fn install_python_requirements(requirements_file: &Path) -> Result<()> {
    println!(
        "{}",
        style(format!(
            "Installing dependencies from {}...",
            requirements_file.display()
        ))
        .cyan()
    );

    let status = Command::new("pip3")
        .args(["install", "-r"])
        .arg(requirements_file)
        .status()
        .context("Failed to execute pip3")?;

    if !status.success() {
        return Err(anyhow!("Failed to install dependencies"));
    }

    println!("{}", style("Dependencies installed successfully!\n").green());
    Ok(())
}

/// Find the project directory containing mkdocs.yml
fn find_mkdocs_project(start_dir: &Path) -> Result<PathBuf> {
    // First check if mkdocs.yml exists in start_dir
    let mkdocs_yml = start_dir.join("mkdocs.yml");
    if mkdocs_yml.is_file() {
        return Ok(start_dir.to_path_buf());
    }

    // Check immediate subdirectories
    if let Ok(entries) = std::fs::read_dir(start_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let mkdocs_yml = entry.path().join("mkdocs.yml");
            if mkdocs_yml.is_file() {
                return Ok(entry.path());
            }
        }
    }

    Err(anyhow!(
        "mkdocs.yml not found in project directory.\n\
         Please ensure you are in a CCGO project directory with MkDocs configured.\n\
         Expected location: <project>/mkdocs.yml or <project>/<subdir>/mkdocs.yml"
    ))
}

/// Check and optionally install MkDocs dependencies
fn check_and_install_deps(project_dir: &Path, auto_install: bool) -> Result<()> {
    // Check if docs/requirements.txt exists
    let requirements_file = project_dir.join("docs").join("requirements.txt");
    if !requirements_file.is_file() {
        return Ok(());
    }

    // Check if key packages are installed
    let mkdocs_installed = check_python_package_installed("mkdocs");
    let material_installed = check_python_package_installed("material");
    let mkdoxy_installed = check_python_package_installed("mkdoxy");

    if mkdocs_installed && material_installed && mkdoxy_installed {
        return Ok(());
    }

    // Some dependencies are missing
    println!(
        "{}",
        style("MkDocs dependencies check:").yellow().bold()
    );
    println!(
        "  mkdocs:          {}",
        if mkdocs_installed {
            style("✓ installed").green()
        } else {
            style("✗ missing").red()
        }
    );
    println!(
        "  mkdocs-material: {}",
        if material_installed {
            style("✓ installed").green()
        } else {
            style("✗ missing").red()
        }
    );
    println!(
        "  mkdoxy:          {}",
        if mkdoxy_installed {
            style("✓ installed").green()
        } else {
            style("✗ missing").red()
        }
    );
    println!();

    if auto_install {
        install_python_requirements(&requirements_file)?;
        return Ok(());
    }

    // Ask user if they want to install
    print!(
        "{}",
        style("Would you like to install missing dependencies now? [Y/n] ")
            .cyan()
            .bold()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "y" || input == "yes" {
        install_python_requirements(&requirements_file)?;
    } else {
        println!(
            "{}",
            style("Skipping dependency installation. Documentation build may fail.\n").yellow()
        );
        println!("You can install dependencies later with:");
        println!("  pip3 install -r {}", requirements_file.display());
        println!("Or run:");
        println!("  ccgo doc --install-deps\n");
    }

    Ok(())
}

/// Generate MkDocs documentation
#[derive(Args, Debug)]
pub struct DocCommand {
    /// Open documentation in browser after building
    #[arg(long)]
    pub open: bool,

    /// Start MkDocs development server with live reload
    #[arg(long)]
    pub serve: bool,

    /// Port for development server (default: 8000, used with --serve)
    #[arg(long, default_value = "8000")]
    pub port: u16,

    /// Clean build artifacts before generating documentation
    #[arg(long)]
    pub clean: bool,

    /// Automatically install missing dependencies without prompting
    #[arg(long)]
    pub install_deps: bool,
}

impl DocCommand {
    /// Execute the doc command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", style("Building project documentation with MkDocs...\n").bold());

        // Get current working directory
        let start_dir = std::env::current_dir()
            .context("Failed to get current working directory")?;

        // Find project directory with mkdocs.yml
        let project_dir = find_mkdocs_project(&start_dir)?;
        println!("Project directory: {}", project_dir.display());

        // Check for mkdocs.yml
        let mkdocs_yml = project_dir.join("mkdocs.yml");
        if !mkdocs_yml.is_file() {
            return Err(anyhow!("mkdocs.yml not found at {}", mkdocs_yml.display()));
        }

        // Check and install Python dependencies
        check_and_install_deps(&project_dir, self.install_deps)?;

        // Check dependencies
        if !check_command_installed("mkdocs") {
            return Err(anyhow!(
                "MkDocs is not installed or not in PATH.\n\
                 Install it with: pip3 install ccgo[docs]\n\
                 Or install from project requirements: pip3 install -r docs/requirements.txt"
            ));
        }

        if !check_command_installed("doxygen") {
            println!(
                "{}",
                style(
                    "WARNING: Doxygen is not installed or not in PATH.\n\
                     MkDoxy requires Doxygen to generate API documentation.\n\
                     Install it with:\n\
                       macOS: brew install doxygen\n\
                       Ubuntu: sudo apt-get install doxygen\n\
                       Windows: choco install doxygen\n\
                     API documentation may not be generated.\n"
                )
                .yellow()
            );
        }

        // Handle --serve mode (development server with live reload)
        if self.serve {
            println!(
                "Starting MkDocs development server on port {}...",
                self.port
            );
            println!("Documentation URL: http://127.0.0.1:{}/", self.port);
            println!("Press Ctrl+C to stop the server\n");

            let status = Command::new("mkdocs")
                .args(["serve", "-a", &format!("127.0.0.1:{}", self.port)])
                .current_dir(&project_dir)
                .status()
                .context("Failed to execute mkdocs serve")?;

            if !status.success() {
                println!("\nServer stopped");
            }
            return Ok(());
        }

        // Build mode
        println!("Mode: Build documentation\n");

        // Output to target/docs/site/ directory
        let site_dir = project_dir.join("target").join("docs").join("site");
        let mut cmd = Command::new("mkdocs");
        cmd.args(["build", "--site-dir"])
            .arg(&site_dir)
            .current_dir(&project_dir);

        if self.clean {
            cmd.arg("--clean");
            println!("Clean build enabled");
        }

        println!("Running: mkdocs build --site-dir {}", site_dir.display());
        if self.clean {
            println!("         --clean");
        }
        println!();

        let status = cmd.status().context("Failed to execute mkdocs build")?;

        if !status.success() {
            return Err(anyhow!("Documentation build failed"));
        }

        println!("\n{}", style("Documentation built successfully!").green().bold());

        // Get output path
        let index_path = site_dir.join("index.html");

        if index_path.exists() {
            println!("Documentation location: {}", site_dir.display());

            // Handle --open option
            if self.open {
                println!("\nOpening documentation in browser...");
                let url = format!("file://{}", index_path.display());
                if let Err(e) = open::that(&url) {
                    println!("Warning: Failed to open browser: {}", e);
                    println!("You can manually open: {}", url);
                }
            }
        } else {
            println!("\nWarning: Documentation output not found at expected location");
            println!("   Expected: {}", index_path.display());
        }

        Ok(())
    }
}
