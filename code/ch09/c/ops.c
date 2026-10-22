/* Глава 9, введение: полиморфизм в C — таблица указателей на функции и
 * контекст через void *. Компилятор не проверяет ни то, что таблица
 * заполнена, ни то, что контекст того типа, который ждут функции. */
#include <stdint.h>
#include <stdio.h>

struct sensor_ops {
    int32_t (*read)(void *ctx);
    const char *(*name)(void *ctx);
};

struct sensor {
    const struct sensor_ops *ops;
    void *ctx;
};

struct thermo {
    int32_t raw;
};

struct counter {
    uint32_t total, last;
};

static int32_t thermo_read(void *ctx)
{
    return ((struct thermo *)ctx)->raw;
}

static const char *thermo_name(void *ctx)
{
    (void)ctx;
    return "термодатчик";
}

static int32_t counter_read(void *ctx)
{
    struct counter *c = ctx;
    int32_t d = (int32_t)(c->total - c->last);
    c->last = c->total;
    return d;
}

static const struct sensor_ops thermo_ops = {thermo_read, thermo_name};
/* name забыли: поле молча стало NULL. */
static const struct sensor_ops counter_ops = {.read = counter_read};

__attribute__((noinline)) int32_t poll_twice(struct sensor *s)
{
    return s->ops->read(s->ctx) + s->ops->read(s->ctx);
}

int main(void)
{
    setvbuf(stdout, NULL, _IOLBF, 0);
    struct thermo t = {2150};
    struct counter c = {10, 4};
    struct sensor sensors[] = {
        {&thermo_ops, &t},
        {&counter_ops, &c},
        {&thermo_ops, &c}, /* перепутан контекст */
    };
    for (int i = 0; i < 3; i++)
        printf("%d: poll_twice = %d\n", i, poll_twice(&sensors[i]));
    for (int i = 0; i < 3; i++)
        printf("%d: %s\n", i, sensors[i].ops->name(sensors[i].ctx));
    return 0;
}
