# 11.2. Что лежит в таблице методов

## Таблица в ассемблере

Приведение `&Thermo` к `&dyn Sensor` ничего не вычисляет: к адресу
объекта добавляется адрес готовой таблицы.

```rust
{{#include ../../code/ch11/src/vtable.rs:as-dyn}}
```

`cargo asm` с ключом `--include-constants` показывает и саму таблицу
(директивы секций опущены):

```console
$ cargo asm -p ch11 --lib --include-constants ch11_as_dyn
ch11_as_dyn:
	mov rax, rdi
	lea rdx, [rip + .Lanon.da1beaeebe5f420a6faa69c57adbc3f3.5]
	ret
…
.Lanon.da1beaeebe5f420a6faa69c57adbc3f3.5:
	.asciz	"\000\000\000\000\000\000\000\000\002\000\000\000\000\000\000\000\002\000\000\000\000\000\000"
	.quad	ch11_read_thermo
	.quad	<ch11::fat::Thermo as ch11::fat::Sensor>::name
$ cargo asm -p ch11 --lib --include-constants --no-default-features --target thumbv7m-none-eabi ch11_as_dyn
…
.Lanon.0d26763af8d9de428da4ab91421b4eea.5:
	.asciz	"\000\000\000\000\002\000\000\000\002\000\000"
	.long	ch11_read_thermo
	.long	<ch11::fat::Thermo as ch11::fat::Sensor>::name
```

Первые три слова LLVM записал строкой: 23 байта и завершающий ноль
`.asciz` — ровно три восьмибайтовых слова на x86-64 и три
четырёхбайтовых на Cortex-M3 (там 11 байт и ноль). Таблица `dyn Sensor`
для `Thermo` у rustc 1.97 такая:

| Слово | Смещение x86-64 | Смещение M3 | Содержимое | Для `Thermo` |
|---|---|---|---|---|
| 0 | 0 | 0 | `drop_in_place::<T>` или ноль | 0: деструктора нет |
| 1 | 8 | 4 | `size_of::<T>()` | 2 |
| 2 | 16 | 8 | `align_of::<T>()` | 2 |
| 3 | 24 | 12 | `read` | `ch11_read_thermo` |
| 4 | 32 | 16 | `name` | `<Thermo as Sensor>::name` |

Методы идут в порядке объявления в трейте. В слоте `read` стоит
`ch11_read_thermo`, а не `<Thermo as Sensor>::read`: тела у них
одинаковые, и проход MergeFunctions из раздела [10.5](../ch10/05-mono.md)
оставил одну функцию под экспортируемым именем. В таблице лежит адрес, а
чьё имя у этого адреса, для вызова неважно.

Метод по умолчанию тоже занимает слот. У `Counter` свой `name` не
написан, и в слоте 4 его таблицы — `name` из трейта, скомпилированный
для `Counter`:

```rust
{{#include ../../code/ch11/src/vtable.rs:name}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_name_dyn
ch11_name_dyn:
	jmp qword ptr [rsi + 32]
```

Порядок и состав таблицы — наблюдение за rustc 1.97, а не обещание
языка: Reference раскладку таблицы не описывает, и при появлении
супертрейтов в ней появляются другие слова (раздел
[11.5](05-any.md)). Задача D читает таблицу напрямую и показывает, чем
это кончается под Miri.

<div class="box hood">

**Под капотом.** Таблица неизменна, и rustc сообщает это LLVM: загрузка
слова из таблицы помечена `!invariant.load`, а параметр-таблица —
`readonly dereferenceable(40)`, пять слов только для чтения (`cargo asm
--llvm` показывает это в IR). Два вызова подряд читают адрес `read` один
раз:

```rust
{{#include ../../code/ch11/src/vtable.rs:twice}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_read_twice
ch11_read_twice:
	push rbp
	push r14
	push rbx
	mov rbx, rdi
	mov r14, qword ptr [rsi + 24]
	call r14
	mov ebp, eax
	mov rdi, rbx
	call r14
	add eax, ebp
	pop rbx
	pop r14
	pop rbp
	ret
```

В `ops.c` из главы 9 компилятор C перечитывал `s->ops` перед вторым
вызовом: функция по указателю могла записать в объект другую таблицу.
Здесь указатель на таблицу лежит в регистре, а не в объекте, и `read`
до него не дотянется.

</div>

## Где лежит таблица

Таблица — константа, которую компилятор построил при компиляции. Если и
объекты — константы, вся конструкция обходится без кода инициализации:

```rust
{{#include ../../code/ch11/src/vtable.rs:statics}}
```

`SENSORS` — массив толстых указателей, четыре слова: адреса `THERMO` и
`PULSES` и адреса их таблиц. На x86-64 Linux эти данные попадают в
`.data.rel.ro`: программа собрана перемещаемой, и адреса в таблицах
загрузчик дописывает при запуске. На Cortex-M адреса известны при
компоновке, и всё остаётся в `.rodata`, то есть во flash. Прошивка
`firmware/ch11-dyn` печатает, где что лежит:

```rust
{{#include ../../firmware/ch11-dyn/src/main.rs:main}}
```

```console
$ cd firmware && cargo run --release -q -p ch11-dyn
&dyn Sensor: 8 байт, &Thermo: 4 байта
термодатчик = 2150: 2 байта, данные 0x0000175c во flash, таблица 0x00001730 во flash
датчик = 12: 4 байта, данные 0x00001758 во flash, таблица 0x00001744 во flash
датчик = 3: 4 байта, данные 0x20003f78 в RAM, таблица 0x000016f4 во flash
данных во flash: 2 из 3, таблиц во flash: 3 из 3
```

Счётчик на стеке живёт в RAM, но таблица его типа всё равно во flash:
её строит компилятор, а не программа. Те же строки печатают прошивки для
Cortex-M0 и Cortex-M4F, адреса у них свои. `firmware/run-demos.sh`
проверяет итоговую строку на всех трёх машинах.

У адресов таблиц есть подробность. Оба `Counter` — одного типа, а
таблицы у них по разным адресам, `0x1744` и `0x16f4`. `llvm-objdump -s`
показывает по обоим адресам одни и те же 20 байт: ноль, 4, 4, адреса
`read` и `name`. Первую таблицу построил крейт `ch11` для `SENSORS`,
вторую — крейт прошивки для `&local`, и даже LTO не слил копии.

## Одна пара «тип — трейт», несколько таблиц

Язык не обещает, что таблица для пары «тип — трейт» одна. rustc строит
её в каждой единице кодогенерации, где она нужна, и копии остаются
разными объектами. Пример `code/ch11/examples/copies.rs` приводит один и
тот же `&Thermo` к `&dyn Sensor` дважды: в `main` и в обобщённой функции,
которая мономорфизуется отдельно.

```console
$ cargo run -q -p ch11 --example copies
ptr::addr_eq: true
ptr::eq:      false
$ cargo run -q -p ch11 --example copies --release
ptr::addr_eq: true
ptr::eq:      true
$ cargo +nightly miri run -q -p ch11 --example copies
ptr::addr_eq: true
ptr::eq:      false
```

В отладочной сборке крейт делится на множество единиц кодогенерации (по
умолчанию до 256), и копия `as_dyn::<Thermo>` попала не в ту, где
`main`, — со своей таблицей. Так делит крейт инкрементальная сборка,
обычная для `cargo run`; с `CARGO_INCREMENTAL=0`, как в CI, единиц не
больше 16, и отладочная сборка тоже печатает `ptr::eq: true`. В release
`as_dyn` встроена в `main`, и таблица одна. Miri нарочно может выдать
каждому приведению свою таблицу: так он ловит код, который полагается
на их единственность. `ptr::eq` для толстых указателей сравнивает оба
слова, `ptr::addr_eq` — только адреса данных. Сравнение `==` двух
`*const dyn Trait` компилятор отмечает предупреждением:

```text
warning: ambiguous wide pointer comparison, the comparison includes metadata which may not be expected
 --> src/main.rs:8:20
  |
8 |     println!("{}", x == y);
  |                    ^^^^^^
  |
  = note: `#[warn(ambiguous_wide_pointer_comparisons)]` on by default
help: use `std::ptr::addr_eq` or untyped pointers to only compare their addresses
```

<div class="box trap">

**Ловушка.** «Тот же ли это объект» для `&dyn` проверяют через
`ptr::addr_eq`. А «того же ли он типа» по адресу таблицы не проверить
вовсе: две таблицы одного типа могут лежать по разным адресам. Для типа
есть `Any` и `TypeId` (раздел [11.5](05-any.md)).

</div>

## drop и освобождение памяти

Деструктор объекта, которого не видно в типе, тоже приходится звать
через таблицу, и `Box<dyn Sensor>` берёт оттуда же размер и
выравнивание, чтобы вернуть память распределителю:

```rust
{{#include ../../code/ch11/src/vtable.rs:drop}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_drop_box
ch11_drop_box:
	push r15
	push r14
	push rbx
	mov r14, rsi
	mov rbx, rdi
	mov rax, qword ptr [rsi]
	test rax, rax
	je .LBB29_2
	mov rdi, rbx
	call rax
.LBB29_2:
	mov rsi, qword ptr [r14 + 8]
	test rsi, rsi
	je .LBB29_7
	mov rdx, qword ptr [r14 + 16]
	mov rdi, rbx
	pop rbx
	pop r14
	pop r15
	jmp qword ptr [rip + __rustc::__rust_dealloc@GOTPCREL]
.LBB29_7:
	pop rbx
	pop r14
	pop r15
	ret
	mov r15, rax
	mov rsi, qword ptr [r14 + 8]
	…
	call _Unwind_Resume@PLT
```

Слово 0 — деструктор: если там не ноль, он вызывается с адресом данных.
Слово 1 — размер: у типа нулевого размера `Box` памяти не выделял, и
освобождать нечего. Слова 1 и 2 — размер и выравнивание для
`__rust_dealloc(ptr, size, align)`. Хвост после `ret` — площадка
раскрутки: если деструктор запаникует, память всё равно вернётся, и
паника пойдёт дальше через `_Unwind_Resume`. На Cortex-M, где паника
останавливает программу, хвоста нет:

```console
$ cargo asm -p ch11 --lib --simplify --no-default-features --features alloc --target thumbv7m-none-eabi ch11_drop_box
ch11_drop_box:
	push {r4, r5, r7, lr}
	add r7, sp, #8
	mov r5, r1
	ldr r1, [r1]
	mov r4, r0
	cmp r1, #0
	itt ne
	movne r0, r4
	blxne r1
	ldr r1, [r5, #4]
	cbz r1, .LBB28_2
	ldr r2, [r5, #8]
	mov r0, r4
	pop.w {r4, r5, r7, lr}
	b __rustc::__rust_dealloc
.LBB28_2:
	pop {r4, r5, r7, pc}
```

`itt ne` делает условными две следующие инструкции: деструктор
вызывается, только если слово 0 не ноль, без перехода в обход.

<div class="box c">

**Как в C.** В `fops.c` за освобождение отвечает метод `release`, и
каждый драйвер сам вызывает `free` для своего типа, а `release` и
`free` легко рассинхронизировать. Ядро Linux поступает так же: память
объекта освобождает метод драйвера, а `kfree` узнаёт размер из
распределителя. В Rust освобождение — общая для всех типов процедура,
которой нужны только три первых слова таблицы.

</div>

<div class="box check">

**Проверь себя.** Сколько слов в таблице `dyn Sensor` для любого типа и
от чего это зависит? Что окажется в слоте `drop` для `Logged` и для
`Window`? Почему `ch11_drop_box` проверяет размер, а не адрес, чтобы
решить, звать ли `__rust_dealloc`? Сколько байт flash займут таблицы,
если прошивка приводит к `dyn Sensor` пять разных типов в трёх
крейтах?

</div>
