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

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

# Script configuration
SCRIPT_PATH = os.getcwd()
# Directory name as project name
PROJECT_NAME = os.path.basename(SCRIPT_PATH).upper()

# Ensure cmake directory exists in project
PROJECT_NAME_LOWER = PROJECT_NAME.lower()
PROJECT_RELATIVE_PATH = PROJECT_NAME.lower()

# Build output paths
BUILD_OUT_PATH = "cmake_build/Windows"
INSTALL_PATH = BUILD_OUT_PATH + "/Windows.out/"

# Visual Studio 2019 build configuration
# Uses Visual Studio 16 2019 generator with v142 platform toolset (C++17 support)
WIN_BUILD_CMD = 'cmake ../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="%s" && cmake --build . --target install --config %s'
WIN_GEN_PROJECT_CMD = (
    'cmake ../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="%s"'
)
WIN_ARCH = "x64"  # Target architecture (64-bit Windows)
WIN_SRC_DIR = "src"  # Source directory name for PDB collection
THIRD_PARTY_MERGE_LIBS = ["pthread"]  # Third-party libraries to merge into final .lib


def build_windows(incremental, tag="", config="Release", link_type='static', use_mingw=False):
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
        tag: Version tag string for metadata (default: '')
        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
        link_type: Library link type ('static', 'shared', or 'both', default: 'static')

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
    before_time = time.time()
    print(f"==================build_windows (link_type: {link_type})========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        incremental=incremental,
        platform="windows",
    )

    # Add link type CMake flags
    link_type_flags = ""
    if link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    elif link_type == 'shared':
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"
    else:  # both
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=ON"

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    if use_mingw:
        # MinGW cross-compilation (for Docker/Linux environments)
        # Use Unix Makefiles generator with MinGW compilers
        cmake_config_cmd = (
            f'cmake ../.. '
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
        cmake_build_cmd = 'cmake --build . --target install'
        cmd = f'{cmake_config_cmd} && {cmake_build_cmd}'
    else:
        # Visual Studio build (native Windows)
        cmd = f'cmake ../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags} && cmake --build . --target install --config {config}'

    print("build cmd:" + cmd)
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!build fail!!!!!!!!!!!!!!!!!!!!")
        return False

    win_result_dir = INSTALL_PATH + f"{PROJECT_NAME_LOWER}.dir/" + WIN_ARCH
    if os.path.exists(win_result_dir):
        shutil.rmtree(win_result_dir)
    os.makedirs(win_result_dir)

    needed_libs = glob.glob(INSTALL_PATH + "*.lib")

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
        # MinGW builds: Copy libraries directly without merging
        # MinGW produces .a files, not .lib files
        # Just copy the built library to the output directory
        mingw_libs = glob.glob(INSTALL_PATH + "*.a")
        for lib in mingw_libs:
            shutil.copy(lib, win_result_dir)
        print(f"Copied MinGW libraries: {mingw_libs}")
    else:
        # Visual Studio builds: Merge multiple .lib files into one
        if not merge_win_static_libs(
            needed_libs, win_result_dir + f"/{PROJECT_NAME_LOWER}.lib"
        ):
            print("!!!!!!!!!!!!!!!!!!merge libs fail!!!!!!!!!!!!!!!!!!!!")
            return False

    headers = dict()
    headers.update(WINDOWS_BUILD_COPY_HEADER_FILES)
    copy_file_mapping(headers, "./", win_result_dir + "/include")

    if use_mingw:
        # MinGW doesn't generate PDB files (uses DWARF debug info instead)
        print("MinGW build: Skipping PDB collection (not applicable)")
    else:
        # Visual Studio builds: Copy PDB debug symbol files
        sub_folders = filtered_lib_names
        # copy pdb of third_party
        copy_windows_pdb(BUILD_OUT_PATH, sub_folders, config, INSTALL_PATH)
        src_dir_folder = PROJECT_NAME_LOWER + "-" + WIN_SRC_DIR
        # copy pdb of src
        sub_folders = list(
            map(lambda x: x.replace(PROJECT_NAME_LOWER, src_dir_folder), sub_folders)
        )
        copy_windows_pdb(
            os.path.join(BUILD_OUT_PATH, src_dir_folder), sub_folders, config, INSTALL_PATH
        )

    # zip pdb files (Visual Studio only)
    if not use_mingw:
        pdf_suffix = ".pdb"
        zip_files_ends_with(
            INSTALL_PATH,
            pdf_suffix,
            win_result_dir + f"/{PROJECT_NAME_LOWER}{pdf_suffix}.zip",
        )

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"libs: {win_result_dir}")
    if not use_mingw:
        print(f"pdb files: {INSTALL_PATH}")

    after_time = time.time()
    print(f"use time: {int(after_time - before_time)} s")
    return True


def gen_win_project(tag="", config="Release"):
    """
    Generate Visual Studio project for Windows development and debugging.

    This function creates a Visual Studio solution (.sln) and project files
    that can be opened in Visual Studio for interactive development, debugging,
    and testing. The project is automatically opened in Visual Studio after generation.

    Args:
        tag: Version tag string for metadata (default: '')
        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
              Note: This parameter is currently unused but reserved for future use

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - VS solution: cmake_build/Windows/{project}.sln (auto-opened)

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
        tag,
        platform="windows",
    )
    clean(BUILD_OUT_PATH, False)
    os.chdir(BUILD_OUT_PATH)
    ret = os.system(WIN_GEN_PROJECT_CMD % CCGO_CMAKE_DIR)
    os.chdir(SCRIPT_PATH)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!gen project file fail!!!!!!!!!!!!!!!!!!!!")
        return False

    project_file_prefix = os.path.join(SCRIPT_PATH, BUILD_OUT_PATH, PROJECT_NAME_LOWER)
    project_file = get_project_file_name(project_file_prefix)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {project_file}")

    os.system(get_open_project_file_cmd(project_file))

    return True


def archive_windows_project():
    """
    Archive Windows static library and related build artifacts.

    This function creates an archive package containing:
    1. Static library (.lib file) and headers
    2. Build artifacts

    The archive is packaged into a ZIP file named:
    (ARCHIVE)_{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip

    Output:
        - target/windows/{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.dir/
        - target/windows/(ARCHIVE)_{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip
    """
    import zipfile
    from pathlib import Path

    print("==================Archive Windows Project========================")

    # Get project version info
    version_name = get_version_name(SCRIPT_PATH)
    project_name_upper = PROJECT_NAME.upper()

    # Try to get publish suffix from git tags or use beta.0 as default
    try:
        git_tags = os.popen("git describe --tags --abbrev=0 2>nul").read().strip()
        if git_tags and "-" in git_tags:
            suffix = git_tags.split("-", 1)[1]
        else:
            git_branch = (
                os.popen("git rev-parse --abbrev-ref HEAD 2>nul").read().strip()
            )
            if git_branch == "master" or git_branch == "main":
                suffix = "release"
            else:
                suffix = "beta.0"
    except:
        suffix = "beta.0"

    # Build full version name with suffix
    full_version = f"{version_name}-{suffix}" if suffix else version_name

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    windows_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find and copy library directory
    lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
    lib_dir_src = os.path.join(windows_install_path, lib_dir_name)

    if not os.path.exists(lib_dir_src):
        print(f"WARNING: Library directory not found at {lib_dir_src}")
        return

    lib_dir_dest = os.path.join(
        bin_dir, f"{project_name_upper}_WINDOWS_SDK-{full_version}.dir"
    )
    if os.path.exists(lib_dir_dest):
        shutil.rmtree(lib_dir_dest)
    shutil.copytree(lib_dir_src, lib_dir_dest)
    print(f"Copied library directory: {lib_dir_dest}")

    # Create archive directory structure
    archive_name = f"(ARCHIVE)_{project_name_upper}_WINDOWS_SDK-{full_version}"
    archive_dir = os.path.join(bin_dir, archive_name)

    if os.path.exists(archive_dir):
        shutil.rmtree(archive_dir)
    os.makedirs(archive_dir, exist_ok=True)

    # Copy library directory to archive
    archive_lib_dir = os.path.join(archive_dir, lib_dir_name)
    shutil.copytree(lib_dir_src, archive_lib_dir)
    print(f"Copied library directory to archive: {lib_dir_name}")

    # Create ZIP archive
    zip_file_path = os.path.join(bin_dir, f"{archive_name}.zip")
    with zipfile.ZipFile(zip_file_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(archive_dir):
            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, bin_dir)
                zipf.write(file_path, arcname)

    # Remove temporary archive directory
    shutil.rmtree(archive_dir)

    print("==================Archive Complete========================")
    print(f"Library directory: {lib_dir_dest}")
    print(f"Archive ZIP: {zip_file_path}")


def print_build_results():
    """
    Print Windows build results from target directory.

    This function displays the build artifacts and moves them to target/windows/:
    1. Static library directory
    2. ARCHIVE zip
    3. Other build artifacts
    """
    print("==================Windows Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for build artifacts
    lib_dirs = [
        f for f in glob.glob(f"{bin_dir}/*.dir") if "WINDOWS_SDK" in os.path.basename(f)
    ]
    archive_zips = glob.glob(f"{bin_dir}/(ARCHIVE)*.zip")

    if not lib_dirs and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Create target/windows directory for platform-specific artifacts
    bin_windows_dir = os.path.join(bin_dir, "windows")
    os.makedirs(bin_windows_dir, exist_ok=True)

    # Move library directories and archive files to target/windows/
    artifacts_moved = []
    for lib_dir in lib_dirs:
        dest = os.path.join(bin_windows_dir, os.path.basename(lib_dir))
        if os.path.exists(dest):
            shutil.rmtree(dest)
        shutil.move(lib_dir, dest)
        artifacts_moved.append(os.path.basename(lib_dir))

    for archive_zip in archive_zips:
        dest = os.path.join(bin_windows_dir, os.path.basename(archive_zip))
        shutil.move(archive_zip, dest)
        artifacts_moved.append(os.path.basename(archive_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/windows/")

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


def main():
    """
    Main entry point for Windows build and project generation.

    This function validates the Visual Studio environment and provides
    an interactive or command-line interface for building libraries
    and generating Visual Studio projects.

    Raises:
        Returns early if Visual Studio environment check fails

    Build Options:
        1 - Build Release configuration with merged static library
        2 - Generate Visual Studio project and open in IDE
        3 - Build Debug configuration with debug symbols
        4 - Exit without action

    Note:
        Requires Visual Studio 2019 or later to be installed, OR MinGW-w64
        for cross-compilation from Linux/macOS (Docker containers).
        The VS environment check ensures required tools are available.
    """
    # Check if MinGW is available (for Docker/cross-compilation)
    mingw_available = shutil.which("x86_64-w64-mingw32-gcc") is not None

    if not mingw_available and not check_vs_env():
        # Neither MinGW nor Visual Studio available
        return

    # Use MinGW if available (takes precedence in Docker/Linux environments)
    use_mingw = mingw_available

    # Command-line interface for Windows builds
    # Supports two invocation modes:
    # 1. Interactive mode (no args): Prompts user for build mode
    # 2. Mode only (1 arg): Uses specified mode directly
    #
    # Build modes:
    # 1 - Build Release: Builds static library in Release configuration
    # 2 - Generate VS project: Creates .sln and opens in Visual Studio
    # 3 - Build Debug: Builds static library in Debug configuration with full symbols
    # 4 - Exit: Quit without building
    while True:
        if len(sys.argv) >= 2:
            num = sys.argv[1]
        else:
            num = input(
                "Enter menu:"
                + f"\n1. Clean && build {PROJECT_NAME_LOWER} Release."
                + f"\n2. Gen Project {PROJECT_NAME_LOWER} file."
                + f"\n3. Clean && build {PROJECT_NAME_LOWER} Debug."
                + "\n4. Exit\n"
            )
        print(f"==================Windows Choose num: {num}==================")
        if num == "1":
            build_windows(incremental=False, tag=num, config="Release", use_mingw=use_mingw)
            # Archive and organize artifacts
            archive_windows_project()
            print_build_results()
            break
        elif num == "2":
            if use_mingw:
                print("WARNING: Project generation not supported with MinGW")
                print("Using MinGW cross-compilation instead")
                build_windows(incremental=False, tag=num, config="Release", use_mingw=use_mingw)
                archive_windows_project()
                print_build_results()
            else:
                gen_win_project(tag=num, config="Release")
            break
        elif num == "3":
            build_windows(incremental=False, tag=num, config="Debug", use_mingw=use_mingw)
            # Archive and organize artifacts
            archive_windows_project()
            print_build_results()
            break
        elif num == "4":
            break
        else:
            break


if __name__ == "__main__":
    main()
