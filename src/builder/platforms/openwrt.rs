//! OpenWrt platform builder
//!
//! Builds static and shared libraries for OpenWrt using musl-libc cross-compilers.
//! Unlike the Linux builder (which targets glibc), this uses musl-based toolchains
//! from musl.cc — the same C library OpenWrt itself uses.

use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::builder::archive::{
    get_unified_include_path, ArchiveBuilder, ARCHIVE_DIR_OBJ, ARCHIVE_DIR_SHARED,
    ARCHIVE_DIR_STATIC,
};
use crate::builder::cmake::{BuildType, CMakeConfig};
use crate::builder::toolchains::openwrt::{OpenwrtArch, OpenwrtToolchain};
use crate::builder::toolchains::Toolchain;
use crate::builder::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;

/// OpenWrt platform builder
pub struct OpenwrtBuilder;

impl OpenwrtBuilder {
    pub fn new() -> Self {
        Self
    }

    fn merge_module_static_libs(
        &self,
        build_dir: &Path,
        lib_name: &str,
        verbose: bool,
        toolchain: &OpenwrtToolchain,
    ) -> Result<()> {
        let out_dir = build_dir.join("out");
        if !out_dir.exists() {
            return Ok(());
        }

        let main_lib_name = format!("lib{}.a", lib_name);
        let main_lib_path = out_dir.join(&main_lib_name);

        if main_lib_path.exists() {
            if let Ok(metadata) = std::fs::metadata(&main_lib_path) {
                if metadata.len() > 0 {
                    for entry in std::fs::read_dir(&out_dir)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_file() && path != main_lib_path {
                            if let Some(ext) = path.extension() {
                                if ext == "a" {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                    return Ok(());
                }
            }
        }

        let mut module_libs: Vec<PathBuf> = Vec::new();
        for entry in std::fs::read_dir(&out_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "a" {
                        module_libs.push(path);
                    }
                }
            }
        }

        module_libs.retain(|p| p != &main_lib_path);

        if module_libs.is_empty() {
            return Ok(());
        }

        if verbose {
            eprintln!("    Merging {} module libraries into {}", module_libs.len(), main_lib_name);
        }

        toolchain.merge_static_libs(&module_libs, &main_lib_path)?;

        for lib in &module_libs {
            let _ = std::fs::remove_file(lib);
        }

        Ok(())
    }

    fn build_link_type_for_arch(
        &self,
        ctx: &BuildContext,
        link_type: &str,
        arch: &str,
        toolchain: &OpenwrtToolchain,
    ) -> Result<PathBuf> {
        let build_dir = ctx.cmake_build_dir.join(link_type).join(arch);
        let install_dir = build_dir.join("install");
        let build_shared = link_type == "shared";

        let mut cmake = CMakeConfig::new(ctx.project_root.clone(), build_dir.clone())
            .build_type(if ctx.options.release { BuildType::Release } else { BuildType::Debug })
            .install_prefix(install_dir)
            .variable("CCGO_BUILD_STATIC", if build_shared { "OFF" } else { "ON" })
            .variable("CCGO_BUILD_SHARED", if build_shared { "ON" } else { "OFF" })
            .variable("CCGO_BUILD_SHARED_LIBS", if build_shared { "ON" } else { "OFF" })
            .variable("CCGO_LIB_NAME", ctx.lib_name())
            .jobs(ctx.jobs())
            .verbose(ctx.options.verbose);

        if let Some(cmake_dir) = ctx.ccgo_cmake_dir() {
            cmake = cmake.variable("CCGO_CMAKE_DIR", cmake_dir.display().to_string());
        }

        cmake = cmake.variable(
            "CCGO_CONFIG_PRESET_VISIBILITY",
            ctx.symbol_visibility().to_string(),
        );

        if let Some(deps_map) = ctx.deps_map() {
            cmake = cmake.variable("CCGO_CONFIG_DEPS_MAP", deps_map);
        }

        if let Ok(feature_defines) = ctx.cmake_feature_defines() {
            if !feature_defines.is_empty() {
                cmake = cmake.feature_definitions(&feature_defines);
            }
        }

        if let Some(cache) = ctx.compiler_cache() {
            cmake = cmake.compiler_cache(cache);
        }

        for (key, val) in toolchain.cmake_variables() {
            cmake = cmake.variable(key, val);
        }

        let user = ctx.cmake_user_config("openwrt");
        cmake = cmake
            .user_arguments(user.arguments)
            .user_c_flags(user.c_flags)
            .user_cpp_flags(user.cpp_flags)
            .user_cmake_files(ctx.cmake_user_files("openwrt"));

        cmake.configure_build_install()?;

        if !build_shared {
            self.merge_module_static_libs(&build_dir, ctx.lib_name(), ctx.options.verbose, toolchain)?;
        }

        Ok(build_dir)
    }

    fn find_lib_dir(&self, build_dir: &Path) -> Option<PathBuf> {
        let candidates = [
            build_dir.join("out"),
            build_dir.join("install/lib"),
            build_dir.join("lib"),
        ];
        candidates.into_iter().find(|d| d.exists())
    }

    fn build_arch(
        &self,
        ctx: &BuildContext,
        arch: &str,
        archive: &ArchiveBuilder,
        symbols_temp: &Path,
    ) -> Result<bool> {
        let openwrt_arch = OpenwrtArch::parse(arch)?;
        let toolchain = OpenwrtToolchain::detect_for_arch(openwrt_arch)?;
        let mut found_symbols = false;

        if matches!(ctx.options.link_type, LinkType::Static | LinkType::Both) {
            let build_dir = self.build_link_type_for_arch(ctx, "static", arch, &toolchain)?;
            if let Some(lib_dir) = self.find_lib_dir(&build_dir) {
                let dest =
                    format!("lib/{}/{}/{}", self.platform_name(), ARCHIVE_DIR_STATIC, arch);
                archive.add_directory_filtered(&lib_dir, &dest, &["a"])?;
            }
        }

        if matches!(ctx.options.link_type, LinkType::Shared | LinkType::Both) {
            let build_dir = self.build_link_type_for_arch(ctx, "shared", arch, &toolchain)?;
            if let Some(lib_dir) = self.find_lib_dir(&build_dir) {
                let dest =
                    format!("lib/{}/{}/{}", self.platform_name(), ARCHIVE_DIR_SHARED, arch);
                archive.add_directory_filtered(&lib_dir, &dest, &["so", "a"])?;
                found_symbols = self.collect_symbols(&lib_dir, symbols_temp, arch)?;
            }
        }

        Ok(found_symbols)
    }

    fn collect_symbols(
        &self,
        lib_dir: &Path,
        symbols_temp: &Path,
        arch: &str,
    ) -> Result<bool> {
        let obj_arch_dir = symbols_temp
            .join(ARCHIVE_DIR_OBJ)
            .join(self.platform_name())
            .join(arch);
        std::fs::create_dir_all(&obj_arch_dir)?;
        let mut found = false;
        for entry in std::fs::read_dir(lib_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "so") {
                let file_name = path.file_name().unwrap();
                std::fs::copy(&path, obj_arch_dir.join(file_name))?;
                found = true;
            }
        }
        Ok(found)
    }
}

fn resolve_arches(ctx: &BuildContext) -> Vec<String> {
    if ctx.options.architectures.is_empty() {
        vec!["mipsel_24kc".to_string(), "aarch64".to_string()]
    } else {
        ctx.options.architectures.clone()
    }
}

impl PlatformBuilder for OpenwrtBuilder {
    fn platform_name(&self) -> &str {
        "openwrt"
    }

    fn default_architectures(&self) -> Vec<String> {
        vec!["mipsel_24kc".to_string(), "aarch64".to_string()]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        if !crate::builder::cmake::is_cmake_available() {
            bail!("CMake is required for OpenWrt builds. Please install CMake.");
        }

        let arches = resolve_arches(ctx);
        for arch in &arches {
            let openwrt_arch = OpenwrtArch::parse(arch)?;
            OpenwrtToolchain::detect_for_arch(openwrt_arch).with_context(|| {
                format!(
                    "musl cross-compiler for '{}' not found.\n\
                     OpenWrt requires musl-based toolchains. Use Docker:\n  \
                     ccgo build openwrt --docker",
                    arch
                )
            })?;
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();
        self.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for OpenWrt...", ctx.lib_name());
        }

        ctx.materialize_source_deps(self.platform_name())?;
        std::fs::create_dir_all(&ctx.output_dir)?;

        let archive = ArchiveBuilder::new(
            ctx.lib_name(),
            ctx.version(),
            ctx.publish_suffix(),
            ctx.options.release,
            "openwrt",
            ctx.output_dir.clone(),
        )?;

        let arches = resolve_arches(ctx);
        let link_type_str = ctx.options.link_type.to_string();
        let symbols_temp = std::env::temp_dir().join(format!("ccgo-symbols-{}", ctx.lib_name()));
        let mut found_symbols = false;

        for arch in &arches {
            if ctx.options.verbose {
                eprintln!("Building for OpenWrt arch: {}", arch);
            }
            if self.build_arch(ctx, arch, &archive, &symbols_temp)? {
                found_symbols = true;
            }
        }

        let include_source = ctx.include_source_dir();
        if include_source.exists() {
            let include_path = get_unified_include_path(ctx.lib_name(), &include_source);
            archive.add_directory(&include_source, &include_path)?;
        }

        let sdk_archive = archive.create_sdk_archive(&arches, &link_type_str)?;

        let symbols_archive_result = if found_symbols {
            Some(archive.create_symbols_archive(&symbols_temp)?)
        } else {
            None
        };
        if symbols_temp.exists() {
            std::fs::remove_dir_all(&symbols_temp).ok();
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: symbols_archive_result,
            aar_archive: None,
            duration_secs: start.elapsed().as_secs_f64(),
            architectures: arches,
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        crate::utils::paths::clean_ccgo_build_platform(&ctx.ccgo_build_root, "openwrt")?;

        for dir in &[
            ctx.project_root.join("target/release/openwrt"),
            ctx.project_root.join("target/debug/openwrt"),
            ctx.project_root.join("target/openwrt"),
        ] {
            if dir.exists() {
                std::fs::remove_dir_all(dir)
                    .with_context(|| format!("Failed to clean {}", dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for OpenwrtBuilder {
    fn default() -> Self {
        Self::new()
    }
}
