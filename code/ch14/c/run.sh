#!/usr/bin/env bash
# Собирает C-примеры главы 14 и сверяет с текстом:
# kref.c — конфигурация со счётчиком ссылок вручную и список подписчиков,
# twice.c — два чтения через const int * вокруг вызова (раздел 14.5).
# GCC и clang с -Wall -Wextra молчат. Без санитайзеров программа печатает
# неверное число ссылок после ошибки в logger_open, читает освобождённую
# конфигурацию после лишнего config_put и теряет вызов подписчика, когда
# обработчик подписывает нового и realloc переносит массив. ASan находит
# обращения к освобождённой памяти, LeakSanitizer — утечку, TSan — гонку на
# неатомарном счётчике. Статические анализаторы не отслеживают значение
# счётчика: GCC -fanalyzer предупреждает в сценарии double, clang --analyze
# — в notify, double и ещё в трёх местах без ошибки, утечку не видит ни
# один. Исходы гонки без санитайзера печатаются для сведения: они меняются
# от запуска к запуску.
# Запуск: bash code/ch14/c/run.sh. Нужен gcc с libasan и libtsan.
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

if ! "$CC" -O2 -Wall -Wextra -pthread -o "$tmp/kref" kref.c 2>"$tmp/gcc.log"; then
    echo "FAIL  kref.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
expect "kref.c: GCC -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/gcc.log")"
if command -v "$CLANG" >/dev/null; then
    "$CLANG" -O2 -Wall -Wextra -pthread -o "$tmp/kref-clang" kref.c 2>"$tmp/clang.log"
    expect "kref.c: clang -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/clang.log")"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/kref")
expect "kref.c: вызов подписчика потерян" есть \
    "подписчиков: 3, вызовов: 1 0 1 (ожидалось 1 1 1)" "$out"
expect "kref.c: ссылка после ошибки не отдана" есть \
    "журнал не открыт, ссылок: 2 (ожидалось 1)" "$out"
expect "kref.c: журнал читает освобождённую конфигурацию" есть "журнал: период " "$out"
expect "kref.c: и читает не то, что было записано" нет "журнал: период 100 мс" "$out"

an=$("$CC" -O2 -fanalyzer -fdiagnostics-plain-output -pthread -o /dev/null kref.c 2>&1)
count "kref.c: GCC -fanalyzer предупреждает дважды" 2 "warning:" "$an"
expect "kref.c: GCC -fanalyzer: use after free в config_put" есть \
    "kref.c:38:12: warning: use after 'free'" "$an"
expect "kref.c: GCC -fanalyzer: use after free в scenario_double" есть \
    "kref.c:136:5: warning: use after 'free'" "$an"
expect "kref.c: GCC -fanalyzer: путь через первый же config_put" есть \
    "kref.c:134:5: note: (13) inlined call to 'sender_stop'" "$an"
expect "kref.c: GCC -fanalyzer утечки не видит" нет "malloc-leak" "$an"
if command -v "$CLANG" >/dev/null; then
    an=$("$CLANG" --analyze -o /dev/null kref.c 2>&1)
    count "kref.c: clang --analyze предупреждает пять раз" 5 "warning: Use of memory after it is freed" "$an"
    for l in 99:9 125:5 135:5 136:52 155:20; do
        expect "kref.c: clang --analyze: строка $l" есть "kref.c:$l: warning" "$an"
    done
    expect "kref.c: clang --analyze утечки не видит" нет "leak" "$an"
fi

"$CC" -O1 -g -fsanitize=address,undefined -fno-sanitize-recover=all -pthread \
    -o "$tmp/kref-asan" kref.c
san() { "$tmp/kref-asan" "$1" 2>&1; }
notify=$(san notify)
expect "kref.c под ASan: подписчик пишет в старый массив" есть \
    "ERROR: AddressSanitizer: heap-use-after-free" "$notify"
expect "kref.c под ASan: место — config_set_period" есть \
    "in config_set_period /" "$notify"
expect "kref.c под ASan: строка 99" есть "kref.c:99 in config_set_period" "$notify"
expect "kref.c под ASan: массив освободил realloc" есть "kref.c:86" "$(grep -A2 "freed by" <<<"$notify")"
leak=$(san leak)
expect "kref.c под ASan: утечка" есть "ERROR: LeakSanitizer: detected memory leaks" "$leak"
expect "kref.c под ASan: утекла конфигурация" есть "Direct leak of 24 byte(s) in 1 object(s)" "$leak"
expect "kref.c под ASan: выделена в config_new" есть "kref.c:19" "$(grep -A2 "Direct leak" <<<"$leak" | grep config_new)"
double=$(san double)
expect "kref.c под ASan: лишний config_put" есть \
    "ERROR: AddressSanitizer: heap-use-after-free" "$double"
expect "kref.c под ASan: читает журнал" есть "kref.c:136 in scenario_double" "$double"
expect "kref.c под ASan: освободил config_put" есть "kref.c:39" "$(grep -A2 "freed by" <<<"$double" | grep config_put)"

"$CC" -O1 -g -fsanitize=thread -pthread -o "$tmp/kref-tsan" kref.c
race=$("$tmp/kref-tsan" race 2>&1)
# TSan из GCC 13 не запускается при 32 битах случайности mmap (ядра 6.5+
# и раннеры GitHub): тогда — без ASLR.
if grep -qF "FATAL: ThreadSanitizer" <<<"$race"; then
    race=$(setarch "$(uname -m)" -R "$tmp/kref-tsan" race 2>&1)
fi
expect "kref.c под TSan: гонка" есть "WARNING: ThreadSanitizer: data race" "$race"
expect "kref.c под TSan: в config_get" есть "kref.c:32 in config_get" "$race"
expect "kref.c под TSan: в config_put" есть "kref.c:38 in config_put" "$race"

# twice.c: значение за const int * после вызова читается снова.
loads=$("$CC" -O2 -S -masm=intel -o - twice.c | grep -ciF "dword ptr [r")
count_eq() { if [ "$2" = "$3" ]; then echo "ok    $1"; else echo "FAIL  $1: $2, ожидалось $3"; fail=1; fi; }
count_eq "twice.c: GCC читает *x дважды" "$loads" 2
if command -v "$CLANG" >/dev/null; then
    loads=$("$CLANG" -O2 -S -masm=intel -o - twice.c | grep -ciF "dword ptr [r")
    count_eq "twice.c: clang читает *x дважды" "$loads" 2
fi

for i in $(seq 20); do
    r=$("$tmp/kref" race 2>&1)
    echo "код $? $r" | head -1
done | sort | uniq -c | sed 's/^/инфо  гонка, 20 запусков: /'

exit $fail
