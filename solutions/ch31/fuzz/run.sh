#!/usr/bin/env bash
# Задача 31.B: исходы фаззинга оракулов из разбора задачи (приложение E).
# packet_injected и config_injected — оракулы на копиях заготовки с
# внедрёнными ошибками: фаззер находит обе с пустого корпуса, без
# затравок, — сумма заголовка IPv4 libFuzzer подбирает сам по сравнениям.
# У ошибки токенизатора два проявления — паника на срезе посреди символа
# и обрезанное значение, — и какое фаззер встретит первым, зависит от
# сборки: годится любое.
# packet и config — те же оракулы на исправленных копиях: падений нет.
# Запуски детерминированы для одной сборки: одно зерно, одно число
# запусков, один поток. Другой компилятор — другие числа запусков.
# Запуск: bash solutions/ch31/fuzz/run.sh. Нужны nightly и cargo-fuzz.
set -u
# Вывод cargo сверяется с текстом: без кодов цвета, даже если снаружи
# задан CARGO_TERM_COLOR=always (так в CI).
export LC_ALL=C CARGO_TERM_COLOR=never
cd "$(dirname "$0")" || exit 2
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
FUZZ=(cargo +nightly fuzz)

"${FUZZ[@]}" build || exit 1

# fuzz <цель> <crash|clean> <подстрока или -> <флаги libFuzzer...>
# Подстрок может быть несколько через « || »: годится любая.
fuzz() {
    local target=$1 want=$2 needle=$3
    shift 3
    local dir="$tmp/$target"
    mkdir -p "$dir/corpus"
    RUST_BACKTRACE=0 "${FUZZ[@]}" run "$target" "$dir/corpus" -- \
        -seed=1 -artifact_prefix="$dir/" "$@" >"$dir/log" 2>&1
    local got=clean runs
    if ls "$dir"/crash-* >"$dir/ls" 2>&1; then got=crash; fi
    if [ "$got" = crash ]; then
        runs="падение после $(awk '/^#[0-9]+/ { n = substr($1, 2) } END { print n }' "$dir/log") запусков"
    else
        runs=$(grep -E '^Done [0-9]+ runs' "$dir/log")
    fi
    local found=no rest=$needle alt
    if [ "$needle" = - ]; then
        found=yes
    else
        while :; do
            alt=${rest%% || *}
            grep -qF -- "$alt" "$dir/log" && found=yes
            [ "$alt" = "$rest" ] && break
            rest=${rest#* || }
        done
    fi
    if [ "$got" = "$want" ] && [ "$found" = yes ]; then
        echo "ok    $target ($*): $runs"
    else
        echo "FAIL  $target ($*): ожидалось $want, получено $got ($runs)"
        tail -n 40 "$dir/log" | sed 's/^/      /'
        fail=1
    fi
}

fuzz packet_injected crash "заголовок и нагрузка — не весь пакет" -runs=2000000
fuzz config_injected crash "is not a char boundary || оборвано: дальше в строке" -runs=2000000
fuzz packet clean - -runs=2000000
fuzz config clean - -runs=2000000

if [ "$fail" -ne 0 ]; then
    echo "фаззинг задачи 31.B: есть расхождения"
    exit 1
fi
echo "фаззинг задачи 31.B: исходы совпали с разбором"
