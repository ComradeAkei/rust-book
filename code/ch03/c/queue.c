/* Глава 3, введение: владение буфером, о котором знает только программист. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

struct frame {
    unsigned char *data;
    size_t len;
};

static struct frame *queue[8];
static int qlen;

/* Очередь становится владельцем кадра и освободит его в flush. */
static void enqueue(struct frame *f) { queue[qlen++] = f; }

static void flush(void)
{
    for (int i = 0; i < qlen; i++) {
        printf("send %zu bytes, first 0x%02x\n", queue[i]->len, queue[i]->data[0]);
        free(queue[i]->data);
        free(queue[i]);
    }
    qlen = 0;
}

static int produce(int seq, int link_up)
{
    struct frame *f = malloc(sizeof *f);
    if (!f)
        return -1;
    f->data = malloc(64);
    if (!f->data)
        goto err;
    f->len = 64;
    memset(f->data, seq, f->len);
    enqueue(f);       /* владение передано очереди... */
    if (!link_up)
        goto err;     /* ...но путь ошибки об этом не знает */
    return 0;
err:
    free(f->data);
    free(f);
    return -1;
}

int main(void)
{
    produce(0xA5, 1);
    produce(0x5A, 0); /* связь пропала */
    flush();
    return 0;
}
