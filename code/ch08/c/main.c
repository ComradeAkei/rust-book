#include <stdio.h>

extern int rx_count;
void uart_rx(void);
void log_line(const char *s);

int main(void) {
    uart_rx();
    uart_rx();
    uart_rx();
    printf("rx_count = %d\n", rx_count);
    log_line("AB");
    printf("rx_count = %d\n", rx_count);
    return 0;
}
