/* Раздел 1.1: integer promotion. Операнды uint8_t и uint16_t до
 * арифметики превращаются в int, и это меняет смысл трёх выражений. */
#include <stdint.h>
#include <stdio.h>

int main(void)
{
    volatile uint8_t a = 200, b = 100;
    if (a + b > 255) /* сумма в int: 300, а не 44 */
        puts("a + b > 255");

    volatile uint16_t x = 0xFFFF;
    uint32_t sq = x * x; /* int * int: 4294836225 не помещается в int, UB */
    printf("x * x = %u\n", sq);

    uint8_t m = 0x0F;
    if (~m == 0xF0) /* ~ над int: 0xFFFFFFF0 */
        puts("~m == 0xF0");
    else
        printf("~m = %#x\n", ~m);
    return 0;
}
