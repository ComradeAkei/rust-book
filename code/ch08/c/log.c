/* Раздел 8.7: журнал по ошибке назвал свой буфер так же. */
#include <stdio.h>

char rx_count[16];

void log_line(const char *s) { snprintf(rx_count, sizeof rx_count, "%s", s); }
