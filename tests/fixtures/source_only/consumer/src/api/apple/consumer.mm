#include "consumer/consumer.h"
#include "leaf/leaf.h"

extern "C" const char* consumer_calls_leaf(void) {
    return leaf_version_marker();
}
