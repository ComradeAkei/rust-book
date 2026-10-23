#!/usr/bin/env bash
# Собирает C-пример из введения к главе 23 и сверяет с текстом: macros.c —
# четыре ловушки препроцессора и X-macro. SQUARE(1 + 2) даёт 5, MAX
# вызывает next() дважды, SWAP(x, tmp) молча ничего не меняет, а LOG2 под
# if печатает вторую строку. GCC с -Wall -Wextra видит только LOG2
# (-Wmultistatement-macros), захват имени tmp — лишь с -Wshadow; clang с
# -Wall -Wextra молчит обо всём.
# Запуск: bash code/ch23/c/run.sh. Нужен gcc.
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

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/macros" macros.c 2>"$tmp/gcc.log"; then
    echo "FAIL  macros.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
log=$(cat "$tmp/gcc.log")
expect "macros.c: GCC -Wall видит LOG2 под if" есть \
    "macro expands to multiple statements [-Wmultistatement-macros]" "$log"
expect "macros.c: GCC -Wall предупреждает один раз" есть "1" "$(grep -c 'warning:' <<<"$log")"
log=$("$CC" -O2 -Wall -Wextra -Wshadow -c -o /dev/null macros.c 2>&1)
expect "macros.c: захват tmp видно только с -Wshadow" есть \
    "declaration of 'tmp' shadows a previous local [-Wshadow]" "$log"
if command -v "$CLANG" >/dev/null; then
    log=$("$CLANG" -O2 -Wall -Wextra -c -o /dev/null macros.c 2>&1)
    expect "macros.c: clang -Wall -Wextra молчит" нет "warning" "$log"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/macros")
expect "macros.c: SQUARE(1 + 2) = 5" есть "SQUARE(1 + 2) = 5" "$out"
expect "macros.c: MAX вызывает next() дважды" есть "MAX(next(), 3) = 20, вызовов next: 2" "$out"
expect "macros.c: SWAP(x, tmp) ничего не меняет" есть "SWAP(x, tmp): x = 1, tmp = 2" "$out"
expect "macros.c: LOG2 под if печатает вторую строку" есть "LOG2: вторая строка" "$out"
expect "macros.c: и не печатает первую" нет "LOG2: первая строка" "$out"
expect "macros.c: X-macro" есть "error_text[ERR_CRC] = ошибка CRC" "$out"

exit $fail
