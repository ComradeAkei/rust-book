/* Раздел 1.2: сдвиг на ширину типа и деление INT_MIN / -1 в C — UB, и
 * процессоры отвечают на них по-разному. На x86_64 деление здесь не
 * выполняется: idiv на таких входах бросает исключение #DE, и программа
 * умирает от SIGFPE (это показывает приложение A). */
#include <limits.h>
#include <stdint.h>
#include <stdio.h>

int main(void)
{
    volatile uint32_t one = 1, n = 32;
    printf("1u << 32 = %u\n", one << n);
#ifdef __arm__
    volatile int32_t min = INT_MIN, m1 = -1, zero = 0, seven = 7;
    printf("INT_MIN / -1 = %d\n", min / m1);
    printf("7 / 0 = %d\n", seven / zero);
#endif
    return 0;
}
