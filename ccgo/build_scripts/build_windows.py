#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_windows.py
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
# substantial portions of the Softwaree

"""
Windows native library build script.

This script builds static libraries (.lib) for Windows platform using CMake
and Visual Studio toolchain. It handles:
- Building with Visual Studio 2019 (v142 toolset)
- Merging multiple static libraries into single .lib
- Collecting and packaging PDB debug symbols
- Header file organization
- Visual Studio project generation
- Support for both Release and Debug configurations

Requirements:
- Visual Studio 2019 or later
- CMake 3.10 or later
- Python 3.7+
- Windows development environment

Usage:
    python3 build_windows.py [mode]

    mode: 1 (build Release), 2 (generate VS project), 3 (build Debug), 4 (exit)

Output:
    - Static library: cmake_build/Windows/Windows.out/{project}.dir/x64/{project}.lib
    - Debug symbols: cmake_build/Windows/Windows.out/{project}.dir/x64/{project}.pdb.zip
    - Headers: cmake_build/Windows/Windows.out/{project}.dir/x64/include/
"""

import os
import sys
import glob
import time
import shutil
import platform

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

# Script configuration
SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils
# which loads them from CCGO.toml configuration file

# Build output paths
BUILD_OUT_PATH = "cmake_build/Windows"
INSTALL_PATH = BUILD_OUT_PATH + "/Windows.out/"

# Visual Studio 2019 build configuration
# Uses Visual Studio 16 2019 generator with v142 platform toolset (C++17 support)
WIN_BUILD_CMD = 'cmake ../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="%s" && cmake --build . --target install --config %s'
WIN_GEN_PROJECT_CMD = (
    'cmake ../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="%s"'
)
WIN_ARCH = "x64"  # Target architecture (64-bit Windows)
WIN_SRC_DIR = "src"  # Source directory name for PDB collection
THIRD_PARTY_MERGE_LIBS = ["pthread"]  # Third-party libraries to merge into final .lib


def build_windows(incremental, tag="", config="Release", link_type='static', use_mingw=False, use_msvc_compat=False):
    """
    Build Windows static library with Visual Studio or MinGW toolchain.

    This function performs the complete Windows build process:
    1. Generates version info header file
    2. Cleans build directory (unless incremental build)
    3. Configures and builds with Visual Studio 2019 (v142 toolset) OR MinGW-w64
    4. Merges multiple static libraries into single .lib file
    5. Copies header files to include directory
    6. Collects PDB debug symbol files (Visual Studio only)
    7. Packages PDB files into zip archive

    Args:
        incremental: If True, skip clean step for faster rebuilds
        tag: Version tag string for metadata (default: '')
        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
        link_type: Library link type ('static', 'shared', or 'both', default: 'static')

    Returns:
        bool: True if build succeeded, False otherwise

    Output:
        - Static library: Windows.out/{project}.dir/x64/{project}.lib
        - Debug symbols: Windows.out/{project}.dir/x64/{project}.pdb.zip
        - Headers: Windows.out/{project}.dir/x64/include/

    Note:
        PDB files are essential for debugging crashes in production.
        Always preserve the .pdb.zip file for release builds.
        The lib file contains merged static libraries from both
        project sources and third-party dependencies.
    """
    before_time = time.time()
    print(f"==================build_windows (link_type: {link_type})========================")
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        incremental=incremental,
        platform="windows",
    )

    # Add link type CMake flags
    link_type_flags = ""
    if link_type == 'static':
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=OFF"
    elif link_type == 'shared':
        link_type_flags = "-DCCGO_BUILD_STATIC=OFF -DCCGO_BUILD_SHARED=ON"
    else:  # both
        link_type_flags = "-DCCGO_BUILD_STATIC=ON -DCCGO_BUILD_SHARED=ON"

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    if use_mingw:
        # MinGW cross-compilation (for Docker/Linux environments)
        # Use Unix Makefiles generator with MinGW compilers
        cmake_config_cmd = (
            f'cmake ../.. '
            f'-G "Unix Makefiles" '
            f'-DCMAKE_SYSTEM_NAME=Windows '
            f'-DCMAKE_C_COMPILER=x86_64-w64-mingw32-gcc '
            f'-DCMAKE_CXX_COMPILER=x86_64-w64-mingw32-g++ '
            f'-DCMAKE_RC_COMPILER=x86_64-w64-mingw32-windres '
            f'-DCMAKE_FIND_ROOT_PATH=/usr/x86_64-w64-mingw32 '
            f'-DCMAKE_FIND_ROOT_PATH_MODE_PROGRAM=NEVER '
            f'-DCMAKE_FIND_ROOT_PATH_MODE_LIBRARY=ONLY '
            f'-DCMAKE_FIND_ROOT_PATH_MODE_INCLUDE=ONLY '
            f'-DCMAKE_BUILD_TYPE={config} '
            f'-DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags}'
        )
        cmake_build_cmd = 'cmake --build . --target install'
        cmd = f'{cmake_config_cmd} && {cmake_build_cmd}'
    elif use_msvc_compat:
        # MSVC-compatible build using clang-cl (for Docker with MSVC ABI)
        # Use Ninja generator for better cross-platform support
        cmake_config_cmd = (
            f'cmake ../.. '
            f'-G "Ninja" '
            f'-DCMAKE_SYSTEM_NAME=Windows '
            f'-DCMAKE_C_COMPILER=clang-cl '
            f'-DCMAKE_CXX_COMPILER=clang-cl '
            f'-DCMAKE_AR=llvm-lib '
            f'-DCMAKE_LINKER=lld-link '
            f'-DCMAKE_MT=llvm-mt '
            f'-DCMAKE_RC_COMPILER=llvm-rc '
            f'-DCMAKE_BUILD_TYPE={config} '
            f'-DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags}'
        )
        # Set environment for Windows SDK paths if available
        if os.environ.get('INCLUDE'):
            cmake_config_cmd += f' -DCMAKE_C_FLAGS="/I{os.environ["INCLUDE"]}"'
            cmake_config_cmd += f' -DCMAKE_CXX_FLAGS="/I{os.environ["INCLUDE"]}"'
        cmake_build_cmd = 'cmake --build . --target install'
        cmd = f'{cmake_config_cmd} && {cmake_build_cmd}'
    else:
        # Visual Studio build (native Windows)
        cmd = f'cmake ../.. -G "Visual Studio 16 2019" -T v142 -DCCGO_CMAKE_DIR="{CCGO_CMAKE_DIR}" {link_type_flags} && cmake --build . --target install --config {config}'

    print("build cmd:" + cmd)
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!build fail!!!!!!!!!!!!!!!!!!!!")
        print("ERROR: Native build failed for Windows. Stopping immediately.")
        sys.exit(1)  # Exit immediately on build failure

    # Create result directory without architecture subdirectory (x64)
    # Windows only supports x64, so no need for architecture-specific subdirectories
    win_result_dir = INSTALL_PATH + f"{PROJECT_NAME_LOWER}.dir"
    if os.path.exists(win_result_dir):
        shutil.rmtree(win_result_dir)
    os.makedirs(win_result_dir)

    needed_libs = glob.glob(INSTALL_PATH + "*.lib")

    for other_lib in THIRD_PARTY_MERGE_LIBS:
        temp_libs_path = (
            SCRIPT_PATH + f"/third_party/{other_lib}/lib/windows/{WIN_ARCH}/"
        )
        temp_libs = glob.glob(temp_libs_path + "*.lib")
        needed_libs.extend(temp_libs)

    filtered_lib_names = list(
        map(lambda x: os.path.splitext(os.path.basename(x))[0], needed_libs)
    )

    print(f"build merge libs: {needed_libs}")

    if use_mingw:
        # MinGW builds: Merge multiple .a files into a single library
        # MinGW produces .a files, not .lib files
        # Merge all .a files from INSTALL_PATH
        mingw_libs = glob.glob(INSTALL_PATH + "*.a")

        # Add third-party libraries to merge list
        for other_lib in THIRD_PARTY_MERGE_LIBS:
            temp_libs_path = (
                SCRIPT_PATH + f"/third_party/{other_lib}/lib/windows/{WIN_ARCH}/"
            )
            temp_libs = glob.glob(temp_libs_path + "*.a")
            mingw_libs.extend(temp_libs)

        print(f"MinGW: Merging libraries: {mingw_libs}")

        # Merge all .a files into a single library using MinGW-specific merge
        output_lib = win_result_dir + f"/{PROJECT_NAME_LOWER}.a"
        if not merge_mingw_static_libs(mingw_libs, output_lib):
            print("!!!!!!!!!!!!!!!!!!merge MinGW libs fail!!!!!!!!!!!!!!!!!!!!")
            return False

        print(f"MinGW: Merged library created: {output_lib}")
    else:
        # Visual Studio builds: Merge multiple .lib files into one
        if not merge_win_static_libs(
            needed_libs, win_result_dir + f"/{PROJECT_NAME_LOWER}.lib"
        ):
            print("!!!!!!!!!!!!!!!!!!merge libs fail!!!!!!!!!!!!!!!!!!!!")
            print("ERROR: Failed to merge Windows libraries. Stopping immediately.")
            sys.exit(1)  # Exit immediately on merge failure

    headers = dict()
    headers.update(WINDOWS_BUILD_COPY_HEADER_FILES)
    copy_file_mapping(headers, "./", win_result_dir + "/include")

    if use_mingw:
        # MinGW doesn't generate PDB files (uses DWARF debug info instead)
        print("MinGW build: Skipping PDB collection (not applicable)")
    else:
        # Visual Studio builds: Copy PDB debug symbol files
        sub_folders = filtered_lib_names
        # copy pdb of third_party
        copy_windows_pdb(BUILD_OUT_PATH, sub_folders, config, INSTALL_PATH)
        src_dir_folder = PROJECT_NAME_LOWER + "-" + WIN_SRC_DIR
        # copy pdb of src
        sub_folders = list(
            map(lambda x: x.replace(PROJECT_NAME_LOWER, src_dir_folder), sub_folders)
        )
        copy_windows_pdb(
            os.path.join(BUILD_OUT_PATH, src_dir_folder), sub_folders, config, INSTALL_PATH
        )

    # zip pdb files (Visual Studio only)
    if not use_mingw:
        pdf_suffix = ".pdb"
        zip_files_ends_with(
            INSTALL_PATH,
            pdf_suffix,
            win_result_dir + f"/{PROJECT_NAME_LOWER}{pdf_suffix}.zip",
        )

    # Check the built library architecture
    print("\n==================Verifying Windows Library========================")
    if use_mingw:
        # MinGW builds .a files
        final_lib = os.path.join(win_result_dir, f"{PROJECT_NAME_LOWER}.a")
    else:
        # MSVC builds .lib files
        final_lib = os.path.join(win_result_dir, f"{PROJECT_NAME_LOWER}.lib")

    if os.path.exists(final_lib):
        check_library_architecture(final_lib, platform_hint="windows")
    else:
        # Try to find any .lib or .a file
        lib_files = glob.glob(os.path.join(win_result_dir, "*.lib")) + glob.glob(os.path.join(win_result_dir, "*.a"))
        if lib_files:
            check_library_architecture(lib_files[0], platform_hint="windows")
    print("====================================================================")

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"libs: {win_result_dir}")
    if not use_mingw:
        print(f"pdb files: {INSTALL_PATH}")

    after_time = time.time()
    print(f"use time: {int(after_time - before_time)} s")
    return True


def gen_win_project(tag="", config="Release"):
    """
    Generate Visual Studio project for Windows development and debugging.

    This function creates a Visual Studio solution (.sln) and project files
    that can be opened in Visual Studio for interactive development, debugging,
    and testing. The project is automatically opened in Visual Studio after generation.

    Args:
        tag: Version tag string for metadata (default: '')
        config: Build configuration - 'Release' or 'Debug' (default: 'Release')
              Note: This parameter is currently unused but reserved for future use

    Returns:
        bool: True if project generation succeeded, False otherwise

    Output:
        - VS solution: cmake_build/Windows/{project}.sln (auto-opened)

    Note:
        The generated Visual Studio project uses the v142 platform toolset.
        This is useful for development workflows where you need Visual Studio's
        debugging tools, IntelliSense, and build system integration.
        The project file is automatically opened in Visual Studio after generation.
    """
    before_time = time.time()
    # Generate version info header file
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="windows",
    )
    clean(BUILD_OUT_PATH, False)
    os.chdir(BUILD_OUT_PATH)
    ret = os.system(WIN_GEN_PROJECT_CMD % CCGO_CMAKE_DIR)
    os.chdir(SCRIPT_PATH)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")

    if 0 != ret:
        print("!!!!!!!!!!!!!!!!!!gen project file fail!!!!!!!!!!!!!!!!!!!!")
        return False

    project_file_prefix = os.path.join(SCRIPT_PATH, BUILD_OUT_PATH, PROJECT_NAME_LOWER)
    project_file = get_project_file_name(project_file_prefix)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {project_file}")

    os.system(get_open_project_file_cmd(project_file))

    return True


def archive_windows_project():
    """
    Archive Windows static library and related build artifacts.

    This function creates two archive packages (matching Linux packaging style):
    1. Main package: {PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip
       - Contains library with simplified structure: {project}.libdir/{project}.lib
    2. Archive package: (ARCHIVE)_{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip
       - Contains full library for debugging (includes version info)

    Output:
        - target/windows/{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip
        - target/windows/(ARCHIVE)_{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip
    """
    import zipfile
    from pathlib import Path

    print("==================Archive Windows Project========================")

    # Get project version info
    version_name = get_version_name(SCRIPT_PATH)
    project_name_upper = PROJECT_NAME.upper()

    # Try to get publish suffix from git tags or use beta.0 as default
    try:
        git_tags = os.popen("git describe --tags --abbrev=0 2>nul").read().strip()
        if git_tags and "-" in git_tags:
            suffix = git_tags.split("-", 1)[1]
        else:
            git_branch = (
                os.popen("git rev-parse --abbrev-ref HEAD 2>nul").read().strip()
            )
            if git_branch == "master" or git_branch == "main":
                suffix = "release"
            else:
                suffix = "beta.0"
    except:
        suffix = "beta.0"

    # Build full version name with suffix
    full_version = f"{version_name}-{suffix}" if suffix else version_name

    # Define paths - create artifacts directly in target/windows/
    target_dir = os.path.join(SCRIPT_PATH, "target")
    bin_dir = os.path.join(target_dir, "windows")
    windows_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create target/windows directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find source library directory
    lib_dir_name = f"{PROJECT_NAME_LOWER}.dir"
    lib_dir_src = os.path.join(windows_install_path, lib_dir_name)

    if not os.path.exists(lib_dir_src):
        print(f"WARNING: Library directory not found at {lib_dir_src}")
        return

    # Create temporary .libdir directory for packaging
    temp_lib_dir = os.path.join(bin_dir, f"{PROJECT_NAME_LOWER}.libdir")
    if os.path.exists(temp_lib_dir):
        shutil.rmtree(temp_lib_dir)
    shutil.copytree(lib_dir_src, temp_lib_dir)
    print(f"Prepared library directory: {temp_lib_dir}")

    # Rename .a file to .lib for Windows standard (if exists)
    for root, dirs, files in os.walk(temp_lib_dir):
        for file in files:
            if file.endswith('.a'):
                old_path = os.path.join(root, file)
                new_path = os.path.join(root, file[:-2] + '.lib')
                shutil.move(old_path, new_path)
                print(f"Renamed: {file} -> {os.path.basename(new_path)}")

    # Create main ZIP archive with simplified structure
    main_zip_name = f"{project_name_upper}_WINDOWS_SDK-{full_version}.zip"
    main_zip_path = os.path.join(bin_dir, main_zip_name)

    print(f"Creating main ZIP archive: {main_zip_name}")
    with zipfile.ZipFile(main_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(temp_lib_dir):
            for file in files:
                file_path = os.path.join(root, file)
                # Use .libdir suffix to distinguish directory from library file
                arcname = os.path.join(
                    f"{PROJECT_NAME_LOWER}.libdir",
                    os.path.relpath(file_path, temp_lib_dir)
                )
                zipf.write(file_path, arcname)

    print(f"Created main archive: {main_zip_path}")

    # Create archive package with full library (includes version info)
    archive_zip_name = f"(ARCHIVE)_{project_name_upper}_WINDOWS_SDK-{full_version}.zip"
    archive_zip_path = os.path.join(bin_dir, archive_zip_name)

    print(f"Creating archive package: {archive_zip_name}")
    with zipfile.ZipFile(archive_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        # Find the library file (should be .lib after renaming)
        lib_files_found = []
        for root, dirs, files in os.walk(temp_lib_dir):
            for file in files:
                if file.endswith('.lib') or file.endswith('.a'):
                    file_path = os.path.join(root, file)
                    arcname = os.path.join(
                        f"{PROJECT_NAME_LOWER}.libdir",
                        os.path.relpath(file_path, temp_lib_dir)
                    )
                    zipf.write(file_path, arcname)
                    lib_files_found.append(arcname)
                    print(f"Added library: {arcname}")

        # Also include headers and other subdirectories
        for subdir in ["include", "Headers"]:
            subdir_path = os.path.join(temp_lib_dir, subdir)
            if os.path.exists(subdir_path):
                for root, dirs, files in os.walk(subdir_path):
                    for file in files:
                        file_path = os.path.join(root, file)
                        arcname = os.path.join(
                            f"{PROJECT_NAME_LOWER}.libdir",
                            os.path.relpath(file_path, temp_lib_dir)
                        )
                        zipf.write(file_path, arcname)

        if not lib_files_found:
            print("WARNING: No library files found for archive package")

    print(f"Created archive package: {archive_zip_path}")

    # Remove temporary .libdir directory after zipping
    shutil.rmtree(temp_lib_dir)
    print(f"Removed temporary directory: {temp_lib_dir}")

    print("==================Archive Complete========================")
    print(f"Main package: {main_zip_path}")
    print(f"Archive package: {archive_zip_path}")


def print_build_results():
    """
    Print Windows build results from target/windows directory.

    This function displays the build artifacts:
    1. Main ZIP archive ({PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip)
    2. Archive package ((ARCHIVE)_{PROJECT_NAME}_WINDOWS_SDK-{version}-{suffix}.zip)
    3. build_info.json
    """
    print("==================Windows Build Results========================")

    # Define paths - artifacts are already in target/windows/
    bin_windows_dir = os.path.join(SCRIPT_PATH, "target", "windows")

    # Check if target/windows directory exists
    if not os.path.exists(bin_windows_dir):
        print(f"ERROR: target/windows directory not found. Please run build first.")
        sys.exit(1)

    # Main package: {PROJECT_NAME}_WINDOWS_SDK-*.zip (not starting with (ARCHIVE)_)
    main_zips = [
        f for f in glob.glob(f"{bin_windows_dir}/*_WINDOWS_SDK-*.zip")
        if not os.path.basename(f).startswith("(ARCHIVE)_")
    ]

    # Archive package: (ARCHIVE)_{PROJECT_NAME}_WINDOWS_SDK-*.zip
    archive_zips = [
        f for f in glob.glob(f"{bin_windows_dir}/(ARCHIVE)_*_WINDOWS_SDK-*.zip")
    ]

    if not main_zips and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_windows_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # List artifacts (no need to move - already in correct location)
    artifacts_found = main_zips + archive_zips
    if artifacts_found:
        print(f"[SUCCESS] Found {len(artifacts_found)} artifact(s) in target/windows/")

    # Copy build_info.json from cmake_build to target/windows
    copy_build_info_to_target("windows", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/windows/:")
    print("-" * 60)

    # List all files in target/windows directory with sizes
    for item in sorted(os.listdir(bin_windows_dir)):
        item_path = os.path.join(bin_windows_dir, item)
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


def main():
    """
    Main entry point for Windows build and project generation.

    This function validates the Visual Studio environment and provides
    an interactive or command-line interface for building libraries
    and generating Visual Studio projects.

    Raises:
        Returns early if Visual Studio environment check fails

    Build Options:
        1 - Build Release configuration with merged static library
        2 - Generate Visual Studio project and open in IDE
        3 - Build Debug configuration with debug symbols
        4 - Exit without action

    Note:
        Requires Visual Studio 2019 or later to be installed, OR MinGW-w64
        for cross-compilation from Linux/macOS (Docker containers).
        The VS environment check ensures required tools are available.
    """
    # Check toolchain availability
    mingw_available = shutil.which("x86_64-w64-mingw32-gcc") is not None
    clang_cl_available = shutil.which("clang-cl") is not None or shutil.which("cl") is not None
    vs_available = check_vs_env()

    # Determine which toolchain to use based on environment and explicit selection
    use_mingw = False
    use_msvc_compat = False

    # Check for explicit toolchain selection via environment variable
    # This can be set by Docker or command line
    toolchain_env = os.environ.get("CCGO_WINDOWS_TOOLCHAIN", "").lower()

    if toolchain_env == "msvc":
        # Explicit MSVC request - use clang-cl if available (Docker), otherwise native VS
        if clang_cl_available:
            use_msvc_compat = True
            print("ℹ Using MSVC-compatible toolchain (clang-cl)")
        elif vs_available:
            use_msvc_compat = False  # Use native Visual Studio
            print("ℹ Using native Visual Studio toolchain")
        else:
            print("ERROR: MSVC toolchain requested but not available")
            return
    elif toolchain_env in ["gnu", "mingw"]:
        # Explicit MinGW request
        if mingw_available:
            use_mingw = True
            print("ℹ Using MinGW toolchain")
        else:
            print("ERROR: MinGW toolchain requested but not available")
            return
    else:
        # Auto-detect: prefer MinGW in Docker/Linux, VS on Windows
        if mingw_available:
            use_mingw = True
            print("ℹ Auto-detected MinGW toolchain")
        elif clang_cl_available:
            use_msvc_compat = True
            print("ℹ Auto-detected MSVC-compatible toolchain (clang-cl)")
        elif vs_available:
            use_msvc_compat = False
            print("ℹ Auto-detected Visual Studio toolchain")
        else:
            print("ERROR: No compatible Windows toolchain found")
            print("Please install MinGW-w64, Visual Studio, or run in Docker")
            return

    # Command-line interface for Windows builds
    # Supports two invocation modes:
    # 1. Interactive mode (no args): Prompts user for build mode
    # 2. Mode only (1 arg): Uses specified mode directly
    #
    # Build modes:
    # 1 - Build Release: Builds static library in Release configuration
    # 2 - Generate VS project: Creates .sln and opens in Visual Studio
    # 3 - Build Debug: Builds static library in Debug configuration with full symbols
    # 4 - Exit: Quit without building
    while True:
        if len(sys.argv) >= 2:
            num = sys.argv[1]
        else:
            num = input(
                "Enter menu:"
                + f"\n1. Clean && build {PROJECT_NAME_LOWER} Release."
                + f"\n2. Gen Project {PROJECT_NAME_LOWER} file."
                + f"\n3. Clean && build {PROJECT_NAME_LOWER} Debug."
                + "\n4. Exit\n"
            )
        print(f"==================Windows Choose num: {num}==================")
        if num == "1":
            build_windows(incremental=False, tag=num, config="Release",
                        use_mingw=use_mingw, use_msvc_compat=use_msvc_compat)
            # Archive and organize artifacts
            archive_windows_project()
            print_build_results()
            break
        elif num == "2":
            if use_mingw or use_msvc_compat:
                print("WARNING: Project generation not supported with MinGW/clang-cl")
                print("Using cross-compilation instead")
                build_windows(incremental=False, tag=num, config="Release",
                            use_mingw=use_mingw, use_msvc_compat=use_msvc_compat)
                archive_windows_project()
                print_build_results()
            else:
                gen_win_project(tag=num, config="Release")
            break
        elif num == "3":
            build_windows(incremental=False, tag=num, config="Debug",
                        use_mingw=use_mingw, use_msvc_compat=use_msvc_compat)
            # Archive and organize artifacts
            archive_windows_project()
            print_build_results()
            break
        elif num == "4":
            break
        else:
            break


if __name__ == "__main__":
    main()
