//! Publish prebuilt KMP SDK artifacts directly to Maven.
//!
//! `ccgo publish kmp --path <SDK zip>` used to require the Gradle plugin so a
//! `ccgoPublishToMaven*` task could upload the prebuilt files. That made a
//! prebuilt-only release depend on the module applying the CCGO plugin. This
//! module uploads the same files without Gradle, using the Maven repository
//! layout described by the SDK manifest and `[publish.kmp.maven]`.
//!
//! Maven Central is intentionally not handled here: it needs the Central
//! Portal/nmcp workflow, so `--registry official` keeps using Gradle.

use super::{
    artifact,
    kmp_sdk::{Artifact, PreparedSdk},
    maven, PublishCommand,
};
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, fs, path::Path};

#[derive(Debug)]
pub(super) struct Dependency {
    pub group: String,
    pub artifact: String,
    pub version: String,
}

#[derive(Debug)]
pub(super) struct Coordinates {
    pub group: String,
    pub artifact_base: String,
    pub dependencies: Vec<Dependency>,
}

pub(super) fn publish(command: &PublishCommand, sdk: &PreparedSdk, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let config = maven::load_config(&cwd)?;
    let coordinates = coordinates(&config, sdk)?;
    let destination = maven::destination(command, &config)?;
    let client = destination.client()?;

    println!(
        "Publishing prebuilt KMP SDK {} {} directly to {} (Gradle not used)",
        sdk.project,
        sdk.version,
        destination.describe()
    );

    let mut targets: BTreeMap<&str, Vec<&Artifact>> = BTreeMap::new();
    for artifact in &sdk.artifacts {
        targets.entry(&artifact.target).or_default().push(artifact);
    }

    for (target, artifacts) in targets {
        let artifact_id = format!("{}-{}", coordinates.artifact_base, target);
        artifact::coordinate(&artifact_id)
            .with_context(|| format!("Invalid Maven artifactId for target {target}"))?;
        let main = artifacts
            .iter()
            .find(|artifact| artifact.classifier.is_none())
            .with_context(|| format!("KMP SDK target {target} has no main artifact"))?;
        let packaging = extension(&main.path)?;
        let pom = pom_xml(
            &sdk.project,
            target,
            &coordinates.group,
            &artifact_id,
            &sdk.version,
            packaging,
            &coordinates.dependencies,
        );

        for artifact in artifacts {
            let relative = maven::layout_path(
                &coordinates.group,
                &artifact_id,
                &sdk.version,
                artifact.classifier.as_deref(),
                extension(&artifact.path)?,
            );
            let bytes = fs::read(&artifact.path)
                .with_context(|| format!("Failed to read {}", artifact.path.display()))?;
            maven::upload_with_checksum(&destination, client.as_ref(), &relative, bytes, verbose)?;
        }
        // POM last: a consumer never sees a complete coordinate before every
        // artifact it references is present.
        maven::upload_with_checksum(
            &destination,
            client.as_ref(),
            &maven::layout_path(&coordinates.group, &artifact_id, &sdk.version, None, "pom"),
            pom.into_bytes(),
            verbose,
        )?;

        println!("  ✓ {}:{}:{}", coordinates.group, artifact_id, sdk.version);
    }

    println!("\n✅ Publish completed successfully!");
    Ok(())
}

pub(super) fn coordinates(config: &toml::Value, sdk: &PreparedSdk) -> Result<Coordinates> {
    let (project, _) = maven::project_identity(config)?;
    if project != sdk.project {
        bail!(
            "KMP SDK project '{}' does not match '{}' in CCGO.toml",
            sdk.project,
            project
        );
    }

    let group = maven::string_at(config, &["publish", "kmp", "maven", "group_id"])
        .or_else(|| maven::string_at(config, &["publish", "kmp", "group_id"]))
        .or_else(|| maven::string_at(config, &["publish", "android", "maven", "group_id"]))
        .or_else(|| maven::string_at(config, &["project", "group_id"]))
        .or_else(|| maven::string_at(config, &["package", "group_id"]))
        .context(
            "KMP publishing requires a Maven group id: set [publish.kmp.maven].group_id, \
             [publish.android.maven].group_id, or [project].group_id in CCGO.toml",
        )?;
    for part in group.split('.') {
        artifact::coordinate(part)
            .with_context(|| format!("Invalid Maven groupId segment in '{group}'"))?;
    }

    let artifact_base = maven::string_at(config, &["publish", "kmp", "maven", "artifact_id"])
        .or_else(|| maven::string_at(config, &["publish", "kmp", "artifact_id"]))
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{project}-kmp"));
    artifact::coordinate(&artifact_base).context("Invalid Maven artifactId")?;
    artifact::coordinate(&sdk.version).context("Invalid KMP SDK version")?;

    Ok(Coordinates {
        group: group.to_owned(),
        artifact_base,
        dependencies: dependencies(config)?,
    })
}

fn dependencies(config: &toml::Value) -> Result<Vec<Dependency>> {
    let entries = maven::value_at(config, &["publish", "kmp", "maven", "dependencies"])
        .or_else(|| maven::value_at(config, &["publish", "kmp", "dependencies"]));
    let Some(toml::Value::Array(entries)) = entries else {
        return Ok(Vec::new());
    };
    entries
        .iter()
        .map(|entry| {
            let field = |name: &str| -> Result<String> {
                let value = entry
                    .get(name)
                    .and_then(|value| value.as_str())
                    .filter(|value| !value.trim().is_empty())
                    .with_context(|| {
                        format!("[publish.kmp.maven].dependencies entry requires {name}")
                    })?;
                artifact::coordinate(value).with_context(|| {
                    format!("Invalid Maven coordinate in dependencies entry: {value}")
                })?;
                Ok(value.to_owned())
            };
            Ok(Dependency {
                group: field("group")?,
                artifact: field("artifact")?,
                version: field("version")?,
            })
        })
        .collect()
}

pub(super) fn extension(path: &Path) -> Result<&str> {
    path.extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .with_context(|| format!("Artifact has no extension: {}", path.display()))
}

fn pom_xml(
    project: &str,
    target: &str,
    group: &str,
    artifact: &str,
    version: &str,
    packaging: &str,
    dependencies: &[Dependency],
) -> String {
    let mut pom = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <project xmlns=\"http://maven.apache.org/POM/4.0.0\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 \
         https://maven.apache.org/xsd/maven-4.0.0.xsd\">\n\
         \x20 <modelVersion>4.0.0</modelVersion>\n",
    );
    for (tag, value) in [
        ("groupId", group),
        ("artifactId", artifact),
        ("version", version),
        ("packaging", packaging),
    ] {
        pom.push_str(&format!("  <{tag}>{}</{tag}>\n", maven::xml_escape(value)));
    }
    pom.push_str(&format!(
        "  <name>{} KMP - {}</name>\n",
        maven::xml_escape(project),
        maven::xml_escape(target)
    ));
    pom.push_str(&format!(
        "  <description>Prebuilt {} KMP SDK ({})</description>\n",
        maven::xml_escape(project),
        maven::xml_escape(target)
    ));
    if !dependencies.is_empty() {
        pom.push_str("  <dependencies>\n");
        for dependency in dependencies {
            pom.push_str("    <dependency>\n");
            for (tag, value) in [
                ("groupId", dependency.group.as_str()),
                ("artifactId", dependency.artifact.as_str()),
                ("version", dependency.version.as_str()),
            ] {
                pom.push_str(&format!(
                    "      <{tag}>{}</{tag}>\n",
                    maven::xml_escape(value)
                ));
            }
            pom.push_str("      <scope>compile</scope>\n");
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
    use std::io::Write;

    fn config(extra: &str) -> toml::Value {
        toml::from_str(&format!(
            "[project]\nname='demo'\nversion='0.0.5'\ngroup_id='com.example'\n{extra}"
        ))
        .unwrap()
    }

    fn sdk() -> PreparedSdk {
        let temporary = tempfile::tempdir().unwrap();
        let zip_path = temporary.path().join("SDK.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&zip_path).unwrap());
        for (name, bytes) in [
            (
                "meta/kmp/build_info.json",
                br#"{"project":"demo","version":"0.0.5"}"#.as_slice(),
            ),
            ("lib/kmp/android/demo.aar", b"aar".as_slice()),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(bytes).unwrap();
        }
        zip.finish().unwrap();
        PreparedSdk::prepare(&zip_path).unwrap()
    }

    #[test]
    fn coordinates_fall_back_from_kmp_to_android_to_project() {
        let sdk = sdk();
        let base = config("");
        let resolved = coordinates(&base, &sdk).unwrap();
        assert_eq!(resolved.group, "com.example");
        assert_eq!(resolved.artifact_base, "demo-kmp");

        let android = config("[publish.android.maven]\ngroup_id='com.android'\n");
        let resolved = coordinates(&android, &sdk).unwrap();
        assert_eq!(resolved.group, "com.android");

        let kmp = config("[publish.kmp.maven]\ngroup_id='com.kmp'\nartifact_id='custom-kmp'\n");
        let resolved = coordinates(&kmp, &sdk).unwrap();
        assert_eq!(resolved.group, "com.kmp");
        assert_eq!(resolved.artifact_base, "custom-kmp");
    }

    #[test]
    fn rejects_mismatched_project_and_invalid_coordinates() {
        let sdk = sdk();
        let config: toml::Value =
            toml::from_str("[project]\nname='other'\nversion='0.0.5'\ngroup_id='com.example'\n")
                .unwrap();
        assert!(coordinates(&config, &sdk)
            .unwrap_err()
            .to_string()
            .contains("does not match"));
        let config: toml::Value =
            toml::from_str("[project]\nname='demo'\nversion='0.0.5'\ngroup_id='../bad'\n").unwrap();
        assert!(coordinates(&config, &sdk).is_err());
    }

    #[test]
    fn pom_carries_dependencies_and_escapes_values() {
        let pom = pom_xml(
            "demo",
            "android",
            "com.example",
            "demo-kmp-android",
            "0.0.5",
            "aar",
            &[Dependency {
                group: "org.example".into(),
                artifact: "dep".into(),
                version: "1.2.3".into(),
            }],
        );
        assert!(pom.contains("<packaging>aar</packaging>"));
        assert!(pom.contains("<groupId>org.example</groupId>"));
        assert!(pom.contains("<scope>compile</scope>"));
        assert!(maven::xml_escape("a<b&c").contains("&lt;"));
    }
}
