if(ANDROID OR APPLE)
    # To resolve the issue of "Cmake: The C compiler “xxx”is not able to compile a simple test"
    set(CMAKE_C_COMPILER_WORKS 1)
    set(CMAKE_CXX_COMPILER_WORKS 1)

    set(SELF_EXTRA_FLAGS "-Werror -Wall -Wtype-limits -Wuninitialized -Wempty-body -Wno-error=unknown-warning-option")

    # for gtest has sign compare
    if(NOT GOOGLETEST_SUPPORT)
        set(SELF_EXTRA_FLAGS "${SELF_EXTRA_FLAGS} -Werror=sign-compare")
    endif()

    if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
        # Clang or AppleClang
        set(SELF_EXTRA_FLAGS "${SELF_EXTRA_FLAGS} -Wconsumed -Wno-error=char-subscripts -Wno-error=gnu-designator -Wno-error=unused-variable")
    else()
        message(STATUS "CMAKE_CXX_COMPILER_ID ${CMAKE_CXX_COMPILER_ID}")
        set(SELF_EXTRA_FLAGS "${SELF_EXTRA_FLAGS} -Wclobbered")
    endif()

    if(APPLE)
        set(SELF_EXTRA_FLAGS "${SELF_EXTRA_FLAGS} -Wno-deprecated-declarations")
    endif()

    # add other c flags
    set(SELF_C_EXTRA_FLAGS "-Wno-deprecated-non-prototype")

    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${SELF_EXTRA_FLAGS}")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${SELF_EXTRA_FLAGS} ${SELF_C_EXTRA_FLAGS}")

elseif(MSVC)
    # set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} /wd4003 /wd4819 /wd4996 /wd4244 /wd4101 /wd4200 /wd4800 /wd4005 /wd4334")
    # set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /wd4003 /wd4819 /wd4996 /wd4244 /wd4101 /wd4200 /wd4800 /wd4005 /wd4334")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -W4")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -W4")
endif()
