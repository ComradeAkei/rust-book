/* Раздел 31.5: функция на C с ошибкой на единицу — пишет len + 1 байт.
 * Её зовёт программа san (code/ch31/src/bin/san.rs). */
#include <stddef.h>

/* ANCHOR: fill */
void fill_counter(unsigned char *buf, size_t len)
{
    for (size_t i = 0; i <= len; i++)
        buf[i] = (unsigned char)i;
}
/* ANCHOR_END: fill */
