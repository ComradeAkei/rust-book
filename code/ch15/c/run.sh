#!/usr/bin/env bash
# Собирает C-пример главы 15 и сверяет с текстом: table.c — реестр датчиков
# на открытой адресации с хешем из K&R. GCC и clang с -Wall -Wextra молчат.
# Без санитайзеров программа теряет измерение, записанное через указатель на
# ячейку, пережившую рост таблицы; не находит датчик BB после удаления Aa,
# с которым у него общий хеш; на именах из блоков «Aa» и «BB» просматривает
# по половине таблицы на вставку. ASan находит только первую ошибку.
# GCC -fanalyzer предупреждает об одном: table_get может вернуть NULL, —
# об обращении к освобождённому массиву молчит, clang --analyze молчит вовсе.
# Время вставок печатается для сведения: оно меняется от запуска к запуску.
# Запуск: bash code/ch15/c/run.sh. Нужен gcc с libasan.
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

# count <описание> <число> <подстрока> <текст>
count() {
    n=$(grep -cF -- "$3" <<<"$4")
    if [ "$n" = "$2" ]; then
        echo "ok    $1"
    else
        echo "FAIL  $1: «$3» встречается $n раз, ожидалось $2"
        echo "$4" | sed 's/^/      /'
        fail=1
    fi
}

# frame <описание> <функция> <файл:строка> <текст> — кадр стека ASan:
# «in функция …/файл:строка».
frame() {
    if grep -F -- "in $2 " <<<"$4" | grep -qE -- "/$3\$"; then
        echo "ok    $1"
    else
        echo "FAIL  $1: нет кадра «$2 $3»"
        echo "$4" | grep -F " in " | sed 's/^/      /'
        fail=1
    fi
}

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/table" table.c 2>"$tmp/gcc.log"; then
    echo "FAIL  table.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
expect "table.c: GCC -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/gcc.log")"
if command -v "$CLANG" >/dev/null; then
    "$CLANG" -O2 -Wall -Wextra -o "$tmp/table-clang" table.c 2>"$tmp/clang.log"
    expect "table.c: clang -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/clang.log")"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/table")
expect "table.c: измерение ушло в старый массив" есть "adc0: измерений 0 (ожидалось 1)" "$out"
expect "table.c: BB не найден после удаления Aa" есть "BB: не найден" "$out"
expect "table.c: обычные имена — 40 ячеек на вставку" есть \
    "обычные имена:  8192 штук, 40.4 ячеек на вставку" "$out"
expect "table.c: имена из Aa/BB — половина таблицы на вставку" есть \
    "имена из Aa/BB: 8192 штук, 4096.5 ячеек на вставку" "$out"
grep "штук" <<<"$out" | sed 's/^/инфо  /'

an=$("$CC" -O2 -fanalyzer -fdiagnostics-plain-output -o /dev/null table.c 2>&1)
count "table.c: GCC -fanalyzer предупреждает один раз" 1 "warning:" "$an"
expect "table.c: GCC -fanalyzer: table_get может вернуть NULL" есть \
    "table.c:104:5: warning: dereference of NULL" "$an"
expect "table.c: GCC -fanalyzer старого массива не видит" нет "use-after-free" "$an"
if command -v "$CLANG" >/dev/null; then
    an=$("$CLANG" --analyze -o /dev/null table.c 2>&1)
    expect "table.c: clang --analyze молчит" нет "warning" "$an"
fi

if ! "$CC" -O1 -g -fsanitize=address -fno-omit-frame-pointer -o "$tmp/table-asan" \
    table.c 2>"$tmp/asan.log"; then
    echo "skip  ASan: gcc без libasan"
    sed 's/^/      /' "$tmp/asan.log"
else
    asan=$("$tmp/table-asan" dangle 2>&1)
    expect "table.c под ASan: обращение к освобождённой памяти" есть \
        "ERROR: AddressSanitizer: heap-use-after-free" "$asan"
    expect "table.c под ASan: чтение счётчика" есть "READ of size 4" "$asan"
    frame "table.c под ASan: строка 102" scenario_dangle table.c:102 "$asan"
    frame "table.c под ASan: массив освободил grow" grow table.c:61 "$asan"
    frame "table.c под ASan: из table_put" table_put table.c:74 "$asan"
    frame "table.c под ASan: выделен в table_init" table_init table.c:46 "$asan"
    asan=$("$tmp/table-asan" delete 2>&1)
    expect "table.c под ASan: удаление ASan не видит" нет "AddressSanitizer" "$asan"
    expect "table.c под ASan: и BB всё так же не найден" есть "BB: не найден" "$asan"
fi
exit $fail
