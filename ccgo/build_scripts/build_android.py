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
import multiprocessing

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *


# Script configuration
# Get the current working directory (project directory)
SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils.py (reads from CCGO.toml)
PROJECT_RELATIVE_PATH = PROJECT_NAME_LOWER

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

# Android project path (where Gradle builds are located)
ANDROID_PROJECT_PATH = "android/main_android_sdk"

# CMake build command template with Android toolchain configuration
# Parameters: source_path, generator, abi, ndk_root, ndk_root, min_sdk, stl, ccgo_cmake_dir, target_option, jobs
ANDROID_BUILD_CMD = (
    'cmake "%s" %s -DANDROID_ABI="%s" '
    "-DCMAKE_BUILD_TYPE=Release "
    "-DCMAKE_TOOLCHAIN_FILE=%s/build/cmake/android.toolchain.cmake "
    "-DANDROID_TOOLCHAIN=clang "
    "-DANDROID_NDK=%s "
    "-DANDROID_PLATFORM=android-%s "
    f'-DANDROID_STL="%s" '
    '-DCCGO_CMAKE_DIR="%s" %s '
    "&& cmake --build . --config Release -- -j%d"
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


def build_android(incremental, arch, target_option, link_type='both', jobs=None):
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
        link_type: Library link type ('static', 'shared', or 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Symbol libraries (with debug info): obj/local/{static|shared}/{arch}/
        - Release libraries (stripped): libs/{static|shared}/{arch}/

    Note:
        Requires NDK_ROOT environment variable to be set.
        Symbol libraries should be stored permanently for crash symbolication.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()
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

    build_cmd = ANDROID_BUILD_CMD % (
        SCRIPT_PATH,
        ANDROID_GENERATOR,
        arch,
        NDK_ROOT,
        NDK_ROOT,
        get_android_min_sdk_version(SCRIPT_PATH),
        get_android_stl(SCRIPT_PATH),
        CCGO_CMAKE_DIR,
        full_target_option,
        jobs,
    )
    print(f"build cmd: [{build_cmd}]")
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!build fail!!!!!!!!!!!!!!!!!!!!")
        print(f"ERROR: Native build failed for {arch}. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure

    # Determine which link types to process based on link_type parameter
    link_types_to_build = []
    if link_type == 'static' or link_type == 'both':
        link_types_to_build.append('static')
    if link_type == 'shared' or link_type == 'both':
        link_types_to_build.append('shared')

    strip_path = get_android_strip_path(arch)

    # Process each link type
    for current_link_type in link_types_to_build:
        # Setup paths for symbols and release libs
        # New structure:
        # - Shared symbols: obj/local/{arch}/  (not obj/local/shared/{arch}/)
        # - Static symbols: obj/static_local/{arch}/  (only if different from release)
        # - Shared release: libs/shared/{arch}/
        # - Static release: libs/static/{arch}/

        if current_link_type == 'shared':
            # Shared libraries: obj/local/{arch}/ (no 'shared' subdirectory)
            symbol_base = ANDROID_SYMBOL_PATH
            lib_base = ANDROID_LIBS_PATH + "shared/"
        else:  # static
            # Static libraries: obj/static_local/{arch}/ (separate from shared symbols)
            symbol_base = ANDROID_SYMBOL_PATH.replace("obj/local/", "obj/static_local/")
            lib_base = ANDROID_LIBS_PATH + "static/"

        if not os.path.exists(symbol_base):
            os.makedirs(symbol_base)

        symbol_path = symbol_base + arch
        if os.path.exists(symbol_path):
            shutil.rmtree(symbol_path)
        os.mkdir(symbol_path)

        if not os.path.exists(lib_base):
            os.makedirs(lib_base)

        lib_path = lib_base + arch
        if os.path.exists(lib_path):
            shutil.rmtree(lib_path)
        os.mkdir(lib_path)

        # Copy built libraries from cmake output directory
        # Static: cmake_build/Android/static/{arch}/*.a
        # Shared: cmake_build/Android/shared/{arch}/*.so
        cmake_output_dir = f"{ANDROID_LIBS_INSTALL_PATH}{current_link_type}/{arch}/"
        file_extension = "*.a" if current_link_type == 'static' else "*.so"

        # For static libraries, check if symbol version differs from release version
        # If they're identical, we don't need obj/static_local/
        static_symbols_needed = False

        for f in glob.glob(cmake_output_dir + file_extension):
            if is_in_lib_list(f, ANDROID_MERGE_EXCLUDE_LIBS):
                continue
            shutil.copy(f, lib_path)

            # For static libraries, only copy to symbol_path if needed
            if current_link_type == 'static':
                # Static libraries typically don't have stripped versions
                # We'll only keep obj/static_local if files are different
                static_symbols_needed = True
                shutil.copy(f, symbol_path)
            else:
                # For shared libraries, always keep symbols (they'll be stripped later)
                shutil.copy(f, symbol_path)

        # Only copy STL for shared libraries
        if current_link_type == 'shared':
            if not os.path.exists("third_party") or "stdcomm" not in os.listdir("third_party"):
                # copy stl
                shutil.copy(ANDROID_STL_FILE[arch], symbol_path)
                shutil.copy(ANDROID_STL_FILE[arch], lib_path)

            if os.path.exists("third_party"):
                # copy third_party/xxx/lib/android/shared/{arch}/*.so
                for f in os.listdir("third_party"):
                    if f.endswith("comm") and (f not in ANDROID_MERGE_THIRD_PARTY_LIBS):
                        # xxxcomm is not default to merge
                        continue
                    # Try new structure first (with static/shared subdirs)
                    target_dir = f"third_party/{f}/lib/android/shared/{arch}/"
                    if not os.path.exists(target_dir):
                        # Fallback to old structure (for backward compatibility)
                        target_dir = f"third_party/{f}/lib/android/{arch}/"
                    if not os.path.exists(target_dir):
                        continue
                    file_names = glob.glob(target_dir + "*.so")
                    for file_name in file_names:
                        if is_in_lib_list(file_name, ANDROID_MERGE_EXCLUDE_LIBS):
                            continue
                        shutil.copy(file_name, lib_path)

            # Strip shared libraries only
            for f in glob.glob(f"{lib_path}/*.so"):
                strip_cmd = f"{strip_path} {f}"
                print(f"strip cmd: [{strip_cmd}]")
                os.system(strip_cmd)
        else:  # static
            # For static libraries, check if symbol and release versions are identical
            # If they are, we don't need obj/static_local/
            import filecmp
            all_identical = True
            for symbol_file in glob.glob(f"{symbol_path}/*.a"):
                lib_file = os.path.join(lib_path, os.path.basename(symbol_file))
                if not os.path.exists(lib_file) or not filecmp.cmp(symbol_file, lib_file, shallow=False):
                    all_identical = False
                    break

            if all_identical and static_symbols_needed:
                # Files are identical, remove obj/static_local/{arch}
                print(f"Static libraries in obj/static_local/{arch} are identical to libs/static/{arch}, removing symbol directory...")
                shutil.rmtree(symbol_path)
                static_symbols_needed = False

        print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
        print(f"==================[{arch} - {current_link_type}] Output========================")
        print(f"libs(release): {lib_path}")
        if current_link_type == 'shared' or (current_link_type == 'static' and static_symbols_needed):
            print(f"symbols(must store permanently): {symbol_path}")
        else:
            print(f"symbols: not needed (identical to release)")

        # Check the built libraries architecture
        print(f"\n==================Verifying {arch} {current_link_type} Libraries========================")
        lib_pattern = os.path.join(lib_path, "*.so" if current_link_type == 'shared' else "*.a")
        lib_files = glob.glob(lib_pattern)
        if lib_files:
            # Only check first few libraries to avoid too much output
            for lib_file in lib_files[:3]:
                check_library_architecture(lib_file, platform_hint="android")
        else:
            print(f"WARNING: No {current_link_type} libraries found in {lib_path}")
        print(f"===================================================================================")

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)}")
    return True


def archive_android_project(link_type='both', archs=None):
    """
    Archive Android native libraries with unified structure.

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_ANDROID_SDK-{version}.zip
       - lib/static/{arch}/lib{project}.a (if link_type is static or both)
       - lib/shared/{arch}/lib{project}.so (if link_type is shared or both)
       - haars/{project}.aar (if exists)
       - include/{project}/
       - build_info.json
    2. Symbols package: {PROJECT_NAME}_ANDROID_SDK-{version}-SYMBOLS.zip
       - obj/{arch}/*.so (unstripped shared libs)

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        archs: List of architectures built (for metadata)

    Output:
        - target/{PROJECT_NAME}_ANDROID_SDK-{version}.zip
        - target/{PROJECT_NAME}_ANDROID_SDK-{version}-SYMBOLS.zip
    """
    print("==================Archive Android Project========================")

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(SCRIPT_PATH)

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    android_sdk_path = os.path.join(SCRIPT_PATH, ANDROID_PROJECT_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Prepare static libs mapping: lib/static/{arch}/*.a
    static_libs = {}
    if link_type in ('static', 'both'):
        static_libs_base = os.path.join(android_sdk_path, "libs", "static")
        if os.path.exists(static_libs_base):
            for arch in os.listdir(static_libs_base):
                arch_dir = os.path.join(static_libs_base, arch)
                if os.path.isdir(arch_dir):
                    for lib_file in glob.glob(os.path.join(arch_dir, "*.a")):
                        lib_name = os.path.basename(lib_file)
                        arc_path = get_unified_lib_path("static", arch=arch, lib_name=lib_name, platform="android")
                        static_libs[arc_path] = lib_file

    # Prepare shared libs mapping: lib/shared/{arch}/*.so
    shared_libs = {}
    if link_type in ('shared', 'both'):
        shared_libs_base = os.path.join(android_sdk_path, "libs", "shared")
        if os.path.exists(shared_libs_base):
            for arch in os.listdir(shared_libs_base):
                arch_dir = os.path.join(shared_libs_base, arch)
                if os.path.isdir(arch_dir):
                    for lib_file in glob.glob(os.path.join(arch_dir, "*.so")):
                        lib_name = os.path.basename(lib_file)
                        arc_path = get_unified_lib_path("shared", arch=arch, lib_name=lib_name, platform="android")
                        shared_libs[arc_path] = lib_file

    # Prepare AAR files mapping: haars/*.aar
    haars = {}
    bin_android_dir = os.path.join(bin_dir, "android")
    # Check for AAR in target/android/ (from Gradle)
    if os.path.exists(bin_android_dir):
        for aar_file in glob.glob(os.path.join(bin_android_dir, "*.aar")):
            aar_name = os.path.basename(aar_file)
            arc_path = get_unified_haar_path(aar_name)
            haars[arc_path] = aar_file

    # Prepare include directories mapping
    include_dirs = {}
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(PROJECT_NAME_LOWER, headers_src)
        include_dirs[arc_path] = headers_src

    # Prepare symbols (unstripped shared libs from obj/local/)
    obj_files = {}
    if link_type in ('shared', 'both'):
        obj_base = os.path.join(android_sdk_path, "obj", "local")
        if os.path.exists(obj_base):
            for arch in os.listdir(obj_base):
                arch_dir = os.path.join(obj_base, arch)
                if os.path.isdir(arch_dir):
                    for so_file in glob.glob(os.path.join(arch_dir, "*.so")):
                        lib_name = os.path.basename(so_file)
                        arc_path = get_unified_obj_path(arch, lib_name)
                        obj_files[arc_path] = so_file

    # Create unified archive packages
    main_zip_path, symbols_zip_path = create_unified_archive(
        output_dir=bin_dir,
        project_name=PROJECT_NAME,
        platform_name="ANDROID",
        version=full_version,
        link_type=link_type,
        static_libs=static_libs if static_libs else None,
        shared_libs=shared_libs if shared_libs else None,
        include_dirs=include_dirs,
        haars=haars if haars else None,
        obj_files=obj_files if obj_files else None,
        architectures=archs or ["armeabi-v7a", "arm64-v8a", "x86_64"],
    )

    print("\n==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    if symbols_zip_path:
        print(f"Symbols package: {symbols_zip_path}")

    return main_zip_path, symbols_zip_path


def print_build_results(link_type='both'):
    """
    Print Android build results from target/android directory.

    This function displays the build artifacts:
    - Main SDK ZIP packages
    - Symbols ZIP packages (-SYMBOLS.zip)
    - AAR files

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')

    Note:
        Android's unified archive structure uses:
        - Main package: lib/static/, lib/shared/, haars/, include/
        - Symbols package: obj/{arch}/
    """
    print("==================Android Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    bin_android_dir = os.path.join(bin_dir, "android")

    # Check for SDK ZIP packages
    all_zips = glob.glob(f"{bin_dir}/*.zip")
    sdk_zips = [
        f for f in all_zips
        if "ANDROID_SDK" in os.path.basename(f) and not os.path.basename(f).startswith("_temp_")
    ]

    # Also check for AAR files in target/android (from Gradle)
    aar_files = []
    if os.path.exists(bin_android_dir):
        aar_files = glob.glob(f"{bin_android_dir}/*.aar")

    if not sdk_zips and not aar_files:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        # Try to create archive from native build output
        android_sdk_path = os.path.join(SCRIPT_PATH, ANDROID_PROJECT_PATH)
        if os.path.exists(os.path.join(android_sdk_path, "libs")):
            print("Found native libs, creating archive...")
            archive_android_project(link_type)
            # Re-check for zips
            sdk_zips = [
                f for f in glob.glob(f"{bin_dir}/*.zip")
                if "ANDROID_SDK" in os.path.basename(f)
            ]
        else:
            sys.exit(1)

    # Ensure target/android directory exists
    os.makedirs(bin_android_dir, exist_ok=True)

    # Move SDK ZIP files to target/android/
    artifacts_moved = []
    for sdk_zip in sdk_zips:
        dest = os.path.join(bin_android_dir, os.path.basename(sdk_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(sdk_zip, dest)
        artifacts_moved.append(os.path.basename(sdk_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/android/")

    # Copy build_info.json from cmake_build to target/android
    copy_build_info_to_target("android", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/android/:")
    print("-" * 60)

    # List all files in target/android directory with sizes
    if os.path.exists(bin_android_dir):
        for item in sorted(os.listdir(bin_android_dir)):
            item_path = os.path.join(bin_android_dir, item)
            if os.path.isfile(item_path):
                size = os.path.getsize(item_path) / (1024 * 1024)  # MB
                print(f"  {item} ({size:.2f} MB)")

                # Print ZIP/AAR file tree structure (AAR is ZIP format)
                if item.endswith(".zip") or item.endswith(".aar"):
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


def main(incremental, build_archs, target_option="", link_type='both', jobs=None):
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
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

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

    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    print(f"main archs:{build_archs}, link_type:{link_type}, jobs:{jobs}")

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),

        incremental=incremental,
        platform="android",
    )

    has_error = False
    success_archs = []
    for arch in build_archs:
        if not build_android(incremental, arch, target_option, link_type, jobs):
            has_error = True
            break
        success_archs.append(arch)
    print("==================Android Build Done========================")
    print(f"Build All:{build_archs}")
    print(f"Build Success:{success_archs}")
    print(f"Build Failed:{list(set(build_archs) - set(success_archs))}")
    print("==================Output========================")
    if link_type == 'static':
        print(f"libs(release - static): {ANDROID_LIBS_PATH}static/")
        print(f"symbols(static - if needed): {ANDROID_SYMBOL_PATH.replace('obj/local/', 'obj/static_local/')}")
    elif link_type == 'shared':
        print(f"libs(release - shared): {ANDROID_LIBS_PATH}shared/")
        print(f"symbols(must store permanently - shared): {ANDROID_SYMBOL_PATH}")
    elif link_type == 'both':
        print(f"libs(release - static): {ANDROID_LIBS_PATH}static/")
        print(f"libs(release - shared/gradle jniLibs): {ANDROID_LIBS_PATH}shared/")
        print(f"symbols(static - if needed): {ANDROID_SYMBOL_PATH.replace('obj/local/', 'obj/static_local/')}")
        print(f"symbols(must store permanently - shared): {ANDROID_SYMBOL_PATH}")

    # Clean up empty obj/static_local directory if all static symbols were removed
    static_local_dir = ANDROID_SYMBOL_PATH.replace("obj/local/", "obj/static_local/")
    if os.path.exists(static_local_dir) and not os.listdir(static_local_dir):
        shutil.rmtree(static_local_dir)
        print(f"Removed empty directory: {static_local_dir}")

    if has_error:
        raise RuntimeError("Exception occurs when build android")


# Command-line interface for Android builds
# Argument-based interface:
# Default (no args): Print build results from target directory
# --native-only: Build native libraries only (for Gradle flow Step 1)
# --native-only --archive: Build native libraries AND create archive (for Docker builds)
# --archive-only: Create unified archive only (for Gradle flow Step 3, after Gradle buildAAR)
# --arch: Specify architectures (comma-separated)
#
# Usage examples:
# python3 build_android.py                              # Print build results (default)
# python3 build_android.py --native-only                # Build native libs only (Gradle flow Step 1)
# python3 build_android.py --native-only --archive      # Build native libs + archive (Docker)
# python3 build_android.py --archive-only               # Create archive only (Gradle flow Step 3)
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
        help="Only build native libraries (skip Gradle packaging)",
    )
    parser.add_argument(
        "--archive",
        action="store_true",
        help="Create archive after native build (use with --native-only for Docker builds)",
    )
    parser.add_argument(
        "--archive-only",
        action="store_true",
        help="Create unified archive only (for Gradle flow Step 3, after Gradle buildAAR)",
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
    parser.add_argument(
        "-j", "--jobs",
        type=int,
        default=None,
        help="Number of parallel build jobs (default: CPU count)",
    )

    args = parser.parse_args()

    if args.archive_only:
        # Create unified archive only (for Gradle flow Step 3)
        # Native libs were built in Step 1, AAR was built by Gradle in Step 2
        print("==================Android Archive Only Mode==================")
        archs = [arch.strip() for arch in args.arch.split(",")]
        archive_android_project(link_type=args.link_type, archs=archs)
        print_build_results(link_type=args.link_type)
    elif args.native_only:
        # Build native libraries
        archs = [arch.strip() for arch in args.arch.split(",")]
        print(
            f"==================Android Native Build, archs: {archs}, link_type: {args.link_type}, jobs: {args.jobs or 'auto'}=================="
        )
        main(args.incremental, archs, link_type=args.link_type, jobs=args.jobs)

        # Create archive if requested (for Docker builds)
        if args.archive:
            archive_android_project(link_type=args.link_type, archs=archs)
            print_build_results(link_type=args.link_type)
    else:
        # Default: Print build results
        print("==================Android Build Results Mode==================")
        print_build_results(link_type=args.link_type)
