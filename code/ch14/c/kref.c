/* Глава 14: конфигурация датчика, общая для нескольких подсистем.
 * Время жизни — счётчик ссылок вручную, как struct kref в ядре Linux:
 * config_get берёт ссылку, config_put отдаёт, последний освобождает.
 * Сборка: gcc -O2 -Wall -Wextra -pthread -o kref kref.c
 * Запуск: ./kref [notify|leak|double|race] — без аргумента первые три. */
#include <pthread.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct config {
    int refs;
    int period_ms;
    char name[16];
};

static struct config *config_new(const char *name, int period_ms)
{
    struct config *c = malloc(sizeof *c);
    if (!c)
        abort();
    c->refs = 1;
    c->period_ms = period_ms;
    snprintf(c->name, sizeof c->name, "%s", name);
    return c;
}

/* noinline: в гонке ниже инкремент и декремент остаются отдельными
 * чтением и записью, как у вызова из другого модуля. */
__attribute__((noinline)) static struct config *config_get(struct config *c)
{
    c->refs++;
    return c;
}

__attribute__((noinline)) static void config_put(struct config *c)
{
    if (--c->refs == 0)
        free(c);
}

/* Журнал держит ссылку, пока открыт. */
struct logger {
    struct config *cfg;
    FILE *out;
};

static int logger_open(struct logger *l, struct config *c, const char *path)
{
    l->cfg = config_get(c);
    l->out = fopen(path, "w");
    if (!l->out)
        return -1; /* ссылка взята и не отдана */
    return 0;
}

/* Отправитель забирает ссылку вызывающего: после sender_start тот не
 * должен звать config_put. Договор записан только в этом комментарии. */
struct sender {
    struct config *cfg;
};

static void sender_start(struct sender *s, struct config *c) { s->cfg = c; }
static void sender_stop(struct sender *s) { config_put(s->cfg); }

static void logger_close(struct logger *l)
{
    fclose(l->out);
    config_put(l->cfg);
}

/* Подписчики на изменение конфигурации. */
struct sub {
    void (*fn)(struct config *, void *);
    void *ctx;
    int calls;
};

static struct sub *subs;
static size_t nsubs, cap;

static void subscribe(void (*fn)(struct config *, void *), void *ctx)
{
    if (nsubs == cap) {
        cap = cap ? 2 * cap : 2;
        subs = realloc(subs, cap * sizeof *subs);
        if (!subs)
            abort();
    }
    subs[nsubs++] = (struct sub){fn, ctx, 0};
}

static void config_set_period(struct config *c, int ms)
{
    c->period_ms = ms;
    for (size_t i = 0; i < nsubs; i++) {
        struct sub *s = &subs[i];
        s->fn(c, s->ctx);
        s->calls++;
    }
}

static void on_change(struct config *c, void *ctx)
{
    (void)c;
    (void)ctx;
}

/* Сторож: если период вырос больше секунды, подписывает тревогу. */
static void watchdog(struct config *c, void *ctx)
{
    (void)ctx;
    if (c->period_ms > 1000)
        subscribe(on_change, NULL);
}

static void scenario_leak(void)
{
    struct config *c = config_new("adc0", 100);
    struct logger l;
    if (logger_open(&l, c, "/nonexistent/adc0.log") < 0)
        printf("журнал не открыт, ссылок: %d (ожидалось 1)\n", c->refs);
    else
        logger_close(&l);
    config_put(c);
}

static void scenario_double(void)
{
    struct config *c = config_new("adc0", 100);
    struct logger l = {config_get(c), stdout};
    struct sender s;
    sender_start(&s, c); /* ссылка ушла отправителю... */
    sender_stop(&s);
    config_put(c); /* ...и отдана ещё раз */
    printf("журнал: период %d мс\n", l.cfg->period_ms);
    config_put(l.cfg);
}

static void scenario_notify(void)
{
    subscribe(on_change, NULL);
    subscribe(watchdog, NULL);
    struct config *c = config_new("adc0", 100);
    config_set_period(c, 2000);
    printf("подписчиков: %zu, вызовов: %d %d %d (ожидалось 1 1 1)\n", nsubs,
           subs[0].calls, subs[1].calls, subs[2].calls);
    config_put(c);
}

static void *worker(void *arg)
{
    struct config *c = arg;
    for (int i = 0; i < 1000000; i++)
        config_put(config_get(c));
    config_put(c);
    return NULL;
}

static void scenario_race(void)
{
    struct config *c = config_new("adc0", 100);
    struct config *refs[2] = {config_get(c), config_get(c)}; /* по одной на поток */
    pthread_t t[2];
    for (int i = 0; i < 2; i++)
        pthread_create(&t[i], NULL, worker, refs[i]);
    for (int i = 0; i < 2; i++)
        pthread_join(t[i], NULL);
    printf("потоки: ссылок %d (ожидалось 1)\n", c->refs);
    config_put(c);
}

int main(int argc, char **argv)
{
    const char *mode = argc > 1 ? argv[1] : "";
    if (!*mode || !strcmp(mode, "notify"))
        scenario_notify();
    if (!*mode || !strcmp(mode, "leak"))
        scenario_leak();
    if (!*mode || !strcmp(mode, "double"))
        scenario_double();
    if (!strcmp(mode, "race"))
        scenario_race();
    return 0;
}
