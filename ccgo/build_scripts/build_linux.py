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

# Build output paths (base path, actual paths include link_type subdirectory)
BUILD_OUT_PATH_BASE = "cmake_build/Linux"


def get_build_out_path(link_type):
    """Get build output path for specified link type."""
    return f"{BUILD_OUT_PATH_BASE}/{link_type}"


def get_install_path(link_type):
    """Get install path for specified link type."""
    return f"{BUILD_OUT_PATH_BASE}/{link_type}/Linux.out"


# CMake build command for Linux Release configuration
# Uses Unix Makefiles generator with parallel build
# Parameters: ccgo_cmake_dir, link_type_flags, jobs
# Note: Uses ../../.. because build directory is now cmake_build/Linux/{link_type}/
BUILD_CMD = 'cmake ../../.. -DCMAKE_BUILD_TYPE=Release -DCCGO_CMAKE_DIR="%s" %s && make -j%d && make install'

# CodeLite IDE project generation command
# CodeLite is a lightweight, cross-platform C/C++ IDE
# Note: IDE project uses static directory by default
GEN_PROJECT_CMD = 'cmake ../../.. -G "CodeLite - Unix Makefiles" -DCCGO_CMAKE_DIR="%s"'


def _build_linux_single(target_option, single_link_type, jobs):
    """
    Build Linux library for a single link type (static or shared).

    This internal function handles the actual build for one link type.

    Args:
        target_option: Additional CMake target options
        single_link_type: Either 'static' or 'shared'
        jobs: Number of parallel build jobs
    """
    build_out_path = get_build_out_path(single_link_type)
    install_path = get_install_path(single_link_type)

    print(f"\n==================build_linux ({single_link_type}, jobs: {jobs})========================")

    # Set link type CMake flags
    if single_link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    else:  # shared
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"

    # Build command with link_type_flags and jobs
    build_cmd = BUILD_CMD % (CCGO_CMAKE_DIR, link_type_flags, jobs)

    clean(build_out_path)
    os.chdir(build_out_path)

    ret = os.system(build_cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build fail!!!!!!!!!!!!!!!")
        print("ERROR: Native build failed. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure

    # Dynamically find the actual install directory (could be Darwin.out, Linux.out, etc.)
    # This is needed because CMAKE_SYSTEM_NAME varies by host OS
    actual_install_path = None
    for out_dir in glob.glob(build_out_path + "/*.out"):
        if single_link_type == 'static':
            # Check for static library in static/ subdirectory (new CMake output location)
            # or root directory (fallback)
            if glob.glob(out_dir + "/static/*.a") or glob.glob(out_dir + "/*.a"):
                actual_install_path = out_dir
                print(f"Found install directory: {actual_install_path}")
                break
        else:  # shared
            if glob.glob(out_dir + "/shared/*.so") or glob.glob(out_dir + "/*.so"):
                actual_install_path = out_dir
                print(f"Found install directory: {actual_install_path}")
                break

    if not actual_install_path:
        # Fallback to default install_path
        actual_install_path = install_path
        print(f"Warning: No library files found, using default: {actual_install_path}")

    if single_link_type == 'static':
        # Merge static libs - check static/ subdirectory first (new CMake output location)
        libtool_src_libs = glob.glob(actual_install_path + "/static/*.a")
        if not libtool_src_libs:
            # Fallback to root directory
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
    else:  # shared
        # Check for shared library
        shared_lib_path = os.path.join(actual_install_path, "shared", f"lib{PROJECT_NAME_LOWER}.so")
        if not os.path.exists(shared_lib_path):
            shared_lib_path = os.path.join(actual_install_path, f"lib{PROJECT_NAME_LOWER}.so")

        if os.path.exists(shared_lib_path):
            print("\n==================Verifying Built Shared Library========================")
            if not check_build_libraries(shared_lib_path, platform_hint="linux"):
                print("ERROR: Shared library verification failed!")
                sys.exit(1)
            print("==================Output========================")
            print(shared_lib_path)
        else:
            print(f"Warning: Shared library not found at expected location")


def build_linux(target_option="", link_type='both', jobs=None):
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
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Static library: Linux.out/{project}.dir/{project}.a
        - Shared library: Linux.out/{project}.dir/{project}.so
        - Headers: Linux.out/{project}.dir/include/

    Note:
        The .a file is an archive containing merged static libraries.
        The .so file is a shared library.
        On Linux, the ar tool is used for merging (similar to libtool on macOS).
        The resulting library can be linked into applications using -l flag.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    before_time = time.time()
    print(f"==================build_linux (link_type: {link_type}, jobs: {jobs})========================")

    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        platform="linux",
    )

    # Build for each link type separately to avoid overwriting
    if link_type == 'both':
        _build_linux_single(target_option, 'static', jobs)
        _build_linux_single(target_option, 'shared', jobs)
    else:
        _build_linux_single(target_option, link_type, jobs)


def gen_linux_project(target_option=""):
    """
    Generate CodeLite project for Linux development and debugging.

    This function creates a CodeLite workspace and project files that can be
    opened in CodeLite IDE for interactive development, debugging, and testing.
    The project is automatically opened in CodeLite after generation.

    Args:
        target_option: Additional CMake target options (default: '')

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - CodeLite workspace: cmake_build/Linux/static/{project}.workspace (auto-opened)

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
        platform="linux",
    )
    # Use static directory for IDE project
    build_out_path = get_build_out_path("static")
    clean(build_out_path)
    os.chdir(build_out_path)

    cmd = GEN_PROJECT_CMD % CCGO_CMAKE_DIR
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


def archive_linux_project(link_type='both'):
    """
    Archive Linux library and related build artifacts with unified structure.

    This function creates two archive packages:
    1. Main package: {PROJECT_NAME}_LINUX_SDK-{version}.zip
       - lib/static/lib{project}.a  (if link_type is static or both)
       - lib/shared/lib{project}.so (if link_type is shared or both)
       - include/{project}/
       - build_info.json
    2. Symbols package: {PROJECT_NAME}_LINUX_SDK-{version}-SYMBOLS.zip
       - obj/linux/lib{project}.so (unstripped shared library)

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')

    Output:
        - target/{PROJECT_NAME}_LINUX_SDK-{version}.zip
        - target/{PROJECT_NAME}_LINUX_SDK-{version}-SYMBOLS.zip (if shared libs exist)
    """
    print("==================Archive Linux Project========================")

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(SCRIPT_PATH)

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Get install paths for both link types
    static_install_path = os.path.join(SCRIPT_PATH, get_install_path("static"))
    shared_install_path = os.path.join(SCRIPT_PATH, get_install_path("shared"))

    # Find actual static install directory
    static_actual_install_path = None
    static_build_out_path = os.path.join(SCRIPT_PATH, get_build_out_path("static"))
    for out_dir in glob.glob(static_build_out_path + "/*.out"):
        lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
        test_lib_dir = os.path.join(out_dir, lib_dir_name)
        test_lib_file = os.path.join(test_lib_dir, f"{PROJECT_NAME_LOWER}.a")
        if os.path.exists(test_lib_file):
            static_actual_install_path = out_dir
            print(f"Found static install directory: {static_actual_install_path}")
            break
    if not static_actual_install_path:
        static_actual_install_path = static_install_path

    # Find actual shared install directory
    shared_actual_install_path = None
    shared_build_out_path = os.path.join(SCRIPT_PATH, get_build_out_path("shared"))
    for out_dir in glob.glob(shared_build_out_path + "/*.out"):
        # Check for shared library in shared/ subdirectory or root
        if glob.glob(out_dir + "/shared/*.so") or glob.glob(out_dir + "/*.so"):
            shared_actual_install_path = out_dir
            print(f"Found shared install directory: {shared_actual_install_path}")
            break
    if not shared_actual_install_path:
        shared_actual_install_path = shared_install_path

    # Prepare static libraries mapping
    static_libs = {}
    if link_type in ('static', 'both'):
        lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
        lib_dir_src = os.path.join(static_actual_install_path, lib_dir_name)
        static_lib_path = os.path.join(lib_dir_src, f"{PROJECT_NAME_LOWER}.a")
        if os.path.exists(static_lib_path):
            arc_path = get_unified_lib_path("static", lib_name=f"lib{PROJECT_NAME_LOWER}.a", platform="linux")
            static_libs[arc_path] = static_lib_path
        else:
            print(f"WARNING: Static library not found at {static_lib_path}")

    # Prepare shared libraries mapping
    shared_libs = {}
    if link_type in ('shared', 'both'):
        # Check in shared/ subdirectory first (new CMake output location)
        shared_lib_path = os.path.join(shared_actual_install_path, "shared", f"lib{PROJECT_NAME_LOWER}.so")
        if not os.path.exists(shared_lib_path):
            # Fallback to root install path
            shared_lib_path = os.path.join(shared_actual_install_path, f"lib{PROJECT_NAME_LOWER}.so")
        if os.path.exists(shared_lib_path):
            arc_path = get_unified_lib_path("shared", lib_name=f"lib{PROJECT_NAME_LOWER}.so", platform="linux")
            shared_libs[arc_path] = shared_lib_path
        else:
            print(f"WARNING: Shared library not found")

    # Prepare include directories mapping (use project's include/ directory)
    include_dirs = {}
    headers_src = os.path.join(SCRIPT_PATH, "include")
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(PROJECT_NAME_LOWER, headers_src)
        include_dirs[arc_path] = headers_src

    # Prepare symbols (unstripped shared library)
    obj_files = {}
    if link_type in ('shared', 'both'):
        # Look for unstripped shared library
        unstripped_so = os.path.join(shared_actual_install_path, "shared", f"lib{PROJECT_NAME_LOWER}.so")
        if not os.path.exists(unstripped_so):
            unstripped_so = os.path.join(shared_actual_install_path, f"lib{PROJECT_NAME_LOWER}.so")
        if os.path.exists(unstripped_so):
            arc_path = get_unified_obj_path("linux", f"lib{PROJECT_NAME_LOWER}.so")
            obj_files[arc_path] = unstripped_so

    # Create unified archive packages
    main_zip_path, symbols_zip_path = create_unified_archive(
        output_dir=bin_dir,
        project_name=PROJECT_NAME,
        platform_name="LINUX",
        version=full_version,
        link_type=link_type,
        static_libs=static_libs,
        shared_libs=shared_libs,
        include_dirs=include_dirs,
        obj_files=obj_files if obj_files else None,
    )

    print("==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    if symbols_zip_path:
        print(f"Symbols package: {symbols_zip_path}")


def print_build_results(link_type='both'):
    """
    Print Linux build results from target directory.

    This function displays the build artifacts and moves them to target/linux/:
    1. Main ZIP archive ({PROJECT_NAME}_LINUX_SDK-{version}.zip)
    2. Symbols package ({PROJECT_NAME}_LINUX_SDK-{version}-SYMBOLS.zip)
    3. build_info.json

    Args:
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
    """
    print("==================Linux Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for build artifacts
    # Main package: {PROJECT_NAME}_LINUX_SDK-*.zip (not ending with -SYMBOLS.zip)
    main_zips = [
        f for f in glob.glob(f"{bin_dir}/*_LINUX_SDK-*.zip")
        if not f.endswith("-SYMBOLS.zip")
    ]

    # Symbols package: {PROJECT_NAME}_LINUX_SDK-*-SYMBOLS.zip
    symbols_zips = [
        f for f in glob.glob(f"{bin_dir}/*_LINUX_SDK-*-SYMBOLS.zip")
    ]

    if not main_zips:
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

    for symbols_zip in symbols_zips:
        dest = os.path.join(bin_linux_dir, os.path.basename(symbols_zip))
        if os.path.exists(dest):
            os.remove(dest)
        shutil.move(symbols_zip, dest)
        artifacts_moved.append(os.path.basename(symbols_zip))

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


def main(target_option="", link_type='both', jobs=None):
    """
    Main entry point for Linux library build.

    This function serves as the primary entry point when building
    distributable Linux libraries.

    Args:
        target_option: Additional CMake target options (default: '')
        link_type: Library link type ('static', 'shared', or 'both', default: 'both')
        jobs: Number of parallel build jobs (default: CPU count)

    Note:
        This function calls build_linux() to create the static library,
        then archives it and moves artifacts to target/linux/ directory.
        For CodeLite project generation, use gen_linux_project() instead.
    """
    # Determine number of parallel jobs
    if jobs is None or jobs <= 0:
        jobs = multiprocessing.cpu_count()

    print(f"main link_type: {link_type}, jobs: {jobs}")

    # Clean target/linux directory at the start of build
    # Note: build_info.json will be regenerated at the end of the build process
    target_linux_dir = os.path.join(SCRIPT_PATH, "target/linux")
    if os.path.exists(target_linux_dir):
        shutil.rmtree(target_linux_dir)
        print(f"[CLEAN] Removed target/linux directory")

    # Build library
    build_linux(target_option=target_option, link_type=link_type, jobs=jobs)

    # Archive and organize artifacts
    archive_linux_project(link_type=link_type)
    print_build_results(link_type=link_type)


# Command-line interface for Linux builds
#
# Usage:
#   python build_linux.py                    # Build static library (default)
#   python build_linux.py --ide-project      # Generate CodeLite project
#   python build_linux.py -j 8               # Build with 8 parallel jobs
#   python build_linux.py --link-type shared # Build shared library
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build Linux static library",
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
        help="Generate CodeLite project instead of building",
    )

    args = parser.parse_args()

    if args.ide_project:
        gen_linux_project()
    else:
        main(link_type=args.link_type, jobs=args.jobs)
