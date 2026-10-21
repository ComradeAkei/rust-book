#!/usr/bin/env bash
# Собирает C-примеры главы 1 и сверяет вывод с тем, что сказано в тексте.
# Запуск: bash code/ch01/c/run.sh. Нужен gcc с libubsan (gcc 9+). Если есть
# arm-none-eabi-gcc с newlib и qemu-system-arm, те же примеры проверяются и
# на Cortex-M3 и Cortex-M4F, иначе эта часть пропускается.
set -u
cd "$(dirname "$0")"
CC=${CC:-gcc}
UBSAN=(-O1 -fsanitize=undefined,float-cast-overflow)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0

# report <описание> <ожидаемая подстрока> <вывод>
report() {
    if grep -qF -- "$2" <<<"$3"; then
        echo "ok    $1"
    else
        echo "FAIL  $1: нет строки «$2»"
        echo "$3" | sed 's/^/      /'
        fail=1
    fi
}

# check <описание> <ожидаемая подстрока> <файл.c> [флаги компилятора...]
check() {
    local what="$1" expect="$2" src="$3"
    shift 3
    if ! "$CC" "$@" -o "$tmp/a.out" "$src" -lm 2>"$tmp/cc.log"; then
        echo "FAIL  $what: не собирается"
        sed 's/^/      /' "$tmp/cc.log"
        fail=1
        return
    fi
    report "$what" "$expect" "$("$tmp/a.out" 2>&1)"
}

# check_asm <описание> <есть|нет> <подстрока> <компилятор> <файл.c> [флаги...]
check_asm() {
    local what="$1" mode="$2" needle="$3" cc="$4" src="$5"
    shift 5
    local asm
    asm=$("$cc" "$@" -S -o - "$src" 2>&1)
    if grep -qF -- "$needle" <<<"$asm"; then found=есть; else found=нет; fi
    if [ "$found" = "$mode" ]; then
        echo "ok    $what"
    else
        echo "FAIL  $what: «$needle» ожидалось: $mode"
        echo "$asm" | grep -v '^\s*\.' | sed 's/^/      /'
        fail=1
    fi
}

check "uint8_t + uint8_t считается в int"  "a + b > 255"                          promote.c -O2
check "uint16_t * uint16_t переполняет int" "signed integer overflow: 65535 * 65535" promote.c "${UBSAN[@]}"
check "~ над uint8_t даёт int"             "~m = 0xfffffff0"                      promote.c -O2
check "сдвиг на 32 на x86_64"               "1u << 32 = 1"                         divshift.c -O2
check "(int32_t)3e9f под UBSan"            "3e+09 is outside the range"           float_cast.c "${UBSAN[@]}"
check "(int32_t)3e9f на x86_64"            "(int32_t)3e9f = -2147483648"          float_cast.c -O2
check "(int32_t)NAN на x86_64"             "(int32_t)NAN = -2147483648"           float_cast.c -O2
check "(uint32_t)-5.7f на x86_64"          "(uint32_t)-5.7f = 4294967291"         float_cast.c -O2

check_asm "GNU C сливает a * b + c в FMA"  есть vfmadd "$CC" fma.c -O2 -mfma
check_asm "ISO C не сливает a * b + c"     нет  vfmadd "$CC" fma.c -O2 -mfma -std=c11

# Cortex-M: тот же исходник, другие ответы на неопределённое поведение.
ARM_CC=arm-none-eabi-gcc
M4F=(-mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16)
if command -v "$ARM_CC" >/dev/null && command -v qemu-system-arm >/dev/null \
    && "$ARM_CC" --specs=rdimon.specs -mcpu=cortex-m3 -mthumb -o "$tmp/probe.elf" \
        m-start.c -x c - <<<'int main(void) { return 0; }' 2>/dev/null; then
    # run_m <машина> <файл.c> [флаги...]: сборка с semihosting и запуск в QEMU.
    run_m() {
        local machine="$1" src="$2"
        shift 2
        "$ARM_CC" -O2 "$@" --specs=rdimon.specs -Wl,--section-start=.vectors=0 \
            -o "$tmp/m.elf" "$src" m-start.c -lm 2>&1 &&
            timeout 20 qemu-system-arm -M "$machine" -nographic -monitor none \
                -semihosting-config enable=on,target=native -kernel "$tmp/m.elf" 2>&1
    }
    out=$(run_m mps2-an386 float_cast.c "${M4F[@]}")
    report "(int32_t)3e9f на Cortex-M4F"    "(int32_t)3e9f = 2147483647"  "$out"
    report "(int32_t)NAN на Cortex-M4F"     "(int32_t)NAN = 0"            "$out"
    report "(uint32_t)-5.7f на Cortex-M4F"  "(uint32_t)-5.7f = 0"         "$out"
    out=$(run_m mps2-an385 float_cast.c -mcpu=cortex-m3 -mthumb)
    report "(int32_t)3e9f на Cortex-M3"     "(int32_t)3e9f = 2147483647"  "$out"
    report "(int32_t)NAN на Cortex-M3"      "(int32_t)NAN = 0"            "$out"
    report "(uint32_t)-5.7f на Cortex-M3"   "(uint32_t)-5.7f = 0"         "$out"
    out=$(run_m mps2-an385 divshift.c -mcpu=cortex-m3 -mthumb)
    report "сдвиг на 32 на Cortex-M3"       "1u << 32 = 0"                "$out"
    report "INT_MIN / -1 на Cortex-M3"      "INT_MIN / -1 = -2147483648"  "$out"
    report "деление на 0 на Cortex-M3"      "7 / 0 = 0"                   "$out"
    check_asm "GCC для Cortex-M4F сливает в vfma" есть vfma.f32 "$ARM_CC" fma.c -O2 "${M4F[@]}"
else
    echo "skip  Cortex-M: нужны arm-none-eabi-gcc с newlib и qemu-system-arm"
fi

exit $fail
