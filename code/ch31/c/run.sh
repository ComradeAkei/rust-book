#!/usr/bin/env bash
# Раздел 31.4, «Как в C»: разбор команд станции на C с той же ошибкой, что
# первая версия на Rust, — калибровка на девять точек пишет за таблицу на
# восемь.
# replay.c — сама команда без фаззера: GCC и clang с -Wall -Wextra молчат,
# программа отвечает «успешно» и портит поле за таблицей. ASan (и GCC, и
# clang) молчит тоже: девятая точка ложится внутрь объемлющей структуры, а
# ASan следит только за границами целых объектов.
# fuzz_shape.c — цель libFuzzer по форме команды: без ASan за два миллиона
# запусков падения нет, с ASan — stack-buffer-overflow за тысячи запусков.
# fuzz_raw.c — цель на сырых байтах: как и в Rust, за три миллиона
# запусков калибровку на девять точек фаззер не собирает.
# cbmc.c — раздел 31.7: проверка моделей CBMC находит запись за таблицу,
# если команда — отдельный объект, и не находит, если она внутри другой
# структуры.
# Запуск: bash code/ch31/c/run.sh. Нужны gcc с libasan и clang с
# рантаймами compiler-rt (в Ubuntu — пакет libclang-rt-18-dev).
set -u
export LC_ALL=C
cd "$(dirname "$0")" || exit 2
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

warn=$("$CC" -std=c17 -O1 -g -Wall -Wextra -o "$tmp/replay-gcc" replay.c cmd.c 2>&1)
expect "GCC -Wall -Wextra молчит" нет "warning" "$warn"
warn=$("$CLANG" -std=c17 -O1 -g -Wall -Wextra -o "$tmp/replay-clang" replay.c cmd.c 2>&1)
expect "clang -Wall -Wextra молчит" нет "warning" "$warn"
"$CC" -std=c17 -O1 -g -fsanitize=address -o "$tmp/replay-gcc-asan" replay.c cmd.c
"$CLANG" -std=c17 -O1 -g -fsanitize=address -o "$tmp/replay-clang-asan" replay.c cmd.c
for exe in replay-gcc replay-clang replay-gcc-asan replay-clang-asan; do
    out=$("$tmp/$exe" 2>&1)
    expect "$exe: девять точек приняты, поле за таблицей испорчено" есть \
        "parse_v1: 0, точек 9, за таблицей 0x41414141" "$out"
    expect "$exe: ASan молчит" нет "AddressSanitizer" "$out"
done

# Раздел 31.7: CBMC — тот же движок, что под Kani; его ставит cargo kani
# setup, путь можно задать переменной CBMC. Отдельный объект команды —
# запись за таблицу найдена; команда внутри объемлющей структуры — нет.
CBMC=${CBMC:-$(command -v cbmc || ls "$HOME"/.kani/kani-*/bin/cbmc 2>"$tmp/err" | tail -n 1)}
if [ -x "$CBMC" ]; then
    flags=(--bounds-check --pointer-check --unwind 44 --unwinding-assertions)
    out=$("$CBMC" cbmc.c cmd.c "${flags[@]}" 2>&1)
    expect "CBMC: запись за таблицу найдена" есть \
        "line 24 memcpy destination region writeable: FAILURE" "$out"
    out=$("$CBMC" -D IN_FRAME cbmc.c cmd.c "${flags[@]}" 2>&1)
    expect "CBMC: таблица внутри структуры — не видит" есть "VERIFICATION SUCCESSFUL" "$out"
else
    echo "skip  CBMC не найден (его ставит cargo kani setup)"
fi

# libFuzzer есть только у clang и только с рантаймами compiler-rt.
if ! "$CLANG" -fsanitize=fuzzer -o "$tmp/probe" fuzz_raw.c cmd.c 2>"$tmp/probe.log"; then
    echo "skip  libFuzzer: clang не нашёл рантайм (нужен пакет libclang-rt-18-dev)"
    sed 's/^/      /' "$tmp/probe.log"
    exit $fail
fi

# fuzz <описание> <crash|clean> <подстрока или -> <исполняемый> <запусков>
fuzz() {
    local what=$1 want=$2 needle=$3 exe=$4 runs=$5
    local dir="$tmp/$exe-run"
    mkdir -p "$dir"
    (cd "$dir" && "$tmp/$exe" -seed=1 -runs="$runs" >log 2>&1)
    local got=clean
    if ls "$dir"/crash-* >"$dir/ls" 2>&1; then got=crash; fi
    local summary
    if [ "$got" = crash ]; then
        summary="падение после $(awk '/^#[0-9]+/ { n = substr($1, 2) } END { print n }' "$dir/log") запусков"
    else
        summary=$(grep -E '^Done [0-9]+ runs' "$dir/log")
    fi
    if [ "$got" = "$want" ] && { [ "$needle" = - ] || grep -qF -- "$needle" "$dir/log"; }; then
        echo "ok    $what: $summary"
    else
        echo "FAIL  $what: ожидалось $want, получено $got ($summary)"
        tail -n 30 "$dir/log" | sed 's/^/      /'
        fail=1
    fi
}

"$CLANG" -std=c17 -O1 -g -fsanitize=fuzzer -o "$tmp/shape" fuzz_shape.c cmd.c
"$CLANG" -std=c17 -O1 -g -fsanitize=fuzzer,address -o "$tmp/shape-asan" fuzz_shape.c cmd.c
"$CLANG" -std=c17 -O1 -g -fsanitize=fuzzer,address -o "$tmp/raw-asan" fuzz_raw.c cmd.c
fuzz "по форме без ASan: падения нет" clean - shape 2000000
fuzz "по форме с ASan: запись за таблицу" crash \
    "ERROR: AddressSanitizer: stack-buffer-overflow" shape-asan 2000000
fuzz "сырые байты с ASan: калибровку не собрал" clean - raw-asan 3000000

exit $fail
