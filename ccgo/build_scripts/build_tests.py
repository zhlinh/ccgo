#!/usr/bin/env python3
# -- coding: utf-8 --
#
# build_tests.py
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
# dir name as project name
PROJECT_NAME = os.path.basename(SCRIPT_PATH).upper()

# Ensure cmake directory exists in project
PROJECT_NAME_LOWER = PROJECT_NAME.lower()
PROJECT_RELATIVE_PATH = PROJECT_NAME.lower()


BUILD_OUT_PATH = os.path.join("cmake_build", "Tests")
CMAKE_SYSTEM_NAME = platform.system()
INSTALL_PATH = os.path.join(BUILD_OUT_PATH, CMAKE_SYSTEM_NAME + ".out")

CURRENT_TIME = datetime.now()
FORMATTED_TIME = CURRENT_TIME.strftime("%Y%m%d_%H%M%S_%f")
if system_is_macos():
    # change Darwin to macos here
    FORMATTED_SYSTEM_NAME = "macos"
else:
    FORMATTED_SYSTEM_NAME = CMAKE_SYSTEM_NAME.lower().replace("/", "_")
PARAM_FOR_OUTPUT_XML = f'--gtest_output=xml:{os.path.join(BUILD_OUT_PATH, f"tests_on_{FORMATTED_SYSTEM_NAME}_result_{FORMATTED_TIME}.xml")}'

BUILD_TYPE = "Release"

TESTS_EXTRA_FLAGS = f"-DGOOGLETEST_SUPPORT=ON -DCCGO_CMAKE_DIR={CCGO_CMAKE_DIR}"

if system_is_windows():
    # -DCMAKE_BUILD_TYPE=xxx not working for vs
    GOOGLETEST_BUILD_CMD = f'cmake ../.. -G "Visual Studio 16 2019" -T v142 {TESTS_EXTRA_FLAGS} && cmake --build . --target install --config {BUILD_TYPE}'
else:
    GOOGLETEST_BUILD_CMD = f"cmake ../.. -DCMAKE_BUILD_TYPE={BUILD_TYPE} {TESTS_EXTRA_FLAGS} && make -j8 && make install"


if system_is_windows():
    GEN_PROJECT_CMD = (
        f'cmake ../.. -G "Visual Studio 16 2019" -T v142 {TESTS_EXTRA_FLAGS}'
    )
elif system_is_macos():
    GEN_PROJECT_CMD = f"cmake ../.. -G Xcode -DCMAKE_OSX_DEPLOYMENT_TARGET:STRING=10.9 -DENABLE_BITCODE=0 {TESTS_EXTRA_FLAGS}"
else:
    GEN_PROJECT_CMD = f'cmake ../.. -G "CodeLite - Unix Makefiles" {TESTS_EXTRA_FLAGS}'


def build_googletest(incremental, tag=""):
    before_time = time.time()
    print(
        f"==================build_googletest with tag: {tag}, install path: {INSTALL_PATH} ========================"
    )

    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        incremental=incremental,
        platform="tests",
    )
    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    ret = os.system(GOOGLETEST_BUILD_CMD)
    os.chdir(SCRIPT_PATH)
    if ret != 0:
        print("!!!!!!!!!!!build googletest fail!!!!!!!!!!!!!!!")
        return False

    clean(BUILD_OUT_PATH, incremental)
    os.chdir(BUILD_OUT_PATH)

    dst_target_path = os.path.relpath(
        os.path.abspath(os.path.join(INSTALL_PATH, f"{PROJECT_NAME_LOWER}_googletest"))
    )

    print(time.strftime("%Y-%m-%d %H:%M:%S", time.localtime()))
    print("==================Output========================")
    print(dst_target_path)

    after_time = time.time()

    print(f"use time: {int(after_time - before_time)} s")
    return True


def run_googletest(filter_rules=""):
    os.chdir(SCRIPT_PATH)
    for fpath, dirs, fs in os.walk(INSTALL_PATH):
        for file_name in fs:
            # for convert / to \ in windows
            file = os.path.relpath(os.path.abspath(os.path.join(fpath, file_name)))
            if file.find("_googletest") >= 0:
                if len(filter_rules) > 0:
                    cmd = f"{file} {filter_rules}"
                else:
                    cmd = f"{file}"
                cmd = f"{cmd} {PARAM_FOR_OUTPUT_XML}"
                print(f"start exec {cmd}")
                ret = os.system(cmd)
                if ret != 0:
                    print(f"!!!!!!!!!!!run googletest {file} fail!!!!!!!!!!!!!!!")
                    return False
                else:
                    print(f"[INFO] run googletest {file} success\n")
    return True


def gen_googletest_project(tag=""):
    print("==================gen_macos_project========================")
    # generate verinfo.h
    gen_project_revision_file(
        PROJECT_NAME,
        OUTPUT_VERINFO_PATH,
        get_version_name(SCRIPT_PATH),
        tag,
        platform="tests",
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


def main(choose, filter_rules=""):
    print(f"==========Choose num: [{choose}], filter: [{filter_rules}]===========")
    if system_is_windows() and (not check_vs_env()):
        return

    result = True
    if choose == "1":
        result = build_googletest(incremental=False, tag=choose)
    elif choose == "2":
        result = gen_googletest_project(tag=choose)
    elif choose == "3":
        # build and run
        if not build_googletest(incremental=True, tag=choose):
            return False
        result = run_googletest(filter_rules=filter_rules)
    elif choose == "4":
        # run
        result = run_googletest(filter_rules=filter_rules)
    else:
        result = build_googletest(incremental=False, tag=choose)

    if not result:
        raise RuntimeError("Exception occurs when build or run googletest")


if __name__ == "__main__":
    while True:
        if len(sys.argv) >= 2:
            filter = ""
            if len(sys.argv) >= 3:
                gtest_filter_list = []
                for i in range(2, len(sys.argv)):
                    cur_filter = sys.argv[i]
                    if not cur_filter.startswith("-"):
                        if ("." not in cur_filter) and (not cur_filter.endswith("*")):
                            # add '*' at the end if not begins with '-'
                            cur_filter = cur_filter + "*"
                        gtest_filter_list.append(cur_filter)
                    else:
                        filter = f"{filter} {cur_filter}"
                if len(gtest_filter_list) > 0:
                    # add '--gtest_filter=' if gtest_filter_list is not empty, ":" as separator
                    filter = f'{filter} --gtest_filter={":".join(gtest_filter_list)}'
            main(sys.argv[1], filter)
            break
        else:
            num = str(
                input(
                    f"Enter menu:(or usage: python3 {os.path.basename(__file__)} <tag> <gtest_filters separated by space>)"
                    f"\n1. Clean && Build {PROJECT_NAME_LOWER} googletest"
                    f"\n2. Gen {PROJECT_NAME_LOWER} googletest Project"
                    f"\n3. Build && Run {PROJECT_NAME_LOWER} googletest"
                    f"\n4. Exit\n"
                )
            )
            main(num)
            break
