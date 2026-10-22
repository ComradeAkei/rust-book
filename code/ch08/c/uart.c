/* Раздел 8.7: драйвер UART считает принятые байты. */
int rx_count;

void uart_rx(void) { rx_count++; }
