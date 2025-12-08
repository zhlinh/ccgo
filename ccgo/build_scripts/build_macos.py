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

# Build output paths - now separated by link type
# Static: cmake_build/macOS/static/Darwin.out/
# Shared: cmake_build/macOS/shared/Darwin.out/
BUILD_OUT_PATH_BASE = "cmake_build/macOS"

def get_build_out_path(link_type):
    """Get build output path for specified link type."""
    return f"{BUILD_OUT_PATH_BASE}/{link_type}"

def get_install_path(link_type):
    """Get install path for specified link type."""
    return f"{BUILD_OUT_PATH_BASE}/{link_type}/Darwin.out"

# Legacy paths for backward compatibility
BUILD_OUT_PATH = BUILD_OUT_PATH_BASE + "/static"
INSTALL_PATH = BUILD_OUT_PATH + "/Darwin.out"

# CMake build command for macOS (defaults to x86_64 if no arch specified)
# Disables ARC and Bitcode for C/C++ native libraries
# Parameters: ccgo_cmake_dir, target_option, jobs
# Note: ../../.. because we're now in cmake_build/macOS/<link_type>/
MACOS_BUILD_OS_CMD = 'cmake ../../.. -DCMAKE_BUILD_TYPE=Release -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DCCGO_CMAKE_DIR="%s" %s && make -j%d && make install'

# CMake build command for Apple Silicon Macs (M1, M2, M3, etc.)
# Builds for arm64 and arm64e architectures
# Parameters: ccgo_cmake_dir, target_option, jobs
MACOS_BUILD_ARM_CMD = 'cmake ../../.. -DCMAKE_BUILD_TYPE=Release -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DCMAKE_OSX_ARCHITECTURES="arm64;arm64e" -DCCGO_CMAKE_DIR="%s" %s && make -j%d && make install'

# CMake build command for Intel Macs
# Builds for x86_64 architecture only
# Parameters: ccgo_cmake_dir, target_option, jobs
MACOS_BUILD_X86_CMD = 'cmake ../../.. -DCMAKE_BUILD_TYPE=Release -DENABLE_ARC=0 -DENABLE_BITCODE=0 -DCMAKE_OSX_ARCHITECTURES="x86_64" -DCCGO_CMAKE_DIR="%s" %s && make -j%d && make install'

# Xcode project generation command
# Targets macOS 10.9+ for broad compatibility, disables Bitcode
# Note: Uses ../../.. because build directory is now cmake_build/macOS/{link_type}/
GEN_MACOS_PROJ = 'cmake ../../.. -G Xcode -DCMAKE_OSX_DEPLOYMENT_TARGET:STRING=10.9 -DENABLE_BITCODE=0 -DCCGO_CMAKE_DIR="%s" %s'


def _build_macos_single(target_option, single_link_type, jobs):
    """
    Internal function to build macOS library for a single link type.

    Args:
        target_option: Additional CMake target options
        single_link_type: Either 'static' or 'shared' (not 'both')
        jobs: Number of parallel build jobs

    Returns:
        bool: True if build succeeded, False otherwise
    """
    build_out_path = get_build_out_path(single_link_type)
    install_path = get_install_path(single_link_type)

    # Set CMake flags for this specific link type
    if single_link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    else:  # shared
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"

    full_target_option = f"{link_type_flags} {target_option}".strip()

    print(f"\n--- Building {single_link_type} library ---")
    print(f"Build directory: {build_out_path}")

    clean(build_out_path)
    os.chdir(build_out_path)

    # Build for ARM (Apple Silicon)
    build_cmd = MACOS_BUILD_ARM_CMD % (CCGO_CMAKE_DIR, full_target_option, jobs)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print(f"!!!!!!!!!!!build ARM fail for {single_link_type}!!!!!!!!!!!!!!!")
        print(f"ERROR: Native build failed for macOS ARM ({single_link_type}). Stopping immediately.")
        sys.exit(1)

    # Collect and merge ARM libraries, then save before clean
    arm_lib_saved = None  # For static: merged .a, for shared: .dylib

    if single_link_type == 'static':
        # Static libraries are now in static/ subdirectory
        static_lib_path = install_path + "/static"
        total_src_lib = glob.glob(static_lib_path + "/*.a")
        # Also check root for backward compatibility
        total_src_lib.extend(glob.glob(install_path + "/*.a"))
        rm_src_lib = []
        libtool_src_lib = [x for x in total_src_lib if x not in rm_src_lib]
        print(f"libtool src lib (ARM): {len(libtool_src_lib)}/{len(total_src_lib)}")

        # Merge ARM libraries
        arm_merged_lib = install_path + f"/{PROJECT_NAME_LOWER}_arm"
        if not libtool_libs(libtool_src_lib, arm_merged_lib):
            print("ERROR: Failed to merge ARM libraries. Stopping immediately.")
            sys.exit(1)

        # Save merged ARM library before clean (since clean will delete it)
        arm_lib_saved = os.path.join(SCRIPT_PATH, f"_temp_arm_{PROJECT_NAME_LOWER}.a")
        shutil.copy2(arm_merged_lib, arm_lib_saved)
        print(f"Saved ARM merged library: {arm_lib_saved}")
    else:
        # For shared, save ARM dylib before clean (since clean will delete it)
        arm_dylib_src = glob.glob(install_path + "/shared/*.dylib")
        if arm_dylib_src:
            # Save to a temporary location outside the build directory
            arm_lib_saved = os.path.join(SCRIPT_PATH, f"_temp_arm_{PROJECT_NAME_LOWER}.dylib")
            shutil.copy2(arm_dylib_src[0], arm_lib_saved)
            print(f"Saved ARM dylib: {arm_lib_saved}")
        else:
            print("WARNING: No ARM dylib found to save")

    clean(build_out_path)
    os.chdir(build_out_path)

    # Build for x86_64 (Intel)
    build_cmd = MACOS_BUILD_X86_CMD % (CCGO_CMAKE_DIR, full_target_option, jobs)
    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print(f"!!!!!!!!!!!build x86 fail for {single_link_type}!!!!!!!!!!!!!!!")
        print(f"ERROR: Native build failed for macOS x86 ({single_link_type}). Stopping immediately.")
        sys.exit(1)

    if single_link_type == 'static':
        # Merge x86 libraries (check static/ subdirectory first, then root for backward compatibility)
        static_lib_path = install_path + "/static"
        x86_static_libs = glob.glob(static_lib_path + "/*.a")
        x86_static_libs.extend(glob.glob(install_path + "/*.a"))
        libtool_x86_dst_lib = install_path + f"/{PROJECT_NAME_LOWER}_x86"
        if not libtool_libs(x86_static_libs, libtool_x86_dst_lib):
            print("ERROR: Failed to merge x86 libraries. Stopping immediately.")
            sys.exit(1)

        # Restore saved ARM library
        arm_restored_lib = install_path + f"/{PROJECT_NAME_LOWER}_arm"
        if arm_lib_saved and os.path.exists(arm_lib_saved):
            shutil.copy2(arm_lib_saved, arm_restored_lib)
            os.remove(arm_lib_saved)
            print(f"Restored ARM merged library: {arm_restored_lib}")
        else:
            print("WARNING: Saved ARM library not found, universal binary may be x86-only")

        # Create universal binary from ARM and x86
        lipo_dst_lib = install_path + f"/{PROJECT_NAME_LOWER}"
        lipo_src_libs = []
        if os.path.exists(arm_restored_lib):
            lipo_src_libs.append(arm_restored_lib)
        if os.path.exists(libtool_x86_dst_lib):
            lipo_src_libs.append(libtool_x86_dst_lib)

        if not lipo_src_libs:
            print("ERROR: No libraries to merge for universal binary. Stopping immediately.")
            sys.exit(1)

        if not libtool_libs(lipo_src_libs, lipo_dst_lib):
            print("ERROR: Failed to create universal binary. Stopping immediately.")
            sys.exit(1)

        # Create framework
        dst_framework_path = install_path + f"/{PROJECT_NAME_LOWER}.framework"
        dst_framework_headers = MACOS_BUILD_COPY_HEADER_FILES
        apple_headers_src = f"include/{PROJECT_NAME_LOWER}/api/apple/"
        make_static_framework(
            lipo_dst_lib, dst_framework_path, dst_framework_headers, "./",
            apple_headers_src=apple_headers_src
        )

        # Verify the built framework
        print(f"\n==================Verifying macOS {single_link_type} Universal Binary========================")
        framework_lib = os.path.join(dst_framework_path, PROJECT_NAME_LOWER)
        if os.path.exists(framework_lib):
            check_library_architecture(framework_lib, platform_hint="macos")
        print("========================================================================")
    else:
        # For shared library, create universal dylib using lipo
        x86_dylib_src = glob.glob(install_path + "/shared/*.dylib")
        print(f"[DEBUG] x86 dylib search path: {install_path}/shared/*.dylib")
        print(f"[DEBUG] x86 dylib found: {x86_dylib_src}")
        print(f"[DEBUG] arm_lib_saved: {arm_lib_saved}")

        universal_dylib = None
        if x86_dylib_src and arm_lib_saved and os.path.exists(arm_lib_saved):
            # We have both x86 and ARM dylibs, merge them
            x86_dylib = install_path + f"/lib{PROJECT_NAME_LOWER}_x86.dylib"
            shutil.copy2(x86_dylib_src[0], x86_dylib)

            arm_dylib = install_path + f"/lib{PROJECT_NAME_LOWER}_arm.dylib"
            shutil.copy2(arm_lib_saved, arm_dylib)

            # Clean up temp file
            os.remove(arm_lib_saved)

            # Create universal dylib using lipo
            universal_dylib = install_path + f"/lib{PROJECT_NAME_LOWER}.dylib"
            lipo_cmd = f'lipo -create "{arm_dylib}" "{x86_dylib}" -output "{universal_dylib}"'
            print(f"[DEBUG] Running lipo: {lipo_cmd}")
            ret = os.system(lipo_cmd)
            if ret != 0:
                print("WARNING: Failed to create universal dylib")
                universal_dylib = None
            else:
                print(f"Created universal dylib: {universal_dylib}")
                check_library_architecture(universal_dylib, platform_hint="macos")
        elif x86_dylib_src:
            # Only x86 available, use it directly
            universal_dylib = install_path + f"/lib{PROJECT_NAME_LOWER}.dylib"
            shutil.copy2(x86_dylib_src[0], universal_dylib)
            print(f"Using x86-only dylib: {universal_dylib}")
            # Clean up temp file if it exists
            if arm_lib_saved and os.path.exists(arm_lib_saved):
                os.remove(arm_lib_saved)
        elif arm_lib_saved and os.path.exists(arm_lib_saved):
            # Only ARM available, use it directly
            universal_dylib = install_path + f"/lib{PROJECT_NAME_LOWER}.dylib"
            shutil.copy2(arm_lib_saved, universal_dylib)
            # Clean up temp file
            os.remove(arm_lib_saved)
            print(f"Using ARM-only dylib: {universal_dylib}")
        else:
            print("WARNING: No shared library found to create universal dylib")

        # Create shared framework (similar to static framework but with dylib)
        if universal_dylib and os.path.exists(universal_dylib):
            dst_framework_path = install_path + f"/{PROJECT_NAME_LOWER}.framework"
            dst_framework_headers = MACOS_BUILD_COPY_HEADER_FILES
            apple_headers_src = f"include/{PROJECT_NAME_LOWER}/api/apple/"
            make_static_framework(
                universal_dylib, dst_framework_path, dst_framework_headers, "./",
                apple_headers_src=apple_headers_src
            )
            print(f"Created shared framework: {dst_framework_path}")

            # Verify the built framework
            print(f"\n==================Verifying macOS {single_link_type} Universal Binary========================")
            framework_lib = os.path.join(dst_framework_path, PROJECT_NAME_LOWER)
            if os.path.exists(framework_lib):
                check_library_architecture(framework_lib, platform_hint="macos")
            print("========================================================================")

    return True


def build_macos(target_option="",  link_type='both', jobs=None):
    """
    Build universal macOS framework supporting both Intel and Apple Silicon.

    This function performs the complete macOS build process:
    1. Generates version info header file
    2. For each link type (static and/or shared):
       - Builds libraries for Apple Silicon (arm64, arm64e)
       - Builds libraries for Intel (x86_64)
       - Creates universal binary with libtool/lipo
       - Creates .framework bundle (for static)

    Build directories:
    - Static: cmake_build/macOS/static/Darwin.out/
    - Shared: cmake_build/macOS/shared/Darwin.out/

    Args:
        target_option: Additional CMake target options (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Static framework: cmake_build/macOS/static/Darwin.out/{project}.framework
        - Shared library: cmake_build/macOS/shared/Darwin.out/lib{project}.dylib
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    before_time = time.time()
    print(f"==================build_macos (link_type: {link_type}, jobs: {jobs})========================")

    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        platform="macos",
    )

    # Build for each link type
    if link_type == 'both':
        # Build static first
        _build_macos_single(target_option, 'static', jobs)
        # Then build shared
        _build_macos_single(target_option, 'shared', jobs)
    else:
        # Build only the specified type
        _build_macos_single(target_option, link_type, jobs)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    after_time = time.time()
    print(f"use time: {int(after_time - before_time)} s")
    return True


def archive_macos_project(link_type='both'):
    """
    Archive macOS framework with unified structure.

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_MACOS_SDK-{version}.zip
       - lib/static/{Project}.framework (if link_type is static or both)
       - lib/shared/{Project}.framework (if link_type is shared or both)
       - include/{project}/
       - build_info.json
    2. Symbols package: {PROJECT_NAME}_MACOS_SDK-{version}-SYMBOLS.zip
       - symbols/static/*.dSYM
       - symbols/shared/*.dSYM

    Note: macOS uses .framework bundles which already contain the binary and headers.
    No need for separate static_libs or shared_libs - frameworks are the recommended
    distribution format for Apple platforms.

    Build directories used:
    - Static: cmake_build/macOS/static/Darwin.out/
    - Shared: cmake_build/macOS/shared/Darwin.out/

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')

    Output:
        - target/macos/{PROJECT_NAME}_MACOS_SDK-{version}.zip
        - target/macos/{PROJECT_NAME}_MACOS_SDK-{version}-SYMBOLS.zip
    """
    print("==================Archive macOS Project========================")

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(SCRIPT_PATH)

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")
    static_install_path = os.path.join(SCRIPT_PATH, get_install_path("static"))
    shared_install_path = os.path.join(SCRIPT_PATH, get_install_path("shared"))

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Note: macOS uses .framework bundles which already contain the binary and headers.
    # No need for separate static_libs or shared_libs - frameworks are the recommended
    # distribution format for Apple platforms.

    # Prepare frameworks mapping
    frameworks = {}
    framework_name = f"{PROJECT_NAME_LOWER}.framework"

    # Static framework (from static build directory)
    if link_type in ('static', 'both'):
        static_framework_src = os.path.join(static_install_path, framework_name)
        if os.path.exists(static_framework_src):
            arc_path = get_unified_framework_path("static", framework_name, platform="macos")
            frameworks[arc_path] = static_framework_src

    # Shared framework (from shared build directory)
    if link_type in ('shared', 'both'):
        shared_framework_src = os.path.join(shared_install_path, framework_name)
        if os.path.exists(shared_framework_src):
            arc_path = get_unified_framework_path("shared", framework_name, platform="macos")
            frameworks[arc_path] = shared_framework_src

    # Prepare include directories mapping
    include_dirs = {}
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(PROJECT_NAME_LOWER, headers_src)
        include_dirs[arc_path] = headers_src

    # Prepare symbols (dSYM files from both directories)
    symbols_static = {}
    symbols_shared = {}

    # Collect from static build directory
    if link_type in ('static', 'both'):
        dsym_pattern = f"{static_install_path}/*.dSYM"
        dsym_files = glob.glob(dsym_pattern)
        for dsym_file in dsym_files:
            dsym_name = os.path.basename(dsym_file)
            arc_path = get_unified_symbol_path("static", dsym_name, platform="macos")
            symbols_static[arc_path] = dsym_file

    # Collect from shared build directory
    if link_type in ('shared', 'both'):
        dsym_pattern = f"{shared_install_path}/*.dSYM"
        dsym_files = glob.glob(dsym_pattern)
        for dsym_file in dsym_files:
            dsym_name = os.path.basename(dsym_file)
            arc_path = get_unified_symbol_path("shared", dsym_name, platform="macos")
            symbols_shared[arc_path] = dsym_file

    # Create unified archive packages
    main_zip_path, symbols_zip_path = create_unified_archive(
        output_dir=bin_dir,
        project_name=PROJECT_NAME,
        platform_name="MACOS",
        version=full_version,
        link_type=link_type,
        include_dirs=include_dirs,
        frameworks=frameworks,
        symbols_static=symbols_static if symbols_static else None,
        symbols_shared=symbols_shared if symbols_shared else None,
        architectures=["x86_64", "arm64"],  # Universal binary
    )

    print("\n==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    if symbols_zip_path:
        print(f"Symbols package: {symbols_zip_path}")


def print_build_results(link_type='both'):
    """
    Print macOS build results from target directory.

    This function displays the build artifacts and moves them to target/macos/:
    - Main SDK ZIP package
    - Symbols ZIP package (if available)
    - build_info.json

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
    """
    print("==================macOS Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for SDK ZIP packages (main and symbols)
    # Main package: {PROJECT_NAME}_MACOS_SDK-*.zip (not ending with -SYMBOLS.zip)
    main_zips = [
        f for f in glob.glob(f"{bin_dir}/*_MACOS_SDK-*.zip")
        if not f.endswith("-SYMBOLS.zip") and not os.path.basename(f).startswith("_temp_")
    ]

    # Symbols package: {PROJECT_NAME}_MACOS_SDK-*-SYMBOLS.zip
    symbols_zips = [
        f for f in glob.glob(f"{bin_dir}/*_MACOS_SDK-*-SYMBOLS.zip")
    ]

    if not main_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Clean and recreate target/macos directory for platform-specific artifacts
    bin_macos_dir = os.path.join(bin_dir, "macos")
    if os.path.exists(bin_macos_dir):
        shutil.rmtree(bin_macos_dir)
        print(f"Cleaned up old target/macos/ directory")
    os.makedirs(bin_macos_dir, exist_ok=True)

    # Move SDK ZIP files to target/macos/
    artifacts_moved = []
    for main_zip in main_zips:
        dest = os.path.join(bin_macos_dir, os.path.basename(main_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(main_zip, dest)
        artifacts_moved.append(os.path.basename(main_zip))

    for symbols_zip in symbols_zips:
        dest = os.path.join(bin_macos_dir, os.path.basename(symbols_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(symbols_zip, dest)
        artifacts_moved.append(os.path.basename(symbols_zip))

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


def gen_macos_project(target_option=""):
    """
    Generate Xcode project for macOS development and debugging.

    This function creates an Xcode project (.xcodeproj) that can be opened in Xcode
    for interactive development, debugging, and testing. The project is automatically
    opened in Xcode after generation.

    Args:
        target_option: Additional CMake target options (default: '')

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - Xcode project: cmake_build/macOS/static/{project}.xcodeproj (auto-opened)

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
        platform="macos",
    )
    # Use static directory for IDE project
    build_out_path = get_build_out_path("static")
    clean(build_out_path)
    os.chdir(build_out_path)

    cmd = GEN_MACOS_PROJ % (CCGO_CMAKE_DIR, target_option)
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!gen fail!!!!!!!!!!!!!!!")
        return False

    project_file_prefix = os.path.join(SCRIPT_PATH, build_out_path, PROJECT_NAME_LOWER)
    project_file = get_project_file_name(project_file_prefix)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {project_file}")

    os.system(get_open_project_file_cmd(project_file))

    return True


def main(target_option="", link_type='both', jobs=None):
    """
    Main entry point for macOS universal framework build.

    This function serves as the primary entry point when building
    distributable macOS frameworks with universal binary support.

    Args:
        target_option: Additional CMake target options (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Note:
        This function calls build_macos() to create the universal framework,
        then archives it and moves artifacts to target/macos/ directory.
        For Xcode project generation, use gen_macos_project() instead.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    print(f"main link_type: {link_type}, jobs: {jobs}")

    # Build universal framework
    if not build_macos(target_option, link_type, jobs):
        print("ERROR: macOS build failed")
        sys.exit(1)

    # Archive and organize artifacts
    archive_macos_project(link_type=link_type)
    print_build_results(link_type=link_type)


# Command-line interface for macOS builds
#
# Usage:
#   python build_macos.py                    # Build static library (default)
#   python build_macos.py --ide-project      # Generate Xcode project
#   python build_macos.py -j 8               # Build with 8 parallel jobs
#   python build_macos.py --link-type shared # Build shared library
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build macOS universal framework",
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
        gen_macos_project()
    else:
        main(link_type=args.link_type, jobs=args.jobs)
