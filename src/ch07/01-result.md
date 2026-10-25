# 7.1. Result и оператор ?

## Ошибка как значение

`Result` — обычное перечисление из `core`, ничем не выделенное в языке:

```rust,ignore
enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

Функция, которая может не выполниться, сообщает об этом типом
результата. Вот модель шины I2C с датчиком температуры. Чтение регистра
может не получить подтверждения от устройства или проиграть арбитраж
другому ведущему:

```rust
{{#include ../../code/ch07/src/bus.rs:bus}}
```

Значение `u8` лежит внутри `Ok`, и достать его, не разобрав вариант,
нельзя. Забыть проверку, как в `unchecked.c`, не получится: без `match`,
`if let`, `?` или явного `unwrap` программа не скомпилируется.

## Оператор ?

Температура собирается из двух регистров, и каждое чтение может
сорваться:

```rust
{{#include ../../code/ch07/src/bus.rs:read_temp}}
```

`?` после выражения типа `Result` делает одно из двух: при `Ok(v)` даёт
значение `v`, при `Err(e)` немедленно возвращает ошибку из функции.
Reference описывает это так (`code/ch07/src/bus.rs`, тест проверяет, что
обе функции дают одинаковый результат для каждого сбоя):

```rust
{{#include ../../code/ch07/src/bus.rs:desugared}}
```

Два наблюдения. Во-первых, это обычный `return`: перед выходом
выполняются деструкторы всех живых переменных, и ничего подобного
`longjmp` из введения здесь не случится. Во-вторых, ошибка проходит
через `From::from`. Пока типы совпадают, это тождественное
преобразование, а зачем оно нужно, показывает раздел
[7.2](02-convert.md).

<div class="box hood">

**Под капотом: трейт `Try`.** На самом деле компилятор раскрывает `?`
через два нестабильных трейта, и это видно в [промежуточном
представлении][g-ir] [HIR][g-hir]. Файл `code/ch07/hir/question.rs`:

```rust
{{#include ../../code/ch07/hir/question.rs:3:}}
```

```console
$ rustc +nightly --edition 2024 --crate-type lib -Zunpretty=hir question.rs
fn first(v: &'_ [u8])
    ->
        Result<u8,
        &'static str> {
    let x =
        match branch(v.first().ok_or("empty")) {
            Break {  0: residual } =>
                #[attr = LintCheck([LintCheck {lint_name: "unreachable_code",
                kind: Allow}])]
                return from_residual(residual),
            Continue {  0: val } =>
                #[attr = LintCheck([LintCheck {lint_name: "unreachable_code",
                kind: Allow}])]
                val,
        };
    Ok(*x)
}
```

`Try::branch` делит значение на «продолжить с `val`» и «остаток»
(`residual`), а `FromResidual::from_residual` строит из остатка
возвращаемое значение функции. Для `Result<T, E>` остаток —
`Result<Infallible, E>`, и `from_residual` вызывает `From::from` для
ошибки. Благодаря этой схеме `?` работает с `Result`, `Option`,
`ControlFlow` и `Poll` (глава 22). Свой тип с `?` на стабильном Rust
сделать нельзя: оба трейта пока нестабильны.

</div>

## ? на Option

С `Option` оператор работает так же, только остаток — `None`. Поле CRC
кадра из главы 6 лежит в байтах 6 и 7, и короткий кадр даёт `None`:

```rust
{{#include ../../code/ch07/src/bus.rs:option}}
```

Смешивать нельзя: `?` на `Option` в функции, которая возвращает
`Result`, не компилируется, потому что из `None` не получить никакого
значения ошибки. Компилятор подсказывает исправление:

```text
error[E0277]: the `?` operator can only be used on `Result`s, not `Option`s, in a function that returns `Result`
 --> e0277.rs:2:27
  |
1 | fn crc_or_err(frame: &[u8]) -> Result<u16, &'static str> {
  | -------------------------------------------------------- this function returns a `Result`
2 |     let lo = *frame.get(6)?;
  |                           ^ use `.ok_or(...)?` to provide an error compatible with `Result<u16, &str>`
```

`ok_or` превращает `Option` в `Result`, указав ошибку для `None`. Оба
варианта записаны [doc-тестами][g-doctest] в `code/ch07/src/bus.rs`.

<div class="box c">

**Как в C.** Ближайший аналог `?` в C — макрос вида
`#define TRY(x) do { int rc_ = (x); if (rc_ < 0) return rc_; } while (0)`
или `goto out` с общей точкой очистки, как в ядре Linux. Тот же
`read_temp` в стиле C:

```rust
{{#include ../../code/ch07/src/bus.rs:c_style}}
```

Разница в трёх местах. Выходной параметр `out` можно прочитать, не
проверив код, и получить мусор. Код −1 ничего не говорит о причине:
номер регистра, на котором сорвался обмен, потерян. И очистку перед
`return` пишет человек, а в Rust её пишет компилятор.

</div>

## main тоже может вернуть Result

```rust
{{#include ../../code/ch07/src/bin/main_result.rs:main}}
```

```console
$ cargo run -q -p ch07 --bin main_result -- 115200
скорость 115200
$ cargo run -q -p ch07 --bin main_result -- 115k; echo "код $?"
Error: ParseIntError { kind: InvalidDigit }
код 1
$ cargo run -q -p ch07 --bin main_result; echo "код $?"
Error: "нужна скорость порта"
код 1
```

<div class="box trap">

**Ловушка.** При ошибке `main` печатает её через `Debug`, а не
`Display`: пользователь видит `ParseIntError { kind: InvalidDigit }`
вместо «invalid digit found in string», а строка выводится в кавычках.
Для программ, которые увидит человек, `main` лучше писать так: вызвать
внутреннюю функцию, которая возвращает `Result`, напечатать ошибку и её
причины самому (раздел [7.3](03-error-trait.md)) и выйти с нужным кодом
через `std::process::ExitCode`.

</div>

## Как Result возвращается из функции

Утверждение «`Result` бесплатен» проверяется ассемблером. Четыре функции
из `code/ch07/src/abi.rs` различаются только типом результата:

```rust
{{#include ../../code/ch07/src/abi.rs:types}}
```

```rust
{{#include ../../code/ch07/src/abi.rs:functions}}
```

<div class="box hood">

**Под капотом.** Масштабирование с проверкой переполнения,
`Result<u32, Overflow>`:

```text
x86_64:                          thumbv7m:
ch07_scale:                      ch07_scale:
	mov eax, edi                 	push {r7, lr}
	xor ecx, ecx                 	mov r7, sp
	mul esi                      	umull r1, r0, r0, r1
	mov edx, eax                 	cmp r0, #0
	seto cl                      	it ne
	mov eax, ecx                 	movne r0, #1
	ret                          	pop {r7, pc}
```

Результат занимает два регистра: признак варианта в `eax` (`r0`),
значение в `edx` (`r1`). Признак — это флаг переполнения умножения:
`seto` на x86_64, старшая половина 64-битного произведения `umull` на
Cortex-M3. Это та же работа, что у C-функции с кодом возврата и
выходным параметром, только без записи результата в память через
указатель.

Установка скорости порта, `Result<(), Errno>`:

```text
x86_64:                          thumbv7m:
ch07_set_baud:                   ch07_set_baud:
	cmp edi, 9600                	push {r7, lr}
	je .LBB46_3                  	mov r7, sp
	cmp edi, 115200              	cmp.w r0, #9600
	je .LBB46_3                  	beq .LBB41_2
	mov eax, 22                  	cmp.w r0, #115200
	cmp edi, 19200               	it ne
	jne .LBB46_4                 	cmpne.w r0, #19200
.LBB46_3:                        	bne .LBB41_3
	xor eax, eax                 .LBB41_2:
.LBB46_4:                        	movs r0, #0
	ret                          	pop {r7, pc}
                                 .LBB41_3:
                                 	movs r0, #22
                                 	pop {r7, pc}
```

Это ровно `int set_baud(uint32_t)` из C: 0 при успехе, 22 (`EINVAL`) при
ошибке, в одном регистре. `Errno` хранит `NonZeroI32`, у которого ноль
недопустим, и `Ok(())` занимает эту [нишу][g-niche] (раздел 6.4). Но в отличие от
C, вызывающий не может принять 22 за успешный результат.

Деление нацело, `Result<u64, DivError>`, 16 байт:

```text
x86_64 (начало и успешный выход):
ch07_div_exact:
	test rdx, rdx
	je .LBB36_1
	...
.LBB36_9:
	mov qword ptr [rdi + 8], rax
	xor eax, eax
	mov byte ptr [rdi], al
	mov rax, rdi
	ret
```

Здесь `Result` возвращается через память: вызывающий передаёт адрес
в `rdi` (на Cortex-M3 — в `r0`, а `a` и `b` сдвигаются в `r2:r3` и на
стек), функция пишет туда признак и значение.

</div>

Сводка по четырём функциям:

| Тип | Размер | x86_64 | thumbv7m |
|---|---|---|---|
| `Result<u8, BadHex>` | 2 | `al` и `dl` | `r0` и `r1` |
| `Result<u32, Overflow>` | 8 | `eax` и `edx` | `r0` и `r1` |
| `Result<(), Errno>` | 4 | `eax` | `r0` |
| `Result<u64, DivError>` | 16 | память, адрес в `rdi` | память, адрес в `r0` |

Правило, которое стоит за таблицей, — в [раскладке][g-layout]. Если у `Result` во
всех вариантах данные укладываются в два скаляра на одних и тех же
местах (признак и одно значение), rustc описывает тип как пару скаляров
и возвращает его в двух регистрах. Так устроены первые два типа.
Третий — один скаляр благодаря нише. У `Result<u64, DivError>` значение
`u64` лежит по смещению 8, а `DivError` — по смещению 1, парой скаляров
это не описать, и 16 байт уходят в память. То же правило для обычных
структур снято в главе 3: агрегат размером до указателя Rust [ABI][g-abi]
возвращает в регистре, больший — через память. Раскладка `repr(Rust)` не
обещана (раздел 6.2), поэтому таблица — наблюдение для rustc 1.97, а не
гарантия.

<div class="box trick">

**Трюк.** Если ошибка не несёт данных и встречается в [горячем коде][g-hot-path],
сделайте её [типом нулевого размера][g-zst] или перечислением без полей и
следите, чтобы `Ok` был не больше указателя. Тогда `Result` останется в
регистрах. Проверить себя можно так же, как в главе 6:
`const _: () = assert!(size_of::<Result<u32, Overflow>>() == 8);`.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `?` вызывает `From::from` даже тогда, когда типы ошибок
   совпадают, и сколько это стоит в машинном коде?
2. В каких регистрах вернётся `Result<u16, BadHex>` на x86_64? А
   `Result<[u8; 3], BadHex>`?
3. Чем `Result<(), Errno>` лучше `i32`-кода, если машинный код у них
   одинаковый?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-abi]: ../appendix-f.md#abi "ABI (application binary interface): Двоичный интерфейс платформы: соглашение о вызовах, раскладка типов в памяти, имена символов."
[g-doctest]: ../appendix-f.md#doctest "doc-тест (doctest): Пример кода в документирующем комментарии ///, который cargo test собирает и запускает как тест."
[g-hir]: ../appendix-f.md#hir "HIR (high-level IR): Промежуточное представление rustc, ближайшее к исходнику: синтаксическое дерево после раскрытия макросов и удаления синтаксического сахара."
[g-hot-path]: ../appendix-f.md#hot-path "Горячий и холодный путь (hot path, cold path): Горячий — код, который выполняется почти всегда: тело цикла, частая ветка; от его скорости зависит скорость программы."
[g-ir]: ../appendix-f.md#ir "Промежуточное представление (intermediate representation, IR): Форма программы между исходником и машинным кодом, удобная для анализа и оптимизации. rustc проходит через несколько: HIR, MIR, затем LLVM IR."
[g-layout]: ../appendix-f.md#layout "Раскладка (layout): Размер, выравнивание и смещения полей типа в памяти."
[g-niche]: ../appendix-f.md#niche "Ниша (niche): Битовый шаблон, который не встречается ни у одного значения типа: 0 у ссылки и NonZeroU32, 2–255 у bool, суррогаты и числа больше 0x10FFFF у char."
[g-zst]: ../appendix-f.md#zst "Тип нулевого размера (zero-sized type, ZST): Тип, значения которого не занимают памяти: (), единичные структуры, PhantomData, [T; 0], перечисление из одного варианта без данных."
