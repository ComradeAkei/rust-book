#!/usr/bin/env bash
# Проверяет задачу A главы 17: три из восьми вызовов transmute дают
# неопределённое поведение, и Miri обязан поймать каждый. Все три создают
# значение, нарушающее инвариант валидности типа (bool, ниша NonZero,
# диапазон char), поэтому UB возникает уже при создании значения.
#
# Тесты в solutions/ch17/tests/ub.rs помечены #[ignore] и гоняются по
# одному: первое же UB прекращает процесс Miri. Запуск:
# bash code/ch17/ub/run.sh. Нужен nightly с компонентом miri (приложение D).
set -u
export LC_ALL=C
cd "$(dirname "$0")/../../.." || exit 2

MIRI=(cargo +nightly miri test -p ch17-solutions --test ch17-ub)
fail=0

expect_ub() {
    local name=$1
    "${MIRI[@]}" "$name" -- --ignored --exact >/tmp/ch17-ub.log 2>&1
    local got=$?
    if [ "$got" -eq 1 ]; then
        echo "ok   $name: Miri поймал UB (код $got)"
    else
        echo "FAIL $name: ожидался код 1 (UB), получен $got"
        sed -n '1,40p' /tmp/ch17-ub.log
        fail=1
    fi
}

for t in a_ub_invalid_bool a_ub_zero_nonzero a_ub_bad_char; do
    expect_ub "$t"
done

rm -f /tmp/ch17-ub.log
if [ "$fail" -ne 0 ]; then
    echo "каталог transmute главы 17: есть расхождения"
    exit 1
fi
echo "каталог transmute главы 17: Miri поймал все UB-вызовы"
