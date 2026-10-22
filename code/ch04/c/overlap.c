/* Раздел 4.5: счётчик внутри данных. Для count_flags это законно, для
 * count_flags_restrict — неопределённое поведение. */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

void count_flags(uint32_t *cnt, const uint8_t *data, size_t n);
void count_flags_restrict(uint32_t *restrict cnt, const uint8_t *data, size_t n);

int main(void)
{
    uint32_t a[2] = {0x7E, 0x7D}, b[2] = {0x7E, 0x7D};
    count_flags(&a[1], (const uint8_t *)a, 8);
    count_flags_restrict(&b[1], (const uint8_t *)b, 8);   /* нарушает restrict */
    printf("без restrict %#x, с restrict %#x\n", (unsigned)a[1], (unsigned)b[1]);
    return 0;
}
