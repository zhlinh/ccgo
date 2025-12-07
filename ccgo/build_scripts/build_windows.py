#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_windows.py
# ccgo
#
# Copyright 2024 zhlinh and ccgo Project Authors. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Softwaree

"""
Windows native library build script.

This script builds static libraries (.lib) for Windows platform using CMake
and Visual Studio toolchain. It handles:
- Building with Visual Studio 2019 (v142 toolset)
- Merging multiple static libraries into single .lib
- Collecting and packaging PDB debug symbols
- Header file organization
- Visual Studio project generation
- Support for both Release and Debug configurations

Requirements:
- Visual Studio 2019 or later
- CMake 3.10 or later
- Python 3.7+
- Windows development environment

Usage:
    python3 build_windows.py [mode]

    mode: 1 (build Release), 2 (generate VS project), 3 (build Debug), 4 (exit)

Output:
    - Static library: cmake_build/Windows/Windows.out/{project}.dir/x64/{project}.lib
    - Debug symbols: cmake_build/Windows/Windows.out/{project}.dir/x64/{project}.pdb.zip
    - Headers: cmake_build/Windows/Windows.out/{project}.dir/x64/include/
"""

import os
import sys
import glob
import time
import shutil
import platform
import multiprocessing

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

# Script configuration
SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils
# which loads them from CCGO.toml configuration file

# Build output paths (base path, actual paths include link_type/toolchain subdirectories)
BUILD_OUT_PATH_BASE = "cmake_build/Windows"


def get_build_out_path(link_type, toolchain="msvc"):
    """Get build output path for specified link type and toolchain.

    Directory structure:
    - cmake_build/Windows/static/mingw/
    - cmake_build/Windows/static/msvc/
    - cmake_build/Windows/shared/mingw/
    - cmake_build/Windows/shared/msvc/
    """
    return f"{BUILD_OUT_PATH_BASE}/{link_type}/{toolchain}"


def get_install_path(link_type, toolchain="msvc"):
    """Get install path for specified link type and toolchain."""
    return f"{BUILD_OUT_PATH_BASE}/{link_type}/{toolchain}/Windows.out/"


# Visual Studio 2019 build configuration
# Uses Visual Studio 16 2019 generator with v142 platform toolset (C++17 support)
# Parameters: ccgo_cmake_dir, config, jobs
# Note: Uses ../../../.. because build directory is now cmake_build/Windows/{link_type}/{toolchain}/
WIN_BUILD_CMD = 'cmake ../../../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="%s" && cmake --build . --target install --config %s --parallel %d'
WIN_GEN_PROJECT_CMD = (
    'cmake ../../../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="%s"'
)
WIN_ARCH = "x64"  # Target architecture (64-bit Windows)
WIN_SRC_DIR = "src"  # Source directory name for PDB collection
THIRD_PARTY_MERGE_LIBS = ["pthread"]  # Third-party libraries to merge into final .lib


def _build_windows_single(incremental, config, single_link_type, use_mingw, use_msvc_compat, jobs):
    """
    Build Windows library for a single link type (static or shared).

    This internal function handles the actual build for one link type.

    Args:
        incremental: If True, skip clean step for faster rebuilds
        config: Build configuration - 'Release' or 'Debug'
        single_link_type: Either 'static' or 'shared'
        use_mingw: If True, use MinGW-w64 toolchain
        use_msvc_compat: If True, use clang-cl for MSVC-compatible builds
        jobs: Number of parallel build jobs

    Returns:
        bool: True if build succeeded, False otherwise
    """
    # Determine toolchain name for path
    if use_mingw:
        toolchain = "mingw"
    else:
        toolchain = "msvc"

    build_out_path = get_build_out_path(single_link_type, toolchain)
    install_path = get_install_path(single_link_type, toolchain)

    print(f"\n==================build_windows ({single_link_type}, {toolchain}, jobs: {jobs})========================")

    # Set link type CMake flags
    if single_link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    else:  # shared
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"

    clean(build_out_path, incremental)
    os.chdir(build_out_path)

    if use_mingw:
        # MinGW cross-compilation (for Docker/Linux environments)
        # Use Unix Makefiles generator with MinGW compilers
        cmake_config_cmd = (
            f'cmake ../../../.. '
            f'-G "Unix Makefiles" '
            f'-DCMAKE_SYSTEM_NAME=Windows '
            f'-DCMAKE_C_COMPILER=x86_64-w64-mingw32-gcc '
            f'-DCMAKE_CXX_COMPILER=x86_64-w64-mingw32-g++ '
            f'-DCMAKE_RC_COMPILER=x86_64-w64-mingw32-windres '
            f'-DCMAKE_FIND_ROOT_PATH=/usr/x86_64-w64-mingw32 '
            f'-DCMAKE_FIND_ROOT_PATH_MODE_PROGRAM=NEVER '
            f'-DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY '
            f'-DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY '
            f'-DCMAKE_BUILD_TYPE={config} '
            f'-DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags}'
        )
        cmake_build_cmd = f'cmake --build . --target install -- -j{jobs}'
        cmd = f'{cmake_config_cmd} && {cmake_build_cmd}'
    elif use_msvc_compat:
        # MSVC-compatible build using clang (for Docker with MSVC ABI)
        # Use toolchain file for proper cross-compilation setup
        # The toolchain file handles:
        # - Setting up clang with MSVC target triple
        # - Configuring include and library paths from xwin SDK
        # - Forcing static CRT (libcmt.lib) since xwin doesn't have debug CRT
        toolchain_file = os.path.join(CCGO_CMAKE_DIR, "windows-msvc.toolchain.cmake")

        cmake_config_cmd = (
            f'cmake ../../../.. '
            f'-G "Ninja" '
            f'-DCMAKE_TOOLCHAIN_FILE="{toolchain_file}" '
            f'-DCMAKE_BUILD_TYPE=Release '
            f'-DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags}'
        )
        cmake_build_cmd = f'cmake --build . --target install -- -j{jobs}'
        cmd = f'{cmake_config_cmd} && {cmake_build_cmd}'
    else:
        # Visual Studio build (native Windows)
        cmd = f'cmake ../../../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags} && cmake --build . --target install --config {config} --parallel {jobs}'

    print("build cmd:" + cmd)
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!build fail!!!!!!!!!!!!!!!!!!!!")
        print("ERROR: Native build failed for Windows. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure

    if single_link_type == 'static':
        # Create result directory without architecture subdirectory (x64)
        # Windows only supports x64, so no need for architecture-specific subdirectories
        win_result_dir = install_path + f"{PROJECT_NAME_LOWER}.dir"
        if os.path.exists(win_result_dir):
            shutil.rmtree(win_result_dir)
        os.makedirs(win_result_dir)

        needed_libs = glob.glob(install_path + "*.lib")

        for other_lib in THIRD_PARTY_MERGE_LIBS:
            temp_libs_path = (
                SCRIPT_PATH + f"/third_party/{other_lib}/lib/windows/{WIN_ARCH}/"
            )
            temp_libs = glob.glob(temp_libs_path + "*.lib")
            needed_libs.extend(temp_libs)

        filtered_lib_names = list(
            map(lambda x: os.path.splitext(os.path.basename(x))[0], needed_libs)
        )

        print(f"build merge libs: {needed_libs}")

        if use_mingw:
            # MinGW builds: Merge multiple .a files into a single library
            # MinGW produces .a files, not .lib files
            # Merge all .a files from install_path
            mingw_libs = glob.glob(install_path + "*.a")

            # Add third-party libraries to merge list
            for other_lib in THIRD_PARTY_MERGE_LIBS:
                temp_libs_path = (
                    SCRIPT_PATH + f"/third_party/{other_lib}/lib/windows/{WIN_ARCH}/"
                )
                temp_libs = glob.glob(temp_libs_path + "*.a")
                mingw_libs.extend(temp_libs)

            print(f"MinGW: Merging libraries: {mingw_libs}")

            # Merge all .a files into a single library using MinGW-specific merge
            output_lib = win_result_dir + f"/{PROJECT_NAME_LOWER}.a"
            if not merge_mingw_static_libs(mingw_libs, output_lib):
                print("!!!!!!!!!!!!!!!!!!merge MinGW libs fail!!!!!!!!!!!!!!!!!!!!")
                return False

            print(f"MinGW: Merged library created: {output_lib}")
        elif use_msvc_compat:
            # MSVC-compatible builds (clang-cl in Docker): Use llvm-lib to merge
            print(f"MSVC-compat: Merging libraries using llvm-lib: {needed_libs}")
            output_lib = win_result_dir + f"/{PROJECT_NAME_LOWER}.lib"
            if not merge_llvm_static_libs(needed_libs, output_lib):
                print("!!!!!!!!!!!!!!!!!!merge LLVM libs fail!!!!!!!!!!!!!!!!!!!!")
                print("ERROR: Failed to merge Windows libraries. Stopping immediately.")
                sys.exit(1)  # Exit immediately on merge failure
            print(f"MSVC-compat: Merged library created: {output_lib}")
        else:
            # Native Visual Studio builds: Merge multiple .lib files using lib.exe
            if not merge_win_static_libs(
                needed_libs, win_result_dir + f"/{PROJECT_NAME_LOWER}.lib"
            ):
                print("!!!!!!!!!!!!!!!!!!merge libs fail!!!!!!!!!!!!!!!!!!!!")
                print("ERROR: Failed to merge Windows libraries. Stopping immediately.")
                sys.exit(1)  # Exit immediately on merge failure

        headers = dict()
        headers.update(WINDOWS_BUILD_COPY_HEADER_FILES)
        copy_file_mapping(headers, "./", win_result_dir + "/include")

        if use_mingw:
            # MinGW doesn't generate PDB files (uses DWARF debug info instead)
            print("MinGW build: Skipping PDB collection (not applicable)")
        elif use_msvc_compat:
            # MSVC-compat builds in Docker don't generate PDB files
            print("MSVC-compat build: Skipping PDB collection (not generated in cross-compilation)")
        else:
            # Visual Studio builds: Copy PDB debug symbol files
            sub_folders = filtered_lib_names
            # copy pdb of third_party
            copy_windows_pdb(build_out_path, sub_folders, config, install_path)
            src_dir_folder = PROJECT_NAME_LOWER + "-" + WIN_SRC_DIR
            # copy pdb of src
            sub_folders = list(
                map(lambda x: x.replace(PROJECT_NAME_LOWER, src_dir_folder), sub_folders)
            )
            copy_windows_pdb(
                os.path.join(build_out_path, src_dir_folder), sub_folders, config, install_path
            )

        # zip pdb files (Visual Studio only)
        if not use_mingw:
            pdf_suffix = ".pdb"
            zip_files_ends_with(
                install_path,
                pdf_suffix,
                win_result_dir + f"/{PROJECT_NAME_LOWER}{pdf_suffix}.zip",
            )

        # Check the built library architecture
        print("\n==================Verifying Windows Library========================")
        if use_mingw:
            # MinGW builds .a files
            final_lib = os.path.join(win_result_dir, f"{PROJECT_NAME_LOWER}.a")
        else:
            # MSVC builds .lib files
            final_lib = os.path.join(win_result_dir, f"{PROJECT_NAME_LOWER}.lib")

        if os.path.exists(final_lib):
            check_library_architecture(final_lib, platform_hint="windows")
        else:
            # Try to find any .lib or .a file
            lib_files = glob.glob(os.path.join(win_result_dir, "*.lib")) + glob.glob(os.path.join(win_result_dir, "*.a"))
            if lib_files:
                check_library_architecture(lib_files[0], platform_hint="windows")
        print("====================================================================")

        print("==================Output========================")
        print(f"libs: {win_result_dir}")
        if not use_mingw:
            print(f"pdb files: {install_path}")
    else:  # shared
        # Check for shared library
        shared_dir = os.path.join(install_path, "shared")
        if use_mingw:
            shared_lib_path = os.path.join(shared_dir, f"lib{PROJECT_NAME_LOWER}.dll")
            if not os.path.exists(shared_lib_path):
                shared_lib_path = os.path.join(install_path, f"lib{PROJECT_NAME_LOWER}.dll")
        else:
            shared_lib_path = os.path.join(shared_dir, f"{PROJECT_NAME_LOWER}.dll")
            if not os.path.exists(shared_lib_path):
                shared_lib_path = os.path.join(install_path, f"{PROJECT_NAME_LOWER}.dll")

        if os.path.exists(shared_lib_path):
            print("\n==================Verifying Windows Shared Library========================")
            check_library_architecture(shared_lib_path, platform_hint="windows")
            print("==================Output========================")
            print(shared_lib_path)
        else:
            print(f"Warning: Shared library not found at expected location")

    return True


def build_windows(incremental, config="Release", link_type='both', use_mingw=False, use_msvc_compat=False, jobs=None):
    """
    Build Windows static library with Visual Studio or MinGW toolchain.

    This function performs the complete Windows build process:
    1. Generates version info header file
    2. Cleans build directory (unless incremental build)
    3. Configures and builds with Visual Studio 2019 (v142 toolset) OR MinGW-w64
    4. Merges multiple static libraries into single .lib file
    5. Copies header files to include directory
    6. Collects PDB debug symbol files (Visual Studio only)
    7. Packages PDB files into zip archive

    Args:
        incremental: If True, skip clean step for faster rebuilds

        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        use_mingw: If True, use MinGW-w64 toolchain for cross-compilation
        use_msvc_compat: If True, use clang-cl for MSVC-compatible builds
        jobs: Number of parallel build jobs (default: CPU count)

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Static library: Windows.out/{project}.dir/x64/{project}.lib
        - Debug symbols: Windows.out/{project}.dir/x64/{project}.pdb.zip
        - Headers: Windows.out/{project}.dir/x64/include/

    Note:
        PDB files are essential for debugging crashes in production.
        Always preserve the .pdb.zip file for release builds.
        The lib file contains merged static libraries from both
        project sources and third-party dependencies.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    before_time = time.time()
    print(f"==================build_windows (link_type: {link_type}, jobs: {jobs})========================")

    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        incremental=incremental,
        platform="windows",
    )

    # Build for each link type separately to avoid overwriting
    if link_type == 'both':
        _build_windows_single(incremental, config, 'static', use_mingw, use_msvc_compat, jobs)
        _build_windows_single(incremental, config, 'shared', use_mingw, use_msvc_compat, jobs)
    else:
        _build_windows_single(incremental, config, link_type, use_mingw, use_msvc_compat, jobs)

    after_time = time.time()
    print(f"use time: {int(after_time - before_time)} s")
    return True


def gen_win_project( config="Release"):
    """
    Generate Visual Studio project for Windows development and debugging.

    This function creates a Visual Studio solution (.sln) and project files
    that can be opened in Visual Studio for interactive development, debugging,
    and testing. The project is automatically opened in Visual Studio after generation.

    Args:

        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
              Note: This parameter is currently unused but reserved for future use

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - VS solution: cmake_build/Windows/static/msvc/{project}.sln (auto-opened)

    Note:
        The generated Visual Studio project uses the v142 platform toolset.
        This is useful for development workflows where you need Visual Studio's
        debugging tools, IntelliSense, and build system integration.
        The project file is automatically opened in Visual Studio after generation.
    """
    before_time = time.time()
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        platform="windows",
    )
    # Use static/msvc directory for IDE project
    build_out_path = get_build_out_path("static", "msvc")
    clean(build_out_path, False)
    os.chdir(build_out_path)
    ret = os.system(WIN_GEN_PROJECT_CMD % CCGO_CMAKE_DIR)
    os.chdir(SCRIPT_PATH)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!gen project file fail!!!!!!!!!!!!!!!!!!!!")
        return False

    project_file_prefix = os.path.join(SCRIPT_PATH, build_out_path, PROJECT_NAME_LOWER)
    project_file = get_project_file_name(project_file_prefix)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {project_file}")

    os.system(get_open_project_file_cmd(project_file))

    return True


def archive_windows_project(link_type='both', toolchain='auto'):
    """
    Archive Windows library and related build artifacts with unified structure.

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_WINDOWS_SDK-{version}.zip
       - lib/static/mingw/lib{project}.a    (MinGW static)
       - lib/static/msvc/{project}.lib      (MSVC static)
       - lib/shared/mingw/lib{project}.dll  (MinGW shared)
       - lib/shared/msvc/{project}.dll      (MSVC shared)
       - include/{project}/
       - build_info.json
    2. Symbols package: {PROJECT_NAME}_WINDOWS_SDK-{version}-SYMBOLS.zip
       - symbols/static/msvc/{project}.pdb
       - symbols/shared/msvc/{project}.pdb

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        toolchain: Toolchain used ('mingw', 'msvc', or 'auto', default: 'auto')
                   When 'auto', includes all available toolchain outputs (both mingw and msvc if present)

    Output:
        - target/windows/{PROJECT_NAME}_WINDOWS_SDK-{version}.zip
        - target/windows/{PROJECT_NAME}_WINDOWS_SDK-{version}-SYMBOLS.zip
    """
    print("==================Archive Windows Project========================")

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(SCRIPT_PATH)

    # Define paths
    target_dir = os.path.join(SCRIPT_PATH, "target")
    bin_dir = os.path.join(target_dir, "windows")

    # Clean and recreate target/windows directory
    if os.path.exists(bin_dir):
        shutil.rmtree(bin_dir)
        print(f"Cleaned up old target/windows/ directory")
    os.makedirs(bin_dir, exist_ok=True)

    # Check which toolchain directories exist
    mingw_static_path = os.path.join(SCRIPT_PATH, get_install_path("static", "mingw"))
    msvc_static_path = os.path.join(SCRIPT_PATH, get_install_path("static", "msvc"))
    mingw_shared_path = os.path.join(SCRIPT_PATH, get_install_path("shared", "mingw"))
    msvc_shared_path = os.path.join(SCRIPT_PATH, get_install_path("shared", "msvc"))

    has_mingw = os.path.exists(mingw_static_path) or os.path.exists(mingw_shared_path)
    has_msvc = os.path.exists(msvc_static_path) or os.path.exists(msvc_shared_path)

    # Determine which toolchains to include
    if toolchain == 'auto':
        # Include all available toolchains
        include_mingw = has_mingw
        include_msvc = has_msvc
        if include_mingw and include_msvc:
            print("Auto-detected both MinGW and MSVC builds - including both in archive")
        elif include_mingw:
            print("Auto-detected MinGW build only")
        elif include_msvc:
            print("Auto-detected MSVC build only")
        else:
            print("WARNING: No build outputs found for any toolchain")
    elif toolchain == 'mingw':
        include_mingw = True
        include_msvc = False
    elif toolchain == 'msvc':
        include_mingw = False
        include_msvc = True
    else:
        # Default fallback
        include_mingw = has_mingw
        include_msvc = has_msvc

    # Prepare static libraries mapping - collect from all included toolchains
    static_libs = {}
    if link_type in ('static', 'both'):
        lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"

        # MinGW static library
        if include_mingw:
            mingw_lib_dir = os.path.join(mingw_static_path, lib_dir_name)
            if os.path.exists(mingw_lib_dir):
                static_lib_path = os.path.join(mingw_lib_dir, f"{PROJECT_NAME_LOWER}.a")
                if os.path.exists(static_lib_path):
                    arc_path = get_unified_lib_path("static", toolchain="mingw", lib_name=f"lib{PROJECT_NAME_LOWER}.a", platform="windows")
                    static_libs[arc_path] = static_lib_path
                    print(f"  + MinGW static: {static_lib_path}")

        # MSVC static library
        if include_msvc:
            msvc_lib_dir = os.path.join(msvc_static_path, lib_dir_name)
            if os.path.exists(msvc_lib_dir):
                static_lib_path = os.path.join(msvc_lib_dir, f"{PROJECT_NAME_LOWER}.lib")
                if os.path.exists(static_lib_path):
                    arc_path = get_unified_lib_path("static", toolchain="msvc", lib_name=f"{PROJECT_NAME_LOWER}.lib", platform="windows")
                    static_libs[arc_path] = static_lib_path
                    print(f"  + MSVC static: {static_lib_path}")

    # Prepare shared libraries mapping - collect from all included toolchains
    shared_libs = {}
    if link_type in ('shared', 'both'):
        # MinGW shared library
        if include_mingw:
            mingw_shared_build_path = os.path.dirname(mingw_shared_path.rstrip('/'))
            dll_search_paths = [
                os.path.join(mingw_shared_path, "shared", f"lib{PROJECT_NAME_LOWER}.dll"),
                os.path.join(mingw_shared_path, f"lib{PROJECT_NAME_LOWER}.dll"),
                os.path.join(mingw_shared_build_path, "bin", f"lib{PROJECT_NAME_LOWER}.dll"),
                os.path.join(mingw_shared_build_path, f"lib{PROJECT_NAME_LOWER}.dll"),
            ]
            dll_path = None
            for path in dll_search_paths:
                if os.path.exists(path):
                    dll_path = path
                    break

            if dll_path:
                arc_path = get_unified_lib_path("shared", toolchain="mingw", lib_name=f"lib{PROJECT_NAME_LOWER}.dll", platform="windows")
                shared_libs[arc_path] = dll_path
                print(f"  + MinGW shared DLL: {dll_path}")

            # MinGW import library
            import_lib_search_paths = [
                os.path.join(mingw_shared_path, "shared", f"lib{PROJECT_NAME_LOWER}.dll.a"),
                os.path.join(mingw_shared_path, f"lib{PROJECT_NAME_LOWER}.dll.a"),
                os.path.join(mingw_shared_build_path, "lib", f"lib{PROJECT_NAME_LOWER}.dll.a"),
                os.path.join(mingw_shared_build_path, f"lib{PROJECT_NAME_LOWER}.dll.a"),
            ]
            import_lib_path = None
            for path in import_lib_search_paths:
                if os.path.exists(path):
                    import_lib_path = path
                    break

            if import_lib_path:
                arc_path = get_unified_lib_path("shared", toolchain="mingw", lib_name=f"lib{PROJECT_NAME_LOWER}.dll.a", platform="windows")
                shared_libs[arc_path] = import_lib_path
                print(f"  + MinGW import lib: {import_lib_path}")

        # MSVC shared library
        if include_msvc:
            dll_path = os.path.join(msvc_shared_path, "shared", f"{PROJECT_NAME_LOWER}.dll")
            if not os.path.exists(dll_path):
                dll_path = os.path.join(msvc_shared_path, f"{PROJECT_NAME_LOWER}.dll")
            if os.path.exists(dll_path):
                arc_path = get_unified_lib_path("shared", toolchain="msvc", lib_name=f"{PROJECT_NAME_LOWER}.dll", platform="windows")
                shared_libs[arc_path] = dll_path
                print(f"  + MSVC shared DLL: {dll_path}")

            # MSVC import library
            import_lib_path = os.path.join(msvc_shared_path, "shared", f"{PROJECT_NAME_LOWER}.lib")
            if not os.path.exists(import_lib_path):
                import_lib_path = os.path.join(msvc_shared_path, f"{PROJECT_NAME_LOWER}_shared.lib")
            if os.path.exists(import_lib_path):
                arc_path = get_unified_lib_path("shared", toolchain="msvc", lib_name=f"{PROJECT_NAME_LOWER}.lib", platform="windows")
                shared_libs[arc_path] = import_lib_path
                print(f"  + MSVC import lib: {import_lib_path}")

    # Prepare include directories mapping (use project's include/ directory)
    include_dirs = {}
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(PROJECT_NAME_LOWER, headers_src)
        include_dirs[arc_path] = headers_src

    # Prepare symbols (PDB files for MSVC only)
    symbols_static = {}
    symbols_shared = {}
    if include_msvc:  # Only MSVC generates PDB files
        # Static library PDB
        msvc_lib_dir = os.path.join(msvc_static_path, f"{PROJECT_NAME_LOWER}.dir")
        static_pdb = os.path.join(msvc_lib_dir, f"{PROJECT_NAME_LOWER}.pdb")
        if os.path.exists(static_pdb) and link_type in ('static', 'both'):
            arc_path = get_unified_symbol_path("static", f"{PROJECT_NAME_LOWER}.pdb", platform="windows")
            symbols_static[arc_path] = static_pdb
            print(f"  + MSVC static PDB: {static_pdb}")

        # Shared library PDB
        shared_pdb = os.path.join(msvc_shared_path, "shared", f"{PROJECT_NAME_LOWER}.pdb")
        if not os.path.exists(shared_pdb):
            shared_pdb = os.path.join(msvc_shared_path, f"{PROJECT_NAME_LOWER}.pdb")
        if os.path.exists(shared_pdb) and link_type in ('shared', 'both'):
            arc_path = get_unified_symbol_path("shared", f"{PROJECT_NAME_LOWER}.pdb", platform="windows")
            symbols_shared[arc_path] = shared_pdb
            print(f"  + MSVC shared PDB: {shared_pdb}")

    # Determine toolchain for archive naming
    # When both toolchains are included, use 'auto' to indicate mixed
    if include_mingw and include_msvc:
        archive_toolchain = 'auto'
    elif include_msvc:
        archive_toolchain = 'msvc'
    else:
        archive_toolchain = 'mingw'

    # Create unified archive packages
    main_zip_path, symbols_zip_path = create_unified_archive(
        output_dir=bin_dir,
        project_name=PROJECT_NAME,
        platform_name="WINDOWS",
        version=full_version,
        link_type=link_type,
        static_libs=static_libs,
        shared_libs=shared_libs,
        include_dirs=include_dirs,
        symbols_static=symbols_static if symbols_static else None,
        symbols_shared=symbols_shared if symbols_shared else None,
        toolchain=archive_toolchain,
    )

    print("==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    if symbols_zip_path:
        print(f"Symbols package: {symbols_zip_path}")


def print_build_results(link_type='both'):
    """
    Print Windows build results from target/windows directory.

    This function displays the build artifacts:
    1. Main ZIP archive ({PROJECT_NAME}_WINDOWS_SDK-{version}.zip)
    2. Symbols package ({PROJECT_NAME}_WINDOWS_SDK-{version}-SYMBOLS.zip)
    3. build_info.json

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
    """
    print("==================Windows Build Results========================")

    # Define paths - artifacts are already in target/windows/
    bin_windows_dir = os.path.join(SCRIPT_PATH, "target", "windows")

    # Check if target/windows directory exists
    if not os.path.exists(bin_windows_dir):
        print(f"ERROR: target/windows directory not found. Please run build first.")
        sys.exit(1)

    # Main package: {PROJECT_NAME}_WINDOWS_SDK-*.zip (not ending with -SYMBOLS.zip)
    main_zips = [
        f for f in glob.glob(f"{bin_windows_dir}/*_WINDOWS_SDK-*.zip")
        if not f.endswith("-SYMBOLS.zip")
    ]

    # Symbols package: {PROJECT_NAME}_WINDOWS_SDK-*-SYMBOLS.zip
    symbols_zips = [
        f for f in glob.glob(f"{bin_windows_dir}/*_WINDOWS_SDK-*-SYMBOLS.zip")
    ]

    if not main_zips:
        print(f"ERROR: No build artifacts found in {bin_windows_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # List artifacts (no need to move - already in correct location)
    artifacts_found = main_zips + symbols_zips
    if artifacts_found:
        print(f"[SUCCESS] Found {len(artifacts_found)} artifact(s) in target/windows/")

    # Copy build_info.json from cmake_build to target/windows
    copy_build_info_to_target("windows", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/windows/:")
    print("-" * 60)

    # List all files in target/windows directory with sizes
    for item in sorted(os.listdir(bin_windows_dir)):
        item_path = os.path.join(bin_windows_dir, item)
        if os.path.isfile(item_path):
            size = os.path.getsize(item_path) / (1024 * 1024)  # MB
            print(f"  {item} ({size:.2f} MB)")

            # Print ZIP file tree structure
            if item.endswith(".zip"):
                print_zip_tree(item_path)
        elif os.path.isdir(item_path):
            # Calculate directory size
            total_size = 0
            for dirpath, dirnames, filenames in os.walk(item_path):
                for filename in filenames:
                    filepath = os.path.join(dirpath, filename)
                    total_size += os.path.getsize(filepath)
            size = total_size / (1024 * 1024)  # MB
            print(f"  {item}/ ({size:.2f} MB)")

    print("-" * 60)
    print("==================Build Complete========================")


def main(config="Release", link_type='both', jobs=None, no_archive=False, archive_toolchain=None):
    """
    Main entry point for Windows build.

    Args:
        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)
        no_archive: If True, skip the archive step (used for multi-toolchain builds)
        archive_toolchain: Override toolchain for archive ('auto', 'mingw', 'msvc')

    Note:
        Requires Visual Studio 2019 or later to be installed, OR MinGW-w64
        for cross-compilation from Linux/macOS (Docker containers).
    """
    # Check toolchain availability
    mingw_available = shutil.which("x86_64-w64-mingw32-gcc") is not None
    clang_cl_available = shutil.which("clang-cl") is not None or shutil.which("cl") is not None
    vs_available = check_vs_env()

    # Determine which toolchain to use based on environment and explicit selection
    use_mingw = False
    use_msvc_compat = False

    # Check for explicit toolchain selection via environment variable
    toolchain_env = os.environ.get("CCGO_WINDOWS_TOOLCHAIN", "").lower()

    if toolchain_env == "msvc":
        if clang_cl_available:
            use_msvc_compat = True
            print("Using MSVC-compatible toolchain (clang-cl)")
        elif vs_available:
            use_msvc_compat = False
            print("Using native Visual Studio toolchain")
        else:
            print("ERROR: MSVC toolchain requested but not available")
            sys.exit(1)
    elif toolchain_env in ["gnu", "mingw"]:
        if mingw_available:
            use_mingw = True
            print("Using MinGW toolchain")
        else:
            print("ERROR: MinGW toolchain requested but not available")
            sys.exit(1)
    else:
        # Auto-detect: prefer MinGW in Docker/Linux, VS on Windows
        if mingw_available:
            use_mingw = True
            print("Auto-detected MinGW toolchain")
        elif clang_cl_available:
            use_msvc_compat = True
            print("Auto-detected MSVC-compatible toolchain (clang-cl)")
        elif vs_available:
            use_msvc_compat = False
            print("Auto-detected Visual Studio toolchain")
        else:
            print("ERROR: No compatible Windows toolchain found")
            print("Please install MinGW-w64, Visual Studio, or run in Docker")
            sys.exit(1)

    build_windows(incremental=False, config=config, link_type=link_type,
                  use_mingw=use_mingw, use_msvc_compat=use_msvc_compat, jobs=jobs)

    # Skip archive if requested (for multi-toolchain builds where archive happens at the end)
    if no_archive:
        print("==================Skipping Archive (--no-archive)========================")
        return

    # Determine toolchain for archive
    # If archive_toolchain is specified, use it; otherwise detect from build
    if archive_toolchain:
        toolchain = archive_toolchain
    else:
        toolchain = "mingw" if use_mingw else "msvc"

    archive_windows_project(link_type=link_type, toolchain=toolchain)
    print_build_results(link_type=link_type)


# Command-line interface for Windows builds
#
# Usage:
#   python build_windows.py                    # Build Release static library (default)
#   python build_windows.py --ide-project      # Generate Visual Studio project
#   python build_windows.py -j 8               # Build with 8 parallel jobs
#   python build_windows.py --link-type shared # Build shared library
#   python build_windows.py --config Debug     # Build Debug configuration
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build Windows static library",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "-j", "--jobs",
        type=int,
        default=None,
        help="Number of parallel build jobs (default: CPU count)",
    )
    parser.add_argument(
        "--link-type",
        type=str,
        choices=['static', 'shared', 'both'],
        default='both',
        help="Library link type (default: both)",
    )
    parser.add_argument(
        "--ide-project",
        action="store_true",
        help="Generate Visual Studio project instead of building",
    )
    parser.add_argument(
        "--config",
        type=str,
        choices=['Release', 'Debug'],
        default='Release',
        help="Build configuration (default: Release)",
    )
    parser.add_argument(
        "--no-archive",
        action="store_true",
        help="Skip archive step (used for multi-toolchain builds)",
    )
    parser.add_argument(
        "--archive-toolchain",
        type=str,
        choices=['auto', 'mingw', 'msvc'],
        default=None,
        help="Override toolchain for archive (used for multi-toolchain builds)",
    )

    args = parser.parse_args()

    if args.ide_project:
        # Check toolchain availability for IDE project generation
        mingw_available = shutil.which("x86_64-w64-mingw32-gcc") is not None
        clang_cl_available = shutil.which("clang-cl") is not None or shutil.which("cl") is not None

        if mingw_available or clang_cl_available:
            print("WARNING: Project generation not supported with MinGW/clang-cl")
            sys.exit(1)
        else:
            gen_win_project(config=args.config)
    else:
        main(
            config=args.config,
            link_type=args.link_type,
            jobs=args.jobs,
            no_archive=args.no_archive,
            archive_toolchain=args.archive_toolchain,
        )
