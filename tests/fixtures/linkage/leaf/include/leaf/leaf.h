// Copyright 2024 Tencent. All rights reserved.
#ifndef TESTS_FIXTURES_LINKAGE_LEAF_INCLUDE_LEAF_LEAF_H_
#define TESTS_FIXTURES_LINKAGE_LEAF_INCLUDE_LEAF_LEAF_H_

#if defined(_WIN32)
#define LEAF_EXPORT __declspec(dllexport)
#elif defined(__GNUC__) || defined(__clang__)
#define LEAF_EXPORT __attribute__((visibility("default")))
#else
#define LEAF_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif
LEAF_EXPORT const char *leaf_version_marker(void);
#ifdef __cplusplus
}
#endif
#endif  // TESTS_FIXTURES_LINKAGE_LEAF_INCLUDE_LEAF_LEAF_H_
