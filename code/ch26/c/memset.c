/* Раздел 26.5: memset циклом. GCC при -O2 узнаёт в цикле memset и
 * вызывает её — то есть саму себя. run.sh проверяет, при каких ключах. */
#include <stddef.h>

void *memset(void *d, int c, size_t n) {
    unsigned char *p = d;
    while (n--) {
        *p++ = (unsigned char)c;
    }
    return d;
}
