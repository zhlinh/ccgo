//! Publish a prebuilt Android AAR directly to Maven.
//!
//! `ccgo publish android --path <aar>` used to require the project's `android/`
//! Gradle build so the CCGO Gradle plugin could upload the AAR. The artifact is
//! already built, so local and private repositories can be served directly.
//! Coordinates and the artifact id mirror the Gradle plugin's rules, including
//! the `channel_desc` and `-stdembed` suffixes.
//!
//! Maven Central uses the separate Central Portal publisher (see
//! `publish_central`), which needs PGP signatures and a bundle.

use super::{artifact, maven, PublishCommand};
use anyhow::{bail, Context, Result};
use std::{fs, path::Path};

pub(super) struct Coordinates {
    pub project: String,
    pub group: String,
    pub artifact_id: String,
    pub version: String,
    pub dependencies: Vec<PomDependency>,
}

pub(super) type PomDependency = (String, String, String);

pub(super) fn coordinates(command: &PublishCommand, config: &toml::Value) -> Result<Coordinates> {
    let (project, version) = maven::project_identity(config)?;
    Ok(Coordinates {
        project: project.to_owned(),
        group: group_id(config)?,
        artifact_id: artifact_id(command, config, project)?,
        version: version.to_owned(),
        dependencies: dependencies(config)?,
    })
}

pub(super) fn publish(command: &PublishCommand, aar: &Path, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let config = maven::load_config(&cwd)?;
    let coordinates = coordinates(command, &config)?;
    let destination = maven::destination(command, &config)?;
    let client = destination.client()?;

    println!(
        "Publishing Android AAR {} directly to {} (no Gradle publish task)",
        aar.display(),
        destination.describe()
    );

    let relative = maven::layout_path(
        &coordinates.group,
        &coordinates.artifact_id,
        &coordinates.version,
        None,
        "aar",
    );
    let bytes = fs::read(aar).with_context(|| format!("Failed to read {}", aar.display()))?;
    maven::upload_with_checksum(&destination, client.as_ref(), &relative, bytes, verbose)?;

    let pom = pom_xml(&coordinates);
    maven::upload_with_checksum(
        &destination,
        client.as_ref(),
        &maven::layout_path(
            &coordinates.group,
            &coordinates.artifact_id,
            &coordinates.version,
            None,
            "pom",
        ),
        pom.into_bytes(),
        verbose,
    )?;

    println!(
        "  ✓ {}:{}:{}",
        coordinates.group, coordinates.artifact_id, coordinates.version
    );
    println!("\n✅ Publish completed successfully!");
    Ok(())
}

fn group_id(config: &toml::Value) -> Result<String> {
    let group = maven::string_at(config, &["publish", "maven", "group_id"])
        .or_else(|| maven::string_at(config, &["publish", "android", "maven", "group_id"]))
        .or_else(|| maven::string_at(config, &["project", "group_id"]))
        .or_else(|| maven::string_at(config, &["package", "group_id"]))
        .context(
            "Android publishing requires a Maven group id: set [publish.android.maven].group_id \
             or [project].group_id in CCGO.toml",
        )?;
    for part in group.split('.') {
        artifact::coordinate(part)
            .with_context(|| format!("Invalid Maven groupId segment in '{group}'"))?;
    }
    Ok(group.to_owned())
}

fn artifact_id(command: &PublishCommand, config: &toml::Value, project: &str) -> Result<String> {
    let base = maven::string_at(config, &["publish", "maven", "artifact_id"])
        .or_else(|| maven::string_at(config, &["publish", "android", "maven", "artifact_id"]))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(project)
        .to_lowercase();
    artifact::coordinate(&base).context("Invalid Maven artifactId")?;

    let channel = maven::string_at(config, &["publish", "maven", "channel_desc"])
        .or_else(|| maven::string_at(config, &["publish", "android", "maven", "channel_desc"]))
        .or_else(|| maven::string_at(config, &["publish", "channel_desc"]))
        .filter(|value| *value != "EMPTY")
        .unwrap_or_default()
        .to_lowercase();

    // `CCGO_ANDROID_STL` is what `ccgo publish android --stl ...` would have
    // handed Gradle; resolving it here keeps the `-stdembed` suffix honest.
    let stl = crate::builder::resolve_stl(
        command.stl.as_ref().map(|stl| stl.as_str()),
        maven::string_at(config, &["android", "stl"]),
    );
    let stl_suffix = if stl.ends_with("_static") {
        "-stdembed"
    } else {
        ""
    };

    let suffix = [channel.as_str(), stl_suffix]
        .into_iter()
        .filter(|part| !part.is_empty())
        .map(|part| part.trim_start_matches('-'))
        .collect::<Vec<_>>()
        .join("-");
    let artifact_id = if suffix.is_empty() {
        base
    } else {
        format!("{base}-{suffix}")
    };
    artifact::coordinate(&artifact_id).context("Invalid Maven artifactId")?;
    Ok(artifact_id)
}

fn dependencies(config: &toml::Value) -> Result<Vec<PomDependency>> {
    let entries = maven::value_at(config, &["publish", "maven", "dependencies"])
        .or_else(|| maven::value_at(config, &["publish", "android", "maven", "dependencies"]));
    let Some(toml::Value::Array(entries)) = entries else {
        return Ok(Vec::new());
    };
    entries
        .iter()
        .map(|entry| {
            let coordinate = entry
                .as_str()
                .with_context(|| "Maven dependencies must be 'group:artifact:version' strings")?;
            let parts: Vec<_> = coordinate.split(':').collect();
            if parts.len() < 3 || parts[..3].iter().any(|part| part.is_empty()) {
                bail!("Invalid Maven dependency coordinate: {coordinate}");
            }
            for part in &parts[..3] {
                artifact::coordinate(part).with_context(|| {
                    format!("Invalid Maven dependency coordinate: {coordinate}")
                })?;
            }
            Ok((
                parts[0].to_owned(),
                parts[1].to_owned(),
                parts[2].to_owned(),
            ))
        })
        .collect()
}

pub(super) fn pom_xml(coordinates: &Coordinates) -> String {
    let mut pom = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <project xmlns=\"http://maven.apache.org/POM/4.0.0\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 \
         https://maven.apache.org/xsd/maven-4.0.0.xsd\">\n\
         \x20 <modelVersion>4.0.0</modelVersion>\n",
    );
    for (tag, value) in [
        ("groupId", coordinates.group.as_str()),
        ("artifactId", coordinates.artifact_id.as_str()),
        ("version", coordinates.version.as_str()),
        ("packaging", "aar"),
        ("name", coordinates.project.as_str()),
    ] {
        pom.push_str(&format!("  <{tag}>{}</{tag}>\n", maven::xml_escape(value)));
    }
    pom.push_str(&format!(
        "  <description>The {} SDK</description>\n",
        maven::xml_escape(&coordinates.project)
    ));
    if !coordinates.dependencies.is_empty() {
        pom.push_str("  <dependencies>\n");
        for (group, artifact, version) in &coordinates.dependencies {
            pom.push_str("    <dependency>\n");
            for (tag, value) in [
                ("groupId", group.as_str()),
                ("artifactId", artifact.as_str()),
                ("version", version.as_str()),
            ] {
                pom.push_str(&format!(
                    "      <{tag}>{}</{tag}>\n",
                    maven::xml_escape(value)
                ));
            }
            pom.push_str("    </dependency>\n");
        }
        pom.push_str("  </dependencies>\n");
    }
    pom.push_str("</project>\n");
    pom
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        command: PublishCommand,
    }

    fn config(extra: &str) -> toml::Value {
        toml::from_str(&format!(
            "[project]\nname='demo'\nversion='1.2.3'\ngroup_id='com.example'\n{extra}"
        ))
        .unwrap()
    }

    fn command(args: &[&str]) -> PublishCommand {
        let mut argv = vec!["ccgo", "android"];
        argv.extend_from_slice(args);
        Cli::parse_from(argv).command
    }

    #[test]
    fn artifact_id_matches_plugin_suffix_rules() {
        let shared = config(
            "[publish.android.maven]\nartifact_id='Demo'\nchannel_desc='BETA'\n\
             [android]\nstl='c++_shared'\n",
        );
        assert_eq!(
            coordinates(&command(&[]), &shared).unwrap().artifact_id,
            "demo-beta"
        );

        let r#static = config("[android]\nstl='c++_static'\n");
        assert_eq!(
            coordinates(&command(&[]), &r#static).unwrap().artifact_id,
            "demo-stdembed"
        );
        assert_eq!(
            coordinates(&command(&["--stl", "c++_static"]), &shared)
                .unwrap()
                .artifact_id,
            "demo-beta-stdembed"
        );
    }

    #[test]
    fn group_and_dependencies_follow_plugin_fallbacks() {
        let manifest = config(
            "[publish.android.maven]\ngroup_id='com.cyber.mna'\n\
             dependencies=['org.example:dep:1.2.3']\n",
        );
        let resolved = coordinates(&command(&[]), &manifest).unwrap();
        assert_eq!(resolved.group, "com.cyber.mna");
        assert_eq!(
            resolved.dependencies,
            vec![(
                "org.example".to_string(),
                "dep".to_string(),
                "1.2.3".to_string()
            )]
        );

        let bad = config("[publish.android.maven]\ndependencies=['broken']\n");
        assert!(coordinates(&command(&[]), &bad).is_err());
    }

    #[test]
    fn pom_matches_android_plugin_shape() {
        let coordinates = Coordinates {
            project: "demo".into(),
            group: "com.example".into(),
            artifact_id: "demo".into(),
            version: "1.2.3".into(),
            dependencies: vec![("org.example".into(), "dep".into(), "1.2.3".into())],
        };
        let pom = pom_xml(&coordinates);
        assert!(pom.contains("<packaging>aar</packaging>"));
        assert!(pom.contains("<artifactId>dep</artifactId>"));
        assert!(!pom.contains("<scope>"));
    }
}
