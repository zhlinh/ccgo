#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_watchos.py
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
watchOS native library build script.

This script builds universal static libraries and XCFrameworks for watchOS platform
using CMake and watchOS toolchain. It handles:
- Building for physical devices (Apple Watch: armv7k for Series 3 and earlier, arm64_32 for Series 4+)
- Building for simulators (x86_64 for Intel Macs, arm64 for M1+ Macs)
- Merging static libraries with libtool
- Creating .framework bundles for device and simulator
- Generating XCFramework for unified distribution
- Xcode project generation for development

Requirements:
- Xcode 12.0 or later with command line tools
- CMake 3.10 or later
- Python 3.7+
- macOS development environment

Usage:
    python3 build_watchos.py [mode]

    mode: 1 (build XCFramework), 2 (generate Xcode project), 3 (exit)

Output:
    - XCFramework: cmake_build/watchOS/Darwin.out/{project}.xcframework
    - Frameworks: cmake_build/watchOS/Darwin.out/os|simulator/{project}.framework
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
PROJECT_NAME_LOWER = PROJECT_NAME.lower()
PROJECT_RELATIVE_PATH = PROJECT_NAME.lower()

# Build output paths
BUILD_OUT_PATH = "cmake_build/watchOS"
# Darwin(Linux,Windows).out = ${CMAKE_SYSTEM_NAME}.out
INSTALL_PATH = BUILD_OUT_PATH + "/Darwin.out"

# CMake build command for watchOS Simulator (x86_64 for Intel Macs, arm64 for M1+ simulators)
# Targets watchOS 3.0+, disables ARC and Bitcode, enables symbol visibility
# Parameters: ccgo_cmake_dir, ccgo_cmake_dir, target_option
WATCHOS_BUILD_SIMULATOR_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE="%s/watchos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=SIMULATOR_WATCHOS -DIOS_ARCH="x86_64;arm64" -DIOS_DEPLOYMENT_TARGET=3.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 %s && make -j8 && make install'

# CMake build command for watchOS physical devices (armv7k for Series 3 and earlier, arm64_32 for Series 4+)
# arm64_32 is a special 32-bit ABI running on 64-bit ARM processors for better memory efficiency
# Parameters: ccgo_cmake_dir, ccgo_cmake_dir, target_option
WATCHOS_BUILD_OS_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE="%s/watchos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=WATCHOS -DIOS_ARCH="armv7k;arm64_32" -DIOS_DEPLOYMENT_TARGET=3.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 %s && make -j8 && make install'

# Xcode project generation command (for development/debugging)
# Parameters: ccgo_cmake_dir, ccgo_cmake_dir, target_option
GEN_WATCHOS_OS_PROJ = 'cmake ../.. -G Xcode -DCMAKE_TOOLCHAIN_FILE="%s/watchos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=WATCHOS -DIOS_ARCH="armv7k;arm64_32" -DIOS_DEPLOYMENT_TARGET=3.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 %s'

# All supported watchOS architectures for third-party library integration
THIRD_PARTY_ARCHS = ["x86_64", "arm64", "armv7k", "arm64_32"]


def build_watchos(target_option="", tag="", link_type='static'):
    """
    Build watchOS XCFramework containing both device and simulator frameworks.

    This function performs the complete watchOS build process:
    1. Generates version info header file
    2. Builds static libraries for physical devices (armv7k, arm64_32)
    3. Merges device static libraries using libtool
    4. Builds static libraries for simulators (x86_64, arm64)
    5. Merges simulator static libraries using libtool
    6. Creates .framework bundle for device libraries
    7. Creates .framework bundle for simulator libraries
    8. Generates XCFramework combining both device and simulator frameworks

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'static')

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Device framework: cmake_build/watchOS/Darwin.out/os/{project}.framework
        - Simulator framework: cmake_build/watchOS/Darwin.out/simulator/{project}.framework
        - XCFramework: cmake_build/watchOS/Darwin.out/{project}.xcframework

    Note:
        The XCFramework is the recommended distribution format for watchOS libraries
        as it contains binaries for both devices and simulators in a single bundle.
        This allows Xcode to automatically select the correct binary during builds.
        arm64_32 is a special 32-bit ABI that runs on 64-bit ARM processors for
        better memory efficiency on Apple Watch Series 4 and later.
    """
    before_time = time.time()
    print(f"==================build_watchos (link_type: {link_type})========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="watchos",
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

    build_cmd = WATCHOS_BUILD_OS_CMD % (CCGO_CMAKE_DIR, CCGO_CMAKE_DIR, full_target_option)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build os fail!!!!!!!!!!!!!!!")
        return False

    # target_option is set, then build project
    lipo_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}"
    libtool_os_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}_os"
    libtool_simulator_dst_lib = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}_simulator"
    dst_framework_path = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}.framework"
    dst_framework_headers = WATCHOS_BUILD_COPY_HEADER_FILES
    # add static libs
    total_src_lib = glob.glob(INSTALL_PATH + "/*.a")
    rm_src_lib = []
    libtool_src_lib = [x for x in total_src_lib if x not in rm_src_lib]
    print(f"libtool src lib: {len(libtool_src_lib)}/{len(total_src_lib)}")

    if not libtool_libs(libtool_src_lib, libtool_os_dst_lib):
        return False

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = WATCHOS_BUILD_SIMULATOR_CMD % (
        CCGO_CMAKE_DIR,
        CCGO_CMAKE_DIR,
        full_target_option,
    )
    ret = os.system(build_cmd)

    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build simulator fail!!!!!!!!!!!!!!!")
        return False

    if not libtool_libs(glob.glob(INSTALL_PATH + "/*.a"), libtool_simulator_dst_lib):
        return False

    # os
    lipo_src_libs = []
    lipo_src_libs.append(libtool_os_dst_lib)
    os_lipo_dst_lib = INSTALL_PATH + f"/os/{PROJECT_NAME_LOWER}"
    if not libtool_libs(lipo_src_libs, os_lipo_dst_lib):
        return False
    os_dst_framework_path = INSTALL_PATH + f"/os/{PROJECT_NAME_LOWER}.framework"
    make_static_framework(
        os_lipo_dst_lib, os_dst_framework_path, dst_framework_headers, "./"
    )
    # simulator
    lipo_src_libs = []
    lipo_src_libs.append(libtool_simulator_dst_lib)
    simulator_lipo_dst_lib = INSTALL_PATH + f"/simulator/{PROJECT_NAME_LOWER}"
    if not libtool_libs(lipo_src_libs, simulator_lipo_dst_lib):
        return False
    simulator_dst_framework_path = (
        INSTALL_PATH + f"/simulator/{PROJECT_NAME_LOWER}.framework"
    )
    make_static_framework(
        simulator_lipo_dst_lib,
        simulator_dst_framework_path,
        dst_framework_headers,
        "./",
    )
    # xcframework
    dst_xcframework_path = INSTALL_PATH + f"/{PROJECT_NAME_LOWER}.xcframework"
    if not make_xcframework(
        os_dst_framework_path, simulator_dst_framework_path, dst_xcframework_path
    ):
        return False

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_xcframework_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def archive_watchos_project():
    """
    Archive watchOS XCFramework into ZIP packages.

    This function creates two ZIP packages:
    1. Release ZIP (xcframework only): {PROJECT_NAME}_WATCHOS_SDK-{version}-{suffix}.zip
    2. Archive ZIP (with symbols, headers, etc.): (ARCHIVE)_{PROJECT_NAME}_WATCHOS_SDK-{version}-{suffix}.zip

    Output:
        - target/watchos/{PROJECT_NAME}_WATCHOS_SDK-{version}-{suffix}.zip
          (contains {project_name}.xcframework)
        - target/watchos/(ARCHIVE)_{PROJECT_NAME}_WATCHOS_SDK-{version}-{suffix}.zip
          (contains xcframework, headers, symbols, etc.)
    """
    import zipfile
    from pathlib import Path

    print("==================Archive watchOS Project========================")

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
    watchos_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find xcframework
    xcframework_name = f"{PROJECT_NAME_LOWER}.xcframework"
    xcframework_src = os.path.join(watchos_install_path, xcframework_name)

    if not os.path.exists(xcframework_src):
        print(f"WARNING: XCFramework not found at {xcframework_src}")
        return

    # ========== 1. Create Release ZIP (xcframework only) ==========
    print("\n--- Creating Release ZIP (xcframework only) ---")
    temp_release_dir = os.path.join(bin_dir, f"_temp_release_{project_name_upper}")
    if os.path.exists(temp_release_dir):
        shutil.rmtree(temp_release_dir)
    os.makedirs(temp_release_dir, exist_ok=True)

    # Copy XCFramework to temporary directory
    temp_xcframework = os.path.join(temp_release_dir, xcframework_name)
    shutil.copytree(xcframework_src, temp_xcframework)
    print(f"Copied XCFramework: {xcframework_name}")

    # Create Release ZIP
    release_zip_name = f"{project_name_upper}_WATCHOS_SDK-{full_version}.zip"
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
    archive_name = f"(ARCHIVE)_{project_name_upper}_WATCHOS_SDK-{full_version}"
    archive_dir = os.path.join(bin_dir, archive_name)

    if os.path.exists(archive_dir):
        shutil.rmtree(archive_dir)
    os.makedirs(archive_dir, exist_ok=True)

    # Copy XCFramework to archive
    archive_xcframework = os.path.join(archive_dir, xcframework_name)
    shutil.copytree(xcframework_src, archive_xcframework)
    print(f"Copied XCFramework to archive: {xcframework_name}")

    # Copy header files if they exist
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        headers_dest = os.path.join(archive_dir, "include")
        shutil.copytree(headers_src, headers_dest)
        print(f"Copied headers: include/")

    # Copy symbol files if they exist (dSYM for watchOS)
    dsym_pattern = f"{watchos_install_path}/*.dSYM"
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
    Print watchOS build results from target directory.

    This function displays the build artifacts and moves them to target/watchos/:
    - Release SDK ZIP packages (xcframework only)
    - Archive SDK ZIP packages (with symbols, headers, etc.)
    """
    print("==================watchOS Build Results========================")

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
        if "WATCHOS_SDK" in os.path.basename(f) and not os.path.basename(f).startswith("_temp_")
    ]

    if not sdk_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Create target/watchos directory for platform-specific artifacts
    bin_watchos_dir = os.path.join(bin_dir, "watchos")
    os.makedirs(bin_watchos_dir, exist_ok=True)

    # Clean up old xcframework directories in target/watchos/
    for item in os.listdir(bin_watchos_dir):
        item_path = os.path.join(bin_watchos_dir, item)
        if os.path.isdir(item_path) and item.endswith('.xcframework'):
            shutil.rmtree(item_path)
            print(f"Cleaned up old xcframework: {item}")

    # Move SDK ZIP files to target/watchos/
    artifacts_moved = []
    for sdk_zip in sdk_zips:
        dest = os.path.join(bin_watchos_dir, os.path.basename(sdk_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(sdk_zip, dest)
        artifacts_moved.append(os.path.basename(sdk_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/watchos/")

    # Copy build_info.json from cmake_build to target/watchos
    copy_build_info_to_target("watchos", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/watchos/:")
    print("-" * 60)

    # List all files in target/watchos directory with sizes
    for item in sorted(os.listdir(bin_watchos_dir)):
        item_path = os.path.join(bin_watchos_dir, item)
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


def gen_watchos_project(target_option="", tag=""):
    """
    Generate Xcode project for watchOS development and debugging.

    This function creates an Xcode project (.xcodeproj) that can be opened in Xcode
    for interactive development, debugging, and testing. Unlike build_watchos() which
    creates distributable frameworks, this generates IDE project files.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - Xcode project: cmake_build/watchOS/{project}.xcodeproj

    Note:
        The generated Xcode project is configured for watchOS device builds.
        To build for simulator, you can switch the scheme in Xcode.
        This is useful for development workflows where you need Xcode's
        debugging tools, code completion, and build system integration.
    """
    print("==================gen_watchos_project========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="watchos",
    )

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    cmd = GEN_WATCHOS_OS_PROJ % (CCGO_CMAKE_DIR, CCGO_CMAKE_DIR, target_option)
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!gen fail!!!!!!!!!!!!!!!")
        return False

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {SCRIPT_PATH}/{BUILD_OUT_PATH}")

    return True


def main(target_option="", tag="", link_type='static'):
    """
    Main entry point for watchOS XCFramework build.

    This function serves as the primary entry point when building
    distributable watchOS frameworks and XCFrameworks.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'static')

    Note:
        This function calls build_watchos() to create the XCFramework,
        then archives it and moves artifacts to target/watchos/ directory.
        For Xcode project generation, use gen_watchos_project() instead.
    """
    print(f"main tag {tag}, link_type: {link_type}")

    # Build XCFramework
    if not build_watchos(target_option, tag, link_type):
        print("ERROR: watchOS build failed")
        sys.exit(1)

    # Archive and organize artifacts
    archive_watchos_project()
    print_build_results()


# Command-line interface for watchOS builds
# Supports two invocation modes:
# 1. Interactive mode (no args): Prompts user for build mode
# 2. Mode only (1 arg): Uses specified mode directly
#
# Build modes:
# 1 - Build XCFramework: Creates distributable framework with device + simulator binaries
# 2 - Generate Xcode project: Creates .xcodeproj for development/debugging in Xcode
# 3 - Exit: Quit without building
if __name__ == "__main__":
    PROJECT_NAME_LOWER = PROJECT_NAME.lower()
    while True:
        if len(sys.argv) >= 2:
            num = sys.argv[1]
        else:
            archs = set(["armeabi-v7a"])
            num = str(
                input(
                    "Enter menu:"
                    + f"\n1. Clean && build watchOS {PROJECT_NAME_LOWER}."
                    + f"\n2. Gen watchOS {PROJECT_NAME_LOWER} Project."
                    + f"\n3. Exit."
                )
            )
        print(f"==================watchOS Choose num: {num}==================")
        if num == "1":
            main(tag=num)
            break
        elif num == "2":
            gen_watchos_project()
            break
        else:
            break
