/* Задача 26.B: один сегмент PT_LOAD, в который попадают и заголовки
   ELF: файл читается в память целиком, начиная с нулевого байта. */
ENTRY(_start)

PHDRS
{
  /* FLAGS(5) — R и X: код и строка, писать некуда. */
  text PT_LOAD FILEHDR PHDRS FLAGS(5);
}

SECTIONS
{
  /* Классический адрес загрузки x86-64; код идёт сразу за заголовками. */
  . = 0x400000 + SIZEOF_HEADERS;
  .text : { *(.text .text.*) *(.rodata .rodata.*) } :text

  /* .comment — имена компоновщика и rustc, .eh_frame — таблицы
     раскрутки, которой нет, .note — Build ID и прочее. */
  /DISCARD/ : { *(.comment) *(.eh_frame*) *(.note*) }
}
