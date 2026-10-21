/* Общая карта памяти для двух машин QEMU.
 * LM3S6965: 256 КиБ flash, 64 КиБ RAM. nRF51822 (micro:bit): 256 КиБ flash,
 * 16 КиБ RAM. Берём меньшее, чтобы один скрипт подходил обеим. */
MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 256K
  RAM   : ORIGIN = 0x20000000, LENGTH = 16K
}
