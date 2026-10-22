#!/usr/bin/env bash
# Прогоняет все прошивки глав 0, 3 и 6 в QEMU и сверяет вывод с ожидаемым.
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

# rungdb <описание> <ожидаемая подстрока> <пакет> <бинарник> <сценарий gdb>
# Запускает прошивку под QEMU с остановкой на первой инструкции и проходит
# её сценарием gdb (задача 0.D). Только Cortex-M4F на mps2-an386.
rungdb() {
    local what="$1" expect="$2" pkg="$3" bin="$4" script="$5"
    if ! command -v gdb-multiarch >/dev/null; then
        echo "skip  $what: нет gdb-multiarch"
        return
    fi
    cargo build --release -q -p "$pkg" --bin "$bin" "${M4F[@]}" || { fail=1; return; }
    local elf="${CARGO_TARGET_DIR:-target}/thumbv7em-none-eabihf/release/$bin"
    qemu-system-arm -cpu cortex-m4 -machine mps2-an386 -nographic \
        -semihosting-config enable=on,target=native -S -gdb tcp::3333 -kernel "$elf" >/dev/null 2>&1 &
    local qemu=$!
    sleep 1
    local out
    out=$(timeout 60 gdb-multiarch -q -batch -x "$script" "$elf" 2>&1)
    kill "$qemu" 2>/dev/null
    wait "$qemu" 2>/dev/null
    if grep -qF -- "$expect" <<<"$out"; then
        echo "ok    $what"
    else
        echo "FAIL  $what: нет строки «$expect»"
        echo "$out" | sed 's/^/      /'
        fail=1
    fi
}

M0=(--target thumbv6m-none-eabi)
M4F=(--target thumbv7em-none-eabihf)

run "M0: один исходник, CPUID"             "Cortex-M0 (CPUID = 0x410cc200)" -p ch00-hello "${M0[@]}"
run "M3: один исходник, CPUID"             "Cortex-M3 (CPUID = 0x410fc231)" -p ch00-hello
run "M4F: один исходник, CPUID"            "Cortex-M4 (CPUID = 0x410fc240)" -p ch00-hello "${M4F[@]}"
run "M4F: .data и .bss до main"            "SIGNATURE = 0x54454c45"     -p ch00-reset --bin ch00-reset "${M4F[@]}"
run "M4F: FPU закрыт, UsageFault NOCP"     "UFSR.NOCP"                  -p ch00-reset --bin nocp "${M4F[@]}"
rungdb "M4F: gdb от сброса до main"        "New value = 0xf00000"       ch00-reset ch00-reset ../solutions/ch00/reset.gdb

run "M3: стек сборки блока по значению"    "через &mut:  crc = 0x2871b, стек 76 байт" -p ch03-stack --bin stack
run "M3: переполнение стека портит .bss"   "COUNTERS[63] = 0xeeeeeeee"  -p ch03-stack --bin overflow
run "M3: стек ниже ОЗУ, lockup"            "Lockup"                     -p ch03-stack --bin lockup
run "M3: страж PRIMASK, let _ и вложенность" "drop(outer), inner жив:  PRIMASK = 0, тиков 5" -p ch03-irq
run "M0: страж PRIMASK, let _ и вложенность" "вложенная, NaiveGuard: PRIMASK = 0, тиков 4" -p ch03-irq "${M0[@]}"

run "UART PL011, таблица размеров на M3"  "offsets Sample: [4, 0, 5]"  -p ch06-uart
run "M3: LDR по нечётному адресу"          "прочитано 0x44332211"       -p ch06-unaligned --bin ldr
run "M3: LDRD по нечётному адресу"         "UFSR.UNALIGNED"             -p ch06-unaligned --bin ldrd
run "M3: LDR с UNALIGN_TRP"                "UFSR.UNALIGNED"             -p ch06-unaligned --bin trap
run "M3: core::fmt с UNALIGN_TRP"          "UFSR.UNALIGNED"             -p ch06-unaligned --bin trap_fmt
run "M0: LDR по нечётному адресу"          "HardFault на Cortex-M0"     -p ch06-unaligned --bin ldr "${M0[@]}"
run "M0: LDRD отсутствует"                 "инструкции LDRD в ARMv6-M нет" -p ch06-unaligned --bin ldrd "${M0[@]}"
run "M0: UNALIGN_TRP всегда включён"       "UNALIGN_TRP = true"         -p ch06-unaligned --bin trap "${M0[@]}"

exit $fail
