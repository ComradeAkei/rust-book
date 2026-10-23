#!/usr/bin/env bash
# Собирает C-пример из введения к главе 10 и сверяет с текстом:
# sort.c — qsort с компаратором через указатель против сортировки,
# размноженной макросом. GCC и clang молчат о компараторе не того типа;
# qsort передают адрес cmp_u32, и на миллионе чисел она вызывает его
# миллионы раз; в sort_uint32_t сравнение встроено, и вызовов, кроме
# рекурсии, нет; ошибка в аргументе макроса видна только после
# подстановки.
# Запуск: bash code/ch10/c/run.sh. Нужен gcc и objdump; если есть clang,
# проверяется и он.
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

if ! "$CC" -O2 -Wall -Wextra -o "$tmp/sort" sort.c 2>"$tmp/gcc.log"; then
    echo "FAIL  sort.c не собирается"
    sed 's/^/      /' "$tmp/gcc.log"
    exit 1
fi
expect "sort.c: GCC -Wall -Wextra молчит" нет "warning" "$(cat "$tmp/gcc.log")"
if command -v "$CLANG" >/dev/null; then
    w=$("$CLANG" -O2 -Wall -Wextra -c -o "$tmp/sort-clang.o" sort.c 2>&1)
    expect "sort.c: clang -Wall -Wextra молчит" нет "warning" "$w"
else
    echo "skip  clang: не найден"
fi

out=$("$tmp/sort" 100000)
expect "sort.c: qsort упорядочила" есть "упорядочен: 1" "$(grep '^qsort:' <<<"$out")"
expect "sort.c: макрос упорядочил" есть "упорядочен: 1" "$(grep '^sort_uint32_t:' <<<"$out")"
expect "sort.c: cmp_u16 для u32 — беспорядок без предупреждений" есть "qsort с cmp_u16: упорядочен: 0" "$out"

dis=$(objdump -d --no-show-raw-insn -M intel "$tmp/sort")
main=$(awk '/<main>:/,/^$/' <<<"$dis")
expect "sort.c: qsort получает адрес cmp_u32" есть "<cmp_u32>" "$main"
calls=$(awk '/<sort_uint32_t>:/,/^$/' <<<"$dis" | grep 'call' | grep -v '<sort_uint32_t>')
if [ -z "$calls" ]; then
    echo "ok    sort.c: в sort_uint32_t сравнение встроено, вызовов нет"
else
    echo "FAIL  sort.c: в sort_uint32_t есть вызовы"
    echo "$calls" | sed 's/^/      /'
    fail=1
fi

"$CC" -O2 -DCOUNT -o "$tmp/count" sort.c
n=$("$tmp/count" 100000 | sed -n 's/^вызовов cmp_u32: //p')
# n·log2(n) ≈ 1,66 млн для 100 000 элементов; qsort из glibc 2.39
# сортирует слиянием (msort_with_tmp) и делает 1 536 447 сравнений.
if [ "${n:-0}" -gt 800000 ] && [ "$n" -lt 3400000 ]; then
    echo "ok    sort.c: $n косвенных вызовов cmp_u32 на 100 000 элементах"
else
    echo "FAIL  sort.c: число вызовов cmp_u32 «$n» вне 0,8–3,4 млн"
    fail=1
fi

err=$("$CC" -O2 -DBAD_POINT -c -o "$tmp/bad.o" sort.c 2>&1)
expect "sort.c: point не сравнить через <" есть "invalid operands to binary < (have 'point' and 'point')" "$err"
expect "sort.c: ошибка найдена внутри подстановки" есть "in expansion of macro 'DEFINE_SORT'" "$err"

exit $fail
