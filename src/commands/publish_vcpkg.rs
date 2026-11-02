//! Generate a vcpkg port from an existing ccgo SDK package.
//!
//! vcpkg is cross-platform, like Conan, so this is a top-level publish target
//! rather than a Windows manager. The port installs the prebuilt SDK artifacts
//! from `ccgo package` instead of rebuilding the project with vcpkg.

use super::{PublishCommand, RegistryType};
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha512};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub(super) fn publish(command: &PublishCommand, _verbose: bool) -> Result<()> {
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
    let port = port_name(name)?;
    let archive = find_package_zip(&project, command.path.as_deref(), &version)?;
    let output = project.join("target/release/vcpkg/ports").join(&port);
    fs::create_dir_all(&output)?;

    let (portfile, copied_archive) = match command.registry {
        RegistryType::Local => {
            let filename = format!("{port}-{version}.zip");
            let destination = output.join(&filename);
            fs::copy(&archive, &destination).with_context(|| {
                format!(
                    "Failed to copy SDK package {} into the vcpkg port",
                    archive.display()
                )
            })?;
            (local_portfile(&filename), Some(destination))
        }
        RegistryType::Private => {
            let template = command.archive_url_template.as_deref().context(
                "Private vcpkg publishing requires --archive-url-template for the SDK package URL",
            )?;
            let url = template
                .replace("{name}", &port)
                .replace("{version}", &version);
            let filename = format!("{port}-{version}.zip");
            let checksum = sha512(&archive)?;
            (remote_portfile(&url, &filename, &checksum), None)
        }
        RegistryType::Official => {
            bail!(
                "vcpkg has no binary upload endpoint and upstream rejects prebuilt-only ports; \
                 use --registry local or private. Submit a source-based port PR to vcpkg separately"
            )
        }
    };

    fs::write(
        output.join("vcpkg.json"),
        manifest(&port, &version, package)?,
    )?;
    fs::write(output.join("portfile.cmake"), portfile)?;
    fs::write(output.join("install.cmake"), install_cmake())?;
    fs::write(output.join("copyright"), copyright(package))?;

    println!("vcpkg port generated: {}", output.display());
    if let Some(archive) = copied_archive {
        println!("Bundled SDK package: {}", archive.display());
    }
    println!(
        "Test locally with:\n  vcpkg install {} --overlay-ports={}",
        port,
        project.join("target/release/vcpkg/ports").display()
    );
    if command.registry == RegistryType::Private {
        println!(
            "Commit the generated port directory to your private vcpkg registry, then update its baseline."
        );
    }
    Ok(())
}

fn find_package_zip(project: &Path, path: Option<&Path>, version: &str) -> Result<PathBuf> {
    if let Some(path) = path {
        if path.is_file() {
            if path.extension().and_then(|ext| ext.to_str()) != Some("zip") {
                bail!("vcpkg --path must be a CCGO_PACKAGE ZIP");
            }
            return Ok(path.to_path_buf());
        }
    }

    let mut candidates = Vec::new();
    let roots = path
        .map(|path| vec![path.to_path_buf(), path.join("target/release/package")])
        .unwrap_or_else(|| vec![project.join("target/release/package")]);
    for root in roots {
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root).max_depth(2).into_iter().flatten() {
            let file = entry.path();
            if !file.is_file() {
                continue;
            }
            let filename = file.file_name().unwrap_or_default().to_string_lossy();
            if filename.contains("_CCGO_PACKAGE-")
                && filename.ends_with(".zip")
                && filename.contains(version)
            {
                candidates.push(file.to_path_buf());
            }
        }
    }
    match candidates.as_slice() {
        [one] => Ok(one.clone()),
        [] => bail!(
            "No CCGO_PACKAGE ZIP found. Run `ccgo package --release` first or pass --path <zip>"
        ),
        _ => bail!(
            "Multiple CCGO_PACKAGE ZIPs matched; pass --path to select one: {}",
            candidates
                .iter()
                .map(|candidate| candidate.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn port_name(name: &str) -> Result<String> {
    let port: String = name
        .chars()
        .map(|character| {
            let character = character.to_ascii_lowercase();
            if character.is_ascii_alphanumeric() || character == '-' {
                character
            } else {
                '-'
            }
        })
        .collect();
    let port = port.trim_matches('-');
    if port.is_empty() {
        bail!("Project name cannot produce an empty vcpkg port name");
    }
    Ok(port.to_string())
}

fn manifest(port: &str, version: &str, package: &toml::Value) -> Result<String> {
    let description = package
        .get("description")
        .and_then(toml::Value::as_str)
        .unwrap_or(port);
    let license = package
        .get("license")
        .and_then(toml::Value::as_str)
        .unwrap_or("MIT");
    let homepage = package
        .get("repository")
        .and_then(toml::Value::as_str)
        .unwrap_or("");
    let version_field = if semver::Version::parse(version).is_ok() {
        format!("\"version\": \"{}\"", json_escape(version))
    } else {
        format!("\"version-string\": \"{}\"", json_escape(version))
    };
    let homepage = if homepage.is_empty() {
        String::new()
    } else {
        format!("  \"homepage\": \"{}\",\n", json_escape(homepage))
    };
    Ok(format!(
        r#"{{
  "name": "{}",
  {},
  "description": "{}",
{homepage}  "license": "{}",
  "supports": "windows | linux | osx"
}}
"#,
        json_escape(port),
        version_field,
        json_escape(description),
        json_escape(license),
    ))
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn sha512(path: &Path) -> Result<String> {
    let mut hasher = Sha512::new();
    hasher.update(fs::read(path)?);
    Ok(format!("{:x}", hasher.finalize()))
}

fn local_portfile(filename: &str) -> String {
    format!(
        r#"cmake_minimum_required(VERSION 3.15)
set(CCGO_ARCHIVE "${{CMAKE_CURRENT_LIST_DIR}}/{filename}")
vcpkg_extract_source_archive(SOURCE_PATH SOURCE_PATH ARCHIVE "${{CCGO_ARCHIVE}}")
include("${{CMAKE_CURRENT_LIST_DIR}}/install.cmake")
"#
    )
}

fn remote_portfile(url: &str, filename: &str, checksum: &str) -> String {
    format!(
        r#"cmake_minimum_required(VERSION 3.15)
vcpkg_download_distfile(CCGO_ARCHIVE
    URLS "{url}"
    FILENAME "{filename}"
    SHA512 {checksum}
)
vcpkg_extract_source_archive(SOURCE_PATH SOURCE_PATH ARCHIVE "${{CCGO_ARCHIVE}}")
include("${{CMAKE_CURRENT_LIST_DIR}}/install.cmake")
"#
    )
}

fn install_cmake() -> String {
    r#"set(CCGO_LINKAGE "shared")
if(VCPKG_LIBRARY_LINKAGE STREQUAL "static")
    set(CCGO_LINKAGE "static")
endif()

if(VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
    set(CCGO_ARCH "x86_64")
elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
    set(CCGO_ARCH "aarch64")
elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "x86")
    set(CCGO_ARCH "x86")
elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm")
    set(CCGO_ARCH "arm")
else()
    message(FATAL_ERROR "ccgo: unsupported vcpkg architecture ${VCPKG_TARGET_ARCHITECTURE}")
endif()

file(INSTALL "${SOURCE_PATH}/include/" DESTINATION "${CURRENT_PACKAGES_DIR}/include")

if(VCPKG_TARGET_IS_WINDOWS)
    if(VCPKG_TARGET_IS_MINGW)
        set(CCGO_TOOLCHAIN "mingw")
    else()
        set(CCGO_TOOLCHAIN "msvc")
    endif()
    set(CCGO_LIBDIR "${SOURCE_PATH}/lib/windows/${CCGO_LINKAGE}/${CCGO_ARCH}/${CCGO_TOOLCHAIN}")
    file(GLOB CCGO_LIBS "${CCGO_LIBDIR}/*.lib" "${CCGO_LIBDIR}/*.a")
    file(GLOB CCGO_DLLS "${CCGO_LIBDIR}/*.dll")
    if(CCGO_LIBS)
        file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
        file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
    endif()
    if(CCGO_DLLS)
        file(INSTALL ${CCGO_DLLS} DESTINATION "${CURRENT_PACKAGES_DIR}/bin")
        file(INSTALL ${CCGO_DLLS} DESTINATION "${CURRENT_PACKAGES_DIR}/debug/bin")
    endif()
elseif(VCPKG_TARGET_IS_LINUX)
    set(CCGO_LIBDIR "${SOURCE_PATH}/lib/linux/${CCGO_LINKAGE}/${CCGO_ARCH}")
    file(GLOB CCGO_LIBS "${CCGO_LIBDIR}/*.a" "${CCGO_LIBDIR}/*.so")
    if(CCGO_LIBS)
        file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
        file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
    endif()
elseif(VCPKG_TARGET_IS_OSX)
    file(GLOB CCGO_XCFRAMEWORKS "${SOURCE_PATH}/lib/macos/${CCGO_LINKAGE}/*.xcframework")
    foreach(CCGO_XCF IN LISTS CCGO_XCFRAMEWORKS)
        file(GLOB_RECURSE CCGO_LIBS
            "${CCGO_XCF}/macos-*${CCGO_ARCH}*/lib*.a"
            "${CCGO_XCF}/macos-*${CCGO_ARCH}*/lib*.dylib"
            "${CCGO_XCF}/macos-*/lib*.a"
            "${CCGO_XCF}/macos-*/lib*.dylib")
        if(CCGO_LIBS)
            file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
            file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
            break()
        endif()
    endforeach()
elseif(VCPKG_TARGET_IS_IOS)
    file(GLOB CCGO_XCFRAMEWORKS "${SOURCE_PATH}/lib/ios/${CCGO_LINKAGE}/*.xcframework")
    foreach(CCGO_XCF IN LISTS CCGO_XCFRAMEWORKS)
        if(VCPKG_TARGET_ARCHITECTURE STREQUAL "x86_64")
            set(CCGO_SLICE "*simulator*")
        else()
            set(CCGO_SLICE "ios-arm64")
        endif()
        file(GLOB_RECURSE CCGO_LIBS
            "${CCGO_XCF}/${CCGO_SLICE}/lib*.a"
            "${CCGO_XCF}/${CCGO_SLICE}/lib*.dylib")
        if(CCGO_LIBS)
            file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
            file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
            break()
        endif()
    endforeach()
elseif(VCPKG_TARGET_IS_ANDROID)
    if(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm64")
        set(CCGO_ABI "arm64-v8a")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "arm")
        set(CCGO_ABI "armeabi-v7a")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "x86")
        set(CCGO_ABI "x86")
    elseif(VCPKG_TARGET_ARCHITECTURE STREQUAL "x64")
        set(CCGO_ABI "x86_64")
    else()
        message(FATAL_ERROR "ccgo: unsupported Android architecture ${VCPKG_TARGET_ARCHITECTURE}")
    endif()
    set(CCGO_LIBDIR "${SOURCE_PATH}/lib/android/${CCGO_LINKAGE}/${CCGO_ABI}")
    file(GLOB CCGO_LIBS "${CCGO_LIBDIR}/*.a" "${CCGO_LIBDIR}/*.so")
    if(CCGO_LIBS)
        file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/lib")
        file(INSTALL ${CCGO_LIBS} DESTINATION "${CURRENT_PACKAGES_DIR}/debug/lib")
    endif()
endif()

file(GLOB CCGO_INSTALLED_LIBS "${CURRENT_PACKAGES_DIR}/lib/*")
if(NOT CCGO_INSTALLED_LIBS)
    message(FATAL_ERROR
        "ccgo: no ${CCGO_LINKAGE} libraries found for triplet ${TARGET_TRIPLET}. "
        "Rebuild the ccgo package with the matching platform and linkage.")
endif()

file(INSTALL "${CMAKE_CURRENT_LIST_DIR}/copyright"
     DESTINATION "${CURRENT_PACKAGES_DIR}/share/${PORT}")
"#
    .to_string()
}

fn copyright(package: &toml::Value) -> String {
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
    let license = package
        .get("license")
        .and_then(toml::Value::as_str)
        .unwrap_or("MIT");
    format!("Copyright {authors}\nLicense: {license}\n")
}
