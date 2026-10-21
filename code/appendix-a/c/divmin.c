/* Приложение A: INT_MIN / -1. */
#include <limits.h>
#include <stdio.h>

// ANCHOR: divmin
int quot(int a, int b)
{
    return a / b; /* INT_MIN / -1 = 2^31, в int не помещается */
}
// ANCHOR_END: divmin

int main(void)
{
    volatile int a = INT_MIN, b = -1;
    printf("%d\n", quot(a, b));
    return 0;
}
