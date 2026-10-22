/* Задача 8.C: программа на C, которая вызывает библиотеку на Rust. */
#include <stdio.h>
#include <string.h>

#include "station.h"

int main(void)
{
    const char *check = "123456789";
    printf("crc8(\"%s\") = 0x%02x\n", check,
           station_crc8((const uint8_t *)check, strlen(check)));
    printf("crc8(пусто) = 0x%02x\n", station_crc8(NULL, 0));

    static const uint16_t ids[] = {1, 7, 12, 3};
    for (size_t i = 0; i < sizeof ids / sizeof ids[0]; i++) {
        int32_t v;
        if (station_scale(ids[i], 1000, &v) == 0)
            printf("датчик %u: 1000 -> %d\n", ids[i], v);
        else
            printf("датчик %u: нет в таблице\n", ids[i]);
    }
    return 0;
}
