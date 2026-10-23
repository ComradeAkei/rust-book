/* Заголовок кольцевого буфера для C (задача C). Такой выдал бы cbindgen по
 * решению: RingBuffer — непрозрачный тип, C знает лишь, что это указатель, а
 * устройство структуры спрятано в Rust. */
#ifndef CH18_RING_BUFFER_H
#define CH18_RING_BUFFER_H

#include <stddef.h>

typedef struct RingBuffer RingBuffer;

RingBuffer *rb_new(size_t cap);
int rb_push(RingBuffer *h, int v);
int rb_pop(RingBuffer *h, int *out);
size_t rb_len(const RingBuffer *h);
void rb_free(RingBuffer *h);

#endif /* CH18_RING_BUFFER_H */
