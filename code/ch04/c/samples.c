/* Указатель в буфер переживает realloc. */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

struct samples {
    uint16_t *data;
    size_t len, cap;
};

static void push(struct samples *s, uint16_t v)
{
    if (s->len == s->cap) {
        s->cap = s->cap ? 2 * s->cap : 4;
        uint16_t *p = realloc(s->data, s->cap * sizeof *s->data);
        if (!p)
            abort();
        s->data = p;    /* старый блок освобождён, если данные переехали */
    }
    s->data[s->len++] = v;
}

int main(void)
{
    struct samples temp = {0}, pres = {0};      /* два канала датчика */
    for (uint16_t i = 0; i < 8; i++) {
        push(&temp, 1000 + i);
        push(&pres, 2000 + i);
    }

    const uint16_t *peak = &temp.data[0];       /* указатель на максимум */
    for (size_t i = 1; i < temp.len; i++)
        if (temp.data[i] > *peak)
            peak = &temp.data[i];

    push(&temp, 1500);                          /* девятый отсчёт: буфер растёт */
    printf("максимум %u\n", (unsigned)*peak);   /* peak указывает в старый буфер */
    free(temp.data);
    free(pres.data);
    return 0;
}
