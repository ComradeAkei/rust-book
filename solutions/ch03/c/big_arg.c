/* Задача 3.C, вопрос 2: структура в 4 КиБ по значению через AAPCS.
 * clang --target=thumbv7em-none-eabihf -mcpu=cortex-m4 -O2 -S -o - big_arg.c */
#include <stdint.h>

struct w1024 {
    uint32_t w[1024];
};

uint32_t arg_c_1024(struct w1024 s);

uint32_t call_c_1024(const struct w1024 *s)
{
    return arg_c_1024(*s);
}
