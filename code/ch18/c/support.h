/* Прототипы вспомогательной C-библиотеки главы 18. Rust объявляет те же
 * функции в блоках `unsafe extern "C"`; совпадение сигнатур — на
 * ответственности программиста (глава 18, задача A). */
#ifndef CH18_SUPPORT_H
#define CH18_SUPPORT_H

#include <stddef.h>
#include <stdint.h>

/* 18.1: простейший вызов C из Rust. */
int c_add(int a, int b);

/* 18.2: структура по значению в обе стороны. */
struct CPoint {
    int32_t x;
    int32_t y;
};
int32_t c_point_sum(struct CPoint p);
struct CPoint c_point_new(int32_t x, int32_t y);

/* 18.3: строки. c_name_len — это strlen; c_unit_name возвращает статическую
 * строку, причём единица «температура» намеренно в Latin-1 (не UTF-8). */
size_t c_name_len(const char *s);
const char *c_unit_name(int kind);

/* 18.4: C лишь хранит и возвращает чужой указатель, не трогая его. */
void *c_hold(void *p);

/* 18.5: обратные вызовы. c_apply вызывает f один раз; c_fold — по разу на
 * элемент, прокидывая ctx. */
int c_apply(int (*f)(int, void *), int x, void *ctx);
int c_fold(const int *arr, size_t n, int (*f)(int, int, void *), int init, void *ctx);

#endif /* CH18_SUPPORT_H */
