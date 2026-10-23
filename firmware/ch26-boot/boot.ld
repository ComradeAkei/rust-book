/* Задача 26.C: скрипт компоновки для своего кода запуска. Области памяти —
   из общего memory.x: одна карта на три машины QEMU. */
INCLUDE memory.x

ENTRY(Reset);
EXTERN(VECTORS);

/* Стек растёт вниз от конца ОЗУ; начальное значение SP — слово 0
   таблицы векторов. */
_stack_start = ORIGIN(RAM) + LENGTH(RAM);
/* Сколько ОЗУ оставить стеку как минимум. */
_stack_reserve = 2K;

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

  /* Изменяемые данные с начальным значением. Живут в ОЗУ (VMA), а их
     начальные значения лежат во flash сразу за .rodata (LMA): код запуска
     копирует __sidata.. в __sdata..__edata. */
  .data : ALIGN(4)
  {
    __sdata = .;
    *(.data .data.*);
    . = ALIGN(4);
    __edata = .;
  } > RAM AT > FLASH
  __sidata = LOADADDR(.data);

  /* Нули: в образе места не занимают, код запуска обнуляет. */
  .bss (NOLOAD) : ALIGN(4)
  {
    __sbss = .;
    *(.bss .bss.*);
    . = ALIGN(4);
    __ebss = .;
  } > RAM

  /* Не трогает никто: значения переживают программный сброс. */
  .uninit (NOLOAD) : ALIGN(4)
  {
    *(.uninit .uninit.*);
    . = ALIGN(4);
  } > RAM

  /DISCARD/ :
  {
    *(.ARM.exidx .ARM.exidx.*);
    *(.ARM.extab .ARM.extab.*);
  }
}

ASSERT(ADDR(.vector_table) == ORIGIN(FLASH), "boot.ld: таблица векторов должна начинаться с адреса 0");
ASSERT(SIZEOF(.vector_table) == 64, "boot.ld: таблица векторов — 16 слов");
ASSERT(__sdata % 4 == 0 && __edata % 4 == 0 && __sidata % 4 == 0, "boot.ld: .data не выровнена на слово");
ASSERT(__sbss % 4 == 0 && __ebss % 4 == 0, "boot.ld: .bss не выровнена на слово");
ASSERT(_stack_start - ADDR(.uninit) - SIZEOF(.uninit) >= _stack_reserve, "boot.ld: стеку не хватает ОЗУ");
