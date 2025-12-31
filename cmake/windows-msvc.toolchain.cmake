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
#
# Toolchain file for cross-compiling to Windows from Linux using clang
# with MSVC ABI compatibility (using xwin for Windows SDK).
#
# This toolchain produces Windows binaries that are ABI-compatible with MSVC,
# using clang as the compiler and lld-link as the linker.
#

# Target system settings
set(CMAKE_SYSTEM_NAME Windows)
set(CMAKE_SYSTEM_PROCESSOR x86_64)

# Signal MSVC-compatible build to CMake
# This enables Windows-specific code paths in CMakeLists.txt files
# Note: MSVC variable is auto-detected by CMake, so we use a custom variable
set(CCGO_MSVC_COMPATIBLE TRUE CACHE BOOL "Building with MSVC ABI compatibility")

# Compiler settings - use clang with MSVC target
set(CMAKE_C_COMPILER clang)
set(CMAKE_CXX_COMPILER clang++)

# Linker and archiver
# Note: Don't set CMAKE_AR directly as CMake will pass Unix-style ar flags
# Instead, use CMAKE_C_CREATE_STATIC_LIBRARY and CMAKE_CXX_CREATE_STATIC_LIBRARY
set(CMAKE_LINKER lld-link)
set(CMAKE_MT llvm-mt)
set(CMAKE_RC_COMPILER llvm-rc)

# Override the static library creation commands to use llvm-lib properly
# llvm-lib uses MSVC-style syntax: llvm-lib /out:lib.lib obj1.obj obj2.obj
set(CMAKE_C_CREATE_STATIC_LIBRARY "<CMAKE_COMMAND> -E rm -f <TARGET> && llvm-lib /out:<TARGET> <OBJECTS>")
set(CMAKE_CXX_CREATE_STATIC_LIBRARY "<CMAKE_COMMAND> -E rm -f <TARGET> && llvm-lib /out:<TARGET> <OBJECTS>")

# Target triple for MSVC ABI
set(CLANG_TARGET_TRIPLE "x86_64-pc-windows-msvc")

# Force static runtime library (libcmt.lib) since xwin doesn't have debug CRT
# This avoids the missing msvcrtd.lib error
set(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded" CACHE STRING "MSVC runtime library" FORCE)

# Disable CMake's automatic MSVC-style flag additions for clang
# This prevents CMake from adding -D_DEBUG and --dependent-lib=msvcrtd
set(CMAKE_C_FLAGS_DEBUG_INIT "-O0 -g")
set(CMAKE_CXX_FLAGS_DEBUG_INIT "-O0 -g")
set(CMAKE_C_FLAGS_RELEASE_INIT "-O2 -DNDEBUG")
set(CMAKE_CXX_FLAGS_RELEASE_INIT "-O2 -DNDEBUG")

# xwin SDK paths (set via environment or default locations)
if(DEFINED ENV{INCLUDE})
    # Parse colon-separated INCLUDE paths
    string(REPLACE ":" ";" XWIN_INCLUDE_DIRS "$ENV{INCLUDE}")
else()
    set(XWIN_INCLUDE_DIRS
        "/opt/xwin/sdk/crt/include"
        "/opt/xwin/sdk/sdk/include/um"
        "/opt/xwin/sdk/sdk/include/shared"
        "/opt/xwin/sdk/sdk/include/ucrt"
    )
endif()

if(DEFINED ENV{LIB})
    # Parse colon-separated LIB paths
    string(REPLACE ":" ";" XWIN_LIB_DIRS "$ENV{LIB}")
else()
    set(XWIN_LIB_DIRS
        "/opt/xwin/sdk/crt/lib/x86_64"
        "/opt/xwin/sdk/sdk/lib/um/x86_64"
        "/opt/xwin/sdk/sdk/lib/ucrt/x86_64"
    )
endif()

# Build include flags
set(INCLUDE_FLAGS "")
foreach(dir ${XWIN_INCLUDE_DIRS})
    if(EXISTS "${dir}")
        set(INCLUDE_FLAGS "${INCLUDE_FLAGS} -I${dir}")
    endif()
endforeach()

# Build library search path flags for linker
set(LIB_FLAGS "")
foreach(dir ${XWIN_LIB_DIRS})
    if(EXISTS "${dir}")
        set(LIB_FLAGS "${LIB_FLAGS} -L${dir}")
    endif()
endforeach()

# Compiler flags for MSVC ABI compatibility
# Use static CRT (libcmt.lib) via --dependent-lib
set(MSVC_COMPAT_FLAGS "--target=${CLANG_TARGET_TRIPLE} -fms-extensions -fms-compatibility -fuse-ld=lld")
set(MSVC_CRT_FLAGS "-D_MT -Xclang --dependent-lib=libcmt")

# Set compile flags
set(CMAKE_C_FLAGS_INIT "${MSVC_COMPAT_FLAGS} ${INCLUDE_FLAGS} ${MSVC_CRT_FLAGS}")
set(CMAKE_CXX_FLAGS_INIT "${MSVC_COMPAT_FLAGS} ${INCLUDE_FLAGS} ${MSVC_CRT_FLAGS}")

# Build library link path flags for lld-link (MSVC-style: /LIBPATH:)
set(LINK_LIB_FLAGS "")
foreach(dir ${XWIN_LIB_DIRS})
    if(EXISTS "${dir}")
        set(LINK_LIB_FLAGS "${LINK_LIB_FLAGS} /LIBPATH:${dir}")
    endif()
endforeach()

# Set linker flags
# For executables: use -L style (clang driver)
set(CMAKE_EXE_LINKER_FLAGS_INIT "${LIB_FLAGS}")
# For shared libraries: use /LIBPATH style (lld-link directly via Wl)
set(CMAKE_SHARED_LINKER_FLAGS_INIT "-fuse-ld=lld ${LIB_FLAGS}")
set(CMAKE_MODULE_LINKER_FLAGS_INIT "-fuse-ld=lld ${LIB_FLAGS}")

# Set shared library link command to use lld-link properly
# clang++ --target=x86_64-pc-windows-msvc -shared -o output.dll input.obj -fuse-ld=lld
set(CMAKE_C_CREATE_SHARED_LIBRARY
    "<CMAKE_C_COMPILER> --target=${CLANG_TARGET_TRIPLE} -fuse-ld=lld -shared <CMAKE_SHARED_LIBRARY_C_FLAGS> <LANGUAGE_COMPILE_FLAGS> <LINK_FLAGS> <CMAKE_SHARED_LIBRARY_CREATE_C_FLAGS> -o <TARGET> <OBJECTS> <LINK_LIBRARIES>")
set(CMAKE_CXX_CREATE_SHARED_LIBRARY
    "<CMAKE_CXX_COMPILER> --target=${CLANG_TARGET_TRIPLE} -fuse-ld=lld -shared <CMAKE_SHARED_LIBRARY_CXX_FLAGS> <LANGUAGE_COMPILE_FLAGS> <LINK_FLAGS> <CMAKE_SHARED_LIBRARY_CREATE_CXX_FLAGS> -o <TARGET> <OBJECTS> <LINK_LIBRARIES>")

# Configure shared library suffixes for Windows
set(CMAKE_SHARED_LIBRARY_PREFIX "")
set(CMAKE_SHARED_LIBRARY_SUFFIX ".dll")
set(CMAKE_IMPORT_LIBRARY_PREFIX "")
set(CMAKE_IMPORT_LIBRARY_SUFFIX ".lib")

# Don't search for programs on the host
set(CMAKE_FIND_ROOT_PATH_MODE_PROGRAM NEVER)
# Search for libraries and includes only in the target environment
set(CMAKE_FIND_ROOT_PATH_MODE_LIBRARY ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_INCLUDE ONLY)
set(CMAKE_FIND_ROOT_PATH_MODE_PACKAGE ONLY)

# For try_compile, use static library to avoid linker issues during configuration
# This only affects CMake's compiler detection, not actual build targets
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)
