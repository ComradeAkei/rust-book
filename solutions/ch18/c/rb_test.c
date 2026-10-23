/* Драйвер задачи C: C управляет кольцевым буфером из Rust через непрозрачный
 * описатель. Проверяет границу ёмкости и порядок FIFO; под ASan/LeakSanitizer
 * забытый rb_free всплыл бы утечкой. */
#include "ring_buffer.h"
#include <stdio.h>

int main(void) {
    RingBuffer *rb = rb_new(3);
    for (int i = 1; i <= 3; i++) {
        if (!rb_push(rb, i)) {
            fprintf(stderr, "push %d неожиданно отвергнут\n", i);
            return 1;
        }
    }
    if (rb_push(rb, 4)) {
        fprintf(stderr, "push в полный буфер должен вернуть 0\n");
        return 1;
    }
    if (rb_len(rb) != 3) {
        fprintf(stderr, "ожидалось len == 3\n");
        return 1;
    }

    int out = 0, sum = 0, n = 0;
    while (rb_pop(rb, &out)) {
        sum += out;
        n++;
    }
    rb_free(rb);

    if (n != 3 || sum != 6) {
        fprintf(stderr, "ожидалось 3 элемента суммой 6, получено n=%d sum=%d\n", n, sum);
        return 1;
    }
    printf("кольцевой буфер из Rust: 3 элемента, сумма %d", sum);
    return 0;
}
