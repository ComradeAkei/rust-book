/* Раздел 8.3: опечатка в имени макроса молча выключает код. */
#include <stdio.h>

#define CONFIG_UART_DMA 1

int main(void) {
#ifdef CONFIG_UART_DAM
    puts("UART: приём через DMA");
#else
    puts("UART: приём опросом");
#endif
#if CONFIG_UART_DAM
    puts("UART: DMA включён");
#endif
    return 0;
}
