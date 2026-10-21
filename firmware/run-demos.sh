#!/usr/bin/env bash
# Прогоняет все прошивки главы 6 в QEMU и сверяет вывод с ожидаемым.
# Запуск: ./run-demos.sh (из каталога firmware). Используется и в CI.
set -u
cd "$(dirname "$0")"
fail=0

# run <описание> <ожидаемая подстрока> <аргументы cargo run...>
run() {
    local what="$1" expect="$2"
    shift 2
    local out
    out=$(timeout 60 cargo run --release -q "$@" 2>&1 | grep -v 'Timer with period zero')
    if grep -qF -- "$expect" <<<"$out"; then
        echo "ok    $what"
    else
        echo "FAIL  $what: нет строки «$expect»"
        echo "$out" | sed 's/^/      /'
        fail=1
    fi
}

M0=(--target thumbv6m-none-eabi)

run "UART PL011, таблица размеров на M3"  "offsets Sample: [4, 0, 5]"  -p ch06-uart
run "M3: LDR по нечётному адресу"          "прочитано 0x44332211"       -p ch06-unaligned --bin ldr
run "M3: LDRD по нечётному адресу"         "UFSR.UNALIGNED"             -p ch06-unaligned --bin ldrd
run "M3: LDR с UNALIGN_TRP"                "UFSR.UNALIGNED"             -p ch06-unaligned --bin trap
run "M3: core::fmt с UNALIGN_TRP"          "UFSR.UNALIGNED"             -p ch06-unaligned --bin trap_fmt
run "M0: LDR по нечётному адресу"          "HardFault на Cortex-M0"     -p ch06-unaligned --bin ldr "${M0[@]}"
run "M0: LDRD отсутствует"                 "инструкции LDRD в ARMv6-M нет" -p ch06-unaligned --bin ldrd "${M0[@]}"
run "M0: UNALIGN_TRP всегда включён"       "UNALIGN_TRP = true"         -p ch06-unaligned --bin trap "${M0[@]}"

exit $fail
