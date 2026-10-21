/* Приложение A: double -> int вне диапазона. */
#include <stdio.h>

int to_int(double d)
{
    return (int)d;
}

int main(void)
{
    volatile double d = 3e9;
    printf("(int)3e9 = %d\n", to_int(d));
    return 0;
}
