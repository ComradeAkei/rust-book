# 6.7. repr(packed) и невыровненный доступ

## Структура без выравнивания

`repr(packed)` убирает все байты выравнивания и делает выравнивание
структуры равным 1. Так описывают кадры, которые приходят из провода
байт за байтом:

```rust
{{#include ../../code/ch06/src/packed.rs:wire}}
```

`WireSample` занимает 6 байт, и `value` лежит по смещению 1. Значит, если
сама структура лежит по чётному адресу, `value` окажется по нечётному.
Есть ещё `repr(packed(N))`: выравнивание полей не больше N, например
`packed(2)` для протоколов, где поля выровнены на 2.

<div class="box ref">

**Справка: невыровненный доступ на трёх процессорах.**

*x86_64.* Обычные `mov` работают с любым адресом. Цена — лишнее обращение,
если значение пересекает границу строки кэша (64 байта) и особенно
границу страницы (4 КиБ). Исключение — часть [SSE][g-sse-avx]-инструкций вроде
`movaps`, требующих выравнивания на 16; компилятор выбирает их, только
если доказал выравнивание.

*Cortex-M0 и M0+ (ARMv6-M).* Любой невыровненный доступ к слову или
полуслову даёт HardFault. Отдельного [UsageFault][g-faults] в ARMv6-M нет, бит
`CCR.UNALIGN_TRP` всегда равен 1.

*Cortex-M3, M4, M7 (ARMv7-M).* Невыровненный доступ разрешён только для
`LDR`, `LDRH`, `LDRSH`, `STR`, `STRH` и их непривилегированных вариантов.
Инструкции `LDRD`, `STRD`, `LDM`, `STM`, `PUSH`, `POP` и эксклюзивный
доступ `LDREX`/`STREX` требуют выравнивания всегда и при нарушении
генерируют UsageFault с битом `UNALIGNED`. Если UsageFault не включён
(так после сброса), он повышается до HardFault. Бит `CCR.UNALIGN_TRP`
заставляет ловить и разрешённые случаи. Источник: ARMv7-M Architecture
Reference Manual, раздел A3.2.1.

</div>

## Опыт в QEMU

Проверим справку на эмуляторе. Прошивки лежат в `firmware/ch06-unaligned`.
Каждая выполняет одну инструкцию по нечётному адресу. Инструкция записана
встроенным ассемблером, потому что в самом Rust разыменование
невыровненного указателя — неопределённое поведение, и компилятор вправе
сгенерировать для него что угодно. Нам же нужна ровно одна конкретная
инструкция:

```rust
{{#include ../../firmware/ch06-unaligned/src/lib.rs:ldr}}
```

Обработчик HardFault печатает адрес инструкции и, на ARMv7-M, регистры
состояния ошибки CFSR и HFSR:

```rust
{{#include ../../firmware/ch06-unaligned/src/lib.rs:fault}}
```

Результаты на машинах `lm3s6965evb` (Cortex-M3) и `microbit` (Cortex-M0):

```console
$ cd firmware
$ cargo run --release -p ch06-unaligned --bin ldr
Cortex-M3 (ARMv7-M): LDR по адресу 0x000013d1
прочитано 0x44332211

$ cargo run --release -p ch06-unaligned --bin ldr --target thumbv6m-none-eabi
Cortex-M0 (ARMv6-M): LDR по адресу 0x00000dd9
HardFault на Cortex-M0 (ARMv6-M): PC = 0x000007ec

$ cargo run --release -p ch06-unaligned --bin ldrd
Cortex-M3 (ARMv7-M): LDRD по 0x00001480: 0x33221100 0x77665544
Cortex-M3 (ARMv7-M): LDRD по 0x00001481
HardFault на Cortex-M3 (ARMv7-M): PC = 0x00000d90
CFSR = 0x01000000, HFSR = 0x40000000
UFSR.UNALIGNED: невыровненный доступ
HFSR.FORCED: UsageFault выключен и повышен до HardFault

$ cargo run --release -p ch06-unaligned --bin trap
Cortex-M3 (ARMv7-M): CCR = 0x00000200, UNALIGN_TRP = false
включаем UNALIGN_TRP и повторяем LDR по 0x000014d9
HardFault на Cortex-M3 (ARMv7-M): PC = 0x000007b2
CFSR = 0x01000000, HFSR = 0x40000000
UFSR.UNALIGNED: невыровненный доступ
HFSR.FORCED: UsageFault выключен и повышен до HardFault

$ cargo run --release -p ch06-unaligned --bin trap --target thumbv6m-none-eabi
Cortex-M0 (ARMv6-M): CCR = 0x00000208, UNALIGN_TRP = true
включаем UNALIGN_TRP и повторяем LDR по 0x00000ed9
HardFault на Cortex-M0 (ARMv6-M): PC = 0x000002c8
```

Всё совпало со справкой: Cortex-M3 читает невыровненное слово через
`LDR`, но падает на `LDRD`, а Cortex-M0 падает сразу. Бит 24 в CFSR —
`UFSR.UNALIGNED`, бит 30 в HFSR — `FORCED`. У Cortex-M0 бит 3 в CCR
выставлен с момента сброса. Скрипт `firmware/run-demos.sh` запускает все
опыты и проверяет вывод, он же работает в CI.

Точные адреса зависят от версии rustc и от кода прошивки: любая правка
сдвигает код и данные во флеш-памяти. Совпадать с книгой должны
нечётность адресов и значения CFSR, HFSR и CCR, поэтому скрипт ищет в
выводе смысловые строки, а не адреса.

## Что генерирует компилятор

Rust никогда не даёт создать ссылку на невыровненное поле, но читать
поле packed-структуры по значению можно, и компилятор сам выбирает
подходящие инструкции:

```rust
{{#include ../../code/ch06/src/asm_demo.rs:field_access}}
```

<div class="box hood">

**Под капотом.** Одно и то же чтение `s.value` по смещению 1.

```text
x86_64:                         thumbv7m:
ch06_value_packed:              ch06_value_packed:
	mov eax, dword ptr [rdi + 1]	push {r7, lr}
	ret                         	mov r7, sp
	                            	ldr.w r0, [r0, #1]
	                            	pop {r7, pc}

thumbv6m:
ch06_value_packed:
	push {r7, lr}
	add r7, sp, #0
	ldrb r1, [r0, #1]
	ldrb r2, [r0, #2]
	lsls r2, r2, #8
	adds r1, r2, r1
	ldrb r2, [r0, #3]
	lsls r2, r2, #16
	ldrb r0, [r0, #4]
	lsls r0, r0, #24
	adds r0, r0, r2
	adds r0, r0, r1
	pop {r7, pc}
```

Для Cortex-M3 LLVM смело использует невыровненный `ldr.w`, а для
Cortex-M0 собирает слово из четырёх байтов. Разницу задаёт описание
цели: у `thumbv6m-none-eabi` в списке возможностей стоит `+strict-align`,
у `thumbv7m-none-eabi` нет.

```console
$ rustc +nightly -Z unstable-options --print target-spec-json --target thumbv6m-none-eabi | grep features
  "features": "+strict-align,+atomics-32",
```

А вот поле `u64` по смещению 1:

```rust
{{#include ../../code/ch06/src/asm_demo.rs:u64_packed}}
```

```text
thumbv7m, packed:              thumbv7m, repr(C):
ch06_ticks:                    ch06_ticks_aligned:
	push {r7, lr}              	push {r7, lr}
	mov r7, sp                 	mov r7, sp
	ldr.w r2, [r0, #1]         	ldrd r0, r1, [r0, #8]
	ldr.w r1, [r0, #5]         	pop {r7, pc}
	mov r0, r2
	pop {r7, pc}
```

Для выровненного поля LLVM читает оба слова одной `ldrd`. Для
невыровненного он знает, что `ldrd` упадёт (наш опыт `ldrd` выше), и
разбивает чтение на два `ldr.w`. Компилятор соблюдает правила процессора
сам, но только пока он знает настоящее выравнивание. Как только
выравнивание соврали, например приведением указателя, он сгенерирует
`ldrd`, и прошивка упадёт.

</div>

## Почему ссылка на поле запрещена

```rust,ignore
{{#include ../../code/ch06/src/packed.rs:e0793}}
```

Если вынести эту функцию вместе с `WireSample` в отдельный файл `e0793.rs`,
rustc ответит так:

```text
error[E0793]: reference to field of packed struct is unaligned
 --> e0793.rs:9:5
  |
9 |     &s.value
  |     ^^^^^^^^
  |
  = note: this struct is 1-byte aligned, but the type of this field may require higher alignment
  = note: creating a misaligned reference is undefined behavior (even if that reference is never dereferenced)
  = help: copy the field contents to a local variable, or replace the reference with a raw pointer
          and use `read_unaligned`/`write_unaligned` (loads and stores via `*p` must be properly aligned
          even when using raw pointers)
```

Ссылка `&u32` в Rust обещает выравнивание на 4, и компилятор использует
это обещание везде, куда эта ссылка попадёт: в другой функции, в другом
крейте, в `ldrd` на Cortex-M3. Поэтому невыровненная ссылка — UB в момент
создания, а не в момент чтения. Сообщение компилятора само подсказывает
два законных пути:

```rust
{{#include ../../code/ch06/src/packed.rs:copy_out}}
```

```rust
{{#include ../../code/ch06/src/packed.rs:raw_ptr}}
```

`&raw const s.value` создаёт сырой указатель без промежуточной ссылки, а
`read_unaligned` сообщает компилятору, что выравнивания нет. Это те же
четыре `ldrb` на Cortex-M0 и тот же `ldr.w` на Cortex-M3.

Но лучше всего не накладывать структуру на байты, а разбирать байты явно:

```rust
{{#include ../../code/ch06/src/packed.rs:from_bytes}}
```

Здесь нет `unsafe`, порядок байт задан явно, а оптимизатор превращает
`from_le_bytes` в одну загрузку там, где процессор её допускает.

<div class="box trap">

**Ловушка.** Хочется включить `UNALIGN_TRP` на Cortex-M3, чтобы ловить
невыровненные доступы ещё в отладке. Попробуйте:

```console
$ cargo run --release -p ch06-unaligned --bin trap_fmt
Cortex-M3 (ARMv7-M): включаем UNALIGN_TRP и печатаем число
десятичное: 1234567
шестнадцатеричное: HardFault на Cortex-M3 (ARMv7-M): PC = 0x00000efc
CFSR = 0x01000000, HFSR = 0x40000000
UFSR.UNALIGNED: невыровненный доступ
```

Падает сама печать `{:#010x}`, а не наш код. Дизассемблер показывает, где:

```text
$ llvm-objdump -d -C target/thumbv7m-none-eabi/release/trap_fmt | grep -B1 ' efc:'
     efa: e067         	b	0xfcc <core::fmt::write+0x1dc> @ imm = #0xce
     efc: f8d6 0001    	ldr.w	r0, [r6, #0x1]
```

Это `core::fmt::write`, и она читает 32-битное значение по смещению 1
одной инструкцией. Судя по тому, что `{}` прошёл, а `{:#010x}` нет, это
ветка, которая разбирает спецификацию с флагами и шириной. Для цели
`thumbv7m-none-eabi` это законно: LLVM знает, что `LDR` допускает
невыровненный адрес, и пользуется этим в стандартной библиотеке. Ловушка
меняет правила процессора, а уже скомпилированный код о ней не знает.
Если в обработчике HardFault не снять бит первым делом, его собственная
печать упадёт снова, и QEMU сообщит `Lockup: can't escalate 3 to
HardFault`. Именно так выглядел первый запуск этого опыта при написании
главы.

</div>

<div class="box c">

**Как в C.** `__attribute__((packed))` в GCC и `#pragma pack(1)` в MSVC
делают то же, что `repr(C, packed)`, но разрешают взять адрес поля.
Указатель `uint32_t *` на нечётный адрес становится бомбой замедленного
действия. GCC 9 добавил предупреждение `-Waddress-of-packed-member`, Clang
выдаёт его давно, но оба только предупреждают.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `let v = { s.value }; &v` законно, а `&s.value` нет?
2. Что сгенерирует LLVM для чтения `u16` по смещению 1 на thumbv6m и на
   thumbv7m?
3. Опыт `ldrd` сначала читает по выровненному адресу и только потом по
   нечётному. Зачем нужна первая половина опыта?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-faults]: ../appendix-f.md#faults "MemManage, BusFault, UsageFault: Отдельные исключения сбоев ARMv7-M: нарушение прав MPU или выполнение из запрещённой области, ошибка шины, неверная инструкция или состояние ядра (например, переход на адрес без бита Thumb)."
[g-sse-avx]: ../appendix-f.md#sse-avx "SSE, AVX (Streaming SIMD Extensions, Advanced Vector Extensions): Векторные расширения x86: SSE даёт 128-битные регистры xmm0–xmm15 и команды над ними (SSE2 есть у любого x86_64), AVX и AVX2 — 256-битные ymm, AVX-512 — 512-битные zmm."
