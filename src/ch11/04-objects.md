# 11.4. Объекты в куче и без неё

## ?Sized

У каждого параметра типа есть неявное ограничение `Sized`: функция из
главы 10 рассчитывает, что значение `S` можно положить на стек и
скопировать. `dyn Sensor` размера не имеет, и обобщённая функция его не
примет, даже если берёт `&S`:

```text
error[E0277]: the size for values of type `dyn Sensor` cannot be known at compilation time
 --> h.rs:9:11
  |
9 |     twice(d)
  |     ----- ^ doesn't have a size known at compile-time
  |     |
  |     required by a bound introduced by this call
  |
  = help: the trait `Sized` is not implemented for `dyn Sensor`
note: required by an implicit `Sized` bound in `twice`
 --> h.rs:4:10
  |
4 | fn twice<S: Sensor>(s: &S) -> i32 {
  |          ^ required by the implicit `Sized` requirement on this type parameter in `twice`
help: consider relaxing the implicit `Sized` restriction
  |
4 | fn twice<S: Sensor + ?Sized>(s: &S) -> i32 {
  |                    ++++++++
```

`?Sized` снимает ограничение: «`S` может и не иметь размера». Тогда `S`
можно держать только за указателем, зато функция принимает и
конкретные типы, и объекты:

```rust
{{#include ../../code/ch11/src/object.rs:unsized}}
```

`report(&t)` и `report(d)` — две копии: для `Thermo` с встроенными
`name` и `read` и для `dyn Sensor` с двумя вызовами через таблицу. Так
написаны функции стандартной библиотеки, которым всё равно, что за
указателем: `size_of_val<T: ?Sized>`, `Box<T: ?Sized>`, `Rc<T:
?Sized>`.

У `Self` в трейте неявного `Sized` нет: внутри `trait Sensor` тип
`Self` может оказаться `dyn Sensor`. Поэтому `where Self: Sized` из
раздела [11.3](03-compat.md) — настоящее ограничение, а не повтор
очевидного.

## Пересылающие реализации

`dyn Sensor` реализует `Sensor`, но `&dyn Sensor` — нет: это другой тип,
указатель. Обобщённый код, который берёт датчик по значению, ссылку не
примет. Чинит это одна реализация на все ссылки:

```rust
{{#include ../../code/ch11/src/object.rs:forward}}
```

`S: ?Sized` здесь обязателен: без него реализация покрыла бы `&Thermo`,
но не `&dyn Sensor`. Теперь `sum_all` принимает массив `[&dyn Sensor;
2]`, массив `[&Thermo; 2]` и вектор `Box<dyn Sensor>`, для которого
ниже есть такая же реализация. Писать её можно потому, что трейт
`Sensor` свой: [правило сироты][g-orphan-rule] из раздела [9.4](../ch09/04-coherence.md)
не даёт реализовать чужой трейт для `&S`. Стандартная библиотека делает
то же для своих трейтов: `impl<W: Write + ?Sized> Write for &mut W`,
`impl<I: Iterator + ?Sized> Iterator for &mut I`, `impl<T: Display +
?Sized> Display for &T` и такие же для `Box`. Благодаря второй из них
у `&mut dyn Iterator` вызываются `map` и `filter`, которые у самого
`dyn Iterator` исключены через `where Self: Sized`.

## `Box<dyn Sensor>`

`Box<dyn Sensor>` [владеет][g-ownership] объектом в куче. Это толстый указатель, как
`&dyn Sensor`, но `drop` для него зовёт деструктор из таблицы и
возвращает память (раздел [11.2](02-vtable.md)):

```rust
{{#include ../../code/ch11/src/object.rs:box}}
```

В векторе лежат три толстых указателя по 16 байт на x86-64, а сами
датчики — в куче, по аллокации на каждый: два байта для `Thermo`,
четыре для `Counter`. `Box<Thermo>` превращается в `Box<dyn Sensor>`
тем же приведением без размера, что и ссылка: адрес в куче не меняется,
к нему добавляется таблица. Тест `unsized_and_forwarding` проверяет
`sum_all(station())`: 2150 + 7 − 250.

<div class="box trap">

**Ловушка.** Пересылающая реализация для `Box` меняет смысл
`&Box<dyn Sensor>`. Привести его к `&dyn Sensor` можно двумя путями:
разыменовать `Box` или сделать объектом сам `Box`, раз он теперь тоже
датчик. Компилятор пробует приведение без размера раньше разыменования
и выбирает второй путь:

```rust
{{#include ../../code/ch11/src/object.rs:double}}
```

`boxed_sizes()` возвращает `(16, 2)` на x86-64 и `(8, 2)` на Cortex-M:
в первом случае за объектом стоит `Box`, и `read` идёт через две
таблицы — `Box<dyn Sensor>` и `Thermo`. Так же выходит в цикле
`for s in &v { object_size(s) }` по вектору `Vec<Box<dyn Sensor>>`:
`s` там — `&Box<dyn Sensor>`. Явное `&**s` или `s.as_ref()` даёт
объект с данными датчика. [Clippy][g-clippy] предупреждает о соседнем случае,
`&Box<T>` в типе параметра (`borrowed_box`), а приведение в аргументе
не замечает.

</div>

## `Rc<dyn Sensor>`

`Rc<T>` указывает на блок в куче, где перед значением лежат два
счётчика, сильных и слабых ссылок. Для `Rc<dyn Sensor>` смещение
значения в блоке зависит от выравнивания типа, а тип известен только
таблице:

```rust
{{#include ../../code/ch11/src/object.rs:rc}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_rc_read
ch11_rc_read:
	mov rax, qword ptr [rdi]
	mov rcx, qword ptr [rdi + 8]
	mov rdx, qword ptr [rcx + 16]
	dec rdx
	and rdx, -16
	lea rdi, [rax + rdx]
	add rdi, 16
	jmp qword ptr [rcx + 24]
$ cargo asm -p ch11 --lib --simplify --no-default-features --features alloc --target thumbv7m-none-eabi ch11_rc_read
ch11_rc_read:
	push {r7, lr}
	mov r7, sp
	ldrd r0, r1, [r0]
	ldrd r1, r2, [r1, #8]
	subs r1, #1
	bic r1, r1, #7
	add r0, r1
	adds r0, #8
	pop.w {r7, lr}
	bx r2
```

Функция получает `&Rc<dyn Sensor>` — тонкий указатель на толстый, и
сначала читает оба его слова: адрес блока и таблицу. Затем — слово 2
таблицы, выравнивание. Счётчики занимают 16 байт на x86-64, и значение
начинается со смещения `16 + ((align − 1) & !15)`, то есть с 16,
округлённого вверх до выравнивания: для `Thermo` с выравниванием 2 — с
16, для типа с выравниванием 32 — с 32. На Cortex-M3 счётчики — два
четырёхбайтовых слова, отсюда `bic r1, r1, #7` и `adds r0, #8`, а
`ldrd` одной инструкцией читает слова 2 и 3 таблицы: выравнивание и
адрес `read`.

`shared()` создаёт один датчик и две ссылки на него: `Rc::new` строит
`Rc<Thermo>`, и приведение превращает его в `Rc<dyn Sensor>` без
копирования. Для потоков то же делает `Arc<dyn Sensor + Send + Sync>`.

## Обработчик без кучи

В прошивке без распределителя `Box` нет, но объект за `&mut dyn` от
кучи не зависит. Обход отсчётов с обработчиком тревоги:

```rust
{{#include ../../code/ch11/src/object.rs:callback}}
```

[Замыкание][g-closure] `|_, s| worst = worst.max(s)` лежит на стеке
`ch11_scan_max`, а `scan` получает на него толстый указатель. Функция
`scan` одна на все обработчики:

```console
$ cargo asm -p ch11 --lib --simplify ch11::object::scan
ch11::object::scan:
	push rbp
	…
	mov qword ptr [rsp], rcx
	…
	mov rbx, qword ptr [r8 + 32]
	xor r12d, r12d
	xor r13d, r13d
	jmp .LBB6_3
.LBB6_4:
	inc r13
	add r14, -2
	je .LBB6_5
.LBB6_3:
	movzx edx, word ptr [r15 + 2*r13]
	cmp dx, bp
	jle .LBB6_4
	inc r12
	mov rdi, qword ptr [rsp]
	mov rsi, r13
	call rbx
	jmp .LBB6_4
	…
```

Срез — `rdi:rsi`, порог — `edx`, обработчик — `rcx:r8`. Адрес метода
читается из таблицы один раз, до цикла: таблица неизменна (раздел
[11.2](02-vtable.md)). Слот 4 со смещением 32 — `call_mut`: у
`dyn FnMut` таблица из пяти слов, слот 3 занят `call_once` из
супертрейта [`FnOnce`][g-fn-traits]. Как устроены замыкания и их трейты, разбирает
глава 12. На Cortex-M3 четыре регистра аргументов заняты срезом,
порогом и адресом замыкания, и таблица приходит через стек:

```console
$ cargo asm -p ch11 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch11::object::scan
ch11::object::scan:
	push {r4, r5, r6, r7, lr}
	add r7, sp, #12
	…
	ldr r0, [r7, #8]
	…
	ldr.w r10, [r0, #16]
	…
.LBB4_3:
	ldrsh.w r2, [r9, r4, lsl #1]
	cmp r2, r6
	ble .LBB4_2
	mov r0, r8
	mov r1, r4
	blx r10
	…
```

`[r7, #8]` — первое слово аргументов на стеке, за сохранёнными `r7` и
`lr`. Смещение `call_mut` — 16, четыре слова по четыре байта.

А в `ch11_scan_max` косвенного вызова нет. `scan` и замыкание лежат в
одном крейте, LLVM встроил `scan` и увидел, какая таблица в него
пришла, — и раскрыл цикл на четыре отсчёта за проход:

```console
$ cargo asm -p ch11 --lib --simplify ch11_scan_max
ch11_scan_max:
	test rsi, rsi
	…
.LBB39_5:
	mov r8, rdi
	movzx edi, word ptr [rdi]
	cmp di, dx
	jle .LBB39_8
	cmp di, ax
	jle .LBB39_8
	mov eax, edi
	…
```

Это *девиртуализация* (devirtualization), и раздел [11.6](06-cost.md)
разбирает, когда она случается.

<div class="box c">

**Как в C.** Тот же обработчик в C — пара из указателя на функцию и
контекста:

```c
size_t scan(const int16_t *s, size_t n, int16_t limit,
            void (*on_alarm)(void *ctx, size_t i, int16_t v), void *ctx);
```

`qsort` из libc берёт только функцию, без контекста, и состояние
приходится держать в глобальной переменной; `qsort_r` из glibc
добавила `void *arg`. В Rust пару собирает компилятор, а замыкание
хранит своё состояние само: `worst` захвачен по `&mut`, и `scan` не
знает о нём ничего.

</div>

Выбор между `&mut dyn FnMut` и `impl FnMut` — тот же, что между `dyn` и
обобщением вообще. С `impl FnMut` у `scan` своя копия на каждое
замыкание, и каждая встроена; с `dyn` — одна копия и косвенный вызов на
каждую тревогу. Для обработчика, который зовётся редко, а функция с
циклом большая, `dyn` экономит flash, ничего не теряя во времени.

<div class="box check">

**Проверь себя.** Почему `impl<S: Sensor> Sensor for &S` без `?Sized`
не позволит вызвать `sum_all([d1, d2])` для `d1, d2: &dyn Sensor`? С
какого смещения начнётся значение в блоке `Rc<dyn Sensor>` на Cortex-M3,
если за объектом стоит тип с `#[repr(align(16))]`? Сколько вызовов через
таблицу выполнит `report(&b)` при `b: Box<dyn Sensor>`, и что изменится,
если убрать пересылающую реализацию для `Box`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-clippy]: ../appendix-f.md#clippy "clippy: Линтер из тулчейна Rust: несколько сотен линтов, от неидиоматичного кода до заведомых ошибок, разбитых на группы с уровнями allow, warn и deny."
[g-closure]: ../appendix-f.md#closure "Замыкание (closure): Функция вместе со своим контекстом — захваченными переменными."
[g-fn-traits]: ../appendix-f.md#fn-traits "Fn, FnMut, FnOnce: Трейты вызова, которые компилятор реализует для замыканий, функций и указателей на функции."
[g-orphan-rule]: ../appendix-f.md#orphan-rule "Правило сироты (orphan rule): Крейт может реализовать трейт для типа, только если свой хотя бы трейт или тип; точнее, среди типов реализации есть локальный, и до него не стоит непокрытый параметр."
[g-ownership]: ../appendix-f.md#ownership "Владение (ownership): Правило Rust: у каждого значения ровно один владелец — переменная, поле или элемент коллекции, — и он отвечает за уничтожение значения."
