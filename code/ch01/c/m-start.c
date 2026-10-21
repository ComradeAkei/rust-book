/* Минимальный старт C-программы для QEMU mps2-an385 (Cortex-M3) и
 * mps2-an386 (Cortex-M4F): таблица векторов на адресе 0, включение FPU и
 * переход в _start из newlib. Вывод идёт через semihosting (rdimon).
 * Начальный SP — верх 4 МиБ ОЗУ по адресу 0x2000_0000 (раздел 0.6). */
extern void _start(void);

static void reset(void)
{
#ifdef __ARM_FP
    *(volatile unsigned *)0xE000ED88 |= 0xFu << 20; /* CPACR: CP10, CP11 */
    __asm volatile("dsb\n isb");
#endif
    _start();
}

__attribute__((section(".vectors"), used))
void (*const vectors[2])(void) = {(void (*)(void))0x20400000, reset};
