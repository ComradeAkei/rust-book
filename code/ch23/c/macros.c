/* Введение к главе 23: четыре ловушки текстового препроцессора и X-macro.
 * Запуск и сверка с текстом: bash code/ch23/c/run.sh. */
#include <stdio.h>

/* ANCHOR: traps */
#define SQUARE(x) x * x                       /* приоритет: SQUARE(1 + 2) */
#define MAX(a, b) ((a) > (b) ? (a) : (b))     /* аргумент вычисляется дважды */
#define SWAP(a, b) { int tmp = a; a = b; b = tmp; }  /* захват имени tmp */
#define LOG2(a, b) puts(a); puts(b)           /* два оператора под одним if */
/* ANCHOR_END: traps */

/* ANCHOR: xmacro */
#define STATION_ERRORS(X)          \
    X(OK, 0, "нет ошибки")         \
    X(TIMEOUT, 1, "тайм-аут")      \
    X(CRC, 2, "ошибка CRC")        \
    X(OVERRUN, 3, "переполнение приёмника")

#define AS_ENUM(name, code, text) ERR_##name = code,
#define AS_TEXT(name, code, text) [code] = text,

enum station_error { STATION_ERRORS(AS_ENUM) };
static const char *const error_text[] = { STATION_ERRORS(AS_TEXT) };
/* ANCHOR_END: xmacro */

static int calls;
static int next(void) { return ++calls * 10; }

int main(void) {
    printf("SQUARE(1 + 2) = %d\n", SQUARE(1 + 2));

    int m = MAX(next(), 3);
    printf("MAX(next(), 3) = %d, вызовов next: %d\n", m, calls);

    int x = 1, tmp = 2;
    SWAP(x, tmp);
    printf("SWAP(x, tmp): x = %d, tmp = %d\n", x, tmp);

    int verbose = 0;
    if (verbose)
        LOG2("LOG2: первая строка", "LOG2: вторая строка");

    printf("error_text[ERR_CRC] = %s\n", error_text[ERR_CRC]);
    return 0;
}
