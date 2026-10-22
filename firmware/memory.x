/* Общая карта памяти для трёх машин QEMU.
 * LM3S6965: 256 КиБ flash, 64 КиБ RAM. nRF51822 (micro:bit): 256 КиБ flash,
 * 16 КиБ RAM. MPS2 AN386: по 4 МиБ SSRAM по тем же адресам 0x0 и 0x2000_0000.
 * Берём меньшее, чтобы один скрипт подходил всем. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 256K
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}
