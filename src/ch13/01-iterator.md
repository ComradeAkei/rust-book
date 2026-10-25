# 13.1. Iterator и IntoIterator

## Один метод

*Итератор* (iterator) — любой тип, реализующий трейт `Iterator`. Обязательных частей
у трейта две: тип элемента и метод, который выдаёт следующий элемент
или `None`, если элементы кончились:

```rust,ignore
pub trait Iterator {
    type Item;
    fn next(&mut self) -> Option<Self::Item>;
    // … и больше полусотни методов с готовыми телами
}
```

`next` берёт `&mut self`: итератор — это состояние обхода, и каждый
вызов его меняет. Обратный отсчёт — число, которое уменьшается:

```rust
{{#include ../../code/ch13/src/basics.rs:countdown}}
```

Всё остальное, что умеет итератор, трейт выражает через `next`, и
`Countdown` получает это даром:

```rust
{{#include ../../code/ch13/src/basics.rs:free_methods}}
```

`Item` — [ассоциированный тип][g-assoc-type], а не параметр трейта, как у
`From<T>`: у итератора один тип элемента, и пользователь его не
выбирает. Поэтому `Countdown(4).max()` не спрашивает, итератор чего
перед ним: это известно из `impl`. (А `sum` спрашивает, но о другом —
о типе результата, раздел [13.3](03-consumers.md).)

<div class="box c">

**Как в C.** Ближайший аналог в C — пара функций `has_next`/`next` или
функция, которая заполняет `*out` и возвращает `bool`, как `readdir`,
`strtok_r` или `getline`. Состояние обхода (позицию, указатель на
структуру `DIR`) приходится хранить и передавать отдельно, а «элементов
больше нет» — особое значение: `NULL`, `-1`, `EOF`. `Option<Item>`
объединяет и то и другое, и забыть проверить его нельзя: `Some(x)`
надо распаковать, чтобы добраться до `x`.

</div>

## for и IntoIterator

Цикл `for` работает не с итератором, а с любым значением, которое
*можно превратить* в итератор, то есть реализует `IntoIterator`. Цикл

```rust,ignore
for x in expr {
    body
}
```

компилятор разворачивает примерно так:

```rust,ignore
let mut it = IntoIterator::into_iter(expr);
loop {
    match it.next() {
        Some(x) => body,
        None => break,
    }
}
```

Каждый итератор реализует `IntoIterator` сам для себя, поэтому
`for x in v.iter()` работает. А у коллекции реализаций три: для самой
коллекции, для общей ссылки на неё и для изменяемой. Отсюда три формы
`for` с разными типами элемента и разной судьбой коллекции:

```rust
{{#include ../../code/ch13/src/basics.rs:three_for}}
```

| Форма | Итератор у rustc 1.97 | Элемент | Коллекция после цикла |
|---|---|---|---|
| `for x in &v` | `core::slice::iter::Iter<'_, u32>` | `&u32` | цела, только читалась |
| `for x in &mut v` | `core::slice::iter::IterMut<'_, u32>` | `&mut u32` | цела, изменена |
| `for x in v` | `alloc::vec::into_iter::IntoIter<u32>` | `u32` | [перемещена][g-move] в цикл и освобождена |

Имена типов проверяет тест `observed_into_iter_types`: имена, которые
печатает `type_name`, стабильностью не обещаны. После `for x in v`
вектора у функции нет, как после любого перемещения:

```console
error[E0382]: borrow of moved value: `v`
2 | let v = vec![1u32, 2, 3];
  |     - move occurs because `v` has type `Vec<u32>`, which does not implement the `Copy` trait
3 | for x in v {
  |          - `v` moved due to this implicit call to `.into_iter()`
...
6 | let n = v.len();
  |         ^ value borrowed here after move
```

Массив тоже отдаёт элементы по значению: `for x in [1, 2, 3]` даёт
`i32`, а не `&i32`, и итератор `core::array::IntoIter<i32, 3>` хранит
сам массив внутри себя.

<div class="box ref">

**Справка: что внутри итераторов.** Размеры у rustc 1.97 (тест
`observed_iterator_sizes`; [раскладка][g-layout] `repr(Rust)` не гарантирована):

| Итератор | Размер | Что хранит |
|---|---|---|
| `slice::Iter<'_, u32>` | 2 слова | указатель на текущий элемент и указатель на конец |
| `Range<usize>` | 2 слова | начало и конец |
| `RangeInclusive<u8>` | 3 байта | начало, конец и флаг «исчерпан» |
| `vec::IntoIter<u32>` | 4 слова | буфер и ёмкость, чтобы освободить; текущий и конец |
| `array::IntoIter<u32, 4>` | 16 байт + 2 слова | сам массив и диапазон ещё не отданных индексов |

Слово — 8 байт на x86-64 и 4 на Cortex-M. Ни один из них не держит
ничего в куче: итератор — несколько слов на стеке или в регистрах.

</div>

## Итератор среза изнутри

`slice::Iter` устроен как обход в C указателем: два указателя, текущий
и «за концом», и шаг — сравнить их и сдвинуть текущий. Функция, которая
делает один шаг, видна в ассемблере целиком:

```rust
{{#include ../../code/ch13/src/basics.rs:next}}
```

```console
$ cargo asm -p ch13 --lib --simplify ch13_next
ch13_next:
	mov rax, qword ptr [rdi]
	cmp rax, qword ptr [rdi + 8]
	je .LBB66_1
	lea rcx, [rax + 4]
	mov qword ptr [rdi], rcx
	ret
.LBB66_1:
	xor eax, eax
	ret
```

`[rdi]` — текущий указатель, `[rdi + 8]` — конец. Если они равны,
функция возвращает ноль: это `None`, потому что `Option<&u32>` занимает
одно слово, и ссылка, которая не бывает нулевой, отдаёт ноль под `None`
([ниша][g-niche] из главы 6). Иначе она сдвигает текущий указатель на 4 байта и
возвращает прежний — `Some(&x)`. [Проверки границ][g-bounds-check] нет: указатель не
выходит за конец по построению. На Cortex-M3 то же самое помещается в
[условное выполнение][g-conditional-execution] без переходов:

```console
$ cargo asm -p ch13 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch13_next
ch13_next:
	push {r7, lr}
	mov r7, sp
	mov r1, r0
	ldrd r0, r2, [r0]
	cmp r0, r2
	itee eq
	moveq r0, #0
	addne r2, r0, #4
	strne r2, [r1]
	pop {r7, pc}
```

`ldrd` загружает оба указателя одной инструкцией, `itee eq` делает
следующую инструкцию условной по «равно», а две за ней — по «не равно».

<div class="box c">

**Как в C.** Это ровно `for (const uint32_t *p = a; p != a + n; p++)`:
указатель «за последний элемент» C разрешает вычислять и сравнивать,
но не разыменовывать. `slice::Iter` разыменовывает только то, что
прошло сравнение с концом, и `p` в нём недоступен снаружи: ни
перепутать конец, ни сдвинуть указатель на два элемента вместо одного
нельзя.

</div>

## by_ref: продолжить после break

`for` забирает итератор себе: `for x in it` вызывает
`it.into_iter()`, а для итератора это он сам. После цикла итератора у
функции нет. Чтобы прервать обход и продолжить с того же места,
итератор отдают циклу взаймы: `by_ref()` возвращает `&mut Self`, а
`&mut I` — тоже итератор, который двигает исходный:

```rust
{{#include ../../code/ch13/src/basics.rs:by_ref}}
```

Для `b"T01\0abc"` функция возвращает `b"T01"` и `b"abc"`: `break`
случился на нуле, ноль `next` уже забрал, и `as_slice()` отдаёт то, что
осталось после него.

## Диапазоны

`a..b` и `a..=b` — не пара чисел, а итераторы: `Range` и
`RangeInclusive` реализуют `Iterator` сами. Поэтому диапазон не `Copy`,
и цикл его расходует:

```console
error[E0382]: borrow of moved value: `r`
2 | let r = 0..3;
  |     - move occurs because `r` has type `std::ops::Range<i32>`, which does not implement the `Copy` trait
3 | for i in r {
  |          - `r` moved due to this implicit call to `.into_iter()`
...
6 | let n = r.len();
  |         ^ value borrowed here after move
```

```rust
{{#include ../../code/ch13/src/basics.rs:ranges}}
```

`ranges()` возвращает `(256, 32640, [9, 6, 3, 0])`. Все 256 значений
`u8` даёт только `0..=255u8`: верхняя граница полуоткрытого диапазона
должна поместиться в тип, а `256u8` не помещается, и это ошибка, а не
предупреждение:

```console
error: range endpoint is out of range for `u8`
2 |     for b in 0..256u8 {
  |              ^^^^^^^^ help: use an inclusive range instead: `0..=255u8`
  |
  = note: `#[deny(overflowing_literals)]` on by default
```

Сам `0..=255u8` не переполняется на последнем шаге: после 255 он не
пытается вычислить 256, а ставит флаг «исчерпан», третий байт
`RangeInclusive<u8>` из справки выше. В C тот же цикл
`for (uint8_t b = 0; b <= 255; b++)` бесконечен: `b++` после 255 даёт 0,
и условие снова истинно.

<div class="box check">

**Проверь себя.** Что вернут `(0..11).rev().step_by(3).last()` и
`(0..11).step_by(3).rev().last()`, и почему для `0..10` ответы
совпадают?
Сколько раз выполнится тело `for _ in 5..3`? Почему
`for b in &mut v.iter()` компилируется, а `for b in &v.iter()` — нет?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-assoc-type]: ../appendix-f.md#assoc-type "Ассоциированный тип (associated type): Тип, который называет каждая реализация трейта: type Sample в трейте Source."
[g-bounds-check]: ../appendix-f.md#bounds-check "Проверка границ (bounds check): Сравнение индекса с длиной среза перед обращением: одна беззнаковая инструкция и переход к панике."
[g-conditional-execution]: ../appendix-f.md#conditional-execution "Условное выполнение (conditional execution): Выполнение команды, только если выполнено условие по флагам, без перехода."
[g-layout]: ../appendix-f.md#layout "Раскладка (layout): Размер, выравнивание и смещения полей типа в памяти."
[g-move]: ../appendix-f.md#move "Перемещение (move): Передача владения значением: после let b = a владеет b, а a больше использовать нельзя."
[g-niche]: ../appendix-f.md#niche "Ниша (niche): Битовый шаблон, который не встречается ни у одного значения типа: 0 у ссылки и NonZeroU32, 2–255 у bool, суррогаты и числа больше 0x10FFFF у char."
