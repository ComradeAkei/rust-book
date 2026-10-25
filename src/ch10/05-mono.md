# 10.5. Мономорфизация и её цена

## Копия на каждый тип

Контрольная сумма для отсчётов любой ширины:

```rust
{{#include ../../code/ch10/src/mono.rs:checksum}}
```

Компилятор создаёт копию `checksum` для каждого типа `T`, с которым её
вызвали, — это *мономорфизация* (monomorphization). Копии видно в таблице символов
библиотеки, собранной для Cortex-M3:

```console
$ cargo build --release --lib -p ch10 --no-default-features --target thumbv7m-none-eabi
$ nm -C -S --size-sort target/thumbv7m-none-eabi/release/libch10.rlib 2>/dev/null | grep -i 'checksum\|ch10_sum'
00000001 0000000c T ch10_sum_char
00000001 0000000c T ch10_sum_u16
00000001 0000000c T ch10_sum_u32
00000001 0000000c T ch10_sum_u8
00000001 0000000e T ch10_sum_u8_again
00000001 00000072 T ch10::mono::checksum::<u16>
00000001 0000007e T ch10::mono::checksum::<char>
00000001 0000007e T ch10::mono::checksum::<u32>
00000001 00000082 T ch10::mono::checksum::<u8>
```

Пять вызывающих, четыре копии: второй вызов с `u8` новой не создал,
`ch10_sum_u8_again` зовёт ту же `checksum::<u8>`. Каждая копия занимает
114–130 байт, потому что LLVM развернул цикл вчетверо, и отличаются они
инструкцией загрузки: `ldrb` для `u8`, `ldrh` для `u16`, `ldr` для `u32`
и `char`. Фрагмент копии для `u16`:

```text
.LBB3_4:
	ldrh r4, [r2, #8]!
	adds r3, #4
	cmp lr, r3
	add r1, r4
	ldrh r4, [r2, #2]
	add r1, r4
	ldrh r4, [r2, #4]
	add r1, r4
	ldrh r4, [r2, #6]
	add r1, r4
	bne .LBB3_4
```

Тип-аргумент записан и в имени символа. `nm` без `-C` показывает имена
в схеме v0, которая в Rust 1.97 включена по умолчанию:

```text
_RINvNtCskN87qZUIOn3_4ch104mono8checksumhEB4_
_RINvNtCskN87qZUIOn3_4ch104mono8checksumtEB4_
_RINvNtCskN87qZUIOn3_4ch104mono8checksummEB4_
_RINvNtCskN87qZUIOn3_4ch104mono8checksumcEB4_
```

`I…E` — инстанциация обобщённого пути, `4ch10`, `4mono`, `8checksum` —
его части с длинами, `Cs…_` — хеш крейта, а буква перед `E` — тип: `h` —
`u8`, `t` — `u16`, `m` — `u32`, `c` — `char`. Хвост `B4_` — ссылка назад
на крейт, в котором копия создана: здесь это тот же `ch10`. По таким
именам копии и ищут: `nm | grep 8checksum` найдёт все.

<div class="box hood">

**Под капотом.** Копии создаются там, где обобщённую функцию вызвали с
конкретным типом, — в том крейте и в той единице кодогенерации, где
стоит вызов. Если `checksum::<u8>` нужна двум крейтам, у каждого может
оказаться своя. Лишние копии убирают два механизма. Проход LLVM
MergeFunctions, который rustc включает по умолчанию, сливает функции с
одинаковым промежуточным представлением: вторая становится псевдонимом
первой. Компоновщик с ключом `--icf=all` (identical code folding, есть в
lld и gold) сливает функции с одинаковым машинным кодом, даже если IR у
них разный. Ни то ни другое не обещано: одинаковый с виду код может
отличаться метаданными, выравниванием или порядком блоков.

</div>

## Сколько это во flash

Список функций прошивки с размерами показывает `cargo bloat`
(устанавливается `cargo install cargo-bloat`, работает и для целей
Cortex-M):

```console
$ cd firmware && cargo bloat --release -p ch10-typestate -n 12
File  .text   Size                Crate Name
0.2%  21.6%   488B                  std <core::fmt::Formatter>::pad_integral
0.2%  18.5%   418B       ch10_typestate ch10_typestate::__cortex_m_rt_main
0.1%  11.7%   264B                  std core::fmt::write
0.1%  10.0%   226B                  std <u32 as core::fmt::Display>::fmt
0.1%   8.2%   186B cortex_m_semihosting <cortex_m_semihosting::hio::HostStream as core::fmt::Write>::write_char
0.1%   7.9%   178B                 ch10 <ch10::typestate::Uart<ch10::typestate::Enabled> as core::fmt::Write>::write_char
0.1%   6.1%   138B                  std <u32 as core::fmt::LowerHex>::fmt
0.0%   3.5%    78B cortex_m_semihosting <cortex_m_semihosting::hio::HostStream as core::fmt::Write>::write_str
0.0%   3.1%    70B                 ch10 <ch10::typestate::Uart<ch10::typestate::Enabled> as core::fmt::Write>::write_str
0.0%   3.0%    68B                  std <core::fmt::Formatter>::pad_integral::write_prefix
0.0%   1.8%    40B          cortex_m_rt Reset
0.0%   1.2%    28B cortex_m_semihosting <cortex_m_semihosting::hio::HostStream as core::fmt::Write>::write_fmt
0.0%   2.2%    50B                      And 5 smaller methods. Use -n N to show more.
0.9% 100.0% 2.2KiB                      .text section size, the file size is 246.2KiB
```

Мономорфизованные функции подписаны вместе с типами-аргументами:
`Uart<Enabled>` из раздела [10.7](07-typestate.md) получил свои
`write_char` и `write_str`. Колонка Crate — догадка `cargo bloat` по
имени символа. `write_char` — метод по умолчанию трейта
`core::fmt::Write`, его тело написано в `core`, но копия для
`Uart<Enabled>` приписана `ch10`, крейту типа. А `std` у `cargo bloat`
обозначает всю стандартную библиотеку, включая `core`. `cargo bloat
--crates` суммирует размер по крейтам, а `nm -S --size-sort` даёт то же без установки, но без
группировки. Для времени сборки есть `cargo llvm-lines`: он считает
строки LLVM IR по функциям вместе со всеми копиями и показывает, какая
обобщённая функция порождает больше всего кода.

## Внутренняя необобщённая функция

Обёртка кадра в SLIP (RFC 1055): байты `END` и `ESC` внутри данных
экранируются, в конце ставится `END`. Данные могут прийти срезом,
массивом, строкой или структурой с `AsRef<[u8]>`:

```rust
{{#include ../../code/ch10/src/mono.rs:slip}}
```

В `slip_fat` от `D` зависит только первая строка, `data.as_ref()`, но
обобщено всё тело, и каждый тип данных получает свою копию целиком. В
`slip_thin` обобщённая часть сводится к вызову `as_ref`, а работу делает
вложенная функция `inner` без параметров типа — она существует в одном
экземпляре. Четыре вызывающих с четырьмя типами:

```rust
{{#include ../../code/ch10/src/mono.rs:slip-users}}
```

Сборка для Cortex-M3 с `opt-level = "s"`, обычным для прошивок:

```console
$ cargo build --release --lib -p ch10 --no-default-features --target thumbv7m-none-eabi \
    --target-dir target/opt-s --config 'profile.release.opt-level="s"'
$ nm -C -S --size-sort target/opt-s/thumbv7m-none-eabi/release/libch10.rlib 2>/dev/null | grep slip
00000001 0000000c T ch10::mono::slip_thin::<&[u8]>
00000001 0000000c T ch10::mono::slip_thin::<&str>
00000001 00000012 T ch10::mono::slip_thin::<&ch10::mono::Payload>
00000001 00000018 T ch10::mono::slip_thin::<[u8; 4]>
00000001 0000005a T ch10_slip_fat
00000001 00000066 T ch10_slip_thin
00000001 00000088 T ch10::mono::slip_fat::<&ch10::mono::Payload>
00000001 0000008a T ch10::mono::slip_fat::<[u8; 4]>
00000001 00000090 T ch10::mono::slip_fat::<&[u8]>
00000001 00000090 T ch10::mono::slip_fat::<&str>
00000001 00000090 T ch10::mono::slip_thin::inner
```

| | Копии тела | Обёртки | Вызывающий | Всего |
|---|---|---|---|---|
| `slip_fat` | 4 × 136–144 = 562 | — | 90 | 652 байта |
| `slip_thin` | 1 × 144 | 12 + 12 + 18 + 24 = 66 | 102 | 312 байт |

Тело одно на четыре типа, и код сократился вдвое. С каждым новым типом
данных `slip_fat` прибавляет ещё 136–144 байта, а `slip_thin` — от 12 до
24. Тест `rustc_inner_function_shrinks` в `code/ch10/tests/cost.rs`
собирает этот вариант закреплённым тулчейном и проверяет, что копий
`slip_fat` четыре, а `slip_thin` меньше `slip_fat` хотя бы в полтора
раза.

С `opt-level = 3`, который у release по умолчанию, картина другая: LLVM
встраивает и `slip_fat`, и `inner` во все четыре места вызова. Обе
функции-вызывающих выходят по 1274 байта, а отдельная `inner` остаётся в
библиотеке для других крейтов, которые вызовут `slip_thin` со своими
типами. Приём экономит место там, где
компилятор сам его экономит, при `"s"` и `"z"`; при `3` он только не
мешает.

<div class="box trick">

**Трюк.** Так устроена стандартная библиотека. `std::fs::read<P:
AsRef<Path>>(path: P)` состоит из одной строки `inner(path.as_ref())`, а
вся работа с файлом — в необобщённой `fn inner(path: &Path)`. Признак,
что приём нужен: обобщённый параметр используется только в начале тела,
чтобы привести аргумент к одному типу (`as_ref`, `into`,
`into_iter().collect()`), а дальше идёт код, которому `T` не нужен.

</div>

<div class="box trap">

**Ловушка.** `#[inline(never)]` на `checksum` в этом разделе стоит
нарочно, чтобы копии остались отдельными функциями и их было видно в
`nm`. Без него маленькая копия, вызванная один раз, встраивается в
вызывающего и как отдельный символ исчезает, хотя её код в прошивке
остаётся. Число копий в `nm` и число мест, где код размножен, — не одно
и то же.

</div>

<div class="box check">

**Проверь себя.** Почему копии для `u32` и `char` одного размера, но обе
остались в библиотеке? Сколько байт добавит к `ch10_slip_fat` при
`opt-level = "s"` пятый вызов с `Vec<u8>`, а сколько — к
`ch10_slip_thin`? Какая буква будет в имени v0 для `checksum::<u64>`?

</div>
