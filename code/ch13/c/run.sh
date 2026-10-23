#!/usr/bin/env bash
# Собирает C-пример из введения к главе 13 и сверяет с текстом:
# loops.c — четыре цикла по индексам с четырьмя ошибками. GCC с -Wextra
# предупреждает только о сравнении `i >= 0` для size_t, clang с -Wall
# -Wextra молчит и о нём; анализаторы GCC и clang молчат обо всём. На
# нормальных данных программа печатает верные числа, кроме drop_spikes,
# которая оставляет один из двух соседних выбросов. Под ASan: окно длиннее
# массива и разности читают за концом, поиск с конца — перед началом.
# Без санитайзера поиск с конца возвращает отрицательный «индекс», но не
# -1, а окно длиннее массива роняет программу.
# Запуск: bash code/ch13/c/run.sh. Нужен gcc.
set -u
export LC_ALL=C
cd "$(dirname "$0")"
CC=${CC:-gcc}
CLANG=${CLANG:-clang}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# expect <описание> <есть|нет> <подстрока> <текст>
expect() {
    if grep -qF -- "$3" <<<"$4"; then found=есть; else found=нет; fi
    if [ "$found" = "$2" ]; then
        echo "ok    $1"
    else
        echo "FAIL  $1: «$3» ожидалось: $2"
        echo "$4" | sed 's/^/      /'
        fail=1
    fi
}

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/loops" loops.c 2>"$tmp/gcc.log"; then
    echo "FAIL  loops.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
log=$(cat "$tmp/gcc.log")
expect "loops.c: GCC -Wextra видит i >= 0" есть \
    "comparison of unsigned expression in '>= 0' is always true [-Wtype-limits]" "$log"
expect "loops.c: GCC предупреждает один раз" есть "1" "$(grep -c 'warning:' <<<"$log")"
log=$("$CC" -O2 -fanalyzer -c -o /dev/null loops.c 2>&1)
expect "loops.c: GCC -fanalyzer молчит" нет "warning" "$log"
if command -v "$CLANG" >/dev/null; then
    log=$("$CLANG" -O2 -Wall -Wextra -o "$tmp/loops-clang" loops.c 2>&1)
    expect "loops.c: clang -Wall -Wextra молчит" нет "warning" "$log"
    log=$("$CLANG" -O2 -Wall -Wextra -Wtype-limits -c -o /dev/null loops.c 2>&1)
    expect "loops.c: clang видит i >= 0 только с -Wtype-limits" есть \
        "[-Wtautological-unsigned-zero-compare]" "$log"
    log=$("$CLANG" --analyze -o /dev/null loops.c 2>&1)
    expect "loops.c: clang --analyze молчит" нет "warning" "$log"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/loops")
expect "loops.c: суммы по окну" есть "окон по 3: 4, первая сумма 922, последняя 974" "$out"
expect "loops.c: поиск с конца" есть "последний выше 500: 3" "$out"
expect "loops.c: выброс 950 остался" есть "без выбросов: 10 12 950 11 13 (n = 5)" "$out"
expect "loops.c: разности без санитайзера верны" есть "разности: 2 888 50 -939 2 (n = 5)" \
    "$("$tmp/loops" deltas)"
last=$(timeout 10 "$tmp/loops" last)
expect "loops.c: поиск с конца без находки — отрицательное число" есть \
    "последний выше 1000: -" "$last"
if grep -qx -- "последний выше 1000: -1" <<<"$last"; then
    echo "FAIL  loops.c: поиск с конца вернул -1, а должен был уйти за начало массива"
    fail=1
else
    echo "ok    loops.c: и это не -1"
fi
rc=$({ timeout 10 "$tmp/loops" short >/dev/null 2>&1; echo $?; } 2>/dev/null)
expect "loops.c: окно длиннее массива роняет программу (SIGSEGV)" есть "139" "$rc"

"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all -o "$tmp/loops-san" loops.c
san() { "$tmp/loops-san" "${1:-}" 2>&1; }
short=$(san short)
expect "loops.c под ASan: окно длиннее массива" есть \
    "ERROR: AddressSanitizer: stack-buffer-overflow" "$short"
expect "loops.c под ASan: место — window_sums" есть "in window_sums" "$short"
last=$(san last)
expect "loops.c под ASan: поиск с конца уходит за начало" есть \
    "ERROR: AddressSanitizer: stack-buffer-underflow" "$last"
expect "loops.c под ASan: место — last_above" есть "in last_above" "$last"
deltas=$(san deltas)
expect "loops.c под ASan: разности читают x[n]" есть \
    "ERROR: AddressSanitizer: stack-buffer-overflow" "$deltas"
expect "loops.c под ASan: место — deltas" есть "in deltas" "$deltas"
expect "loops.c под ASan: drop_spikes ошибок не видно" нет "ERROR" "$(san)"
expect "loops.c под ASan: выброс 950 остался" есть "без выбросов: 10 12 950 11 13 (n = 5)" "$(san)"

if [ -x "$tmp/loops-clang" ]; then
    timeout 10 "$tmp/loops-clang" last | sed 's/^/инфо  clang: /'
fi
timeout 10 "$tmp/loops" last | sed 's/^/инфо  gcc: /'

exit $fail
