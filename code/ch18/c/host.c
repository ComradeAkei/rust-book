/* Раздел 18.4, обратная сторона: C вызывает Rust. Создаём непрозрачный
 * счётчик, дёргаем его и освобождаем — всё через экспортированные Rust
 * функции с C-ABI. Если забыть ch18_counter_free, LeakSanitizer увидит
 * утечку Box под Rust. */
#include <stdint.h>
#include <stdio.h>

extern void *ch18_counter_new(void);
extern uint64_t ch18_counter_bump(void *h);
extern void ch18_counter_free(void *h);

int main(void) {
    void *c = ch18_counter_new();
    uint64_t last = 0;
    for (int i = 0; i < 5; i++) {
        last = ch18_counter_bump(c);
    }
    ch18_counter_free(c);
    if (last != 5) {
        fprintf(stderr, "ожидалось 5, получено %llu\n", (unsigned long long)last);
        return 1;
    }
    printf("счётчик из Rust: 5 нажатий = %llu", (unsigned long long)last);
    return 0;
}
