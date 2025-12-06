#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_include.py
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

import glob
import os
import sys
import time
import platform
import shutil

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils.py (reads from CCGO.toml)
PROJECT_RELATIVE_PATH = PROJECT_NAME_LOWER

BUILD_OUT_PATH = "cmake_build/Include"
CMAKE_SYSTEM_NAME = platform.system()
INSTALL_PATH = BUILD_OUT_PATH + "/" + CMAKE_SYSTEM_NAME + ".out"


def build_include(incremental):
    before_time = time.time()
    print(
        f"==================build include install path: {INSTALL_PATH} ========================"
    )

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        incremental=incremental,
        platform="include",
    )
    clean(BUILD_OUT_PATH, incremental)

    if os.path.exists(INSTALL_PATH):
        shutil.rmtree(INSTALL_PATH)

    os.chdir(SCRIPT_PATH)
    copy_file_mapping(INCLUDE_BUILD_COPY_HEADER_FILES, "./", INSTALL_PATH + "/include")

    dst_target_path = INSTALL_PATH

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_target_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def archive_include_project():
    """
    Archive include headers and related build artifacts.

    This function creates an archive package containing:
    1. Header files from include directory

    The archive is packaged into a ZIP file named:
    ARCHIVE_{PROJECT_NAME}_INCLUDE-{version}-{suffix}.zip

    Output:
        - target/include/{PROJECT_NAME}_INCLUDE-{version}-{suffix}/
        - target/include/ARCHIVE_{PROJECT_NAME}_INCLUDE-{version}-{suffix}.zip
    """
    import zipfile
    from pathlib import Path

    print("==================Archive Include Project========================")

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
    include_install_path = os.path.join(SCRIPT_PATH, INSTALL_PATH)

    # Create target directory
    os.makedirs(bin_dir, exist_ok=True)

    # Find and copy include directory
    include_dir_src = os.path.join(include_install_path, "include")

    if not os.path.exists(include_dir_src):
        print(f"WARNING: Include directory not found at {include_dir_src}")
        return

    include_dir_dest = os.path.join(
        bin_dir, f"{project_name_upper}_INCLUDE-{full_version}"
    )
    if os.path.exists(include_dir_dest):
        shutil.rmtree(include_dir_dest)
    shutil.copytree(include_dir_src, include_dir_dest, ignore=get_archive_include_ignore_patterns())
    print(f"Copied include directory: {include_dir_dest}")

    # Create archive directory structure
    archive_name = f"ARCHIVE_{project_name_upper}_INCLUDE-{full_version}"
    archive_dir = os.path.join(bin_dir, archive_name)

    if os.path.exists(archive_dir):
        shutil.rmtree(archive_dir)
    os.makedirs(archive_dir, exist_ok=True)

    # Copy include directory to archive (excluding CPPLINT.cfg and other dev files)
    archive_include_dir = os.path.join(archive_dir, "include")
    shutil.copytree(include_dir_src, archive_include_dir, ignore=get_archive_include_ignore_patterns())
    print(f"Copied include directory to archive: include")

    # Create ZIP archive
    zip_file_path = os.path.join(bin_dir, f"{archive_name}.zip")
    with zipfile.ZipFile(zip_file_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        for root, dirs, files in os.walk(archive_dir):
            for file in files:
                file_path = os.path.join(root, file)
                arcname = os.path.relpath(file_path, bin_dir)
                zipf.write(file_path, arcname)

    # Remove temporary archive directory
    shutil.rmtree(archive_dir)

    print("==================Archive Complete========================")
    print(f"Include directory: {include_dir_dest}")
    print(f"Archive ZIP: {zip_file_path}")


def print_build_results():
    """
    Print include build results from target directory.

    This function displays the build artifacts and moves them to target/include/:
    1. Include directory
    2. ARCHIVE zip
    """
    print("==================Include Build Results========================")

    # Define paths
    bin_dir = os.path.join(SCRIPT_PATH, "target")

    # Check if target directory exists
    if not os.path.exists(bin_dir):
        print(f"ERROR: target directory not found. Please run build first.")
        sys.exit(1)

    # Check for build artifacts
    include_dirs = [
        f
        for f in glob.glob(f"{bin_dir}/*_INCLUDE-*")
        if os.path.isdir(f) and "ARCHIVE" not in f
    ]
    archive_zips = glob.glob(f"{bin_dir}/ARCHIVE*_INCLUDE-*.zip")

    if not include_dirs and not archive_zips:
        print(f"ERROR: No build artifacts found in {bin_dir}")
        print("Please ensure build completed successfully.")
        sys.exit(1)

    # Clean and recreate target/include directory for platform-specific artifacts
    bin_include_dir = os.path.join(bin_dir, "include")
    if os.path.exists(bin_include_dir):
        shutil.rmtree(bin_include_dir)
        print(f"Cleaned up old target/include/ directory")
    os.makedirs(bin_include_dir, exist_ok=True)

    # Move include directories and archive files to target/include/
    artifacts_moved = []
    for include_dir in include_dirs:
        dest = os.path.join(bin_include_dir, os.path.basename(include_dir))
        if os.path.exists(dest):
            shutil.rmtree(dest)
        shutil.move(include_dir, dest)
        artifacts_moved.append(os.path.basename(include_dir))

    for archive_zip in archive_zips:
        dest = os.path.join(bin_include_dir, os.path.basename(archive_zip))
        shutil.move(archive_zip, dest)
        artifacts_moved.append(os.path.basename(archive_zip))

    if artifacts_moved:
        print(f"[SUCCESS] Moved {len(artifacts_moved)} artifact(s) to target/include/")

    # Copy build_info.json from cmake_build to target/include
    copy_build_info_to_target("include", SCRIPT_PATH)

    print(f"\nBuild artifacts in target/include/:")
    print("-" * 60)

    # List all files in target/include directory with sizes
    for item in sorted(os.listdir(bin_include_dir)):
        item_path = os.path.join(bin_include_dir, item)
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


def main():
    """
    Main entry point for building include headers.
    """
    if not build_include(False):
        raise RuntimeError("Exception occurs when build include")

    # Archive and organize artifacts
    archive_include_project()
    print_build_results()


# Command-line interface for include builds
#
# Usage:
#   python build_include.py    # Build and package include headers
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build and package include headers",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )

    args = parser.parse_args()
    main()
