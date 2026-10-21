#!/usr/bin/env bash
# Собирает C-примеры приложения A и сверяет вывод с тем, что сказано в
# тексте: неопределённое поведение ловит UBSan, остальное видно по выводу.
# Запуск: bash code/appendix-a/c/run.sh. Нужен gcc с libubsan (gcc 9+).
set -u
cd "$(dirname "$0")"
CC=${CC:-gcc}
UBSAN=(-O1 -fsanitize=undefined,float-cast-overflow)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# check <описание> <ожидаемая подстрока> <файл.c> [флаги компилятора...]
check() {
    local what="$1" expect="$2" src="$3"
    shift 3
    local out
    if ! "$CC" "$@" -o "$tmp/a.out" "$src" 2>"$tmp/cc.log"; then
        echo "FAIL  $what: не собирается"
        sed 's/^/      /' "$tmp/cc.log"
        fail=1
        return
    fi
    out=$("$tmp/a.out" 2>&1; echo "код выхода $?")
    if grep -qF -- "$expect" <<<"$out"; then
        echo "ok    $what"
    else
        echo "FAIL  $what: нет строки «$expect»"
        echo "$out" | sed 's/^/      /'
        fail=1
    fi
}

check "uint16_t * uint16_t переполняет int" "signed integer overflow: 65535 * 65535" promote.c "${UBSAN[@]}"
check "INT_MIN / -1 под UBSan"              "division of -2147483648 by -1"        divmin.c  "${UBSAN[@]}"
check "INT_MIN / -1 без проверок: SIGFPE"   "код выхода 136"                       divmin.c  -O2
check "сдвиг на 32"                         "shift exponent 32 is too large"       shift.c   "${UBSAN[@]}"
check "сдвиг на 32 без проверок"            "1 << 32 = 1"                          shift.c   -O2
check "(int)3e9"                            "3e+09 is outside the range"           float_cast.c "${UBSAN[@]}"
check "(int)3e9 без проверок"               "(int)3e9 = -2147483648"               float_cast.c -O2
check "__builtin_clz(0)"                    "passing zero to clz()"                clz.c     "${UBSAN[@]}"
check "MAX(i++, j) вычисляет i++ дважды"    "m = 6, i = 7"                         max.c     -O2
check "exit() сбрасывает буфер stdio"       "строка в буфере stdio"                exit_flush.c -O2
check "компаратор qsort через вычитание"    "signed integer overflow: 2147483647 - -1" qsort.c "${UBSAN[@]}"
check "qsort с таким компаратором путает порядок" "2147483647 -2147483648 -1 0"    qsort.c   -O2

# Строки ниже повторяются в тестах src/strings.rs как ожидаемый вывод format!.
check "printf: ширина и заполнение целых"   "[   42|42   |00042|+5]"               printf.c  -O2
check "printf: шестнадцатеричный вывод"     "[ff|FF|0xff|000000ff|0x000000ff|10]"  printf.c  -O2
check "printf: вещественные"                "[1.235|    1.23|1.234560e+00]"        printf.c  -O2
check "printf: строки выравниваются вправо" "[abcdef|    abcdef|abcdef    |abc]"   printf.c  -O2
check "atoi и strtol"                       "[42|255|0]"                           printf.c  -O2

exit $fail
