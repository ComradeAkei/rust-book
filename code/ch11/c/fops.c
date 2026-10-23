/* Глава 11, введение: таблица методов по образцу struct file_operations из
 * ядра Linux. Указатель на таблицу лежит в каждом объекте, а объект
 * драйвера получают из общей части через container_of. Режим bench меряет
 * косвенный вызов: устройства по порядку типов и вперемешку.
 * Запуск: ./fops [bench [sorted|shuffled [повторов]]]. */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#define container_of(ptr, type, member) \
    ((type *)((char *)(ptr) - offsetof(type, member)))

struct dev;

struct dev_ops {
    int (*read)(struct dev *d);
    int (*write)(struct dev *d, int v); /* необязательный */
    void (*release)(struct dev *d);
};

/* Общая часть каждого устройства. */
struct dev {
    const struct dev_ops *ops;
    const char *name;
};

struct thermo {
    int16_t raw; /* шестнадцатые доли градуса */
    struct dev dev;
};

struct counter {
    struct dev dev;
    uint32_t pulses;
};

static int thermo_read(struct dev *d)
{
    return container_of(d, struct thermo, dev)->raw * 100 / 16;
}

static int counter_read(struct dev *d)
{
    return (int)container_of(d, struct counter, dev)->pulses;
}

static int counter_write(struct dev *d, int v)
{
    container_of(d, struct counter, dev)->pulses = (uint32_t)v;
    return 0;
}

static void thermo_release(struct dev *d)
{
    free(container_of(d, struct thermo, dev));
}

static void counter_release(struct dev *d)
{
    free(container_of(d, struct counter, dev));
}

static const struct dev_ops thermo_ops = {
    .read = thermo_read,
    .release = thermo_release,
};
static const struct dev_ops counter_ops = {
    .read = counter_read,
    .write = counter_write,
    .release = counter_release,
};

static struct dev *new_dev(int kind, int v)
{
    if (kind == 0) {
        struct thermo *t = malloc(sizeof *t);
        *t = (struct thermo){(int16_t)v, {&thermo_ops, "thermo"}};
        return &t->dev;
    }
    struct counter *c = malloc(sizeof *c);
    *c = (struct counter){{&counter_ops, "counter"}, (uint32_t)v};
    return &c->dev;
}

/* Как vfs_write: необязательный метод проверяет каждый вызывающий. */
static int dev_write(struct dev *d, int v)
{
    if (!d->ops->write)
        return -22; /* -EINVAL */
    return d->ops->write(d, v);
}

__attribute__((noinline)) static long poll(struct dev **v, size_t n, int reps)
{
    long sum = 0;
    for (int r = 0; r < reps; r++)
        for (size_t i = 0; i < n; i++)
            sum += v[i]->ops->read(v[i]);
    return sum;
}

static double now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return (double)t.tv_sec + (double)t.tv_nsec * 1e-9;
}

int main(int argc, char **argv)
{
    enum { N = 1000 };
    struct dev *v[N];
    int shuffled = argc > 2 && strcmp(argv[2], "shuffled") == 0;
    int reps = argc > 3 ? atoi(argv[3]) : 100000;
    for (int i = 0; i < N; i++)
        v[i] = new_dev(i >= N / 2, i);
    /* Те же объекты в другом порядке: перестановка Фишера — Йетса. */
    for (unsigned i = N - 1, x = 1; shuffled && i > 0; i--) {
        x = x * 1103515245 + 12345;
        unsigned j = (x >> 16) % (i + 1);
        struct dev *t = v[i];
        v[i] = v[j];
        v[j] = t;
    }
    if (argc > 1 && strcmp(argv[1], "bench") == 0) {
        double t0 = now();
        long sum = poll(v, N, reps);
        printf("%s: %.2f нс на вызов (сумма %ld)\n", shuffled ? "вперемешку" : "по порядку",
               (now() - t0) * 1e9 / ((double)N * reps), sum);
    } else {
        printf("sizeof(struct thermo) = %zu, sizeof(struct counter) = %zu\n",
               sizeof(struct thermo), sizeof(struct counter));
        printf("%s: %d, %s: %d\n", v[1]->name, v[1]->ops->read(v[1]),
               v[N - 1]->name, v[N - 1]->ops->read(v[N - 1]));
        printf("write: %d, %d\n", dev_write(v[1], 5), dev_write(v[N - 1], 5));
    }
    for (int i = 0; i < N; i++)
        v[i]->ops->release(v[i]);
    return 0;
}
