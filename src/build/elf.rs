//! ELF binary file parsing for library information extraction
//!
//! This module provides functions to extract architecture information
//! from ELF binaries, including Android NDK version and API level
//! from the .note.android.ident section.


/// ELF e_machine values to architecture names
const ELF_MACHINE_MAP: &[(u16, &str)] = &[
    (0x03, "x86"),
    (0x3E, "x86_64"),
    (0x28, "arm"),
    (0xB7, "aarch64"),
    (0x08, "mips"),
    (0xF3, "riscv"),
];

/// Mach-O CPU types to architecture names
#[allow(dead_code)]
const MACHO_CPU_MAP: &[(u32, &str)] = &[
    (0x00000007, "x86"),
    (0x01000007, "x86_64"),
    (0x0000000C, "arm"),
    (0x0100000C, "arm64"),
];

/// PE/COFF Machine types to architecture names
#[allow(dead_code)]
const PE_MACHINE_MAP: &[(u16, &str)] = &[
    (0x014c, "x86"),
    (0x8664, "x64"),
    (0xAA64, "ARM64"),
    (0x01c0, "ARM"),
    (0x01c4, "ARMv7"),
];

/// Android NDK information extracted from ELF .note.android.ident section
#[derive(Debug, Clone)]
pub struct AndroidNdkInfo {
    pub ndk_version: Option<String>,
    pub api_level: Option<u16>,
}

/// Library information extracted from binary files
#[derive(Debug, Clone)]
pub struct LibraryInfo {
    pub arch: Option<String>,
    pub ndk_info: Option<AndroidNdkInfo>,
}

impl LibraryInfo {
    /// Format the library info as a display string
    ///
    /// Returns format like: " [aarch64, NDK r27, API 24]"
    pub fn to_display_string(&self) -> String {
        let mut parts = Vec::new();

        if let Some(arch) = &self.arch {
            parts.push(arch.clone());
        }

        if let Some(ndk) = &self.ndk_info {
            if let Some(version) = &ndk.ndk_version {
                parts.push(format!("NDK {}", version));
            }
            if let Some(api) = ndk.api_level {
                parts.push(format!("API {}", api));
            }
        }

        if parts.is_empty() {
            String::new()
        } else {
            format!(" [{}]", parts.join(", "))
        }
    }
}

/// Parse ELF file to get architecture
pub fn parse_elf_arch(data: &[u8]) -> Option<String> {
    if data.len() < 20 {
        return None;
    }

    // Check ELF magic
    if &data[..4] != b"\x7fELF" {
        return None;
    }

    // Get endianness: 1 = little, 2 = big
    let is_little_endian = data[5] == 1;

    // e_machine is at offset 18 (2 bytes)
    let e_machine = if is_little_endian {
        u16::from_le_bytes([data[18], data[19]])
    } else {
        u16::from_be_bytes([data[18], data[19]])
    };

    for (machine, name) in ELF_MACHINE_MAP {
        if *machine == e_machine {
            return Some(name.to_string());
        }
    }

    Some(format!("unknown(0x{:X})", e_machine))
}

/// Parse Android NDK info from ELF .note.android.ident section
pub fn parse_elf_android_ndk_info(data: &[u8]) -> Option<AndroidNdkInfo> {
    if data.len() < 64 || &data[..4] != b"\x7fELF" {
        return None;
    }

    // Get ELF class and endianness
    let elf_class = data[4]; // 1 = 32-bit, 2 = 64-bit
    let is_little_endian = data[5] == 1;

    // Parse ELF header to find section headers
    let (e_shoff, e_shentsize, e_shnum, e_shstrndx) = if elf_class == 1 {
        // 32-bit ELF
        if data.len() < 52 {
            return None;
        }
        let e_shoff = read_u32(data, 32, is_little_endian) as usize;
        let e_shentsize = read_u16(data, 46, is_little_endian) as usize;
        let e_shnum = read_u16(data, 48, is_little_endian) as usize;
        let e_shstrndx = read_u16(data, 50, is_little_endian) as usize;
        (e_shoff, e_shentsize, e_shnum, e_shstrndx)
    } else {
        // 64-bit ELF
        if data.len() < 64 {
            return None;
        }
        let e_shoff = read_u64(data, 40, is_little_endian) as usize;
        let e_shentsize = read_u16(data, 58, is_little_endian) as usize;
        let e_shnum = read_u16(data, 60, is_little_endian) as usize;
        let e_shstrndx = read_u16(data, 62, is_little_endian) as usize;
        (e_shoff, e_shentsize, e_shnum, e_shstrndx)
    };

    if e_shoff == 0 || e_shnum == 0 {
        return None;
    }

    // Read section header string table
    let shstrtab_off = e_shoff + e_shstrndx * e_shentsize;
    if shstrtab_off + e_shentsize > data.len() {
        return None;
    }

    let (strtab_offset, strtab_size) = if elf_class == 1 {
        let offset = read_u32(data, shstrtab_off + 16, is_little_endian) as usize;
        let size = read_u32(data, shstrtab_off + 20, is_little_endian) as usize;
        (offset, size)
    } else {
        let offset = read_u64(data, shstrtab_off + 24, is_little_endian) as usize;
        let size = read_u64(data, shstrtab_off + 32, is_little_endian) as usize;
        (offset, size)
    };

    if strtab_offset + strtab_size > data.len() {
        return None;
    }

    let strtab = &data[strtab_offset..strtab_offset + strtab_size];

    // Find .note.android.ident section
    let target_section = b".note.android.ident";

    for i in 0..e_shnum {
        let sh_off = e_shoff + i * e_shentsize;
        if sh_off + e_shentsize > data.len() {
            continue;
        }

        let sh_name = read_u32(data, sh_off, is_little_endian) as usize;

        let (sec_offset, sec_size) = if elf_class == 1 {
            let offset = read_u32(data, sh_off + 16, is_little_endian) as usize;
            let size = read_u32(data, sh_off + 20, is_little_endian) as usize;
            (offset, size)
        } else {
            let offset = read_u64(data, sh_off + 24, is_little_endian) as usize;
            let size = read_u64(data, sh_off + 32, is_little_endian) as usize;
            (offset, size)
        };

        // Get section name from string table
        if sh_name >= strtab.len() {
            continue;
        }

        let name_end = strtab[sh_name..]
            .iter()
            .position(|&b| b == 0)
            .map(|p| sh_name + p)
            .unwrap_or(strtab.len());
        let section_name = &strtab[sh_name..name_end];

        if section_name == target_section {
            // Found .note.android.ident section
            if sec_offset + sec_size > data.len() {
                return None;
            }

            let note_data = &data[sec_offset..sec_offset + sec_size];
            return parse_android_note(note_data, is_little_endian);
        }
    }

    None
}

/// Parse Android note section content
///
/// Format:
///   uint32_t namesz (8 for "Android\0")
///   uint32_t descsz
///   uint32_t type (1)
///   char name[8] "Android\0"
///   uint16_t api_level
///   uint16_t padding (usually NDK major version in newer NDKs)
///   char ndk_version[] (null-terminated, e.g., "r27")
///   char ndk_build[] (null-terminated, e.g., "12117859")
fn parse_android_note(note_data: &[u8], is_little_endian: bool) -> Option<AndroidNdkInfo> {
    if note_data.len() < 16 {
        return None;
    }

    let namesz = read_u32(note_data, 0, is_little_endian) as usize;
    let descsz = read_u32(note_data, 4, is_little_endian) as usize;
    // let note_type = read_u32(note_data, 8, is_little_endian);

    // Name is aligned to 4 bytes
    let name_aligned = (namesz + 3) & !3;
    let desc_start = 12 + name_aligned;

    if desc_start + descsz > note_data.len() {
        return None;
    }

    // Check name is "Android"
    let name = &note_data[12..12 + namesz];
    let name_str = name.split(|&b| b == 0).next().unwrap_or(&[]);
    if name_str != b"Android" {
        return None;
    }

    let desc = &note_data[desc_start..desc_start + descsz];
    if desc.len() < 4 {
        return None;
    }

    // Parse descriptor
    let api_level = read_u16(desc, 0, is_little_endian);

    // Find NDK version string (starts after api_level + padding)
    let mut ndk_version = None;
    if desc.len() > 4 {
        let remaining = &desc[4..];
        // Parse null-terminated strings
        let mut strings = Vec::new();
        let mut current = Vec::new();

        for &byte in remaining {
            if byte == 0 {
                if !current.is_empty() {
                    if let Ok(s) = String::from_utf8(current.clone()) {
                        strings.push(s);
                    }
                    current.clear();
                }
            } else {
                current.push(byte);
            }
        }
        if !current.is_empty() {
            if let Ok(s) = String::from_utf8(current) {
                strings.push(s);
            }
        }

        // First string starting with 'r' is likely NDK version
        for s in strings {
            if s.starts_with('r') && s.len() <= 10 {
                ndk_version = Some(s);
                break;
            }
        }
    }

    Some(AndroidNdkInfo {
        api_level: Some(api_level),
        ndk_version,
    })
}

/// Get library information from binary data
///
/// Parses the binary to extract architecture info, and for Android .so files,
/// also extracts NDK version and API level.
pub fn get_library_info(data: &[u8], filename: &str, file_path: &str) -> LibraryInfo {
    let mut info = LibraryInfo {
        arch: None,
        ndk_info: None,
    };

    if data.len() < 4 {
        return info;
    }

    let magic = &data[..4];

    // ELF
    if magic == b"\x7fELF" {
        info.arch = parse_elf_arch(data);

        // Check for Android .so files
        // Match paths containing "android" or "jni/" (AAR format uses jni/ for native libs)
        let path_lower = file_path.to_lowercase();
        if filename.ends_with(".so")
            && (path_lower.contains("android") || path_lower.contains("jni/"))
        {
            info.ndk_info = parse_elf_android_ndk_info(data);
        }
    }
    // Mach-O (various magic values)
    else if matches!(
        magic,
        b"\xfe\xed\xfa\xce"  // 32-bit
            | b"\xfe\xed\xfa\xcf"  // 64-bit
            | b"\xce\xfa\xed\xfe"  // 32-bit reversed
            | b"\xcf\xfa\xed\xfe"  // 64-bit reversed
            | b"\xca\xfe\xba\xbe"  // FAT binary
            | b"\xbe\xba\xfe\xca"  // FAT binary reversed
    ) {
        info.arch = parse_macho_arch(data);
    }
    // PE (MZ header)
    else if &magic[..2] == b"MZ" {
        info.arch = parse_pe_arch(data);
    }
    // AR archive
    else if data.len() >= 8 && &data[..8] == b"!<arch>\n" {
        info.arch = parse_ar_member_arch(data);
    }

    info
}

/// Check if a file is a library file that should have info extracted
pub fn is_library_file(filename: &str, file_path: &str) -> bool {
    // Check if in lib, jni, or frameworks directory
    let path_lower = file_path.to_lowercase();
    if !path_lower.contains("lib/")
        && !path_lower.contains("jni/")
        && !path_lower.contains("frameworks/")
    {
        return false;
    }

    // Check extensions
    let lib_extensions = [".a", ".so", ".dylib", ".dll", ".lib"];
    if lib_extensions.iter().any(|ext| filename.ends_with(ext)) {
        return true;
    }

    // macOS framework binary (no extension, inside .framework)
    if file_path.contains(".framework/") && !filename.contains('.') {
        return true;
    }

    false
}

// Helper functions for reading integers from byte slices

fn read_u16(data: &[u8], offset: usize, little_endian: bool) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    if little_endian {
        u16::from_le_bytes([data[offset], data[offset + 1]])
    } else {
        u16::from_be_bytes([data[offset], data[offset + 1]])
    }
}

fn read_u32(data: &[u8], offset: usize, little_endian: bool) -> u32 {
    if offset + 4 > data.len() {
        return 0;
    }
    if little_endian {
        u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
    } else {
        u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
    }
}

fn read_u64(data: &[u8], offset: usize, little_endian: bool) -> u64 {
    if offset + 8 > data.len() {
        return 0;
    }
    if little_endian {
        u64::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])
    } else {
        u64::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])
    }
}

/// Parse Mach-O file to get architecture
fn parse_macho_arch(data: &[u8]) -> Option<String> {
    if data.len() < 8 {
        return None;
    }

    let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);

    // Check for fat binary (universal)
    if magic == 0xCAFEBABE || magic == 0xBEBAFECA {
        if data.len() < 8 {
            return None;
        }
        let nfat = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        if nfat > 10 {
            // Sanity check
            return None;
        }

        let mut archs = Vec::new();
        for i in 0..nfat as usize {
            let offset = 8 + i * 20;
            if offset + 8 > data.len() {
                break;
            }
            let cputype =
                u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]);
            for (cpu, name) in MACHO_CPU_MAP {
                if *cpu == cputype && !archs.contains(&name.to_string()) {
                    archs.push(name.to_string());
                }
            }
        }

        return if archs.is_empty() {
            None
        } else {
            Some(archs.join(", "))
        };
    }

    // Single architecture Mach-O
    let (is_le, _is_64) = match magic {
        0xFEEDFACE => (true, false),  // 32-bit LE
        0xFEEDFACF => (true, true),   // 64-bit LE
        0xCEFAEDFE => (false, false), // 32-bit BE
        0xCFFAEDFE => (false, true),  // 64-bit BE
        _ => return None,
    };

    if data.len() < 8 {
        return None;
    }

    let cputype = if is_le {
        u32::from_le_bytes([data[4], data[5], data[6], data[7]])
    } else {
        u32::from_be_bytes([data[4], data[5], data[6], data[7]])
    };

    for (cpu, name) in MACHO_CPU_MAP {
        if *cpu == cputype {
            return Some(name.to_string());
        }
    }

    Some(format!("unknown(0x{:X})", cputype))
}

/// Parse PE file to get architecture
fn parse_pe_arch(data: &[u8]) -> Option<String> {
    if data.len() < 64 || &data[..2] != b"MZ" {
        return None;
    }

    // Get PE header offset from DOS header at offset 0x3C
    if data.len() < 0x40 {
        return None;
    }
    let pe_offset = u32::from_le_bytes([data[0x3C], data[0x3D], data[0x3E], data[0x3F]]) as usize;

    if pe_offset + 6 > data.len() {
        return None;
    }

    // Check PE signature
    if &data[pe_offset..pe_offset + 4] != b"PE\x00\x00" {
        return None;
    }

    // Machine type is at PE header + 4
    let machine = u16::from_le_bytes([data[pe_offset + 4], data[pe_offset + 5]]);

    for (m, name) in PE_MACHINE_MAP {
        if *m == machine {
            return Some(name.to_string());
        }
    }

    Some(format!("unknown(0x{:X})", machine))
}

/// Parse AR archive to get member architecture
fn parse_ar_member_arch(data: &[u8]) -> Option<String> {
    if data.len() < 68 || &data[..8] != b"!<arch>\n" {
        return None;
    }

    // Skip to first file header (at offset 8)
    // AR format: "!<arch>\n" + 60-byte file header + file content
    let header_size = 60;
    let content_start = 8 + header_size;

    if content_start + 4 > data.len() {
        return None;
    }

    // Try to parse the first member as ELF
    let member_data = &data[content_start..];
    if member_data.len() >= 4 && &member_data[..4] == b"\x7fELF" {
        return parse_elf_arch(member_data);
    }

    // Try Mach-O
    if member_data.len() >= 4 {
        let magic = u32::from_le_bytes([
            member_data[0],
            member_data[1],
            member_data[2],
            member_data[3],
        ]);
        if matches!(
            magic,
            0xFEEDFACE | 0xFEEDFACF | 0xCEFAEDFE | 0xCFFAEDFE | 0xCAFEBABE | 0xBEBAFECA
        ) {
            return parse_macho_arch(member_data);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_library_file() {
        assert!(is_library_file("libfoo.so", "lib/android/shared/arm64-v8a/libfoo.so"));
        assert!(is_library_file("libfoo.so", "jni/arm64-v8a/libfoo.so"));
        assert!(is_library_file("libfoo.a", "lib/android/static/arm64-v8a/libfoo.a"));
        assert!(is_library_file("foo.dylib", "lib/macos/shared/foo.dylib"));
        assert!(!is_library_file("foo.txt", "lib/android/foo.txt"));
        assert!(!is_library_file("libfoo.so", "src/libfoo.so"));
    }

    #[test]
    fn test_library_info_display() {
        let info = LibraryInfo {
            arch: Some("aarch64".to_string()),
            ndk_info: Some(AndroidNdkInfo {
                ndk_version: Some("r27".to_string()),
                api_level: Some(24),
            }),
        };
        assert_eq!(info.to_display_string(), " [aarch64, NDK r27, API 24]");

        let info2 = LibraryInfo {
            arch: Some("x86_64".to_string()),
            ndk_info: None,
        };
        assert_eq!(info2.to_display_string(), " [x86_64]");

        let info3 = LibraryInfo {
            arch: None,
            ndk_info: None,
        };
        assert_eq!(info3.to_display_string(), "");
    }
}
