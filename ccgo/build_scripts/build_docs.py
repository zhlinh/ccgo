#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_docs.py
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
import webbrowser

# Use absolute import for module compatibility
try:
    from ccgo.build_scripts.build_utils import *
except ImportError:
    # Fallback to relative import when run directly
    from build_utils import *

SCRIPT_PATH = os.getcwd()
# PROJECT_NAME and PROJECT_NAME_LOWER are imported from build_utils.py (reads from CCGO.toml)
PROJECT_RELATIVE_PATH = PROJECT_NAME_LOWER

if system_is_windows():
    CMAKE_GENERATOR = '-G "Unix Makefiles"'
else:
    CMAKE_GENERATOR = ""

BUILD_OUT_PATH = "cmake_build/Docs"
CMAKE_SYSTEM_NAME = platform.system()
INSTALL_PATH = BUILD_OUT_PATH + "/" + CMAKE_SYSTEM_NAME + ".out"

DOCS_BUILD_CMD = (
    "cmake ../../docs %s -DCMAKE_BUILD_TYPE=Release && make -j8 && make install"
)


def build_docs(incremental):
    before_time = time.time()
    print(
        f"==================build docs install path: {INSTALL_PATH} ========================"
    )

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        incremental=incremental,
        platform="docs",
    )

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    ret = os.system(DOCS_BUILD_CMD % CMAKE_GENERATOR)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build docs fail!!!!!!!!!!!!!!!")
        return False

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    dst_target_path = INSTALL_PATH + f"/_html/index.html"

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_target_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def run_docs(incremental):
    if not build_docs(incremental):
        return False
    dst_target_path = INSTALL_PATH + f"/_html/index.html"
    print("---------")
    print(f"start open {dst_target_path}")
    webbrowser.open_new_tab(f"file:///{SCRIPT_PATH}/{dst_target_path}")
    return True


def main(open_browser=False):
    """
    Main entry point for building documentation.

    Args:
        open_browser: If True, open the documentation in browser after building
    """
    if open_browser:
        if not run_docs(False):
            raise RuntimeError("Exception occurs when build or run docs")
    else:
        if not build_docs(False):
            raise RuntimeError("Exception occurs when build docs")


# Command-line interface for docs builds
#
# Usage:
#   python build_docs.py              # Build documentation (default)
#   python build_docs.py --open       # Build and open in browser
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Build documentation",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--open",
        action="store_true",
        help="Open documentation in browser after building",
    )

    args = parser.parse_args()
    main(open_browser=args.open)
