/* Раздел 1.7: float в целое вне диапазона — неопределённое поведение.
 * Без санитайзера ответ зависит от того, какую инструкцию выбрал
 * компилятор: cvttss2si на x86_64 или vcvt на Cortex-M4F. */
#include <math.h>
#include <stdint.h>
#include <stdio.h>

int main(void)
{
    volatile float big = 3e9f, nan = NAN, neg = -5.7f;
    printf("(int32_t)3e9f = %d\n", (int32_t)big);
    printf("(int32_t)NAN = %d\n", (int32_t)nan);
    printf("(uint32_t)-5.7f = %u\n", (uint32_t)neg);
    return 0;
}
