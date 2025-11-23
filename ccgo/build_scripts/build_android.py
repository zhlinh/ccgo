#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_android.py
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
Android native library build script.

This script builds native libraries (.so) for Android platform using CMake
and Android NDK toolchain. It supports multiple ABIs (armeabi-v7a, arm64-v8a,
x86, x86_64) and handles:
- CMake configuration with Android toolchain
- Native library compilation
- Symbol stripping for release builds
- C++ STL library copying
- Third-party library integration
- Output organization (symbol/release libs)

Requirements:
- Android NDK r25c or later (set in NDK_ROOT environment variable)
- CMake 3.10 or later
- Python 3.7+

Usage:
    python3 build_android.py <mode> [arch1] [arch2] ...

    mode: 1 (build), 2 (build and generate project), 3 (incremental build)
    arch: armeabi-v7a, arm64-v8a, x86, x86_64 (default: arm64-v8a)
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
# Get the current working directory (project directory)
SCRIPT_PATH = os.getcwd()
# Directory name as project name
PROJECT_NAME = os.path.basename(SCRIPT_PATH).upper()
PROJECT_NAME_LOWER = PROJECT_NAME.lower()
PROJECT_RELATIVE_PATH = PROJECT_NAME.lower()

# CMake generator selection (Windows requires Unix Makefiles for Android builds)
if system_is_windows():
    ANDROID_GENERATOR = '-G "Unix Makefiles"'
else:
    ANDROID_GENERATOR = ""

# Android NDK root path from environment variable
try:
    NDK_ROOT = os.environ["NDK_ROOT"]
except KeyError as identifier:
    NDK_ROOT = ""

# Build output paths
BUILD_OUT_PATH = "cmake_build/Android"
ANDROID_LIBS_INSTALL_PATH = BUILD_OUT_PATH + "/"

# CMake build command template with Android toolchain configuration
# Parameters: source_path, generator, abi, ndk_root, ndk_root, min_sdk, stl, ccgo_cmake_dir, target_option
ANDROID_BUILD_CMD = (
    'cmake "%s" %s -DANDROID_ABI="%s" '
    "-DCMAKE_BUILD_TYPE=Release "
    "-DCMAKE_TOOLCHAIN_FILE=%s/build/cmake/android.toolchain.cmake "
    "-DANDROID_TOOLCHAIN=clang "
    "-DANDROID_NDK=%s "
    "-DANDROID_PLATFORM=android-%s "
    f'-DANDROID_STL="%s" '
    '-DCCGO_CMAKE_DIR="%s" %s '
    "&& cmake --build . --config Release -- -j8"
)

# Output paths for symbol and release libraries
ANDROID_SYMBOL_PATH = f"{ANDROID_PROJECT_PATH}/obj/local/"
ANDROID_LIBS_PATH = f"{ANDROID_PROJECT_PATH}/libs/"

# llvm-strip tool paths for each ABI (used to strip debug symbols from release builds)
ANDROID_STRIP_FILE = {
    "armeabi-v7a": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/bin/llvm-strip",
    "x86": NDK_ROOT + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/bin/llvm-strip",
    "arm64-v8a": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/bin/llvm-strip",
    "x86_64": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/bin/llvm-strip",
}

# C++ STL shared library paths for each ABI
# Note: NDK r25c location differs from r23 and below
ANDROID_STL_FILE = {
    "armeabi-v7a": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/sysroot/usr/lib/arm-linux-androideabi/libc++_shared.so",
    "x86": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/sysroot/usr/lib/i686-linux-android/libc++_shared.so",
    "arm64-v8a": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/sysroot/usr/lib/aarch64-linux-android/libc++_shared.so",
    "x86_64": NDK_ROOT
    + f"/toolchains/llvm/prebuilt/{get_ndk_host_tag()}/sysroot/usr/lib/x86_64-linux-android/libc++_shared.so",
}


def get_android_strip_path(arch):
    """
    Get the path to llvm-strip tool for a specific Android ABI.

    Args:
        arch: Android ABI name (armeabi-v7a, arm64-v8a, x86, x86_64)

    Returns:
        str: Full path to llvm-strip executable

    Note:
        llvm-strip is used to remove debug symbols from release builds,
        significantly reducing library file size.
    """
    strip_path = ANDROID_STRIP_FILE[arch]
    return strip_path


def build_android(incremental, arch, target_option, tag):
    """
    Build native libraries for a specific Android ABI.

    This function performs the complete build process:
    1. Cleans build directory (unless incremental build)
    2. Configures CMake with Android toolchain
    3. Compiles native libraries
    4. Copies built libraries to symbol/release directories
    5. Copies C++ STL shared library
    6. Copies third-party libraries
    7. Strips debug symbols from release libraries

    Args:
        incremental: If True, skip clean step for faster rebuilds
        arch: Android ABI to build (armeabi-v7a, arm64-v8a, x86, x86_64)
        target_option: Additional CMake target options
        tag: Version tag string for metadata

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Symbol libraries (with debug info): obj/local/{arch}/
        - Release libraries (stripped): libs/{arch}/

    Note:
        Requires NDK_ROOT environment variable to be set.
        Symbol libraries should be stored permanently for crash symbolication.
    """
    before_time = time.time()

    clean(os.path.join(SCRIPT_PATH, BUILD_OUT_PATH), incremental)
    os.chdir(os.path.join(SCRIPT_PATH, BUILD_OUT_PATH))

    build_cmd = ANDROID_BUILD_CMD % (
        SCRIPT_PATH,
        ANDROID_GENERATOR,
        arch,
        NDK_ROOT,
        NDK_ROOT,
        get_android_min_sdk_version(SCRIPT_PATH),
        get_android_stl(SCRIPT_PATH),
        CCGO_CMAKE_DIR,
        target_option,
    )
    print(f"build cmd: [{build_cmd}]")
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!build fail!!!!!!!!!!!!!!!!!!!!")
        return False

    symbol_path = ANDROID_SYMBOL_PATH
    lib_path = ANDROID_LIBS_PATH

    if not os.path.exists(symbol_path):
        os.makedirs(symbol_path)

    symbol_path = symbol_path + arch
    if os.path.exists(symbol_path):
        shutil.rmtree(symbol_path)

    os.mkdir(symbol_path)

    if not os.path.exists(lib_path):
        os.makedirs(lib_path)

    lib_path = lib_path + arch
    if os.path.exists(lib_path):
        shutil.rmtree(lib_path)

    os.mkdir(lib_path)

    for f in glob.glob(ANDROID_LIBS_INSTALL_PATH + "*.so"):
        if is_in_lib_list(f, ANDROID_MERGE_EXCLUDE_LIBS):
            continue
        shutil.copy(f, symbol_path)
        shutil.copy(f, lib_path)

    if not os.path.exists("third_party") or "stdcomm" not in os.listdir("third_party"):
        # copy stl
        shutil.copy(ANDROID_STL_FILE[arch], symbol_path)
        shutil.copy(ANDROID_STL_FILE[arch], lib_path)

    if os.path.exists("third_party"):
        # copy third_party/xxx/lib/android/yyy/*.so
        for f in os.listdir("third_party"):
            if f.endswith("comm") and (f not in ANDROID_MERGE_THIRD_PARTY_LIBS):
                # xxxcomm is not default to merge
                continue
            target_dir = f"third_party/{f}/lib/android/{arch}/"
            if not os.path.exists(target_dir):
                continue
            file_names = glob.glob(target_dir + "*.so")
            for file_name in file_names:
                if is_in_lib_list(file_name, ANDROID_MERGE_EXCLUDE_LIBS):
                    continue
                shutil.copy(file_name, lib_path)

    # strip
    strip_path = get_android_strip_path(arch)
    for f in glob.glob(f"{lib_path}/*.so"):
        strip_cmd = f"{strip_path} {f}"
        print(f"strip cmd: [{strip_cmd}]")
        os.system(strip_cmd)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print(f"==================[{arch}] Output========================")
    print(f"libs(release): {lib_path}")
    print(f"symbols(must store permanently): {symbol_path}")

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)}")
    return True


def print_build_results():
    """
    Print Android build results from bin directory.

    This function displays the build artifacts that were created by Gradle's archiveProject task:
    1. AAR file
    2. ARCHIVE zip (created by Gradle)
    3. Other build artifacts

    Note:
        Android's Gradle archiveProject task already handles the complete
        archiving process, so this function only needs to display the results.
        The archiveProject task creates the ARCHIVE zip with AAR, symbol libraries, etc.
    """
    print("==================Android Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "bin")

    # Check if bin directory exists
    if not os.path.exists(bin_dir):
        print(
            f"ERROR: bin directory not found. Please run './gradlew :archiveProject' first."
        )
        sys.exit(1)

    # Check for build artifacts
    aar_files = glob.glob(f"{bin_dir}/*.aar")
    archive_zips = glob.glob(f"{bin_dir}/(ARCHIVE)*.zip")

    if not aar_files and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure './gradlew :archiveProject' was executed successfully.")
        sys.exit(1)

    # Create bin/android directory for platform-specific artifacts
    bin_android_dir = os.path.join(bin_dir, "android")
    os.makedirs(bin_android_dir, exist_ok=True)

    # Move all .aar and (ARCHIVE)*.zip files to bin/android/
    artifacts_moved = []
    for aar_file in aar_files:
        dest = os.path.join(bin_android_dir, os.path.basename(aar_file))
        shutil.move(aar_file, dest)
        artifacts_moved.append(os.path.basename(aar_file))

    for archive_zip in archive_zips:
        dest = os.path.join(bin_android_dir, os.path.basename(archive_zip))
        shutil.move(archive_zip, dest)
        artifacts_moved.append(os.path.basename(archive_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to bin/android/")

    # Copy build_info.json from cmake_build to bin/android
    copy_build_info_to_bin("android", SCRIPT_PATH)

    print(f"\nBuild artifacts in bin/android/:")
    print("-" * 60)

    # List all files in bin/android directory with sizes
    for item in sorted(os.listdir(bin_android_dir)):
        item_path = os.path.join(bin_android_dir, item)
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


def main(incremental, build_archs, target_option="", tag=""):
    """
    Main entry point for building Android native libraries across multiple ABIs.

    This function orchestrates the complete Android build process:
    1. Validates Android NDK environment
    2. Generates version info header file
    3. Iterates through requested ABIs and builds each
    4. Reports build results (success/failure per ABI)

    Args:
        incremental: If True, skip clean step for faster rebuilds
        build_archs: List of Android ABIs to build (e.g., ['arm64-v8a', 'armeabi-v7a'])
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Raises:
        RuntimeError: If NDK environment check fails or any build fails

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
    if not check_ndk_env():
        raise RuntimeError(
            f"Exception occurs when check ndk env, please install ndk {get_ndk_desc()} and put in env NDK_ROOT"
        )

    print(f"main tag {tag}, archs:{build_archs}")

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        incremental=incremental,
        platform="android",
    )

    has_error = False
    success_archs = []
    for arch in build_archs:
        if not build_android(incremental, arch, target_option, tag):
            has_error = True
            break
        success_archs.append(arch)
    print("==================Android Build Done========================")
    print(f"Build All:{build_archs}")
    print(f"Build Success:{success_archs}")
    print(f"Build Failed:{list(set(build_archs) - set(success_archs))}")
    print("==================Output========================")
    print(f"libs(release): {ANDROID_LIBS_PATH}")
    print(f"symbols(must store permanently): {ANDROID_SYMBOL_PATH}")
    if has_error:
        raise RuntimeError("Exception occurs when build android")


# Command-line interface for Android builds
# New argument-based interface:
# Default (no args): Print build results from bin directory (Gradle archiveProject already created archive)
# --native-only: Build native libraries only
# --arch: Specify architectures (comma-separated)
#
# Usage examples:
# python3 build_android.py                              # Print build results (default)
# python3 build_android.py --native-only                # Build native libs (all archs)
# python3 build_android.py --native-only --arch arm64-v8a,armeabi-v7a
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build Android native libraries and package AAR",
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

    args = parser.parse_args()

    if args.native_only:
        # Build native libraries only
        archs = [arch.strip() for arch in args.arch.split(",")]
        print(
            f"==================Android Native Build, archs: {archs}=================="
        )
        main(args.incremental, archs, tag="native")
    else:
        # Default: Print build results (Gradle archiveProject already handles archiving)
        print("==================Android Build Results Mode==================")
        print_build_results()
