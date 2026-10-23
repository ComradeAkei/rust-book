#!/usr/bin/env bash
# Прогоняет все прошивки глав 0, 3, 5, 6, 7, 8, 9, 10, 11 и 12 в QEMU и сверяет вывод с ожидаемым.
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

# symbols <описание> <есть|нет> <подстрока> <пакет> <бинарник>
# Ищет подстроку в именах символов прошивки для Cortex-M3 (раздел 7.10).
symbols() {
    local what="$1" want="$2" needle="$3" pkg="$4" bin="$5"
    if ! command -v llvm-nm >/dev/null; then
        echo "skip  $what: нет llvm-nm"
        return
    fi
    cargo build --release -q -p "$pkg" --bin "$bin" || { fail=1; return; }
    local elf="${CARGO_TARGET_DIR:-target}/thumbv7m-none-eabi/release/$bin" found=нет
    if llvm-nm -C "$elf" | grep -qF -- "$needle"; then found=есть; fi
    if [ "$found" = "$want" ]; then
        echo "ok    $what"
    else
        echo "FAIL  $what: «$needle» ожидалось: $want"
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

run "M3: 'static во flash и в ОЗУ"        "&GAINS (const)           0x000017d2  flash" -p ch05-addr
run "M0: 'static во flash и в ОЗУ"        "singleton! буфер         0x20000010  ОЗУ" -p ch05-addr "${M0[@]}"

run "UART PL011, таблица размеров на M3"  "offsets Sample: [4, 0, 5]"  -p ch06-uart
run "M3: LDR по нечётному адресу"          "прочитано 0x44332211"       -p ch06-unaligned --bin ldr
run "M3: LDRD по нечётному адресу"         "UFSR.UNALIGNED"             -p ch06-unaligned --bin ldrd
run "M3: LDR с UNALIGN_TRP"                "UFSR.UNALIGNED"             -p ch06-unaligned --bin trap
run "M3: core::fmt с UNALIGN_TRP"          "UFSR.UNALIGNED"             -p ch06-unaligned --bin trap_fmt
run "M0: LDR по нечётному адресу"          "HardFault на Cortex-M0"     -p ch06-unaligned --bin ldr "${M0[@]}"
run "M0: LDRD отсутствует"                 "инструкции LDRD в ARMv6-M нет" -p ch06-unaligned --bin ldrd "${M0[@]}"
run "M0: UNALIGN_TRP всегда включён"       "UNALIGN_TRP = true"         -p ch06-unaligned --bin trap "${M0[@]}"

run "M3: место паники из PanicInfo"        "паника в ch07-panic/src/lib.rs:24:5" -p ch07-panic --bin report
run "M3: сообщение паники"                 "index out of bounds: the len is 4 but the index is 4" -p ch07-panic --bin report
run "M0: сообщение паники"                 "index out of bounds: the len is 4 but the index is 4" -p ch07-panic --bin report "${M0[@]}"
run "M3: паника без отчёта"                "канал прочитан"             -p ch07-panic --bin silent

run "M3: реакция на ошибку кадра"          "искать маркер со следующего байта" -p ch07-frame --bin quiet
run "M3: цепочка ошибок кадра"             "  причина: CRC не совпадает: в кадре 0x5555, посчитано 0x9db4" -p ch07-frame --bin verbose
run "M0: цепочка ошибок кадра"             "ошибка: кадр со смещения 8 отвергнут" -p ch07-frame --bin verbose "${M0[@]}"
run "M3: причина без dyn Error"            "  причина: CRC не совпадает: в кадре 0x5555, посчитано 0x9db4" -p ch07-frame --bin direct
symbols "M3: без сообщений нет core::fmt"  нет "core::fmt" ch07-frame quiet
symbols "M3: без dyn Error нет Debug"      нет "FrameError as core::fmt::Debug" ch07-frame direct
symbols "M3: dyn Error тянет Debug"        есть "FrameError as core::fmt::Debug" ch07-frame verbose

run "M3: ch08_bump на LDREX/STREX"         "ch08_bump (LDREX/STREX): было 41, стало 42" -p ch08-sections --bin target
run "M0: ch08_bump с запретом прерываний"  "ch08_bump (запрет прерываний): было 41, стало 42" -p ch08-sections --bin target "${M0[@]}"
run "M0: cfg! в build.rs — про хост"       "cfg!(target_arch = \"arm\") = false" -p ch08-sections --bin target "${M0[@]}"
run "M3: таблица из sensors.csv"           "датчик 12 battery    ×2 mV" -p ch08-sections --bin target
run "M3: .uninit переживает сброс"         "загрузка 2: .uninit 0x2000000c, RUNS = 0" -p ch08-sections --bin boot
run "M0: .uninit переживает сброс"         "загрузка 2: .uninit 0x2000000c, RUNS = 0" -p ch08-sections --bin boot "${M0[@]}"
run "M4F: .uninit переживает сброс"        "загрузка 2: .uninit 0x2000000c, RUNS = 0" -p ch08-sections --bin boot "${M4F[@]}"

run "M3: SysTick через трейт Register"     "SysTick: 3 периода по 100000 тактов отсчитаны" -p ch09-systick
run "M0: SysTick через трейт Register"     "SysTick: 3 периода по 100000 тактов отсчитаны" -p ch09-systick "${M0[@]}"
run "M4F: SysTick через трейт Register"    "SysTick: 3 периода по 100000 тактов отсчитаны" -p ch09-systick "${M4F[@]}"

run "M3: Uart<S> не больше ссылки"         "Uart<Enabled>: 4 байта, Uart<Disabled>: 4 байта, &Pl011: 4 байта" -p ch10-typestate
run "M3: disable и повторный enable"       "снова включён: CR = 0x301" -p ch10-typestate

run "M3: &dyn Sensor — два слова"          "&dyn Sensor: 8 байт, &Thermo: 4 байта" -p ch11-dyn
run "M3: объект на стеке, таблица во flash" "датчик = 3: 4 байта, данные 0x2000" -p ch11-dyn
run "M3: таблицы методов во flash"         "данных во flash: 2 из 3, таблиц во flash: 3 из 3" -p ch11-dyn
run "M0: таблицы методов во flash"         "данных во flash: 2 из 3, таблиц во flash: 3 из 3" -p ch11-dyn "${M0[@]}"
run "M4F: таблицы методов во flash"        "данных во flash: 2 из 3, таблиц во flash: 3 из 3" -p ch11-dyn "${M4F[@]}"

run "M3: размер замыкания — захваченное"   "|| x + 1: 4 байта, move || x + 1: 4 байта, || n += 1 (u64): 4 байта" -p ch12-closures
run "M3: таблица обработчиков во flash"    "во flash" -p ch12-closures
run "M3: бит Thumb в адресах функций"      "адресов с битом Thumb: 4 из 4" -p ch12-closures
run "M0: бит Thumb в адресах функций"      "адресов с битом Thumb: 4 из 4" -p ch12-closures "${M0[@]}"
run "M4F: бит Thumb в адресах функций"     "адресов с битом Thumb: 4 из 4" -p ch12-closures "${M4F[@]}"
run "M3: разбор кадра комбинаторами"       "parse::decode совпал с decode: 3 из 3" -p ch12-closures
run "M0: разбор кадра комбинаторами"       "parse::decode совпал с decode: 3 из 3" -p ch12-closures "${M0[@]}"

exit $fail
