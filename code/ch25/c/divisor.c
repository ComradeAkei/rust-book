/* Глава 25: делитель UART PL011 препроцессором. Арифметика в #if — целая,
 * в intmax_t, и умеет только числа: ни sizeof, ни перечислений, ни
 * функций. */
#include <stdio.h>

#ifndef UART_CLK
#define UART_CLK 16000000u
#endif
#ifndef BAUD
#define BAUD 115200u
#endif

// ANCHOR: divisor
/* BAUDDIV · 64 с округлением к ближайшему. */
#define DIV64 ((UART_CLK * 4u + BAUD / 2u) / BAUD)
#if (DIV64 >> 6) == 0
#error "UART: делитель меньше 1, скорость недостижима при этой частоте"
#endif

static const unsigned ibrd = DIV64 >> 6;
static const unsigned fbrd = DIV64 & 63u;
// ANCHOR_END: divisor

int main(void) {
    printf("IBRD = %u, FBRD = %u\n", ibrd, fbrd);
    return 0;
}
