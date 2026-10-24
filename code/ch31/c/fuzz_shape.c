/* Раздел 31.4: цель по форме команды без Arbitrary. Первый байт входа —
 * код команды, дальше поля нагрузки; длину, число точек и CRC считает
 * обёртка. */
#include "cmd.h"

#include <string.h>

/* ANCHOR: shape */
int LLVMFuzzerTestOneInput(const uint8_t *data, size_t size)
{
    uint8_t wire[4 + 255];
    if (size < 3 || size - 1 > 255)
        return 0;
    size_t plen = size - 1;
    wire[0] = SYNC;
    wire[1] = (uint8_t)(1 + data[0] % 4);
    wire[2] = (uint8_t)plen;
    memcpy(wire + 3, data + 1, plen);
    if (wire[1] == 3) {
        plen = 3 + (plen - 3) / 4 * 4;
        wire[2] = (uint8_t)plen;
        wire[5] = (uint8_t)((plen - 3) / 4); /* n — по числу точек */
    }
    wire[3 + plen] = crc8(wire + 1, 2 + plen);
    struct command cmd;
    parse_v1(wire, 4 + plen, &cmd);
    return 0;
}
/* ANCHOR_END: shape */
