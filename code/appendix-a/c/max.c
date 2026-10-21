/* Приложение A: макрос вычисляет аргумент дважды. */
#include <stdio.h>

// ANCHOR: max
#define MAX(a, b) ((a) > (b) ? (a) : (b))

int main(void)
{
    int i = 5, j = 3;
    int m = MAX(i++, j); /* i++ подставится в текст два раза */
    printf("m = %d, i = %d\n", m, i);
    return 0;
}
// ANCHOR_END: max
