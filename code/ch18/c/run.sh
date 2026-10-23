#!/usr/bin/env bash
# Раздел 18.4: C вызывает Rust. Собираем статическую библиотеку крейта ch18 и
# линкуем с ней C-программу host.c, которая создаёт, дёргает и освобождает
# непрозрачный счётчик. Под AddressSanitizer с LeakSanitizer: забыли бы
# ch18_counter_free — санитайзер сообщил бы об утечке Box под Rust.
# Запуск: bash code/ch18/c/run.sh. Нужен cc/gcc и тулчейн из
# rust-toolchain.toml (приложение D).
set -u
export LC_ALL=C
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
cd "$root" || exit 2
CC=${CC:-cc}
fail=0

echo "== сборка статической библиотеки ch18 =="
# Системные библиотеки, которых требует статическая библиотека: те, что
# нужны libstd, и те, что крейт сам просит через #[link] (zlib, раздел 18.6).
# Спрашиваем у rustc при сборке самого крейта, а не зашиваем список:
# пустой крейт про -lz не знает, а без инкрементальной сборки (как в CI)
# обёртка над zlib попадает в один объектный файл с экспортом для C, и
# ссылки на функции zlib остаются неразрешёнными.
if ! build=$(cargo rustc -p ch18 --lib --quiet -- --print native-static-libs 2>&1); then
    echo "FAIL  cargo rustc -p ch18"
    printf '%s\n' "$build" | sed 's/^/      /'
    exit 1
fi
lib="$root/target/debug/libch18.a"
if [ ! -f "$lib" ]; then
    echo "FAIL  нет $lib"
    exit 1
fi

native=$(printf '%s\n' "$build" | sed -n 's/.*native-static-libs: //p' | tail -n 1)
[ -n "$native" ] || native="-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc -lz"

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

echo "== C вызывает Rust под ASan =="
# shellcheck disable=SC2086
if "$CC" -fsanitize=address -O1 -g "$here/host.c" -o "$tmp/host" "$lib" $native 2>"$tmp/cc.log"; then
    if out=$(ASAN_OPTIONS=detect_leaks=1 "$tmp/host" 2>"$tmp/run.log"); then
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
