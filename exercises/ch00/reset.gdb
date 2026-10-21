# Задача 0.D. Сценарий gdb: от вектора сброса до первой строки main.
#
# Прошивка: firmware/ch00-reset, цель thumbv7em-none-eabihf, машина QEMU
# mps2-an386 (Cortex-M4F). Запуск (из каталога firmware):
#
#   cargo build --release -p ch00-reset --target thumbv7em-none-eabihf
#   qemu-system-arm -cpu cortex-m4 -machine mps2-an386 -nographic \
#       -semihosting-config enable=on,target=native -S -gdb tcp::3333 \
#       -kernel target/thumbv7em-none-eabihf/release/ch00-reset &
#   gdb-multiarch -q -x ../exercises/ch00/reset.gdb \
#       target/thumbv7em-none-eabihf/release/ch00-reset
#
# Ключ -S останавливает процессор до первой инструкции, -gdb открывает порт
# для отладчика. Допишите сценарий так, чтобы он показал:
#   1. начальные SP и PC и откуда процессор их взял;
#   2. момент, когда обнуляется .bss (переменная BOOTS);
#   3. момент, когда .data копируется из флеш-памяти (SIGNATURE), и адрес,
#      откуда пришло значение;
#   4. момент, когда включается FPU (регистр CPACR, 0xE000ED88);
#   5. остановку в main.
# Сценарий не должен зависеть от конкретных адресов кода: они меняются от
# любой правки прошивки.

set pagination off
target remote :3333
info registers pc sp
