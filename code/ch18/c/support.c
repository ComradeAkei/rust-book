/* Вспомогательная C-библиотека главы 18: маленькие функции, которые Rust
 * вызывает через FFI. Компилируется build.rs в статический архив. */
#include "support.h"
#include <string.h>

int c_add(int a, int b) {
    return a + b;
}

int32_t c_point_sum(struct CPoint p) {
    return p.x + p.y;
}

struct CPoint c_point_new(int32_t x, int32_t y) {
    struct CPoint p = {x, y};
    return p;
}

size_t c_name_len(const char *s) {
    return strlen(s);
}

const char *c_unit_name(int kind) {
    switch (kind) {
    case 0:
        return "mV";
    /* «температура»: байт 0xB0 — это «°» в Latin-1, но не корректный UTF-8.
     * На этом строится пример про CStr::to_str() в разделе 18.3. */
    case 1:
        return "\xb0" "C";
    case 2:
        return "kPa";
    default:
        return "n/a";
    }
}

void *c_hold(void *p) {
    /* Храним и отдаём тот же указатель, не разыменовывая и не освобождая:
     * владение остаётся за Rust. */
    return p;
}

int c_apply(int (*f)(int, void *), int x, void *ctx) {
    return f(x, ctx);
}

int c_fold(const int *arr, size_t n, int (*f)(int, int, void *), int init, void *ctx) {
    int acc = init;
    for (size_t i = 0; i < n; i++) {
        acc = f(acc, arr[i], ctx);
    }
    return acc;
}
