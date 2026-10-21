/* Приложение A: сдвиг на ширину типа. */
#include <stdint.h>
#include <stdio.h>

uint32_t shl(uint32_t x, unsigned n)
{
    return x << n;
}

int main(void)
{
    volatile unsigned n = 32;
    printf("1 << 32 = %u\n", shl(1, n));
    return 0;
}
