/* Приложение A: exit() сбрасывает буферы stdio. */
#include <stdio.h>
#include <stdlib.h>

// ANCHOR: exit
int main(void)
{
    printf("строка в буфере stdio\n");
    exit(0); /* atexit-обработчики libc сбросят буфер */
}
// ANCHOR_END: exit
