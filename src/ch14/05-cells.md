# 14.5. Внутренняя изменяемость

Правило главы 4: через `&T` значение не меняется. Это не пожелание, а
обещание, на которое опирается оптимизатор: параметр `&T` rustc
помечает в LLVM как `noalias readonly` (раздел
[4.5](../ch04/05-noalias.md)), и компилятор вправе не перечитывать значение, даже
если между двумя чтениями вызвана неизвестная функция. Но счётчик
`Rc`, флаг тени регистров, кэш вычисленного значения — всё это нужно
менять, имея только общую ссылку. *Внутренняя изменяемость* — изменение
через `&` — в Rust законна в одном месте: внутри `UnsafeCell<T>`. `Cell`,
`RefCell`, `OnceCell`, атомики и `Mutex` построены на нём.

## Что будет без UnsafeCell

Пример `code/ch14/examples/sneaky.rs` пишет в значение через указатель,
полученный из `&u32`:

```rust
{{#include ../../code/ch14/examples/sneaky.rs:8:25}}
```

```console
$ cargo run -p ch14 --example sneaky
twice(&a) = 100
$ cargo run --release -p ch14 --example sneaky
twice(&a) = 2
```

Отладочная сборка перечитала `*r` после вызова и сложила 1 и 99.
Release поверил, что за `&u32` ничего не меняется, и сложил первое
чтение с самим собой. Ни один из ответов не «правильный»: программа с
неопределённым поведением может напечатать что угодно, и тест
`cargo_observed_sneaky` фиксирует только то, что сделал rustc 1.97.
Miri называет ошибку по имени:

```console
$ cargo +nightly miri run -p ch14 --example sneaky
error: Undefined Behavior: attempting a write access using <496> at alloc206[0x0], but that tag only grants SharedReadOnly permission for this location
  --> code/ch14/examples/sneaky.rs:12:14
   |
12 |     unsafe { *(p as *mut u32) = 99 }
   |              ^^^^^^^^^^^^^^^^^^^^^ this error occurs as part of an access at alloc206[0x0..0x4]
…
help: <496> was created by a SharedReadOnly retag at offsets [0x0..0x4]
  --> code/ch14/examples/sneaky.rs:18:12
   |
18 |     sneaky(r);
   |            ^
```

Прямую запись через приведение ссылки — `*(x as *const u32 as *mut u32)
+= 1` — компилятор отвергает сам, линтом `invalid_reference_casting`,
который по умолчанию запрещён: «assigning to `&T` is undefined
behavior, consider using an `UnsafeCell`». Через функцию с сырым
указателем, как в примере, линт уже не видит.

<div class="box c">

**Как в C.** `const int *` в C — обещание функции не писать через этот
указатель, а не обещание, что значение не изменится: кто-то другой
может держать неконстантный указатель на тот же `int`. Поэтому
компилятор C обязан перечитать значение после вызова. Файл
`code/ch14/c/twice.c` — та же функция, что `ch14_ref_twice` ниже, и GCC
13 и clang 18 с `-O2` читают `*x` дважды (это проверяет `run.sh`). Код
clang совпадает с кодом Rust для `&Cell<i32>` инструкция в
инструкцию. `restrict` дал бы C то же право, что `&` в Rust, но
программист ставит его сам, и компилятор не проверяет, правда ли это.

</div>

## Что меняет UnsafeCell

```rust
{{#include ../../code/ch14/src/codegen.rs:twice}}
```

```console
$ cargo asm -p ch14 --lib --simplify ch14_ref_twice
ch14_ref_twice:
	push rbx
	mov ebx, dword ptr [rdi]
	call rsi
	add ebx, ebx
	mov eax, ebx
	pop rbx
	ret
$ cargo asm -p ch14 --lib --simplify ch14_cell_twice
ch14_cell_twice:
	push rbp
	push rbx
	push rax
	mov rbx, rdi
	mov ebp, dword ptr [rdi]
	call rsi
	add ebp, dword ptr [rbx]
	mov eax, ebp
	add rsp, 8
	pop rbx
	pop rbp
	ret
```

Через `&i32` значение прочитано один раз, и результат — удвоенное
первое чтение: `f` не может изменить значение за общей ссылкой. Через
`&Cell<i32>` компилятор сохранил адрес в `rbx`, который переживает
вызов, и прочитал значение снова: `f` вправе держать ту же ссылку на
ячейку и записать в неё. На Cortex-M3 то же самое:

```console
$ cargo asm -p ch14 --lib --simplify --no-default-features --features alloc --target thumbv7m-none-eabi ch14_ref_twice
ch14_ref_twice:
	push {r4, r6, r7, lr}
	add r7, sp, #8
	ldr r4, [r0]
	blx r1
	lsls r0, r4, #1
	pop {r4, r6, r7, pc}
$ cargo asm -p ch14 --lib --simplify --no-default-features --features alloc --target thumbv7m-none-eabi ch14_cell_twice
ch14_cell_twice:
	push {r4, r5, r7, lr}
	add r7, sp, #8
	mov r4, r0
	ldr r5, [r0]
	blx r1
	ldr r0, [r4]
	add r0, r5
	pop {r4, r5, r7, pc}
```

`lsls r0, r4, #1` — сдвиг на один разряд, удвоение единственного
чтения. Тест `rustc_shared_ref_read_once` проверяет число загрузок для
x86-64, Cortex-M3 и Cortex-M0.

`UnsafeCell<T>` хранит `T` и даёт одну операцию: `get(&self) -> *mut T`.
Указатель можно получить через общую ссылку, а что с ним делать —
отвечает программист, в блоке `unsafe`. Для компилятора байты внутри
`UnsafeCell` — исключение из правила `noalias readonly`: через `&` на
тип, содержащий ячейку, они могут измениться. Документация `std`
гарантирует, что `UnsafeCell<T>` и `Cell<T>` имеют ту же раскладку в
памяти, что `T`: ячейка ничего не добавляет в память, она меняет только
то, что компилятору разрешено предполагать. В IR release-сборки
параметр `x` у `ch14_ref_twice` помечен `noalias noundef readonly align
4 dereferenceable(4)`, а у `ch14_cell_twice` — только `noundef nonnull
readonly align 4`: ни `noalias`, ни `dereferenceable`. `readonly` там
вывел сам LLVM — эта функция через `x` не пишет, но кто-то другой может.

## Свой Cell

Безопасная обёртка над `UnsafeCell` должна сама обеспечить то, что
компилятор больше не проверяет: никто не держит ссылку внутрь значения,
пока его меняют. `Cell` решает это радикально — ссылок внутрь не
выдаёт вовсе, только копирует значение наружу и внутрь:

```rust
{{#include ../../code/ch14/src/cells.rs:mycell}}
```

`get` и `set` безопасны вместе: оба обращаются к значению на время
одной операции, а `UnsafeCell` делает `MyCell` не `Sync`, и два потока
одновременно его не тронут. Положить `MyCell` в `static` поэтому нельзя —
`static` виден всем потокам:

```console
error[E0277]: `Cell<u32>` cannot be shared between threads safely
 --> src/main.rs:2:14
  |
2 | static HITS: Cell<u32> = Cell::new(0);
  |              ^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
  |
  = help: the trait `Sync` is not implemented for `Cell<u32>`
  = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU32` instead
  = note: shared static variables must have a type that implements `Sync`
```

Для `MyCell` ошибка та же (doc-тест `compile_fail,E0277` в
`code/ch14/src/cells.rs`). Если обёртка всё-таки выдаст ссылку внутрь, а
потом изменит значение, это неопределённое поведение, даже если
ссылка лежит внутри `UnsafeCell`:

```rust
{{#include ../../code/ch14/tests/ub.rs:unsafecell_ref}}
```

```console
$ cargo +nightly miri test -p ch14 --test ub -- --ignored --exact write_while_ref_inside_cell
error: Undefined Behavior: trying to retag from <136362> for SharedReadOnly permission at alloc42283[0x0], but that tag does not exist in the borrow stack for this location
  --> code/ch14/tests/ub.rs:34:5
…
help: <136362> was created by a SharedReadOnly retag at offsets [0x0..0x4]
  --> code/ch14/tests/ub.rs:32:28
…
help: <136362> was later invalidated at offsets [0x0..0x4] by a write access
  --> code/ch14/tests/ub.rs:33:14
```

`UnsafeCell` разрешает менять значение за `&UnsafeCell<T>`, но не за
`&T`, полученной изнутри: такая ссылка — обычная общая ссылка со всеми
её обещаниями.

## Cell

`Cell<T>` — `MyCell` из `std` с методами на все случаи: `get` для
`Copy`-типов, `set`, `replace` и `take` для любых, `swap` двух ячеек,
`into_inner`. Порт, общий для нескольких задач одного потока, считает
отправленные кадры через `&self`:

```rust
{{#include ../../code/ch14/src/cells.rs:port}}
```

`take` возвращает значение и оставляет `Default` — прочесть и обнулить
одной операцией. Цена `Cell` — ноль: `x.set(x.get() + 1)` для
`&Cell<u32>` компилируется в одну инструкцию, ту же, что для `&mut
u32`:

```console
$ cargo asm -p ch14 --lib --simplify ch14_cell_inc
ch14_cell_inc:
	inc dword ptr [rdi]
	ret
```

<div class="box ref">

**Справка: инвариантность ячеек.** `Cell<T>` и `UnsafeCell<T>`
*инвариантны* по `T` (раздел [5.4](../ch05/04-variance.md)):
`Cell<&'static str>` нельзя передать туда, где ждут `Cell<&'a str>`.
Будь это можно, функция записала бы в ячейку ссылку `'a`, а владелец
ячейки прочёл бы её как `'static` после того, как `'a` кончилось.
Для `&T` такое сужение безопасно — через неё не пишут, — и `&T`
ковариантна.

</div>

## RefCell: проверка заимствований во время работы

`Cell` не даёт ссылок, и для `Vec` или `String` это неудобно: вектор
пришлось бы вынимать из ячейки целиком, чтобы добавить элемент.
`RefCell<T>` даёт ссылки, но проверяет правила главы 4 во время
работы. Рядом со значением лежит флаг: 0 — заимствований нет,
положительное число — столько читателей, −1 — писатель. `borrow()`
увеличивает флаг и возвращает `Ref<T>` — страж, который уменьшит флаг в
`drop`; `borrow_mut()` ставит −1 и возвращает `RefMut<T>`. Нарушение
правила — паника: `borrow_mut` при живом `Ref` или `RefMut` паникует с
`RefCell already borrowed`, `borrow` при живом `RefMut` — с `RefCell
already mutably borrowed`. `try_borrow` и `try_borrow_mut` вместо паники
возвращают `Err`.

Реестр подписчиков из `kref.c` на `RefCell` повторяет ошибку C
буквально — подписка из обработчика во время обхода:

```rust
{{#include ../../code/ch14/src/cells.rs:registry}}
```

```rust
{{#include ../../code/ch14/src/cells.rs:watchdog}}
```

```console
thread 'main' (1749) panicked at code/ch14/src/cells.rs:100:34:
RefCell already borrowed
```

Там, где C молча писал в освобождённую память, Rust остановился в
`subscribe` с точным местом. Ошибка та же, и исправление то же, что
нужно было в C: не держать заимствование списка, пока работает чужой
код. `set_period_fixed` берёт заимствование на один шаг — прочесть
обработчик, отпустить, вызвать, снова занять, чтобы посчитать вызов, —
и тест `registry_fixed_notifies_all` проверяет, что все три подписчика
получили по вызову. Длина списка прочитана заранее, а не в заголовке
`for`, и это не случайность: о временных значениях в заголовке цикла —
раздел [14.8](08-tricks.md).

Цена проверки — сравнение и ветка:

```console
$ cargo asm -p ch14 --lib --simplify ch14_refcell_inc
ch14_refcell_inc:
	cmp qword ptr [rdi], 0
	jne .LBB111_2
	inc dword ptr [rdi + 8]
	ret
.LBB111_2:
	push rax
	lea rdi, [rip + .Lanon.2a52286e3143c3c0aa495546361760e9.77]
	call qword ptr [rip + core::cell::panic_already_borrowed@GOTPCREL]
$ cargo asm -p ch14 --lib --simplify ch14_refcell_get
ch14_refcell_get:
	movabs rax, 9223372036854775807
	cmp qword ptr [rdi], rax
	jae .LBB110_2
	mov eax, dword ptr [rdi + 8]
	ret
.LBB110_2:
	push rax
	lea rdi, [rip + .Lanon.2a52286e3143c3c0aa495546361760e9.76]
	call qword ptr [rip + core::cell::panic_already_mutably_borrowed@GOTPCREL]
```

`*x.borrow_mut() += 1` — проверка, что флаг ноль, и инкремент значения
за флагом. Запись −1 во флаг и возврат нуля оптимизатор убрал: между
ними нет кода, который мог бы на флаг посмотреть. `borrow()` сравнивает
флаг с `isize::MAX` как беззнаковое: `jae` ловит сразу и писателя (−1
как беззнаковое — самое большое число), и переполнение числа
читателей. На Cortex-M3 проверка флага превратилась в блок условных
инструкций `itttt eq` — без единого перехода на быстром пути:

```console
$ cargo asm -p ch14 --lib --simplify --no-default-features --features alloc --target thumbv7m-none-eabi ch14_refcell_inc
ch14_refcell_inc:
	ldr r1, [r0]
	cmp r1, #0
	itttt eq
	ldreq r1, [r0, #4]
	addeq r1, #1
	streq r1, [r0, #4]
	bxeq lr
```

Флаг занимает слово: `RefCell<u32>` — 16 байт на x86-64, `RefCell<u8>` —
тоже 16, `RefCell<[u64; 2]>` — 24 (тест `observed_cell_sizes`; раскладку
`RefCell` документация не обещает). Пример `bench` выше показал цену
на деле: 2,24 нс за инкремент через `RefCell` против 2,07 через `Cell`.

## Что выбрать

| Нужно | Тип |
|---|---|
| Менять маленькое `Copy`-значение через `&`: счётчик, флаг | `Cell<T>` |
| Менять что угодно через `&` в одном потоке, со ссылками внутрь | `RefCell<T>` |
| Вычислить значение один раз при первом обращении | `OnceCell<T>`, `LazyCell<T>` (раздел [14.6](06-lazy.md)) |
| Общий счётчик или флаг для потоков | атомики `AtomicU32` и др. (глава 21) |
| Общее изменяемое значение для потоков | `Mutex<T>`, `RwLock<T>` (глава 20), `OnceLock<T>`, `LazyLock<T>` |
| Своя структура с особыми правилами | `UnsafeCell<T>` и `unsafe` с доказательством |

`Rc<RefCell<T>>` — общее изменяемое значение в одном потоке: `Rc` даёт
несколько владельцев, `RefCell` — изменение через общую ссылку. Пара
удобна и потому опасна: она возвращает в программу ту самую
неконтролируемую общую изменяемость, от которой спасают правила
заимствования, только ошибка теперь — паника во время работы, а не
отказ компиляции. Если структура данных позволяет обойтись владением
и `&mut`, так и стоит сделать; арена с индексами из задачи
[14.C](09-tasks.md#c-двусвязный-список-дважды) — один из способов.

<div class="box check">

**Проверь себя.** Почему `Cell<T>::get` требует `T: Copy`, а `take` —
только `T: Default`? Что напечатает `ch14_cell_twice`, если `f`
запишет в ту же ячейку 10, а ячейка до вызова хранила 5? Почему
`borrow()` проверяет флаг через `jae` с `isize::MAX`, а не просто
`js` — переход по знаку?

</div>
