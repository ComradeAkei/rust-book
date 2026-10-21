# Задача 0.D: путь Cortex-M4F от вектора сброса до main.
#
# Запуск (из каталога firmware; QEMU ждёт отладчик на порту 3333):
#   qemu-system-arm -cpu cortex-m4 -machine mps2-an386 -nographic \
#       -semihosting-config enable=on,target=native -S -gdb tcp::3333 \
#       -kernel target/thumbv7em-none-eabihf/release/ch00-reset &
#   gdb-multiarch -q -batch -x ../solutions/ch00/reset.gdb \
#       target/thumbv7em-none-eabihf/release/ch00-reset
#
# Сценарий не зависит от адресов: точки останова ставятся на символы, а
# моменты инициализации ловятся точками наблюдения (watchpoint) на самих
# переменных и на регистре CPACR.

set pagination off
set confirm off
set output-radix 16
target remote :3333

# ANCHOR: reset
echo \n=== 1. Сразу после сброса\n
info registers pc sp
echo Таблица векторов: начальный SP, Reset, NMI, HardFault\n
x/4xw 0
# ANCHOR_END: reset

# ANCHOR: garbage
echo \n=== 2. ОЗУ до инициализации\n
# QEMU обнуляет ОЗУ при старте, настоящий кристалл нет. Запишем мусор, чтобы
# увидеть, что .bss обнуляет именно Reset, а не эмулятор.
set var ch00_reset::BOOTS = 0xdeadbeef
print/x ch00_reset::BOOTS
print/x ch00_reset::SIGNATURE
print/x *(unsigned int *)0xE000ED88
# ANCHOR_END: garbage

# ANCHOR: watch
watch *&ch00_reset::BOOTS
watch *&ch00_reset::SIGNATURE
watch *(unsigned int *)0xE000ED88
break main

echo \n=== 3. Обнуление .bss\n
continue
x/i $pc - 2
info symbol $pc

echo \n=== 4. Копирование .data из флеш-памяти\n
continue
x/i $pc - 2
print/x $r2 - 4

echo \n=== 5. Включение FPU: CPACR.CP10 и CP11 = 0b11\n
continue
x/3i $pc

echo \n=== 6. main\n
delete 1 2 3
continue
backtrace
info registers sp
# ANCHOR_END: watch

kill
