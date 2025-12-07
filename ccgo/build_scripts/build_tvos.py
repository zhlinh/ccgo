#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_tvos.py
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
tvOS native library build script.

This script builds universal static libraries and XCFrameworks for tvOS platform
using CMake and tvOS toolchain. It handles:
- Building for physical devices (Apple TV 4K: arm64, arm64e)
- Building for simulators (x86_64, arm64, arm64e)
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
    python3 build_tvos.py [mode]

    mode: 1 (build XCFramework), 2 (generate Xcode project), 3 (exit)

Output:
    - XCFramework: cmake_build/tvOS/Darwin.out/{project}.xcframework
    - Frameworks: cmake_build/tvOS/Darwin.out/os|simulator/{project}.framework
"""

import glob
import os
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

# Build output paths
BUILD_OUT_PATH = "cmake_build/tvOS"
# Darwin(Linux,Windows).out = ${CMAKE_SYSTEM_NAME}.out
INSTALL_PATH = BUILD_OUT_PATH + "/Darwin.out"

# CMake build command for tvOS Simulator (x86_64 for Intel Macs, arm64/arm64e for M1+ simulators)
# Targets tvOS 10.0+, disables ARC and Bitcode, enables symbol visibility
# Parameters: ccgo_cmake_dir, ccgo_cmake_dir, target_option, jobs
TVOS_BUILD_SIMULATOR_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE="%s/tvos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=SIMULATOR_TVOS -DIOS_ARCH="x86_64;arm64;arm64e" -DIOS_DEPLOYMENT_TARGET=10.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 %s && make -j%d && make install'

# CMake build command for tvOS physical devices (arm64/arm64e for Apple TV 4K and later)
# tvOS does not support armv7/armv7s (no legacy device support)
# Parameters: ccgo_cmake_dir, ccgo_cmake_dir, target_option, jobs
TVOS_BUILD_OS_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE="%s/tvos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=TVOS -DIOS_ARCH="arm64;arm64e" -DIOS_DEPLOYMENT_TARGET=10.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 %s && make -j%d && make install'

# CMake build commands for shared library (dylib) - used for dynamic frameworks
# Simulator shared library build command
TVOS_BUILD_SIMULATOR_SHARED_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE="%s/tvos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=SIMULATOR_TVOS -DIOS_ARCH="x86_64;arm64;arm64e" -DIOS_DEPLOYMENT_TARGET=10.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 -DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON %s && make -j%d && make install'

# Device shared library build command
TVOS_BUILD_OS_SHARED_CMD = 'cmake ../.. -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE="%s/tvos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=TVOS -DIOS_ARCH="arm64;arm64e" -DIOS_DEPLOYMENT_TARGET=10.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 -DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON %s && make -j%d && make install'

# Xcode project generation command (for development/debugging)
# Parameters: ccgo_cmake_dir, ccgo_cmake_dir, target_option
GEN_TVOS_OS_PROJ = 'cmake ../.. -G Xcode -DCMAKE_TOOLCHAIN_FILE="%s/tvos.toolchain.cmake" -DCCGO_CMAKE_DIR="%s" -DIOS_PLATFORM=TVOS -DIOS_ARCH="arm64;arm64e" -DIOS_DEPLOYMENT_TARGET=10.0 -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DENABLE_VISIBILITY=1 %s'

# All supported tvOS architectures for third-party library integration
THIRD_PARTY_ARCHS = ["x86_64", "arm64e", "arm64"]


def build_tvos(target_option="",  link_type='both', jobs=None):
    """
    Build tvOS XCFramework containing both device and simulator frameworks.

    This function performs the complete tvOS build process:
    1. Generates version info header file
    2. Builds static libraries for physical devices (arm64, arm64e)
    3. Merges device static libraries using libtool
    4. Builds static libraries for simulators (x86_64, arm64, arm64e)
    5. Merges simulator static libraries using libtool
    6. Creates .framework bundle for device libraries
    7. Creates .framework bundle for simulator libraries
    8. Generates XCFramework combining both device and simulator frameworks

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Device framework: cmake_build/tvOS/Darwin.out/os/{project}.framework
        - Simulator framework: cmake_build/tvOS/Darwin.out/simulator/{project}.framework
        - XCFramework: cmake_build/tvOS/Darwin.out/{project}.xcframework

    Note:
        The XCFramework is the recommended distribution format for tvOS libraries
        as it contains binaries for both devices and simulators in a single bundle.
        This allows Xcode to automatically select the correct binary during builds.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    before_time = time.time()
    print(f"==================build_tvos (link_type: {link_type}, jobs: {jobs})========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        
        platform="tvos",
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

    build_cmd = TVOS_BUILD_OS_CMD % (CCGO_CMAKE_DIR, CCGO_CMAKE_DIR, full_target_option, jobs)
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
    dst_framework_headers = TVOS_BUILD_COPY_HEADER_FILES
    # add static libs
    total_src_lib = glob.glob(INSTALL_PATH + "/*.a")
    rm_src_lib = []
    libtool_src_lib = [x for x in total_src_lib if x not in rm_src_lib]
    print(f"libtool src lib: {len(libtool_src_lib)}/{len(total_src_lib)}")

    if not libtool_libs(libtool_src_lib, libtool_os_dst_lib):
        return False

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = TVOS_BUILD_SIMULATOR_CMD % (
        CCGO_CMAKE_DIR,
        CCGO_CMAKE_DIR,
        full_target_option,
        jobs,
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
    apple_headers_src = f"include/{PROJECT_NAME_LOWER}/api/apple/"
    make_static_framework(
        os_lipo_dst_lib, os_dst_framework_path, dst_framework_headers, "./",
        apple_headers_src=apple_headers_src
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
        apple_headers_src=apple_headers_src
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


def build_tvos_shared(target_option="", jobs=None):
    """
    Build tvOS Dynamic Framework (XCFramework with dylib inside).

    This function builds shared libraries (dylib) for tvOS and packages them
    as dynamic frameworks, which is the required format for App Store submission.

    The build process:
    1. Build dylib for physical devices (arm64, arm64e)
    2. Merge device dylibs using lipo
    3. Build dylib for simulators (x86_64, arm64, arm64e)
    4. Merge simulator dylibs using lipo
    5. Create dynamic .framework bundle for device
    6. Create dynamic .framework bundle for simulator
    7. Generate XCFramework combining both

    Args:
        target_option: Additional CMake target options (default: '')
        jobs: Number of parallel build jobs (default: CPU count)

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Device framework: cmake_build/tvOS/Darwin.out/shared/os/{project}.framework
        - Simulator framework: cmake_build/tvOS/Darwin.out/shared/simulator/{project}.framework
        - XCFramework: cmake_build/tvOS/Darwin.out/shared/{project}.xcframework
    """
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    before_time = time.time()
    print(f"==================build_tvos_shared (jobs: {jobs})========================")

    shared_install_path = INSTALL_PATH + "/shared"

    # Create shared install directories first
    os_shared_dir = os.path.join(shared_install_path, "os")
    simulator_shared_dir = os.path.join(shared_install_path, "simulator")
    os.makedirs(os_shared_dir, exist_ok=True)
    os.makedirs(simulator_shared_dir, exist_ok=True)

    # Build device dylib - reuse BUILD_OUT_PATH like static build
    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = TVOS_BUILD_OS_SHARED_CMD % (CCGO_CMAKE_DIR, CCGO_CMAKE_DIR, target_option, jobs)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build shared os fail!!!!!!!!!!!!!!!")
        return False

    # Find and lipo merge device dylibs
    os_dylibs = glob.glob(INSTALL_PATH + f"/*.dylib")
    if not os_dylibs:
        print("ERROR: No device dylibs found")
        return False

    os_lipo_dst = os.path.join(os_shared_dir, f"lib{PROJECT_NAME_LOWER}.dylib")

    if len(os_dylibs) > 1:
        if not lipo_libs(os_dylibs, os_lipo_dst):
            print("ERROR: Failed to lipo device dylibs")
            return False
    else:
        shutil.copy(os_dylibs[0], os_lipo_dst)

    # Save device dylib to temp location before cleaning
    temp_os_dylib = os.path.join(SCRIPT_PATH, f"_temp_os_lib{PROJECT_NAME_LOWER}.dylib")
    shutil.copy(os_lipo_dst, temp_os_dylib)

    # Build simulator dylib
    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    build_cmd = TVOS_BUILD_SIMULATOR_SHARED_CMD % (CCGO_CMAKE_DIR, CCGO_CMAKE_DIR, target_option, jobs)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build shared simulator fail!!!!!!!!!!!!!!!")
        # Clean up temp file
        if os.path.exists(temp_os_dylib):
            os.remove(temp_os_dylib)
        return False

    # Find and lipo merge simulator dylibs
    simulator_dylibs = glob.glob(INSTALL_PATH + f"/*.dylib")
    if not simulator_dylibs:
        print("ERROR: No simulator dylibs found")
        # Clean up temp file
        if os.path.exists(temp_os_dylib):
            os.remove(temp_os_dylib)
        return False

    # Recreate shared directories (may have been cleaned)
    os.makedirs(os_shared_dir, exist_ok=True)
    os.makedirs(simulator_shared_dir, exist_ok=True)

    # Restore device dylib from temp location
    shutil.copy(temp_os_dylib, os_lipo_dst)
    os.remove(temp_os_dylib)

    simulator_lipo_dst = os.path.join(simulator_shared_dir, f"lib{PROJECT_NAME_LOWER}.dylib")

    if len(simulator_dylibs) > 1:
        if not lipo_libs(simulator_dylibs, simulator_lipo_dst):
            print("ERROR: Failed to lipo simulator dylibs")
            return False
    else:
        shutil.copy(simulator_dylibs[0], simulator_lipo_dst)

    # Create dynamic frameworks
    dst_framework_headers = TVOS_BUILD_COPY_HEADER_FILES
    apple_headers_src = f"include/{PROJECT_NAME_LOWER}/api/apple/"

    # Device dynamic framework
    os_dst_framework_path = os.path.join(os_shared_dir, f"{PROJECT_NAME_LOWER}.framework")
    make_dynamic_framework(
        os_lipo_dst, os_dst_framework_path, dst_framework_headers, "./",
        apple_headers_src=apple_headers_src
    )

    # Simulator dynamic framework
    simulator_dst_framework_path = os.path.join(simulator_shared_dir, f"{PROJECT_NAME_LOWER}.framework")
    make_dynamic_framework(
        simulator_lipo_dst, simulator_dst_framework_path, dst_framework_headers, "./",
        apple_headers_src=apple_headers_src
    )

    # Create dynamic XCFramework
    dst_xcframework_path = os.path.join(shared_install_path, f"{PROJECT_NAME_LOWER}.xcframework")
    if not make_xcframework(
        os_dst_framework_path, simulator_dst_framework_path, dst_xcframework_path
    ):
        print("ERROR: Failed to create dynamic XCFramework")
        return False

    # Verify built frameworks
    print("\n==================Verifying tvOS Dynamic Frameworks========================")
    os_lib = os.path.join(os_dst_framework_path, PROJECT_NAME_LOWER)
    if os.path.exists(os_lib):
        print("Device Dynamic Framework:")
        check_library_architecture(os_lib, platform_hint="tvos")

    simulator_lib = os.path.join(simulator_dst_framework_path, PROJECT_NAME_LOWER)
    if os.path.exists(simulator_lib):
        print("\nSimulator Dynamic Framework:")
        check_library_architecture(simulator_lib, platform_hint="tvos")
    print("=====================================================================")

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_xcframework_path)

    after_time = time.time()
    print(f"use time: {int(after_time - before_time)} s")
    return True


def archive_tvos_project(link_type='both'):
    """
    Archive tvOS XCFramework with unified structure.

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_TVOS_SDK-{version}.zip
       - frameworks/tvos/static/{Project}.xcframework (static XCFramework)
       - frameworks/tvos/shared/{Project}.xcframework (dynamic XCFramework)
       - include/{project}/
       - build_info.json
    2. Symbols package: {PROJECT_NAME}_TVOS_SDK-{version}-SYMBOLS.zip
       - symbols/tvos/static/*.dSYM
       - symbols/tvos/shared/*.dSYM

    Note:
        tvOS supports both static and dynamic frameworks packaged as XCFramework.
        Dynamic frameworks contain dylib inside .framework bundle and are suitable
        for App Store distribution.

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')

    Output:
        - target/tvos/{PROJECT_NAME}_TVOS_SDK-{version}.zip
        - target/tvos/{PROJECT_NAME}_TVOS_SDK-{version}-SYMBOLS.zip
    """
    print("==================Archive tvOS Project========================")

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(SCRIPT_PATH)

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    tvos_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Prepare frameworks mapping
    frameworks = {}
    xcframework_name = f"{PROJECT_NAME_LOWER}.xcframework"

    # Static XCFramework (from root install path)
    static_xcframework_src = os.path.join(tvos_install_path, xcframework_name)
    if os.path.exists(static_xcframework_src) and link_type in ('static', 'both'):
        arc_path = get_unified_framework_path("static", xcframework_name, platform="tvos")
        frameworks[arc_path] = static_xcframework_src

    # Dynamic XCFramework (from shared subdirectory)
    shared_xcframework_src = os.path.join(tvos_install_path, "shared", xcframework_name)
    if os.path.exists(shared_xcframework_src) and link_type in ('shared', 'both'):
        arc_path = get_unified_framework_path("shared", xcframework_name, platform="tvos")
        frameworks[arc_path] = shared_xcframework_src

    if not frameworks:
        print(f"WARNING: No XCFramework found at {tvos_install_path}")
        return

    # Prepare include directories mapping
    include_dirs = {}
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(PROJECT_NAME_LOWER, headers_src)
        include_dirs[arc_path] = headers_src

    # Prepare symbols (dSYM files)
    symbols_static = {}
    symbols_shared = {}

    # Static dSYM files (from root install path)
    dsym_pattern = f"{tvos_install_path}/*.dSYM"
    dsym_files = glob.glob(dsym_pattern)
    for dsym_file in dsym_files:
        dsym_name = os.path.basename(dsym_file)
        if link_type in ('static', 'both'):
            arc_path = get_unified_symbol_path("static", dsym_name, platform="tvos")
            symbols_static[arc_path] = dsym_file

    # Shared dSYM files (from shared subdirectory)
    shared_dsym_pattern = f"{tvos_install_path}/shared/*.dSYM"
    shared_dsym_files = glob.glob(shared_dsym_pattern)
    for dsym_file in shared_dsym_files:
        dsym_name = os.path.basename(dsym_file)
        if link_type in ('shared', 'both'):
            arc_path = get_unified_symbol_path("shared", dsym_name, platform="tvos")
            symbols_shared[arc_path] = dsym_file

    # Create unified archive packages
    main_zip_path, symbols_zip_path = create_unified_archive(
        output_dir=bin_dir,
        project_name=PROJECT_NAME,
        platform_name="TVOS",
        version=full_version,
        link_type=link_type,
        include_dirs=include_dirs,
        frameworks=frameworks,
        symbols_static=symbols_static if symbols_static else None,
        symbols_shared=symbols_shared if symbols_shared else None,
        architectures=["arm64", "arm64e", "x86_64"],  # Device + Simulator
    )

    print("\n==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    if symbols_zip_path:
        print(f"Symbols package: {symbols_zip_path}")


def print_build_results():
    """
    Print tvOS build results from target directory.

    This function displays the build artifacts and moves them to target/tvos/:
    - Main SDK ZIP packages
    - Symbols ZIP packages (-SYMBOLS.zip)
    """
    print("==================tvOS Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for SDK ZIP packages (both main and symbols)
    all_zips = glob.glob(f"{bin_dir}/*.zip")
    sdk_zips = [
        f for f in all_zips
        if "TVOS_SDK" in os.path.basename(f) and not os.path.basename(f).startswith("_temp_")
    ]

    if not sdk_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Clean and recreate target/tvos directory for platform-specific artifacts
    bin_tvos_dir = os.path.join(bin_dir, "tvos")
    if os.path.exists(bin_tvos_dir):
        shutil.rmtree(bin_tvos_dir)
        print(f"Cleaned up old target/tvos/ directory")
    os.makedirs(bin_tvos_dir, exist_ok=True)

    # Move SDK ZIP files to target/tvos/
    artifacts_moved = []
    for sdk_zip in sdk_zips:
        dest = os.path.join(bin_tvos_dir, os.path.basename(sdk_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(sdk_zip, dest)
        artifacts_moved.append(os.path.basename(sdk_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/tvos/")

    # Copy build_info.json from cmake_build to target/tvos
    copy_build_info_to_target("tvos", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/tvos/:")
    print("-" * 60)

    # List all files in target/tvos directory with sizes
    for item in sorted(os.listdir(bin_tvos_dir)):
        item_path = os.path.join(bin_tvos_dir, item)
        if os.path.isfile(item_path):
            size = os.path.getsize(item_path) / (1024 * 1024)  # MB
            print(f"  {item} ({size:.2f} MB)")

            # Print ZIP file tree structure
            if item.endswith(".zip"):
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


def gen_tvos_project(target_option=""):
    """
    Generate Xcode project for tvOS development and debugging.

    This function creates an Xcode project (.xcodeproj) that can be opened in Xcode
    for interactive development, debugging, and testing. Unlike build_tvos() which
    creates distributable frameworks, this generates IDE project files.

    Args:
        target_option: Additional CMake target options (default: '')
        tag: Version tag string for metadata (default: '')

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - Xcode project: cmake_build/tvOS/{project}.xcodeproj

    Note:
        The generated Xcode project is configured for tvOS device builds.
        To build for simulator, you can switch the scheme in Xcode.
        This is useful for development workflows where you need Xcode's
        debugging tools, code completion, and build system integration.
    """
    print("==================gen_tvos_project========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        
        platform="tvos",
    )

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    cmd = GEN_TVOS_OS_PROJ % (CCGO_CMAKE_DIR, CCGO_CMAKE_DIR, target_option)
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!gen fail!!!!!!!!!!!!!!!")
        return False

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {SCRIPT_PATH}/{BUILD_OUT_PATH}")

    return True


def main(target_option="", link_type='both', jobs=None):
    """
    Main entry point for tvOS XCFramework build.

    This function serves as the primary entry point when building
    distributable tvOS frameworks and XCFrameworks.

    Args:
        target_option: Additional CMake target options (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Note:
        This function calls build_tvos() to create the static XCFramework,
        and build_tvos_shared() to create the dynamic XCFramework,
        then archives and moves artifacts to target/tvos/ directory.
        For Xcode project generation, use gen_tvos_project() instead.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    print(f"main link_type: {link_type}, jobs: {jobs}")

    # Temp storage for static XCFramework (to preserve it during shared build)
    temp_static_xcframework = None
    static_xcframework_path = os.path.join(INSTALL_PATH, f"{PROJECT_NAME_LOWER}.xcframework")

    # Build static XCFramework
    if link_type in ('static', 'both'):
        if not build_tvos(target_option, 'static', jobs):
            print("ERROR: tvOS static build failed")
            sys.exit(1)

        # If we also need to build shared, save the static XCFramework first
        if link_type == 'both' and os.path.exists(static_xcframework_path):
            temp_static_xcframework = os.path.join(SCRIPT_PATH, f"_temp_static_{PROJECT_NAME_LOWER}.xcframework")
            if os.path.exists(temp_static_xcframework):
                shutil.rmtree(temp_static_xcframework)
            shutil.copytree(static_xcframework_path, temp_static_xcframework)
            print(f"Saved static XCFramework to temp location")

    # Build dynamic XCFramework (shared library packaged as framework)
    if link_type in ('shared', 'both'):
        if not build_tvos_shared(target_option, jobs):
            print("ERROR: tvOS shared/dynamic build failed")
            # Clean up temp static XCFramework
            if temp_static_xcframework and os.path.exists(temp_static_xcframework):
                shutil.rmtree(temp_static_xcframework)
            sys.exit(1)

        # Restore static XCFramework if we built both
        if temp_static_xcframework and os.path.exists(temp_static_xcframework):
            # The static XCFramework should go to the root INSTALL_PATH
            if os.path.exists(static_xcframework_path):
                shutil.rmtree(static_xcframework_path)
            shutil.copytree(temp_static_xcframework, static_xcframework_path)
            shutil.rmtree(temp_static_xcframework)
            print(f"Restored static XCFramework from temp location")

    # Archive and organize artifacts
    archive_tvos_project(link_type)
    print_build_results()


# Command-line interface for tvOS builds
#
# Usage:
#   python build_tvos.py                    # Build static library (default)
#   python build_tvos.py --ide-project      # Generate Xcode project
#   python build_tvos.py -j 8               # Build with 8 parallel jobs
#   python build_tvos.py --link-type shared # Build shared library
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build tvOS XCFramework",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "-j", "--jobs",
        type=int,
        default=None,
        help="Number of parallel build jobs (default: CPU count)",
    )
    parser.add_argument(
        "--link-type",
        type=str,
        choices=['static', 'shared', 'both'],
        default='both',
        help="Library link type (default: both)",
    )
    parser.add_argument(
        "--ide-project",
        action="store_true",
        help="Generate Xcode project instead of building",
    )

    args = parser.parse_args()

    if args.ide_project:
        gen_tvos_project()
    else:
        main(link_type=args.link_type, jobs=args.jobs)
