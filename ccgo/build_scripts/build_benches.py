#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_benches.py
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
from datetime import datetime

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils.py (reads from CCGO.toml)
PROJECT_RELATIVE_PATH = PROJECT_NAME_LOWER


BUILD_OUT_PATH = os.path.join("cmake_build", "Benches")
CMAKE_SYSTEM_NAME = platform.system()
INSTALL_PATH = os.path.join(BUILD_OUT_PATH, CMAKE_SYSTEM_NAME + ".out")

CURRENT_TIME = datetime.now()
FORMATTED_TIME = CURRENT_TIME.strftime("%Y%m%d_%H%M%S_%f")
if system_is_macos():
    # change Darwin to macos here
    FORMATTED_SYSTEM_NAME = "macos"
else:
    FORMATTED_SYSTEM_NAME = CMAKE_SYSTEM_NAME.lower().replace("/", "_")
PARAM_FOR_OUTPUT_XML = f'--benchmark_format=console --benchmark_out={os.path.join(BUILD_OUT_PATH, f"benches_on_{FORMATTED_SYSTEM_NAME}_result_{FORMATTED_TIME}.json")}'

BUILD_TYPE = "Release"

BENCHES_EXTRA_FLAGS = f"-DBENCHMARK_SUPPORT=ON -DCCGO_CMAKE_DIR={CCGO_CMAKE_DIR}"

if system_is_windows():
    # -DCMAKE_BUILD_TYPE=xxx not working for vs
    GOOGLEBENCHMARK_BUILD_CMD = f'cmake ../.. -G "Visual Studio 16 2019" -T v142 {BENCHES_EXTRA_FLAGS} && cmake --build . --target install --config {BUILD_TYPE}'
else:
    GOOGLEBENCHMARK_BUILD_CMD = f"cmake ../.. -DCMAKE_BUILD_TYPE={BUILD_TYPE} {BENCHES_EXTRA_FLAGS} && make -j8 && make install"


if system_is_windows():
    GEN_PROJECT_CMD = (
        f'cmake ../.. -G "Visual Studio 16 2019" -T v142 {BENCHES_EXTRA_FLAGS}'
    )
elif system_is_macos():
    GEN_PROJECT_CMD = f"cmake ../.. -G Xcode -DCMAKE_OSX_DEPLOYMENT_TARGET:STRING=10.9 -DENABLE_BITCODE=0 {BENCHES_EXTRA_FLAGS}"
else:
    GEN_PROJECT_CMD = (
        f'cmake ../.. -G "CodeLite - Unix Makefiles" {BENCHES_EXTRA_FLAGS}'
    )


def build_googlebenchmark(incremental):
    before_time = time.time()
    print(
        f"==================build_googlebenchmark install path: {INSTALL_PATH} ========================"
    )

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        incremental=incremental,
        platform="benches",
    )

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    ret = os.system(GOOGLEBENCHMARK_BUILD_CMD)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build googlebenchmark fail!!!!!!!!!!!!!!!")
        return False

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    dst_target_path = os.path.relpath(
        os.path.abspath(
            os.path.join(INSTALL_PATH, f"{PROJECT_NAME_LOWER}_googlebenchmark")
        )
    )

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_target_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def run_googlebenchmark(filter_rules=""):
    os.chdir(SCRIPT_PATH)
    for fpath, dirs, fs in os.walk(INSTALL_PATH):
        for file_name in fs:
            # for convert / to \ in windows
            file = os.path.relpath(os.path.abspath(os.path.join(fpath, file_name)))
            if file.find("_bench") >= 0:
                if len(filter_rules) > 0:
                    cmd = f"{file} {filter_rules}"
                else:
                    cmd = f"{file}"
                cmd = f"{cmd} {PARAM_FOR_OUTPUT_XML}"
                print(f"start exec {cmd}")
                ret = os.system(cmd)
                if ret != 0:
                    print(f"!!!!!!!!!!!run googlebenchmark {file} fail!!!!!!!!!!!!!!!")
                    return False
                else:
                    print(f"[INFO] run googlebenchmark {file} success\n")
    return True


def gen_googlebenchmark_project():
    print("==================gen_googlebenchmark_project========================")
    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        platform="benches",
    )

    clean(BUILD_OUT_PATH)
    os.chdir(BUILD_OUT_PATH)

    cmd = GEN_PROJECT_CMD
    ret = os.system(cmd)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!gen fail!!!!!!!!!!!!!!!")
        return False

    project_file_prefix = os.path.join(SCRIPT_PATH, BUILD_OUT_PATH, PROJECT_NAME_LOWER)
    project_file = get_project_file_name(project_file_prefix)

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(f"project file: {project_file}")

    os.system(get_open_project_file_cmd(project_file))

    return True


def main(filter_rules=""):
    """
    Main entry point for building and running benchmarks.

    Args:
        filter_rules: Optional benchmark filter rules (e.g., --benchmark_filter=BenchSuite.*)
    """
    if system_is_windows() and (not check_vs_env()):
        sys.exit(1)

    if not build_googlebenchmark(incremental=False):
        raise RuntimeError("Exception occurs when build googlebenchmark")

    if filter_rules:
        if not run_googlebenchmark(filter_rules=filter_rules):
            raise RuntimeError("Exception occurs when run googlebenchmark")


# Command-line interface for benchmark builds
#
# Usage:
#   python build_benches.py                    # Build benchmarks (default)
#   python build_benches.py --ide-project      # Generate IDE project
#   python build_benches.py --run              # Build and run benchmarks
#   python build_benches.py --run BenchSuite   # Build and run specific benchmarks
#   python build_benches.py --run-only         # Run benchmarks without building
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build and run googlebenchmark",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--ide-project",
        action="store_true",
        help="Generate IDE project instead of building",
    )
    parser.add_argument(
        "--run",
        action="store_true",
        help="Build and run benchmarks after building",
    )
    parser.add_argument(
        "--run-only",
        action="store_true",
        help="Run benchmarks without building (assumes already built)",
    )
    parser.add_argument(
        "filters",
        nargs="*",
        help="Benchmark filters (e.g., BenchSuite BenchCase.Bench)",
    )

    args = parser.parse_args()

    # Build filter string from filter arguments
    filter_rules = ""
    if args.filters:
        benchmark_filter_list = []
        for cur_filter in args.filters:
            if not cur_filter.startswith("-"):
                if ("." not in cur_filter) and (not cur_filter.endswith("*")):
                    cur_filter = cur_filter + ".*"
                benchmark_filter_list.append(cur_filter)
            else:
                filter_rules = f"{filter_rules} {cur_filter}"
        if benchmark_filter_list:
            filter_rules = f'{filter_rules} --benchmark_filter={":".join(benchmark_filter_list)}'

    if args.ide_project:
        gen_googlebenchmark_project()
    elif args.run_only:
        if not run_googlebenchmark(filter_rules=filter_rules):
            raise RuntimeError("Exception occurs when run googlebenchmark")
    elif args.run:
        if not build_googlebenchmark(incremental=True):
            raise RuntimeError("Exception occurs when build googlebenchmark")
        if not run_googlebenchmark(filter_rules=filter_rules):
            raise RuntimeError("Exception occurs when run googlebenchmark")
    else:
        main(filter_rules=filter_rules)
