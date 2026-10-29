# 6.3. enum как размеченное объединение

## Перечисления без данных

```rust
{{#include ../../code/ch06/src/enums.rs:c_style}}
```

`Mode` занимает 1 байт: компилятор выбирает самый узкий целый тип, в
который помещаются все значения тега. `repr(u8)` закрепляет этот тип
явно и разрешает задать значения, как в протоколе. Привести такой enum к
числу можно через `as`, а обратно нельзя: не каждый байт является
допустимым кодом. Обратное преобразование пишут явно:

```rust
{{#include ../../code/ch06/src/enums.rs:try_from}}
```

<div class="box trap">

**Ловушка.** Получить enum из числа через `transmute` — неопределённое
поведение, если число не совпадает ни с одним значением тега. Компилятор
считает недопустимые значения невозможными и может, например, выбросить
ветку `_` в `match` или использовать эти значения как [нишу][g-niche] для чужого
тега (раздел [6.4](04-niches.md)). `TryFrom` с `match` компилируется в одну-две
проверки и всегда корректен.

</div>

## Перечисления с данными

```rust
{{#include ../../code/ch06/src/enums.rs:tagged}}
```

`Message` — тип всех сообщений. `Message::Heartbeat` — одно значение
без дополнительных данных, а `Message::Pressure(101_325)` — одно
значение с числом внутри. Запись `Temperature { sensor: 1,
centi_celsius: 2150 }` хранит сразу два поля выбранного варианта:
номер датчика 1 и температуру 21,50 °C в сотых долях градуса.
Одно сообщение содержит ровно один из вариантов, а не все четыре
набора полей одновременно.

При разборе того же значения имя слева от `=>` получает содержимое:

```rust
let msg = Message::Pressure(101_325);
let pressure = match msg {
    Message::Pressure(pa) => Some(pa),
    _ => None,
};
assert_eq!(pressure, Some(101_325));
```

Здесь `Pressure(101_325)` *создаёт* значение, а `Pressure(pa)` в
[образце][g-pattern] *разбирает* его и называет число `pa`. Число не доступно как
`msg.pa`: у других вариантов такого поля нет. Ветка `_` выбрана здесь
намеренно: для всех сообщений, кроме давления, результат — `None`.

Это *размеченное объединение* (tagged union): тег, или *дискриминант*
(discriminant), говорит, какой вариант лежит в памяти, а данные
вариантов делят одно место. Раскладка, которую выбрал
rustc:

```text
print-type-size type: `enums::Message`: 8 bytes, alignment: 4 bytes
print-type-size     discriminant: 1 bytes
print-type-size     variant `Pressure`: 7 bytes
print-type-size         padding: 3 bytes
print-type-size         field `.0`: 4 bytes, alignment: 4 bytes
print-type-size     variant `Temperature`: 3 bytes
print-type-size         field `.sensor`: 1 bytes
print-type-size         field `.centi_celsius`: 2 bytes
print-type-size     variant `Fault`: 3 bytes
print-type-size         padding: 1 bytes
print-type-size         field `.code`: 2 bytes, alignment: 2 bytes
print-type-size     variant `Heartbeat`: 0 bytes
```

```text
байт          0     1       2   3      4   5   6   7
Temperature  [tag][sensor][centi    ]
Pressure     [tag][  ··· pad ···   ][      pa       ]
Fault        [tag][pad  ][code     ]
```

Поля `Temperature` начинаются сразу за тегом, в байте 1: компилятор
раскладывает каждый вариант отдельно, и выравнивание `u32` из `Pressure`
не мешает плотно уложить `Temperature`.

<div class="box c">

**Как в C.** То же самое на C пишут вручную:

```c
enum msg_kind { HEARTBEAT, TEMPERATURE, PRESSURE, FAULT };

struct message {
    enum msg_kind kind;     /* 4 байта на x86_64, 1 на arm-none-eabi */
    union {
        struct { uint8_t sensor; int16_t centi_celsius; } temperature;
        uint32_t pressure;
        struct { uint16_t code; } fault;
    } u;
};

uint32_t describe(const struct message *m)
{
    switch (m->kind) {
    case TEMPERATURE: return (uint32_t)m->u.pressure;   /* опечатка: не то поле */
    case PRESSURE:    return m->u.pressure;
    }                                                   /* HEARTBEAT и FAULT забыты */
    return 0;
}
```

Компилятор не связывает тег с полем объединения: чтение не того поля
компилируется без единого предупреждения, а `-Wswitch` замечает
пропущенные варианты только у `switch` без `default`. Структура занимает
8 байт, как и `Message`, но лишь потому, что здесь повезло с размерами.

</div>

В Rust добраться до данных можно только через сопоставление с [образцом][g-pattern],
и `match` обязан покрыть все варианты:

```rust
{{#include ../../code/ch06/src/enums.rs:describe}}
```

<div class="box hood">

**Под капотом.** `match` по плотным значениям тега превращается в [таблицу
переходов][g-jump-table].

x86_64: байт тега, загрузка смещения из таблицы, косвенный переход.

```text
ch06_describe:
	movzx eax, byte ptr [rdi]
	lea rcx, [rip + .LJTI0_0]
	movsxd rdx, dword ptr [rcx + 4*rax]
	add rdx, rcx
	jmp rdx
	movsx eax, word ptr [rdi + 2]          ; Temperature: centi_celsius из байт 2..4
	ret
	mov eax, dword ptr [rdi + 4]           ; Pressure: u32 из байт 4..8
	ret
	movzx eax, word ptr [rdi + 2]          ; Fault: code
	or eax, -2147483648                    ; | 0x8000_0000
	ret
```

thumbv7m: у Cortex-M3 есть специальная инструкция `tbb` (table branch
byte): таблица байтовых смещений лежит прямо за ней, переход занимает одну
инструкцию.

```text
ch06_describe:
	push {r7, lr}
	mov r7, sp
	mov r1, r0
	ldrb r0, [r0]
	tbb [pc, r0]
.LBB0_2:
	ldrsh.w r0, [r1, #2]
	pop {r7, pc}
.LBB0_3:
	ldr r0, [r1, #4]
	pop {r7, pc}
.LBB0_4:
	ldrh r0, [r1, #2]
	add.w r0, r0, #-2147483648
.LBB0_5:
	pop {r7, pc}
```

В [ARMv6-M][g-armv7m] (`thumbv6m`) инструкции `tbb` нет, поэтому LLVM строит таблицу
адресов и прыгает через `mov pc, r2`. Пары `push {r7, lr}` / `pop {r7, pc}`
в каждой функции для Cortex-M — это [пролог][g-prologue] с указателем кадра: в описании
целей `thumb*-none-eabi` указано `"frame-pointer": "always"`, чтобы
отладчик и [паника][g-panic] могли пройти по стеку.

Вариант `Heartbeat` возвращает 0, и для него нет отдельной ветки: в `r0`
к этому моменту уже лежит тег, равный 0.

</div>

## repr(C) и repr(u8) для enum с данными

Раскладка `repr(Rust)` для enum не обещана, а иногда нужна точная: чтобы
C-код прочитал сообщение без копирования. Для этого есть два атрибута,
их раскладка описана в RFC 2195:

```rust
{{#include ../../code/ch06/src/enums.rs:repr_c_enum}}
```

- `repr(C)`: структура из тега и объединения структур вариантов. Ровно та
  `struct message`, что выше. Тег имеет тот же размер, что `enum` в C на
  этой цели, а он разный: на x86_64 Linux это `int` (4 байта), а в ABI для
  голого железа ARM (arm-none-eabi) enum сжимается до наименьшего типа, в
  который помещаются значения. rustc повторяет это правило: в описании
  целей `thumb*-none-eabi` стоит `"c-enum-min-bits": 8`.
- `repr(u8)` (и любой другой целый тип): объединение структур вида
  `repr(C) struct { tag: u8, поля варианта... }`. Тег занимает 1 байт, и
  данные вариантов начинаются после него с учётом выравнивания.

Оба атрибута гарантируют, что первым в памяти лежит тег. Тест
`repr_c_enum_uses_int_tag` читает его через указатель: для `repr(u8)` это
законно, для `repr(Rust)` нет.

<div class="box hood">

**Под капотом.** Одна и та же проверка на двух целях:

```rust
{{#include ../../code/ch06/src/asm_demo.rs:repr_c_tag}}
```

x86_64 сравнивает 32-битный тег:

```text
ch06_is_pressure_c:
	cmp dword ptr [rdi], 2
	sete al
	ret
```

Cortex-M3 читает один байт. Сравнение с 2 LLVM сделал без перехода:
`clz` от нуля даёт 32, а `32 >> 5 == 1`; от любого другого числа `clz`
меньше 32, и сдвиг даёт 0.

```text
ch06_is_pressure_c:
	push {r7, lr}
	mov r7, sp
	ldrb r0, [r0]
	subs r0, #2
	clz r0, r0
	lsrs r0, r0, #5
	pop {r7, pc}
```

Размер `MessageC` при этом на обеих целях 8 байт, потому что объединение
вариантов выровнено на 4. Но ширина тега различается, 4 байта против
1, и общий заголовочный файл для прошивки и хоста с
таким enum читает на хосте мусор в старших байтах тега. Для обмена между
разными машинами `repr(C)` enum не годится, нужен `repr(u8)` или явное
кодирование.

</div>

<div class="box trap">

**Ловушка.** Хочется «просто посмотреть на байты» enum через `transmute`
в массив. Байты выравнивания и неиспользованные байты коротких вариантов
не инициализированы, и чтение их как `u8` — неопределённое поведение.
Компилятор пропустит такой код, а Miri нет:

```rust,ignore
#[derive(Clone, Copy)]
enum Message { Heartbeat, Pressure(u32) }

let bytes: [u8; 8] = unsafe { std::mem::transmute(Message::Heartbeat) };
```

```text
error: Undefined Behavior: constructing invalid value of type [u8; 8]: at [4],
       encountered uninitialized memory, but expected an integer
  --> src/main.rs:11:35
```

Обратите внимание на `at [4]`: байты 1..4 оказались инициализированы.
Здесь rustc расширил тег до 4 байт, чтобы он занял место перед `u32`, и
так ему проще сравнивать тег. Это ещё один пример того, что раскладка
`repr(Rust)` не обязана быть такой, как вы ожидаете. Чтобы получить байты,
кодируйте значение явно, как в разделе [6.11](11-project.md).

</div>

<div class="box check">

**Проверь себя.**

1. Сколько байт займёт `enum E { A(u8), B(u16), C(u32, u8) }` и почему не
   12?
2. Почему тег `repr(C)` enum на x86_64 занимает 4 байта, а на
   thumbv7m 1 байт? Что выберете для кадра, который читают обе стороны?
3. Компилятор знает, что значение `Opcode` лежит в `{1, 2, 16, 17}`. Какие
   оптимизации это открывает и чем это опасно при `transmute`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-armv7m]: ../appendix-f.md#armv7m "ARMv6-M, ARMv7-M, ARMv7E-M: Версии архитектуры ARM для микроконтроллеров."
[g-jump-table]: ../appendix-f.md#jump-table "Таблица переходов (jump table): Массив адресов или смещений веток, по которому match выполняет косвенный переход после одной проверки границы."
[g-niche]: ../appendix-f.md#niche "Ниша (niche): Битовый шаблон, который не встречается ни у одного значения типа: 0 у ссылки и NonZeroU32, 2–255 у bool, суррогаты и числа больше 0x10FFFF у char."
[g-panic]: ../appendix-f.md#panic "Паника (panic): Аварийная реакция на ошибку в самой программе: индекс за границей, unwrap на None, переполнение в отладочной сборке."
[g-pattern]: ../appendix-f.md#pattern "Образец (pattern): Описание формы значения в match, let, if let и параметрах функций: литералы, диапазоны, альтернативы |, разбор кортежей, структур и срезов, привязки имён."
[g-prologue]: ../appendix-f.md#prologue "Пролог и эпилог (prologue, epilogue): Начальные и конечные команды функции: пролог сохраняет callee-saved регистры и адрес возврата и выделяет кадр стека, эпилог восстанавливает их и возвращается."
