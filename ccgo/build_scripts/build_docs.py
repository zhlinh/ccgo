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
# dir name as project name
PROJECT_NAME = os.path.basename(SCRIPT_PATH).upper()

# Ensure cmake directory exists in project
PROJECT_NAME_LOWER = PROJECT_NAME.lower()
PROJECT_RELATIVE_PATH = PROJECT_NAME.lower()

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


def build_docs(incremental, tag=""):
    before_time = time.time()
    print(
        f"==================build docs with tag: {tag}, install path: {INSTALL_PATH} ========================"
    )

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
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


def run_docs(incremental, tag=""):
    if not build_docs(incremental, tag):
        return False
    dst_target_path = INSTALL_PATH + f"/_html/index.html"
    print("---------")
    print(f"start open {dst_target_path}")
    webbrowser.open_new_tab(f"file:///{SCRIPT_PATH}/{dst_target_path}")
    return True


def main(choose, filter_rules=""):
    print(f"==========Choose num: [{choose}], filter: [{filter_rules}]===========")

    result = True
    if choose == "1":
        result = build_docs(False, choose)
    elif choose == "2":
        result = run_docs(False, choose)
    else:
        return

    if not result:
        raise RuntimeError("Exception occurs when build or run docs")


if __name__ == "__main__":
    while True:
        if len(sys.argv) >= 2:
            tag = ""
            if len(sys.argv) >= 3:
                tag = sys.argv[2]
            main(sys.argv[1], tag)

            break
        else:
            num = str(
                input(
                    "Enter menu:"
                    + f"\n1. Clean && build {PROJECT_NAME_LOWER} docs"
                    + f"\n2. Run docs"
                    + f"\n3. Exit"
                )
            )
            main(num)
            break
