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
if ! cargo build -p ch18 --quiet; then
    echo "FAIL  cargo build -p ch18"
    exit 1
fi
lib="$root/target/debug/libch18.a"
if [ ! -f "$lib" ]; then
    echo "FAIL  нет $lib"
    exit 1
fi

# Системные библиотеки, которых требует статически слинкованная libstd:
# спрашиваем у самого rustc, а не зашиваем список.
native=$(printf '' | rustc --edition 2024 --crate-type staticlib \
    --print native-static-libs -o /dev/null - 2>&1 |
    sed -n 's/.*native-static-libs: //p')
[ -n "$native" ] || native="-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc"

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
