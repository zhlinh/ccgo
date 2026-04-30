#include "consumer-reg/consumer-reg.h"
#include "leaf/leaf.h"

extern "C" const char* consumer_reg_calls_leaf(void) {
    return leaf_version_marker();
}
