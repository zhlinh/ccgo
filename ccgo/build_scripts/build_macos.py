#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_macos.py
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
macOS native library build script.

This script builds universal static libraries and frameworks for macOS platform
using CMake. It handles:
- Building for Apple Silicon (arm64, arm64e)
- Building for Intel processors (x86_64)
- Merging architectures into universal binaries with libtool
- Creating .framework bundles for distribution
- Xcode project generation for development

Requirements:
- Xcode 12.0 or later with command line tools
- CMake 3.10 or later
- Python 3.7+
- macOS development environment

Usage:
    python3 build_macos.py [mode]

    mode: 1 (build framework), 2 (generate Xcode project), 3 (exit)

Output:
    - Universal framework: cmake_build/macOS/Darwin.out/{project}.framework
    - Framework supports both Intel and Apple Silicon Macs
"""

import glob
import os
import sys
import time

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
BUILD_OUT_PATH = "cmake_build/macOS"
INSTALL_PATH = BUILD_OUT_PATH + "/Darwin.out"

# CMake build command for macOS (defaults to x86_64 if no arch specified)
# Disables ARC and Bitcode for C/C++ native libraries
MACOS_BUILD_OS_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DCCGO_CMAKE_DIR="%s" %s && make -j8 && make install'

# CMake build command for Apple Silicon Macs (M1, M2, M3, etc.)
# Builds for arm64 and arm64e architectures
MACOS_BUILD_ARM_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DCMAKE_OSX_ARCHITECTURES="arm64;arm64e" -DCCGO_CMAKE_DIR="%s" %s && make -j8 && make install'

# CMake build command for Intel Macs
# Builds for x86_64 architecture only
MACOS_BUILD_X86_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DCMAKE_OSX_ARCHITECTURES="x86_64" -DCCGO_CMAKE_DIR="%s" %s && make -j8 && make install'

# Xcode project generation command
# Targets macOS 10.9+ for broad compatibility, disables Bitcode
GEN_MACOS_PROJ = 'cmake ../.. -G Xcode -DCMAKE_OSX_DEPLOYMENT_TARGET:STRING=10.9 -DENABLE_BITCODE=0 -DCCGO_CMAKE_DIR="%s" %s'


def build_macos(target_option="", tag=""):
    """
    Build universal macOS framework supporting both Intel and Apple Silicon.

    This function performs the complete macOS build process:
    1. Generates version info header file
    2. Builds static libraries for Apple Silicon (arm64, arm64e)
    3. Merges ARM static libraries using libtool
    4. Builds static libraries for Intel (x86_64)
    5. Merges Intel static libraries using libtool
    6. Combines ARM and Intel libraries into universal binary with libtool
    7. Creates .framework bundle with universal binary

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Universal framework: cmake_build/macOS/Darwin.out/{project}.framework

    Note:
        The resulting framework is a universal binary that runs natively on both
        Intel and Apple Silicon Macs. This provides optimal performance on all
        Mac architectures without requiring Rosetta 2 translation.
    """
    before_time = time.time()
    print("==================build_macos========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="macos",
    )
    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = MACOS_BUILD_ARM_CMD % (CCGO_CMAKE_DIR, target_option)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build os fail!!!!!!!!!!!!!!!")
        return False

    lipo_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}"
    libtool_os_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}_os"
    libtool_simulator_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}_simulator"
    dst_framework_path = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}.framework"
    dst_framework_headers = MACOS_BUILD_COPY_HEADER_FILES
    # add static libs
    total_src_lib = glob.glob(INSTALL_PATH + "/*.a")
    rm_src_lib = []
    libtool_src_lib = [x for x in total_src_lib if x not in rm_src_lib]
    print(f"libtool src lib: {len(libtool_src_lib)}/{len(total_src_lib)}")

    if not libtool_libs(libtool_src_lib, libtool_os_dst_lib):
        return False

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = MACOS_BUILD_X86_CMD % (CCGO_CMAKE_DIR, target_option)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build simulator fail!!!!!!!!!!!!!!!")
        return False
    if not libtool_libs(glob.glob(INSTALL_PATH + "/*.a"), libtool_simulator_dst_lib):
        return False

    # src libs to be libtool
    lipo_src_libs = []
    lipo_src_libs.append(libtool_os_dst_lib)
    lipo_src_libs.append(libtool_simulator_dst_lib)
    # if len(target_option) <= 0:
    #    lipo_src_libs.append(libtool_xlog_dst_lib)

    if not libtool_libs(lipo_src_libs, lipo_dst_lib):
        return False

    make_static_framework(lipo_dst_lib, dst_framework_path, dst_framework_headers, "./")

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_framework_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def archive_macos_project():
    """
    Archive macOS framework and related build artifacts.

    This function creates an archive package containing:
    1. Framework file (copied to bin/macos/)
    2. Header files

    The archive is packaged into a ZIP file named:
    (ARCHIVE)_{PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.zip

    Output:
        - bin/macos/{PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.framework
        - bin/macos/(ARCHIVE)_{PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.zip
    """
    import zipfile
    from pathlib import Path

    print("==================Archive macOS Project========================")

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
    macos_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create bin directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find and copy Framework
    framework_name = f"{PROJECT_NAME_LOWER}.framework"
    framework_src = os.path.join(macos_install_path, framework_name)

    if not os.path.exists(framework_src):
        print(f"WARNING: Framework not found at {framework_src}")
        return

    framework_dest = os.path.join(
        bin_dir, f"{project_name_upper}_MACOS_SDK-{full_version}.framework"
    )
    if os.path.exists(framework_dest):
        shutil.rmtree(framework_dest)
    shutil.copytree(framework_src, framework_dest)
    print(f"Copied Framework: {framework_dest}")

    # Create archive directory structure
    archive_name = f"(ARCHIVE)_{project_name_upper}_MACOS_SDK-{full_version}"
    archive_dir = os.path.join(bin_dir, archive_name)

    if os.path.exists(archive_dir):
        shutil.rmtree(archive_dir)
    os.makedirs(archive_dir, exist_ok=True)

    # Copy Framework to archive
    archive_framework = os.path.join(archive_dir, framework_name)
    shutil.copytree(framework_src, archive_framework)
    print(f"Copied Framework to archive: {framework_name}")

    # Copy header files if they exist
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        headers_dest = os.path.join(archive_dir, "include")
        shutil.copytree(headers_src, headers_dest)
        print(f"Copied headers: include")

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
    print(f"Framework: {framework_dest}")
    print(f"Archive ZIP: {zip_file_path}")


def print_build_results():
    """
    Print macOS build results from bin directory.

    This function displays the build artifacts and moves them to bin/macos/:
    1. Framework
    2. ARCHIVE zip
    3. Other build artifacts
    """
    print("==================macOS Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "bin")

    # Check if bin directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: bin directory not found. Please run build first.")
        sys.exit(1)

    # Check for build artifacts
    framework_files = [
        f
        for f in glob.glob(f"{bin_dir}/*.framework")
        if "MACOS_SDK" in os.path.basename(f)
    ]
    archive_zips = glob.glob(f"{bin_dir}/(ARCHIVE)*.zip")

    if not framework_files and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Create bin/macos directory for platform-specific artifacts
    bin_macos_dir = os.path.join(bin_dir, "macos")
    os.makedirs(bin_macos_dir, exist_ok=True)

    # Move framework and archive files to bin/macos/
    artifacts_moved = []
    for framework_file in framework_files:
        dest = os.path.join(bin_macos_dir, os.path.basename(framework_file))
        if os.path.exists(dest):
            shutil.rmtree(dest)
        shutil.move(framework_file, dest)
        artifacts_moved.append(os.path.basename(framework_file))

    for archive_zip in archive_zips:
        dest = os.path.join(bin_macos_dir, os.path.basename(archive_zip))
        shutil.move(archive_zip, dest)
        artifacts_moved.append(os.path.basename(archive_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to bin/macos/")

    # Copy build_info.json from cmake_build to bin/macos
    copy_build_info_to_bin("macos", SCRIPT_PATH)

    print(f"\nBuild artifacts in bin/macos/:")
    print("-" * 60)

    # List all files in bin/macos directory with sizes
    for item in sorted(os.listdir(bin_macos_dir)):
        item_path = os.path.join(bin_macos_dir, item)
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


def gen_macos_project(target_option="", tag=""):
    """
    Generate Xcode project for macOS development and debugging.

    This function creates an Xcode project (.xcodeproj) that can be opened in Xcode
    for interactive development, debugging, and testing. The project is automatically
    opened in Xcode after generation.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - Xcode project: cmake_build/macOS/{project}.xcodeproj (auto-opened)

    Note:
        The project targets macOS 10.9+ for broad compatibility.
        This is useful for development workflows where you need Xcode's
        debugging tools, code completion, and build system integration.
        The project file is automatically opened in Xcode after generation.
    """
    print("==================gen_macos_project========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="macos",
    )
    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    cmd = GEN_MACOS_PROJ % (CCGO_CMAKE_DIR, target_option)
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


def main(target_option="", tag=""):
    """
    Main entry point for macOS universal framework build.

    This function serves as the primary entry point when building
    distributable macOS frameworks with universal binary support.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Note:
        This function calls build_macos() to create the universal framework,
        then archives it and moves artifacts to bin/macos/ directory.
        For Xcode project generation, use gen_macos_project() instead.
    """
    print("main tag %s" % tag)

    # Build universal framework
    if not build_macos(target_option, tag):
        print("ERROR: macOS build failed")
        sys.exit(1)

    # Archive and organize artifacts
    archive_macos_project()
    print_build_results()


# Command-line interface for macOS builds
# Supports two invocation modes:
# 1. Interactive mode (no args): Prompts user for build mode
# 2. Mode only (1 arg): Uses specified mode directly
#
# Build modes:
# 1 - Build universal framework: Creates .framework with Intel + Apple Silicon binaries
# 2 - Generate Xcode project: Creates .xcodeproj and opens in Xcode for development
# 3 - Exit: Quit without building
if __name__ == "__main__":
    while True:
        if len(sys.argv) >= 2:
            num = sys.argv[1]
        else:
            archs = set(["armeabi-v7a"])
            num = str(
                input(
                    "Enter menu:"
                    + f"\n1. Clean && Build macOS {PROJECT_NAME_LOWER}."
                    + f"\n2. Gen macOS {PROJECT_NAME_LOWER} Project."
                    + "\n3. Exit.\n"
                )
            )
        print(f"==================MacOS Choose num: {num}==================")
        if num == "1":
            main(tag=num)
            break
        elif num == "2":
            gen_macos_project(tag=num)
            break
        else:
            break
