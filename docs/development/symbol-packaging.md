# Native SDK symbol packaging

Native platform builds follow these rules:

- Release builds use `RelWithDebInfo`; Debug builds keep debug information.
- Static libraries retain debug information in the SDK and are never stripped.
- Shared library symbols are saved before Release SDK libraries are stripped.
- Debug SDK libraries are not stripped.
- MSVC uses PDB files rather than DWARF and GNU/Apple strip tools.

| Platform | Static SDK | Release shared SDK | SYMBOLS contents |
| --- | --- | --- | --- |
| iOS, macOS, tvOS, watchOS | Unstripped `.a` | `strip -S` | Unstripped static archives and shared dSYM bundles |
| Android | Unstripped `.a` | NDK strip | Unstripped shared `.so` |
| OHOS | Unstripped `.a` | SDK `llvm-strip --strip-unneeded` | Unstripped shared `.so` |
| Linux, OpenWrt | Unstripped `.a` | Target `strip --strip-unneeded` on SDK staging copies | Unstripped shared `.so`, including versioned names |
| Windows MinGW | Unstripped `.a` | Target `strip --strip-unneeded` | Unstripped `.dll` |
| Windows MSVC | `.lib`, with PDB collected separately | DLL with separate PDB | Compiler/linker PDB files, including static builds |

ELF symbols use `symbols/<platform>/obj/<architecture>/`. Windows adds toolchain
and linkage directories to avoid name collisions. Apple symbols use
`symbols/<platform>/<static|shared>/<device|simulator>/`; macOS uses `universal/`.
Release Apple dSYM bundles are distributed only in SYMBOLS, while static DWARF
remains available in the SDK for the consuming application's dSYM generation.

The same strip policy does not require identical debug formats or a separate
SYMBOLS archive for every static-only build: ELF/MinGW static debug information
already travels in the SDK archive. Apple additionally archives the static
library; MSVC static builds need their PDB files.

KMP and Conan are packaging/build backends rather than fixed native targets.
They currently return no separate SYMBOLS archive; Gradle tasks and Conan
recipes control their native artifacts. They are not covered by the native
builder strip policy here.

Apple, Linux and OpenWrt preserve original build outputs when stripping SDK
copies. Android, OHOS and MinGW currently strip shared build outputs in place;
their symbol collection precedes that step. This implementation difference
remains relevant when investigating incremental builds.
