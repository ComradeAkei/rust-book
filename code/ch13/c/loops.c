/* Глава 13, введение: четыре цикла по индексам из обработки отсчётов АЦП
 * и четыре классические ошибки в них. Компиляторы предупреждают только об
 * одной; две находит ASan, а последнюю не находит никто.
 * Запуск: ./loops [short|last|deltas]; без аргументов печатает результаты
 * на нормальных данных, с аргументом выполняет вызов с ошибкой. */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

/* Суммы по скользящему окну ширины w: их n - w + 1. */
size_t window_sums(const int16_t *x, size_t n, size_t w, int32_t *out)
{
    size_t k = 0;
    for (size_t i = 0; i <= n - w; i++) { /* n < w: n - w = SIZE_MAX */
        int32_t s = 0;
        for (size_t j = 0; j < w; j++)
            s += x[i + j];
        out[k++] = s;
    }
    return k;
}

/* Индекс последнего отсчёта выше порога или -1, если такого нет. */
long last_above(const int16_t *x, size_t n, int16_t lim)
{
    for (size_t i = n - 1; i >= 0; i--) /* i >= 0 истинно всегда */
        if (x[i] > lim)
            return (long)i;
    return -1;
}

/* Разности соседних отсчётов: их n - 1. */
void deltas(const int16_t *x, size_t n, int16_t *d)
{
    for (size_t i = 0; i < n; i++) /* при i = n - 1 читается x[n] */
        d[i] = (int16_t)(x[i + 1] - x[i]);
}

/* Убирает выбросы на месте и возвращает новую длину. */
size_t drop_spikes(int16_t *x, size_t n, int16_t lim)
{
    for (size_t i = 0; i < n; i++)
        if (x[i] > lim || x[i] < -lim) {
            memmove(&x[i], &x[i + 1], (n - i - 1) * sizeof x[0]);
            n--; /* а i++ перешагнёт через сдвинутый отсчёт */
        }
    return n;
}

static void print(const char *what, const int16_t *x, size_t n)
{
    printf("%s:", what);
    for (size_t i = 0; i < n; i++)
        printf(" %d", x[i]);
    printf(" (n = %zu)\n", n);
}

int main(int argc, char **argv)
{
    int16_t adc[6] = {10, 12, 900, 950, 11, 13};
    int32_t sums[8];
    int16_t d[6];
    const char *mode = argc > 1 ? argv[1] : "";

    if (strcmp(mode, "short") == 0) {
        /* Два отсчёта, окно из трёх. */
        printf("окон: %zu\n", window_sums(adc, 2, 3, sums));
        return 0;
    }
    if (strcmp(mode, "last") == 0) {
        /* Ни одного отсчёта выше 1000. */
        printf("последний выше 1000: %ld\n", last_above(adc, 6, 1000));
        return 0;
    }
    if (strcmp(mode, "deltas") == 0) {
        deltas(adc, 6, d);
        print("разности", d, 5);
        return 0;
    }

    size_t k = window_sums(adc, 6, 3, sums);
    printf("окон по 3: %zu, первая сумма %d, последняя %d\n", k, sums[0], sums[k - 1]);
    printf("последний выше 500: %ld\n", last_above(adc, 6, 500));
    print("отсчёты", adc, 6);
    int16_t copy[6];
    memcpy(copy, adc, sizeof adc);
    print("без выбросов", copy, drop_spikes(copy, 6, 500));
    return 0;
}
