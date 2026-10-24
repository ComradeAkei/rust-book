/* Раздел 31.4: команда калибровки на девять точек — то, что нашёл
 * фаззер, — без фаззера. */
#include "cmd.h"

#include <stdio.h>
#include <string.h>

/* ANCHOR: replay */
int main(void)
{
    uint8_t wire[3 + 3 + 36 + 1] = {SYNC, 3, 3 + 36, 7, 0, 9};
    for (int i = 0; i < 36; i++)
        wire[6 + i] = 0x41;
    wire[42] = crc8(wire + 1, 41);
    struct {
        struct command cmd;
        uint32_t canary;
    } frame = {.canary = 0xC0FFEE};
    int rc = parse_v1(wire, sizeof wire, &frame.cmd);
    printf("parse_v1: %d, точек %u, за таблицей %#x\n", rc, frame.cmd.n, frame.canary);
    return 0;
}
/* ANCHOR_END: replay */
