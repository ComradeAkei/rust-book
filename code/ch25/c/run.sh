#!/usr/bin/env bash
# Собирает C-примеры главы 25 и сверяет с текстом: const_size.c (C17:
# `const int` — не константное выражение), constexpr.c (C23: constexpr —
# только объекты, без функций), divisor.c (делитель UART в #if и #error).
# Запуск: bash code/ch25/c/run.sh. Нужен gcc 13 или новее.
set -u
export LC_ALL=C
cd "$(dirname "$0")"
CC=${CC:-gcc}
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

log=$("$CC" -std=c17 -c -o "$tmp/a.o" const_size.c 2>&1)
expect "const_size.c: C17 отвергает const int в размере" есть \
    "variably modified 'buf' at file scope" "$log"

log=$("$CC" -std=c2x -c -o "$tmp/b.o" constexpr.c 2>&1)
expect "constexpr.c: размер из constexpr принят" нет "variably modified" "$log"
expect "constexpr.c: элемент constexpr-массива — не константа" есть \
    "expression in static assertion is not constant" "$log"
expect "constexpr.c: вызов функции — не константа" есть \
    "initializer element is not constant" "$log"

if "$CC" -std=c17 -O2 -o "$tmp/div" divisor.c 2>"$tmp/div.log"; then
    expect "divisor.c: 16 МГц, 115200 бод" есть "IBRD = 8, FBRD = 44" "$("$tmp/div")"
else
    echo "FAIL  divisor.c не собирается"
    sed 's/^/      /' "$tmp/div.log"
    fail=1
fi
log=$("$CC" -std=c17 -DUART_CLK=12000000u -DBAUD=921600u -c -o "$tmp/d.o" divisor.c 2>&1)
expect "divisor.c: 12 МГц, 921600 бод — #error" есть \
    "#error \"UART: делитель меньше 1" "$log"

exit $fail
