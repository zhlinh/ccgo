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
    tomllib = None

# Load configuration from CCGO.toml in project directory (current working directory)
PROJECT_DIR = os.getcwd()


def load_ccgo_config():
    """
    Load configuration from CCGO.toml file.

    Returns a dictionary with build configuration values that were previously
    defined in build_config.py. Falls back to default values if CCGO.toml
    is not found or cannot be parsed.
    """
    config_file = os.path.join(PROJECT_DIR, "CCGO.toml")

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
            "MACOS_BUILD_COPY_HEADER_FILES": {},
            "WINDOWS_BUILD_COPY_HEADER_FILES": {},
            "LINUX_BUILD_COPY_HEADER_FILES": {},
            "INCLUDE_BUILD_COPY_HEADER_FILES": {},
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
            "MACOS_BUILD_COPY_HEADER_FILES": {},
            "WINDOWS_BUILD_COPY_HEADER_FILES": {},
            "LINUX_BUILD_COPY_HEADER_FILES": {},
            "INCLUDE_BUILD_COPY_HEADER_FILES": {},
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

        ios_headers = convert_headers(toml_data.get("ios", {}))
        macos_headers = convert_headers(toml_data.get("macos", {}))
        windows_headers = convert_headers(toml_data.get("windows", {}))
        linux_headers = convert_headers(toml_data.get("linux", {}))
        include_headers = convert_headers(toml_data.get("include", {}))

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
            "MACOS_BUILD_COPY_HEADER_FILES": macos_headers,
            "WINDOWS_BUILD_COPY_HEADER_FILES": windows_headers,
            "LINUX_BUILD_COPY_HEADER_FILES": linux_headers,
            "INCLUDE_BUILD_COPY_HEADER_FILES": include_headers,
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
            "MACOS_BUILD_COPY_HEADER_FILES": {},
            "WINDOWS_BUILD_COPY_HEADER_FILES": {},
            "LINUX_BUILD_COPY_HEADER_FILES": {},
            "INCLUDE_BUILD_COPY_HEADER_FILES": {},
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
MACOS_BUILD_COPY_HEADER_FILES = _CONFIG["MACOS_BUILD_COPY_HEADER_FILES"]
WINDOWS_BUILD_COPY_HEADER_FILES = _CONFIG["WINDOWS_BUILD_COPY_HEADER_FILES"]
LINUX_BUILD_COPY_HEADER_FILES = _CONFIG["LINUX_BUILD_COPY_HEADER_FILES"]
INCLUDE_BUILD_COPY_HEADER_FILES = _CONFIG["INCLUDE_BUILD_COPY_HEADER_FILES"]

# Store the build script path for cmake directory access
BUILD_SCRIPT_PATH = os.path.dirname(os.path.realpath(__file__))
# CCGO cmake directory path (in the ccgo package)
CCGO_CMAKE_DIR = os.path.join(BUILD_SCRIPT_PATH, "cmake")


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

    dSYM files contain debugging information extracted from binaries,
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
    src_lib, dst_framework, header_file_mappings, header_files_src_base="./"
):
    """
    Create an iOS/macOS static framework bundle from a library and headers.

    A framework is a bundle directory containing:
    - The compiled library binary
    - Headers/ directory with public headers

    Args:
        src_lib: Source library file path (.a static library)
        dst_framework: Destination framework bundle path (.framework)
        header_file_mappings: Dict mapping header source paths to subdirectory destinations
        header_files_src_base: Base path for header source files

    Returns:
        bool: True if framework creation succeeded

    Note:
        Removes existing framework at dst_framework if it exists.
    """
    if os.path.exists(dst_framework):
        shutil.rmtree(dst_framework)

    os.makedirs(dst_framework)
    shutil.copy(src_lib, dst_framework)

    framework_path = dst_framework + "/Headers"
    for src, dst in header_file_mappings.items():
        if not os.path.exists(src):
            continue
        copy_file(
            header_files_src_base + src,
            framework_path + "/" + dst + "/" + src[src.rfind("/") :],
        )

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
    if check_ndk_revision(ndk_revision[:4]):
        return True

    print(
        f"Error: make sure ndk's version == {get_ndk_desc()}, current is {ndk_revision[:4]}"
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
            - revision: Short commit hash
            - branch_or_path: Current branch name
            - url: Remote origin URL (with OAuth2 credentials removed)

    Note:
        Changes current working directory temporarily during execution.
    """
    curdir = os.getcwd()
    os.chdir(path)
    revision = os.popen("git rev-parse --short HEAD").read().strip()
    path = os.popen("git rev-parse --abbrev-ref HEAD").read().strip()
    url = os.popen("git remote get-url origin").read().strip()
    # Remove OAuth2 credentials from URL for security
    pos = url.find("oauth2")
    if pos >= 0:
        pos_to_trim = url.find("@")
        if pos_to_trim >= 0:
            url = "git" + url[pos_to_trim:]

    os.chdir(curdir)

    return revision, path, url


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
    tag,
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
    - bin/{platform}/build_info.json (e.g., bin/android/build_info.json)
    - Falls back to project root if platform is not specified

    Args:
        project_name: Project name
        project_dir_path: Path to project root directory
        version_name: Project version string
        tag: Release tag
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
            "tag": tag,
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
    # The file will be copied to bin/{platform}/ by the build script after packaging
    if target_platform:
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
    tag="",
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
    - Release tag
    - Android NDK/STL configuration

    The function also outputs a simple build description for CI/CD systems
    and optionally generates a JSON file with comprehensive build metadata.

    Args:
        project_name: Project name (used in macro names, uppercased)
        origin_version_file_path: Relative path where verinfo.h will be created
        version_name: Version string (e.g., "1.2.3")
        tag: Release tag string (e.g., "v1.2.3" or "beta.23")
        incremental: If True, only includes date (not time) in build timestamp
        json_output: If True, generates build_info.json. If None, reads from CCGO.toml (default: None)
        platform: Target platform (e.g., "android", "ios", "macos", "windows", "linux", "ohos")

    Note:
        - Only writes file if content has changed (avoids unnecessary rebuilds)
        - Retrieves git info from the version file directory
        - Outputs build description to stdout for CI/CD parsing
        - JSON output is saved to bin/{platform}/build_info.json for platform-specific builds
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
        tag=tag,
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
            tag=tag,
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


def copy_build_info_to_bin(platform_name, project_dir=None):
    """
    Copy build_info.json from cmake_build directory to bin/{platform}/ directory.

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

    # Create bin/{platform} directory
    bin_platform_dir = os.path.join(project_dir, "bin", platform_name.lower())
    os.makedirs(bin_platform_dir, exist_ok=True)

    # Copy build_info.json
    build_info_dst = os.path.join(bin_platform_dir, "build_info.json")
    shutil.copy2(build_info_src, build_info_dst)

    print(f"[SUCCESS] Copied build_info.json to bin/{platform_name.lower()}/")
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
