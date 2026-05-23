//! OpenWrt toolchain detection (musl-libc GCC cross-compilers)
//!
//! OpenWrt uses musl libc, not glibc. The toolchains here are the musl.cc
//! pre-built cross-compilers installed by Dockerfile.openwrt.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};

use super::{find_executable, CompilerInfo, CompilerType, Toolchain};

/// OpenWrt target architecture
///
/// Names follow OpenWrt's own target/subtarget naming convention so that
/// `--arch mipsel_24kc` in ccgo maps directly to the right toolchain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpenwrtArch {
    /// Little-endian MIPS 24Kc (most common: TP-Link, Netgear, Asus)
    Mipsel24kc,
    /// Big-endian MIPS 24Kc
    Mips24kc,
    /// ARMv7-A Cortex-A7 hard-float with NEON
    ArmCortexA7,
    /// AArch64 (modern 64-bit ARM routers)
    Aarch64,
}

impl OpenwrtArch {
    /// musl.cc cross-compiler triple prefix (matches Dockerfile.openwrt paths)
    pub fn triple_prefix(self) -> &'static str {
        match self {
            OpenwrtArch::Mipsel24kc => "mipsel-linux-musl",
            OpenwrtArch::Mips24kc => "mips-linux-musl",
            OpenwrtArch::ArmCortexA7 => "arm-linux-musleabihf",
            OpenwrtArch::Aarch64 => "aarch64-linux-musl",
        }
    }

    /// Canonical arch string used in archive paths
    pub fn arch_string(self) -> &'static str {
        match self {
            OpenwrtArch::Mipsel24kc => "mipsel_24kc",
            OpenwrtArch::Mips24kc => "mips_24kc",
            OpenwrtArch::ArmCortexA7 => "arm_cortex_a7",
            OpenwrtArch::Aarch64 => "aarch64",
        }
    }

    /// `CMAKE_SYSTEM_PROCESSOR` value
    pub fn cmake_system_processor(self) -> &'static str {
        match self {
            OpenwrtArch::Mipsel24kc => "mipsel",
            OpenwrtArch::Mips24kc => "mips",
            OpenwrtArch::ArmCortexA7 => "arm",
            OpenwrtArch::Aarch64 => "aarch64",
        }
    }

    /// Extra C/CXX flags for CPU-specific tuning
    pub fn cpu_cflags(self) -> &'static str {
        match self {
            OpenwrtArch::Mipsel24kc => "-march=mips32r2 -mtune=24kc",
            OpenwrtArch::Mips24kc => "-march=mips32r2 -mtune=24kc",
            OpenwrtArch::ArmCortexA7 => {
                "-march=armv7-a -mcpu=cortex-a7 -mfpu=neon-vfpv4 -mfloat-abi=hard"
            }
            OpenwrtArch::Aarch64 => "",
        }
    }

    /// Parse from a user-supplied string
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "mipsel_24kc" | "mipsel" => Ok(OpenwrtArch::Mipsel24kc),
            "mips_24kc" | "mips" => Ok(OpenwrtArch::Mips24kc),
            "arm_cortex_a7" | "arm_cortex-a7" | "arm" => Ok(OpenwrtArch::ArmCortexA7),
            "aarch64" | "arm64" => Ok(OpenwrtArch::Aarch64),
            other => bail!(
                "Unknown OpenWrt architecture '{}'. Supported: mipsel_24kc, mips_24kc, arm_cortex_a7, aarch64",
                other
            ),
        }
    }
}

/// OpenWrt musl cross-compiler toolchain
pub struct OpenwrtToolchain {
    compiler: CompilerInfo,
    arch: OpenwrtArch,
}

impl OpenwrtToolchain {
    /// Detect toolchain for a specific OpenWrt target architecture.
    ///
    /// Looks for `{triple}-gcc` in PATH (installed by Dockerfile.openwrt).
    pub fn detect_for_arch(arch: OpenwrtArch) -> Result<Self> {
        let prefix = arch.triple_prefix();
        let cc_name = format!("{}-gcc", prefix);
        let cxx_name = format!("{}-g++", prefix);

        let cc = find_executable(&cc_name).ok_or_else(|| {
            anyhow::anyhow!(
                "OpenWrt cross-compiler not found: {}\n\
                 Use Docker to build OpenWrt targets: ccgo build openwrt --docker",
                cc_name
            )
        })?;
        let cxx = find_executable(&cxx_name).with_context(|| {
            format!("OpenWrt C++ cross-compiler not found: {}", cxx_name)
        })?;

        let version = super::get_compiler_version(&cc).unwrap_or_else(|| "unknown".to_string());
        Ok(Self {
            compiler: CompilerInfo { compiler_type: CompilerType::Gcc, cc, cxx, version },
            arch,
        })
    }

    /// Get the target architecture
    pub fn arch(&self) -> OpenwrtArch {
        self.arch
    }

    /// Path to the `ar` archiver for this toolchain
    pub fn ar_path(&self) -> PathBuf {
        let ar_name = format!("{}-ar", self.arch.triple_prefix());
        find_executable(&ar_name).unwrap_or_else(|| PathBuf::from("ar"))
    }

    /// CMake variables for cross-compilation
    pub fn cross_cmake_variables(&self) -> Vec<(String, String)> {
        let prefix = self.arch.triple_prefix();
        let sysroot = format!("/opt/cross/{}-cross/{}", prefix, prefix);

        let mut vars = vec![
            ("CMAKE_SYSTEM_NAME".to_string(), "Linux".to_string()),
            (
                "CMAKE_SYSTEM_PROCESSOR".to_string(),
                self.arch.cmake_system_processor().to_string(),
            ),
            ("CMAKE_C_COMPILER".to_string(), self.compiler.cc.display().to_string()),
            ("CMAKE_CXX_COMPILER".to_string(), self.compiler.cxx.display().to_string()),
            ("CMAKE_FIND_ROOT_PATH".to_string(), sysroot),
            ("CMAKE_FIND_ROOT_PATH_MODE_PROGRAM".to_string(), "NEVER".to_string()),
            ("CMAKE_FIND_ROOT_PATH_MODE_LIBRARY".to_string(), "ONLY".to_string()),
            ("CMAKE_FIND_ROOT_PATH_MODE_INCLUDE".to_string(), "ONLY".to_string()),
        ];

        let cflags = self.arch.cpu_cflags();
        if !cflags.is_empty() {
            vars.push(("CMAKE_C_FLAGS_INIT".to_string(), cflags.to_string()));
            vars.push(("CMAKE_CXX_FLAGS_INIT".to_string(), cflags.to_string()));
        }

        vars
    }

    /// Merge multiple static libraries into one using the arch-appropriate `ar`
    pub fn merge_static_libs(&self, src_libs: &[PathBuf], dst_lib: &PathBuf) -> Result<()> {
        if src_libs.is_empty() {
            bail!("No source libraries to merge");
        }

        if let Some(parent) = dst_lib.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let temp_dir = std::env::temp_dir().join(format!(
            "ccgo-openwrt-merge-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        ));
        std::fs::create_dir_all(&temp_dir)?;

        let ar_cmd = self.ar_path();

        for (idx, lib) in src_libs.iter().enumerate() {
            let extract_dir = temp_dir.join(format!("lib{}", idx));
            std::fs::create_dir_all(&extract_dir)?;

            let output = std::process::Command::new(&ar_cmd)
                .arg("x")
                .arg(lib)
                .current_dir(&extract_dir)
                .output()
                .context("Failed to run ar for extraction")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                std::fs::remove_dir_all(&temp_dir).ok();
                bail!("ar extraction failed for {}: {}", lib.display(), stderr);
            }
        }

        let mut object_files: Vec<PathBuf> = Vec::new();
        for entry in walkdir::WalkDir::new(&temp_dir) {
            let entry = entry?;
            let ext = entry.path().extension().and_then(|e| e.to_str());
            if matches!(ext, Some("o") | Some("obj")) {
                object_files.push(entry.path().to_path_buf());
            }
        }

        if object_files.is_empty() {
            std::fs::remove_dir_all(&temp_dir).ok();
            bail!("No object files found in source libraries");
        }

        if dst_lib.exists() {
            std::fs::remove_file(dst_lib)?;
        }

        let mut cmd = std::process::Command::new(&ar_cmd);
        cmd.arg("rcs").arg(dst_lib);
        for obj in &object_files {
            cmd.arg(obj);
        }

        let output = cmd.output().context("Failed to run ar for merging")?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            std::fs::remove_dir_all(&temp_dir).ok();
            bail!("ar merge failed: {}", stderr);
        }

        std::fs::remove_dir_all(&temp_dir).ok();
        Ok(())
    }
}

impl Toolchain for OpenwrtToolchain {
    fn name(&self) -> &str {
        "musl-gcc"
    }

    fn is_available(&self) -> bool {
        self.compiler.cc.exists() && self.compiler.cxx.exists()
    }

    fn path(&self) -> Option<PathBuf> {
        self.compiler.cc.parent().map(|p| p.to_path_buf())
    }

    fn cmake_variables(&self) -> Vec<(String, String)> {
        self.cross_cmake_variables()
    }

    fn validate(&self) -> Result<()> {
        if !self.compiler.cc.exists() {
            bail!("C cross-compiler not found at: {}", self.compiler.cc.display());
        }
        if !self.compiler.cxx.exists() {
            bail!("C++ cross-compiler not found at: {}", self.compiler.cxx.display());
        }
        Ok(())
    }
}
