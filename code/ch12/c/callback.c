/* Глава 12, введение: обратный вызов в C — указатель на функцию и
 * void *ctx. Реестр обработчиков событий хранит по три указателя на
 * подписку: функцию, контекст и деструктор контекста. Режим bench меряет,
 * во что обходится вызов через указатель в каждом элементе массива.
 * Запуск: ./callback [mixup|session|heap|bench [повторов]]; без аргументов
 * по очереди выполняются mixup, session и heap. */
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

typedef void (*handler_fn)(void *ctx, int ev);

struct handler {
    handler_fn fn;
    void *ctx;
    void (*destroy)(void *ctx); /* NULL, если контекст не наш */
};

#define MAX_HANDLERS 8
static struct handler handlers[MAX_HANDLERS];
static int nhandlers;

static int subscribe(handler_fn fn, void *ctx, void (*destroy)(void *))
{
    if (nhandlers == MAX_HANDLERS)
        return -1;
    handlers[nhandlers++] = (struct handler){fn, ctx, destroy};
    return 0;
}

static void emit(int ev)
{
    for (int i = 0; i < nhandlers; i++)
        handlers[i].fn(handlers[i].ctx, ev);
}

static void unsubscribe_all(void)
{
    for (int i = 0; i < nhandlers; i++)
        if (handlers[i].destroy)
            handlers[i].destroy(handlers[i].ctx);
    nhandlers = 0;
}

struct counter {
    int n;
};

struct average {
    long sum;
    int n;
};

static void on_count(void *ctx, int ev)
{
    struct counter *c = ctx;
    (void)ev;
    c->n++;
}

static void on_average(void *ctx, int ev)
{
    struct average *a = ctx;
    a->sum += ev;
    a->n++;
}

static struct counter hits;
static struct average avg;

/* Контекст живёт в кадре стека, а подписка его переживает. noinline
 * оставляет функции свой кадр, как если бы она лежала в другом файле. */
__attribute__((noinline)) static void start_session(void)
{
    struct counter session = {0};
    subscribe(on_count, &session, NULL);
    emit(1);
    printf("в сессии: %d\n", session.n);
}

/* Библиотечная функция: что за f, она не знает, и noipa не даёт GCC
 * подсмотреть это из места вызова, как не подсмотрит он и в libc. */
#ifdef __clang__
#define NOIPA __attribute__((noinline))
#else
#define NOIPA __attribute__((noipa))
#endif
NOIPA void apply(int16_t *a, size_t n, int16_t (*f)(void *, int16_t), void *ctx)
{
    for (size_t i = 0; i < n; i++)
        a[i] = f(ctx, a[i]);
}

/* Калибровка отсчёта АЦП: x * k + b. */
struct affine {
    int16_t k, b;
};

static int16_t scale(void *ctx, int16_t x)
{
    const struct affine *p = ctx;
    return (int16_t)(x * p->k + p->b);
}

/* То же преобразование, но операция известна внутри цикла: так выглядел
 * бы apply, если бы для каждой f была своя копия. */
NOIPA void scale_all(int16_t *a, size_t n, const struct affine *p)
{
    const int16_t k = p->k, b = p->b; /* a[i] может указывать на *p */
    for (size_t i = 0; i < n; i++)
        a[i] = (int16_t)(a[i] * k + b);
}

static double now(void)
{
    struct timespec t;
    clock_gettime(CLOCK_MONOTONIC, &t);
    return t.tv_sec + t.tv_nsec * 1e-9;
}

static int bench(int reps)
{
    enum { N = 4096 };
    static int16_t a[N], b[N];
    struct affine p = {3, 1};
    for (int i = 0; i < N; i++)
        a[i] = b[i] = (int16_t)i;
    double t0 = now();
    for (int r = 0; r < reps; r++)
        apply(a, N, scale, &p);
    double t1 = now();
    for (int r = 0; r < reps; r++)
        scale_all(b, N, &p);
    double t2 = now();
    for (int i = 0; i < N; i++)
        if (a[i] != b[i])
            return printf("расхождение в элементе %d\n", i), 1;
    printf("через указатель: %.2f нс на элемент\n", (t1 - t0) * 1e9 / reps / N);
    printf("своя копия:       %.2f нс на элемент\n", (t2 - t1) * 1e9 / reps / N);
    return 0;
}

/* Обработчик и контекст разных типов: компилятор этого не видит. */
static void mixup(void)
{
    subscribe(on_count, &hits, NULL);
    subscribe(on_average, &hits, NULL); /* контекст не того типа */
    emit(10);
    emit(20);
    emit(30);
    printf("событий: %d (ожидалось 3)\n", hits.n);
    unsubscribe_all();
}

static void session(void)
{
    start_session();
    emit(2); /* пишет в кадр стека, которого уже нет */
    unsubscribe_all();
}

/* Контекст в куче: освободить его должен тот, кто отписывает. */
static void heap(void)
{
    struct average *a = malloc(sizeof *a);
    *a = (struct average){0, 0};
    subscribe(on_average, a, free);
    subscribe(on_average, &avg, NULL);
    emit(4);
    emit(8);
    printf("среднее: %ld\n", a->sum / a->n);
    unsubscribe_all(); /* free(a), а avg — не наш */
}

int main(int argc, char **argv)
{
    const char *mode = argc > 1 ? argv[1] : "";
    if (strcmp(mode, "bench") == 0)
        return bench(argc > 2 ? atoi(argv[2]) : 20000);
    if (!*mode)
        printf("sizeof(struct handler) = %zu\n", sizeof(struct handler));
    if (!*mode || strcmp(mode, "mixup") == 0)
        mixup();
    if (!*mode || strcmp(mode, "session") == 0)
        session();
    if (!*mode || strcmp(mode, "heap") == 0)
        heap();
    return 0;
}
