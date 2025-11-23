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

# check ${CMAKE_SOURCE_DIR}/CMakeConfig.local.cmake exists
if(EXISTS ${CMAKE_SOURCE_DIR}/CMakeConfig.local.cmake)
    # eixst then include it
    include(${CMAKE_SOURCE_DIR}/CMakeConfig.local.cmake)
endif()

if (NOT DEFINED CONFIG_COMM_CMAKE_CXX_STANDARD)
    # use c++14 as default
    set(CONFIG_COMM_CMAKE_CXX_STANDARD 14 CACHE STRING "c++ standard")
endif()
message(STATUS "CONFIG_COMM_CMAKE_CXX_STANDARD: ${CONFIG_COMM_CMAKE_CXX_STANDARD}")

if(NOT DEFINED CONFIG_COMM_DEPS_MAP)
    # deps map as empty by default
    set(CONFIG_COMM_DEPS_MAP "" CACHE STRING "deps map")
endif()
message(STATUS "CONFIG_COMM_DEPS_MAP: ${CONFIG_COMM_DEPS_MAP}")

if(NOT DEFINED CONFIG_COMM_PRESET_VISIBILITY)
    # Unless specified, disable symbols visibility by default
    set(CONFIG_COMM_PRESET_VISIBILITY 1 CACHE BOOL "preset visibility")
endif()
message(STATUS "CONFIG_COMM_PRESET_VISIBILITY: ${CONFIG_COMM_PRESET_VISIBILITY}")

if(CONFIG_COMM_PRESET_VISIBILITY)
    set(CMAKE_CXX_VISIBILITY_PRESET default)
    set(CMAKE_C_VISIBILITY_PRESET default)
    # for toolchain.ios.cmake and CMakeFunctions.cmake
    set(ENABLE_VISIBILITY 1)
else()
    set(CMAKE_CXX_VISIBILITY_PRESET hidden)
    set(CMAKE_C_VISIBILITY_PRESET hidden)
    # for toolchain.ios.cmake and CMakeFunctions.cmake
    set(ENABLE_VISIBILITY 0)
endif()
