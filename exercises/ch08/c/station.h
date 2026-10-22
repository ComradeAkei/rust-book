/* Задача 8.C: функции станции, реализованные на Rust в station/. */
#ifndef STATION_H
#define STATION_H

#include <stddef.h>
#include <stdint.h>

/* CRC-8 (полином 0x07, начальное значение 0) над len байтами data.
 * data может быть NULL, только если len == 0. */
uint8_t station_crc8(const uint8_t *data, size_t len);

/* Отсчёт raw датчика id в единицах измерения. Возвращает 0 и пишет
 * результат в *out; для неизвестного датчика возвращает -1, *out не
 * трогает. */
int station_scale(uint16_t id, int32_t raw, int32_t *out);

#endif
