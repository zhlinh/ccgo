# Windows Toolchain Selection Guide

CCGO now supports dual toolchain for Windows builds, similar to how Rust's cargo handles Windows targets.

## Supported Toolchains

### 1. MinGW-w64 (GNU ABI)
- **Default in Docker builds**
- Cross-platform friendly
- No Microsoft licenses required
- Produces `.a` static libraries (renamed to `.lib` in packages)
- Compatible with MinGW/MSYS ecosystem

### 2. MSVC (Microsoft ABI)
- **Better Windows ecosystem compatibility**
- Links with Visual Studio libraries
- Produces `.lib` static libraries and `.pdb` debug symbols
- Required for C++/COM interop with Windows APIs
- Available via:
  - Native Visual Studio 2019+ on Windows
  - Clang-cl + Windows SDK in Docker (experimental)

## Usage

### Command Line

```bash
# Auto-detect toolchain (default)
ccgo build windows

# Explicitly use MinGW
ccgo build windows --toolchain=mingw
ccgo build windows --toolchain=gnu

# Explicitly use MSVC
ccgo build windows --toolchain=msvc

# Docker builds with toolchain selection
ccgo build windows --docker --toolchain=mingw
ccgo build windows --docker --toolchain=msvc
```

### Docker Support

Two Docker images are available:

1. **ccgo-builder-windows** (default, MinGW)
   - Dockerfile: `Dockerfile.windows-mingw`
   - Uses MinGW-w64 cross-compiler
   - Size: ~1.2GB
   - Stable and well-tested

2. **ccgo-builder-windows-msvc** (experimental)
   - Dockerfile: `Dockerfile.windows-msvc`
   - Uses clang-cl with Windows SDK
   - Size: ~1.5GB
   - MSVC ABI compatible
   - Downloads Windows SDK from Microsoft

### Toolchain Selection Logic

```
┌─────────────────────────┐
│  User specifies         │
│  --toolchain=xxx?       │
└──────────┬──────────────┘
           │
      ┌────▼────┐
      │  auto   │────► Detect available toolchains
      └─────────┘      │
                       ▼
      ┌────────────────────────────────┐
      │ Docker/Linux: Prefer MinGW     │
      │ Windows: Prefer Visual Studio  │
      └────────────────────────────────┘
```

### Environment Variable

You can also set the toolchain via environment variable:

```bash
# Force MSVC toolchain
export CCGO_WINDOWS_TOOLCHAIN=msvc
ccgo build windows

# Force MinGW toolchain
export CCGO_WINDOWS_TOOLCHAIN=mingw
ccgo build windows
```

## Comparison with Rust Cargo

| Feature | CCGO | Rust Cargo |
|---------|------|------------|
| Target naming | `--toolchain=msvc/gnu` | `x86_64-pc-windows-msvc/gnu` |
| Default on Windows | Visual Studio (if available) | MSVC |
| Default in Docker | MinGW | N/A (requires explicit target) |
| Multiple toolchains | Supported | Supported via rustup |
| Cross-compilation | Built-in Docker support | Requires manual setup |

## Library Compatibility

### MinGW Libraries (.a)
- Compatible with: MinGW, MSYS2, Cygwin
- Not compatible with: Visual Studio projects
- Use when: Building for MinGW ecosystem or cross-platform projects

### MSVC Libraries (.lib)
- Compatible with: Visual Studio, Windows SDK
- Not compatible with: MinGW linker
- Use when: Integrating with Windows native APIs or Visual Studio projects

## Recommendations

1. **For cross-platform projects**: Use MinGW (default)
2. **For Windows-specific projects**: Use MSVC
3. **For maximum compatibility**: Build with both toolchains
4. **For CI/CD**: Use Docker with explicit `--toolchain` for reproducibility

## Troubleshooting

### "MSVC toolchain requested but not available"
- Install Visual Studio 2019+ with C++ workload
- Or use `--docker --toolchain=msvc` for Docker-based MSVC build

### "MinGW toolchain requested but not available"
- Install MinGW-w64: `apt-get install mingw-w64` (Linux)
- Or use `--docker --toolchain=mingw` for Docker-based MinGW build

### Linking errors between libraries
- Ensure all libraries use the same toolchain (all MinGW or all MSVC)
- MinGW and MSVC libraries are NOT binary compatible

## Future Improvements

- [ ] Support for ARM64 Windows targets
- [ ] Automatic toolchain detection from existing project
- [ ] Side-by-side builds with both toolchains
- [ ] Better MSVC Docker image with full Visual Studio Build Tools