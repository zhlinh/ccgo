// Copyright 2024 Tencent. All rights reserved.
#include "consumer/consumer.h"

#include "leaf/leaf.h"
extern "C" const char *consumer_call_through(void) {
  return leaf_version_marker();
}
