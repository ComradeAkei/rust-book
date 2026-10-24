/* Раздел 31.4: разбор команды на C. */
#include "cmd.h"

#include <string.h>

uint8_t crc8(const uint8_t *data, size_t len)
{
    uint8_t crc = 0;
    for (size_t i = 0; i < len; i++) {
        crc ^= data[i];
        for (int bit = 0; bit < 8; bit++)
            crc = crc & 0x80 ? (uint8_t)(crc << 1) ^ 0x07 : (uint8_t)(crc << 1);
    }
    return crc;
}

/* ANCHOR: calibrate_v1 */
static int calibrate_v1(const uint8_t *bytes, size_t len, uint8_t n, struct command *out)
{
    if (len != 4u * n)
        return CMD_BAD_LENGTH;
    out->n = n;
    for (size_t i = 0; i < n; i++) {
        memcpy(&out->points[i].raw, bytes + 4 * i, 2);
        memcpy(&out->points[i].value, bytes + 4 * i + 2, 2);
    }
    return CMD_OK;
}
/* ANCHOR_END: calibrate_v1 */

int parse_v1(const uint8_t *data, size_t len, struct command *out)
{
    if (len < 3)
        return CMD_SHORT;
    if (data[0] != SYNC)
        return CMD_BAD_SYNC;
    size_t plen = data[2];
    if (len < 4 + plen)
        return CMD_SHORT;
    if (data[3 + plen] != crc8(data + 1, 2 + plen))
        return CMD_BAD_CRC;
    const uint8_t *p = data + 3;
    out->cmd = data[1];
    switch (data[1]) {
    case 1:
    case 4:
        return plen == 0 ? CMD_OK : CMD_BAD_LENGTH;
    case 2:
        if (plen != 4)
            return CMD_BAD_LENGTH;
        out->sensor = (uint16_t)(p[0] | p[1] << 8);
        out->period_ms = (uint16_t)(p[2] | p[3] << 8);
        return out->sensor ? CMD_OK : CMD_BAD_SENSOR;
    case 3:
        if (plen < 3)
            return CMD_BAD_LENGTH;
        out->sensor = (uint16_t)(p[0] | p[1] << 8);
        if (!out->sensor)
            return CMD_BAD_SENSOR;
        return calibrate_v1(p + 3, plen - 3, p[2], out);
    default:
        return CMD_UNKNOWN;
    }
}
