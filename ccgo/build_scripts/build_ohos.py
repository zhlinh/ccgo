#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_ohos.py
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
OpenHarmony (OHOS) native library build script.

This script builds native libraries (.so) for OpenHarmony OS platform using CMake
and OHOS Native SDK toolchain. OpenHarmony is Huawei's open-source distributed OS.
The script supports multiple ABIs (armeabi-v7a, arm64-v8a, x86_64) and handles:
- CMake configuration with OHOS toolchain
- Native library compilation for OpenHarmony
- Symbol stripping for release builds
- C++ STL library copying
- Third-party library integration
- Output organization (symbol/release libs)

Requirements:
- OHOS Native SDK (set in OHOS_SDK_HOME or HOS_SDK_HOME environment variable)
- CMake 3.10 or later
- Python 3.7+

Usage:
    python3 build_ohos.py <mode> [arch1] [arch2] ...

    mode: 1 (build), 2 (incremental build), 3 (test build), 4 (exit)
    arch: armeabi-v7a, arm64-v8a, x86_64 (default: armeabi-v7a, arm64-v8a, x86_64)
"""

import glob
import os
import platform
import shutil
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
# Use the uppercase of the current directory name as the project name
PROJECT_NAME = os.path.basename(SCRIPT_PATH).upper()

# Ensure cmake directory exists in project
PROJECT_NAME_LOWER = PROJECT_NAME.lower()
PROJECT_RELATIVE_PATH = PROJECT_NAME.lower()

# CMake generator selection (Windows requires Unix Makefiles for OHOS builds)
if system_is_windows():
    OHOS_GENERATOR = '-G "Unix Makefiles"'
else:
    OHOS_GENERATOR = ""

# OHOS SDK root path from environment variable
# Supports both OHOS_SDK_HOME and HOS_SDK_HOME
try:
    OHOS_SDK_ROOT = os.environ["OHOS_SDK_HOME"] or os.environ["HOS_SDK_HOME"]
except KeyError as identifier:
    OHOS_SDK_ROOT = ""

# Build output paths
BUILD_OUT_PATH = "cmake_build/OHOS"
OHOS_LIBS_INSTALL_PATH = BUILD_OUT_PATH + "/"

# OHOS project path (where Hvigor builds are located)
OHOS_PROJECT_PATH = "ohos/main_ohos_sdk"

# CMake build command template with OHOS toolchain configuration
# Parameters: source_path, generator, arch, sdk_root (4x), min_sdk, stl, ccgo_cmake_dir, target_option
OHOS_BUILD_CMD = (
    'cmake "%s" %s -DOHOS_ARCH="%s" '
    "-DOHOS=1 "
    "-D__OHOS__=1 "
    "-DCMAKE_BUILD_TYPE=Release "
    "-DOHOS_PLATFORM=OHOS "
    "-DCMAKE_TOOLCHAIN_FILE=%s/native/build/cmake/ohos.toolchain.cmake "
    "-DOHOS_TOOLCHAIN=clang "
    "-DOHOS_SDK_NATIVE=%s/native/ "
    "-DOHOS_SDK_NATIVE_PLATFORM=ohos-%s "
    '-DOHOS_STL="%s" '
    '-DCCGO_CMAKE_DIR="%s" %s '
    "&& cmake --build . --config Release -- -j8"
)

# Output paths for symbol and release libraries
OHOS_SYMBOL_PATH = f"{OHOS_PROJECT_PATH}/obj/local/"
OHOS_LIBS_PATH = f"{OHOS_PROJECT_PATH}/libs/"

# llvm-strip tool path (used to strip debug symbols from release builds)
# Unlike Android NDK which has per-arch strip tools, OHOS uses a single universal llvm-strip
OHOS_STRIP_FILE = OHOS_SDK_ROOT + f"/native/llvm/bin/llvm-strip"

# C++ STL shared library paths for each ABI
# OHOS uses libc++_shared.so located in architecture-specific directories
OHOS_STL_FILE = {
    "armeabi-v7a": OHOS_SDK_ROOT + f"/native/llvm/lib/arm-linux-ohos/libc++_shared.so",
    "arm64-v8a": OHOS_SDK_ROOT
    + f"/native/llvm/lib/aarch64-linux-ohos/libc++_shared.so",
    "x86_64": OHOS_SDK_ROOT + f"/native/llvm/lib/x86_64-linux-ohos/libc++_shared.so",
}


def get_ohos_strip_path(arch):
    """
    Get the path to llvm-strip tool for OHOS builds.

    Args:
        arch: OHOS ABI name (armeabi-v7a, arm64-v8a, x86_64)
              Note: Unlike Android, OHOS uses a single universal llvm-strip
              for all architectures, so this parameter is currently unused.

    Returns:
        str: Full path to llvm-strip executable

    Note:
        llvm-strip is used to remove debug symbols from release builds,
        significantly reducing library file size.
    """
    strip_path = OHOS_STRIP_FILE
    return strip_path


def build_ohos(incremental, arch, target_option, tag, link_type='both'):
    """
    Build native libraries for a specific OHOS ABI.

    This function performs the complete build process for OpenHarmony OS:
    1. Cleans build directory (unless incremental build)
    2. Configures CMake with OHOS toolchain
    3. Compiles native libraries
    4. Copies built libraries to symbol/release directories
    5. Copies C++ STL shared library
    6. Copies third-party libraries
    7. Strips debug symbols from release libraries

    Args:
        incremental: If True, skip clean step for faster rebuilds
        arch: OHOS ABI to build (armeabi-v7a, arm64-v8a, x86_64)
        target_option: Additional CMake target options
        tag: Version tag string for metadata
        link_type: Library link type ('static', 'shared', or 'both')

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Shared symbol libraries (with debug info): obj/local/{arch}/
        - Static symbol libraries (if needed): obj/static_local/{arch}/
        - Shared release libraries (stripped): libs/shared/{arch}/
        - Static release libraries: libs/static/{arch}/

    Note:
        Requires OHOS_SDK_HOME or HOS_SDK_HOME environment variable to be set.
        Symbol libraries should be stored permanently for crash symbolication.
    """
    before_time = time.time()

    clean(os.path.join(SCRIPT_PATH, BUILD_OUT_PATH), incremental)
    os.chdir(os.path.join(SCRIPT_PATH, BUILD_OUT_PATH))

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

    build_cmd = OHOS_BUILD_CMD % (
        SCRIPT_PATH,
        OHOS_GENERATOR,
        arch,
        OHOS_SDK_ROOT,
        OHOS_SDK_ROOT,
        get_ohos_min_sdk_version(SCRIPT_PATH),
        get_ohos_stl(SCRIPT_PATH),
        CCGO_CMAKE_DIR,
        full_target_option,
    )
    print(f"build cmd: [{build_cmd}]")
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!build fail!!!!!!!!!!!!!!!!!!!!")
        return False

    # Determine which link types to process based on link_type parameter
    link_types_to_build = []
    if link_type == 'static' or link_type == 'both':
        link_types_to_build.append('static')
    if link_type == 'shared' or link_type == 'both':
        link_types_to_build.append('shared')

    strip_path = get_ohos_strip_path(arch)

    # Process each link type
    for current_link_type in link_types_to_build:
        # Setup paths for symbols and release libs
        # OHOS structure (different from Android to maintain HAR compatibility):
        # - Shared symbols: obj/local/{arch}/
        # - Static symbols: obj/static_local/{arch}/ (only if different from release)
        # - Shared release: libs/{arch}/ (for HAR packaging - Hvigor expects this structure)
        # - Static release: Not placed in libs/ (HAR doesn't need static libs)

        if current_link_type == 'shared':
            # Shared libraries: obj/local/{arch}/ for symbols
            # libs/{arch}/ for release (HAR packaging needs this structure)
            symbol_base = OHOS_SYMBOL_PATH
            lib_base = OHOS_LIBS_PATH  # libs/{arch}/ - no 'shared' subdirectory for OHOS
        else:  # static
            # Static libraries: obj/static_local/{arch}/ for symbols
            # Static release libs are not needed for HAR, so we don't copy them to libs/
            symbol_base = OHOS_SYMBOL_PATH.replace("obj/local/", "obj/static_local/")
            lib_base = None  # Don't create libs/ output for static

        if not os.path.exists(symbol_base):
            os.makedirs(symbol_base)

        symbol_path = symbol_base + arch
        if os.path.exists(symbol_path):
            shutil.rmtree(symbol_path)
        os.mkdir(symbol_path)

        # Only create lib_path for shared libraries (static libs don't go to libs/)
        lib_path = None
        if lib_base is not None:
            if not os.path.exists(lib_base):
                os.makedirs(lib_base)

            lib_path = lib_base + arch
            if os.path.exists(lib_path):
                shutil.rmtree(lib_path)
            os.mkdir(lib_path)

        # Copy built libraries from cmake output directory
        # Static: cmake_build/OHOS/static/{arch}/*.a
        # Shared: cmake_build/OHOS/shared/{arch}/*.so
        cmake_output_dir = f"{OHOS_LIBS_INSTALL_PATH}{current_link_type}/{arch}/"
        file_extension = "*.a" if current_link_type == 'static' else "*.so"

        # For static libraries, check if symbol version differs from release version
        # If they're identical, we don't need obj/static_local/
        static_symbols_needed = False

        for f in glob.glob(cmake_output_dir + file_extension):
            if is_in_lib_list(f, OHOS_MERGE_EXCLUDE_LIBS):
                continue

            # Copy to lib_path only if it exists (shared libs only)
            if lib_path is not None:
                shutil.copy(f, lib_path)

            # For static libraries, only copy to symbol_path
            if current_link_type == 'static':
                # Static libraries typically don't have stripped versions
                # We only keep symbols in obj/static_local/
                static_symbols_needed = True
                shutil.copy(f, symbol_path)
            else:
                # For shared libraries, always keep symbols (they'll be stripped later)
                shutil.copy(f, symbol_path)

        # Only copy STL for shared libraries
        if current_link_type == 'shared':
            if not os.path.exists("third_party") or "stdcomm" not in os.listdir("third_party"):
                # copy stl
                shutil.copy(OHOS_STL_FILE[arch], symbol_path)
                shutil.copy(OHOS_STL_FILE[arch], lib_path)

            if os.path.exists("third_party"):
                # copy third_party/xxx/lib/ohos/yyy/*.so
                for f in os.listdir("third_party"):
                    if f.endswith("comm") and (f not in OHOS_MERGE_THIRD_PARTY_LIBS):
                        # xxxcomm is not default to merge
                        continue
                    target_dir = f"third_party/{f}/lib/ohos/{arch}/"
                    if not os.path.exists(target_dir):
                        continue
                    file_names = glob.glob(target_dir + "*.so")
                    for file_name in file_names:
                        if is_in_lib_list(file_name, OHOS_MERGE_EXCLUDE_LIBS):
                            continue
                        shutil.copy(file_name, lib_path)

            # Strip shared libraries only
            for f in glob.glob(f"{lib_path}/*.so"):
                strip_cmd = f"{strip_path} {f}"
                print(f"strip cmd: [{strip_cmd}]")
                os.system(strip_cmd)

        print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
        print(f"==================[{arch} - {current_link_type}] Output========================")
        if lib_path is not None:
            print(f"libs(release): {lib_path}")
        if current_link_type == 'shared':
            print(f"symbols(must store permanently): {symbol_path}")
        elif static_symbols_needed:
            print(f"symbols(static): {symbol_path}")

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)}")
    return True


def print_build_results():
    """
    Print OHOS build results from target/ohos directory.

    This function displays the build artifacts in target/ohos/:
    1. HAR file
    2. ARCHIVE zip (created by hvigor archiveProject task)
    3. build_info.json

    Note:
        The hvigor archive-plugin.ts directly outputs to target/ohos/,
        so this function only needs to display the results.
    """
    print("==================OHOS Build Results========================")

    # Define paths - artifacts are directly in target/ohos/
    target_ohos_dir = os.path.join(SCRIPT_PATH, "target", "ohos")

    # Check if target/ohos directory exists
    if not os.path.exists(target_ohos_dir):
        print(f"ERROR: target/ohos directory not found.")
        print("Please ensure hvigor archiveProject was executed successfully.")
        sys.exit(1)

    # Check for build artifacts in target/ohos/
    har_files = glob.glob(f"{target_ohos_dir}/*OHOS_SDK*.har")
    archive_zips = glob.glob(f"{target_ohos_dir}/(ARCHIVE)*.zip")
    build_info = os.path.join(target_ohos_dir, "build_info.json")

    if not har_files and not archive_zips:
        print(f"ERROR: No build artifacts found in {target_ohos_dir}")
        print("Please ensure hvigor archiveProject was executed successfully.")
        sys.exit(1)

    print(f"\nBuild artifacts in target/ohos/:")
    print("-" * 60)

    # List all files in target/ohos directory with sizes
    for item in sorted(os.listdir(target_ohos_dir)):
        item_path = os.path.join(target_ohos_dir, item)
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


def archive_ohos_project():
    """
    Archive OHOS HAR and related build artifacts.

    This function creates an archive package containing:
    1. HAR file (copied to target/ohos/)
    2. Symbol libraries with debug info (obj/local/**/*.so)
    3. ArkTS/ets source files
    4. Mapping files if available

    The archive is packaged into a ZIP file named:
    (ARCHIVE)_{PROJECT_NAME}_OHOS_SDK-{version}-{suffix}.zip

    Output:
        - target/ohos/{PROJECT_NAME}_OHOS_SDK-{version}-{suffix}.har
        - target/ohos/(ARCHIVE)_{PROJECT_NAME}_OHOS_SDK-{version}-{suffix}.zip
    """
    import zipfile
    from pathlib import Path

    print("==================Archive OHOS Project========================")

    # Get project version info
    version_name = get_version_name(SCRIPT_PATH)
    project_name_upper = PROJECT_NAME.upper()

    # Try to get publish suffix from git tags or use beta.0 as default
    try:
        git_tags = os.popen("git describe --tags --abbrev=0 2>/dev/null").read().strip()
        if git_tags and "-" in git_tags:
            # Extract suffix from tag like v1.0.0-beta.0
            suffix = git_tags.split("-", 1)[1]
        else:
            # Check if this is a release build or beta
            git_branch = os.popen("git rev-parse --abbrev-ref HEAD 2>/dev/null").read().strip()
            if git_branch == "master" or git_branch == "main":
                suffix = "release"
            else:
                suffix = "beta.0"
    except:
        suffix = "beta.0"

    # Build full version name with suffix
    full_version = f"{version_name}-{suffix}" if suffix else version_name

    # Define paths - use bin/ for temporary storage, will be moved to target/ohos/ later
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    ohos_main_sdk = os.path.join(SCRIPT_PATH, "ohos", "main_ohos_sdk")

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find and copy HAR file
    har_search_path = os.path.join(ohos_main_sdk, "build", "default", "outputs", "default")
    har_files = glob.glob(f"{har_search_path}/*.har")

    if not har_files:
        print(f"WARNING: No HAR file found in {har_search_path}")
    else:
        har_file = har_files[0]
        har_dest = os.path.join(bin_dir, f"{project_name_upper}_OHOS_SDK-{full_version}.har")
        shutil.copy(har_file, har_dest)
        print(f"Copied HAR: {har_dest}")

    # Create archive directory structure
    archive_name = f"(ARCHIVE)_{project_name_upper}_OHOS_SDK-{full_version}"
    archive_dir = os.path.join(bin_dir, archive_name)
    os.makedirs(archive_dir, exist_ok=True)

    # Copy symbol libraries (obj/local with debug info)
    obj_local_src = os.path.join(ohos_main_sdk, "obj", "local")
    if os.path.exists(obj_local_src):
        obj_local_dst = os.path.join(archive_dir, "obj", "local")
        if os.path.exists(obj_local_dst):
            shutil.rmtree(obj_local_dst)
        shutil.copytree(obj_local_src, obj_local_dst)
        print(f"Copied symbol libraries: {obj_local_dst}")

    # Copy libs (if needed for reference)
    libs_src = os.path.join(ohos_main_sdk, "libs")
    if os.path.exists(libs_src):
        libs_dst = os.path.join(archive_dir, "libs")
        if os.path.exists(libs_dst):
            shutil.rmtree(libs_dst)
        shutil.copytree(libs_src, libs_dst)
        print(f"Copied libs: {libs_dst}")

    # Copy source files (ArkTS/ets)
    src_main = os.path.join(ohos_main_sdk, "src", "main")
    if os.path.exists(src_main):
        # Copy ets files
        ets_src = os.path.join(src_main, "ets")
        if os.path.exists(ets_src):
            ets_dst = os.path.join(archive_dir, "ets")
            if os.path.exists(ets_dst):
                shutil.rmtree(ets_dst)
            shutil.copytree(ets_src, ets_dst)
            print(f"Copied ets source: {ets_dst}")

    # Create ZIP archive
    zip_file_path = os.path.join(bin_dir, f"{archive_name}.zip")
    with zipfile.ZipFile(zip_file_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(archive_dir):
            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, bin_dir)
                zipf.write(file_path, arcname)

    # Remove temporary archive directory
    shutil.rmtree(archive_dir)

    print("==================Archive Complete (temporary location)========================")
    print(f"HAR file: {bin_dir}/{project_name_upper}_OHOS_SDK-{full_version}.har")
    print(f"Archive ZIP: {zip_file_path}")
    print("\nMoving artifacts to platform-specific directory...")

    # Move artifacts to target/ohos/ and display final results
    print_build_results()


def main(incremental, build_archs, target_option="", tag="", link_type='both'):
    """
    Main entry point for building OHOS native libraries across multiple ABIs.

    This function orchestrates the complete OHOS build process:
    1. Validates OHOS Native SDK environment
    2. Generates version info header file
    3. Iterates through requested ABIs and builds each
    4. Reports build results (success/failure per ABI)

    Args:
        incremental: If True, skip clean step for faster rebuilds
        build_archs: List of OHOS ABIs to build (e.g., ['arm64-v8a', 'armeabi-v7a'])
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')

    Raises:
        RuntimeError: If OHOS SDK environment check fails or any build fails

    Output:
        Prints build summary including:
        - All requested architectures
        - Successfully built architectures
        - Failed architectures (if any)
        - Output paths for symbol and release libraries

    Note:
        Symbol libraries contain debug information for crash symbolication
        and should be stored permanently for production releases.
    """
    if not check_ohos_native_env():
        raise RuntimeError(
            f"Exception occurs when check ohos native env, please install ndk {get_ohos_native_desc()} and put in env OHOS_SDK_HOME"
        )

    print(f"main tag {tag}, archs [{build_archs}], link_type:{link_type}")

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        incremental=incremental,
        platform="ohos",
    )

    # Clean up old directory structures from previous versions
    # 1. Old obj/local/shared and obj/local/static (before restructuring)
    old_shared_dir = os.path.join(OHOS_SYMBOL_PATH, "shared")
    old_static_dir = os.path.join(OHOS_SYMBOL_PATH, "static")
    if os.path.exists(old_shared_dir):
        shutil.rmtree(old_shared_dir)
        print(f"Cleaned up old directory structure: {old_shared_dir}")
    if os.path.exists(old_static_dir):
        shutil.rmtree(old_static_dir)
        print(f"Cleaned up old directory structure: {old_static_dir}")

    # 2. Old libs/shared and libs/static (from Android-style structure)
    old_libs_shared = os.path.join(OHOS_LIBS_PATH, "shared")
    old_libs_static = os.path.join(OHOS_LIBS_PATH, "static")
    if os.path.exists(old_libs_shared):
        shutil.rmtree(old_libs_shared)
        print(f"Cleaned up old directory structure: {old_libs_shared}")
    if os.path.exists(old_libs_static):
        shutil.rmtree(old_libs_static)
        print(f"Cleaned up old directory structure: {old_libs_static}")

    has_error = False
    success_archs = []
    for arch in build_archs:
        if not build_ohos(incremental, arch, target_option, tag, link_type):
            has_error = True
            break
        success_archs.append(arch)
    print("==================OHOS Build Done========================")
    print(f"Build All:{build_archs}")
    print(f"Build Success:{success_archs}")
    print(f"Build Failed:{list(set(build_archs) - set(success_archs))}")
    print("==================Output========================")
    if link_type == 'static':
        print(f"symbols(static): {OHOS_SYMBOL_PATH.replace('obj/local/', 'obj/static_local/')}")
        print(f"Note: Static libs are not placed in libs/ (HAR doesn't need them)")
    elif link_type == 'shared':
        print(f"libs(release - shared): {OHOS_LIBS_PATH} (for HAR packaging)")
        print(f"symbols(must store permanently): {OHOS_SYMBOL_PATH}")
    elif link_type == 'both':
        print(f"libs(release - shared): {OHOS_LIBS_PATH} (for HAR packaging)")
        print(f"symbols(shared - must store permanently): {OHOS_SYMBOL_PATH}")
        print(f"symbols(static): {OHOS_SYMBOL_PATH.replace('obj/local/', 'obj/static_local/')}")
        print(f"Note: Static libs are only kept as symbols in obj/static_local/")

    if has_error:
        raise RuntimeError("Exception occurs when build ohos")


# Command-line interface for OHOS builds
# New argument-based interface:
# Default (no args): Print build results from target directory (hvigor already created HAR)
# --native-only: Build native libraries only
# --arch: Specify architectures (comma-separated)
#
# Usage examples:
# python3 build_ohos.py                              # Print build results (default)
# python3 build_ohos.py --native-only                # Build native libs (all archs)
# python3 build_ohos.py --native-only --arch arm64-v8a,armeabi-v7a
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build OHOS native libraries and package HAR",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--native-only",
        action="store_true",
        help="Only build native libraries (skip archive)",
    )
    parser.add_argument(
        "--arch",
        type=str,
        default="armeabi-v7a,arm64-v8a,x86_64",
        help="Architectures to build (comma-separated, default: armeabi-v7a,arm64-v8a,x86_64)",
    )
    parser.add_argument(
        "--incremental",
        action="store_true",
        help="Incremental build (skip clean step)",
    )
    parser.add_argument(
        "--link-type",
        type=str,
        choices=['static', 'shared', 'both'],
        default='both',
        help="Library link type (default: both)",
    )

    args = parser.parse_args()

    if args.native_only:
        # Build native libraries only
        archs = [arch.strip() for arch in args.arch.split(",")]
        print(f"==================OHOS Native Build, archs: {archs}, link_type: {args.link_type}==================")
        main(args.incremental, archs, tag="native", link_type=args.link_type)
    else:
        # Default: Print build results (hvigor assembleHar already handles building)
        print("==================OHOS Build Results Mode==================")
        print_build_results()
