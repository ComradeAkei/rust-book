# 11.5. Any и приведение к супертрейту

## Проверка типа во время работы

Объект трейта забывает тип: через `&dyn Sensor` видны только методы
`Sensor`. Иногда тип нужно вернуть — достать из реестра узел нужного
класса, разобрать сообщение, которое пришло как `Box<dyn Any + Send>`
из запаниковавшего потока (раздел [7.5](../ch07/05-panic.md)). Для этого
в `core::any` есть трейт `Any` с одним методом, `type_id(&self) ->
TypeId`, и обобщённой реализацией для каждого типа `T: 'static`. У
`dyn Any` поверх него написаны `is`, `downcast_ref` и `downcast_mut`, а
у `Box<dyn Any>` — `downcast`:

```rust
{{#include ../../code/ch11/src/any.rs:downcast}}
```

`describe(&-3)` отвечает «отрицательное число», `describe(&"ок")` —
«строка», а `describe(&7u8)` — «неизвестно»: `u8` и `i32` — разные
типы, и неявного преобразования `downcast_ref` не делает.

`TypeId` во время работы — 128-битный хеш типа, который компилятор
вычисляет при компиляции. `downcast_ref::<T>` сравнивает `TypeId::of::<T>()`
с тем, что вернул `type_id` из таблицы объекта, и при совпадении
приводит указатель на данные к `&T`. Это `unsafe` внутри библиотеки, и
опирается оно на то, что реализация `Any` одна на все типы и
переопределить `type_id` нельзя.

Ограничение `'static` нужно потому, что `TypeId` не различает времена
жизни: у `&'a u8` и `&'static u8` он один. Будь `Any` у типов с заёмом,
`downcast_ref::<&'static u8>` продлил бы заём локальной переменной.
Компилятор не даёт сделать первый шаг. Doc-тест в
`code/ch11/src/any.rs` кладёт в `&dyn Any` ссылку на локальную
переменную:

```text
error[E0597]: `local` does not live long enough
 --> m.rs:6:22
  |
5 |         let local = 5u8;
  |             ----- binding `local` declared here
6 |         let r: &u8 = &local;
  |                      ^^^^^^ borrowed value does not live long enough
7 |         keep = &r;
  |         --------- coercion requires that `local` is borrowed for `'static`
8 |     }
  |     - `local` dropped here while still borrowed
```

Приведение к `dyn Any` требует `&u8: 'static`, то есть заёма на всё
время работы программы.

<div class="box c">

**Как в C.** В ядре Linux тип устройства узнают сравнением указателя на
таблицу: `is_usb_interface(dev)` проверяет `dev->type ==
&usb_if_device_type`. В C это надёжно, потому что таблица — одна
глобальная переменная. В Rust таблиц одного типа может быть несколько
(раздел [11.2](02-vtable.md)), и адрес таблицы о типе ничего не
говорит. Поэтому тип хранится отдельно, в виде хеша, и сравнивается
целиком.

</div>

## Супертрейты в объекте

Узел станции — и датчик, и исполнительное устройство, а `Any` в
супертрейтах позволяет вернуть его тип:

```rust
{{#include ../../code/ch11/src/any.rs:node}}
```

Таблица `dyn Node` для `Fan` содержит методы всех трёх супертрейтов и
свой:

```console
$ cargo asm -p ch11 --lib --include-constants ch11_as_node
…
.Lanon.da1beaeebe5f420a6faa69c57adbc3f3.15:
	.asciz	"\000\000\000\000\000\000\000\000\004\000\000\000\000\000\000\000\002\000\000\000\000\000\000"
	.quad	<ch11::any::Fan as ch11::fat::Sensor>::read
	.quad	<ch11::any::Fan as ch11::fat::Sensor>::name
	.quad	<ch11::any::Fan as ch11::any::Actuator>::set
	.quad	.Lanon.da1beaeebe5f420a6faa69c57adbc3f3.13
	.quad	<ch11::any::Fan as core::any::Any>::type_id
	.quad	.Lanon.da1beaeebe5f420a6faa69c57adbc3f3.14
	.quad	<ch11::any::Fan as ch11::any::Node>::id
```

| Слово | Смещение x86-64 | Смещение M3 | Содержимое |
|---|---|---|---|
| 0–2 | 0–16 | 0–8 | `drop` (ноль), размер 4, выравнивание 2 |
| 3 | 24 | 12 | `Sensor::read` |
| 4 | 32 | 16 | `Sensor::name` |
| 5 | 40 | 20 | `Actuator::set` |
| 6 | 48 | 24 | адрес таблицы `dyn Actuator` для `Fan` |
| 7 | 56 | 28 | `Any::type_id` |
| 8 | 64 | 32 | адрес таблицы `dyn Any` для `Fan` |
| 9 | 72 | 36 | `Node::id` |

Начало таблицы — готовая таблица `dyn Sensor`: заголовок, `read`,
`name`. Методы второго и третьего супертрейтов тоже лежат внутри, чтобы
`n.set(…)` звался одной загрузкой, но таблицей `dyn Actuator` этот
кусок служить не может: перед `set` нет заголовка из трёх слов. Поэтому
рядом лежит адрес отдельной, полной таблицы — для приведения.

## Приведение к супертрейту

С Rust 1.86 `&dyn Node` приводится к `&dyn Sensor`, `&dyn Actuator` и
`&dyn Any`, как `&Fan` приводится к любому из них:

```rust
{{#include ../../code/ch11/src/any.rs:upcast}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_upcast_first
ch11_upcast_first:
	mov rdx, rsi
	mov rax, rdi
	ret
$ cargo asm -p ch11 --lib --simplify ch11_upcast_second
ch11_upcast_second:
	mov rax, rdi
	mov rdx, qword ptr [rsi + 48]
	ret
$ cargo asm -p ch11 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch11_upcast_second
ch11_upcast_second:
	push {r7, lr}
	mov r7, sp
	ldr r1, [r1, #24]
	pop {r7, pc}
```

К первому супертрейту приведение бесплатно: таблица `dyn Node` и есть
таблица `dyn Sensor`, оба слова указателя остаются прежними. Ко второму
— одна загрузка, слово 6. Адрес данных не меняется ни в каком случае,
и тест `upcasting` проверяет это через `ptr::addr_eq`. `poll_node`
передаёт `&dyn Node` в `ch11_read_dyn`, которая ждёт `&dyn Sensor`, —
приведение неявное, как у ссылок на конкретный тип.

До 1.86 для этого в трейт добавляли метод `fn as_sensor(&self) -> &dyn
Sensor` и реализовывали его в каждом типе одной строкой `self`. Такой
код ещё встречается, и его можно удалить, подняв `rust-version` до
1.86, как сделано в `code/ch11/Cargo.toml`.

Обратного приведения нет: таблица `dyn Sensor` ничего не знает про
`Actuator`, и от датчика к узлу не добраться:

```text
error[E0605]: non-primitive cast: `&dyn Sensor` as `&(dyn Node + 'static)`
 --> l.rs:4:5
  |
4 |     s as &dyn Node
  |     ^^^^^^^^^^^^^^ an `as` expression can only be used to convert between primitive types or to coerce to a specific trait object
```

Если узел нужно найти среди датчиков, вниз ведёт только `Any`: от `&dyn
Sensor` с `Any` в супертрейтах — к конкретному `&Fan`.

## Вниз через Any

`ch11_as_fan` проходит весь путь: `&dyn Node` → `&dyn Any` → `&Fan`.

```console
$ cargo asm -p ch11 --lib --simplify ch11_as_fan
ch11_as_fan:
	push rbx
	sub rsp, 16
	mov rbx, rdi
	mov rax, qword ptr [rsi + 64]
	mov rdi, rsp
	mov rsi, rbx
	call qword ptr [rax + 24]
	movdqa xmm0, xmmword ptr [rsp]
	pcmpeqb xmm0, xmmword ptr [rip + .LCPI23_0]
	pmovmskb ecx, xmm0
	xor eax, eax
	cmp ecx, 65535
	cmove rax, rbx
	add rsp, 16
	pop rbx
	ret
```

Сначала приведение к `dyn Any` — загрузка слова 8. Затем вызов
`type_id` из таблицы `dyn Any`: 16 байт `TypeId` возвращаются через
память, адрес которой передан в `rdi`. `pcmpeqb` сравнивает их с
константой — `TypeId` типа `Fan` — побайтно, `pmovmskb` собирает 16
результатов в маску, и маска `0xffff` значит «все байты совпали». Тогда
результат — адрес данных, иначе ноль: `Option<&Fan>` со значением `None`
— нулевой указатель (раздел [6.4](../ch06/04-niches.md)). Слово 7, тот
же `type_id` прямо в таблице `dyn Node`, LLVM не использовал:
`downcast_ref` написан для `dyn Any`, и после приведения функция
работает уже с другой таблицей.

<div class="box trap">

**Ловушка.** `Box<dyn Any>` и сам `'static`, и сам реализует `Any`.
Метод `type_id`, вызванный у коробки, находит реализацию для `Box`
раньше, чем для значения внутри:

```rust
{{#include ../../code/ch11/src/any.rs:box-any}}
```

`box_ids(Box::new(5u8))` возвращает `(false, true)`. `downcast` и
`downcast_ref` у `Box<dyn Any>` написаны правильно, ошибаются при
ручном сравнении `TypeId`. Clippy знает этот случай: линт
`type_id_on_box` включён по умолчанию и советует `(*b)`.

</div>

<div class="box check">

**Проверь себя.** Сколько слов в таблице `dyn Node`, если поменять
местами `Sensor` и `Actuator` в списке супертрейтов, и какое приведение
станет бесплатным? Почему `Any` требует `'static`, а `dyn Sensor` — нет?
Что вернёт `describe(&Fan { rpm: 0, id: 0 })`, и как добавить в
`describe` ветку для узлов, не перечисляя их типы?

</div>
