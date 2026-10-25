# 0.5. Проверка корректности: Miri, санитайзеры, clippy

Компилятор проверяет безопасное подмножество языка. В блоке `unsafe`
программист обещает то, что компилятор проверить не может, и если обещание
ложно, поведение программы не определено, ровно как в C. Правила `unsafe`
разобраны в главе 16. Этот раздел — об инструментах, которые находят
нарушение раньше, чем оно станет загадочным сбоем на стенде.

Подопытный пример содержит две ошибки, и обе проходят компиляцию без
предупреждений:

```rust
{{#include ../../code/ch00/examples/ub.rs:stale}}
```

```rust
{{#include ../../code/ch00/examples/ub.rs:off_by_one}}
```

## Обычный запуск

```console
$ cargo run -p ch00 --example ub
первый элемент: 1

thread 'main' (12146) panicked at code/ch00/examples/ub.rs:31:29:
unsafe precondition(s) violated: slice::get_unchecked requires that the index is within the slice

This indicates a bug in the program. This Undefined Behavior check is optional, and cannot be relied on for safety.
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
thread caused non-unwinding panic. aborting.

$ cargo run -p ch00 --example ub --release
первый элемент: 1
среднее: 3756348008
$ cargo run -p ch00 --example ub --release
первый элемент: 1
среднее: 2065320952
```

Первую ошибку не видно ни в одном профиле: указатель висячий, но читает
правильное число. Вектор растёт с трёх элементов до шести и просит у
`realloc` вместо 12 байт 24. Самый маленький блок кучи glibc на x86_64
вмещает 24 байта, поэтому `realloc` расширяет блок на месте и возвращает
тот же адрес. Для языка это ничего не меняет: старый указатель относится к
освобождённому выделению, и чтение через него — UB, даже когда адрес
совпал. Смените аллокатор (на Cortex-M его выбираете вы сами, глава 15),
и вместо единицы прочитается мусор.

Вторую ошибку в dev-сборке поймала сама программа. `get_unchecked`
обещает не проверять индекс, но в отладочной сборке проверяет: в `core`
встроены *проверки предусловий* (`ub_checks`), и они включены вместе с
`debug-assertions`. Сообщение честно предупреждает, что проверка
необязательная. В release её нет, и программа печатает мусор, каждый раз
другой.

<div class="box hood">

**Под капотом: что оптимизатор сделал из ошибки.** В примере `average`
встроена в `main`, поэтому отдельной функции в ассемблере нет. Та же
функция вынесена в библиотеку главы отдельным символом:

```rust
{{#include ../../code/ch00/src/hood.rs:average_bad}}
```

```text
$ cargo asm -p ch00 --lib ch00_average_bad
ch00_average_bad:
	test rsi, rsi
	je .LBB2_2
.LBB2_1:
	jmp .LBB2_1
.LBB2_2:
	push rax
	lea rdi, [rip + .Lanon.292e048a2bfb50c6e667bb239c5e7ca9.1]
	call qword ptr [rip + core::panicking::panic_const::panic_const_div_by_zero@GOTPCREL]
```

Ни чтения памяти, ни сложения, ни деления. Пустой [срез][g-slice] ведёт к панике
деления на ноль, непустой — в бесконечный цикл `jmp` сам на себя. Для
`thumbv7m-none-eabi` результат тот же: `cbz r1, .LBB2_2` и `b .LBB2_1`.
Логику видно в LLVM IR (`--llvm`):

```text
bb2.i:
  %iter.sroa.0.012 = phi i64 [ %_0.i4.i, %bb2.i ], [ 1, %start ]
  %_0.i4.i = add nuw i64 %iter.sroa.0.012, 1
  %_14 = icmp ult i64 %iter.sroa.0.012, %xs.1
  tail call void @llvm.assume(i1 %_14)
  br label %bb2.i
```

`get_unchecked` сообщает оптимизатору `assume(i < len)`: «индекс всегда в
границах». Цикл `1..=len` выходит только после итерации с `i == len`, где
предположение ложно. Значит, законного выхода из цикла нет, а раз
функция не возвращается, то и сумма никому не нужна. Оптимизатор вправе
так рассуждать: программа без UB сюда не попадает. В `main` из-за
встраивания получилось иначе: ячейка стека для результата `average`
выделена, но в неё ничего не записано, и `println!` печатает то, что там
осталось от предыдущего кода. Запустите `ub --release -- average`, и
«среднее» окажется равно длине пути к исполняемому файлу: в этой ячейке
лежала ёмкость строки `argv[0]`. Одна ошибка, три разных поведения:
паника, мусор и зависание.

</div>

## Miri

*Miri* (MIR interpreter) — интерпретатор MIR, промежуточного представления rustc после
проверки типов и заимствований, до LLVM. Он не запускает машинный код, а
исполняет программу по правилам языка. Для каждого выделения памяти Miri
помнит размер, выравнивание и освобождено ли оно, для каждого байта —
инициализирован ли он и какому выделению принадлежит указатель, который
на него смотрит ([provenance][g-provenance], глава 16). Ещё он проверяет правила
заимствования для ссылок, созданных из [сырых указателей][g-raw-pointer].

```console
$ rustup +nightly component add miri
$ cargo +nightly miri run -p ch00 --example ub
error: Undefined Behavior: memory access failed: alloc705 has been freed, so this pointer is dangling
  --> code/ch00/examples/ub.rs:21:14
   |
21 |     unsafe { *first }
   |              ^^^^^^ Undefined Behavior occurred here
   |
   = help: this indicates a bug in the program: it performed an invalid operation, and caused Undefined Behavior
   = help: see https://doc.rust-lang.org/nightly/reference/behavior-considered-undefined.html for further information
help: alloc705 was allocated here:
  --> code/ch00/examples/ub.rs:41:21
   |
41 |         let mut v = Vec::with_capacity(3);
   |                     ^^^^^^^^^^^^^^^^^^^^^
help: alloc705 was deallocated here:
  --> code/ch00/examples/ub.rs:19:5
   |
19 |     v.push(4); // ёмкость 3 исчерпана: realloc
   |     ^^^^^^^^^
```

Miri нашёл ошибку, которую не видно на железе: адрес тот же, но выделение
`alloc705` освобождено при `push`. Miri останавливается на первой ошибке,
поэтому вторую запускаем отдельно. В профиле dev её снова перехватит
проверка предусловия в `get_unchecked`, и Miri сообщит только об аварийном
завершении. Чтобы увидеть диагноз самого Miri, нужен профиль без
`debug-assertions`:

```console
$ cargo +nightly miri run -p ch00 --example ub --release -- average
warning: Miri does not support optimizations: the opt-level is ignored. ...

error: Undefined Behavior: `assume` called with `false`
  --> code/ch00/examples/ub.rs:31:26
   |
31 |         sum += unsafe { *xs.get_unchecked(i) };
   |                          ^^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

Это то самое `assume(i < len)`, которое оптимизатор превратил в
бесконечный цикл. Miri сообщает о нарушении в момент, когда оно
происходит, а не тогда, когда его последствия добрались до вывода.

Плата за точность — скорость. Программа, которая создаёт вектор из
100 000 чисел и складывает их, в debug-сборке работает 5 мс вместе с
запуском процесса, а под Miri — 20 с. Поэтому под Miri гоняют тесты на
маленьких входах, а не всю программу. Запускаются они так же:
`cargo +nightly miri test -p ch00`. В CI книги тесты глав и решений
проходят под Miri отдельным заданием. Тесты, которые запускают внешние
процессы, помечены `#[cfg_attr(miri, ignore)]`: интерпретатор по
умолчанию изолирован от системы.

## Санитайзеры

*Санитайзер* (sanitizer) — это инструментирование машинного кода. Компилятор вставляет
проверку перед каждым обращением к памяти, а библиотека времени
выполнения подменяет `malloc` и `free` и ведёт *теневую память* (shadow memory): на каждые
8 байт программы один байт описывает, можно ли их трогать. *AddressSanitizer*
(ASan) в Rust тот же, что в Clang, но включается нестабильным ключом,
поэтому только на nightly:

```console
$ RUSTFLAGS=-Zsanitizer=address cargo +nightly run -p ch00 --example ub \
>     --target x86_64-unknown-linux-gnu
=================================================================
==12248==ERROR: AddressSanitizer: heap-use-after-free on address 0x7b21a29e0010 at pc 0x55db8edf7d73 bp 0x7ffdb2ff70c0 sp 0x7ffdb2ff70b8
READ of size 4 at 0x7b21a29e0010 thread T0
    #0 0x55db8edf7d72 in ub::first_after_push .../code/ch00/examples/ub.rs:21:14
    #1 0x55db8edf8209 in ub::main .../code/ch00/examples/ub.rs:43:53
    ...
0x7b21a29e0010 is located 0 bytes inside of 12-byte region [0x7b21a29e0010,0x7b21a29e001c)
freed by thread T0 here:
    #0 0x55db8edc8ad1 in realloc (.../examples/ub+0xddad1)
    #1 0x55db8ee26272 in <alloc::raw_vec::RawVecInner>::finish_grow (.../examples/ub+0x13b272)
    ...
previously allocated by thread T0 here:
    #0 0x55db8edc86c4 in malloc (.../examples/ub+0xdd6c4)
    ...
SUMMARY: AddressSanitizer: heap-use-after-free .../code/ch00/examples/ub.rs:21:14 in ub::first_after_push
Shadow bytes around the buggy address:
=>0x7b21a29e0000: fa fa[fd]fd fa fa fa fa fa fa fa fa fa fa fa fa
```

`realloc` из ASan никогда не расширяет блок на месте: он выделяет новый,
а старый держит в карантине, помеченным как освобождённый. В строке теневой
памяти `fd` — освобождённая память, `fa` — *красная зона* (redzone)
вокруг блоков кучи: байты, обращение к которым ASan считает выходом за
границу блока.
Квадратные скобки отмечают байт, который описывает прочитанный адрес.

`--target` в команде обязателен, хотя цель та же, что по умолчанию. Если
цель указана явно, cargo применяет `RUSTFLAGS` только к коду цели, а [сценарии сборки][g-build-script] и [процедурные макросы][g-proc-macro], которые исполняются внутри
компилятора, собирает без санитайзера.

Вторую ошибку ASan в dev не увидит: раньше сработает проверка предусловия.
Без неё (`-Zub-checks=no`) он находит выход за границу константы:

```console
$ RUSTFLAGS="-Zsanitizer=address -Zub-checks=no" cargo +nightly run -p ch00 --example ub \
>     --target x86_64-unknown-linux-gnu -- average
==12400==ERROR: AddressSanitizer: global-buffer-overflow on address 0x56345dba89cc ...
READ of size 4 at 0x56345dba89cc thread T0
    #0 0x56345dc94bb3 in ub::average .../code/ch00/examples/ub.rs:31:25
...
0x56345dba89cc is located 0 bytes after global variable 'alloc_f53a061652f7116af86393f561fe2419' ... of size 12
```

А в release тот же санитайзер молчит:

```console
$ RUSTFLAGS=-Zsanitizer=address cargo +nightly run -p ch00 --example ub --release \
>     --target x86_64-unknown-linux-gnu -- average
среднее: 0
```

Санитайзер проверяет те обращения к памяти, которые остались в
машинном коде. Оптимизатор выбросил чтение за границей раньше, чем до
него дошёл проход ASan, и проверять стало нечего.

| | Проверки предусловий | Miri | ASan |
|---|---|---|---|
| Что это | Проверки в `core`, встроенные в вашу программу | Интерпретатор MIR | Проверки в машинном коде и свой аллокатор |
| Тулчейн | stable, профиль dev | nightly | nightly, цели с ОС (x86_64, AArch64) |
| Замедление | небольшое | в тысячи раз | около 2× по документации Clang |
| Ошибка 1 (висячий указатель) | не видит | находит | находит |
| Ошибка 2 (выход за границу) | находит | находит | находит без оптимизации |
| Чего не видит | всё, что не проверено явно | [FFI][g-ffi], ассемблер, невыполненные пути | правила заимствования, UB, уже использованное оптимизатором |

Кроме ASan, rustc поддерживает *ThreadSanitizer* (TSan, гонки данных),
*MemorySanitizer* (MSan, чтение неинициализированной памяти) и
*LeakSanitizer* (LSan, утечки). Они
понадобятся в главах 20 и 31.

<div class="box trap">

**Ловушка.** Miri и санитайзеры проверяют одно выполнение, а не программу.
Если тест не дошёл до ветки с ошибкой, оба промолчат. В `first_after_push`
ошибка есть только потому, что ёмкость вектора ровно 3: с
`Vec::with_capacity(4)` перевыделения не будет, указатель останется
действительным, и Miri честно не найдёт ничего, потому что UB действительно
нет. Зелёный прогон под Miri значит «на этих входах UB не случилось», а
доказательство корректности остаётся за вами и комментарием `SAFETY`.

</div>

<div class="box c">

**Как в C.** ASan в GCC и Clang устроен так же и ловит те же ошибки. Valgrind
(memcheck) не требует пересборки, но замедляет программу, по его же
документации, в 20–30 раз. Прямого аналога Miri в C нет: Valgrind и ASan
видят машину, а не язык. Если память жива и выровнена, обращение для них
корректно. Поэтому нарушения правил заимствования, например две `&mut` на
один и тот же байт, полученные из сырых указателей, не находит ни один из
них, а Miri находит (глава 16).

</div>

## clippy

*clippy* — *линтер* (linter), который ставится вместе с тулчейном. Он
знает несколько сотен *линтов* (lints) — проверяемых шаблонов: от
неидиоматичного кода до заведомых ошибок. Пример лежит
в отдельном крейте, чтобы не сломать проверку всей книги:

```rust
{{#include ../../code/ch00/lints/src/lib.rs:lints}}
```

```console
$ cd code/ch00/lints && cargo clippy
warning: it looks like you're manually copying between slices
  --> src/lib.rs:8:5
   |
 8 | /     for i in 0..src.len() {
 9 | |         dst[i] = src[i];
10 | |     }
   | |_____^ help: try replacing the loop by: `dst[..src.len()].copy_from_slice(&src[..]);`
   |
   = note: `#[warn(clippy::manual_memcpy)]` on by default

error: this public function might dereference a raw pointer but is not marked `unsafe`
  --> src/lib.rs:15:14
   |
15 |     unsafe { addr.read_volatile() }
   |              ^^^^
   |
   = note: `#[deny(clippy::not_unsafe_ptr_arg_deref)]` on by default

error: this comparison involving the minimum or maximum element for this type contains a case that is always true or always false
  --> src/lib.rs:20:5
   |
20 |     level <= 255
   |     ^^^^^^^^^^^^
   |
   = help: because `255` is the maximum value for this type, this comparison is always true
   = note: `#[deny(clippy::absurd_extreme_comparisons)]` on by default
```

Сам rustc из трёх ошибок заметил только последнюю, и то как
предупреждение `unused_comparisons`. Разберём находки clippy:

- `manual_memcpy`: цикл проверяет границы `dst` на каждой итерации и
  паникует посреди копирования, если `dst` короче. `copy_from_slice`
  проверяет длины один раз и вызывает `memcpy`.
- `not_unsafe_ptr_arg_deref`: самая серьёзная. `read_reg` — безопасная
  функция, но читает по любому адресу, который ей передали. Вызов
  `read_reg(1 as *const u32)` компилируется без единого `unsafe` и ведёт
  к UB. Блок `unsafe` внутри функции не делает её безопасной: обещание
  дать верный адрес должен дать вызывающий, значит, функция должна быть
  `unsafe fn` с разделом `# Safety` в документации. На этой границе
  ответственности построена вся часть III.
- `absurd_extreme_comparisons`: сравнение всегда истинно. Скорее всего,
  автор имел в виду другой предел или другой тип.

Линты clippy разбиты на группы:

| Группа | Уровень по умолчанию | Что ищет |
|---|---|---|
| `correctness` | deny | Код, который почти наверняка ошибочен |
| `suspicious` | warn | Код, который, вероятно, ошибочен |
| `style` | warn | Неидиоматичный код |
| `complexity` | warn | Излишне сложный код |
| `perf` | warn | Медленный код, у которого есть быстрый аналог |
| `pedantic` | allow | Строгие линты, возможны ложные срабатывания |
| `restriction` | allow | Запреты для отдельных проектов, включаются поштучно |
| `nursery` | allow | Линты в разработке |
| `cargo` | allow | Проблемы в `Cargo.toml` |

В примере `manual_memcpy` из группы `perf`, а два других из
`correctness`, поэтому это ошибки, а не предупреждения. CI книги запускает
`cargo clippy --workspace --all-targets -- -D warnings`: любое
предупреждение становится ошибкой сборки.

<div class="box trick">

**Трюк.** Если линт сработал на коде, который действительно нужен таким,
отключайте его не `#[allow]`, а `#[expect]` с причиной:

```rust
#[expect(clippy::manual_memcpy, reason = "демонстрация")]
```

Когда код исправят и линт перестанет срабатывать, компилятор скажет об
этом сам:

```text
warning: this lint expectation is unfulfilled
 --> src/lib.rs:7:10
  |
7 | #[expect(clippy::manual_memcpy, reason = "демонстрация")]
  |          ^^^^^^^^^^^^^^^^^^^^^
  |
  = note: демонстрация
```

`#[allow]` в той же ситуации молча остаётся в коде навсегда.

</div>

<div class="box check">

**Проверь себя.**

1. Почему висячий указатель в `first_after_push` читает правильное число,
   и при каких условиях он прочитает мусор?
2. Почему Miri в профиле dev не сообщает о второй ошибке сам, а в release
   говорит про `assume`?
3. ASan в release не нашёл выход за границу, который нашёл в dev. Какой
   вывод из этого делать о проверке оптимизированных сборок санитайзерами?
4. Почему clippy считает ошибкой `read_reg`, хотя разыменование в ней
   стоит внутри `unsafe`?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-build-script]: ../appendix-f.md#build-script "Сценарий сборки (build script): Программа build.rs в корне пакета, которую cargo собирает и запускает до компиляции крейта: генерирует исходники, собирает C-код, сообщает компилятору ключи и пути."
[g-ffi]: ../appendix-f.md#ffi "FFI (foreign function interface): Вызов функций, написанных на другом языке, и показ своих функций ему."
[g-proc-macro]: ../appendix-f.md#proc-macro "Процедурный макрос (procedural macro): Функция на Rust, которую компилятор вызывает при компиляции: на входе поток токенов, на выходе поток токенов."
[g-provenance]: ../appendix-f.md#provenance "Provenance (происхождение указателя): Невидимая часть указателя: к какому выделению памяти он относится и что через него можно трогать."
[g-raw-pointer]: ../appendix-f.md#raw-pointer "Сырой указатель (raw pointer): Указатель *const T или *mut T без гарантий ссылки: он может быть нулевым, висячим, невыровненным."
[g-slice]: ../appendix-f.md#slice "Срез (slice): Непрерывная последовательность элементов, длина которой известна только во время выполнения: [T], а str — срез байт UTF-8."
