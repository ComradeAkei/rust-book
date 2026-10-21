/* Приложение A: компаратор для qsort через вычитание. */
#include <limits.h>
#include <stdio.h>
#include <stdlib.h>

// ANCHOR: cmp
int cmp_int(const void *a, const void *b)
{
    return *(const int *)a - *(const int *)b; /* переполнение int */
}
// ANCHOR_END: cmp

int main(void)
{
    int v[] = {INT_MAX, -1, INT_MIN, 0};
    qsort(v, 4, sizeof v[0], cmp_int);
    printf("%d %d %d %d\n", v[0], v[1], v[2], v[3]);
    return 0;
}
