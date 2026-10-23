#!/usr/bin/env bash
# Проверяет, что каталог неопределённого поведения главы 16 действительно
# неопределённое поведение: каждый фрагмент задачи A и висячая ссылка из
# unsound-API задачи C обязаны вызывать ошибку Miri. Задача D зависит от
# модели алиасинга: под Stacked Borrows (по умолчанию) — ошибка, под
# Tree Borrows (-Zmiri-tree-borrows) — программа законна.
#
# Тесты в solutions/ch16/tests/ub.rs помечены #[ignore]: обычный cargo test
# их не запускает, а этот скрипт гоняет поодиночке, потому что первое же UB
# прекращает весь процесс Miri. Запуск: bash code/ch16/ub/run.sh. Нужен
# nightly с компонентом miri (см. приложение D).
set -u
export LC_ALL=C
cd "$(dirname "$0")/../../.." || exit 2

MIRI=(cargo +nightly miri test -p ch16-solutions --test ch16-ub)
fail=0

# Запустить один тест и сверить код возврата с ожидаемым (0 — прошёл,
# 1 — Miri нашёл UB). $1 — имя, $2 — ожидаемый код, $3 — пояснение,
# дальше — переменные окружения для Miri.
expect() {
    local name=$1 want=$2 note=$3
    shift 3
    env "$@" "${MIRI[@]}" "$name" -- --ignored --exact >/tmp/ch16-ub.log 2>&1
    local got=$?
    if [ "$got" -eq "$want" ]; then
        echo "ok   $name ($note): код $got"
    else
        echo "FAIL $name ($note): ожидался код $want, получен $got"
        sed -n '1,40p' /tmp/ch16-ub.log
        fail=1
    fi
}

# Задача A: каждый фрагмент — UB, Miri обязан его поймать (код 1).
for t in a_bounds a_use_after_free a_invalid_bool a_aliasing a_uninit a_misaligned; do
    expect "$t" 1 "UB под Miri"
done

# Задача C: unsound-API даёт висячую ссылку, Miri ловит use-after-free.
expect c_unsound_dangles 1 "use-after-free"

# Задача D: Tree Borrows принимает (код 0), Stacked Borrows отвергает (код 1).
expect d_tree_ok_stack_ub 1 "Stacked Borrows: UB"
expect d_tree_ok_stack_ub 0 "Tree Borrows: законно" MIRIFLAGS=-Zmiri-tree-borrows

rm -f /tmp/ch16-ub.log
if [ "$fail" -ne 0 ]; then
    echo "каталог UB главы 16: есть расхождения"
    exit 1
fi
echo "каталог UB главы 16: все вердикты Miri совпали с ожидаемыми"
