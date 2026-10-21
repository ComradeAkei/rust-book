/* Приложение A: целочисленное расширение (integer promotion). */
#include <stdint.h>
#include <stdio.h>

// ANCHOR: promote
uint32_t square(uint16_t x)
{
    return x * x; /* оба операнда сначала становятся int */
}
// ANCHOR_END: promote

int main(void)
{
    volatile uint16_t x = 65535;
    printf("square(65535) = %u\n", square(x));
    return 0;
}
