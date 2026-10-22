#!/usr/bin/env bash
# Собирает queue.c из введения к главе 3 и сверяет с текстом: AddressSanitizer
# находит обращение к освобождённой памяти, а статические анализаторы GCC и
# clang молчат.
# Запуск: bash code/ch03/c/run.sh. Нужен gcc с libasan; если есть clang,
# проверяется и его анализатор, иначе эта часть пропускается.
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

if ! "$CC" -O1 -g -fsanitize=address -o "$tmp/asan" queue.c 2>"$tmp/cc.log"; then
    echo "FAIL  queue.c не собирается с -fsanitize=address"
    sed 's/^/      /' "$tmp/cc.log"
    exit 1
fi
out=$(ASAN_OPTIONS=detect_leaks=0 "$tmp/asan" 2>&1)
expect "ASan: обращение к освобождённой памяти" есть "ERROR: AddressSanitizer: heap-use-after-free" "$out"
expect "ASan: место обращения — flush"          есть "in flush" "$out"
expect "ASan: строка обращения"                 есть "queue.c:20" "$out"
expect "ASan: освобождено в produce"            есть "queue.c:43" "$out"

w=$("$CC" -O2 -Wall -Wextra -fanalyzer -c -o /dev/null queue.c 2>&1)
expect "GCC -Wall -Wextra -fanalyzer молчит"   нет "warning" "$w"

if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" --analyze -o /dev/null queue.c 2>&1)
    expect "clang --analyze молчит"             нет "warning" "$w"
else
    echo "skip  clang: не найден"
fi

exit $fail
