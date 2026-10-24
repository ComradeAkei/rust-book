/* Раздел 31.4: цель libFuzzer на сырых байтах — то, во что
 * разворачивается fuzz_target! из cargo fuzz. */
#include "cmd.h"

/* ANCHOR: raw */
int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    struct command cmd;
    parse_v1(data, size, &cmd);
    return 0;
}
/* ANCHOR_END: raw */
