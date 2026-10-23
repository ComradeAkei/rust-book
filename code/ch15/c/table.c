/* Глава 15: реестр датчиков — хеш-таблица, какую пишут в каждом проекте на C.
 * Открытая адресация с линейным пробированием, хеш из K&R (раздел 6.6),
 * рост вдвое при заполнении на три четверти.
 * Сборка: gcc -O2 -Wall -Wextra -o table table.c
 * Запуск: ./table [dangle|delete|collide] — без аргумента все три. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

struct entry {
    char name[32]; /* пустая строка — свободная ячейка */
    uint32_t count;
    int32_t last;
};

struct table {
    struct entry *slots;
    size_t cap, len;   /* cap — степень двойки */
    unsigned long probes; /* сколько ячеек просмотрено за всё время */
};

static unsigned hash(const char *s)
{
    unsigned h = 0;
    for (; *s; s++)
        h = *s + 31 * h;
    return h;
}

static struct entry *find_slot(struct table *t, const char *name)
{
    size_t i = hash(name) & (t->cap - 1);
    for (;;) {
        struct entry *e = &t->slots[i];
        t->probes++;
        if (e->name[0] == '\0' || strcmp(e->name, name) == 0)
            return e;
        i = (i + 1) & (t->cap - 1);
    }
}

static void table_init(struct table *t, size_t cap)
{
    t->slots = calloc(cap, sizeof *t->slots);
    t->cap = cap;
    t->len = 0;
    t->probes = 0;
}

static void grow(struct table *t)
{
    struct table bigger;
    table_init(&bigger, t->cap * 2);
    for (size_t i = 0; i < t->cap; i++)
        if (t->slots[i].name[0] != '\0')
            *find_slot(&bigger, t->slots[i].name) = t->slots[i];
    bigger.len = t->len;
    bigger.probes = t->probes;
    free(t->slots);
    *t = bigger;
}

struct entry *table_get(struct table *t, const char *name)
{
    struct entry *e = find_slot(t, name);
    return e->name[0] != '\0' ? e : NULL;
}

struct entry *table_put(struct table *t, const char *name)
{
    if ((t->len + 1) * 4 > t->cap * 3)
        grow(t);
    struct entry *e = find_slot(t, name);
    if (e->name[0] == '\0') {
        snprintf(e->name, sizeof e->name, "%s", name);
        t->len++;
    }
    return e;
}

void table_del(struct table *t, const char *name)
{
    struct entry *e = table_get(t, name);
    if (e != NULL) {
        memset(e, 0, sizeof *e);
        t->len--;
    }
}

static void scenario_dangle(void)
{
    struct table t;
    table_init(&t, 8);
    struct entry *adc = table_put(&t, "adc0");
    char name[32];
    for (int i = 0; i < 16; i++) { /* подключились ещё датчики */
        snprintf(name, sizeof name, "i2c%d", i);
        table_put(&t, name);
    }
    adc->count++; /* пришло измерение adc0 */
    adc->last = 1250;
    printf("adc0: измерений %u (ожидалось 1)\n", table_get(&t, "adc0")->count);
    free(t.slots);
}

static void scenario_delete(void)
{
    struct table t;
    table_init(&t, 8);
    table_put(&t, "Aa")->count = 1;
    table_put(&t, "BB")->count = 2;
    table_del(&t, "Aa"); /* датчик Aa отключён */
    struct entry *bb = table_get(&t, "BB");
    printf("BB: %s\n", bb != NULL ? "найден" : "не найден");
    free(t.slots);
}

/* Имя из k блоков «Aa» и «BB»: биты числа i выбирают блок. У всех 2^k имён
 * одинаковой длины один и тот же хеш: hash("Aa") == hash("BB"). */
static void colliding_name(char *out, unsigned i, int k)
{
    for (int b = 0; b < k; b++)
        memcpy(out + 2 * b, (i >> b) & 1 ? "BB" : "Aa", 2);
    out[2 * k] = '\0';
}

static double fill(struct table *t, int n, int k)
{
    char name[32];
    struct timespec a, b;
    clock_gettime(CLOCK_MONOTONIC, &a);
    for (int i = 0; i < n; i++) {
        if (k > 0)
            colliding_name(name, (unsigned)i, k);
        else
            snprintf(name, sizeof name, "sensor%05d", i);
        table_put(t, name);
    }
    clock_gettime(CLOCK_MONOTONIC, &b);
    return (b.tv_sec - a.tv_sec) * 1e3 + (b.tv_nsec - a.tv_nsec) / 1e6;
}

static void scenario_collide(void)
{
    const int k = 13, n = 1 << k;
    struct table t;
    table_init(&t, 8);
    double ms = fill(&t, n, 0);
    printf("обычные имена:  %d штук, %.1f ячеек на вставку, %.2f мс\n", n,
           (double)t.probes / n, ms);
    free(t.slots);
    table_init(&t, 8);
    ms = fill(&t, n, k);
    printf("имена из Aa/BB: %d штук, %.1f ячеек на вставку, %.2f мс\n", n,
           (double)t.probes / n, ms);
    free(t.slots);
}

int main(int argc, char **argv)
{
    const char *which = argc > 1 ? argv[1] : "";
    if (!*which || !strcmp(which, "dangle"))
        scenario_dangle();
    if (!*which || !strcmp(which, "delete"))
        scenario_delete();
    if (!*which || !strcmp(which, "collide"))
        scenario_collide();
    return 0;
}
