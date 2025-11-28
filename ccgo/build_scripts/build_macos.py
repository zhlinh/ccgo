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


def build_macos(target_option="", tag="", link_type='static'):
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
        link_type: Library link type ('static', 'shared', or 'both', default: 'static')

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
    print(f"==================build_macos (link_type: {link_type})========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="macos",
    )

    # Add link type CMake flags
    link_type_flags = ""
    if link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    elif link_type == 'shared':
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"
    else:  # both
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=ON"

    # Combine with existing target options
    full_target_option = f"{link_type_flags} {target_option}".strip()

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = MACOS_BUILD_ARM_CMD % (CCGO_CMAKE_DIR, full_target_option)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build os fail!!!!!!!!!!!!!!!")
        print("ERROR: Native build failed for macOS ARM. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure

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
        print("ERROR: Failed to merge ARM libraries. Stopping immediately.")
        sys.exit(1)  # Exit immediately on merge failure

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = MACOS_BUILD_X86_CMD % (CCGO_CMAKE_DIR, full_target_option)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build simulator fail!!!!!!!!!!!!!!!")
        print("ERROR: Native build failed for macOS x86. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure
    if not libtool_libs(glob.glob(INSTALL_PATH + "/*.a"), libtool_simulator_dst_lib):
        print("ERROR: Failed to merge x86 libraries. Stopping immediately.")
        sys.exit(1)  # Exit immediately on merge failure

    # src libs to be libtool
    lipo_src_libs = []
    lipo_src_libs.append(libtool_os_dst_lib)
    lipo_src_libs.append(libtool_simulator_dst_lib)
    # if len(target_option) <= 0:
    #    lipo_src_libs.append(libtool_xlog_dst_lib)

    if not libtool_libs(lipo_src_libs, lipo_dst_lib):
        print("ERROR: Failed to create universal binary. Stopping immediately.")
        sys.exit(1)  # Exit immediately on universal binary creation failure

    make_static_framework(lipo_dst_lib, dst_framework_path, dst_framework_headers, "./")

    # Check the built universal binary architecture
    print("\n==================Verifying macOS Universal Binary========================")
    framework_lib = os.path.join(dst_framework_path, PROJECT_NAME_LOWER)
    if os.path.exists(framework_lib):
        check_library_architecture(framework_lib, platform_hint="macos")
    else:
        # Try with .a extension
        framework_lib_a = f"{framework_lib}.a"
        if os.path.exists(framework_lib_a):
            check_library_architecture(framework_lib_a, platform_hint="macos")
    print("========================================================================")

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_framework_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def archive_macos_project():
    """
    Archive macOS framework into ZIP packages.

    This function creates two ZIP packages:
    1. Release ZIP (framework only): {PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.zip
    2. Archive ZIP (with symbols, headers, etc.): (ARCHIVE)_{PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.zip

    Output:
        - target/macos/{PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.zip
          (contains {project_name}.framework)
        - target/macos/(ARCHIVE)_{PROJECT_NAME}_MACOS_SDK-{version}-{suffix}.zip
          (contains framework, headers, symbols, etc.)
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
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    macos_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find framework
    framework_name = f"{PROJECT_NAME_LOWER}.framework"
    framework_src = os.path.join(macos_install_path, framework_name)

    if not os.path.exists(framework_src):
        print(f"WARNING: Framework not found at {framework_src}")
        return

    # ========== 1. Create Release ZIP (framework only) ==========
    print("\n--- Creating Release ZIP (framework only) ---")
    temp_release_dir = os.path.join(bin_dir, f"_temp_release_{project_name_upper}")
    if os.path.exists(temp_release_dir):
        shutil.rmtree(temp_release_dir)
    os.makedirs(temp_release_dir, exist_ok=True)

    # Copy Framework to temporary directory
    temp_framework = os.path.join(temp_release_dir, framework_name)
    shutil.copytree(framework_src, temp_framework)
    print(f"Copied Framework: {framework_name}")

    # Create Release ZIP
    release_zip_name = f"{project_name_upper}_MACOS_SDK-{full_version}.zip"
    release_zip_path = os.path.join(bin_dir, release_zip_name)

    if os.path.exists(release_zip_path):
        os.remove(release_zip_path)

    with zipfile.ZipFile(release_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(temp_release_dir):
            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, temp_release_dir)
                zipf.write(file_path, arcname)

    shutil.rmtree(temp_release_dir)
    print(f"Created Release ZIP: {release_zip_name}")

    # ========== 2. Create Archive ZIP (with additional files) ==========
    print("\n--- Creating Archive ZIP (with symbols and headers) ---")
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
        print(f"Copied headers: include/")

    # Copy symbol files if they exist (dSYM for macOS)
    dsym_pattern = f"{macos_install_path}/*.dSYM"
    dsym_files = glob.glob(dsym_pattern)
    if dsym_files:
        for dsym_file in dsym_files:
            dsym_name = os.path.basename(dsym_file)
            dsym_dest = os.path.join(archive_dir, dsym_name)
            shutil.copytree(dsym_file, dsym_dest)
            print(f"Copied symbols: {dsym_name}")

    # Create Archive ZIP
    archive_zip_path = os.path.join(bin_dir, f"{archive_name}.zip")
    if os.path.exists(archive_zip_path):
        os.remove(archive_zip_path)

    with zipfile.ZipFile(archive_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(archive_dir):
            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, bin_dir)
                zipf.write(file_path, arcname)

    shutil.rmtree(archive_dir)
    print(f"Created Archive ZIP: {archive_name}.zip")

    print("\n==================Archive Complete========================")
    print(f"Release ZIP: {release_zip_path}")
    print(f"Archive ZIP: {archive_zip_path}")


def print_build_results():
    """
    Print macOS build results from target directory.

    This function displays the build artifacts and moves them to target/macos/:
    - Release SDK ZIP packages (framework only)
    - Archive SDK ZIP packages (with symbols, headers, etc.)
    """
    print("==================macOS Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for SDK ZIP packages (both release and archive)
    all_zips = glob.glob(f"{bin_dir}/*.zip")
    sdk_zips = [
        f for f in all_zips
        if "MACOS_SDK" in os.path.basename(f) and not os.path.basename(f).startswith("_temp_")
    ]

    if not sdk_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Create target/macos directory for platform-specific artifacts
    bin_macos_dir = os.path.join(bin_dir, "macos")
    os.makedirs(bin_macos_dir, exist_ok=True)

    # Clean up old framework directories in target/macos/
    for item in os.listdir(bin_macos_dir):
        item_path = os.path.join(bin_macos_dir, item)
        if os.path.isdir(item_path) and item.endswith('.framework'):
            shutil.rmtree(item_path)
            print(f"Cleaned up old framework: {item}")

    # Move SDK ZIP files to target/macos/
    artifacts_moved = []
    for sdk_zip in sdk_zips:
        dest = os.path.join(bin_macos_dir, os.path.basename(sdk_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(sdk_zip, dest)
        artifacts_moved.append(os.path.basename(sdk_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/macos/")

    # Copy build_info.json from cmake_build to target/macos
    copy_build_info_to_target("macos", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/macos/:")
    print("-" * 60)

    # List all files in target/macos directory with sizes
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


def main(target_option="", tag="", link_type='static'):
    """
    Main entry point for macOS universal framework build.

    This function serves as the primary entry point when building
    distributable macOS frameworks with universal binary support.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Note:
        This function calls build_macos() to create the universal framework,
        then archives it and moves artifacts to target/macos/ directory.
        For Xcode project generation, use gen_macos_project() instead.
    """
    print("main tag %s" % tag)

    # Build universal framework
    if not build_macos(target_option, tag, link_type):
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
