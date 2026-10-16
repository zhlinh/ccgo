//! Prepare prebuilt CCGO KMP SDKs without changing the input or project tree.
use super::input::extract_zip as extract;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
struct Artifact {
    target: String,
    path: PathBuf,
    classifier: Option<String>,
}

#[derive(Serialize)]
struct Manifest {
    project: String,
    version: String,
    artifacts: Vec<Artifact>,
}

pub(super) struct PreparedSdk {
    pub manifest: PathBuf,
    pub guard: PathBuf,
    // Keep extracted artifacts alive until Gradle exits.
    _temporary: tempfile::TempDir,
}

impl PreparedSdk {
    pub fn prepare(input: &Path) -> Result<Self> {
        let temporary = tempfile::tempdir()?;
        let root = if input.is_file() {
            let destination = temporary.path().join("sdk");
            fs::create_dir(&destination)?;
            extract(input, &destination)?;
            destination
        } else {
            input.to_path_buf()
        };
        let root = find_root(&root)?;
        let info = root.join("meta/kmp/build_info.json");
        let metadata: serde_json::Value = serde_json::from_slice(
            &fs::read(&info)
                .with_context(|| format!("Missing KMP SDK metadata: {}", info.display()))?,
        )
        .context("Invalid KMP SDK build_info.json")?;
        let field = |name: &str| -> Result<String> {
            metadata[name]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .map(str::to_owned)
                .with_context(|| format!("KMP SDK metadata requires {name}"))
        };
        let artifacts = collect_artifacts(&root)?;
        let manifest = temporary.path().join("manifest.json");
        fs::write(
            &manifest,
            serde_json::to_vec(&Manifest {
                project: field("project")?,
                version: field("version")?,
                artifacts,
            })?,
        )?;
        let guard = temporary.path().join("guard.gradle");
        fs::write(
            &guard,
            r#"
gradle.taskGraph.whenReady {
    if (gradle.rootProject.allprojects.any { it.projectDir.canonicalPath == System.getenv('CCGO_PUBLISH_PROJECT_DIR') } &&
        !gradle.rootProject.allprojects.any {
        it.extensions.extraProperties.has('ccgoKmpSdkSupported') &&
        it.extensions.extraProperties.get('ccgoKmpSdkSupported') == true
    }) {
        throw new GradleException('Update the CCGO Gradle plugin: this version does not support prebuilt KMP SDK publishing.')
    }
}
"#,
        )?;
        Ok(Self {
            manifest,
            guard,
            _temporary: temporary,
        })
    }
}

fn collect_artifacts(root: &Path) -> Result<Vec<Artifact>> {
    let mut artifacts = Vec::new();
    for entry in walkdir::WalkDir::new(root.join("lib/kmp")) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            bail!(
                "KMP SDK must not contain symlinks: {}",
                entry.path().display()
            );
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let rel = entry
            .path()
            .strip_prefix(root.join("lib/kmp"))?
            .to_path_buf();
        let (target, classifier) = classify_artifact(&rel)?;
        if artifacts
            .iter()
            .any(|a: &Artifact| a.target == target && a.classifier == classifier)
        {
            bail!("Ambiguous KMP SDK artifacts for target {target}, classifier {classifier:?}");
        }
        artifacts.push(Artifact {
            target,
            classifier,
            path: fs::canonicalize(entry.path())?,
        });
    }
    if artifacts.is_empty() {
        bail!("KMP SDK contains no publishable artifacts under lib/kmp");
    }
    for artifact in &artifacts {
        if !artifacts
            .iter()
            .any(|a| a.target == artifact.target && a.classifier.is_none())
        {
            bail!(
                "KMP SDK target {} is missing its main artifact",
                artifact.target
            );
        }
    }
    artifacts.sort_by(|a, b| (&a.target, &a.classifier).cmp(&(&b.target, &b.classifier)));
    Ok(artifacts)
}

fn classify_artifact(rel: &Path) -> Result<(String, Option<String>)> {
    let parts: Vec<_> = rel.iter().map(|s| s.to_str().unwrap_or("")).collect();
    Ok(match parts.as_slice() {
        ["android", file] if file.ends_with(".aar") => ("android".to_string(), None),
        ["desktop", file] if file.ends_with(".jar") => {
            let classifier = if file.ends_with("-sources.jar") {
                Some("sources".into())
            } else if file.ends_with("-javadoc.jar") {
                Some("javadoc".into())
            } else {
                None
            };
            ("desktop".to_string(), classifier)
        }
        ["native", target, kind, file] if file.ends_with(".klib") => {
            let classifier = match *kind {
                "klib" => None,
                "cinterop" => Some(file.trim_end_matches(".klib").to_string()),
                _ => bail!("Unsupported KMP SDK artifact: {}", rel.display()),
            };
            (target.to_lowercase(), classifier)
        }
        _ => bail!("Unsupported KMP SDK artifact: {}", rel.display()),
    })
}

fn find_root(root: &Path) -> Result<PathBuf> {
    if root.join("lib/kmp").is_dir() {
        return Ok(root.to_path_buf());
    }
    let candidates = fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.join("lib/kmp").is_dir())
        .collect::<Vec<_>>();
    match candidates.as_slice() {
        [path] => Ok(path.clone()),
        _ => {
            bail!("Expected one CCGO KMP SDK root containing lib/kmp and meta/kmp/build_info.json")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn sdk_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for (name, data) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }

    #[test]
    fn prepares_ccgo_sdk_zip_and_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let zip = tmp.path().join("SDK.zip");
        sdk_zip(
            &zip,
            &[
                (
                    "SDK/meta/kmp/build_info.json",
                    br#"{"project":"stdcomm","version":"0.0.5"}"#,
                ),
                ("SDK/lib/kmp/android/stdcomm-kmp-release.aar", b"aar"),
                ("SDK/lib/kmp/desktop/stdcomm.jar", b"jar"),
                ("SDK/lib/kmp/native/iosArm64/klib/stdcomm.klib", b"klib"),
                (
                    "SDK/lib/kmp/native/iosArm64/cinterop/stdcomm-cinterop.klib",
                    b"interop",
                ),
            ],
        );
        let sdk = PreparedSdk::prepare(&zip).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&sdk.manifest).unwrap()).unwrap();
        assert_eq!(manifest["version"], "0.0.5");
        assert_eq!(manifest["artifacts"].as_array().unwrap().len(), 4);
        for artifact in manifest["artifacts"].as_array().unwrap() {
            assert!(Path::new(artifact["path"].as_str().unwrap()).is_file());
        }
        let unpacked = tmp.path().join("unpacked");
        fs::create_dir(&unpacked).unwrap();
        extract(&zip, &unpacked).unwrap();
        PreparedSdk::prepare(&unpacked.join("SDK")).unwrap();
    }

    #[test]
    fn rejects_invalid_unsafe_and_ambiguous_sdk_archives() {
        let tmp = tempfile::tempdir().unwrap();
        let zip = tmp.path().join("SDK.zip");
        fs::write(&zip, b"invalid").unwrap();
        assert!(PreparedSdk::prepare(&zip).is_err());
        sdk_zip(&zip, &[("../outside.aar", b"aar")]);
        assert!(PreparedSdk::prepare(&zip).is_err());
        assert!(!tmp.path().join("outside.aar").exists());
        sdk_zip(&zip, &[("lib/kmp/android/lib.aar", b"aar")]);
        assert!(PreparedSdk::prepare(&zip).is_err());
        sdk_zip(
            &zip,
            &[
                (
                    "meta/kmp/build_info.json",
                    br#"{"project":"stdcomm","version":"0.0.5"}"#,
                ),
                ("lib/kmp/android/debug.aar", b"debug"),
                ("lib/kmp/android/release.aar", b"release"),
            ],
        );
        assert!(PreparedSdk::prepare(&zip)
            .err()
            .unwrap()
            .to_string()
            .contains("Ambiguous"));
    }
}
