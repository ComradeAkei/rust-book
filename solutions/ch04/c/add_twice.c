/* Задача 4.B: add_twice на C без restrict и с ним. */

void add_twice(int *a, const int *b)
{
    *a += *b;
    *a += *b;
}

void add_twice_restrict(int *restrict a, const int *restrict b)
{
    *a += *b;
    *a += *b;
}
