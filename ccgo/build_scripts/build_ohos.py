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
import multiprocessing

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

# Script configuration
SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils.py (reads from CCGO.toml)
PROJECT_RELATIVE_PATH = PROJECT_NAME_LOWER

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
# Parameters: source_path, generator, arch, sdk_root (4x), min_sdk, stl, ccgo_cmake_dir, target_option, jobs
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
    "&& cmake --build . --config Release -- -j%d"
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


def build_ohos(incremental, arch, target_option, link_type='both', jobs=None):
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
        jobs: Number of parallel build jobs (default: CPU count)

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
        jobs,
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
        # OHOS structure (same as Android):
        # - Shared symbols: obj/local/{arch}/
        # - Static symbols: obj/static_local/{arch}/
        # - Shared release: libs/shared/{arch}/
        # - Static release: libs/static/{arch}/

        if current_link_type == 'shared':
            # Shared libraries: obj/local/{arch}/ for symbols
            # libs/shared/{arch}/ for release
            symbol_base = OHOS_SYMBOL_PATH
            lib_base = OHOS_LIBS_PATH + "shared/"
        else:  # static
            # Static libraries: obj/static_local/{arch}/ for symbols
            # libs/static/{arch}/ for release
            symbol_base = OHOS_SYMBOL_PATH.replace("obj/local/", "obj/static_local/")
            lib_base = OHOS_LIBS_PATH + "static/"

        if not os.path.exists(symbol_base):
            os.makedirs(symbol_base)

        symbol_path = symbol_base + arch
        if os.path.exists(symbol_path):
            shutil.rmtree(symbol_path)
        os.mkdir(symbol_path)

        # Create lib_path for both shared and static libraries
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

            # Copy to lib_path for both shared and static libraries
            shutil.copy(f, lib_path)

            # For static libraries, also copy to symbol_path
            if current_link_type == 'static':
                # Static libraries typically don't have stripped versions
                # We keep symbols in obj/static_local/
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
        print(f"libs(release): {lib_path}")
        if current_link_type == 'shared':
            print(f"symbols(must store permanently): {symbol_path}")
        elif static_symbols_needed:
            print(f"symbols(static): {symbol_path}")

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)}")
    return True


def print_build_results(link_type='both'):
    """
    Print OHOS build results from target/ohos directory.

    This function displays the build artifacts:
    - Main SDK ZIP packages
    - Symbols ZIP packages (-SYMBOLS.zip)
    - HAR files

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')

    Note:
        OHOS's unified archive structure uses:
        - Main package: lib/static/, lib/shared/, haars/, include/
        - Symbols package: obj/{arch}/
    """
    print("==================OHOS Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    bin_ohos_dir = os.path.join(bin_dir, "ohos")

    # Check for SDK ZIP packages
    all_zips = glob.glob(f"{bin_dir}/*.zip")
    sdk_zips = [
        f for f in all_zips
        if "OHOS_SDK" in os.path.basename(f) and not os.path.basename(f).startswith("_temp_")
    ]

    # Also check for HAR files in ohos project output
    har_files = []
    ohos_sdk_path = os.path.join(SCRIPT_PATH, OHOS_PROJECT_PATH)
    har_search_path = os.path.join(ohos_sdk_path, "build", "default", "outputs", "default")
    har_files = glob.glob(f"{har_search_path}/*.har")

    if not sdk_zips and not har_files:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        # Try to create archive from native build output
        if os.path.exists(os.path.join(ohos_sdk_path, "libs")):
            print("Found native libs, creating archive...")
            archive_ohos_project(link_type)
            # Re-check for zips
            sdk_zips = [
                f for f in glob.glob(f"{bin_dir}/*.zip")
                if "OHOS_SDK" in os.path.basename(f)
            ]
        else:
            sys.exit(1)

    # Ensure target/ohos directory exists
    os.makedirs(bin_ohos_dir, exist_ok=True)

    # Move SDK ZIP files to target/ohos/
    artifacts_moved = []
    for sdk_zip in sdk_zips:
        dest = os.path.join(bin_ohos_dir, os.path.basename(sdk_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(sdk_zip, dest)
        artifacts_moved.append(os.path.basename(sdk_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/ohos/")

    # Copy build_info.json from cmake_build to target/ohos
    copy_build_info_to_target("ohos", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/ohos/:")
    print("-" * 60)

    # List all files in target/ohos directory with sizes
    if os.path.exists(bin_ohos_dir):
        for item in sorted(os.listdir(bin_ohos_dir)):
            item_path = os.path.join(bin_ohos_dir, item)
            if os.path.isfile(item_path):
                size = os.path.getsize(item_path) / (1024 * 1024)  # MB
                print(f"  {item} ({size:.2f} MB)")

                # Print ZIP/HAR file tree structure (HAR is ZIP format)
                if item.endswith(".zip") or item.endswith(".har"):
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


def archive_ohos_project(link_type='both', archs=None):
    """
    Archive OHOS native libraries with unified structure.

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_OHOS_SDK-{version}.zip
       - lib/static/{arch}/lib{project}.a (if link_type is static or both)
       - lib/shared/{arch}/lib{project}.so (if link_type is shared or both)
       - haars/{project}.har (if exists)
       - include/{project}/
       - build_info.json
    2. Symbols package: {PROJECT_NAME}_OHOS_SDK-{version}-SYMBOLS.zip
       - obj/{arch}/*.so (unstripped shared libs)

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        archs: List of architectures built (for metadata)

    Output:
        - target/{PROJECT_NAME}_OHOS_SDK-{version}.zip
        - target/{PROJECT_NAME}_OHOS_SDK-{version}-SYMBOLS.zip
    """
    print("==================Archive OHOS Project========================")

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(SCRIPT_PATH)

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    ohos_sdk_path = os.path.join(SCRIPT_PATH, OHOS_PROJECT_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Prepare static libs mapping: lib/static/{arch}/*.a
    static_libs = {}
    if link_type in ('static', 'both'):
        static_libs_base = os.path.join(ohos_sdk_path, "libs", "static")
        if os.path.exists(static_libs_base):
            for arch in os.listdir(static_libs_base):
                arch_dir = os.path.join(static_libs_base, arch)
                if os.path.isdir(arch_dir):
                    for lib_file in glob.glob(os.path.join(arch_dir, "*.a")):
                        lib_name = os.path.basename(lib_file)
                        arc_path = get_unified_lib_path("static", arch=arch, lib_name=lib_name, platform="ohos")
                        static_libs[arc_path] = lib_file

    # Prepare shared libs mapping: lib/shared/{arch}/*.so
    shared_libs = {}
    if link_type in ('shared', 'both'):
        shared_libs_base = os.path.join(ohos_sdk_path, "libs", "shared")
        if os.path.exists(shared_libs_base):
            for arch in os.listdir(shared_libs_base):
                arch_dir = os.path.join(shared_libs_base, arch)
                if os.path.isdir(arch_dir):
                    for lib_file in glob.glob(os.path.join(arch_dir, "*.so")):
                        lib_name = os.path.basename(lib_file)
                        arc_path = get_unified_lib_path("shared", arch=arch, lib_name=lib_name, platform="ohos")
                        shared_libs[arc_path] = lib_file

    # Prepare HAR files mapping: haars/*.har
    # Only use HAR from target/ohos/ (Hvigor buildHAR copies the renamed HAR there)
    haars = {}
    target_ohos_dir = os.path.join(bin_dir, "ohos")
    if os.path.exists(target_ohos_dir):
        for har_file in glob.glob(os.path.join(target_ohos_dir, "*.har")):
            har_name = os.path.basename(har_file)
            arc_path = get_unified_haar_path(har_name)
            haars[arc_path] = har_file

    # Prepare include directories mapping
    include_dirs = {}
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(PROJECT_NAME_LOWER, headers_src)
        include_dirs[arc_path] = headers_src

    # Prepare symbols (unstripped shared libs from obj/local/)
    obj_files = {}
    if link_type in ('shared', 'both'):
        obj_base = os.path.join(ohos_sdk_path, "obj", "local")
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
        platform_name="OHOS",
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


def main(incremental, build_archs, target_option="", link_type='both', jobs=None):
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
        jobs: Number of parallel build jobs (default: CPU count)

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

    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    print(f"main archs [{build_archs}], link_type:{link_type}, jobs:{jobs}")

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),

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

    # 2. Old libs/{arch}/ (from old OHOS-specific structure, now we use libs/shared/ and libs/static/)
    # Clean up old arch directories directly under libs/ (e.g., libs/armeabi-v7a/)
    for old_arch in ["armeabi-v7a", "arm64-v8a", "x86_64"]:
        old_libs_arch = os.path.join(OHOS_LIBS_PATH, old_arch)
        if os.path.exists(old_libs_arch):
            shutil.rmtree(old_libs_arch)
            print(f"Cleaned up old directory structure: {old_libs_arch}")

    has_error = False
    success_archs = []
    for arch in build_archs:
        if not build_ohos(incremental, arch, target_option, link_type, jobs):
            has_error = True
            break
        success_archs.append(arch)
    print("==================OHOS Build Done========================")
    print(f"Build All:{build_archs}")
    print(f"Build Success:{success_archs}")
    print(f"Build Failed:{list(set(build_archs) - set(success_archs))}")
    print("==================Output========================")
    if link_type == 'static':
        print(f"libs(release - static): {OHOS_LIBS_PATH}static/")
        print(f"symbols(static): {OHOS_SYMBOL_PATH.replace('obj/local/', 'obj/static_local/')}")
    elif link_type == 'shared':
        print(f"libs(release - shared): {OHOS_LIBS_PATH}shared/")
        print(f"symbols(must store permanently): {OHOS_SYMBOL_PATH}")
    elif link_type == 'both':
        print(f"libs(release - shared): {OHOS_LIBS_PATH}shared/")
        print(f"libs(release - static): {OHOS_LIBS_PATH}static/")
        print(f"symbols(shared - must store permanently): {OHOS_SYMBOL_PATH}")
        print(f"symbols(static): {OHOS_SYMBOL_PATH.replace('obj/local/', 'obj/static_local/')}")

    if has_error:
        raise RuntimeError("Exception occurs when build ohos")


# Command-line interface for OHOS builds
# Argument-based interface:
# Default (no args): Print build results from target directory
# --native-only: Build native libraries only (for Hvigor flow Step 1)
# --native-only --archive: Build native libraries AND create archive (for Docker builds)
# --archive-only: Create unified archive only (for Hvigor flow Step 3, after Hvigor buildHAR)
# --arch: Specify architectures (comma-separated)
#
# Usage examples:
# python3 build_ohos.py                              # Print build results (default)
# python3 build_ohos.py --native-only                # Build native libs only (Hvigor flow Step 1)
# python3 build_ohos.py --native-only --archive      # Build native libs + archive (Docker)
# python3 build_ohos.py --archive-only               # Create archive only (Hvigor flow Step 3)
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
        help="Only build native libraries (skip Hvigor packaging)",
    )
    parser.add_argument(
        "--archive",
        action="store_true",
        help="Create archive after native build (use with --native-only for Docker builds)",
    )
    parser.add_argument(
        "--archive-only",
        action="store_true",
        help="Create unified archive only (for Hvigor flow Step 3, after Hvigor buildHAR)",
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
        # Create unified archive only (for Hvigor flow Step 3)
        # Native libs were built in Step 1, HAR was built by Hvigor in Step 2
        print("==================OHOS Archive Only Mode==================")
        archs = [arch.strip() for arch in args.arch.split(",")]
        archive_ohos_project(link_type=args.link_type, archs=archs)
        print_build_results(link_type=args.link_type)
    elif args.native_only:
        # Build native libraries
        archs = [arch.strip() for arch in args.arch.split(",")]
        print(f"==================OHOS Native Build, archs: {archs}, link_type: {args.link_type}, jobs: {args.jobs or 'auto'}==================")
        main(args.incremental, archs, link_type=args.link_type, jobs=args.jobs)

        # Create archive if requested (for Docker builds)
        if args.archive:
            archive_ohos_project(link_type=args.link_type, archs=archs)
            print_build_results(link_type=args.link_type)
    else:
        # Default: Print build results
        print("==================OHOS Build Results Mode==================")
        print_build_results(link_type=args.link_type)
