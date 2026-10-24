/* Раздел 31.4: команды хоста для станции — тот же формат, что в
 * code/ch31/src/cmd.rs, и та же ошибка в разборе калибровки. */
#ifndef CMD_H
#define CMD_H

#include <stddef.h>
#include <stdint.h>

#define SYNC 0x5A
#define MAX_POINTS 8

struct point {
    int16_t raw, value;
};

/* ANCHOR: types */
struct command {
    uint8_t cmd; /* 1 ping, 2 период, 3 калибровка, 4 сброс */
    uint16_t sensor;
    uint16_t period_ms;
    uint8_t n;
    struct point points[MAX_POINTS];
};
/* ANCHOR_END: types */

enum cmd_error {
    CMD_OK = 0,
    CMD_SHORT = -1,
    CMD_BAD_SYNC = -2,
    CMD_BAD_CRC = -3,
    CMD_UNKNOWN = -4,
    CMD_BAD_LENGTH = -5,
    CMD_BAD_SENSOR = -6,
};

uint8_t crc8(const uint8_t *data, size_t len);
int parse_v1(const uint8_t *data, size_t len, struct command *out);

#endif
