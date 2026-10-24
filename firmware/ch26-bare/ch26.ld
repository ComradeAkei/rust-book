INCLUDE memory.x

ENTRY(Reset);
EXTERN(VECTORS);

_stack_start = ORIGIN(RAM) + LENGTH(RAM);

SECTIONS
{
  .vector_table ORIGIN(FLASH) :
  {
    KEEP(*(.vector_table));
  } > FLASH

  .text :
  {
    *(.text .text.*);
  } > FLASH

  .rodata : ALIGN(4)
  {
    *(.rodata .rodata.*);
    . = ALIGN(4);
  } > FLASH

  /* GUARD: изменяемым static здесь не место: код запуска этой прошивки
     не копирует .data и не обнуляет .bss. Секция собирает их, чтобы
     ASSERT в конце остановил компоновку. */
  .ram (NOLOAD) :
  {
    *(.data .data.*);
    *(.bss .bss.*);
  } > RAM
  /* END GUARD */

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
    *(.ARM.extab .ARM.extab.*);
  }
}

ASSERT(ADDR(.vector_table) == ORIGIN(FLASH), "таблица векторов должна начинаться с адреса 0");
/* GUARD */
ASSERT(SIZEOF(.ram) == 0, "ch26.ld: изменяемый static, а код запуска не готовит .data и .bss");
/* END GUARD */
