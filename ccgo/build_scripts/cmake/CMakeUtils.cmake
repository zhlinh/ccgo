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

message(STATUS "==============config ${PROJECT_NAME}====================")

include(${CCGO_CMAKE_DIR}/CMakeConfig.cmake)
include(${CCGO_CMAKE_DIR}/CMakeFunctions.cmake)

message(STATUS "==============CMakeConfig ${CCGO_CMAKE_DIR}/CMakeConfig.cmake====================")

# just debug;release
set(CMAKE_CONFIGURATION_TYPES "Debug;Release" CACHE STRING "" FORCE)

set_property(GLOBAL PROPERTY USE_FOLDERS ON)

# include root dir
include_directories(${CMAKE_SOURCE_DIR}/)

# include root 'include' dir
include_directories(${CMAKE_SOURCE_DIR}/include)

# include root 'src' dir
include_directories(${CMAKE_SOURCE_DIR}/src)

# export compile_commands.json
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# set visibility
set(CMAKE_CXX_VISIBILITY_PRESET default)
set(CMAKE_C_VISIBILITY_PRESET default)

if(COMM_USE_SYSTEM_INCLUDES)
    set(COMM_INTERNAL_INCLUDE_WARNING_GUARD SYSTEM)
else()
    set(COMM_INTERNAL_INCLUDE_WARNING_GUARD "")
endif()

# only the submodules of add_subdirectory are installed
if(NOT CMAKE_SOURCE_DIR STREQUAL PROJECT_SOURCE_DIR)
    option(COMM_ENABLE_INSTALL "Enable install rule" OFF)
else()
    option(COMM_ENABLE_INSTALL "Enable install rule" ON)
endif()

# set ide folder
if(NOT DEFINED COMM_IDE_FOLDER)
    set(COMM_IDE_FOLDER ${MAIN_PROJECT_NAME})
endif()

# enable export
add_definitions(-DCOMM_ENABLE_EXPORTS=1)

macro(add_third_party_option conf_name desc value)
    message(STATUS "add_third_party_option ${conf_name} '${desc}' ${value}")
    option(${conf_name} ${desc} ${value})

    if(${value})
        option(THIRD_PARTY_OPTION "third party option" ON)
    endif()
endmacro()

# The prefix for logging tags, usually the project name.
if(COMM_TAG_PREFIX)
    add_definitions(-DCOMM_TAG_PREFIX=\"${COMM_TAG_PREFIX}\")
else()
    add_definitions(-DCOMM_TAG_PREFIX=\"${MAIN_PROJECT_NAME}\")
endif()

# The suffix for logging tags, usually the build time.
if(COMM_LOG_TAG_SUFFIX)
    add_definitions(-DCOMM_LOG_TAG_SUFFIX=\"${COMM_LOG_TAG_SUFFIX}\")
endif()

if(NOT COMM_REVISION)
    # get revision (suppress errors if not in a git repository)
    execute_process(COMMAND git rev-parse --short HEAD
        WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR}
        OUTPUT_VARIABLE COMM_REVISION
        ERROR_QUIET
        OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()

if(COMM_REVISION)
    add_definitions(-DCOMM_REVISION=\"${COMM_REVISION}\")
endif()

# logcomm does not have source code, just use option
option(LOGCOMM_SUPPORT "use logcomm provided logfile support" ON)

# foundrycomm does not have source code, just use option
option(FOUNDRYCOMM_SUPPORT "use foundrycomm provided base support" ON)

# rapidjson does not have source code, just use option
option(RAPIDJSON_SUPPORT "use rapidjson provided json support" ON)

# this will loop all third_party sub dir, if the sub dir has src dir, then add option
file(GLOB third_party_children RELATIVE
    "${CMAKE_SOURCE_DIR}/third_party/"
    ${CMAKE_SOURCE_DIR}/third_party/*
)

foreach(dir ${third_party_children})
    if(IS_DIRECTORY ${CMAKE_SOURCE_DIR}/third_party/${dir})
        if(EXISTS ${CMAKE_SOURCE_DIR}/third_party/${dir}/src)
            string(TOUPPER ${dir} upper_dir)
            add_third_party_option(${upper_dir}_SUPPORT "use ${upper_dir} support" ON)
        endif()
    endif()
endforeach()

if(EXISTS ${CMAKE_SOURCE_DIR}/third_party/zlib/src/)
    add_third_party_option(ZLIB_SUPPORT "use zlib minizip support" ON)
endif()

add_third_party_option(GOOGLETEST_SUPPORT "use googletest provided cpp unittest support" OFF)
add_third_party_option(BENCHMARK_SUPPORT "use googlebenchmark provided benchmark support" OFF)

# include api of ios, macos, apple
include_directories(${CMAKE_SOURCE_DIR}/include/${MAIN_PROJECT_NAME}/api/ios/)
include_directories(${CMAKE_SOURCE_DIR}/include/${MAIN_PROJECT_NAME}/api/macos/)
include_directories(${CMAKE_SOURCE_DIR}/include/${MAIN_PROJECT_NAME}/api/apple/)

# include third party include dir
set(COMM_THIRD_PARTY_INCLUDE_DIRS "")
get_third_party_include_directories(COMM_THIRD_PARTY_INCLUDE_DIRS ${CMAKE_SOURCE_DIR}/third_party/)

foreach(dir ${COMM_THIRD_PARTY_INCLUDE_DIRS})
    set(third_party_sub_dir ${CMAKE_SOURCE_DIR}/third_party/${dir})
    include_directories(${third_party_sub_dir}/include/)
endforeach()

if(MSVC)
    include_directories(${CMAKE_SOURCE_DIR}/third_party/pthread/src/windows)

    # for pthread
    add_definitions(-D__PTW32_STATIC_LIB=1)
endif()

# google test or benchmark, then disable log
# if (GOOGLETEST_SUPPORT OR BENCHMARK_SUPPORT)
# add_definitions(-DCOMM_DISABLE_LOG=1)
# endif()
if(GOOGLETEST_SUPPORT)
    add_definitions(-DCOMM_ENABLE_GOOGLETEST=1)
    set(ENABLE_VISIBILITY 1)
endif()

if(BENCHMARK_SUPPORT)
    add_definitions(-DCOMM_ENABLE_BENCHMARK=1)
    set(ENABLE_VISIBILITY 1)
endif()

if(LOGCOMM_SUPPORT)
    add_definitions(-DCOMM_ENABLE_LOGCOMM=1)
endif()

set(CMAKE_CXX_STANDARD ${CONFIG_COMM_CMAKE_CXX_STANDARD})
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(ANDROID)
    if(NOT PROTOBUF_SUPPORT)
        set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fPIC")
    endif()

    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=gnu++${CONFIG_COMM_CMAKE_CXX_STANDARD}")

    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -O0 -DDEBUG")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fexceptions")

    # Don't re-export libgcc symbols in every binary.
    set(SELF_LINKER_FLAGS "")
    add_exclude_lib_link_option(SELF_LINKER_FLAGS libgcc.a)

    # arm32 currently uses a linker script in place of libgcc to ensure that
    # libunwind is linked in the correct order. --exclude-libs does not propagate to
    # the contents of the linker script and can't be specified within the linker
    # script. Hide both regardless of architecture to future-proof us in case we
    # move other architectures to a linker script (which we may want to do so we
    # automatically link libclangrt on other architectures).
    add_exclude_lib_link_option(SELF_LINKER_FLAGS libgcc_real.a)
    add_exclude_lib_link_option(SELF_LINKER_FLAGS libatomic.a)

    # STL specific flags.
    message(STATUS "ANDROID_STL: ${ANDROID_STL}")

    if(ANDROID_STL MATCHES "^c\\+\\+_")
        if(ANDROID_ABI MATCHES "^armeabi")
            add_exclude_lib_link_option(SELF_LINKER_FLAGS libunwind.a)
        endif()

        if(ANDROID_STL MATCHES "_static$")
            add_exclude_lib_link_option(SELF_LINKER_FLAGS libc++_static.a)
        endif()
    endif()

    message(STATUS "ANDROID SELF_LINKER_FLAGS: ${SELF_LINKER_FLAGS}")
    set(CMAKE_SHARED_LINKER_FLAGS "${SELF_LINKER_FLAGS} ${CMAKE_SHARED_LINKER_FLAGS}")
    set(CMAKE_MODULE_LINKER_FLAGS "${SELF_LINKER_FLAGS} ${CMAKE_MODULE_LINKER_FLAGS}")
    set(CMAKE_EXE_LINKER_FLAGS "${SELF_LINKER_FLAGS} ${CMAKE_EXE_LINKER_FLAGS}")

elseif(OHOS)
    if(NOT PROTOBUF_SUPPORT)
        set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fPIC")
    endif()

    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=gnu++${CONFIG_COMM_CMAKE_CXX_STANDARD}")

    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -O0 -DDEBUG")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fexceptions")

    # Don't re-export libgcc symbols in every binary.
    set(SELF_LINKER_FLAGS "")
    add_exclude_lib_link_option(SELF_LINKER_FLAGS libgcc.a)

    # arm32 currently uses a linker script in place of libgcc to ensure that
    # libunwind is linked in the correct order. --exclude-libs does not propagate to
    # the contents of the linker script and can't be specified within the linker
    # script. Hide both regardless of architecture to future-proof us in case we
    # move other architectures to a linker script (which we may want to do so we
    # automatically link libclangrt on other architectures).
    add_exclude_lib_link_option(SELF_LINKER_FLAGS libgcc_real.a)
    add_exclude_lib_link_option(SELF_LINKER_FLAGS libatomic.a)

    # STL specific flags.
    message(STATUS "OHOS_STL: ${OHOS_STL}")

    if(OHOS_STL MATCHES "^c\\+\\+_")
        if(OHOS_STL MATCHES "^armeabi")
            add_exclude_lib_link_option(SELF_LINKER_FLAGS libunwind.a)
        endif()

        if(OHOS_STL MATCHES "_static$")
            add_exclude_lib_link_option(SELF_LINKER_FLAGS libc++_static.a)
        endif()
    endif()

    message(STATUS "OHOS SELF_LINKER_FLAGS: ${SELF_LINKER_FLAGS}")
    set(CMAKE_SHARED_LINKER_FLAGS "${SELF_LINKER_FLAGS} ${CMAKE_SHARED_LINKER_FLAGS}")
    set(CMAKE_MODULE_LINKER_FLAGS "${SELF_LINKER_FLAGS} ${CMAKE_MODULE_LINKER_FLAGS}")
    set(CMAKE_EXE_LINKER_FLAGS "${SELF_LINKER_FLAGS} ${CMAKE_EXE_LINKER_FLAGS}")

elseif(APPLE)
    # for gen xcode project file
    set(CMAKE_XCODE_ATTRIBUTE_CLANG_CXX_LANGUAGE_STANDARD "gnu++${CONFIG_COMM_CMAKE_CXX_STANDARD}")
    set(CMAKE_XCODE_ATTRIBUTE_CLANG_CXX_LIBRARY "libc++")

    # for build
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=gnu++${CONFIG_COMM_CMAKE_CXX_STANDARD}")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -stdlib=libc++")
    set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} -g -gline-tables-only -Os")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -g -gline-tables-only -Os")

    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -O0 -DDEBUG")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fexceptions")
    set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} -g -O0")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -g -O0")

    if(NOT DEFINED ENABLE_BITCODE)
        # Unless specified, enable BITCODE support by default
        set(ENABLE_BITCODE TRUE CACHE BOOL "[DEFAULTS] Enabling BITCODE support by default.")
    endif()

    if(ENABLE_BITCODE)
        set(CMAKE_XCODE_ATTRIBUTE_ENABLE_BITCODE "YES")
    else()
        set(CMAKE_XCODE_ATTRIBUTE_ENABLE_BITCODE "NO")
    endif()

    set(CMAKE_XCODE_ATTRIBUTE_STRIP_STYLE "all")
    set(CMAKE_XCODE_ATTRIBUTE_IPHONEOS_DEPLOYMENT_TARGET "10.0")
    set(CMAKE_XCODE_ATTRIBUTE_OTHER_LDFLAGS "-ObjC")
    set(CMAKE_XCODE_ATTRIBUTE_DEBUG_INFORMATION_FORMAT "dwarf-with-dsym")
    set(CMAKE_XCODE_ATTRIBUTE_CLANG_DEBUG_INFORMATION_LEVEL[variant=Debug] "default")
    set(CMAKE_XCODE_ATTRIBUTE_CLANG_DEBUG_INFORMATION_LEVEL[variant=Release] "line-tables-only")

    if(DEFINED IOS_DEPLOYMENT_TARGET)
        message(STATUS "setting IOS_DEPLOYMENT_TARGET=${IOS_DEPLOYMENT_TARGET}")
        set(CMAKE_XCODE_ATTRIBUTE_IPHONEOS_DEPLOYMENT_TARGET "${IOS_DEPLOYMENT_TARGET}")
    endif()

elseif(MSVC)
    add_definitions(-DCMAKE_CXX_STANDARD=${CONFIG_COMM_CMAKE_CXX_STANDARD})

    # pthreadVC2 import by static
    add_definitions(-DPTW32_STATIC_LIB)

    # add DEBUG macro .. release has NDEBUG defaultly
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} /DDEBUG")

    if(CMAKE_CL_64)
        add_definitions(-D_WIN64 -DWIN64)
    endif()

    add_definitions(-D_WIN32 -DWIN32 -DUNICODE -D_UNICODE -DNOMINMAX -D_LIB)

    # set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /MP /Zc:threadSafeInit-")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /MP")

    # generate pdb file
    # /Z7: will generate target file with debug info, but will not generate program database (.pdb) file. This option can be used for debugging without Visual Studio, but will increase the size of the target file.
    # /Zi: will generate a program database (.pdb) file containing debug information for use during debugging. This option is only added in Release mode, because in Debug mode, CMake will add the /Zi option by default.
    # /Zd: will generate target file with debug info and embed debug info into the target file. This option can be used for debugging without Visual Studio, but will increase the size of the target file.
    # /ZI: This option is similar to /Zi, but will generate a precompiled header file (.pch) during compilation to speed up compilation. This option is only added in Release mode, because in Debug mode, CMake will add the /ZI option by default.
    if(BENCHMARK_SUPPORT OR BENCHMARK_SUPPORT)
        set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /Z7")
        set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} /Z7")
        set(CMAKE_SHARED_LINKER_FLAGS_RELEASE "${CMAKE_SHARED_LINKER_FLAGS_RELEASE} /DEBUG:FULL")
    else()
        set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} /Zi")
        set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} /Zi")
        set(CMAKE_SHARED_LINKER_FLAGS_RELEASE "${CMAKE_SHARED_LINKER_FLAGS_RELEASE} /DEBUG /OPT:REF /OPT:ICF")
    endif()

    set(CompilerFlags
        CMAKE_CXX_FLAGS
        CMAKE_CXX_FLAGS_DEBUG
        CMAKE_CXX_FLAGS_RELEASE
        CMAKE_C_FLAGS
        CMAKE_C_FLAGS_DEBUG
        CMAKE_C_FLAGS_RELEASE)

    foreach(CompilerFlag ${CompilerFlags})
        string(REPLACE "/MD" "/MT" ${CompilerFlag} "${${CompilerFlag}}")
    endforeach()

    add_compile_options("$<$<C_COMPILER_ID:MSVC>:/utf-8>")
    add_compile_options("$<$<CXX_COMPILER_ID:MSVC>:/utf-8>")
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} /FORCE:MULTIPLE")
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} /FORCE:MULTIPLE")

elseif(UNIX)
    add_definitions(-D__linux__ -Dlinux -D__linux)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -std=gnu++${CONFIG_COMM_CMAKE_CXX_STANDARD}")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fexceptions")

    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fPIC -ffunction-sections -fdata-sections -Os")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -fPIC -ffunction-sections -fdata-sections -Os")

    set(CMAKE_C_FLAGS_RELEASE "${CMAKE_C_FLAGS_RELEASE} -g")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -g")

    set(CMAKE_C_FLAGS_DEBUG "${CMAKE_C_FLAGS_DEBUG} -g -O0")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -g -O0")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -O0 -DDEBUG")

    # for error of .rodata can not be used when making a PIE object; recompile with -fPIC
    SET(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -no-pie")

    # when use GNU toolchain
    if(CMAKE_C_COMPILER_ID STREQUAL "GNU")
        # add link option
        set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -Wl,--no-undefined -Wl,--allow-multiple-definition")
    endif()
endif()
