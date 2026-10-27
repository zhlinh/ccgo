//! Conan 2 binary creation and publishing through the existing Conan target.
use super::{PublishCommand, RegistryType};
use anyhow::{bail, Context, Result};
use clap::Args;
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    process::{Command, Stdio},
};

#[derive(Args, Debug, Default)]
pub struct ConanOptions {
    /// Build-machine profile (defaults to Conan's default profile)
    #[arg(long)]
    pub build_profile: Option<String>,
    /// Conan build types, comma-separated (default: Release with --release, otherwise Debug)
    #[arg(long, value_delimiter = ',')]
    pub conan_build_types: Vec<String>,
    /// Additional host setting KEY=VALUE, repeatable (e.g. compiler.runtime=dynamic)
    #[arg(long)]
    pub conan_setting: Vec<String>,
    /// Additional host option PATTERN:KEY=VALUE, repeatable
    #[arg(long)]
    pub conan_option: Vec<String>,
}

fn exact(value: &str) -> Result<&str> {
    if value.is_empty() || value.starts_with('-') || value.contains(['*', '?', '\n', '\r']) {
        bail!("Conan did not return an exact reference");
    }
    Ok(value)
}

fn run_json(cmd: &mut Command) -> Result<Value> {
    cmd.arg("--format=json")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let output = cmd.output().context("Failed to execute Conan 2")?;
    if !output.status.success() {
        bail!("Conan command failed; no further packages will be uploaded");
    }
    serde_json::from_slice(&output.stdout).context("Conan did not return valid JSON")
}

fn package_list(graph: &Value, reference: &str, list: &mut Value) -> Result<()> {
    let nodes = graph
        .pointer("/graph/nodes")
        .and_then(Value::as_object)
        .context("Conan create did not return graph nodes")?;
    let (recipe, recipe_revision) = reference
        .split_once('#')
        .context("Missing Conan recipe revision")?;
    exact(recipe_revision)?;
    let mut found = false;
    for node in nodes.values() {
        if node.get("ref").and_then(Value::as_str) != Some(reference)
            || node.get("context").and_then(Value::as_str) == Some("build")
        {
            continue;
        }
        let id = exact(
            node.get("package_id")
                .and_then(Value::as_str)
                .context("Missing Conan package ID")?,
        )?;
        let revision = exact(
            node.get("prev")
                .and_then(Value::as_str)
                .context("Missing Conan package revision")?,
        )?;
        list["Local Cache"][recipe]["revisions"][recipe_revision]["packages"][id]["revisions"]
            [revision] = json!({});
        found = true;
    }
    if !found {
        bail!("Conan create graph does not contain the exported recipe {reference}");
    }
    Ok(())
}

fn validate_profiles(command: &PublishCommand) -> Result<()> {
    let profiles: BTreeSet<_> = command.profile.split(',').map(str::trim).collect();
    if profiles.iter().any(|p| p.is_empty()) {
        bail!("Conan host profiles must not be empty");
    }
    if command
        .conan
        .build_profile
        .as_deref()
        .is_some_and(|p| p.trim().is_empty())
    {
        bail!("Conan build profile must not be empty");
    }
    Ok(())
}

fn build_types(command: &PublishCommand) -> Result<Vec<String>> {
    let build_types = if command.conan.conan_build_types.is_empty() {
        vec![if command.release {
            "Release".to_string()
        } else {
            "Debug".to_string()
        }]
    } else {
        command
            .conan
            .conan_build_types
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    for kind in &build_types {
        if !["Debug", "Release", "RelWithDebInfo", "MinSizeRel"].contains(&kind.as_str()) {
            bail!("Invalid Conan build type: {kind}");
        }
    }
    Ok(build_types)
}

fn linkages(command: &PublishCommand) -> Result<Vec<bool>> {
    let linkages = match command.link_type.as_str() {
        "static" => vec![false],
        "shared" => vec![true],
        "both" => vec![false, true],
        _ => bail!("Conan --link-type must be static, shared or both"),
    };
    Ok(linkages)
}

fn validate_overrides(command: &PublishCommand) -> Result<()> {
    for setting in &command.conan.conan_setting {
        let (key, value) = setting
            .split_once('=')
            .context("Conan settings require KEY=VALUE")?;
        if key.is_empty() || value.is_empty() || key == "build_type" {
            bail!(
                "Use --conan-build-types for build_type; other settings require nonempty KEY=VALUE"
            );
        }
    }
    for option in &command.conan.conan_option {
        let (key, value) = option
            .split_once('=')
            .context("Conan options require KEY=VALUE")?;
        if key.is_empty() || value.is_empty() || key == "shared" || key.ends_with(":shared") {
            bail!("Use --link-type for shared; other options require nonempty KEY=VALUE");
        }
    }
    Ok(())
}

fn create_matrix(
    command: &PublishCommand,
    recipe: &std::path::Path,
    base: &impl Fn() -> Command,
    build_types: &[String],
    linkages: &[bool],
    reference: &str,
    packages: &mut Value,
) -> Result<()> {
    for profile in command
        .profile
        .split(',')
        .map(str::trim)
        .collect::<BTreeSet<_>>()
    {
        for kind in build_types {
            for shared in linkages {
                println!(
                    "Creating Conan package: profile={profile}, build_type={kind}, shared={shared}"
                );
                let mut create = base();
                create
                    .arg("create")
                    .arg(recipe)
                    .args(["--user=ccgo", "--channel=stable", "--build=missing"])
                    .arg(format!("-pr:h={profile}"))
                    .arg(format!(
                        "-pr:b={}",
                        command.conan.build_profile.as_deref().unwrap_or("default")
                    ))
                    .arg(format!("-s:h=build_type={kind}"))
                    .arg(format!(
                        "-o:h=&:shared={}",
                        if *shared { "True" } else { "False" }
                    ))
                    .env("CCGO_BUILD_SHARED", if *shared { "ON" } else { "OFF" });
                for setting in &command.conan.conan_setting {
                    create.arg(format!("-s:h={setting}"));
                }
                for option in &command.conan.conan_option {
                    create.arg(format!("-o:h={option}"));
                }
                let graph = run_json(&mut create).with_context(|| {
                        format!("Conan create failed: profile={profile}, build_type={kind}, shared={shared}")
                    })?;
                package_list(&graph, reference, packages).with_context(|| {
                        format!("Invalid Conan package graph: profile={profile}, build_type={kind}, shared={shared}")
                    })?;
            }
        }
    }
    Ok(())
}

fn destination(command: &PublishCommand) -> Result<Option<&str>> {
    if matches!(command.registry, RegistryType::Official) {
        bail!("ConanCenter does not accept direct CLI uploads; submit a recipe PR to https://github.com/conan-io/conan-center-index");
    }
    let remote = if matches!(command.registry, RegistryType::Private) {
        Some(
            command
                .remote
                .as_deref()
                .context("Private Conan registry requires --remote")?,
        )
    } else {
        None
    };
    if remote.is_some_and(|value| value.trim().is_empty() || value.starts_with('-')) {
        bail!("Private Conan registry requires a nonempty configured remote name");
    }
    Ok(remote)
}

fn upload_packages(
    command: &PublishCommand,
    base: &impl Fn() -> Command,
    remote: &str,
    reference: &str,
    packages: &Value,
) -> Result<()> {
    let temporary = tempfile::tempdir()?;
    let mut upload = base();
    upload.arg("upload");
    if command.skip_build {
        upload.arg(reference).arg("--only-recipe");
    } else {
        let path = temporary.path().join("packages.json");
        fs::write(&path, serde_json::to_vec_pretty(&packages)?)?;
        upload.arg("--list").arg(path);
    }
    upload.args(["--remote", remote, "--confirm"]);
    if !upload.status()?.success() {
        bail!("Conan upload failed; local packages remain available for retry");
    }
    Ok(())
}

fn export_recipe(
    base: &impl Fn() -> Command,
    recipe: &std::path::Path,
    recipe_only: bool,
) -> Result<Option<String>> {
    let mut export = base();
    export
        .arg("export")
        .arg(recipe)
        .args(["--user=ccgo", "--channel=stable"]);
    if recipe_only {
        if !export.status()?.success() {
            bail!("Conan export failed");
        }
        return Ok(None);
    }
    let exported = run_json(&mut export)?;
    let reference = exact(
        exported
            .get("reference")
            .and_then(Value::as_str)
            .context("Conan export did not return an exact recipe reference")?,
    )?;
    Ok(Some(reference.to_owned()))
}

fn recipe_paths(command: &PublishCommand) -> Result<(std::path::PathBuf, std::path::PathBuf)> {
    let cwd = std::env::current_dir()?;
    let project = match &command.path {
        Some(path) => path
            .parent()
            .context("Conan recipe has no parent")?
            .to_path_buf(),
        None => PublishCommand::find_project_dir(&cwd)?,
    };
    let recipe = command.path.clone().unwrap_or_else(|| project.clone());
    Ok((project, recipe))
}

pub(super) fn publish(command: &PublishCommand, verbose: bool) -> Result<()> {
    let remote = destination(command)?;
    validate_profiles(command)?;
    validate_overrides(command)?;
    let build_types = build_types(command)?;
    let linkages = linkages(command)?;
    let (project, recipe) = recipe_paths(command)?;
    let base = || {
        let mut cmd = Command::new("conan");
        cmd.current_dir(&project);
        if verbose {
            cmd.env("CONAN_LOG_LEVEL", "debug");
        }
        cmd
    };
    let Some(reference) = export_recipe(&base, &recipe, command.skip_build && remote.is_none())?
    else {
        return Ok(());
    };
    let mut packages = json!({"Local Cache": {}});
    if !command.skip_build {
        create_matrix(
            command,
            &recipe,
            &base,
            &build_types,
            &linkages,
            &reference,
            &mut packages,
        )?;
    }
    if let Some(remote) = remote {
        upload_packages(command, &base, remote, &reference, &packages)?;
    }
    println!("Conan publishing completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_includes_only_exact_recipe_and_package_revision() {
        let graph = json!({"graph":{"nodes":{
            "1":{"ref":"demo/1#r", "package_id":"id", "prev":"rev"},
            "2":{"ref":"dependency/1#r", "package_id":"other", "prev":"rev"}
        }}});
        let mut list = json!({"Local Cache":{}});
        package_list(&graph, "demo/1#r", &mut list).unwrap();
        assert_eq!(list["Local Cache"].as_object().unwrap().len(), 1);
        assert!(list
            .pointer("/Local Cache/demo~11/revisions/r/packages/id/revisions/rev")
            .is_some());
        assert!(package_list(&graph, "missing/1#r", &mut list).is_err());
    }

    #[test]
    fn selection_excludes_build_context_and_rejects_missing_revisions() {
        let mut graph = json!({"graph":{"nodes":{
            "1":{"ref":"demo/1#r", "context":"build", "package_id":"tool", "prev":"rev"},
            "2":{"ref":"demo/1#r", "context":"host", "package_id":"host", "prev":"rev"}
        }}});
        let mut list = json!({"Local Cache":{}});
        package_list(&graph, "demo/1#r", &mut list).unwrap();
        let packages = &list["Local Cache"]["demo/1"]["revisions"]["r"]["packages"];
        assert_eq!(packages.as_object().unwrap().len(), 1);
        assert!(packages.get("host").is_some());
        graph["graph"]["nodes"]["2"]["prev"] = Value::Null;
        assert!(package_list(&graph, "demo/1#r", &mut list).is_err());
    }
}
