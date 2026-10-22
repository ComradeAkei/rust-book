/* Задача 1.A на C: те же двенадцать приведений, сначала из констант
 * (их сворачивает компилятор), потом из volatile (их считает процессор).
 * Запуск и сводная таблица: bash solutions/ch01/c/run.sh. */
#include <math.h>
#include <stdint.h>
#include <stdio.h>

/* gcc принимает 'я' как многосимвольную константу, clang отказывается:
 * для него собираем с -DYA="u'я'" (char16_t, C11). */
#ifndef YA
#define YA 'я'
#endif

#define ROW(n, expr) printf("%2d %-34s %lld\n", n, #expr, (long long)(expr))
/* Строка 10 беззнаковая: её значение может не поместиться в long long. */
#define UROW(n, expr) printf("%2d %-34s %llu\n", n, #expr, (unsigned long long)(expr))

static void folded(void)
{
    puts("константы:");
    ROW(1, (uint8_t)300);
    ROW(2, (uint32_t)(int8_t)-1);
    ROW(3, (int32_t)NAN);
    ROW(4, (uint8_t)1e10);
    ROW(5, (uint8_t)-1.9);
    ROW(6, (int8_t)(uint8_t)128);
    ROW(7, (int32_t)(uint8_t)(int8_t)-1);
    ROW(8, (int32_t)(int8_t)(uint8_t)255);
    ROW(9, (int32_t)(float)16777217);
    UROW(10, (uint64_t)(float)UINT64_MAX);
    ROW(11, (uint8_t)YA);
    ROW(12, (uint8_t)(0.1f + 0.2f == 0.3f));
}

static void runtime(void)
{
    volatile int32_t i300 = 300, i16m = 16777217;
    volatile int8_t im1 = -1;
    volatile uint8_t u128 = 128, u255 = 255;
    volatile double dnan = NAN, d1e10 = 1e10, dm19 = -1.9;
    volatile uint64_t umax = UINT64_MAX;
    volatile float f01 = 0.1f, f02 = 0.2f, f03 = 0.3f;
    volatile int ya = YA;
    puts("во время выполнения:");
    ROW(1, (uint8_t)i300);
    ROW(2, (uint32_t)im1);
    ROW(3, (int32_t)dnan);
    ROW(4, (uint8_t)d1e10);
    ROW(5, (uint8_t)dm19);
    ROW(6, (int8_t)u128);
    ROW(7, (int32_t)(uint8_t)im1);
    ROW(8, (int32_t)(int8_t)u255);
    ROW(9, (int32_t)(float)i16m);
    UROW(10, (uint64_t)(float)umax);
    ROW(11, (uint8_t)ya);
    ROW(12, (uint8_t)(f01 + f02 == f03));
}

int main(void)
{
    folded();
    runtime();
    return 0;
}
