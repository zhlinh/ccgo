# FindCCGODependencies.cmake
#
# Automatically discover and configure dependencies installed via 'ccgo install'
# Dependencies are scanned from multiple directories (in priority order):
#   1. vendor/           - Vendored dependencies (committed to git)
#   2. .ccgo/deps/       - Dependencies from 'ccgo install' (not committed)
#   3. third_party/      - Manual third-party libraries (committed to git)
#
# This module sets:
#   CCGO_DEPENDENCIES_FOUND    - TRUE if any dependencies found
#   CCGO_DEPENDENCY_<NAME>_FOUND - TRUE if specific dependency found
#   CCGO_DEPENDENCY_<NAME>_INCLUDE_DIRS - Include directories
#   CCGO_DEPENDENCY_<NAME>_LIBRARIES - Libraries to link
#   CCGO_DEPENDENCY_<NAME>_STATIC_LIBRARIES - Static libraries
#   CCGO_DEPENDENCY_<NAME>_SHARED_LIBRARIES - Shared libraries
#
# Usage:
#   include(FindCCGODependencies)
#   find_ccgo_dependencies()
#
#   # Link specific dependency
#   target_link_libraries(myapp PRIVATE ${CCGO_DEPENDENCY_libfoo_LIBRARIES})
#   target_include_directories(myapp PRIVATE ${CCGO_DEPENDENCY_libfoo_INCLUDE_DIRS})
#

# Determine current platform
function(ccgo_detect_platform OUT_PLATFORM)
    if(ANDROID)
        set(${OUT_PLATFORM} "android" PARENT_SCOPE)
    elseif(IOS)
        set(${OUT_PLATFORM} "ios" PARENT_SCOPE)
    elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
        if(CMAKE_OSX_SYSROOT MATCHES "appletvos")
            set(${OUT_PLATFORM} "tvos" PARENT_SCOPE)
        elseif(CMAKE_OSX_SYSROOT MATCHES "watchos")
            set(${OUT_PLATFORM} "watchos" PARENT_SCOPE)
        else()
            set(${OUT_PLATFORM} "macos" PARENT_SCOPE)
        endif()
    elseif(CMAKE_SYSTEM_NAME STREQUAL "Windows")
        set(${OUT_PLATFORM} "windows" PARENT_SCOPE)
    elseif(CMAKE_SYSTEM_NAME STREQUAL "Linux")
        set(${OUT_PLATFORM} "linux" PARENT_SCOPE)
    elseif(OHOS)
        set(${OUT_PLATFORM} "ohos" PARENT_SCOPE)
    else()
        set(${OUT_PLATFORM} "unknown" PARENT_SCOPE)
    endif()
endfunction()

# Detect architecture
function(ccgo_detect_architecture OUT_ARCH)
    if(ANDROID)
        if(ANDROID_ABI STREQUAL "arm64-v8a")
            set(${OUT_ARCH} "arm64-v8a" PARENT_SCOPE)
        elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
            set(${OUT_ARCH} "armeabi-v7a" PARENT_SCOPE)
        elseif(ANDROID_ABI STREQUAL "x86_64")
            set(${OUT_ARCH} "x86_64" PARENT_SCOPE)
        else()
            set(${OUT_ARCH} "${ANDROID_ABI}" PARENT_SCOPE)
        endif()
    elseif(CMAKE_SYSTEM_NAME STREQUAL "Windows")
        set(${OUT_ARCH} "x64" PARENT_SCOPE)
    elseif(OHOS)
        if(OHOS_ARCH STREQUAL "arm64-v8a")
            set(${OUT_ARCH} "arm64-v8a" PARENT_SCOPE)
        elseif(OHOS_ARCH STREQUAL "armeabi-v7a")
            set(${OUT_ARCH} "armeabi-v7a" PARENT_SCOPE)
        elseif(OHOS_ARCH STREQUAL "x86_64")
            set(${OUT_ARCH} "x86_64" PARENT_SCOPE)
        else()
            set(${OUT_ARCH} "${OHOS_ARCH}" PARENT_SCOPE)
        endif()
    else()
        # For Apple platforms and Linux, architecture is not used in path
        set(${OUT_ARCH} "" PARENT_SCOPE)
    endif()
endfunction()

# Find libraries for a specific dependency
function(ccgo_find_dependency_libraries DEP_NAME DEP_PATH PLATFORM ARCH LINK_TYPE)
    set(LIBRARIES "")
    set(INCLUDE_DIRS "")

    # Find include directory
    if(EXISTS "${DEP_PATH}/include")
        list(APPEND INCLUDE_DIRS "${DEP_PATH}/include")
    endif()

    # Determine library directory based on platform and architecture
    set(LIB_DIR "${DEP_PATH}/lib/${PLATFORM}")

    # Check for link type subdirectory (static or shared)
    if(EXISTS "${LIB_DIR}/${LINK_TYPE}")
        set(LIB_DIR "${LIB_DIR}/${LINK_TYPE}")
    endif()

    # Add architecture subdirectory for platforms that use it
    if(ARCH AND EXISTS "${LIB_DIR}/${ARCH}")
        set(LIB_DIR "${LIB_DIR}/${ARCH}")
    endif()

    if(NOT EXISTS "${LIB_DIR}")
        message(WARNING "Library directory not found for ${DEP_NAME}: ${LIB_DIR}")
        return()
    endif()

    # Find libraries based on platform
    if(PLATFORM STREQUAL "android" OR PLATFORM STREQUAL "ohos")
        if(LINK_TYPE STREQUAL "static")
            file(GLOB LIB_FILES "${LIB_DIR}/*.a")
        else()
            file(GLOB LIB_FILES "${LIB_DIR}/*.so")
        endif()
    elseif(PLATFORM STREQUAL "ios" OR PLATFORM STREQUAL "macos" OR PLATFORM STREQUAL "tvos" OR PLATFORM STREQUAL "watchos")
        # Check for frameworks first
        file(GLOB FRAMEWORKS "${LIB_DIR}/*.framework" "${LIB_DIR}/*.xcframework")
        if(FRAMEWORKS)
            list(APPEND LIBRARIES ${FRAMEWORKS})
        else()
            if(LINK_TYPE STREQUAL "static")
                file(GLOB LIB_FILES "${LIB_DIR}/*.a")
            else()
                file(GLOB LIB_FILES "${LIB_DIR}/*.dylib")
            endif()
        endif()
    elseif(PLATFORM STREQUAL "windows")
        if(LINK_TYPE STREQUAL "static")
            file(GLOB LIB_FILES "${LIB_DIR}/*.lib")
        else()
            file(GLOB LIB_FILES "${LIB_DIR}/*.dll" "${LIB_DIR}/*.lib")
        endif()
    elseif(PLATFORM STREQUAL "linux")
        if(LINK_TYPE STREQUAL "static")
            file(GLOB LIB_FILES "${LIB_DIR}/*.a")
        else()
            file(GLOB LIB_FILES "${LIB_DIR}/*.so*")
        endif()
    endif()

    if(LIB_FILES)
        list(APPEND LIBRARIES ${LIB_FILES})
    endif()

    # Export to parent scope
    string(TOUPPER ${DEP_NAME} DEP_NAME_UPPER)
    if(LINK_TYPE STREQUAL "static")
        set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_STATIC_LIBRARIES ${LIBRARIES} PARENT_SCOPE)
    else()
        set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_SHARED_LIBRARIES ${LIBRARIES} PARENT_SCOPE)
    endif()
    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS ${INCLUDE_DIRS} PARENT_SCOPE)
    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_LIBRARIES ${LIBRARIES} PARENT_SCOPE)
    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_FOUND TRUE PARENT_SCOPE)
endfunction()

# Main function to discover all dependencies
function(find_ccgo_dependencies)
    # Detect platform and architecture
    ccgo_detect_platform(PLATFORM)
    ccgo_detect_architecture(ARCH)

    message(STATUS "CCGO Dependencies: Scanning for platform=${PLATFORM}, arch=${ARCH}")

    # Find project root
    set(PROJECT_ROOT "${CMAKE_SOURCE_DIR}")

    # Define dependency directories in priority order
    # Higher priority directories are scanned first, and their dependencies take precedence
    set(CCGO_DEP_DIRS
        "${PROJECT_ROOT}/vendor"        # Vendored dependencies (highest priority)
        "${PROJECT_ROOT}/.ccgo/deps"    # Dependencies from 'ccgo install'
        "${PROJECT_ROOT}/third_party"   # Manual third-party libraries (lowest priority)
    )

    # Determine link type preference (for backward compatibility)
    if(NOT DEFINED CCGO_DEPENDENCY_LINK_TYPE)
        # Default to static
        set(CCGO_DEPENDENCY_LINK_TYPE "static")
    endif()

    set(FOUND_COUNT 0)
    set(FOUND_DEP_NAMES "")  # Track found dependencies to avoid duplicates

    # Scan each dependency directory
    foreach(DEP_SEARCH_DIR ${CCGO_DEP_DIRS})
        if(NOT EXISTS "${DEP_SEARCH_DIR}")
            continue()
        endif()

        message(STATUS "CCGO Dependencies: Scanning ${DEP_SEARCH_DIR}")

        file(GLOB DEPENDENCY_DIRS "${DEP_SEARCH_DIR}/*")

        foreach(DEP_DIR ${DEPENDENCY_DIRS})
            if(IS_DIRECTORY "${DEP_DIR}")
                get_filename_component(DEP_NAME ${DEP_DIR} NAME)
                string(TOUPPER ${DEP_NAME} DEP_NAME_UPPER)

                # Skip if already found in a higher priority directory
                list(FIND FOUND_DEP_NAMES ${DEP_NAME} DEP_FOUND_INDEX)
                if(NOT DEP_FOUND_INDEX EQUAL -1)
                    message(STATUS "CCGO Dependencies: Skipping ${DEP_NAME} (already found in higher priority dir)")
                    continue()
                endif()

                message(STATUS "CCGO Dependencies: Found ${DEP_NAME} in ${DEP_SEARCH_DIR}")

                # Try to find both static and shared libraries
                ccgo_find_dependency_libraries(
                    "${DEP_NAME}"
                    "${DEP_DIR}"
                    "${PLATFORM}"
                    "${ARCH}"
                    "static"
                )

                ccgo_find_dependency_libraries(
                    "${DEP_NAME}"
                    "${DEP_DIR}"
                    "${PLATFORM}"
                    "${ARCH}"
                    "shared"
                )

                # Check if found and set default LIBRARIES based on preference
                if(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_FOUND)
                    math(EXPR FOUND_COUNT "${FOUND_COUNT} + 1")
                    list(APPEND FOUND_DEP_NAMES ${DEP_NAME})

                    # Set default LIBRARIES based on preference
                    if(CCGO_DEPENDENCY_LINK_TYPE STREQUAL "shared" AND CCGO_DEPENDENCY_${DEP_NAME_UPPER}_SHARED_LIBRARIES)
                        set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_LIBRARIES ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_SHARED_LIBRARIES} PARENT_SCOPE)
                        message(STATUS "  - Include: ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS}")
                        message(STATUS "  - Libraries (shared): ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_SHARED_LIBRARIES}")
                    elseif(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_STATIC_LIBRARIES)
                        set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_LIBRARIES ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_STATIC_LIBRARIES} PARENT_SCOPE)
                        message(STATUS "  - Include: ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS}")
                        message(STATUS "  - Libraries (static): ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_STATIC_LIBRARIES}")
                    endif()

                    # Export both static and shared libraries
                    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_STATIC_LIBRARIES ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_STATIC_LIBRARIES} PARENT_SCOPE)
                    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_SHARED_LIBRARIES ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_SHARED_LIBRARIES} PARENT_SCOPE)
                    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS} PARENT_SCOPE)
                    set(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_FOUND ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_FOUND} PARENT_SCOPE)
                endif()
            endif()
        endforeach()
    endforeach()

    if(FOUND_COUNT GREATER 0)
        set(CCGO_DEPENDENCIES_FOUND TRUE PARENT_SCOPE)
        message(STATUS "CCGO Dependencies: Found ${FOUND_COUNT} dependency(ies) total")
    else()
        set(CCGO_DEPENDENCIES_FOUND FALSE PARENT_SCOPE)
        message(STATUS "CCGO Dependencies: No dependencies found for current platform")
    endif()
endfunction()

# Helper function to link a dependency to a target
function(ccgo_link_dependency TARGET_NAME DEP_NAME)
    string(TOUPPER ${DEP_NAME} DEP_NAME_UPPER)

    if(NOT CCGO_DEPENDENCY_${DEP_NAME_UPPER}_FOUND)
        message(WARNING "CCGO dependency '${DEP_NAME}' not found, skipping link")
        return()
    endif()

    # Add include directories
    if(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS)
        target_include_directories(${TARGET_NAME} PRIVATE
            ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_INCLUDE_DIRS}
        )
    endif()

    # Link libraries
    if(CCGO_DEPENDENCY_${DEP_NAME_UPPER}_LIBRARIES)
        target_link_libraries(${TARGET_NAME} PRIVATE
            ${CCGO_DEPENDENCY_${DEP_NAME_UPPER}_LIBRARIES}
        )
    endif()

    message(STATUS "CCGO Dependencies: Linked ${DEP_NAME} to ${TARGET_NAME}")
endfunction()
