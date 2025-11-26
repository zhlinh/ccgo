#
# Copyright 2024 ccgo Project. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Software.

# CCGO Dependencies Module
# This module helps integrate dependencies managed by CCGO into CMake builds

# Function to add CCGO dependencies to a target
function(ccgo_add_dependencies TARGET_NAME)
    # Check if dependency include directories are defined
    if(DEFINED CCGO_DEP_INCLUDE_DIRS AND NOT "${CCGO_DEP_INCLUDE_DIRS}" STREQUAL "")
        # Convert semicolon-separated string to list
        string(REPLACE ";" "\\;" _include_dirs "${CCGO_DEP_INCLUDE_DIRS}")

        message(STATUS "Adding CCGO dependency include directories to ${TARGET_NAME}")

        # Add include directories to target
        target_include_directories(${TARGET_NAME}
            PRIVATE
            ${_include_dirs}
        )
    endif()

    # Check if dependency paths are defined
    if(DEFINED CCGO_DEP_PATHS AND NOT "${CCGO_DEP_PATHS}" STREQUAL "")
        string(REPLACE ";" "\\;" _dep_paths "${CCGO_DEP_PATHS}")

        message(STATUS "CCGO dependency paths available for ${TARGET_NAME}")
        # Dependency paths are available for manual linking if needed
        # Store in parent scope for use in find_library() calls
        set(CCGO_DEP_PATHS_LIST ${_dep_paths} PARENT_SCOPE)
    endif()
endfunction()

# Function to find and link a CCGO dependency library
# Usage: ccgo_link_dependency(my_target DEPENDENCY_NAME lib_name)
function(ccgo_link_dependency TARGET_NAME DEP_NAME LIB_NAME)
    if(NOT DEFINED CCGO_DEP_PATHS OR "${CCGO_DEP_PATHS}" STREQUAL "")
        message(WARNING "No CCGO dependencies defined, cannot link ${LIB_NAME}")
        return()
    endif()

    # Convert to list
    string(REPLACE ";" "\\;" _dep_paths "${CCGO_DEP_PATHS}")

    # Try to find the library in dependency paths
    set(_lib_found FALSE)
    foreach(_dep_path ${_dep_paths})
        # Check common library locations
        set(_potential_paths
            "${_dep_path}/lib"
            "${_dep_path}/build/lib"
            "${_dep_path}/cmake_build/lib"
            "${_dep_path}"
        )

        foreach(_lib_path ${_potential_paths})
            # Try different library naming conventions
            set(_lib_names
                "lib${LIB_NAME}.a"
                "lib${LIB_NAME}.so"
                "lib${LIB_NAME}.dylib"
                "${LIB_NAME}.lib"
            )

            foreach(_lib_file ${_lib_names})
                set(_full_path "${_lib_path}/${_lib_file}")
                if(EXISTS "${_full_path}")
                    message(STATUS "Found CCGO dependency library: ${_full_path}")
                    target_link_libraries(${TARGET_NAME} PRIVATE "${_full_path}")
                    set(_lib_found TRUE)
                    break()
                endif()
            endforeach()

            if(_lib_found)
                break()
            endif()
        endforeach()

        if(_lib_found)
            break()
        endif()
    endforeach()

    if(NOT _lib_found)
        message(WARNING "Could not find library ${LIB_NAME} for dependency ${DEP_NAME}")
    endif()
endfunction()

# Function to add a CCGO dependency subdirectory
# This is useful when the dependency has a CMakeLists.txt
# Usage: ccgo_add_subdirectory(DEPENDENCY_NAME)
function(ccgo_add_subdirectory DEP_NAME)
    if(NOT DEFINED CCGO_DEP_PATHS OR "${CCGO_DEP_PATHS}" STREQUAL "")
        message(WARNING "No CCGO dependencies defined, cannot add subdirectory for ${DEP_NAME}")
        return()
    endif()

    # Convert to list
    string(REPLACE ";" "\\;" _dep_paths "${CCGO_DEP_PATHS}")

    # Find the dependency
    set(_dep_found FALSE)
    foreach(_dep_path ${_dep_paths})
        get_filename_component(_dep_name_check "${_dep_path}" NAME)
        if("${_dep_name_check}" STREQUAL "${DEP_NAME}")
            # Check if CMakeLists.txt exists
            if(EXISTS "${_dep_path}/CMakeLists.txt")
                message(STATUS "Adding CCGO dependency as subdirectory: ${_dep_path}")
                add_subdirectory("${_dep_path}" "${CMAKE_BINARY_DIR}/ccgo_deps/${DEP_NAME}")
                set(_dep_found TRUE)
                break()
            else()
                message(WARNING "Dependency ${DEP_NAME} at ${_dep_path} does not have CMakeLists.txt")
            endif()
        endif()
    endforeach()

    if(NOT _dep_found)
        message(WARNING "Could not find CCGO dependency: ${DEP_NAME}")
    endif()
endfunction()

# Print dependency information for debugging
function(ccgo_print_dependencies)
    message(STATUS "=== CCGO Dependencies ===")

    if(DEFINED CCGO_DEP_INCLUDE_DIRS AND NOT "${CCGO_DEP_INCLUDE_DIRS}" STREQUAL "")
        message(STATUS "Include directories:")
        string(REPLACE ";" "\\;" _include_dirs "${CCGO_DEP_INCLUDE_DIRS}")
        foreach(_dir ${_include_dirs})
            message(STATUS "  - ${_dir}")
        endforeach()
    else()
        message(STATUS "No include directories defined")
    endif()

    if(DEFINED CCGO_DEP_PATHS AND NOT "${CCGO_DEP_PATHS}" STREQUAL "")
        message(STATUS "Dependency paths:")
        string(REPLACE ";" "\\;" _dep_paths "${CCGO_DEP_PATHS}")
        foreach(_path ${_dep_paths})
            message(STATUS "  - ${_path}")
        endforeach()
    else()
        message(STATUS "No dependency paths defined")
    endif()

    message(STATUS "========================")
endfunction()

# Automatically include CCGO dependency directories if they exist
if(DEFINED CCGO_DEP_INCLUDE_DIRS AND NOT "${CCGO_DEP_INCLUDE_DIRS}" STREQUAL "")
    message(STATUS "CCGO dependencies detected")
endif()
