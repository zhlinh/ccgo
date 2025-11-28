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

# Load project configuration from CCGO.toml
# If CCGO.toml doesn't exist or returns default "SDK", fallback to directory name
config = load_ccgo_config()
if config["PROJECT_NAME"] == "SDK" and not os.path.exists(os.path.join(SCRIPT_PATH, "CCGO.toml")):
    # Fallback to directory name if CCGO.toml doesn't exist
    PROJECT_NAME = os.path.basename(SCRIPT_PATH).upper()
    PROJECT_NAME_LOWER = PROJECT_NAME.lower()
else:
    # Use project name from CCGO.toml
    PROJECT_NAME = config["PROJECT_NAME"]
    PROJECT_NAME_LOWER = config["PROJECT_NAME_LOWER"]
PROJECT_RELATIVE_PATH = PROJECT_NAME_LOWER

# Build output paths
BUILD_OUT_PATH = "cmake_build/Linux"
INSTALL_PATH = BUILD_OUT_PATH + "/Linux.out"

# CMake build command for Linux Release configuration
# Uses Unix Makefiles generator with parallel build (-j8)
BUILD_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCCGO_CMAKE_DIR="%s" && make -j8 && make install'

# CodeLite IDE project generation command
# CodeLite is a lightweight, cross-platform C/C++ IDE
GEN_PROJECT_CMD = 'cmake ../.. -G "CodeLite - Unix Makefiles" -DCCGO_CMAKE_DIR="%s"'


def build_linux(target_option="", tag="", link_type='static'):
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
        link_type: Library link type ('static', 'shared', or 'both', default: 'static')

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
    print(f"==================build_linux (link_type: {link_type})========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="linux",
    )

    # Add link type CMake flags
    link_type_flags = ""
    if link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    elif link_type == 'shared':
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"
    else:  # both
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=ON"

    # Update BUILD_CMD to include link_type_flags
    build_cmd = f'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags} && make -j8 && make install'

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build fail!!!!!!!!!!!!!!!")
        print("ERROR: Native build failed. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure

    # Dynamically find the actual install directory (could be Darwin.out, Linux.out, etc.)
    # This is needed because CMAKE_SYSTEM_NAME varies by host OS
    actual_install_path = None
    for out_dir in glob.glob(BUILD_OUT_PATH + "/*.out"):
        if glob.glob(out_dir + "/*.a"):
            actual_install_path = out_dir
            print(f"Found install directory: {actual_install_path}")
            break

    if not actual_install_path:
        # Fallback to default INSTALL_PATH
        actual_install_path = INSTALL_PATH
        print(f"Warning: No .a files found, using default: {actual_install_path}")

    # add static libs
    libtool_src_libs = glob.glob(actual_install_path + "/*.a")

    libtool_dst_lib = actual_install_path + f"/{PROJECT_NAME_LOWER}.a"
    if not libtool_libs(libtool_src_libs, libtool_dst_lib):
        print("ERROR: Failed to merge static libraries. Stopping immediately.")
        sys.exit(1)  # Exit immediately on merge failure

    dst_framework_path = actual_install_path + f"/{PROJECT_NAME_LOWER}.dir"
    make_static_framework(
        libtool_dst_lib, dst_framework_path, LINUX_BUILD_COPY_HEADER_FILES, "./"
    )

    # Check the built library architecture
    print("\n==================Verifying Built Library========================")
    final_lib = os.path.join(dst_framework_path, f"{PROJECT_NAME_LOWER}.a")
    if not check_build_libraries(final_lib, platform_hint="linux"):
        print("ERROR: Library verification failed!")
        sys.exit(1)

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

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip
       - Contains stripped library with simplified structure: {project}.libdir/{project}.a
    2. Archive package: (ARCHIVE)_{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip
       - Contains unstripped library for debugging (includes version info)

    Output:
        - target/{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip
        - target/(ARCHIVE)_{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip
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
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Dynamically find the actual install directory (could be Darwin.out, Linux.out, etc.)
    # Look for the directory containing the actual .a file in the .dir subdirectory
    actual_install_path = None
    build_out_full_path = os.path.join(SCRIPT_PATH, BUILD_OUT_PATH)
    for out_dir in glob.glob(build_out_full_path + "/*.out"):
        lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
        test_lib_dir = os.path.join(out_dir, lib_dir_name)
        test_lib_file = os.path.join(test_lib_dir, f"{PROJECT_NAME_LOWER}.a")
        if os.path.exists(test_lib_file):
            actual_install_path = out_dir
            print(f"Found install directory for archive: {actual_install_path}")
            break

    if not actual_install_path:
        # Fallback to default INSTALL_PATH
        actual_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)
        print(f"Warning: Using default install path: {actual_install_path}")

    linux_install_path = actual_install_path

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find source library directory
    lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
    lib_dir_src = os.path.join(linux_install_path, lib_dir_name)

    if not os.path.exists(lib_dir_src):
        print(f"WARNING: Library directory not found at {lib_dir_src}")
        return

    # Create temporary .libdir directory for packaging
    temp_lib_dir = os.path.join(bin_dir, f"{PROJECT_NAME_LOWER}.libdir")
    if os.path.exists(temp_lib_dir):
        shutil.rmtree(temp_lib_dir)
    shutil.copytree(lib_dir_src, temp_lib_dir)
    print(f"Prepared library directory: {temp_lib_dir}")

    # Create main ZIP archive with simplified structure
    main_zip_name = f"{project_name_upper}_LINUX_SDK-{full_version}.zip"
    main_zip_path = os.path.join(bin_dir, main_zip_name)

    print(f"Creating main ZIP archive: {main_zip_name}")
    with zipfile.ZipFile(main_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(temp_lib_dir):
            for file in files:
                file_path = os.path.join(root, file)
                # Use .libdir suffix to distinguish directory from library file
                arcname = os.path.join(
                    f"{PROJECT_NAME_LOWER}.libdir",
                    os.path.relpath(file_path, temp_lib_dir)
                )
                zipf.write(file_path, arcname)

    print(f"Created main archive: {main_zip_path}")

    # Create archive package with unstripped library (includes version info)
    archive_zip_name = f"(ARCHIVE)_{project_name_upper}_LINUX_SDK-{full_version}.zip"
    archive_zip_path = os.path.join(bin_dir, archive_zip_name)

    print(f"Creating archive package: {archive_zip_name}")
    with zipfile.ZipFile(archive_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        # Find the .a file (unstripped)
        static_lib = os.path.join(temp_lib_dir, f"{PROJECT_NAME_LOWER}.a")
        if os.path.exists(static_lib):
            arcname = f"{PROJECT_NAME_LOWER}.libdir/{PROJECT_NAME_LOWER}.a"
            zipf.write(static_lib, arcname)
            print(f"Added unstripped library: {arcname}")

        # Also include headers
        headers_dir = os.path.join(temp_lib_dir, "include")
        if os.path.exists(headers_dir):
            for root, dirs, files in os.walk(headers_dir):
                for file in files:
                    file_path = os.path.join(root, file)
                    arcname = os.path.join(
                        f"{PROJECT_NAME_LOWER}.libdir",
                        os.path.relpath(file_path, temp_lib_dir)
                    )
                    zipf.write(file_path, arcname)

    print(f"Created archive package: {archive_zip_path}")

    # Remove temporary .libdir directory after zipping
    shutil.rmtree(temp_lib_dir)
    print(f"Removed temporary directory: {temp_lib_dir}")

    print("==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    print(f"Archive package: {archive_zip_path}")


def print_build_results():
    """
    Print Linux build results from target directory.

    This function displays the build artifacts and moves them to target/linux/:
    1. Main ZIP archive ({PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip)
    2. Archive package ((ARCHIVE)_{PROJECT_NAME}_LINUX_SDK-{version}-{suffix}.zip)
    3. build_info.json
    """
    print("==================Linux Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for build artifacts (main ZIP and archive ZIP)
    # Main package: {PROJECT_NAME}_LINUX_SDK-*.zip (not starting with (ARCHIVE)_)
    main_zips = [
        f for f in glob.glob(f"{bin_dir}/*_LINUX_SDK-*.zip")
        if not os.path.basename(f).startswith("(ARCHIVE)_")
    ]

    # Archive package: (ARCHIVE)_{PROJECT_NAME}_LINUX_SDK-*.zip
    archive_zips = [
        f for f in glob.glob(f"{bin_dir}/(ARCHIVE)_*_LINUX_SDK-*.zip")
    ]

    if not main_zips and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Create target/linux directory for platform-specific artifacts
    bin_linux_dir = os.path.join(bin_dir, "linux")
    os.makedirs(bin_linux_dir, exist_ok=True)

    # Move archive files to target/linux/
    artifacts_moved = []
    for main_zip in main_zips:
        dest = os.path.join(bin_linux_dir, os.path.basename(main_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(main_zip, dest)
        artifacts_moved.append(os.path.basename(main_zip))

    for archive_zip in archive_zips:
        dest = os.path.join(bin_linux_dir, os.path.basename(archive_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(archive_zip, dest)
        artifacts_moved.append(os.path.basename(archive_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/linux/")

    # Copy build_info.json from cmake_build to target/linux
    copy_build_info_to_target("linux", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/linux/:")
    print("-" * 60)

    # List all files in target/linux directory with sizes
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


def main(target_option="", tag="", link_type='static'):
    """
    Main entry point for Linux static library build.

    This function serves as the primary entry point when building
    distributable Linux static libraries.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Note:
        This function calls build_linux() to create the static library,
        then archives it and moves artifacts to target/linux/ directory.
        For CodeLite project generation, use gen_linux_project() instead.
    """
    # Clean target/linux directory at the start of build
    # Note: build_info.json will be regenerated at the end of the build process
    target_linux_dir = os.path.join(SCRIPT_PATH, "target/linux")
    if os.path.exists(target_linux_dir):
        shutil.rmtree(target_linux_dir)
        print(f"[CLEAN] Removed target/linux directory")

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
