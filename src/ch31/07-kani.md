# 31.7. Kani: для всех входов

Тест, proptest и фаззер проверяют входы, которые им попались. Проверка
моделей отвечает на другой вопрос: есть ли *хоть один* вход, на котором
проверка не проходит. Если нет — это доказательство для всех входов в
заданных границах, а не мера усилия. Если есть — инструмент выдаёт сам
вход.

<div class="box ref">

**Справка: как работает Kani.** Kani — подключаемый к cargo
проверяльщик моделей. Он компилирует крейт своим драйвером rustc в
программу на языке GOTO — промежуточном представлении CBMC, проверяльщика
моделей для C, которому больше двадцати лет. Значение `kani::any()` —
не случайное, а *символьное*: «любое значение типа». CBMC
разворачивает циклы до заданной границы, переводит программу вместе с
отрицанием всех проверок в одну логическую формулу и отдаёт её
SAT-решателю. Формула выполнима — значит, есть входы, на которых
проверка ломается, и решатель называет их. Невыполнима — таких входов
нет. Проверки Kani вставляет сам: переполнение, выход за границы,
деление на ноль, `unwrap` на `None`, любая паника, а в `unsafe` —
корректность указателей. Функция с атрибутом `#[kani::proof]` —
*доказательство* (harness): она задаёт символьные входы, ограничения на
них и вызывает проверяемый код. Собирается такой код только под
`cargo kani` — Kani задаёт `cfg(kani)`.

</div>

## Середина отрезка

Классическая ошибка двоичного поиска, которую почти двадцать лет носили
учебники и стандартные библиотеки:

```rust,ignore
{{#include ../../code/ch31/src/verify.rs:midpoint}}
```

```rust,ignore
{{#include ../../code/ch31/src/verify.rs:proof_midpoint}}
```

```console
$ cargo kani -p ch31 --harness verify::proofs::midpoint_v1_no_overflow --exact
...
Check 1: verify::midpoint_v1.assertion.1
	 - Status: FAILURE
	 - Description: "attempt to add with overflow"
	 - Location: code/ch31/src/verify.rs:8:5 in function verify::midpoint_v1

Check 2: verify::midpoint_v1.assertion.2
	 - Status: SUCCESS
	 - Description: "attempt to divide by zero"
	 - Location: code/ch31/src/verify.rs:8:5 in function verify::midpoint_v1
...
SUMMARY:
 ** 1 of 3 failed
Failed Checks: attempt to add with overflow
 File: "code/ch31/src/verify.rs", line 8, in verify::midpoint_v1

VERIFICATION:- FAILED
Verification Time: 0.02804056s
```

Проверки взяты из самого кода: сложение `u32` Kani проверяет на
переполнение при любом профиле сборки, деление на константу 2 — на ноль.
Исправленная версия:

```console
$ cargo kani -p ch31 --harness verify::proofs::midpoint_is_exact --exact
...
SUMMARY:
 ** 0 of 10 failed

VERIFICATION:- SUCCESSFUL
Verification Time: 0.07648707s
```

Восемь сотых секунды на все 2⁶⁴ пар. Перебором столько не пройти и за
века; решателю перебор не нужен — сложение, сдвиг и сравнение он видит
как формулу над 64 битами и показывает, что противоречия в ней нет.
Сравнение с `u32::midpoint` из std — ещё и проверка модели: два
независимых способа посчитать одно.

## Индукция вместо развёртки

CRC проекта считается таблицей, и в разделе 31.3 proptest сверял её с
побитовой версией на случайных данных. Kani может сделать то же для
всех данных — но не развернув цикл по произвольной длине, а доказав один
шаг:

```rust,ignore
{{#include ../../code/ch31/src/verify.rs:proof_crc}}
```

Проверка — для всех 2¹⁶ состояний и 2⁸ байтов, 0,09 секунды. Дальше
работает индукция на бумаге: обе функции обрабатывают данные байт за
байтом, начиная с одного состояния; раз шаг совпадает из любого
состояния, совпадает и результат после любого числа шагов. Так
доказательства и пишут: Kani проверяет утверждение о шаге, а человек —
что из шага следует целое.

## Разбор команд: контрпример

Раздел 31.4 оставил вопрос: цель фаззера на сырых байтах за три миллиона
запусков не нашла калибровку на девять точек. Kani на тех же сырых
байтах:

```rust,ignore
{{#include ../../code/ch31/src/verify.rs:proof_cmd}}
```

Буфер — 43 символьных байта и символьная длина до 43: это все входы
такой длины разом, включая все команды, — самая длинная, калибровка на
восемь точек, занимает 39 байт.

````console
$ cargo kani -p ch31 --harness verify::proofs::parse_v1_never_panics --exact \
>     -Z concrete-playback --concrete-playback=print
...
SUMMARY:
 ** 1 of 227 failed (3 unreachable)
Failed Checks: index out of bounds: the length is less than or equal to the given index
 File: "code/ch31/src/cmd.rs", line 166, in cmd::calibrate_v1

VERIFICATION:- FAILED
Verification Time: 49.432484s

Concrete playback unit test for `verify::proofs::parse_v1_never_panics`:
```
/// Test generated for harness `verify::proofs::parse_v1_never_panics`
///
/// Check for `assertion`: "index out of bounds: the length is less than or equal to the given index"

#[test]
fn kani_concrete_playback_parse_v1_never_panics_2218049127995399542() {
    let concrete_vals: Vec<Vec<u8>> = vec![
        // 90
        vec![90],
        // 3
        vec![3],
        // 39
        vec![39],
        // 55
        vec![55],
        // 15
        vec![15],
        // 9
        vec![9],
...
        // 253
        vec![253],
        // 43ul
        vec![43, 0, 0, 0, 0, 0, 0, 0],
    ];
    kani::concrete_playback_run(concrete_vals, parse_v1_never_panics);
}
```
````

Пятьдесят секунд — и вход, который фаззер не собрал. Значения идут в
порядке вызовов `kani::any()`: 43 байта буфера, потом длина. Байты —
`5a 03 27`: маркер, калибровка, длина нагрузки 39; датчик `0x0f37`
(3895); `n = 9`; 36 байт точек и CRC `0xfd`; длина входа 43. То, что
фаззеру нужно было угадать одновременно, — `n ≥ 9`, длину `3 + 4·n`,
столько же байтов и верную CRC, — решатель вывел из формулы: CRC для
него — такое же уравнение, как остальные.

*Concrete playback* превращает контрпример в обычный тест.
`--concrete-playback=inplace` вписывает его в исходник рядом с
доказательством, а `cargo kani playback` собирает и выполняет его
обычным rustc — с отладчиком, `println!` и полной паникой:

```console
$ cargo kani playback -Z concrete-playback -p ch31 -- kani_concrete_playback_parse_v1
running 1 test
test verify::proofs::kani_concrete_playback_parse_v1_never_panics_2218049127995399542 ... FAILED
...
thread 'verify::proofs::kani_concrete_playback_parse_v1_never_panics_2218049127995399542' (25135) panicked at code/ch31/src/cmd.rs:166:9:
index out of bounds: the len is 8 but the index is 8
...
   3: ch31::cmd::calibrate_v1
             at ./src/cmd.rs:166:9
   4: ch31::cmd::parse_with
             at ./src/cmd.rs:147:25
   5: ch31::cmd::parse_v1
             at ./src/cmd.rs:175:5
```

Сгенерированный тест читать неудобно — сорок четыре строки по байту.
Поэтому в `cmd.rs` его место занял тест `nine_points`, который строит ту
же команду через `encode`. Для исправленной `parse` то же
доказательство проходит:

```console
$ bash code/ch31/kani/run.sh
ok    ch31 midpoint_v1_no_overflow: FAILED (attempt to add with overflow), 0.041795276s
ok    ch31 midpoint_is_exact: SUCCESSFUL, 0.15399016s
ok    ch31 crc_table_step_matches_bitwise: SUCCESSFUL, 0.2531665s
ok    ch31 parse_v1_never_panics: FAILED (index out of bounds: the length is less than or equal to the given index), 53.20793s
ok    ch31 parse_never_panics: SUCCESSFUL, 62.133713s
ok    ch31 parse_v1_vacuous: SUCCESSFUL, 25.014898s
ok    ch31 parse_v1_vacuous: cover недостижим — входов нет
ok    контрпример: 5a 03 27, датчик 3895, 9 точек, длина 43
ok    ch31-unwind20 parse_never_panics: FAILED (unwinding assertion loop 1), 9.666424s
...
```

Время от прогона к прогону гуляет: три коротких доказательства в
прогоне скрипта шли в полтора-три раза дольше, чем поодиночке выше в
разделе, `parse_v1_never_panics` — на десятую долю. Строки
`parse_v1_vacuous` и `ch31-unwind20` разобраны ниже в этом разделе,
остальные строки отчёта — в разделе [31.10](10-project.md) и в задачах
C и D.

## Границы развёртки

CBMC не исполняет циклы — он их разворачивает: тело повторяется в
формуле столько раз, сколько указано, а после последней копии стоит
*проверка развёртки* — утверждение, что цикл к этому моменту кончился.
Без неё доказательство было бы честным только для коротких входов, а
сказать об этом было бы некому. `#[kani::unwind(44)]` у доказательства —
граница для всех его циклов: цикл CRC проходит не больше 41 байта. С
меньшей границей:

```console
$ cargo kani -p ch31 --harness verify::proofs::parse_never_panics --exact --unwind 20
...
SUMMARY:
 ** 1 of 227 failed (226 undetermined)
Failed Checks: unwinding assertion loop 1
 File: "code/ch31/src/cmd.rs", line 93, in cmd::crc8

VERIFICATION:- FAILED
[Kani] info: Verification output shows one or more unwinding failures.
[Kani] tip: Consider increasing the unwinding value or disabling `--unwinding-assertions`.
Verification Time: 9.266784s
```

Доказательство не прошло — и это правильно: 226 проверок
«неопределённы», потому что для входов длиннее двадцати байтов Kani
ничего не знает. Совет «disabling `--unwinding-assertions`» превратит
этот провал в ложное «успешно» для всех входов, на которых цикл
недоразвёрнут.

<div class="box trap">

**Ловушка: «доказано» — в каких границах.** Доказательство говорит ровно
о том, что в нём написано. `parse_never_panics` — о входах до 43 байт;
у входа длиннее поле длины доходит до 255, а ветки разбора те же, — но
это рассуждение, а не проверка. Ограничение `kani::assume` или
`any_where`, которое сужает входы сильнее, чем задумано, делает
доказательство пустым. Если ограничения противоречат друг другу,
проверять нечего:

```rust,ignore
{{#include ../../code/ch31/src/verify.rs:vacuous}}
```

```console
$ cargo kani -p ch31 --harness verify::proofs::parse_v1_vacuous --exact
...
SUMMARY:
 ** 0 of 227 failed (19 unreachable)

 ** 0 of 1 cover properties satisfied (1 unreachable)


VERIFICATION:- SUCCESSFUL
Verification Time: 13.041526s
```

«SUCCESSFUL» для функции с известной ошибкой. Недостижимыми Kani
пометил лишь 19 проверок из 227 — остальные «успешны», потому что
формула с ложным допущением невыполнима при любых проверках. Выдаёт
пустоту только `kani::cover!`: условие, которое должно быть выполнимо
хотя бы на одном входе, — здесь «0 of 1 cover properties satisfied».
Ставить его стоит в каждое доказательство с нетривиальными
ограничениями. И размер: с каждым байтом символьного входа и каждой
копией цикла формула растёт. Разбор потока кадров проекта, развёрнутый
целиком на 16 байт, CBMC не закончил и за 28 минут — в разделе
[31.10](10-project.md) его заменяет доказательство одного шага. Потоков
Kani не перебирает: для них — loom.

</div>

<div class="box c">

**Как в C.** CBMC — проверяльщик моделей для C, и под Kani работает он
же. Доказательство на C — функция `main` с произвольными значениями от
`nondet_*` и ограничениями `__CPROVER_assume`:

```c
{{#include ../../code/ch31/c/cbmc.c:harness}}
```

```console
$ cbmc cbmc.c cmd.c --bounds-check --pointer-check --unwind 44 --unwinding-assertions
...
[calibrate_v1.precondition_instance.3] line 24 memcpy destination region writeable: FAILURE
[calibrate_v1.precondition_instance.6] line 25 memcpy destination region writeable: FAILURE
** 2 of 198 failed (2 iterations)
VERIFICATION FAILED
```

Нашёл, и за 1,4 секунды: версия на C проще — ни срезов символьной
длины, ни итераторов, ни `Result`. Но какие проверки включить, решает
автор: без `--bounds-check --pointer-check` CBMC проверяет только
`assert`. И проверка здесь — «запись внутри объекта», как у ASan. Если
команда лежит внутри объемлющей структуры, как в `replay.c` из
раздела 31.4 (`-D IN_FRAME`), — `VERIFICATION SUCCESSFUL`: девятая точка
не выходит за объект, и для CBMC ошибки нет. В Rust `table.points[i]`
— индекс массива с проверкой границ в самом языке, и Kani находит
ошибку, где бы таблица ни лежала.

</div>

<div class="box check">

**Проверь себя.** 1. Почему `midpoint_is_exact` проходит за доли
секунды, хотя пар 2⁶⁴? 2. Доказательство `crc_table_step_matches_bitwise`
проверяет один байт. Почему этого достаточно для данных любой длины, и
какое свойство обеих функций для этого нужно? 3. Что ответит Kani, если
в `midpoint_v1_no_overflow` добавить
`kani::assume(a < 1 << 31 && b < 1 << 31)`? Доказывает ли это
что-нибудь о `midpoint_v1`? 4. Почему с `-D IN_FRAME` CBMC молчит, а
Kani для той же ошибки в Rust — нет?

</div>
