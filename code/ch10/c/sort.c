/* Глава 10, введение: два способа написать обобщённый код на C.
 *
 * qsort обобщена через void * и указатель на функцию: одна копия кода на
 * все типы, сравнение — косвенный вызов, который нельзя встроить. Макрос
 * DEFINE_SORT порождает свою функцию для каждого типа, сравнение
 * встраивается, но типы проверяются только после подстановки. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#ifdef COUNT
static unsigned long calls; /* сколько раз qsort вызвала cmp_u32 */
#define COUNT_CALL() (calls++)
#else
#define COUNT_CALL() ((void)0)
#endif

static int cmp_u32(const void *a, const void *b)
{
    COUNT_CALL();
    uint32_t x = *(const uint32_t *)a, y = *(const uint32_t *)b;
    return (x > y) - (x < y);
}

static int cmp_u16(const void *a, const void *b)
{
    uint16_t x = *(const uint16_t *)a, y = *(const uint16_t *)b;
    return (x > y) - (x < y);
}

/* Быстрая сортировка для типа T с операцией LESS(a, b). */
#define DEFINE_SORT(T, LESS)                                         \
    static void sort_##T(T *v, size_t n)                             \
    {                                                                \
        while (n > 16) {                                             \
            T p = v[n / 2];                                          \
            size_t i = 0, j = n - 1;                                 \
            for (;;) {                                               \
                while (LESS(v[i], p))                                \
                    i++;                                             \
                while (LESS(p, v[j]))                                \
                    j--;                                             \
                if (i >= j)                                          \
                    break;                                           \
                T t = v[i];                                          \
                v[i] = v[j];                                         \
                v[j] = t;                                            \
                i++;                                                 \
                j--;                                                 \
            }                                                        \
            sort_##T(v, j + 1);                                      \
            v += j + 1;                                              \
            n -= j + 1;                                              \
        }                                                            \
        for (size_t i = 1; i < n; i++)                               \
            for (size_t j = i; j > 0 && LESS(v[j], v[j - 1]); j--) { \
                T t = v[j];                                          \
                v[j] = v[j - 1];                                     \
                v[j - 1] = t;                                        \
            }                                                        \
    }

#define LESS_NUM(a, b) ((a) < (b))
DEFINE_SORT(uint32_t, LESS_NUM)

#ifdef BAD_POINT
/* Структуру нельзя сравнить через <, но узнаем мы об этом только внутри
 * подставленного тела макроса. */
typedef struct {
    int x, y;
} point;
DEFINE_SORT(point, LESS_NUM)
#endif

static uint32_t seed = 12345;

static uint32_t next(void) /* xorshift32 */
{
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    return seed;
}

static double now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + t.tv_nsec * 1e-9;
}

static int sorted(const uint32_t *v, size_t n)
{
    for (size_t i = 1; i < n; i++)
        if (v[i - 1] > v[i])
            return 0;
    return 1;
}

int main(int argc, char **argv)
{
    size_t n = argc > 1 ? strtoul(argv[1], NULL, 10) : 1000000;
    uint32_t *a = malloc(n * sizeof *a), *b = malloc(n * sizeof *b);
    if (!a || !b)
        return 1;
    for (size_t i = 0; i < n; i++)
        a[i] = b[i] = next();

    double t0 = now();
    qsort(a, n, sizeof a[0], cmp_u32);
    double t1 = now();
    sort_uint32_t(b, n);
    double t2 = now();
    printf("qsort:         %6.1f мс, упорядочен: %d\n", (t1 - t0) * 1e3, sorted(a, n));
    printf("sort_uint32_t: %6.1f мс, упорядочен: %d\n", (t2 - t1) * 1e3, sorted(b, n));
#ifdef COUNT
    printf("вызовов cmp_u32: %lu\n", calls);
#endif

    /* Компаратор не того типа: void * примет любой. */
    for (size_t i = 0; i < n; i++)
        a[i] = next();
    qsort(a, n, sizeof a[0], cmp_u16);
    printf("qsort с cmp_u16: упорядочен: %d\n", sorted(a, n));
    free(a);
    free(b);
    return 0;
}
