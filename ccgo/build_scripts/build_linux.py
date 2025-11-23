#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_linux.py
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
# substantial portions of the Software.

"""
Linux native library build script.

This script builds static libraries (.a) for Linux platform using CMake
and GCC/Clang toolchain. It handles:
- Building with standard Linux build tools (make, gcc/clang)
- Merging multiple static libraries with ar
- Header file organization
- CodeLite IDE project generation
- Directory-based library organization

Requirements:
- GCC or Clang compiler
- CMake 3.10 or later
- GNU Make
- Python 3.7+
- Linux development environment

Usage:
    python3 build_linux.py [mode]

    mode: 1 (build), 2 (generate CodeLite project), 3 (exit)

Output:
    - Static library: cmake_build/Linux/Linux.out/{project}.dir/{project}.a
    - Headers: cmake_build/Linux/Linux.out/{project}.dir/include/
"""

import os
import sys
import glob

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
BUILD_OUT_PATH = "cmake_build/Linux"
INSTALL_PATH = BUILD_OUT_PATH + "/Linux.out"

# CMake build command for Linux Release configuration
# Uses Unix Makefiles generator with parallel build (-j8)
BUILD_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCCGO_CMAKE_DIR="%s" && make -j8 && make install'

# CodeLite IDE project generation command
# CodeLite is a lightweight, cross-platform C/C++ IDE
GEN_PROJECT_CMD = 'cmake ../.. -G "CodeLite - Unix Makefiles" -DCCGO_CMAKE_DIR="%s"'


def build_linux(target_option="", tag=""):
    """
    Build Linux static library with GCC/Clang toolchain.

    This function performs the complete Linux build process:
    1. Generates version info header file
    2. Cleans build directory
    3. Configures and builds with CMake and make
    4. Merges multiple static libraries into single .a file using ar
    5. Creates directory structure with library and headers

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Static library: Linux.out/{project}.dir/{project}.a
        - Headers: Linux.out/{project}.dir/include/

    Note:
        The .a file is an archive containing merged static libraries.
        On Linux, the ar tool is used for merging (similar to libtool on macOS).
        The resulting library can be linked into applications using -l flag.
    """
    before_time = time.time()
    print("==================build_linux========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="linux",
    )
    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    ret = os.system(BUILD_CMD % CCGO_CMAKE_DIR)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build fail!!!!!!!!!!!!!!!")
        return False

    # add static libs
    libtool_src_libs = glob.glob(INSTALL_PATH + "/*.a")

    libtool_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}.a"
    if not libtool_libs(libtool_src_libs, libtool_dst_lib):
        return False

    dst_framework_path = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}.dir"
    make_static_framework(
        libtool_dst_lib, dst_framework_path, LINUX_BUILD_COPY_HEADER_FILES, "./"
    )

    print("==================Output========================")
    print(dst_framework_path)


def gen_linux_project(target_option="", tag=""):
    """
    Generate CodeLite project for Linux development and debugging.

    This function creates a CodeLite workspace and project files that can be
    opened in CodeLite IDE for interactive development, debugging, and testing.
    The project is automatically opened in CodeLite after generation.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - CodeLite workspace: cmake_build/Linux/{project}.workspace (auto-opened)

    Note:
        CodeLite is a lightweight, cross-platform C/C++ IDE with good
        CMake integration. This is useful for Linux development workflows
        where you need IDE features like debugging, code completion,
        and integrated build tools. The workspace is automatically
        opened in CodeLite after generation.
    """
    print("==================gen_linux_project========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="linux",
    )
    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    cmd = GEN_PROJECT_CMD % CCGO_CMAKE_DIR
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!gen fail!!!!!!!!!!!!!!!")
        return False

    project_file_prefix = os.path.join(SCRIPT_PATH, BUILD_OUT_PATH, PROJECT_NAME_LOWER)
    project_file = get_project_file_name(project_file_prefix)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {project_file}")

    os.system(get_open_project_file_cmd(project_file))

    return True


def archive_linux_project():
    """
    Archive Linux static library and related build artifacts.

    This function creates an archive package containing:
    1. Static library (.a file) and headers
    2. Build artifacts

    The archive is packaged into a ZIP file named:
    (ARCHIVE)_{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip

    Output:
        - bin/linux/{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.dir/
        - bin/linux/(ARCHIVE)_{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip
    """
    import zipfile
    from pathlib import Path

    print("==================Archive Linux Project========================")

    # Get project version info
    version_name = get_version_name(SCRIPT_PATH)
    project_name_upper = PROJECT_NAME.upper()

    # Try to get publish suffix from git tags or use beta.0 as default
    try:
        git_tags = os.popen("git describe --tags --abbrev=0 2>/dev/null").read().strip()
        if git_tags and "-" in git_tags:
            suffix = git_tags.split("-", 1)[1]
        else:
            git_branch = (
                os.popen("git rev-parse --abbrev-ref HEAD 2>/dev/null").read().strip()
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
    bin_dir = os.path.join(SCRIPT_PATH, "bin")
    linux_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create bin directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find and copy library directory
    lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
    lib_dir_src = os.path.join(linux_install_path, lib_dir_name)

    if not os.path.exists(lib_dir_src):
        print(f"WARNING: Library directory not found at {lib_dir_src}")
        return

    lib_dir_dest = os.path.join(
        bin_dir, f"{project_name_upper}_LINUX_SDK-{full_version}.dir"
    )
    if os.path.exists(lib_dir_dest):
        shutil.rmtree(lib_dir_dest)
    shutil.copytree(lib_dir_src, lib_dir_dest)
    print(f"Copied library directory: {lib_dir_dest}")

    # Create archive directory structure
    archive_name = f"(ARCHIVE)_{project_name_upper}_LINUX_SDK-{full_version}"
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
    Print Linux build results from bin directory.

    This function displays the build artifacts and moves them to bin/linux/:
    1. Static library directory
    2. ARCHIVE zip
    3. Other build artifacts
    """
    print("==================Linux Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "bin")

    # Check if bin directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: bin directory not found. Please run build first.")
        sys.exit(1)

    # Check for build artifacts
    lib_dirs = [
        f for f in glob.glob(f"{bin_dir}/*.dir") if "LINUX_SDK" in os.path.basename(f)
    ]
    archive_zips = glob.glob(f"{bin_dir}/(ARCHIVE)*.zip")

    if not lib_dirs and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Create bin/linux directory for platform-specific artifacts
    bin_linux_dir = os.path.join(bin_dir, "linux")
    os.makedirs(bin_linux_dir, exist_ok=True)

    # Move library directories and archive files to bin/linux/
    artifacts_moved = []
    for lib_dir in lib_dirs:
        dest = os.path.join(bin_linux_dir, os.path.basename(lib_dir))
        if os.path.exists(dest):
            shutil.rmtree(dest)
        shutil.move(lib_dir, dest)
        artifacts_moved.append(os.path.basename(lib_dir))

    for archive_zip in archive_zips:
        dest = os.path.join(bin_linux_dir, os.path.basename(archive_zip))
        shutil.move(archive_zip, dest)
        artifacts_moved.append(os.path.basename(archive_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to bin/linux/")

    # Copy build_info.json from cmake_build to bin/linux
    copy_build_info_to_bin("linux", SCRIPT_PATH)

    print(f"\nBuild artifacts in bin/linux/:")
    print("-" * 60)

    # List all files in bin/linux directory with sizes
    for item in sorted(os.listdir(bin_linux_dir)):
        item_path = os.path.join(bin_linux_dir, item)
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


def main(target_option="", tag=""):
    """
    Main entry point for Linux static library build.

    This function serves as the primary entry point when building
    distributable Linux static libraries.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Note:
        This function calls build_linux() to create the static library,
        then archives it and moves artifacts to bin/linux/ directory.
        For CodeLite project generation, use gen_linux_project() instead.
    """
    # Build static library
    build_linux(target_option=target_option, tag=tag)

    # Archive and organize artifacts
    archive_linux_project()
    print_build_results()


# Command-line interface for Linux builds
# Supports two invocation modes:
# 1. Interactive mode (no args): Prompts user for build mode
# 2. Mode only (1 arg): Uses specified mode directly
#
# Build modes:
# 1 - Build static library: Creates .a archive with merged libraries
# 2 - Generate CodeLite project: Creates workspace and opens in CodeLite IDE
# 3 - Exit: Quit without building
if __name__ == "__main__":
    while True:
        if len(sys.argv) >= 2:
            num = sys.argv[1]
        else:
            num = str(
                input(
                    "Enter menu:"
                    + "\n1. Clean && Build Linux."
                    + "\n2. Gen Linux CodeLite Project."
                    + "\n3. Exit\n"
                )
            )
        if num == "1":
            main(tag=num)
            break
        elif num == "2":
            gen_linux_project(tag=num)
            break
        elif num == "3":
            break
        else:
            main()
            break

if __name__ == "__main__":
    main()
