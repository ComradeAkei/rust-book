/* Раздел 31.7, «Как в C»: тот же разбор под CBMC — движком, на котором
 * стоит Kani. Значения nondet_* CBMC считает произвольными, как
 * kani::any(), а __CPROVER_assume — как kani::assume. С -D IN_FRAME
 * команда лежит внутри объемлющей структуры, как в replay.c, и запись за
 * таблицу CBMC уже не видит: он, как ASan, проверяет границы объектов, а
 * не полей. */
#include "cmd.h"

uint8_t nondet_u8(void);
size_t nondet_size(void);

/* ANCHOR: harness */
int main(void)
{
    uint8_t buf[43];
    for (size_t i = 0; i < sizeof buf; i++)
        buf[i] = nondet_u8();
    size_t len = nondet_size();
    __CPROVER_assume(len <= sizeof buf);
#ifndef IN_FRAME
    struct command c;
    parse_v1(buf, len, &c);
#else
    struct {
        struct command cmd;
        uint32_t canary;
    } frame;
    parse_v1(buf, len, &frame.cmd);
#endif
    return 0;
}
/* ANCHOR_END: harness */
