#pragma once

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
LEAF_EXPORT const char* leaf_version_marker(void);
#ifdef __cplusplus
}
#endif
