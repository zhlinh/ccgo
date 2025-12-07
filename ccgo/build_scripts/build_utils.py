#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_utils.py
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
Build utility functions for cross-platform native library compilation.

This module provides shared utilities used by all platform-specific build scripts:
- Library combining and merging (libtool, lipo, ar)
- File operations (copy, clean, zip)
- Version management and git integration
- Environment checking (NDK, OHOS SDK, Visual Studio)
- Framework creation (iOS/macOS)
- Cross-platform compatibility helpers

The utilities handle platform differences between macOS, Linux, Windows, Android,
iOS, and HarmonyOS (OHOS) builds.
"""

import glob
import os
import shutil
import time
import glob
import codecs
import io
import platform
import subprocess
import sys
import re
import zipfile
import json
from urllib.parse import urlparse
from datetime import datetime

if sys.version_info >= (3, 11, 0, "alpha", 7):
    import tomllib
else:
    # For Python < 3.11, try to import tomli as fallback
    try:
        import tomli as tomllib
    except ImportError:
        tomllib = None

# Files to exclude when archiving include/headers directories
# These are development/tooling files that should not be distributed
ARCHIVE_INCLUDE_EXCLUDE_FILES = [
    "CPPLINT.cfg",
    ".clang-format",
    ".clang-tidy",
]


def get_archive_include_ignore_patterns():
    """
    Get ignore patterns for shutil.copytree when copying include directories.

    Returns:
        A callable suitable for shutil.copytree's ignore parameter.
    """
    return shutil.ignore_patterns(*ARCHIVE_INCLUDE_EXCLUDE_FILES)


def should_include_file_in_archive(filename):
    """
    Check if a file should be included in the archive.

    Args:
        filename: The filename to check (not the full path).

    Returns:
        bool: True if the file should be included, False if it should be excluded.
    """
    return filename not in ARCHIVE_INCLUDE_EXCLUDE_FILES


# Load configuration from CCGO.toml in project directory (current working directory)
PROJECT_DIR = os.getcwd()


def load_ccgo_config():
    """
    Load configuration from CCGO.toml file.

    Returns a dictionary with build configuration values that were previously
    defined in build_config.py. Falls back to default values if CCGO.toml
    is not found or cannot be parsed.
    """
    # Use current working directory instead of module-level PROJECT_DIR
    project_dir = os.getcwd()
    config_file = os.path.join(project_dir, "CCGO.toml")

    if not os.path.isfile(config_file):
        print(f"   ⚠️  Warning: CCGO.toml not found at {config_file}")
        print("   ⚠️  Using default configuration values")
        return {
            "PROJECT_NAME": "SDK",
            "PROJECT_NAME_LOWER": "sdk",
            "CONFIG_PROJECT_VERSION": "1.0.0",
            "OUTPUT_VERINFO_PATH": "include/sdk/base/",
            "ANDROID_PROJECT_PATH": "android/main_android_sdk",
            "ANDROID_MERGE_THIRD_PARTY_LIBS": [],
            "ANDROID_MERGE_EXCLUDE_LIBS": [],
            "OHOS_PROJECT_PATH": "ohos/main_ohos_sdk",
            "OHOS_MERGE_THIRD_PARTY_LIBS": [],
            "OHOS_MERGE_EXCLUDE_LIBS": [],
            "IOS_BUILD_COPY_HEADER_FILES": {},
            "WATCHOS_BUILD_COPY_HEADER_FILES": {},
            "TVOS_BUILD_COPY_HEADER_FILES": {},
            "MACOS_BUILD_COPY_HEADER_FILES": {},
            "WINDOWS_BUILD_COPY_HEADER_FILES": {},
            "LINUX_BUILD_COPY_HEADER_FILES": {},
            "INCLUDE_BUILD_COPY_HEADER_FILES": {},
            "DEPENDENCIES": {},
            "PLATFORM_DEPENDENCIES": {},
        }

    if not tomllib:
        print("   ⚠️  Warning: tomllib not available. Install 'tomli' for Python < 3.11")
        print("   ⚠️  Using default configuration values")
        return {
            "PROJECT_NAME": "SDK",
            "PROJECT_NAME_LOWER": "sdk",
            "CONFIG_PROJECT_VERSION": "1.0.0",
            "OUTPUT_VERINFO_PATH": "include/sdk/base/",
            "ANDROID_PROJECT_PATH": "android/main_android_sdk",
            "ANDROID_MERGE_THIRD_PARTY_LIBS": [],
            "ANDROID_MERGE_EXCLUDE_LIBS": [],
            "OHOS_PROJECT_PATH": "ohos/main_ohos_sdk",
            "OHOS_MERGE_THIRD_PARTY_LIBS": [],
            "OHOS_MERGE_EXCLUDE_LIBS": [],
            "IOS_BUILD_COPY_HEADER_FILES": {},
            "WATCHOS_BUILD_COPY_HEADER_FILES": {},
            "TVOS_BUILD_COPY_HEADER_FILES": {},
            "MACOS_BUILD_COPY_HEADER_FILES": {},
            "WINDOWS_BUILD_COPY_HEADER_FILES": {},
            "LINUX_BUILD_COPY_HEADER_FILES": {},
            "INCLUDE_BUILD_COPY_HEADER_FILES": {},
            "DEPENDENCIES": {},
            "PLATFORM_DEPENDENCIES": {},
        }

    try:
        with open(config_file, "rb") as f:
            toml_data = tomllib.load(f)

        # Extract project information
        project_name_lower = toml_data.get("project", {}).get("name", "sdk")
        project_name = project_name_lower.upper()
        version = toml_data.get("project", {}).get("version", "1.0.0")
        verinfo_path = toml_data.get("build", {}).get(
            "verinfo_path", f"include/{project_name_lower}/base/"
        )

        # Extract Android configuration
        android = toml_data.get("android", {})
        android_project_path = android.get("project_path", "android/main_android_sdk")
        android_merge_libs = android.get("merge_libs", [])
        android_exclude_libs = android.get("exclude_libs", [])

        # Extract OHOS configuration
        ohos = toml_data.get("ohos", {})
        ohos_project_path = ohos.get("project_path", "ohos/main_ohos_sdk")
        ohos_merge_libs = ohos.get("merge_libs", [])
        ohos_exclude_libs = ohos.get("exclude_libs", [])

        # Extract platform-specific header file mappings
        # Convert from TOML format: [{ src = "path", dest = "dir" }]
        # To Python dict format: { "path": "dir" }
        def convert_headers(platform_config):
            export_headers = platform_config.get("export_headers", [])
            header_dict = {}
            for header in export_headers:
                if isinstance(header, dict) and "src" in header and "dest" in header:
                    header_dict[header["src"]] = header["dest"]
            return header_dict

        # Apple platforms: use [apple] as base config, platform-specific can override
        apple_headers = convert_headers(toml_data.get("apple", {}))
        ios_headers = convert_headers(toml_data.get("ios", {})) or apple_headers
        watchos_headers = convert_headers(toml_data.get("watchos", {})) or apple_headers
        tvos_headers = convert_headers(toml_data.get("tvos", {})) or apple_headers
        macos_headers = convert_headers(toml_data.get("macos", {})) or apple_headers
        windows_headers = convert_headers(toml_data.get("windows", {}))
        linux_headers = convert_headers(toml_data.get("linux", {}))
        include_headers = convert_headers(toml_data.get("include", {}))

        # Extract dependencies
        dependencies = toml_data.get("dependencies", {})

        # Extract platform-specific dependencies
        platform_dependencies = {}
        for key in toml_data.keys():
            if key.startswith("target."):
                # Extract platform config like target.'cfg(windows)'
                target_cfg = key.replace("target.", "").strip("'\"")
                target_data = toml_data[key]
                if "dependencies" in target_data:
                    platform_dependencies[target_cfg] = target_data["dependencies"]

        return {
            "PROJECT_NAME": project_name,
            "PROJECT_NAME_LOWER": project_name_lower,
            "CONFIG_PROJECT_VERSION": version,
            "OUTPUT_VERINFO_PATH": verinfo_path,
            "ANDROID_PROJECT_PATH": android_project_path,
            "ANDROID_MERGE_THIRD_PARTY_LIBS": android_merge_libs,
            "ANDROID_MERGE_EXCLUDE_LIBS": android_exclude_libs,
            "OHOS_PROJECT_PATH": ohos_project_path,
            "OHOS_MERGE_THIRD_PARTY_LIBS": ohos_merge_libs,
            "OHOS_MERGE_EXCLUDE_LIBS": ohos_exclude_libs,
            "IOS_BUILD_COPY_HEADER_FILES": ios_headers,
            "WATCHOS_BUILD_COPY_HEADER_FILES": watchos_headers,
            "TVOS_BUILD_COPY_HEADER_FILES": tvos_headers,
            "MACOS_BUILD_COPY_HEADER_FILES": macos_headers,
            "WINDOWS_BUILD_COPY_HEADER_FILES": windows_headers,
            "LINUX_BUILD_COPY_HEADER_FILES": linux_headers,
            "INCLUDE_BUILD_COPY_HEADER_FILES": include_headers,
            "DEPENDENCIES": dependencies,
            "PLATFORM_DEPENDENCIES": platform_dependencies,
        }
    except Exception as e:
        print(f"   ⚠️  Error reading CCGO.toml: {e}")
        print("   ⚠️  Using default configuration values")
        return {
            "PROJECT_NAME": "SDK",
            "PROJECT_NAME_LOWER": "sdk",
            "CONFIG_PROJECT_VERSION": "1.0.0",
            "OUTPUT_VERINFO_PATH": "include/sdk/base/",
            "ANDROID_PROJECT_PATH": "android/main_android_sdk",
            "ANDROID_MERGE_THIRD_PARTY_LIBS": [],
            "ANDROID_MERGE_EXCLUDE_LIBS": [],
            "OHOS_PROJECT_PATH": "ohos/main_ohos_sdk",
            "OHOS_MERGE_THIRD_PARTY_LIBS": [],
            "OHOS_MERGE_EXCLUDE_LIBS": [],
            "IOS_BUILD_COPY_HEADER_FILES": {},
            "WATCHOS_BUILD_COPY_HEADER_FILES": {},
            "TVOS_BUILD_COPY_HEADER_FILES": {},
            "MACOS_BUILD_COPY_HEADER_FILES": {},
            "WINDOWS_BUILD_COPY_HEADER_FILES": {},
            "LINUX_BUILD_COPY_HEADER_FILES": {},
            "INCLUDE_BUILD_COPY_HEADER_FILES": {},
            "DEPENDENCIES": {},
            "PLATFORM_DEPENDENCIES": {},
        }


# Load configuration and make variables available globally
_CONFIG = load_ccgo_config()
PROJECT_NAME = _CONFIG["PROJECT_NAME"]
PROJECT_NAME_LOWER = _CONFIG["PROJECT_NAME_LOWER"]
CONFIG_PROJECT_VERSION = _CONFIG["CONFIG_PROJECT_VERSION"]
OUTPUT_VERINFO_PATH = _CONFIG["OUTPUT_VERINFO_PATH"]
ANDROID_PROJECT_PATH = _CONFIG["ANDROID_PROJECT_PATH"]
ANDROID_MERGE_THIRD_PARTY_LIBS = _CONFIG["ANDROID_MERGE_THIRD_PARTY_LIBS"]
ANDROID_MERGE_EXCLUDE_LIBS = _CONFIG["ANDROID_MERGE_EXCLUDE_LIBS"]
OHOS_PROJECT_PATH = _CONFIG["OHOS_PROJECT_PATH"]
OHOS_MERGE_THIRD_PARTY_LIBS = _CONFIG["OHOS_MERGE_THIRD_PARTY_LIBS"]
OHOS_MERGE_EXCLUDE_LIBS = _CONFIG["OHOS_MERGE_EXCLUDE_LIBS"]
IOS_BUILD_COPY_HEADER_FILES = _CONFIG["IOS_BUILD_COPY_HEADER_FILES"]
WATCHOS_BUILD_COPY_HEADER_FILES = _CONFIG["WATCHOS_BUILD_COPY_HEADER_FILES"]
TVOS_BUILD_COPY_HEADER_FILES = _CONFIG["TVOS_BUILD_COPY_HEADER_FILES"]
MACOS_BUILD_COPY_HEADER_FILES = _CONFIG["MACOS_BUILD_COPY_HEADER_FILES"]
WINDOWS_BUILD_COPY_HEADER_FILES = _CONFIG["WINDOWS_BUILD_COPY_HEADER_FILES"]
LINUX_BUILD_COPY_HEADER_FILES = _CONFIG["LINUX_BUILD_COPY_HEADER_FILES"]
INCLUDE_BUILD_COPY_HEADER_FILES = _CONFIG["INCLUDE_BUILD_COPY_HEADER_FILES"]
DEPENDENCIES = _CONFIG["DEPENDENCIES"]
PLATFORM_DEPENDENCIES = _CONFIG["PLATFORM_DEPENDENCIES"]

# Store the build script path for cmake directory access
BUILD_SCRIPT_PATH = os.path.dirname(os.path.realpath(__file__))
# CCGO cmake directory path (in the ccgo package)
CCGO_CMAKE_DIR = os.path.join(BUILD_SCRIPT_PATH, "cmake")


def resolve_dependencies(platform=None):
    """
    Resolve all dependencies for the project.

    Args:
        platform: Target platform (e.g., "android", "ios", "windows", "linux", "macos", "ohos")
                  If None, only resolves common dependencies

    Returns:
        Dictionary mapping dependency name to resolved path
    """
    # Import here to avoid circular dependencies
    from .dependency_manager import DependencyManager, should_include_platform_dependencies

    if not DEPENDENCIES and not PLATFORM_DEPENDENCIES:
        # No dependencies to resolve
        return {}

    print("   🔍 Resolving dependencies...")

    # Create dependency manager
    dep_manager = DependencyManager(PROJECT_DIR)

    # Start with common dependencies
    all_deps = dict(DEPENDENCIES)

    # Add platform-specific dependencies
    if platform and PLATFORM_DEPENDENCIES:
        for platform_cfg, deps in PLATFORM_DEPENDENCIES.items():
            if should_include_platform_dependencies(platform_cfg, platform):
                print(f"   📦 Including platform-specific dependencies for: {platform_cfg}")
                all_deps.update(deps)

    if not all_deps:
        print("   ℹ️  No dependencies to resolve")
        return {}

    # Resolve all dependencies
    try:
        resolved = dep_manager.resolve_all_dependencies(all_deps)
        print(f"   ✅ Successfully resolved {len(resolved)} dependencies")
        return resolved
    except Exception as e:
        print(f"   ❌ Failed to resolve dependencies: {e}")
        raise


def get_dependency_cmake_args(resolved_deps=None):
    """
    Get CMake arguments for dependencies.

    Args:
        resolved_deps: Dictionary of resolved dependencies (if None, dependencies are resolved automatically)

    Returns:
        List of CMake arguments to include dependencies
    """
    if resolved_deps is None:
        if not DEPENDENCIES and not PLATFORM_DEPENDENCIES:
            return []
        # Dependencies will be resolved later by the build script
        return []

    if not resolved_deps:
        return []

    # Import here to avoid circular dependencies
    from .dependency_manager import DependencyManager

    dep_manager = DependencyManager(PROJECT_DIR)

    # Get include directories
    include_dirs = dep_manager.get_cmake_include_dirs(resolved_deps)

    cmake_args = []
    if include_dirs:
        # Add include directories
        cmake_args.append(f"-DCCGO_DEP_INCLUDE_DIRS={';'.join(include_dirs)}")

    # Add dependency paths
    dep_paths = list(resolved_deps.values())
    if dep_paths:
        cmake_args.append(f"-DCCGO_DEP_PATHS={';'.join(dep_paths)}")

    return cmake_args


def libtool_libs(src_libs, dst_lib):
    """
    Combine multiple static libraries into a single static library.

    This function merges multiple static library files (.a) into a single
    output library. The implementation differs based on platform:
    - macOS: Uses 'libtool' command with -static flag
    - Linux: Uses 'gcc -r' to relocate and 'ar crs' to create archive

    Args:
        src_libs: List of source library file paths to merge
        dst_lib: Destination library file path for the merged output

    Returns:
        bool: True if merge succeeded, False otherwise

    Note:
        On Linux, the ar command flags mean:
        - r: Replace existing files or add new files
        - c: Create archive if it doesn't exist
        - s: Create symbol table index for linking

        The Linux implementation uses --whole-archive to include all symbols
        and --allow-multiple-definition to handle duplicate symbols.
    """
    src_lib_str = " ".join(src_libs)
    os.makedirs(dst_lib[: dst_lib.rfind("/")], exist_ok=True)

    if platform.system().lower() == "darwin":
        # macOS: Use libtool for static library merging
        cmd = f"libtool -static -no_warning_for_no_symbols -o {dst_lib} {src_lib_str}"
    else:
        # Linux: Use gcc relocatable link + ar archiver
        # ar crs means:
        # r option: add/replace files in the static library
        # c option: create library if it doesn't exist
        # s option: create symbol table for linking
        if dst_lib.endswith(".a"):
            temp_static_lib = dst_lib
        else:
            temp_static_lib = f"{dst_lib}.a"
        cmd = (
            f"gcc -r -nostdlib -Wl,--whole-archive -Wl,--allow-multiple-definition -o {dst_lib}.o {src_lib_str} && ar crs {temp_static_lib} {dst_lib}.o "
            f"&& rm -f {dst_lib}.o"
        )
        if temp_static_lib != dst_lib:
            cmd += f" && mv {temp_static_lib} {dst_lib}"

    print(cmd)
    ret = os.system(cmd)
    if ret != 0:
        print(f"!!!!!!!!!!! libtool {dst_lib} failed, cmd:['{cmd}'] !!!!!!!!!!!!!!!")
        return False

    return True


def lipo_libs(src_libs, dst_lib):
    """
    Create a universal (fat) binary from multiple architecture-specific libraries.

    This function is primarily used on macOS/iOS to create universal binaries that
    support multiple CPU architectures (e.g., arm64, x86_64) in a single file.
    On non-macOS platforms, it falls back to libtool_libs for static library merging.

    Args:
        src_libs: List of architecture-specific library file paths
        dst_lib: Destination path for the universal binary library

    Returns:
        bool: True if creation succeeded, False otherwise

    Example:
        # Create universal binary from arm64 and x86_64 libraries
        lipo_libs(['libfoo.arm64.a', 'libfoo.x86_64.a'], 'libfoo.a')
    """
    if platform.system().lower() != "darwin":
        # Non-macOS platforms don't support lipo, use static library merging instead
        return libtool_libs(src_libs, dst_lib)

    src_lib_str = " ".join(src_libs)
    cmd = "lipo -create %s -output %s" % (src_lib_str, dst_lib)
    ret = os.system(cmd)
    if ret != 0:
        print(f"!!!!!!!!!!! lipo_libs {dst_lib} failed, cmd:['{cmd}'] !!!!!!!!!!!!!!!")
        return False

    return True


def lipo_thin_libs(src_lib, dst_lib, archs):
    """
    Extract specific architectures from a universal binary and optionally recombine.

    This function uses 'lipo -thin' to extract one or more architectures from
    a universal (fat) binary. If multiple architectures are specified, it extracts
    each one separately then recombines them using lipo_libs.

    Args:
        src_lib: Source universal binary library path
        dst_lib: Destination library path
        archs: List of architecture names to extract (e.g., ['arm64', 'x86_64'])

    Returns:
        bool: True if extraction/recombination succeeded, False otherwise

    Example:
        # Extract only arm64 from a universal binary
        lipo_thin_libs('libfoo.a', 'libfoo_arm64.a', ['arm64'])

        # Extract and recombine arm64 and armv7
        lipo_thin_libs('libfoo.a', 'libfoo_mobile.a', ['arm64', 'armv7'])
    """
    tmp_results = []
    for arch in archs:
        if len(archs) == 1:
            tmp_result = dst_lib
        else:
            tmp_result = dst_lib + "." + arch

        cmd = f"lipo {src_lib} -thin {arch} -output {tmp_result}"
        ret = os.system(cmd)
        if ret != 0:
            print(
                f"!!!!!!!!!!!lipo_thin_libs {tmp_result} failed, cmd:{cmd}!!!!!!!!!!!!!!!"
            )
            return False
        tmp_results.append(tmp_result)

    if len(archs) == 1:
        return True
    else:
        return lipo_libs(tmp_results, dst_lib)


GENERATE_DSYM_FILE_CMD = "dsymutil {src_dylib} -o {dst_dsym}"


def gen_dwarf_with_dsym(src_dylib, dst_dsym):
    """
    Generate dSYM debug symbol file from a dynamic library (macOS/iOS).

    dSYM files contain debugging information extracted from targetaries,
    used for crash symbolication and debugging on Apple platforms.

    Args:
        src_dylib: Source dynamic library (.dylib) file path
        dst_dsym: Destination dSYM bundle path
    """
    os.system(GENERATE_DSYM_FILE_CMD.format(src_dylib=src_dylib, dst_dsym=dst_dsym))


def decode_bytes(input: bytes) -> str:
    """
    Decode bytes to string with fallback encoding support.

    Attempts UTF-8 decoding first, falls back to GBK for Chinese Windows systems.

    Args:
        input: Bytes object to decode

    Returns:
        str: Decoded string
    """
    err_msg = ""
    try:
        err_msg = bytes.decode(input, "UTF-8")
    except UnicodeDecodeError:
        err_msg = bytes.decode(input, "GBK")
    return err_msg


def exec_command(command):
    """
    Execute a shell command and capture its output.

    On Windows, sets console code page to UTF-8 for proper character encoding.
    Captures both stdout and stderr combined.

    Args:
        command: Shell command string to execute

    Returns:
        tuple: (exit_code, output_message)
            - exit_code: Integer return code from command (0 = success)
            - output_message: Combined stdout/stderr as decoded string
    """
    if sys.platform.startswith("win"):
        # Windows: set console charset to UTF-8
        subprocess.call("chcp 65001", shell=True)
    compile_popen = subprocess.Popen(
        command, shell=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT
    )
    compile_popen.wait()
    err_code = compile_popen.returncode
    err_msg = decode_bytes(compile_popen.stdout.read())
    return err_code, err_msg


def remove_cmake_files(path):
    """
    Remove CMake-generated files and built libraries from a directory.

    Cleans up:
    - CMakeFiles directory
    - Makefile
    - CMakeCache.txt
    - All library files (.a, .so, .lib, .dylib)
    - Framework bundles (.framework, .xcframework)

    Args:
        path: Directory path to clean
    """
    cmake_files = path + "/CMakeFiles"
    if os.path.exists(cmake_files):
        shutil.rmtree(cmake_files)

    make_files = path + "/Makefile"
    if os.path.isfile(make_files):
        os.remove(make_files)

    cmake_cache = path + "/CMakeCache.txt"
    if os.path.isfile(cmake_cache):
        os.remove(cmake_cache)

    # Remove all built library files
    for f in glob.glob(path + "/*.a"):
        os.remove(f)
    for f in glob.glob(path + "/*.so"):
        os.remove(f)
    for f in glob.glob(path + "/*.lib"):
        os.remove(f)
    for f in glob.glob(path + "/*.dylib"):
        os.remove(f)
    for f in glob.glob(path + "/*.xcframework"):
        shutil.rmtree(f)
    for f in glob.glob(path + "/*.framework"):
        shutil.rmtree(f)


def clean_except(path, except_list):
    """
    Clean CMake files from directory tree, excluding specified paths.

    Walks the directory tree and removes CMake-generated files from all
    directories except those matching any string in except_list.

    Args:
        path: Root directory path to start cleaning
        except_list: List of path substrings to exclude from cleaning
    """
    for fpath, dirs, fs in os.walk(path):
        in_except = False
        for exc in except_list:
            if exc in fpath:
                in_except = True
                break
        if not in_except:
            remove_cmake_files(fpath)

    if not os.path.exists(path):
        os.makedirs(path)


def clean_unix(path, incremental=False):
    """
    Clean build directory on Unix-like systems (Linux, macOS).

    Removes CMake-generated files from all subdirectories unless incremental build.

    Args:
        path: Build directory path to clean
        incremental: If True, skip cleaning to preserve incremental build state
    """
    if not incremental:
        for fpath, dirs, fs in os.walk(path):
            remove_cmake_files(fpath)

    if not os.path.exists(path):
        os.makedirs(path)


def clean_windows(path, incremental):
    """
    Clean build directory on Windows.

    On non-incremental builds, completely removes and recreates the directory.
    This is more aggressive than Unix cleaning due to Windows file locking issues.

    Args:
        path: Build directory path to clean
        incremental: If True, skip cleaning to preserve incremental build state
    """
    if not os.path.exists(path):
        os.makedirs(path)
        return

    if incremental:
        return

    try:
        if os.path.exists(path):
            shutil.rmtree(path)
            if not os.path.exists(path):
                os.makedirs(path)
    except Exception:
        pass


def clean(path, incremental=False):
    """
    Clean build directory in a platform-appropriate way.

    Dispatches to clean_windows() or clean_unix() based on platform.

    Args:
        path: Build directory path to clean
        incremental: If True, skip cleaning to preserve incremental build state
    """
    if system_is_windows():
        clean_windows(path, incremental)
    else:
        clean_unix(path, incremental)


def copy_file(src, dst):
    """
    Copy a file or directory, creating destination directories as needed.

    Args:
        src: Source file or directory path
        dst: Destination file or directory path

    Note:
        If src is a directory, the entire tree is copied recursively.
        Creates parent directories for dst if they don't exist.
    """
    if not os.path.exists(src):
        return
    if os.path.isfile(src):
        if dst.rfind("/") != -1 and not os.path.exists(dst[: dst.rfind("/")]):
            os.makedirs(dst[: dst.rfind("/")], exist_ok=True)
        shutil.copy(src, dst)
    else:
        # dirs_exist_ok = True needs python 3.8+
        shutil.copytree(src, dst)


def copy_file_mapping(
    header_file_mappings, header_files_src_base, header_files_dst_end
):
    """
    Copy header files according to a source-to-destination mapping.

    Args:
        header_file_mappings: Dict mapping source paths to destination subdirectory names
        header_files_src_base: Base path prepended to source paths
        header_files_dst_end: Base destination path

    Example:
        mappings = {'include/foo.h': 'public', 'include/bar.h': 'internal'}
        copy_file_mapping(mappings, './', './output')
        # Copies ./include/foo.h to ./output/public/foo.h
    """
    for src, dst in header_file_mappings.items():
        if not os.path.exists(src):
            continue
        copy_file(
            header_files_src_base + src,
            header_files_dst_end + "/" + dst + "/" + src[src.rfind("/") :],
        )


def make_static_framework(
    src_lib, dst_framework, header_file_mappings, header_files_src_base="./",
    apple_headers_src=None
):
    """
    Create an iOS/macOS static framework bundle from a library and headers.

    A framework is a bundle directory containing:
    - The compiled library binary
    - Headers/ directory with public headers (Apple standard convention)
    - An umbrella header that imports all public headers

    Args:
        src_lib: Source library file path (.a static library)
        dst_framework: Destination framework bundle path (.framework)
        header_file_mappings: Dict mapping header source paths to subdirectory destinations
        header_files_src_base: Base path for header source files
        apple_headers_src: Optional path to apple API headers directory (e.g., include/project/api/apple/)

    Returns:
        bool: True if framework creation succeeded

    Note:
        Removes existing framework at dst_framework if it exists.
    """
    if os.path.exists(dst_framework):
        shutil.rmtree(dst_framework)

    os.makedirs(dst_framework)
    shutil.copy(src_lib, dst_framework)

    # Use Headers/ directory following Apple framework convention
    framework_headers_path = dst_framework + "/Headers"
    os.makedirs(framework_headers_path, exist_ok=True)

    # Track all copied headers for umbrella header generation
    copied_headers = []

    # Copy headers from header_file_mappings
    for src, dst in header_file_mappings.items():
        if not os.path.exists(src):
            continue
        header_filename = src[src.rfind("/") + 1:]
        relative_path = dst + "/" + header_filename
        copy_file(
            header_files_src_base + src,
            framework_headers_path + "/" + relative_path,
        )
        copied_headers.append(relative_path)

    # Generate umbrella header that imports all other headers
    if apple_headers_src and os.path.exists(apple_headers_src):
        # Get framework name from path (e.g., ccgonow.framework -> Ccgonow)
        framework_name = os.path.basename(dst_framework).replace('.framework', '')
        umbrella_name = framework_name[0].upper() + framework_name[1:] + ".h"
        umbrella_path = os.path.join(framework_headers_path, umbrella_name)

        # Generate umbrella header content
        umbrella_content = f"""//
//  {umbrella_name}
//  {framework_name}.framework
//
//  Auto-generated umbrella header by CCGO
//

#ifndef {framework_name.upper()}_UMBRELLA_H
#define {framework_name.upper()}_UMBRELLA_H

"""
        # Add imports for all copied headers
        for header in sorted(copied_headers):
            umbrella_content += f'#import "{header}"\n'

        umbrella_content += f"""
#endif // {framework_name.upper()}_UMBRELLA_H
"""

        # Write umbrella header
        with open(umbrella_path, 'w') as f:
            f.write(umbrella_content)
        print(f"  Generated umbrella header: {umbrella_name}")

    return True


def make_dynamic_framework(
    src_dylib, dst_framework, header_file_mappings, header_files_src_base="./",
    apple_headers_src=None, framework_id=None
):
    """
    Create an iOS/macOS dynamic framework bundle from a dylib and headers.

    A dynamic framework is a bundle directory containing:
    - The compiled dynamic library (dylib renamed to match framework name)
    - Headers/ directory with public headers (Apple standard convention)
    - An umbrella header that imports all public headers
    - Info.plist (optional, for proper framework identification)

    Args:
        src_dylib: Source dynamic library file path (.dylib)
        dst_framework: Destination framework bundle path (.framework)
        header_file_mappings: Dict mapping header source paths to subdirectory destinations
        header_files_src_base: Base path for header source files
        apple_headers_src: Optional path to apple API headers directory
        framework_id: Optional framework identifier for install_name_tool

    Returns:
        bool: True if framework creation succeeded

    Note:
        For iOS App Store submission, dynamic libraries must be packaged
        as frameworks. Standalone .dylib files are not accepted.
    """
    if os.path.exists(dst_framework):
        shutil.rmtree(dst_framework)

    os.makedirs(dst_framework)

    # Get framework name from path (e.g., ccgonow.framework -> ccgonow)
    framework_name = os.path.basename(dst_framework).replace('.framework', '')

    # Copy dylib and rename to framework binary name (without lib prefix and .dylib suffix)
    framework_binary = os.path.join(dst_framework, framework_name)
    shutil.copy(src_dylib, framework_binary)

    # Update the install name to use @rpath for proper dynamic linking
    if framework_id is None:
        framework_id = f"@rpath/{framework_name}.framework/{framework_name}"

    # Use install_name_tool to set the correct install name
    install_name_cmd = f'install_name_tool -id "{framework_id}" "{framework_binary}"'
    ret = os.system(install_name_cmd)
    if ret != 0:
        print(f"WARNING: Failed to set install name for {framework_binary}")

    # Use Headers/ directory following Apple framework convention
    framework_headers_path = dst_framework + "/Headers"
    os.makedirs(framework_headers_path, exist_ok=True)

    # Track all copied headers for umbrella header generation
    copied_headers = []

    # Copy headers from header_file_mappings
    for src, dst in header_file_mappings.items():
        if not os.path.exists(src):
            continue
        header_filename = src[src.rfind("/") + 1:]
        relative_path = dst + "/" + header_filename
        copy_file(
            header_files_src_base + src,
            framework_headers_path + "/" + relative_path,
        )
        copied_headers.append(relative_path)

    # Generate umbrella header that imports all other headers
    if apple_headers_src and os.path.exists(apple_headers_src):
        umbrella_name = framework_name[0].upper() + framework_name[1:] + ".h"
        umbrella_path = os.path.join(framework_headers_path, umbrella_name)

        # Generate umbrella header content
        umbrella_content = f"""//
//  {umbrella_name}
//  {framework_name}.framework
//
//  Auto-generated umbrella header by CCGO
//

#ifndef {framework_name.upper()}_UMBRELLA_H
#define {framework_name.upper()}_UMBRELLA_H

"""
        # Add imports for all copied headers
        for header in sorted(copied_headers):
            umbrella_content += f'#import "{header}"\n'

        umbrella_content += f"""
#endif // {framework_name.upper()}_UMBRELLA_H
"""

        # Write umbrella header
        with open(umbrella_path, 'w') as f:
            f.write(umbrella_content)
        print(f"  Generated umbrella header: {umbrella_name}")

    print(f"  Created dynamic framework: {dst_framework}")
    return True


def make_xcframework(os_framework, simulator_framework, dst_framework):
    """
    Create an XCFramework from device and simulator frameworks.

    XCFrameworks bundle multiple architectures/platforms into a single
    distributable package that works on both iOS devices and simulators.

    Args:
        os_framework: Path to device (iOS/OS) framework
        simulator_framework: Path to simulator framework
        dst_framework: Destination XCFramework path (.xcframework)

    Returns:
        bool: True if XCFramework creation succeeded, False otherwise

    Note:
        Requires Xcode command-line tools to be installed.
    """
    cmd = "xcodebuild -create-xcframework"
    cmd += f" -framework {os_framework}"
    cmd += f" -framework {simulator_framework}"
    cmd += f" -output {dst_framework}"
    ret = os.system(cmd)
    if ret != 0:
        print(
            f"!!!!!!!!!!! make_xcframework {dst_framework} failed, cmd:['{cmd}'] !!!!!!!!!!!!!!!"
        )
        return False

    return True


def get_ndk_desc():
    """
    Get the expected Android NDK version string.

    Returns:
        str: NDK version identifier (e.g., "r25c")
    """
    return "r25c"


def check_ndk_revision(revision):
    """
    Check if NDK revision meets minimum version requirement.

    Args:
        revision: NDK revision string (e.g., "25.2.9519653")

    Returns:
        bool: True if revision >= 25.2, False otherwise
    """
    if revision >= "25.2":
        return True
    return False


def get_ndk_revision():
    """
    Get the installed Android NDK version from NDK_ROOT environment variable.

    Reads the version from NDK's source.properties file.

    Returns:
        tuple: (error_code, ndk_revision_or_error_message)
            - On success: (0, revision_string)
            - On error: (negative_code, error_message)

    Note:
        Requires NDK_ROOT environment variable to be set to NDK installation path.
    """
    try:
        ndk_path = os.environ["NDK_ROOT"]
    except KeyError as identifier:
        return -1, "Error: ndk does not exist or you do not set it into NDK_ROOT."

    if not ndk_path:
        return -3, "Error: ndk does not exist or you do not set it into NDK_ROOT."

    if not os.path.isfile(os.path.join(ndk_path, "source.properties")):
        return (
            -4,
            f"Error: source.properties does not exist, make sure ndk's version=={get_ndk_desc()}",
        )

    ndk_revision = None

    f = open(os.path.join(ndk_path, "source.properties"))
    line = f.readline()
    while line:
        if line.startswith("Pkg.Revision") and len(line.split("=")) == 2:
            ndk_revision = line.split("=")[1].strip()
            break
        line = f.readline()

    f.close()

    if not ndk_revision or len(ndk_revision) < 4:
        return -5, "Error: parse source.properties fail"
    return 0, ndk_revision


def check_ndk_env():
    """
    Validate that Android NDK is installed and meets version requirements.

    Checks for NDK_ROOT environment variable and verifies NDK version >= r25.2.

    Returns:
        bool: True if NDK environment is valid, False otherwise

    Note:
        Prints error messages to stdout on validation failure.
    """
    err_code, ndk_revision = get_ndk_revision()
    if err_code != 0:
        print(ndk_revision)
        return False

    # Extract major.minor version (e.g., "25.2" from "25.2.9519653")
    version_parts = ndk_revision.split('.')
    if len(version_parts) >= 2:
        major_minor = f"{version_parts[0]}.{version_parts[1]}"
    else:
        major_minor = ndk_revision[:4]

    if check_ndk_revision(major_minor):
        return True

    print(
        f"Error: make sure ndk's version == {get_ndk_desc()}, current is {major_minor}"
    )
    return False


def get_ndk_host_tag():
    """
    Get the NDK host platform tag for toolchain paths.

    Constructs a platform identifier string used by NDK for host-specific tools.

    Returns:
        str: Platform tag (e.g., "darwin-x86_64", "linux-x86_64", "windows")

    Example:
        On macOS 64-bit: returns "darwin-x86_64"
        On Linux 64-bit: returns "linux-x86_64"
    """
    system_str = platform.system().lower()
    if system_architecture_is64():
        system_str = system_str + "-x86_64"
    return system_str


def get_ohos_native_target():
    """
    Get the target OHOS (HarmonyOS) Native API level.

    Returns:
        str: API level number (e.g., "12")
    """
    return "12"


def get_ohos_native_desc():
    """
    Get the OHOS Native SDK description string.

    Returns:
        str: SDK description (e.g., "api-12")
    """
    return f"api-{get_ohos_native_target()}"


def check_ohos_native_revision(revision):
    """
    Check if OHOS Native SDK revision meets minimum API level requirement.

    Args:
        revision: OHOS Native SDK API version string

    Returns:
        bool: True if revision >= target API level, False otherwise
    """
    if revision >= str(get_ohos_native_target()):
        return True
    return False


def get_ohos_native_revision():
    """
    Get the installed OHOS Native SDK version from environment variables.

    Reads the API version from OHOS SDK's oh-uni-package.json file.
    Checks both OHOS_SDK_HOME and HOS_SDK_HOME environment variables.

    Returns:
        tuple: (error_code, ohos_revision_or_error_message)
            - On success: (0, api_version_string)
            - On error: (negative_code, error_message)

    Note:
        Requires OHOS_SDK_HOME or HOS_SDK_HOME environment variable to be set
        to the OHOS SDK installation path.
    """
    try:
        ohos_sdk_path = os.environ["OHOS_SDK_HOME"] or os.environ["HOS_SDK_HOME"]
    except KeyError as identifier:
        return (
            -1,
            "Error: ohos sdk does not exist or you do not set it into OHOS_SDK_HOME.",
        )

    if not ohos_sdk_path:
        return -3, "Error: ohos not exist or you do not set it into OHOS_SDK_HOME."

    native_version_file_path = os.path.join(
        ohos_sdk_path, "native", "oh-uni-package.json"
    )
    if not os.path.isfile(native_version_file_path):
        return (
            -4,
            f"Error: oh-uni-package.json does not exist, make sure ohos native's version=={get_ohos_native_desc()}",
        )

    ohos_native_revision = None

    f = open(native_version_file_path)
    line = f.readline()
    while line:
        line = line.strip()
        if line.startswith('"apiVersion"') and len(line.split(":")) == 2:
            ohos_native_revision = line.split(":")[1].strip().strip(",").strip('"')
            break
        line = f.readline()

    f.close()

    if not ohos_native_revision or len(ohos_native_revision) < 2:
        return -5, "Error: parse oh-uni-package.json fail"
    return 0, ohos_native_revision


def check_ohos_native_env():
    """
    Validate that OHOS Native SDK is installed and meets version requirements.

    Checks for OHOS_SDK_HOME/HOS_SDK_HOME environment variable and verifies
    SDK API version meets the minimum requirement.

    Returns:
        bool: True if OHOS SDK environment is valid, False otherwise

    Note:
        Prints error messages to stdout on validation failure.
    """
    err_code, ohos_native_revision = get_ohos_native_revision()
    if err_code != 0:
        print(ohos_native_revision)
        return False
    if check_ohos_native_revision(ohos_native_revision[:4]):
        return True

    print(
        f"Error: make sure ohos native's version == {get_ohos_native_desc()}, current is {ohos_native_revision[:4]}"
    )
    return False


def parse_as_git(path):
    """
    Extract git repository information (revision, branch, remote URL).

    Executes git commands to retrieve current commit hash, branch name,
    and remote origin URL. Sanitizes OAuth2 credentials from URL if present.

    Args:
        path: Path to git repository directory

    Returns:
        tuple: (revision, branch_or_path, url)
            - revision: Short commit hash (default: "unknown" if not available)
            - branch_or_path: Current branch name (default: "unknown" if not available)
            - url: Remote origin URL (default: "" if not available, with OAuth2 credentials removed)

    Note:
        Changes current working directory temporarily during execution.
        Returns default values if git commands fail (e.g., not a git repository).
    """
    curdir = os.getcwd()
    os.chdir(path)

    # Get revision with default fallback
    try:
        revision = os.popen("git rev-parse --short HEAD 2>/dev/null").read().strip()
        if not revision:
            revision = "unknown"
    except:
        revision = "unknown"

    # Get branch with default fallback
    try:
        branch = os.popen("git rev-parse --abbrev-ref HEAD 2>/dev/null").read().strip()
        if not branch:
            branch = "unknown"
    except:
        branch = "unknown"

    # Get remote URL with default fallback
    try:
        url = os.popen("git remote get-url origin 2>/dev/null").read().strip()
        if not url:
            url = ""
    except:
        url = ""

    # Remove OAuth2 credentials from URL for security
    if url:
        pos = url.find("oauth2")
        if pos >= 0:
            pos_to_trim = url.find("@")
            if pos_to_trim >= 0:
                url = "git" + url[pos_to_trim:]

    os.chdir(curdir)

    return revision, branch, url


def normalize_git_url(url):
    """
    Normalize and anonymize git URL for safe display.

    Converts git URLs to path format and masks usernames for privacy.
    Usernames are truncated to first 2 characters followed by '***'.

    Args:
        url: Git repository URL (SSH or HTTPS format)

    Returns:
        str: Normalized and anonymized URL path

    Example:
        'git@github.com:username/repo.git' -> '/us***/repo.git'
        'https://github.com/username/repo.git' -> '/us***/repo.git'
    """
    if url.startswith("git@"):
        url = "/" + url.split(":")[-1]
    url_obj = urlparse(url)
    url = url_obj.path
    # Anonymize username: only show first 2 chars
    url = re.sub(r"/([^/]{2})[^/]*/", r"/\1***/", url)
    return url


def _gen_build_info_json(
    project_name,
    project_dir_path,
    version_name,
    revision,
    path,
    url,
    build_time,
    ndk_revision,
    target_platform=None,
):
    """
    Generate build_info.json file with comprehensive build metadata.

    Creates a JSON file containing detailed build information including:
    - Build metadata (version, timestamp, generator)
    - Project information (name, version, group)
    - Git information (branch, revision, tag, URL)
    - Build configuration (time, platform, architectures)
    - Environment details (OS, Python version, ccgo version)

    The JSON file is saved to platform-specific directories:
    - target/{platform}/build_info.json (e.g., target/android/build_info.json)
    - Falls back to project root if platform is not specified

    Args:
        project_name: Project name
        project_dir_path: Path to project root directory
        version_name: Project version string
        revision: Git commit hash
        path: Git branch name
        url: Git remote URL (anonymized)
        build_time: Build timestamp string
        ndk_revision: Android NDK version
        target_platform: Target platform (e.g., "android", "ios", "windows", etc.)

    Returns:
        str: JSON content that was written to file, or None if failed
    """
    # Parse timestamp
    timestamp = int(time.time())
    build_datetime = (
        datetime.strptime(build_time, "%Y-%m-%d %H:%M:%S")
        if " " in build_time
        else None
    )

    # Get full revision if available
    full_revision = ""
    try:
        result = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=project_dir_path,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode == 0:
            full_revision = result.stdout.strip()
    except Exception:
        pass

    # Get git tag if available
    git_tag = ""
    try:
        result = subprocess.run(
            ["git", "describe", "--tags", "--abbrev=0"],
            cwd=project_dir_path,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode == 0:
            git_tag = result.stdout.strip()
    except Exception:
        pass

    # Check if git working tree is dirty
    is_dirty = False
    try:
        result = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=project_dir_path,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode == 0:
            is_dirty = len(result.stdout.strip()) > 0
    except Exception:
        pass

    # Get ccgo version
    ccgo_version = ""
    try:
        # Try to import ccgo package and get version
        import importlib.metadata

        ccgo_version = importlib.metadata.version("ccgo")
    except Exception:
        ccgo_version = "unknown"

    # Build JSON structure
    build_info = {
        "build_metadata": {
            "version": "1.0",
            "generated_at": (
                datetime.now().isoformat() if build_datetime else build_time
            ),
            "generator": "ccgo",
        },
        "project": {
            "name": project_name,
            "version": version_name,
        },
        "git": {
            "branch": path,
            "revision": revision,
            "revision_full": full_revision if full_revision else revision,
            "tag": git_tag,
            "is_dirty": is_dirty,
            "remote_url": url,
        },
        "build": {
            "time": build_time,
            "timestamp": timestamp,
        },
        "environment": {
            "os": platform.system(),
            "os_version": platform.version(),
            "python_version": platform.python_version(),
            "ccgo_version": ccgo_version,
        },
    }

    # Add platform information if specified
    if target_platform:
        build_info["build"]["platform"] = target_platform

    # Add platform-specific build information based on target platform
    if target_platform:
        platform_lower = target_platform.lower()

        if platform_lower == "android" and ndk_revision:
            # Android-specific information
            build_info["build"]["android"] = {
                "ndk_version": ndk_revision,
                "stl": get_android_stl(project_dir_path),
                "min_sdk_version": get_android_min_sdk_version(project_dir_path),
            }
        elif platform_lower == "ohos":
            # OHOS-specific information
            err_code, ohos_revision = get_ohos_native_revision()
            if err_code == 0:
                build_info["build"]["ohos"] = {
                    "native_api_version": ohos_revision,
                    "stl": get_ohos_stl(project_dir_path),
                    "min_api_version": get_ohos_min_sdk_version(project_dir_path),
                }
        elif platform_lower == "ios":
            # iOS-specific information
            try:
                # Get Xcode version
                result = subprocess.run(
                    ["xcodebuild", "-version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                if result.returncode == 0:
                    xcode_info = result.stdout.strip().split("\n")
                    xcode_version = xcode_info[0].replace("Xcode ", "") if xcode_info else "unknown"
                    build_number = xcode_info[1].replace("Build version ", "") if len(xcode_info) > 1 else ""

                    build_info["build"]["ios"] = {
                        "xcode_version": xcode_version,
                        "xcode_build": build_number,
                    }
            except Exception:
                pass
        elif platform_lower == "watchos":
            # watchOS-specific information
            try:
                # Get Xcode version
                result = subprocess.run(
                    ["xcodebuild", "-version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                if result.returncode == 0:
                    xcode_info = result.stdout.strip().split("\n")
                    xcode_version = xcode_info[0].replace("Xcode ", "") if xcode_info else "unknown"
                    build_number = xcode_info[1].replace("Build version ", "") if len(xcode_info) > 1 else ""

                    build_info["build"]["watchos"] = {
                        "xcode_version": xcode_version,
                        "xcode_build": build_number,
                    }
            except Exception:
                pass
        elif platform_lower == "tvos":
            # tvOS-specific information
            try:
                # Get Xcode version
                result = subprocess.run(
                    ["xcodebuild", "-version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                if result.returncode == 0:
                    xcode_info = result.stdout.strip().split("\n")
                    xcode_version = xcode_info[0].replace("Xcode ", "") if xcode_info else "unknown"
                    build_number = xcode_info[1].replace("Build version ", "") if len(xcode_info) > 1 else ""

                    build_info["build"]["tvos"] = {
                        "xcode_version": xcode_version,
                        "xcode_build": build_number,
                    }
            except Exception:
                pass
        elif platform_lower == "macos":
            # macOS-specific information
            try:
                # Get Xcode version
                result = subprocess.run(
                    ["xcodebuild", "-version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                if result.returncode == 0:
                    xcode_info = result.stdout.strip().split("\n")
                    xcode_version = xcode_info[0].replace("Xcode ", "") if xcode_info else "unknown"
                    build_number = xcode_info[1].replace("Build version ", "") if len(xcode_info) > 1 else ""

                    build_info["build"]["macos"] = {
                        "xcode_version": xcode_version,
                        "xcode_build": build_number,
                    }
            except Exception:
                pass
        elif platform_lower == "windows":
            # Windows-specific information
            vs_tool_dir = os.getenv("VS140COMNTOOLS")
            if vs_tool_dir:
                build_info["build"]["windows"] = {
                    "visual_studio": "2015",
                    "vs_tools_path": vs_tool_dir,
                }
        elif platform_lower == "linux":
            # Linux-specific information
            try:
                # Get GCC version
                result = subprocess.run(
                    ["gcc", "--version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                )
                if result.returncode == 0:
                    gcc_version_line = result.stdout.strip().split("\n")[0]
                    # Extract version number from output
                    gcc_version = gcc_version_line.split()[-1] if gcc_version_line else "unknown"

                    build_info["build"]["linux"] = {
                        "compiler": "gcc",
                        "compiler_version": gcc_version,
                    }
            except Exception:
                pass

    # Determine output directory based on platform
    # Use cmake_build/{Platform}/ for platform-specific builds during build process
    # The file will be copied to target/{platform}/ by the build script after packaging
    if target_platform:
        # Map platform names to cmake build directories
        platform_build_dirs = {
            "android": "cmake_build/Android",
            "ios": "cmake_build/iOS",
            "watchos": "cmake_build/watchOS",
            "tvos": "cmake_build/tvOS",
            "macos": "cmake_build/macOS",
            "windows": "cmake_build/Windows",
            "linux": "cmake_build/Linux",
            "ohos": "cmake_build/OHOS",
            "tests": "cmake_build/Tests",
            "benches": "cmake_build/Benches",
            "docs": "cmake_build/Docs",
            "include": "cmake_build/Include",
        }

        # Get the build directory for this platform
        build_dir = platform_build_dirs.get(target_platform.lower(), f"cmake_build/{target_platform.capitalize()}")
        output_dir = os.path.join(project_dir_path, build_dir)
        os.makedirs(output_dir, exist_ok=True)
        json_file_path = os.path.join(output_dir, "build_info.json")
    else:
        json_file_path = os.path.join(project_dir_path, "build_info.json")
    json_content = json.dumps(build_info, indent=2, ensure_ascii=False)

    # Only write if content changed
    existing_content = ""
    if os.path.exists(json_file_path):
        try:
            with open(json_file_path, "r", encoding="utf-8") as f:
                existing_content = f.read()
        except Exception:
            pass

    if existing_content != json_content:
        with open(json_file_path, "w", encoding="utf-8") as f:
            f.write(json_content)
            f.flush()
        print(f"[SUCCESS] Generated build metadata: {json_file_path}")
    else:
        print(f"[SKIP] build_info.json is not changed")

    return json_content


def gen_project_revision_file(
    project_name,
    origin_version_file_path,
    version_name,
    incremental=False,
    json_output=None,
    platform=None,
):
    """
    Generate verinfo.h header file containing build version and git information.

    Creates a C/C++ header file with preprocessor macros defining:
    - Version number
    - Git revision (commit hash)
    - Git branch name
    - Git remote URL (anonymized)
    - Build timestamp
    - Git tag (auto-detected)
    - Android NDK/STL configuration

    The function also outputs a simple build description for CI/CD systems
    and optionally generates a JSON file with comprehensive build metadata.

    Args:
        project_name: Project name (used in macro names, uppercased)
        origin_version_file_path: Relative path where verinfo.h will be created
        version_name: Version string (e.g., "1.2.3")
        incremental: If True, only includes date (not time) in build timestamp
        json_output: If True, generates build_info.json. If None, reads from CCGO.toml (default: None)
        platform: Target platform (e.g., "android", "ios", "macos", "windows", "linux", "ohos")

    Note:
        - Only writes file if content has changed (avoids unnecessary rebuilds)
        - Retrieves git info from the version file directory
        - Git tag is auto-detected using 'git describe --tags --abbrev=0'
        - Outputs build description to stdout for CI/CD parsing
        - JSON output is saved to target/{platform}/build_info.json for platform-specific builds
        - Falls back to <project_root>/build_info.json if platform is not specified
        - JSON generation is controlled by [build.generate_json_metadata] in CCGO.toml
    """
    print(f"version name {version_name}")
    err_code, ndk_revision = get_ndk_revision()
    if err_code != 0:
        ndk_revision = ""
    # Use PROJECT_DIR (current working directory) instead of script directory
    project_dir_path = PROJECT_DIR
    version_file_path = os.path.join(project_dir_path, origin_version_file_path)
    os.makedirs(version_file_path, exist_ok=True)
    revision, path, url = parse_as_git(version_file_path)
    url = normalize_git_url(url)

    # Get git tag automatically
    git_tag = ""
    try:
        result = subprocess.run(
            ["git", "describe", "--tags", "--abbrev=0"],
            cwd=project_dir_path,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
        )
        if result.returncode == 0:
            git_tag = result.stdout.strip()
    except Exception:
        pass

    build_date = time.strftime("%Y-%m-%d", time.localtime(time.time()))
    build_time = (
        build_date
        if incremental
        else time.strftime("%Y-%m-%d %H:%M:%S", time.localtime(time.time()))
    )
    version_file_name = "verinfo.h"
    contents = """//
// {version_file_name}
// {origin_version_file_path}
//
// Create by ccgo on {build_date}
// Copyright {build_year} ccgo Project Authors. All rights reserved.

#ifndef {project_name}_BASE_VERINFO_H_
#define {project_name}_BASE_VERINFO_H_

#define CCGO_{project_name}_VERSION "{version_name}"
#define CCGO_{project_name}_REVISION "{revision}"
#define CCGO_{project_name}_PATH "{path}"
#define CCGO_{project_name}_URL "{url}"
#define CCGO_{project_name}_BUILD_TIME "{build_time}"
#define CCGO_{project_name}_TAG "{tag}"
#define CCGO_{project_name}_ANDROID_STL "{android_stl}"
#define CCGO_{project_name}_ANDROID_NDK_VERSION "{android_ndk_version}"
#define CCGO_{project_name}_ANDROID_MIN_SDK_VERSION "{android_min_sdk_version}"

#endif  // {project_name}_BASE_VERINFO_H_
""".format(
        version_file_name=version_file_name,
        origin_version_file_path=origin_version_file_path,
        build_date=build_date,
        build_year=build_date.partition("-")[0],
        project_name=project_name.upper(),
        version_name=version_name,
        revision=revision,
        path=path,
        url=url,
        build_time=build_time,
        tag=git_tag,
        android_stl=get_android_stl(project_dir_path),
        android_ndk_version=ndk_revision,
        android_min_sdk_version=get_android_min_sdk_version(project_dir_path),
    )
    file_path = os.path.join(version_file_path, version_file_name)
    file_content = ""
    if os.path.exists(file_path):
        with open(file_path, "r", encoding="utf-8") as f:
            file_content = f.read()
    if file_content != contents:
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(contents)
            f.flush()
    else:
        print(f"[SKIP]verinfo file {file_path} is not changed")

    # Generate JSON build metadata file
    # Read from CCGO.toml if json_output not explicitly specified
    if json_output is None:
        json_output = get_ccgo_config_value(
            project_dir_path, "build.generate_json_metadata", True
        )

    if json_output:
        json_content = _gen_build_info_json(
            project_name=project_name,
            project_dir_path=project_dir_path,
            version_name=version_name,
            revision=revision,
            path=path,
            url=url,
            build_time=build_time,
            ndk_revision=ndk_revision,
            target_platform=platform,
        )
        # Print JSON output for visibility
        if json_content:
            print("\n[[==BUILD_INFO_JSON==]]")
            print(json_content)
            print("[[==END_BUILD_INFO_JSON==]]")


def copy_build_info_to_target(platform_name, project_dir=None):
    """
    Copy build_info.json from cmake_build directory to target/{platform}/ directory.

    This function is called after the build artifacts are packaged to ensure
    build_info.json is included in the final distribution directory.

    Args:
        platform_name: Platform name (e.g., "android", "ios", "macos")
        project_dir: Project root directory (defaults to current directory)

    Returns:
        bool: True if copied successfully, False otherwise
    """
    if project_dir is None:
        project_dir = os.getcwd()

    # Map platform names to cmake build directories
    platform_build_dirs = {
        "android": "cmake_build/Android",
        "ios": "cmake_build/iOS",
        "macos": "cmake_build/macOS",
        "windows": "cmake_build/Windows",
        "linux": "cmake_build/Linux",
        "ohos": "cmake_build/OHOS",
        "tests": "cmake_build/Tests",
        "benches": "cmake_build/Benches",
        "docs": "cmake_build/Docs",
        "include": "cmake_build/Include",
    }

    build_dir = platform_build_dirs.get(platform_name.lower(), f"cmake_build/{platform_name.capitalize()}")
    build_info_src = os.path.join(project_dir, build_dir, "build_info.json")

    if not os.path.exists(build_info_src):
        return False

    # Create target/{platform} directory
    target_platform_dir = os.path.join(project_dir, "target", platform_name.lower())
    os.makedirs(target_platform_dir, exist_ok=True)

    # Copy build_info.json
    build_info_dst = os.path.join(target_platform_dir, "build_info.json")
    shutil.copy2(build_info_src, build_info_dst)

    print(f"[SUCCESS] Copied build_info.json to target/{platform_name.lower()}/")
    return True


def system_is_windows():
    """Check if current platform is Windows."""
    return platform.system().lower() == "windows"


def system_is_macos():
    """Check if current platform is macOS/Darwin."""
    return platform.system().lower() == "darwin"


def system_architecture_is64():
    """Check if current system architecture is 64-bit."""
    return platform.machine().endswith("64")


def get_gradle_file_path(project_path):
    """
    Get Gradle build file path, preferring Kotlin DSL (.kts) over Groovy.

    Args:
        project_path: Path to project root directory

    Returns:
        str: Path to build.gradle.kts or build.gradle
    """
    gradle_file = f"{project_path}/build.gradle.kts"
    if not os.path.exists(gradle_file):
        gradle_file = f"{project_path}/build.gradle"
    return gradle_file


def get_version_file_path(project_path):
    """
    Get path to Gradle version catalog file (libs.versions.toml).

    Args:
        project_path: Path to project root directory

    Returns:
        str: Path to android/gradle/libs.versions.toml
    """
    return f"{project_path}/android/gradle/libs.versions.toml"


def get_value_from_toml_pure(toml_path, key, not_found_value=None):
    """
    Parse TOML file manually without using tomllib (for Python < 3.11).

    Implements a simple TOML parser that handles section headers and key-value pairs.

    Args:
        toml_path: Path to TOML file
        key: Key to lookup (supports dot notation like "versions.commMainProject")
        not_found_value: Default value if key not found

    Returns:
        Value from TOML file or not_found_value if not found

    Note:
        This is a fallback parser for Python versions < 3.11 that lack tomllib.
        Handles basic TOML syntax but not all advanced features.
    """
    with open(toml_path, "r") as f:
        content = f.read()
    if "." not in key:
        key = "versions." + key
    keys = key.split(".")
    current_section = None
    for line in content.splitlines():
        line = line.strip()
        if line.startswith("[") and line.endswith("]"):
            current_section = line[1:-1]
        elif "=" in line:
            k, v = line.split("=", 1)
            k = k.strip()
            v = v.strip().strip('"')
            if (
                (not current_section)
                or (len(keys) == 1)
                or (current_section == keys[0])
            ):
                if len(keys) == 1 and k == keys[0]:
                    return v
                elif k == keys[1]:
                    return v
    return not_found_value


def get_value_from_toml_by_lib(toml_path, key, not_found_value=None):
    """
    Parse TOML file using standard library tomllib (Python >= 3.11).

    Args:
        toml_path: Path to TOML file
        key: Key to lookup (supports dot notation like "versions.commMainProject")
        not_found_value: Default value if key not found

    Returns:
        Value from TOML file

    Note:
        Must open file in binary mode ('rb') for tomllib.
    """
    # Must open in rb mode for tomllib
    with open(toml_path, "rb") as f:
        data = tomllib.load(f)
    if "." not in key:
        key = "versions." + key
    keys = key.split(".")
    for k in keys:
        data = data[k]
    return data


def get_value_from_toml(toml_path, key, not_found_value=None):
    """
    Parse TOML file using best available method (tomllib or pure Python parser).

    Automatically selects tomllib (Python >= 3.11) or falls back to manual parser.

    Args:
        toml_path: Path to TOML file
        key: Key to lookup (supports dot notation like "versions.commMainProject")
        not_found_value: Default value if key not found

    Returns:
        Value from TOML file or not_found_value if not found
    """
    if tomllib is not None:
        return get_value_from_toml_by_lib(toml_path, key, not_found_value)
    return get_value_from_toml_pure(toml_path, key, not_found_value)


def get_version_file_value(project_path, key):
    """
    Get a configuration value from the project's version catalog file.

    Args:
        project_path: Path to project root directory
        key: Configuration key to lookup (e.g., "commMainProject", "minSdkVersion")

    Returns:
        Value from gradle/libs.versions.toml or None if not found
    """
    return get_value_from_toml(get_version_file_path(project_path), key)


def get_version_name(project_path):
    """
    Get the project version string from gradle/libs.versions.toml.

    Reads the 'commMainProject' key from the version catalog.

    Args:
        project_path: Path to project root directory

    Returns:
        str: Version string (e.g., "1.2.3")
    """
    # Get from gradle/libs.versions.toml file
    # commMainProject = "x.x.x"
    return get_version_file_value(project_path, "commMainProject")


def get_ccgo_config_value(project_path, key, default_value=None):
    """
    Get a configuration value from the project's CCGO.toml file.

    Args:
        project_path: Path to project root directory
        key: Configuration key to lookup (supports dot notation like "build.generate_json_metadata")
        default_value: Default value if key not found

    Returns:
        Value from CCGO.toml or default_value if not found
    """
    ccgo_toml_path = os.path.join(project_path, "CCGO.toml")
    if not os.path.exists(ccgo_toml_path):
        return default_value
    try:
        return get_value_from_toml(ccgo_toml_path, key, default_value)
    except Exception:
        return default_value


def get_android_stl(project_path):
    """
    Get the Android STL (Standard Template Library) type from version catalog.

    Args:
        project_path: Path to project root directory

    Returns:
        str: STL type (e.g., "c++_shared", "c++_static")
    """
    return get_version_file_value(project_path, "commAndroidStl")


def get_android_min_sdk_version(project_path):
    """
    Get the minimum Android SDK version from version catalog.

    Args:
        project_path: Path to project root directory

    Returns:
        str: Minimum SDK version (e.g., "21")
    """
    return get_version_file_value(project_path, "minSdkVersion")


def get_ohos_min_sdk_version(project_path):
    """
    Get the minimum OHOS (HarmonyOS) API level.

    Args:
        project_path: Path to project root directory (unused but kept for API consistency)

    Returns:
        str: Minimum API level ("10")
    """
    # API 10
    return "10"


def get_ohos_stl(project_path):
    """
    Get the OHOS STL type (same as Android STL configuration).

    Args:
        project_path: Path to project root directory

    Returns:
        str: STL type (e.g., "c++_shared", "c++_static")
    """
    # Same as Android
    return get_version_file_value(project_path, "commAndroidStl")


def check_vs_env():
    """
    Check and initialize Visual Studio 2015 build environment.

    Verifies VS140COMNTOOLS environment variable is set and executes
    vcvarsall.bat to setup compiler environment.

    Returns:
        bool: True if Visual Studio 2015 is available, False otherwise

    Note:
        Requires Visual Studio 2015 (VS140) to be installed.
        Prints error message if VS not found.
    """
    vs_tool_dir = os.getenv("VS140COMNTOOLS")

    if not vs_tool_dir:
        print("You must install visual studio 2015 for build.")
        return False

    print("vs.dir: " + vs_tool_dir)
    envbat = vs_tool_dir + "../../vc/vcvarsall.bat"
    print("vsvar.dir: " + envbat)
    p = subprocess.Popen(envbat)
    p.wait()

    return True


def merge_win_static_libs(src_libs, dst_lib):
    """
    Merge multiple Windows static libraries into a single library using lib.exe.

    Uses Visual Studio's lib.exe tool to combine .lib files.

    Args:
        src_libs: List of source library file paths to merge
        dst_lib: Destination library file path

    Returns:
        bool: True if merge succeeded, False otherwise

    Note:
        Requires Visual Studio 2015 (VS140) environment variables to be set.
    """
    vs_tool_dir = os.getenv("VS140COMNTOOLS")
    lib_cmd = vs_tool_dir + "/../../VC/bin/lib.exe"
    print("lib cmd:" + lib_cmd)

    src_libs.insert(0, "/OUT:" + dst_lib)
    src_libs.insert(0, lib_cmd)

    p = subprocess.Popen(src_libs)
    p.wait()
    if p.returncode != 0:
        print(f"!!!!!!!!!!!lib.exe {dst_lib} fail!!!!!!!!!!!!!!!")
        return False

    return True


def merge_llvm_static_libs(src_libs, dst_lib):
    """
    Merge multiple MSVC-compatible static libraries using llvm-lib.

    Uses LLVM's llvm-lib tool to combine .lib files (COFF format).
    This is used for MSVC-compatible cross-compilation from Linux.

    Args:
        src_libs: List of source library file paths to merge
        dst_lib: Destination library file path

    Returns:
        bool: True if merge succeeded, False otherwise

    Note:
        Requires llvm-lib to be installed and in PATH.
    """
    # Detect which lib tool to use
    lib_tool = None
    if shutil.which("llvm-lib"):
        lib_tool = "llvm-lib"
    elif shutil.which("llvm-lib-19"):
        lib_tool = "llvm-lib-19"
    else:
        print("!!!!!!!!!!!llvm-lib tool not found!!!!!!!!!!!!!!!")
        return False

    print(f"Using lib tool: {lib_tool}")

    # Ensure destination directory exists
    dst_dir = os.path.dirname(dst_lib)
    if dst_dir:
        os.makedirs(dst_dir, exist_ok=True)

    # llvm-lib syntax: llvm-lib /out:output.lib input1.lib input2.lib ...
    cmd = [lib_tool, f"/out:{dst_lib}"] + src_libs
    print(f"Merge command: {' '.join(cmd)}")

    p = subprocess.Popen(cmd)
    p.wait()
    if p.returncode != 0:
        print(f"!!!!!!!!!!!llvm-lib {dst_lib} fail!!!!!!!!!!!!!!!")
        return False

    return True


def merge_mingw_static_libs(src_libs, dst_lib):
    """
    Merge multiple MinGW static libraries into a single library using ar.

    Uses MinGW's ar tool to combine .a files without requiring gcc.
    This method extracts all object files from source libraries and
    repackages them into a single archive.

    Args:
        src_libs: List of source library file paths to merge
        dst_lib: Destination library file path

    Returns:
        bool: True if merge succeeded, False otherwise

    Note:
        Works in MinGW cross-compilation environments where only
        x86_64-w64-mingw32-ar is available (no gcc).
    """
    import tempfile

    # Detect which ar tool to use
    # Try MinGW ar first, then fall back to system ar
    ar_tool = "ar"
    if shutil.which("x86_64-w64-mingw32-ar"):
        ar_tool = "x86_64-w64-mingw32-ar"
    elif shutil.which("ar"):
        ar_tool = "ar"
    else:
        print("!!!!!!!!!!!ar tool not found!!!!!!!!!!!!!!!")
        return False

    print(f"Using ar tool: {ar_tool}")

    # Create a temporary directory for extracting object files
    with tempfile.TemporaryDirectory() as temp_dir:
        # Ensure destination directory exists
        dst_dir = os.path.dirname(dst_lib)
        if dst_dir:
            os.makedirs(dst_dir, exist_ok=True)

        # Extract all object files from source libraries
        for lib in src_libs:
            if not os.path.exists(lib):
                print(f"Warning: Library not found: {lib}")
                continue

            # Use ar to extract object files from this library
            # Change to temp directory to extract files there
            extract_cmd = f"cd {temp_dir} && {ar_tool} x {os.path.abspath(lib)}"
            print(f"Extracting: {extract_cmd}")
            ret = os.system(extract_cmd)
            if ret != 0:
                print(f"!!!!!!!!!!!{ar_tool} extract from {lib} failed!!!!!!!!!!!!!!!")
                return False

        # List all extracted object files
        # Try both .o and .obj extensions (MinGW may use either)
        obj_files = glob.glob(os.path.join(temp_dir, "*.o"))
        obj_files.extend(glob.glob(os.path.join(temp_dir, "*.obj")))

        if not obj_files:
            # Debug: List all files in temp directory
            all_files = os.listdir(temp_dir)
            print(f"Debug: Files in temp directory: {all_files}")
            print(f"!!!!!!!!!!!No object files extracted!!!!!!!!!!!!!!!")
            return False

        print(f"Extracted {len(obj_files)} object files")

        # Create new archive from all object files
        # Use relative paths by changing to temp directory
        obj_names = [os.path.basename(f) for f in obj_files]
        obj_list_str = " ".join(obj_names)

        # Create the merged library
        create_cmd = f"cd {temp_dir} && {ar_tool} crs {os.path.abspath(dst_lib)} {obj_list_str}"
        print(f"Creating merged library: {create_cmd}")
        ret = os.system(create_cmd)
        if ret != 0:
            print(f"!!!!!!!!!!!{ar_tool} create {dst_lib} failed!!!!!!!!!!!!!!!")
            return False

    print(f"Successfully merged {len(src_libs)} libraries into {dst_lib}")
    return True


def copy_windows_pdb(cmake_out, sub_folder, config, dst_folder):
    """
    Copy Windows PDB (Program Database) debug symbol files.

    Searches for .pdb files in CMake output subdirectories and copies them
    to the destination folder. PDB files are needed for debugging on Windows.

    Args:
        cmake_out: CMake output root directory
        sub_folder: List of subdirectory names to search
        config: Build configuration (e.g., "Debug", "Release")
        dst_folder: Destination directory for PDB files

    Note:
        Prints warnings if expected PDB files are not found.
    """
    for sf in sub_folder:
        src_file = f"{cmake_out}/{sf}/{config}"
        pdbs = glob.glob(src_file + "/*.pdb")
        if len(pdbs) != 1:
            print(f"Warning: {src_file} path error.")
            continue
        else:
            print(f"start copy {pdbs[0]}")
        pdb = pdbs[0]
        if os.path.isfile(pdb):
            shutil.copy(pdb, dst_folder)
        else:
            print(f"{pdb} not exists")


def merge_files_ends_with(src_dir, suffix, out_file):
    """
    Concatenate all files with a given suffix into a single output file.

    Reads all matching files in binary mode and writes them sequentially
    to the output file.

    Args:
        src_dir: Source directory to search for files
        suffix: File suffix to match (e.g., ".txt", ".log")
        out_file: Output file path for merged content

    Note:
        Creates parent directories for out_file if they don't exist.
    """
    # List of files to merge
    file_list = glob.glob(f"{src_dir}/*{suffix}")
    # Open the output file for writing
    if out_file.rfind("/") != -1 and not os.path.exists(
        out_file[: out_file.rfind("/")]
    ):
        os.makedirs(out_file[: out_file.rfind("/")], exist_ok=True)
    with open(out_file, "wb") as outfile:
        # Loop through the input files and write their contents to the output file
        for filename in file_list:
            with open(filename, "rb") as infile:
                outfile.write(infile.read())


def zip_files_ends_with(src_dir, suffix, out_file):
    """
    Create a ZIP archive containing all files with a given suffix.

    Args:
        src_dir: Source directory to search for files
        suffix: File suffix to match (e.g., ".so", ".a")
        out_file: Output ZIP file path

    Note:
        Files are stored with their base names only (no directory structure).
        Creates parent directories for out_file if they don't exist.
    """
    # List of files to zip
    file_list = glob.glob(f"{src_dir}/*{suffix}")
    # Open the output file for writing
    if out_file.rfind("/") != -1 and not os.path.exists(
        out_file[: out_file.rfind("/")]
    ):
        os.makedirs(out_file[: out_file.rfind("/")], exist_ok=True)
    # Zip all the files
    with zipfile.ZipFile(out_file, "w") as zipf:
        for filename in file_list:
            zipf.write(filename, os.path.basename(filename))


# =============================================================================
# Library File Information Parsing
# =============================================================================

# ELF e_machine values
ELF_MACHINE_MAP = {
    0x03: "x86",
    0x3E: "x86_64",
    0x28: "arm",
    0xB7: "aarch64",
    0x08: "mips",
    0xF3: "riscv",
}

# Mach-O CPU types
MACHO_CPU_MAP = {
    0x00000007: "x86",
    0x01000007: "x86_64",
    0x0000000C: "arm",
    0x0100000C: "arm64",
}

# PE/COFF Machine types
PE_MACHINE_MAP = {
    0x014c: "x86",
    0x8664: "x64",
    0xAA64: "ARM64",
    0x01c0: "ARM",
    0x01c4: "ARMv7",
}


def _parse_elf_arch(data):
    """
    Parse ELF file to get architecture.

    Args:
        data: File bytes

    Returns:
        str: Architecture name or None
    """
    if len(data) < 20:
        return None

    # Check ELF magic
    if data[:4] != b'\x7fELF':
        return None

    # Get class (32 or 64 bit)
    elf_class = data[4]  # 1 = 32-bit, 2 = 64-bit

    # Get endianness
    endian = '<' if data[5] == 1 else '>'  # 1 = little, 2 = big

    # e_machine is at offset 18 (2 bytes)
    import struct
    e_machine = struct.unpack(f'{endian}H', data[18:20])[0]

    return ELF_MACHINE_MAP.get(e_machine, f"unknown(0x{e_machine:X})")


def _parse_elf_android_ndk_info(data):
    """
    Parse Android NDK info from ELF .note.android.ident section.

    Args:
        data: Complete ELF file bytes

    Returns:
        dict: {ndk_version: str, api_level: int} or None
    """
    import struct

    if len(data) < 64 or data[:4] != b'\x7fELF':
        return None

    # Get ELF class and endianness
    elf_class = data[4]  # 1 = 32-bit, 2 = 64-bit
    endian = '<' if data[5] == 1 else '>'

    # Parse ELF header to find section headers
    if elf_class == 1:  # 32-bit
        # e_shoff at offset 32, e_shentsize at 46, e_shnum at 48, e_shstrndx at 50
        if len(data) < 52:
            return None
        e_shoff = struct.unpack(f'{endian}I', data[32:36])[0]
        e_shentsize = struct.unpack(f'{endian}H', data[46:48])[0]
        e_shnum = struct.unpack(f'{endian}H', data[48:50])[0]
        e_shstrndx = struct.unpack(f'{endian}H', data[50:52])[0]
        sh_struct = f'{endian}IIIIIIIIII'  # 10 x 4 bytes = 40 bytes
        sh_name_off, sh_offset_off, sh_size_off = 0, 16, 20
    else:  # 64-bit
        # e_shoff at offset 40, e_shentsize at 58, e_shnum at 60, e_shstrndx at 62
        if len(data) < 64:
            return None
        e_shoff = struct.unpack(f'{endian}Q', data[40:48])[0]
        e_shentsize = struct.unpack(f'{endian}H', data[58:60])[0]
        e_shnum = struct.unpack(f'{endian}H', data[60:62])[0]
        e_shstrndx = struct.unpack(f'{endian}H', data[62:64])[0]
        sh_struct = f'{endian}IIQQQQIIQQ'  # varies
        sh_name_off, sh_offset_off, sh_size_off = 0, 24, 32

    if e_shoff == 0 or e_shnum == 0:
        return None

    # Read section header string table
    shstrtab_off = e_shoff + e_shstrndx * e_shentsize
    if shstrtab_off + e_shentsize > len(data):
        return None

    if elf_class == 1:
        strtab_offset = struct.unpack(f'{endian}I', data[shstrtab_off + 16:shstrtab_off + 20])[0]
        strtab_size = struct.unpack(f'{endian}I', data[shstrtab_off + 20:shstrtab_off + 24])[0]
    else:
        strtab_offset = struct.unpack(f'{endian}Q', data[shstrtab_off + 24:shstrtab_off + 32])[0]
        strtab_size = struct.unpack(f'{endian}Q', data[shstrtab_off + 32:shstrtab_off + 40])[0]

    if strtab_offset + strtab_size > len(data):
        return None

    strtab = data[strtab_offset:strtab_offset + strtab_size]

    # Find .note.android.ident section
    target_section = b'.note.android.ident'
    for i in range(e_shnum):
        sh_off = e_shoff + i * e_shentsize
        if sh_off + e_shentsize > len(data):
            continue

        if elf_class == 1:
            sh_name = struct.unpack(f'{endian}I', data[sh_off:sh_off + 4])[0]
            sec_offset = struct.unpack(f'{endian}I', data[sh_off + 16:sh_off + 20])[0]
            sec_size = struct.unpack(f'{endian}I', data[sh_off + 20:sh_off + 24])[0]
        else:
            sh_name = struct.unpack(f'{endian}I', data[sh_off:sh_off + 4])[0]
            sec_offset = struct.unpack(f'{endian}Q', data[sh_off + 24:sh_off + 32])[0]
            sec_size = struct.unpack(f'{endian}Q', data[sh_off + 32:sh_off + 40])[0]

        # Get section name from string table
        if sh_name >= len(strtab):
            continue
        name_end = strtab.find(b'\x00', sh_name)
        if name_end == -1:
            name_end = len(strtab)
        section_name = strtab[sh_name:name_end]

        if section_name == target_section:
            # Found .note.android.ident section
            if sec_offset + sec_size > len(data):
                return None

            note_data = data[sec_offset:sec_offset + sec_size]
            return _parse_android_note(note_data, endian)

    return None


def _parse_android_note(note_data, endian='<'):
    """
    Parse Android note section content.

    Format:
        uint32_t namesz (8 for "Android\0")
        uint32_t descsz
        uint32_t type (1)
        char name[8] "Android\0"
        uint16_t api_level
        uint16_t padding (usually NDK major version in newer NDKs)
        char ndk_version[] (null-terminated, e.g., "r27")
        char ndk_build[] (null-terminated, e.g., "12117859")
    """
    import struct

    if len(note_data) < 16:
        return None

    namesz = struct.unpack(f'{endian}I', note_data[0:4])[0]
    descsz = struct.unpack(f'{endian}I', note_data[4:8])[0]
    # note_type = struct.unpack(f'{endian}I', note_data[8:12])[0]

    # Name is aligned to 4 bytes
    name_aligned = (namesz + 3) & ~3
    desc_start = 12 + name_aligned

    if desc_start + descsz > len(note_data):
        return None

    # Check name is "Android"
    name = note_data[12:12 + namesz].rstrip(b'\x00')
    if name != b'Android':
        return None

    desc = note_data[desc_start:desc_start + descsz]
    if len(desc) < 4:
        return None

    # Parse descriptor
    api_level = struct.unpack(f'{endian}H', desc[0:2])[0]

    # Find NDK version string (starts after api_level + padding)
    # Format varies by NDK version, try to find "r" followed by version
    ndk_version = None
    try:
        # Skip first 4 bytes (api_level + ndk_major_version or padding)
        remaining = desc[4:]
        # Find null-terminated strings
        strings = []
        current = b''
        for byte in remaining:
            if byte == 0:
                if current:
                    strings.append(current.decode('ascii', errors='ignore'))
                    current = b''
            else:
                current += bytes([byte])
        if current:
            strings.append(current.decode('ascii', errors='ignore'))

        # First string starting with 'r' is likely NDK version
        for s in strings:
            if s.startswith('r') and len(s) <= 10:
                ndk_version = s
                break
    except Exception:
        pass

    return {
        'api_level': api_level,
        'ndk_version': ndk_version
    }


def _parse_macho_arch(data):
    """
    Parse Mach-O file to get architecture(s).

    Args:
        data: File bytes

    Returns:
        str: Architecture name(s) or None
    """
    import struct

    if len(data) < 8:
        return None

    magic = struct.unpack('<I', data[:4])[0]

    # Check for fat binary (universal)
    if magic == 0xCAFEBABE or magic == 0xBEBAFECA:
        # Fat binary - big endian header
        if len(data) < 8:
            return None
        nfat = struct.unpack('>I', data[4:8])[0]
        if nfat > 10:  # Sanity check
            return None

        archs = []
        for i in range(nfat):
            offset = 8 + i * 20
            if offset + 8 > len(data):
                break
            cputype = struct.unpack('>I', data[offset:offset + 4])[0]
            arch = MACHO_CPU_MAP.get(cputype)
            if arch and arch not in archs:
                archs.append(arch)

        return ', '.join(archs) if archs else None

    # Single architecture Mach-O
    # 0xFEEDFACE = 32-bit, 0xFEEDFACF = 64-bit
    # 0xCEFAEDFE = 32-bit swapped, 0xCFFAEDFE = 64-bit swapped
    if magic in (0xFEEDFACE, 0xFEEDFACF):
        endian = '<'
    elif magic in (0xCEFAEDFE, 0xCFFAEDFE):
        endian = '>'
    else:
        return None

    if len(data) < 8:
        return None

    cputype = struct.unpack(f'{endian}I', data[4:8])[0]
    return MACHO_CPU_MAP.get(cputype, f"unknown(0x{cputype:X})")


def _parse_pe_arch(data):
    """
    Parse PE/COFF file to get architecture.

    Args:
        data: File bytes

    Returns:
        str: Architecture name or None
    """
    import struct

    if len(data) < 64:
        return None

    # Check for MZ header (PE)
    if data[:2] == b'MZ':
        # Get PE header offset from offset 0x3C
        pe_offset = struct.unpack('<I', data[0x3C:0x40])[0]
        if pe_offset + 6 > len(data):
            return None
        # Check PE signature
        if data[pe_offset:pe_offset + 4] != b'PE\x00\x00':
            return None
        # Machine type is 2 bytes after PE signature
        machine = struct.unpack('<H', data[pe_offset + 4:pe_offset + 6])[0]
        return PE_MACHINE_MAP.get(machine, f"unknown(0x{machine:X})")

    # Check for COFF (no MZ header, starts with machine type)
    # COFF files start directly with the COFF header
    machine = struct.unpack('<H', data[0:2])[0]
    if machine in PE_MACHINE_MAP:
        return PE_MACHINE_MAP[machine]

    return None


def _parse_ar_member_arch(data):
    """
    Parse AR archive to get architecture from first object file.

    Args:
        data: File bytes

    Returns:
        str: Architecture name or None
    """
    if len(data) < 68:  # AR header + file header
        return None

    # Check AR magic
    if data[:8] != b'!<arch>\n':
        return None

    # Skip AR global header, find first file
    pos = 8
    while pos + 60 < len(data):
        # AR file header is 60 bytes
        # name[16], date[12], uid[6], gid[6], mode[8], size[10], magic[2]
        header = data[pos:pos + 60]
        if header[58:60] != b'`\n':
            break

        try:
            size = int(header[48:58].strip())
        except ValueError:
            break

        file_start = pos + 60
        file_data = data[file_start:file_start + min(size, 512)]

        # Skip symbol table entries (/ and //)
        name = header[:16].rstrip()
        if name in (b'/', b'//', b'__.SYMDEF', b'__.SYMDEF SORTED'):
            pos = file_start + size
            if size % 2:
                pos += 1  # AR entries are 2-byte aligned
            continue

        # Try to parse the member
        if file_data[:4] == b'\x7fELF':
            return _parse_elf_arch(file_data)
        elif file_data[:4] in (b'\xfe\xed\xfa\xce', b'\xfe\xed\xfa\xcf',
                               b'\xce\xfa\xed\xfe', b'\xcf\xfa\xed\xfe'):
            return _parse_macho_arch(file_data)
        elif len(file_data) >= 2:
            import struct
            machine = struct.unpack('<H', file_data[0:2])[0]
            if machine in PE_MACHINE_MAP:
                return PE_MACHINE_MAP[machine]

        break

    return None


def get_library_info(data, filename, file_path=""):
    """
    Get detailed information about a library file.

    Args:
        data: File bytes
        filename: File name
        file_path: Full path in archive (for context like platform detection)

    Returns:
        str: Information string like "[x86_64]" or "[aarch64, NDK r27, API 24]" or ""
    """
    if not data or len(data) < 4:
        return ""

    info_parts = []

    # Detect format and parse
    magic = data[:4]

    # ELF
    if magic == b'\x7fELF':
        arch = _parse_elf_arch(data)
        if arch:
            info_parts.append(arch)

        # Check for Android .so files
        if filename.endswith('.so') and 'android' in file_path.lower():
            ndk_info = _parse_elf_android_ndk_info(data)
            if ndk_info:
                if ndk_info.get('ndk_version'):
                    info_parts.append(f"NDK {ndk_info['ndk_version']}")
                if ndk_info.get('api_level'):
                    info_parts.append(f"API {ndk_info['api_level']}")

    # Mach-O
    elif magic in (b'\xfe\xed\xfa\xce', b'\xfe\xed\xfa\xcf',
                   b'\xce\xfa\xed\xfe', b'\xcf\xfa\xed\xfe',
                   b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca'):
        arch = _parse_macho_arch(data)
        if arch:
            info_parts.append(arch)

    # PE (MZ header)
    elif magic[:2] == b'MZ':
        arch = _parse_pe_arch(data)
        if arch:
            info_parts.append(arch)

    # AR archive
    elif data[:8] == b'!<arch>\n':
        arch = _parse_ar_member_arch(data)
        if arch:
            info_parts.append(arch)

    # COFF (Windows .lib without MZ)
    elif len(data) >= 2:
        import struct
        machine = struct.unpack('<H', data[0:2])[0]
        if machine in PE_MACHINE_MAP:
            info_parts.append(PE_MACHINE_MAP[machine])

    if info_parts:
        return f" [{', '.join(info_parts)}]"
    return ""


def _is_library_file(filename, file_path):
    """
    Check if a file is a library file that should be analyzed.

    Args:
        filename: File name
        file_path: Full path in archive

    Returns:
        bool: True if this is a library file
    """
    # Check if in lib or frameworks directory
    if not any(d in file_path.lower() for d in ['lib/', 'frameworks/']):
        return False

    # Check extensions
    lib_extensions = ('.a', '.so', '.dylib', '.dll', '.lib')
    if filename.endswith(lib_extensions):
        return True

    # macOS framework binary (no extension, inside .framework)
    if '.framework/' in file_path and '.' not in filename:
        return True

    return False


def _get_library_info_dict(data, filename, file_path=""):
    """
    Get detailed information about a library file as a dictionary.

    Args:
        data: File bytes
        filename: File name
        file_path: Full path in archive (for context like platform detection)

    Returns:
        dict: Library information or None
    """
    if not data or len(data) < 4:
        return None

    info = {}
    magic = data[:4]

    # ELF
    if magic == b'\x7fELF':
        info['format'] = 'ELF'
        arch = _parse_elf_arch(data)
        if arch:
            info['arch'] = arch

        # Check for Android .so files
        if filename.endswith('.so') and 'android' in file_path.lower():
            ndk_info = _parse_elf_android_ndk_info(data)
            if ndk_info:
                if ndk_info.get('ndk_version'):
                    info['ndk_version'] = ndk_info['ndk_version']
                if ndk_info.get('api_level'):
                    info['api_level'] = ndk_info['api_level']

    # Mach-O
    elif magic in (b'\xfe\xed\xfa\xce', b'\xfe\xed\xfa\xcf',
                   b'\xce\xfa\xed\xfe', b'\xcf\xfa\xed\xfe',
                   b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca'):
        info['format'] = 'Mach-O'
        arch = _parse_macho_arch(data)
        if arch:
            info['arch'] = arch

    # PE (MZ header)
    elif magic[:2] == b'MZ':
        info['format'] = 'PE'
        arch = _parse_pe_arch(data)
        if arch:
            info['arch'] = arch

    # AR archive
    elif data[:8] == b'!<arch>\n':
        info['format'] = 'AR'
        arch = _parse_ar_member_arch(data)
        if arch:
            info['arch'] = arch

    # COFF (Windows .lib without MZ)
    elif len(data) >= 2:
        import struct
        machine = struct.unpack('<H', data[0:2])[0]
        if machine in PE_MACHINE_MAP:
            info['format'] = 'COFF'
            info['arch'] = PE_MACHINE_MAP[machine]

    return info if info else None


def generate_archive_info(zip_path, output_path=None):
    """
    Generate archive_info.json with detailed information about archive contents.

    Args:
        zip_path: Path to the ZIP file
        output_path: Path to output JSON file. If None, uses same directory as zip_path.

    Returns:
        str: Path to generated JSON file, or None on error
    """
    import json
    from datetime import datetime

    if not os.path.exists(zip_path):
        return None

    if output_path is None:
        output_path = os.path.join(os.path.dirname(zip_path), 'archive_info.json')

    try:
        with zipfile.ZipFile(zip_path, 'r') as zf:
            archive_info = {
                'archive_metadata': {
                    'version': '1.0',
                    'generated_at': datetime.now().isoformat(),
                    'archive_name': os.path.basename(zip_path),
                    'archive_size': os.path.getsize(zip_path),
                },
                'files': [],
                'libraries': [],
                'summary': {
                    'total_files': 0,
                    'total_size': 0,
                    'library_count': 0,
                    'platforms': [],
                    'architectures': [],
                }
            }

            platforms = set()
            architectures = set()

            for info in zf.infolist():
                if info.filename.endswith('/'):
                    continue  # Skip directories

                file_entry = {
                    'path': info.filename,
                    'size': info.file_size,
                    'compressed_size': info.compress_size,
                }

                archive_info['files'].append(file_entry)
                archive_info['summary']['total_files'] += 1
                archive_info['summary']['total_size'] += info.file_size

                # Check if this is a library file
                filename = os.path.basename(info.filename)
                if _is_library_file(filename, info.filename):
                    try:
                        data = zf.read(info.filename)
                        lib_info = _get_library_info_dict(data, filename, info.filename)

                        lib_entry = {
                            'path': info.filename,
                            'name': filename,
                            'size': info.file_size,
                        }

                        if lib_info:
                            lib_entry.update(lib_info)
                            if 'arch' in lib_info:
                                # Handle multi-arch (comma-separated)
                                for arch in lib_info['arch'].split(', '):
                                    architectures.add(arch)

                        # Detect platform from path
                        path_lower = info.filename.lower()
                        if '/android/' in path_lower:
                            lib_entry['platform'] = 'android'
                            platforms.add('android')
                        elif '/ios/' in path_lower:
                            lib_entry['platform'] = 'ios'
                            platforms.add('ios')
                        elif '/macos/' in path_lower:
                            lib_entry['platform'] = 'macos'
                            platforms.add('macos')
                        elif '/windows/' in path_lower:
                            lib_entry['platform'] = 'windows'
                            platforms.add('windows')
                        elif '/linux/' in path_lower:
                            lib_entry['platform'] = 'linux'
                            platforms.add('linux')
                        elif '/ohos/' in path_lower:
                            lib_entry['platform'] = 'ohos'
                            platforms.add('ohos')
                        elif '/tvos/' in path_lower:
                            lib_entry['platform'] = 'tvos'
                            platforms.add('tvos')
                        elif '/watchos/' in path_lower:
                            lib_entry['platform'] = 'watchos'
                            platforms.add('watchos')

                        # Detect link type from path
                        if '/static/' in path_lower:
                            lib_entry['link_type'] = 'static'
                        elif '/shared/' in path_lower:
                            lib_entry['link_type'] = 'shared'

                        # Detect toolchain from path (Windows)
                        if '/mingw/' in path_lower:
                            lib_entry['toolchain'] = 'mingw'
                        elif '/msvc/' in path_lower:
                            lib_entry['toolchain'] = 'msvc'

                        archive_info['libraries'].append(lib_entry)
                        archive_info['summary']['library_count'] += 1

                    except Exception:
                        pass

            archive_info['summary']['platforms'] = sorted(list(platforms))
            archive_info['summary']['architectures'] = sorted(list(architectures))

            # Write JSON file
            with open(output_path, 'w', encoding='utf-8') as f:
                json.dump(archive_info, f, indent=2, ensure_ascii=False)

            return output_path

    except Exception as e:
        print(f"Error generating archive info: {e}")
        return None


def print_zip_tree(zip_path, indent="    ", generate_info_file=True):
    """
    Print the tree structure of a ZIP file with library file details.

    Args:
        zip_path: Path to the ZIP file
        indent: Base indentation string (default: 4 spaces)
        generate_info_file: Whether to generate archive_info.json (default: True)

    Example output:
        ZIP contents:
        ├── lib/
        │   └── android/
        │       └── shared/
        │           └── arm64-v8a/
        │               └── libfoo.so (0.89 MB) [aarch64, NDK r27, API 24]
        ├── include/
        │   └── foo/
        │       └── foo.h (0.01 MB)
        └── build_info.json (0.00 MB)
    """
    if not os.path.exists(zip_path):
        print(f"{indent}[ZIP file not found]")
        return

    # Generate archive_info.json if requested
    if generate_info_file:
        info_path = generate_archive_info(zip_path)
        if info_path:
            print(f"{indent}Generated: {os.path.basename(info_path)}")

    try:
        with zipfile.ZipFile(zip_path, 'r') as zf:
            # Build directory tree structure with file paths
            tree = {}

            for info in zf.infolist():
                parts = info.filename.split('/')
                current = tree
                for i, part in enumerate(parts):
                    if not part:  # Skip empty parts (trailing slashes)
                        continue
                    if part not in current:
                        # Check if this is a file (last part and not ending with /)
                        is_file = (i == len(parts) - 1) and not info.filename.endswith('/')
                        if is_file:
                            current[part] = {
                                '__size__': info.file_size,
                                '__path__': info.filename
                            }
                        else:
                            current[part] = {}
                    current = current[part]

            # Print tree structure
            print(f"{indent}ZIP contents:")
            _print_tree_level(tree, indent, "", zf)

    except zipfile.BadZipFile:
        print(f"{indent}[Invalid ZIP file]")
    except Exception as e:
        print(f"{indent}[Error reading ZIP: {e}]")


def _print_tree_level(tree, base_indent, prefix, zf=None):
    """
    Recursively print a level of the tree structure.

    Args:
        tree: Dictionary representing the tree structure
        base_indent: Base indentation string
        prefix: Current line prefix for tree drawing
        zf: ZipFile object for reading library file contents
    """
    items = sorted(tree.items())
    for i, (name, subtree) in enumerate(items):
        is_last = (i == len(items) - 1)
        connector = "└── " if is_last else "├── "

        if '__size__' in subtree:
            # This is a file
            size_mb = subtree['__size__'] / (1024 * 1024)
            file_path = subtree.get('__path__', '')

            # Format size string
            if size_mb >= 0.01:
                size_str = f"({size_mb:.2f} MB)"
            else:
                size_kb = subtree['__size__'] / 1024
                size_str = f"({size_kb:.1f} KB)"

            # Get library info if this is a library file
            lib_info = ""
            if zf and _is_library_file(name, file_path):
                try:
                    data = zf.read(file_path)
                    lib_info = get_library_info(data, name, file_path)
                except Exception:
                    pass

            print(f"{base_indent}{prefix}{connector}{name} {size_str}{lib_info}")
        else:
            # This is a directory
            print(f"{base_indent}{prefix}{connector}{name}/")
            # Recurse into subdirectory
            new_prefix = prefix + ("    " if is_last else "│   ")
            _print_tree_level(subtree, base_indent, new_prefix, zf)


def get_project_file_name(project_file_prefix):
    """
    Get platform-specific IDE project file name.

    Args:
        project_file_prefix: Base project name

    Returns:
        str: Platform-specific project file name
            - macOS: .xcodeproj
            - Windows: .sln (Visual Studio solution)
            - Linux: .workspace (CodeLite workspace)
    """
    if system_is_macos():
        project_file = f"{project_file_prefix}.xcodeproj"
    elif system_is_windows():
        project_file = f"{project_file_prefix}.sln"
    else:
        project_file = f"{project_file_prefix}.workspace"
    return project_file


def get_open_project_file_cmd(project_file):
    """
    Get platform-specific command to open IDE project file.

    Args:
        project_file: IDE project file path

    Returns:
        str: Shell command to open the project in the appropriate IDE
            - macOS: 'open' command for Xcode
            - Windows: 'start' with Release configuration for Visual Studio
            - Linux: 'codelite' command in background
    """
    if system_is_macos():
        # Open Xcode project
        return f"open {project_file}"
    elif system_is_windows():
        # Open Visual Studio project with Release configuration
        return f'start "" {project_file} /property:Configuration=Release'
    else:
        # Open CodeLite project in background
        return f"codelite {project_file} &"


def is_in_lib_list(target, lib_list):
    """
    Check if a library target name is in a list of library names.

    Handles various library naming conventions:
    - Exact match
    - Match without file extension
    - Match without 'lib' prefix (Unix convention)

    Args:
        target: Library file path or name to check
        lib_list: List of library names to match against

    Returns:
        bool: True if target matches any name in lib_list

    Example:
        is_in_lib_list('libfoo.a', ['foo'])  # Returns True
        is_in_lib_list('bar.so', ['bar'])    # Returns True
    """
    target = os.path.basename(target)
    for lib in lib_list:
        if target == lib:
            return True
        # Remove suffix
        target = os.path.splitext(target)[0]
        if target == lib:
            return True
        if target.startswith("lib"):
            if target[3:] == lib:
                return True
    return False


def check_library_architecture(library_path, platform_hint=None):
    """
    Check and print library architecture information.

    Args:
        library_path: Path to the library file (.a, .so, .lib, etc.)
        platform_hint: Optional platform hint ('linux', 'android', 'windows', 'macos', 'ios')

    Returns:
        dict: Architecture information including platform, arch, bits, etc.
    """
    if not os.path.exists(library_path):
        print(f"WARNING: Library not found: {library_path}")
        return None

    print("\n==================== Library Architecture Check ====================")
    print(f"Library: {os.path.basename(library_path)}")
    print(f"Path: {library_path}")

    # Check file size
    file_size = os.path.getsize(library_path)
    print(f"Size: {file_size / (1024 * 1024):.2f} MB")

    if file_size == 0:
        print("WARNING: File is empty (0 bytes)")
        print("=====================================================================\n")
        return {"path": library_path, "size": 0, "error": "empty file"}

    result = {"path": library_path, "size": file_size}

    # Check if file command is available
    file_cmd_available = shutil.which("file") is not None

    if file_cmd_available:
        # Run file command
        try:
            file_output = subprocess.check_output(["file", library_path], text=True).strip()
            print(f"File type: {file_output}")

            # Parse architecture from file output
            if "ELF" in file_output:
                # Linux/Android ELF binary
                if "64-bit" in file_output:
                    result["bits"] = 64
                elif "32-bit" in file_output:
                    result["bits"] = 32

                if "x86-64" in file_output or "x86_64" in file_output:
                    result["arch"] = "x86_64"
                    result["platform"] = platform_hint or "linux"
                elif "ARM aarch64" in file_output:
                    result["arch"] = "arm64-v8a" if platform_hint == "android" else "arm64"
                    result["platform"] = platform_hint or "linux"
                elif "ARM" in file_output and "32-bit" in file_output:
                    result["arch"] = "armeabi-v7a" if platform_hint == "android" else "armv7"
                    result["platform"] = platform_hint or "linux"
                elif "Intel 80386" in file_output:
                    result["arch"] = "x86"
                    result["platform"] = platform_hint or "linux"

                # Try readelf for more details
                if shutil.which("readelf"):
                    try:
                        readelf_output = subprocess.check_output(
                            ["readelf", "-h", library_path],
                            stderr=subprocess.DEVNULL,
                            text=True
                        )
                        for line in readelf_output.split('\n'):
                            if "Machine:" in line:
                                print(f"Machine: {line.split('Machine:')[1].strip()}")
                                break
                    except:
                        pass

            elif "Mach-O" in file_output:
                # macOS/iOS Mach-O binary
                result["platform"] = platform_hint or "macos"

                # Try lipo for universal binary info
                if shutil.which("lipo"):
                    try:
                        lipo_output = subprocess.check_output(
                            ["lipo", "-info", library_path],
                            stderr=subprocess.DEVNULL,
                            text=True
                        ).strip()
                        print(f"Architectures: {lipo_output}")

                        if "x86_64" in lipo_output and "arm64" in lipo_output:
                            result["arch"] = "universal (x86_64 + arm64)"
                        elif "x86_64" in lipo_output:
                            result["arch"] = "x86_64"
                        elif "arm64" in lipo_output:
                            result["arch"] = "arm64"
                        elif "armv7" in lipo_output:
                            result["arch"] = "armv7"
                    except:
                        if "universal" in file_output:
                            result["arch"] = "universal"

            elif "PE32" in file_output or "MS Windows" in file_output:
                # Windows PE binary
                result["platform"] = "windows"
                if "PE32+" in file_output:
                    result["arch"] = "x86_64"
                    result["bits"] = 64
                else:
                    result["arch"] = "x86"
                    result["bits"] = 32

            elif "ar archive" in file_output:
                # Static library archive - need to check object files
                result["type"] = "static archive"

                # For .a files, extract and check first object
                if library_path.endswith('.a'):
                    import tempfile
                    with tempfile.TemporaryDirectory() as tmpdir:
                        try:
                            # Extract first object file
                            subprocess.run(
                                ["ar", "x", library_path],
                                cwd=tmpdir,
                                capture_output=True,
                                check=False
                            )
                            # Check first .o file
                            obj_files = glob.glob(os.path.join(tmpdir, "*.o"))
                            if obj_files:
                                first_obj = obj_files[0]
                                obj_file_output = subprocess.check_output(
                                    ["file", first_obj], text=True
                                ).strip()
                                print(f"First object: {obj_file_output}")
                                # Parse object file architecture
                                if "ELF" in obj_file_output:
                                    if "x86-64" in obj_file_output:
                                        result["arch"] = "x86_64"
                                    elif "ARM aarch64" in obj_file_output:
                                        result["arch"] = "arm64"
                                    elif "ARM" in obj_file_output:
                                        result["arch"] = "armv7"
                        except Exception as e:
                            print(f"Could not extract object files: {e}")

        except Exception as e:
            print(f"ERROR: Could not run file command: {e}")
    else:
        # file command not available, use fallback methods
        print("INFO: 'file' command not available, using fallback detection methods")

        # Try to detect file type based on extension and magic bytes
        filename = os.path.basename(library_path).lower()

        # Read first few bytes for magic number detection
        try:
            with open(library_path, 'rb') as f:
                magic_bytes = f.read(64)  # Read more bytes for better detection
        except:
            magic_bytes = b''

        # Detect by extension and magic bytes
        if filename.endswith('.so') or (magic_bytes and magic_bytes[:4] == b'\x7fELF'):
            # ELF file (Linux/Android)
            print(f"File type: ELF shared object (detected by extension/magic)")
            result["platform"] = platform_hint or "linux"

            # Try readelf if available
            if shutil.which("readelf"):
                try:
                    readelf_output = subprocess.check_output(
                        ["readelf", "-h", library_path],
                        stderr=subprocess.DEVNULL,
                        text=True
                    )
                    for line in readelf_output.split('\n'):
                        if "Class:" in line:
                            if "ELF64" in line:
                                result["bits"] = 64
                            elif "ELF32" in line:
                                result["bits"] = 32
                        if "Machine:" in line:
                            machine = line.split('Machine:')[1].strip()
                            print(f"Machine: {machine}")
                            if "AArch64" in machine:
                                result["arch"] = "arm64-v8a" if platform_hint == "android" else "arm64"
                            elif "ARM" in machine:
                                result["arch"] = "armeabi-v7a" if platform_hint == "android" else "armv7"
                            elif "X86-64" in machine or "x86-64" in machine:
                                result["arch"] = "x86_64"
                            elif "Intel 80386" in machine:
                                result["arch"] = "x86"
                            break
                except:
                    pass
            else:
                # Try to parse ELF header directly if readelf isn't available
                if len(magic_bytes) >= 20:
                    # ELF class is at offset 4 (1=32-bit, 2=64-bit)
                    if magic_bytes[4] == 2:
                        result["bits"] = 64
                    elif magic_bytes[4] == 1:
                        result["bits"] = 32

                    # Machine type is at offset 18-19 (little endian)
                    if magic_bytes[5] == 1:  # Little endian
                        machine_type = int.from_bytes(magic_bytes[18:20], 'little')
                        if machine_type == 0xB7:  # AArch64
                            result["arch"] = "arm64-v8a" if platform_hint == "android" else "arm64"
                            print(f"Architecture: ARM64 (from ELF header)")
                        elif machine_type == 0x28:  # ARM
                            result["arch"] = "armeabi-v7a" if platform_hint == "android" else "armv7"
                            print(f"Architecture: ARM (from ELF header)")
                        elif machine_type == 0x3E:  # x86-64
                            result["arch"] = "x86_64"
                            print(f"Architecture: x86_64 (from ELF header)")
                        elif machine_type == 0x03:  # i386
                            result["arch"] = "x86"
                            print(f"Architecture: x86 (from ELF header)")

        elif filename.endswith('.a') or (magic_bytes and magic_bytes[:8] == b'!<arch>\n'):
            # Static library archive (ar archive format)
            result["type"] = "static archive"
            result["platform"] = platform_hint or "unknown"

            # Check for ar archive magic bytes
            if magic_bytes[:8] == b'!<arch>\n':
                print(f"File type: AR archive (detected by magic bytes)")
            else:
                print(f"File type: Static library (detected by extension)")

            # Try ar command to list contents
            if shutil.which("ar"):
                try:
                    ar_output = subprocess.check_output(
                        ["ar", "t", library_path],
                        stderr=subprocess.DEVNULL,
                        text=True
                    )
                    obj_files = [f for f in ar_output.split('\n') if f.endswith('.o')]
                    if obj_files:
                        print(f"Archive contains {len(obj_files)} object file(s)")
                        result["object_count"] = len(obj_files)

                        # Try to get more info with ar tv for size information
                        try:
                            ar_tv_output = subprocess.check_output(
                                ["ar", "tv", library_path],
                                stderr=subprocess.DEVNULL,
                                text=True
                            )
                            # Parse total size from ar tv output
                            total_size = sum(int(line.split()[2]) for line in ar_tv_output.split('\n')
                                           if line and len(line.split()) >= 3)
                            if total_size > 0:
                                print(f"Total object size: {total_size / 1024:.2f} KB")
                        except:
                            pass
                except:
                    pass
            else:
                # ar command not available, try basic detection from magic bytes
                if magic_bytes[:8] == b'!<arch>\n':
                    # Parse archive header to get basic info
                    try:
                        # AR format has 60-byte headers after magic
                        header_start = 8
                        if len(magic_bytes) > header_start + 60:
                            # First member name is at offset 0-15 in header
                            first_member = magic_bytes[header_start:header_start+16].decode('ascii', errors='ignore').strip()
                            if first_member:
                                print(f"First archive member: {first_member}")
                    except:
                        pass

            # Platform-specific hints for .a files
            if platform_hint == "windows" or "mingw" in library_path.lower():
                result["platform"] = "windows"
                result["arch"] = "x86_64"  # Assume x64 for modern builds
                print("Platform hint: Windows/MinGW static library")
            elif platform_hint == "android":
                result["platform"] = "android"
                # Try to infer architecture from path
                if "arm64-v8a" in library_path:
                    result["arch"] = "arm64-v8a"
                elif "armeabi-v7a" in library_path:
                    result["arch"] = "armeabi-v7a"
                elif "x86_64" in library_path:
                    result["arch"] = "x86_64"
                elif "x86" in library_path:
                    result["arch"] = "x86"
                print(f"Platform hint: Android static library")
            elif platform_hint:
                print(f"Platform hint: {platform_hint}")

        elif filename.endswith('.lib') or filename.endswith('.dll'):
            # Windows library
            result["platform"] = "windows"

            if filename.endswith('.dll'):
                result["type"] = "shared library"
                print(f"File type: Windows DLL (detected by extension)")
            else:
                result["type"] = "static library"
                print(f"File type: Windows static library (detected by extension)")

            # Check for PE/COFF magic bytes
            if magic_bytes and len(magic_bytes) >= 2:
                if magic_bytes[:2] == b'MZ':  # DOS/PE header
                    print(f"Format: PE/COFF executable (detected by magic)")
                    # Try to determine architecture from PE header
                    try:
                        # PE header offset is at 0x3C
                        if len(magic_bytes) >= 64:
                            pe_offset = int.from_bytes(magic_bytes[0x3C:0x3C+4], 'little')
                            if len(magic_bytes) > pe_offset + 6:
                                # Machine type is at PE+4
                                machine = int.from_bytes(magic_bytes[pe_offset+4:pe_offset+6], 'little')
                                if machine == 0x8664:  # AMD64
                                    result["arch"] = "x86_64"
                                    result["bits"] = 64
                                    print(f"Architecture: x86_64 (from PE header)")
                                elif machine == 0x14c:  # i386
                                    result["arch"] = "x86"
                                    result["bits"] = 32
                                    print(f"Architecture: x86 (from PE header)")
                    except:
                        pass

            # Try dumpbin if available (unlikely in Docker)
            if shutil.which("dumpbin"):
                try:
                    dumpbin_output = subprocess.check_output(
                        ["dumpbin", "/HEADERS", library_path],
                        stderr=subprocess.DEVNULL,
                        text=True
                    )
                    if "x64" in dumpbin_output or "AMD64" in dumpbin_output:
                        result["arch"] = "x86_64"
                        result["bits"] = 64
                    elif "x86" in dumpbin_output:
                        result["arch"] = "x86"
                        result["bits"] = 32
                except:
                    pass
            elif "arch" not in result:
                # Default to x64 for modern Windows builds if we couldn't detect
                result["arch"] = "x86_64"
                print(f"Architecture: x86_64 (assumed for modern Windows)")

        elif filename.endswith(('.dylib', '.framework')) or \
             (magic_bytes and len(magic_bytes) >= 4 and magic_bytes[:4] in [
                 b'\xfe\xed\xfa\xce',  # Mach-O 32-bit big endian
                 b'\xce\xfa\xed\xfe',  # Mach-O 32-bit little endian
                 b'\xfe\xed\xfa\xcf',  # Mach-O 64-bit big endian
                 b'\xcf\xfa\xed\xfe',  # Mach-O 64-bit little endian
                 b'\xca\xfe\xba\xbe',  # Mach-O fat binary
                 b'\xbe\xba\xfe\xca'   # Mach-O fat binary (reversed)
             ]):
            # macOS/iOS library
            result["platform"] = platform_hint or "macos"

            # Detect Mach-O format from magic bytes
            if magic_bytes and len(magic_bytes) >= 4:
                magic = magic_bytes[:4]
                if magic in [b'\xca\xfe\xba\xbe', b'\xbe\xba\xfe\xca']:
                    print(f"File type: Mach-O universal binary (detected by magic)")
                    result["arch"] = "universal"
                elif magic in [b'\xfe\xed\xfa\xcf', b'\xcf\xfa\xed\xfe']:
                    print(f"File type: Mach-O 64-bit (detected by magic)")
                    # Try to determine arch from Mach-O header
                    if len(magic_bytes) >= 12:
                        cpu_type = int.from_bytes(magic_bytes[4:8], 'little' if magic == b'\xcf\xfa\xed\xfe' else 'big')
                        if cpu_type == 0x0100000c:  # ARM64
                            result["arch"] = "arm64"
                        elif cpu_type == 0x01000007:  # x86_64
                            result["arch"] = "x86_64"
                elif magic in [b'\xfe\xed\xfa\xce', b'\xce\xfa\xed\xfe']:
                    print(f"File type: Mach-O 32-bit (detected by magic)")
                    result["arch"] = "i386"
                else:
                    print(f"File type: macOS/iOS library (detected by extension)")
            else:
                print(f"File type: macOS/iOS library (detected by extension)")

            # Try lipo if available for more detailed info
            if shutil.which("lipo"):
                try:
                    lipo_output = subprocess.check_output(
                        ["lipo", "-info", library_path],
                        stderr=subprocess.DEVNULL,
                        text=True
                    ).strip()
                    print(f"Architectures (lipo): {lipo_output}")

                    if "x86_64" in lipo_output and "arm64" in lipo_output:
                        result["arch"] = "universal (x86_64 + arm64)"
                    elif "x86_64" in lipo_output:
                        result["arch"] = "x86_64"
                    elif "arm64" in lipo_output:
                        result["arch"] = "arm64"
                except:
                    pass
        else:
            # Unknown file type
            print(f"File type: Unknown (no 'file' command, unable to detect type)")
            result["platform"] = platform_hint or "unknown"

    # Print result summary
    print("\n--- Architecture Summary ---")
    if "platform" in result:
        print(f"Platform: {result['platform']}")
    if "arch" in result:
        print(f"Architecture: {result['arch']}")
    if "bits" in result:
        print(f"Bits: {result['bits']}-bit")
    if "type" in result:
        print(f"Type: {result['type']}")

    print("=====================================================================\n")

    return result


def check_build_libraries(lib_paths, platform_hint=None):
    """
    Check multiple libraries and print summary.

    Args:
        lib_paths: List of library paths or glob patterns
        platform_hint: Optional platform hint for architecture detection

    Returns:
        bool: True if all libraries exist and were checked successfully
    """
    if isinstance(lib_paths, str):
        lib_paths = [lib_paths]

    checked_libs = []
    missing_libs = []

    for path_pattern in lib_paths:
        # Handle glob patterns
        if '*' in path_pattern:
            matched_files = glob.glob(path_pattern)
            if not matched_files:
                missing_libs.append(path_pattern)
                continue
            for lib_file in matched_files:
                if os.path.exists(lib_file):
                    result = check_library_architecture(lib_file, platform_hint)
                    if result:
                        checked_libs.append(result)
                else:
                    missing_libs.append(lib_file)
        else:
            if os.path.exists(path_pattern):
                result = check_library_architecture(path_pattern, platform_hint)
                if result:
                    checked_libs.append(result)
            else:
                missing_libs.append(path_pattern)

    # Print summary
    if checked_libs or missing_libs:
        print("\n==================== Build Libraries Summary ====================")
        if checked_libs:
            print(f"✓ Successfully checked {len(checked_libs)} libraries")
            for lib in checked_libs:
                arch_info = lib.get('arch', 'unknown')
                platform_info = lib.get('platform', 'unknown')
                print(f"  - {os.path.basename(lib['path'])}: {platform_info}/{arch_info}")

        if missing_libs:
            print(f"✗ Missing {len(missing_libs)} libraries:")
            for lib in missing_libs:
                print(f"  - {lib}")
            print("\nERROR: Some expected libraries were not found!")
            return False

        print("==================================================================\n")

    return len(missing_libs) == 0


# =============================================================================
# Unified Archive Structure Constants and Functions
# =============================================================================

# Standard directory names for unified archive structure
ARCHIVE_DIR_LIB = "lib"
ARCHIVE_DIR_STATIC = "static"
ARCHIVE_DIR_SHARED = "shared"
ARCHIVE_DIR_INCLUDE = "include"
# Note: Apple frameworks (xcframework/framework) are now stored under lib/ directory
# to maintain consistency with other platforms (was "frameworks")
ARCHIVE_DIR_FRAMEWORKS = "lib"
ARCHIVE_DIR_HAARS = "haars"
ARCHIVE_DIR_SYMBOLS = "symbols"
ARCHIVE_DIR_OBJ = "obj"

# Platform-specific subdirectories
ARCHIVE_DIR_MSVC = "msvc"
ARCHIVE_DIR_MINGW = "mingw"

# Build info file name
BUILD_INFO_FILE = "build_info.json"


def get_archive_version_info(script_path):
    """
    Get version information for archive naming.

    Args:
        script_path: Path to the project script directory

    Returns:
        tuple: (version_name, suffix, full_version)
            - version_name: Base version (e.g., "1.0.0")
            - suffix: Version suffix (e.g., "release", "beta.0")
            - full_version: Combined version (e.g., "1.0.0-release")
    """
    version_name = get_version_name(script_path)

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

    full_version = f"{version_name}-{suffix}" if suffix else version_name
    return version_name, suffix, full_version


def generate_build_info(
    project_name,
    target_platform,
    version,
    link_type="both",
    architectures=None,
    toolchain=None,
    extra_info=None
):
    """
    Generate build_info.json content for archive packages.

    Args:
        project_name: Project name (lowercase)
        target_platform: Target platform (linux, windows, macos, ios, android, ohos, etc.)
        version: Full version string (e.g., "1.0.0-release")
        link_type: Library link type (static, shared, both)
        architectures: List of target architectures (e.g., ["arm64", "x86_64"])
        toolchain: Toolchain used (e.g., "msvc", "mingw", "auto")
        extra_info: Additional platform-specific information dict

    Returns:
        dict: Build information suitable for JSON serialization
    """
    build_info = {
        "project": project_name,
        "platform": target_platform,
        "version": version,
        "link_type": link_type,
        "build_time": datetime.now().isoformat(),
        "build_host": platform.system(),
    }

    if architectures:
        build_info["architectures"] = architectures

    if toolchain:
        build_info["toolchain"] = toolchain

    if extra_info:
        build_info.update(extra_info)

    return build_info


def create_unified_archive(
    output_dir,
    project_name,
    platform_name,
    version,
    link_type="both",
    static_libs=None,
    shared_libs=None,
    include_dirs=None,
    frameworks=None,
    haars=None,
    symbols_static=None,
    symbols_shared=None,
    obj_files=None,
    architectures=None,
    toolchain=None,
    extra_info=None
):
    """
    Create unified archive packages with standard directory structure.

    This function creates two packages:
    1. Main package: {PROJECT}_{PLATFORM}_SDK-{version}.zip
       - lib/static/  : Static libraries
       - lib/shared/  : Shared libraries
       - include/     : Header files
       - frameworks/  : Apple frameworks (iOS/macOS/watchOS/tvOS)
       - haars/       : Android AAR / OHOS HAR packages
       - build_info.json

    2. Symbols package: {PROJECT}_{PLATFORM}_SDK-{version}-SYMBOLS.zip
       - symbols/static/ : Debug symbols for static libraries
       - symbols/shared/ : Debug symbols for shared libraries
       - obj/            : Unstripped libraries (Android/OHOS/Linux)

    Args:
        output_dir: Output directory for ZIP files
        project_name: Project name (used in ZIP file naming)
        platform_name: Platform name (LINUX, WINDOWS, MACOS, IOS, ANDROID, OHOS, etc.)
        version: Full version string (e.g., "1.0.0-release")
        link_type: Library link type ("static", "shared", "both")
        static_libs: Dict mapping archive paths to source file paths for static libs
                     e.g., {"lib/static/libfoo.a": "/path/to/libfoo.a"}
                     or {"lib/static/arm64/libfoo.a": "/path/to/libfoo.a"} for multi-arch
        shared_libs: Dict mapping archive paths to source file paths for shared libs
        include_dirs: Dict mapping archive paths to source directories for headers
                      e.g., {"include/foo": "/path/to/headers"}
        frameworks: Dict mapping archive paths to source paths for frameworks
                    e.g., {"frameworks/static/Foo.xcframework": "/path/to/xcframework"}
        haars: Dict mapping archive paths to source file paths for AAR/HAR
               e.g., {"haars/foo.aar": "/path/to/foo.aar"}
        symbols_static: Dict mapping archive paths to source paths for static lib symbols
        symbols_shared: Dict mapping archive paths to source paths for shared lib symbols
        obj_files: Dict mapping archive paths to source paths for unstripped objects
        architectures: List of target architectures for build_info
        toolchain: Toolchain name for build_info (e.g., "msvc", "mingw")
        extra_info: Additional info for build_info.json

    Returns:
        tuple: (main_zip_path, symbols_zip_path) or (main_zip_path, None) if no symbols
    """
    import zipfile

    os.makedirs(output_dir, exist_ok=True)

    project_upper = project_name.upper()
    main_zip_name = f"{project_upper}_{platform_name}_SDK-{version}.zip"
    main_zip_path = os.path.join(output_dir, main_zip_name)

    print(f"Creating unified archive: {main_zip_name}")

    # Helper function to add files/directories to zip
    def add_to_zip(zipf, items, item_type="file"):
        """Add files or directories to zip archive."""
        if not items:
            return

        for arc_path, src_path in items.items():
            if not os.path.exists(src_path):
                print(f"  Warning: {item_type} not found: {src_path}")
                continue

            if os.path.isdir(src_path):
                # Add directory recursively
                for root, dirs, files in os.walk(src_path):
                    for file in files:
                        if not should_include_file_in_archive(file):
                            continue
                        file_path = os.path.join(root, file)
                        rel_path = os.path.relpath(file_path, src_path)
                        arcname = os.path.join(arc_path, rel_path)
                        zipf.write(file_path, arcname)
                        print(f"  + {arcname}")
            else:
                # Add single file
                zipf.write(src_path, arc_path)
                print(f"  + {arc_path}")

    # Create main package
    with zipfile.ZipFile(main_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
        # Add static libraries (if link_type is static or both)
        if link_type in ("static", "both") and static_libs:
            print("Adding static libraries...")
            add_to_zip(zipf, static_libs, "static library")

        # Add shared libraries (if link_type is shared or both)
        if link_type in ("shared", "both") and shared_libs:
            print("Adding shared libraries...")
            add_to_zip(zipf, shared_libs, "shared library")

        # Add include directories
        if include_dirs:
            print("Adding header files...")
            add_to_zip(zipf, include_dirs, "include directory")

        # Add frameworks (Apple platforms)
        if frameworks:
            print("Adding frameworks...")
            # Filter frameworks based on link_type
            filtered_frameworks = {}
            for arc_path, src_path in frameworks.items():
                if link_type == "both":
                    filtered_frameworks[arc_path] = src_path
                elif link_type == "static" and "/static/" in arc_path:
                    filtered_frameworks[arc_path] = src_path
                elif link_type == "shared" and "/shared/" in arc_path:
                    filtered_frameworks[arc_path] = src_path
            add_to_zip(zipf, filtered_frameworks, "framework")

        # Add haars (Android/OHOS)
        if haars:
            print("Adding AAR/HAR packages...")
            add_to_zip(zipf, haars, "AAR/HAR")

        # Generate and add build_info.json
        build_info = generate_build_info(
            project_name=project_name.lower(),
            target_platform=platform_name.lower(),
            version=version,
            link_type=link_type,
            architectures=architectures,
            toolchain=toolchain,
            extra_info=extra_info
        )
        build_info_json = json.dumps(build_info, indent=2)
        zipf.writestr(BUILD_INFO_FILE, build_info_json)
        print(f"  + {BUILD_INFO_FILE}")

    print(f"Created main package: {main_zip_path}")

    # Create symbols package if any symbols are provided
    has_symbols = any([symbols_static, symbols_shared, obj_files])
    symbols_zip_path = None

    if has_symbols:
        symbols_zip_name = f"{project_upper}_{platform_name}_SDK-{version}-SYMBOLS.zip"
        symbols_zip_path = os.path.join(output_dir, symbols_zip_name)

        print(f"Creating symbols package: {symbols_zip_name}")

        with zipfile.ZipFile(symbols_zip_path, "w", zipfile.ZIP_DEFLATED) as zipf:
            # Add static library symbols
            if symbols_static and link_type in ("static", "both"):
                print("Adding static library symbols...")
                add_to_zip(zipf, symbols_static, "static symbol")

            # Add shared library symbols
            if symbols_shared and link_type in ("shared", "both"):
                print("Adding shared library symbols...")
                add_to_zip(zipf, symbols_shared, "shared symbol")

            # Add unstripped object files
            if obj_files:
                print("Adding unstripped objects...")
                add_to_zip(zipf, obj_files, "object file")

        print(f"Created symbols package: {symbols_zip_path}")

    return main_zip_path, symbols_zip_path


def get_unified_lib_path(link_type, arch=None, toolchain=None, lib_name=None, extension=None, platform=None):
    """
    Get the standard archive path for a library file.

    Args:
        link_type: "static" or "shared"
        arch: Architecture name (e.g., "arm64", "x86_64") - optional for single-arch
        toolchain: Toolchain name (e.g., "msvc", "mingw") - Windows only
        lib_name: Library file name (e.g., "libfoo.a", "foo.lib")
        extension: File extension to use (overrides lib_name extension)
        platform: Platform name (e.g., "windows", "linux", "android") - adds platform directory

    Returns:
        str: Archive path like "lib/windows/static/mingw/libfoo.a" or "lib/static/arm64/libfoo.a"
    """
    parts = [ARCHIVE_DIR_LIB]

    # Add platform directory if specified
    if platform:
        parts.append(platform)

    parts.append(link_type)

    if toolchain:
        parts.append(toolchain)

    if arch:
        parts.append(arch)

    if lib_name:
        if extension:
            base_name = os.path.splitext(lib_name)[0]
            lib_name = f"{base_name}{extension}"
        parts.append(lib_name)

    return "/".join(parts)


def get_unified_framework_path(link_type, framework_name, platform=None):
    """
    Get the standard archive path for an Apple framework.

    Args:
        link_type: "static" or "shared"
        framework_name: Framework name (e.g., "Foo.xcframework")
        platform: Platform name (e.g., "ios", "macos") - adds platform directory

    Returns:
        str: Archive path like "frameworks/ios/static/Foo.xcframework"
    """
    parts = [ARCHIVE_DIR_FRAMEWORKS]
    if platform:
        parts.append(platform)
    parts.append(link_type)
    parts.append(framework_name)
    return "/".join(parts)


def get_unified_haar_path(haar_name):
    """
    Get the standard archive path for an Android AAR or OHOS HAR package.

    Args:
        haar_name: Package file name (e.g., "foo.aar", "foo.har")

    Returns:
        str: Archive path like "haars/foo.aar"
    """
    return f"{ARCHIVE_DIR_HAARS}/{haar_name}"


def get_unified_include_path(project_name, src_include_dir=None):
    """
    Get the standard archive path for include directory.

    Intelligently handles include path to avoid duplication:
    - If src_include_dir contains a project_name subdirectory, returns "include"
      (source structure: include/ccgonow/ -> archive: include/ccgonow/)
    - If src_include_dir does NOT contain a project_name subdirectory, returns "include/project_name"
      (source structure: include/*.h -> archive: include/ccgonow/*.h)

    Args:
        project_name: Project name (lowercase)
        src_include_dir: Source include directory path to check for existing project subdirectory

    Returns:
        str: Archive path - either "include" or "include/{project_name}"
    """
    if src_include_dir and os.path.isdir(src_include_dir):
        # Check if source directory already has a project-named subdirectory
        project_subdir = os.path.join(src_include_dir, project_name)
        if os.path.isdir(project_subdir):
            # Source already has project subdirectory, don't add it again
            return ARCHIVE_DIR_INCLUDE

    # Source doesn't have project subdirectory, add it to archive path
    return f"{ARCHIVE_DIR_INCLUDE}/{project_name}"


def get_unified_symbol_path(link_type, symbol_name, arch=None, platform=None):
    """
    Get the standard archive path for debug symbols.

    Args:
        link_type: "static" or "shared"
        symbol_name: Symbol file/directory name (e.g., "foo.dSYM", "foo.pdb")
        arch: Architecture name - optional for single-arch
        platform: Platform name (e.g., "ios", "macos", "windows") - adds platform directory

    Returns:
        str: Archive path like "symbols/ios/static/foo.dSYM"
    """
    parts = [ARCHIVE_DIR_SYMBOLS]

    if platform:
        parts.append(platform)

    parts.append(link_type)

    if arch:
        parts.append(arch)

    parts.append(symbol_name)
    return "/".join(parts)


def get_unified_obj_path(arch, lib_name):
    """
    Get the standard archive path for unstripped object files.

    Args:
        arch: Architecture name (e.g., "arm64-v8a")
        lib_name: Library file name (e.g., "libfoo.so")

    Returns:
        str: Archive path like "obj/arm64-v8a/libfoo.so"
    """
    return f"{ARCHIVE_DIR_OBJ}/{arch}/{lib_name}"


def main():
    """
    Main entry point when build_utils.py is run as a script.

    Generates the project version info header file (verinfo.h).
    """
    cur_path = os.path.dirname(os.path.realpath(__file__))
    gen_project_revision_file(
        PROJECT_NAME, OUTPUT_VERINFO_PATH, get_version_name(cur_path)
    )


if __name__ == "__main__":
    main()
