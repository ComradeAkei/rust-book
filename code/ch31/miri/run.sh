#!/usr/bin/env bash
# Раздел 31.6: опыты программы san (раздел 31.5) под Miri. Каждый опыт —
# отдельный запуск: первое же UB останавливает Miri. Use-after-free, утечку
# и обе гонки Miri ловит сам; вызов C-функции он выполнить не может; чтение
# за концом в сборке dev раньше Miri ловит проверка предусловия
# get_unchecked; очередь главы 21 с порядками AcqRel проходит, и таблица
# в OnceLock тоже — Miri выполняет и код std, ложной гонки, как у TSan над
# готовой std, у него не бывает.
# Запуск: bash code/ch31/miri/run.sh. Нужен nightly с компонентом miri.
set -u
# Вывод cargo сверяется с текстом: без кодов цвета, даже если снаружи
# задан CARGO_TERM_COLOR=always (так в CI).
export LC_ALL=C CARGO_TERM_COLOR=never
cd "$(dirname "$0")/../../.." || exit 2
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# expect <опыт> <код> <подстрока>
expect() {
    local what=$1 want=$2 needle=$3
    cargo +nightly miri run -q -p ch31 --bin san -- "$what" >"$tmp/log" 2>&1
    local got=$?
    if [ "$got" -eq "$want" ] && grep -qF -- "$needle" "$tmp/log"; then
        echo "ok    san $what: код $got, «$needle»"
    else
        echo "FAIL  san $what: ожидались код $want и «$needle», получен код $got"
        sed -n '1,40p' "$tmp/log" | sed 's/^/      /'
        fail=1
    fi
}

expect uaf 1 "has been freed, so this pointer is dangling"
expect overflow 1 "unsafe precondition(s) violated: slice::get_unchecked requires that the index is within the slice"
expect ffi 1 "unsupported operation: can't call foreign function \`fill_counter\`"
expect leak 1 "error: memory leaked:"
expect race 1 "Data race detected between (1) non-atomic write on thread"
expect relaxed 1 "Data race detected between (1) retag write on thread"
expect spsc 0 "сумма: 49995000"
expect once 0 "999"

if [ "$fail" -ne 0 ]; then
    echo "опыты главы 31 под Miri: есть расхождения"
    exit 1
fi
echo "опыты главы 31 под Miri: вердикты совпали с текстом"
