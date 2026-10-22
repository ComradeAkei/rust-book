#!/usr/bin/env bash
# Собирает config.c из введения к главе 5 и сверяет с текстом: все записи
# указывают в один буфер строки и после чтения показывают последнюю строку,
# а предупреждения GCC и clang, AddressSanitizer и UBSan молчат — ошибки
# памяти здесь нет, есть ошибка времени жизни данных.
# Запуск: bash code/ch05/c/run.sh. Нужен gcc с libasan и libubsan; если есть
# clang, проверяется и его анализатор.
set -u
export LC_ALL=C
cd "$(dirname "$0")"
CC=${CC:-gcc}
CLANG=${CLANG:-clang}
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
input=$'rate=100\nport=uart1\nmode=burst\n'

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

# same <описание> <ожидаемое> <текст>
same() {
    if [ "$2" = "$3" ]; then
        echo "ok    $1"
    else
        echo "FAIL  $1"
        diff <(echo "$2") <(echo "$3") | sed 's/^/      /'
        fail=1
    fi
}

wrong=$'mode = burst\nmode = burst\nmode = burst'

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/plain" config.c 2>"$tmp/cc.log"; then
    echo "FAIL  config.c не собирается"
    sed 's/^/      /' "$tmp/cc.log"
    exit 1
fi
same "три записи, и все показывают последнюю строку" "$wrong" "$(printf %s "$input" | "$tmp/plain")"

# Пустая строка в конце файла перезаписывает начало буфера: ключи
# становятся переводом строки, а значения уцелели только потому, что
# лежат дальше.
out=$(printf '%s\n' "$input" | "$tmp/plain")
expect "лишняя пустая строка портит ключи" нет "mode" "$out"
same "значения после пустой строки" "3" "$(grep -c '^ = burst$' <<<"$out")"

w=$("$CC" -O2 -Wall -Wextra -fanalyzer -c -o "$tmp/config.o" config.c 2>&1)
expect "GCC -Wall -Wextra -fanalyzer молчит" нет "warning" "$w"

if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" -Wall -Wextra --analyze -o "$tmp/config.plist" config.c 2>&1)
    expect "clang -Wall -Wextra --analyze молчит" нет "warning" "$w"
else
    echo "skip  clang: не найден"
fi

if "$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all \
    -o "$tmp/san" config.c 2>"$tmp/cc.log"; then
    out=$(printf %s "$input" | "$tmp/san" 2>&1)
    same "ASan и UBSan молчат, вывод тот же" "$wrong" "$out"
else
    echo "FAIL  config.c не собирается с -fsanitize=address,undefined"
    sed 's/^/      /' "$tmp/cc.log"
    fail=1
fi

exit $fail
