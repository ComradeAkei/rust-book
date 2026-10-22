#!/usr/bin/env bash
# Задача 1.A на C: собирает casts.c разными компиляторами и уровнями
# оптимизации, печатает сводную таблицу и сверяет ячейки, на которые
# опирается разбор в приложении E. Нужны gcc и clang; если есть
# arm-none-eabi-gcc с newlib и qemu-system-arm, добавляются столбцы для
# Cortex-M3 и Cortex-M4F.
set -u
cd "$(dirname "$0")"
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
fail=0
cols=()

# build <столбец> <команда сборки...>: собирает и запускает, вывод в файл.
build() {
    local name="$1"
    shift
    if "$@" -o "$tmp/$name" 2>"$tmp/$name.log"; then
        "$tmp/$name" | awk '/^ *[0-9]+ /{print $NF}' >"$tmp/$name.txt"
        cols+=("$name")
    else
        echo "skip  $name: не собирается"
        sed 's/^/      /' "$tmp/$name.log"
    fi
}

for o in O0 O2; do
    build "gcc-$o" gcc -std=c17 -$o -w casts.c -lm
    # clang не принимает 'я' в одинарных кавычках: берём char16_t.
    build "clang-$o" clang -std=c17 -$o -w "-DYA=u'я'" casts.c -lm
done

M=(--specs=rdimon.specs -Wl,--section-start=.vectors=0 ../../../code/ch01/c/m-start.c -lm)
if command -v arm-none-eabi-gcc >/dev/null && command -v qemu-system-arm >/dev/null; then
    # run_m <столбец> <машина> <флаги...>: прошивка с semihosting в QEMU.
    run_m() {
        local name="$1" machine="$2"
        shift 2
        if arm-none-eabi-gcc -std=c17 -O2 -w "$@" casts.c "${M[@]}" -o "$tmp/$name.elf" 2>/dev/null; then
            timeout 20 qemu-system-arm -M "$machine" -nographic -monitor none \
                -semihosting-config enable=on,target=native -kernel "$tmp/$name.elf" 2>&1 |
                awk '/^ *[0-9]+ /{print $NF}' >"$tmp/$name.txt"
            cols+=("$name")
        fi
    }
    run_m m3-O2 mps2-an385 -mcpu=cortex-m3 -mthumb
    run_m m4f-O2 mps2-an386 -mcpu=cortex-m4 -mthumb -mfloat-abi=hard -mfpu=fpv4-sp-d16
fi

gcc -std=c17 -O0 -w casts.c -lm -o "$tmp/names"
"$tmp/names" | awk '/^ *[0-9]+ /{n = $1; $1 = ""; $NF = ""; printf "%2d%s\n", n, $0}' >"$tmp/names.txt"
(
    printf '%-38s' ''
    printf ' %21s' "${cols[@]}"
    echo
    paste -d'|' "$tmp/names.txt" $(printf "$tmp/%s.txt " "${cols[@]}") |
        awk -F'|' '{printf "%-38s", $1; for (i = 2; i <= NF; i++) printf " %21s", $i; print ""}'
) | sed '14i\--- во время выполнения'

# cell <столбец> <строка 1..24> <значение>: проверка одной ячейки.
cell() {
    local got
    [ -f "$tmp/$1.txt" ] || return 0
    got=$(sed -n "$2p" "$tmp/$1.txt")
    if [ "$got" != "$3" ]; then
        echo "FAIL  $1, строка $2: $got, а в приложении E $3"
        fail=1
    fi
}
cell gcc-O2 4 255            # константа: gcc насыщает при свёртке
cell clang-O2 4 0            # константа: clang получает 0
cell gcc-O2 15 -2147483648   # NaN во время выполнения на x86_64
cell gcc-O2 16 0             # 1e10 во время выполнения на x86_64
cell gcc-O2 17 255           # -1.9 во время выполнения на x86_64
cell gcc-O2 22 0             # (uint64_t)(float)UINT64_MAX у gcc
cell clang-O2 22 9223372036854775808
cell gcc-O2 23 143           # 'я' как многосимвольная константа
cell m4f-O2 15 0
cell m4f-O2 16 255
cell m4f-O2 17 0
cell m4f-O2 22 18446744073709551615
exit $fail
