// Copyright 2024 Tencent. All rights reserved.
#ifndef TESTS_FIXTURES_LINKAGE_CONSUMER_STATIC_EMBEDDED_INCLUDE_CONSUMER_CONSUMER_H_
#define TESTS_FIXTURES_LINKAGE_CONSUMER_STATIC_EMBEDDED_INCLUDE_CONSUMER_CONSUMER_H_

#if defined(_WIN32)
#define CONSUMER_EXPORT __declspec(dllexport)
#elif defined(__GNUC__) || defined(__clang__)
#define CONSUMER_EXPORT __attribute__((visibility("default")))
#else
#define CONSUMER_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif
CONSUMER_EXPORT const char *consumer_call_through(void);
#ifdef __cplusplus
}
#endif
#endif  // TESTS_FIXTURES_LINKAGE_CONSUMER_STATIC_EMBEDDED_INCLUDE_CONSUMER_CONSUMER_H_
