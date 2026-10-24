#!/usr/bin/env bash
# Раздел 31.4: опыты с cargo fuzz и их исходы из текста главы.
# cmd_v1 — первая версия разбора команд на сырых байтах: фаззер проходит
# маркер, код команды и CRC, но за миллионы запусков не собирает
# калибровку на девять точек с согласованной длиной, в том числе с
# профилем значений сравнений. cmd_v1_shape — та же функция, но вход —
# форма команды (Arbitrary), длину и CRC считает обёртка: паника на
# девятой точке находится за тысячи запусков, и без санитайзера тоже.
# cmd, cmd_round_trip и slip — исправленный разбор и SLIP: падений нет.
# frames — шаг проекта: decode и Frames разбора кадров на любых байтах.
# Запуски детерминированы: одно зерно, одно число запусков, один поток.
# Запуск: bash code/ch31/fuzz/run.sh. Нужны nightly и cargo-fuzz.
set -u
export LC_ALL=C
cd "$(dirname "$0")" || exit 2
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
FUZZ=(cargo +nightly fuzz)

"${FUZZ[@]}" build || exit 1
"${FUZZ[@]}" build --sanitizer none cmd_v1_shape || exit 1

# fuzz <цель> <crash|clean> <подстрока или -> <санитайзер> <флаги libFuzzer...>
fuzz() {
    local target=$1 want=$2 needle=$3 san=$4
    shift 4
    local dir="$tmp/$target-$san-$#-$RANDOM"
    mkdir -p "$dir/corpus"
    RUST_BACKTRACE=0 "${FUZZ[@]}" run --sanitizer "$san" "$target" "$dir/corpus" -- \
        -seed=1 -artifact_prefix="$dir/" "$@" >"$dir/log" 2>&1
    local got=clean runs
    if ls "$dir"/crash-* >"$dir/ls" 2>&1; then got=crash; fi
    if [ "$got" = crash ]; then
        runs="падение после $(awk '/^#[0-9]+/ { n = substr($1, 2) } END { print n }' "$dir/log") запусков"
    else
        runs=$(grep -E '^Done [0-9]+ runs' "$dir/log")
    fi
    if [ "$got" = "$want" ] && { [ "$needle" = - ] || grep -qF -- "$needle" "$dir/log"; }; then
        echo "ok    $target ($san $*): $runs"
    else
        echo "FAIL  $target ($san $*): ожидалось $want, получено $got ($runs)"
        tail -n 40 "$dir/log" | sed 's/^/      /'
        fail=1
    fi
    if [ "$got" = crash ] && [ "$target" = cmd_v1_shape ] && [ "$san" = address ]; then
        local fmt
        fmt=$("${FUZZ[@]}" fmt "$target" "$dir"/crash-* 2>&1)
        if grep -qF 'Calibrate {' <<<"$fmt"; then
            echo "ok    cargo fuzz fmt: вход-структура напечатан через Debug"
        else
            echo "FAIL  cargo fuzz fmt: нет «Calibrate {»"
            echo "$fmt" | sed 's/^/      /'
            fail=1
        fi
    fi
}

oob="index out of bounds: the len is 8 but the index is 8"
fuzz cmd_v1 clean - address -runs=3000000
fuzz cmd_v1 clean - address -runs=2000000 -use_value_profile=1
fuzz cmd_v1_shape crash "$oob" address -runs=300000
fuzz cmd_v1_shape crash "$oob" none -runs=300000
fuzz cmd clean - address -runs=2000000
fuzz cmd_round_trip clean - address -runs=1000000
fuzz slip clean - address -runs=2000000
fuzz frames clean - address -runs=1000000

if [ "$fail" -ne 0 ]; then
    echo "фаззинг главы 31: есть расхождения"
    exit 1
fi
echo "фаззинг главы 31: исходы совпали с текстом"
