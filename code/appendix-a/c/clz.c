/* Приложение A: __builtin_clz(0). */
#include <stdio.h>

int clz(unsigned x)
{
    return __builtin_clz(x); /* для x == 0 результат не определён */
}

int main(void)
{
    volatile unsigned x = 0;
    printf("clz(0) = %d\n", clz(x));
    return 0;
}
