#!/usr/bin/env bash
# Задача C: C управляет кольцевым буфером из Rust через непрозрачный описатель.
# Собираем статическую библиотеку ch18-solutions, линкуем с ней драйвер
# rb_test.c и гоняем под AddressSanitizer с LeakSanitizer: забытый rb_free или
# выход за буфер всплыли бы ошибкой санитайзера. Запуск: bash
# solutions/ch18/c/run.sh. Нужен cc/gcc и тулчейн из rust-toolchain.toml.
set -u
export LC_ALL=C
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
cd "$root" || exit 2
CC=${CC:-cc}
fail=0

echo "== сборка статической библиотеки ch18-solutions =="
# Системные библиотеки, которых требует статическая библиотека: те, что
# нужны libstd, и те, что крейт сам просит через #[link] (zlib, задача D).
# Спрашиваем у rustc при сборке самого крейта, а не зашиваем список:
# пустой крейт про -lz не знает, а без инкрементальной сборки (как в CI)
# обёртка над zlib попадает в один объектный файл с экспортом для C, и
# ссылки на функции zlib остаются неразрешёнными. --color never: при
# CARGO_TERM_COLOR=always (так в CI) строка кончалась бы кодом цвета, и
# компоновщик искал бы библиотеку «z\e[0m».
if ! build=$(cargo rustc --color never -p ch18-solutions --lib --quiet -- --print native-static-libs 2>&1); then
    echo "FAIL  cargo rustc -p ch18-solutions"
    printf '%s\n' "$build" | sed 's/^/      /'
    exit 1
fi
lib="$root/target/debug/libch18_solutions.a"
if [ ! -f "$lib" ]; then
    echo "FAIL  нет $lib"
    exit 1
fi

native=$(printf '%s\n' "$build" | sed -n 's/.*native-static-libs: //p' | tail -n 1)
[ -n "$native" ] || native="-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -lz"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "== C управляет кольцевым буфером под ASan =="
# shellcheck disable=SC2086
if "$CC" -fsanitize=address -O1 -g -I "$here" "$here/rb_test.c" -o "$tmp/rb" "$lib" $native 2>"$tmp/cc.log"; then
    if out=$(ASAN_OPTIONS=detect_leaks=1 "$tmp/rb" 2>"$tmp/run.log"); then
        echo "ok    $out"
    else
        echo "FAIL  программа или ASan сообщили об ошибке:"
        sed 's/^/      /' "$tmp/run.log"
        fail=1
    fi
else
    echo "FAIL  компоновка не удалась:"
    sed 's/^/      /' "$tmp/cc.log"
    fail=1
fi

exit $fail
