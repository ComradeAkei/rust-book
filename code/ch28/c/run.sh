#!/usr/bin/env bash
# Собирает C-пример раздела 28.2 и сверяет с текстом: copy.c — копия
# файла с errno и выходом через goto. Правильная версия копирует и на
# пути ошибки ничего не оставляет открытым. С -DLEAK второй open
# возвращается мимо close: GCC -Wall -Wextra и clang --analyze молчат,
# анализатор GCC видит утечку только без оптимизации, valgrind
# --track-fds=yes находит по дескриптору на каждую неудачную попытку.
# Запуск: bash code/ch28/c/run.sh. Нужен gcc; clang и valgrind — если
# есть.
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

for v in ok leak; do
    def=""
    [ "$v" = leak ] && def=-DLEAK
    # shellcheck disable=SC2086
    if ! "$CC" -O2 -Wall -Wextra $def -o "$tmp/copy-$v" copy.c 2>"$tmp/gcc-$v.log"; then
        echo "FAIL  copy.c ($v) не собирается"
        sed 's/^/      /' "$tmp/gcc-$v.log"
        exit 1
    fi
    expect "copy.c ($v): GCC -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/gcc-$v.log")"
    # shellcheck disable=SC2086
    w=$("$CC" -O0 -fanalyzer $def -c -o "$tmp/a-$v.o" copy.c 2>&1)
    if [ "$v" = leak ]; then
        expect "copy.c (leak): -fanalyzer -O0 видит утечку in" есть \
            "leak of file descriptor 'in' [CWE-775] [-Wanalyzer-fd-leak]" "$w"
        # shellcheck disable=SC2086
        w=$("$CC" -O2 -fanalyzer $def -c -o "$tmp/a2-$v.o" copy.c 2>&1)
        expect "copy.c (leak): -fanalyzer -O2 утечку не видит" нет "fd-leak" "$w"
    else
        expect "copy.c (ok): -fanalyzer молчит" нет "warning" "$w"
    fi
    if command -v "$CLANG" >"$tmp/which"; then
        # shellcheck disable=SC2086
        w=$("$CLANG" --analyze $def -o "$tmp/c-$v.plist" copy.c 2>&1)
        expect "copy.c ($v): clang --analyze молчит" нет "warning" "$w"
    else
        echo "skip  clang: не найден"
    fi
done

head -c 10000 copy.c >"$tmp/src"
out=$("$tmp/copy-ok" "$tmp/src" "$tmp/dst")
expect "copy.c: копия" есть "попытка 1: скопировано" "$out"
if cmp -s "$tmp/src" "$tmp/dst"; then
    echo "ok    copy.c: копия совпадает с источником"
else
    echo "FAIL  copy.c: копия не совпадает"
    fail=1
fi
out=$("$tmp/copy-ok" "$tmp/src" "$tmp/нет/dst" 3)
expect "copy.c: ошибка второго open — errno первой ошибки" есть \
    "попытка 3: ошибка: No such file or directory" "$out"

# Запуск без дескрипторов, унаследованных от родителя: valgrind
# --track-fds считает и их (`<inherited from parent>`), а раннер CI
# оставляет открытыми два своих — 142 и 145.
only_std_fds() {
    (
        for fd in /proc/self/fd/*; do
            fd=${fd##*/}
            if [ "$fd" -gt 2 ]; then eval "exec $fd>&-"; fi
        done
        exec "$@"
    )
}

if command -v valgrind >"$tmp/which"; then
    v=$(only_std_fds valgrind -q --track-fds=yes "$tmp/copy-ok" "$tmp/src" "$tmp/нет/dst" 3 2>&1)
    expect "copy.c (ok): valgrind — только stdin, stdout, stderr" нет "Open file descriptor" "$v"
    v=$(only_std_fds valgrind -q --track-fds=yes "$tmp/copy-leak" "$tmp/src" "$tmp/нет/dst" 3 2>&1)
    expect "copy.c (leak): valgrind — три лишних дескриптора" есть \
        "FILE DESCRIPTORS: 6 open (3 std) at exit." "$v"
    expect "copy.c (leak): открыт источник" есть "Open file descriptor 5: $tmp/src" "$v"
else
    echo "skip  valgrind: не найден"
fi

exit $fail
