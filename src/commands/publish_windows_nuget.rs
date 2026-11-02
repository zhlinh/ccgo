//! Publish a Windows SDK as a native NuGet package.

use super::{input, AppleManager, PublishCommand, RegistryType};
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;

const NUGET_OFFICIAL: &str = "https://api.nuget.org/v3/index.json";

pub(super) fn publish(command: &PublishCommand, verbose: bool) -> Result<()> {
    if !matches!(command.manager, AppleManager::Nuget | AppleManager::All) {
        bail!("Windows publishing supports --manager nuget only");
    }
    if command.tag.is_some() {
        bail!("Windows publishing does not use --tag; use --version instead");
    }

    let project = std::env::current_dir()?;
    let configuration: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("CCGO.toml"))?)?;
    let package = configuration
        .get("project")
        .or_else(|| configuration.get("package"))
        .context("CCGO.toml requires [project] or [package]")?;
    let name = package
        .get("name")
        .and_then(toml::Value::as_str)
        .context("Project name is missing")?;
    let version = command
        .version
        .clone()
        .or_else(|| {
            package
                .get("version")
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        })
        .context("Project version is missing")?;
    let id = nuget_id(name)?;

    let nupkg = match windows_input(command, &project, &version, verbose)? {
        WindowsInput::Nupkg(path) => path,
        WindowsInput::Sdk(path) => build_nupkg(&path, &id, &version, package)?,
    };
    validate_nupkg(&nupkg)?;

    match command.registry {
        RegistryType::Local => {
            println!("NuGet package created: {}", nupkg.display());
            Ok(())
        }
        RegistryType::Official => {
            let key = std::env::var("NUGET_API_KEY")
                .context("Publishing to nuget.org requires NUGET_API_KEY")?;
            push(&nupkg, NUGET_OFFICIAL, Some(&key), verbose)
        }
        RegistryType::Private => {
            let source = command
                .remote
                .as_deref()
                .context("Private NuGet publishing requires --remote <feed-url>")?;
            let key = std::env::var("NUGET_API_KEY").ok();
            push(&nupkg, source, key.as_deref(), verbose)
        }
    }
}

enum WindowsInput {
    Nupkg(PathBuf),
    Sdk(PathBuf),
}

fn windows_input(
    command: &PublishCommand,
    project: &Path,
    version: &str,
    verbose: bool,
) -> Result<WindowsInput> {
    if let Some(path) = &command.path {
        if path.is_file() {
            return match path.extension().and_then(|ext| ext.to_str()) {
                Some("nupkg") => Ok(WindowsInput::Nupkg(path.clone())),
                Some("zip") => Ok(WindowsInput::Sdk(path.clone())),
                _ => bail!("Windows publish --path must be an SDK ZIP or a .nupkg"),
            };
        }
        return find_sdk_zip(path, version).map(WindowsInput::Sdk);
    }

    if command.skip_build {
        bail!("--skip-build without --path cannot choose a Windows SDK ZIP");
    }

    println!("Building the release Windows SDK with ccgo...");
    let build = crate::commands::build::BuildCommand::for_windows_publish()?;
    let results = build.build_artifacts(verbose)?;
    let sdk = results
        .first()
        .map(|result| result.sdk_archive.clone())
        .context("Windows build did not produce an SDK archive")?;
    let _ = project;
    Ok(WindowsInput::Sdk(sdk))
}

fn find_sdk_zip(directory: &Path, version: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();
    let roots = [
        directory.join("target/release/windows"),
        directory.join("target/windows"),
        directory.to_path_buf(),
    ];
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root).max_depth(2).into_iter().flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            if filename.starts_with("ARCHIVE")
                || !filename.contains("WINDOWS_SDK")
                || !filename.ends_with(".zip")
                || filename.contains("-SYMBOLS.zip")
            {
                continue;
            }
            if filename.contains(version) {
                candidates.push(path.to_path_buf());
            }
        }
    }
    match candidates.as_slice() {
        [one] => Ok(one.clone()),
        [] => bail!(
            "No Windows SDK ZIP for version {version} found under {}",
            directory.display()
        ),
        _ => bail!(
            "Multiple Windows SDK ZIPs matched; pass --path to select one: {}",
            candidates
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn nuget_id(name: &str) -> Result<String> {
    let id: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect();
    if id.is_empty() {
        bail!("Project name cannot produce an empty NuGet package ID");
    }
    Ok(id)
}

fn nuget_arch(arch: &str) -> Result<&'static str> {
    Ok(match arch {
        "x86_64" | "x64" => "x64",
        "aarch64" | "arm64" => "arm64",
        "x86" => "x86",
        "arm" => "arm",
        other => bail!("Unsupported Windows architecture for NuGet: {other}"),
    })
}

#[derive(Default)]
struct PackageFiles {
    include: Vec<(String, Vec<u8>)>,
    static_libs: Vec<(String, Vec<u8>)>,
    shared_libs: Vec<(String, Vec<u8>)>,
    runtimes: Vec<(String, Vec<u8>)>,
}

fn collect_package_files(sdk_root: &Path, id: &str) -> Result<PackageFiles> {
    let mut files = PackageFiles::default();

    let include_root = sdk_root.join("include");
    if include_root.is_dir() {
        for entry in WalkDir::new(&include_root).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(&include_root)?
                .to_string_lossy()
                .replace('\\', "/");
            files.include.push((
                format!("build/native/include/{relative}"),
                fs::read(entry.path())?,
            ));
        }
    }

    for (kind, target) in [
        ("static", &mut files.static_libs),
        ("shared", &mut files.shared_libs),
    ] {
        let root = sdk_root.join("lib/windows").join(kind);
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root).into_iter().flatten() {
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry.path().strip_prefix(&root)?;
            let parts: Vec<_> = relative.components().collect();
            if parts.len() < 3 {
                continue;
            }
            let arch = nuget_arch(&parts[0].as_os_str().to_string_lossy())?;
            let toolchain = parts[1].as_os_str().to_string_lossy();
            let filename = parts[parts.len() - 1].as_os_str().to_string_lossy();
            let destination = format!("build/native/lib/{arch}/{toolchain}/{kind}/{filename}");
            let bytes = fs::read(entry.path())?;
            if kind == "shared" && filename.ends_with(".dll") {
                files.runtimes.push((
                    format!("runtimes/win-{arch}/native/{filename}"),
                    bytes.clone(),
                ));
            }
            target.push((destination, bytes));
        }
    }

    if files.static_libs.is_empty() && files.shared_libs.is_empty() {
        bail!("Windows SDK ZIP has no lib/windows/static or shared libraries");
    }
    let _ = id;
    Ok(files)
}

fn build_nupkg(sdk_zip: &Path, id: &str, version: &str, package: &toml::Value) -> Result<PathBuf> {
    let temporary = tempfile::tempdir()?;
    let sdk_root = temporary.path().join("sdk");
    fs::create_dir_all(&sdk_root)?;
    input::extract_zip(sdk_zip, &sdk_root)?;
    let files = collect_package_files(&sdk_root, id)?;

    let output_dir = std::env::current_dir()?.join("target/release/nuget");
    fs::create_dir_all(&output_dir)?;
    let output = output_dir.join(format!("{id}.{version}.nupkg"));
    let mut archive = zip::ZipWriter::new(fs::File::create(&output)?);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let nuspec = nuspec_xml(id, version, package);
    add(
        &mut archive,
        &options,
        &format!("{id}.nuspec"),
        nuspec.as_bytes(),
    )?;
    for (name, bytes) in &files.include {
        add(&mut archive, &options, name, bytes)?;
    }
    for (name, bytes) in &files.static_libs {
        add(&mut archive, &options, name, bytes)?;
    }
    for (name, bytes) in &files.shared_libs {
        add(&mut archive, &options, name, bytes)?;
    }
    for (name, bytes) in &files.runtimes {
        add(&mut archive, &options, name, bytes)?;
    }

    let static_names = library_names(&files.static_libs, "msvc", "x64");
    let shared_names = library_names(&files.shared_libs, "msvc", "x64");
    let prop = build_props(id);
    let targets = build_targets(id, &static_names, &shared_names, !shared_names.is_empty());
    add(
        &mut archive,
        &options,
        &format!("build/native/{id}.props"),
        prop.as_bytes(),
    )?;
    add(
        &mut archive,
        &options,
        &format!("build/native/{id}.targets"),
        targets.as_bytes(),
    )?;

    let nuspec_name = format!("{id}.nuspec");
    add_content_types(&mut archive, &options, &files)?;
    add_relationships(&mut archive, &options, id, &nuspec_name)?;
    add_core_properties(&mut archive, &options, id, version, package)?;

    archive.finish()?;
    Ok(output)
}

fn library_names(files: &[(String, Vec<u8>)], toolchain: &str, arch: &str) -> Vec<String> {
    files
        .iter()
        .filter_map(|(name, _)| {
            let prefix = format!("build/native/lib/{arch}/{toolchain}/");
            let rest = name.strip_prefix(&prefix)?;
            let filename = rest.rsplit('/').next()?;
            if !filename.ends_with(".lib") {
                return None;
            }
            Some(filename.trim_end_matches(".lib").to_string())
        })
        .collect()
}

fn add(
    archive: &mut zip::ZipWriter<fs::File>,
    options: &SimpleFileOptions,
    name: &str,
    bytes: &[u8],
) -> Result<()> {
    archive.start_file(name, *options)?;
    archive.write_all(bytes)?;
    Ok(())
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn metadata(package: &toml::Value, key: &str) -> Option<String> {
    package
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::to_string)
}

fn nuspec_xml(id: &str, version: &str, package: &toml::Value) -> String {
    let authors = package
        .get("authors")
        .and_then(toml::Value::as_array)
        .map(|authors| {
            authors
                .iter()
                .filter_map(toml::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|authors| !authors.is_empty())
        .or_else(|| metadata(package, "group_id"))
        .unwrap_or_else(|| "ccgo".to_string());
    let description = metadata(package, "description").unwrap_or_else(|| id.to_string());
    let license = metadata(package, "license")
        .map(|license| {
            format!(
                "    <license type=\"expression\">{}</license>\n",
                xml_escape(&license)
            )
        })
        .unwrap_or_default();
    let project_url = metadata(package, "repository")
        .map(|url| format!("    <projectUrl>{}</projectUrl>\n", xml_escape(&url)))
        .unwrap_or_default();
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<package xmlns="http://schemas.microsoft.com/packaging/2013/05/nuspec.xsd">
  <metadata>
    <id>{id}</id>
    <version>{version}</version>
    <authors>{authors}</authors>
    <description>{description}</description>
{license}{project_url}    <tags>native;windows;c++;ccgo</tags>
  </metadata>
</package>
"#,
        id = xml_escape(id),
        version = xml_escape(version),
        authors = xml_escape(&authors),
        description = xml_escape(&description),
    )
}

fn msbuild_property(id: &str) -> String {
    let property: String = id
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect();
    if property.is_empty() || property.starts_with(|character: char| character.is_ascii_digit()) {
        format!("Ccgo{property}")
    } else {
        property
    }
}

fn build_props(id: &str) -> String {
    let property = msbuild_property(id);
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <PropertyGroup>
    <{property}NativeRoot>$(MSBuildThisFileDirectory)</{property}NativeRoot>
    <{property}Linkage Condition="'$({property}Linkage)' == ''">shared</{property}Linkage>
  </PropertyGroup>
  <ItemDefinitionGroup>
    <ClCompile>
      <AdditionalIncludeDirectories>$({property}NativeRoot)include;%(AdditionalIncludeDirectories)</AdditionalIncludeDirectories>
    </ClCompile>
  </ItemDefinitionGroup>
</Project>
"#
    )
}

fn build_targets(
    id: &str,
    static_names: &[String],
    shared_names: &[String],
    has_shared: bool,
) -> String {
    let property = msbuild_property(id);
    let default_linkage = if has_shared { "shared" } else { "static" };
    let static_deps = static_names
        .iter()
        .map(|name| format!("{name}.lib"))
        .collect::<Vec<_>>()
        .join(";");
    let shared_deps = shared_names
        .iter()
        .map(|name| format!("{name}.lib"))
        .collect::<Vec<_>>()
        .join(";");
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Project xmlns="http://schemas.microsoft.com/developer/msbuild/2003">
  <PropertyGroup>
    <{property}Linkage Condition="'$({property}Linkage)' == ''">{default_linkage}</{property}Linkage>
  </PropertyGroup>
  <ItemDefinitionGroup Condition="'$({property}Linkage)' == 'static'">
    <Link>
      <AdditionalLibraryDirectories>$({property}NativeRoot)lib\$(Platform)\msvc\static;%(AdditionalLibraryDirectories)</AdditionalLibraryDirectories>
      <AdditionalDependencies>{static_deps};%(AdditionalDependencies)</AdditionalDependencies>
    </Link>
  </ItemDefinitionGroup>
  <ItemDefinitionGroup Condition="'$({property}Linkage)' == 'shared'">
    <Link>
      <AdditionalLibraryDirectories>$({property}NativeRoot)lib\$(Platform)\msvc\shared;%(AdditionalLibraryDirectories)</AdditionalLibraryDirectories>
      <AdditionalDependencies>{shared_deps};%(AdditionalDependencies)</AdditionalDependencies>
    </Link>
  </ItemDefinitionGroup>
</Project>
"#
    )
}

fn add_content_types(
    archive: &mut zip::ZipWriter<fs::File>,
    options: &SimpleFileOptions,
    files: &PackageFiles,
) -> Result<()> {
    let mut extensions = std::collections::BTreeSet::new();
    for (name, _) in files
        .include
        .iter()
        .chain(files.static_libs.iter())
        .chain(files.shared_libs.iter())
        .chain(files.runtimes.iter())
    {
        if let Some(extension) = name.rsplit('.').next() {
            extensions.insert(extension.to_string());
        }
    }
    let defaults = extensions
        .into_iter()
        .map(|extension| {
            format!(
                "  <Default Extension=\"{}\" ContentType=\"application/octet\" />\n",
                xml_escape(&extension)
            )
        })
        .collect::<String>();
    let content = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" />
  <Default Extension="psmdcp" ContentType="application/vnd.openxmlformats-package.core-properties+xml" />
  <Default Extension="nuspec" ContentType="application/octet" />
  <Default Extension="props" ContentType="application/octet" />
  <Default Extension="targets" ContentType="application/octet" />
{defaults}</Types>
"#
    );
    add(archive, options, "[Content_Types].xml", content.as_bytes())
}

fn add_relationships(
    archive: &mut zip::ZipWriter<fs::File>,
    options: &SimpleFileOptions,
    id: &str,
    nuspec: &str,
) -> Result<()> {
    let content = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Type="http://schemas.microsoft.com/packaging/2010/07/manifest" Target="/{nuspec}" Id="R1" />
  <Relationship Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties" Target="/package/services/metadata/core-properties/{id}.psmdcp" Id="R2" />
</Relationships>
"#
    );
    add(archive, options, "_rels/.rels", content.as_bytes())
}

fn add_core_properties(
    archive: &mut zip::ZipWriter<fs::File>,
    options: &SimpleFileOptions,
    id: &str,
    version: &str,
    package: &toml::Value,
) -> Result<()> {
    let authors = package
        .get("authors")
        .and_then(toml::Value::as_array)
        .map(|authors| {
            authors
                .iter()
                .filter_map(toml::Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "ccgo".to_string());
    let content = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<coreProperties xmlns="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
  xmlns:dc="http://purl.org/dc/elements/1.1/"
  xmlns:dcterms="http://purl.org/dc/terms/"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:creator>{authors}</dc:creator>
  <dc:description>{id}</dc:description>
  <dc:identifier>{id}</dc:identifier>
  <version>{version}</version>
  <lastModifiedBy>ccgo</lastModifiedBy>
</coreProperties>
"#,
        authors = xml_escape(&authors),
        id = xml_escape(id),
        version = xml_escape(version),
    );
    add(
        archive,
        options,
        &format!("package/services/metadata/core-properties/{id}.psmdcp"),
        content.as_bytes(),
    )
}

fn validate_nupkg(path: &Path) -> Result<()> {
    let mut archive = zip::ZipArchive::new(fs::File::open(path)?)?;
    let mut has_nuspec = false;
    for index in 0..archive.len() {
        if archive.by_index(index)?.name().ends_with(".nuspec") {
            has_nuspec = true;
            break;
        }
    }
    if !has_nuspec {
        bail!("NuGet package is missing a .nuspec: {}", path.display());
    }
    Ok(())
}

fn push(path: &Path, source: &str, api_key: Option<&str>, verbose: bool) -> Result<()> {
    let mut command = Command::new("dotnet");
    command
        .args(["nuget", "push"])
        .arg(path)
        .args(["--source", source, "--skip-duplicate"]);
    if let Some(api_key) = api_key {
        command.args(["--api-key", api_key]);
    }
    if verbose {
        eprintln!("Running: {:?}", command);
    }
    let status = command.status().context(
        "Failed to run `dotnet`; install the .NET SDK and ensure `dotnet nuget push` is available",
    )?;
    if !status.success() {
        bail!("NuGet push failed");
    }
    println!("Pushed {} to {}", path.display(), source);
    Ok(())
}
