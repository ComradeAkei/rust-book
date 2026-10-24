# 31.6. Miri и loom

Санитайзер проверяет машинный код, который уже построил компилятор, и
видит то, что случилось на этом процессоре. Неопределённое поведение
определено не процессором, а языком: правило нарушено, даже если
x86-64 выполнил код «как задумано». Miri проверяет программу по правилам
Rust, loom — по модели памяти C11, перебирая чередования потоков.

<div class="box ref">

**Справка: Miri.** Интерпретатор промежуточного представления MIR — того,
во что rustc превращает программу до LLVM. Каждое выделение памяти для
Miri — отдельный объект с границами, каждый указатель несёт
*происхождение* (provenance): из какого выделения он получен и по какой
ссылке. Каждая операция проверяется: чтение неинициализированного,
выход за объект, обращение к освобождённому, невыровненный доступ,
недопустимое значение (`bool` со значением 2, ссылка на ноль),
нарушение правил алиасинга (модель Stacked Borrows по умолчанию, Tree
Borrows с `-Zmiri-tree-borrows`), гонка данных (векторные часы, как у
TSan), утечка при выходе. Порядок потоков Miri выбирает сам, случайно,
с вытеснением, а `Relaxed`- и `Acquire`-загрузки могут вернуть не
последнюю запись — так Miri приближает слабую модель памяти. Платит
скоростью: опыт `spsc` идёт около 8 секунд против 5 мс в debug. И выполняет
только код на Rust: вызов функции C или системный вызов, которого Miri
не эмулирует, — «unsupported operation».

</div>

## Опыты раздела 31.5 под Miri

`cargo +nightly miri run` запускает программу в Miri, `cargo miri test`
— тесты. Те же опыты, что под санитайзерами:

```console
$ bash code/ch31/miri/run.sh
ok    san uaf: код 1, «has been freed, so this pointer is dangling»
ok    san overflow: код 1, «unsafe precondition(s) violated: slice::get_unchecked requires that the index is within the slice»
ok    san ffi: код 1, «unsupported operation: can't call foreign function `fill_counter`»
ok    san leak: код 1, «error: memory leaked:»
ok    san race: код 1, «Data race detected between (1) non-atomic write on thread»
ok    san relaxed: код 1, «Data race detected between (1) retag write on thread»
ok    san spsc: код 0, «сумма: 49995000»
ok    san once: код 0, «999»
опыты главы 31 под Miri: вердикты совпали с текстом
```

Отчёт Miri привязан к строкам программы, а не к адресам:

```console
$ cargo +nightly miri run -q -p ch31 --bin san -- uaf
error: Undefined Behavior: memory access failed: alloc811 has been freed, so this pointer is dangling
  --> code/ch31/src/bin/san.rs:21:45
   |
21 |     println!("первый элемент: {}", unsafe { *first });
   |                                             ^^^^^^ Undefined Behavior occurred here
   |
...
help: alloc811 was allocated here:
  --> code/ch31/src/bin/san.rs:17:17
   |
17 |     let mut v = vec![1u32, 2, 3];
   |                 ^^^^^^^^^^^^^^^^
help: alloc811 was deallocated here:
  --> code/ch31/src/bin/san.rs:19:5
   |
19 |     v.extend(4..100);
   |     ^^^^^^^^^^^^^^^^
```

Как и `realloc` из ASan, перераспределение в Miri всегда даёт новое
выделение, так что ошибка видна при каждом запуске, а не только когда
куча легла неудачно. Выход за границу в профиле dev раньше Miri ловит
проверка предусловия `get_unchecked`. В `--release` её нет, и ошибку
находит сам Miri — `get_unchecked` сообщает оптимизатору, что индекс в
пределах, через `assume`:

```console
$ cargo +nightly miri run --release -q -p ch31 --bin san -- overflow
warning: Miri does not support optimizations: the opt-level is ignored. ...

error: Undefined Behavior: `assume` called with `false`
  --> code/ch31/src/bin/san.rs:33:39
   |
33 |     println!("v[{i}] = {}", unsafe { *v.get_unchecked(i) });
   |                                       ^^^^^^^^^^^^^^^^^^ Undefined Behavior occurred here
```

Вызов `fill_counter` Miri выполнить не может, и это не вердикт о
программе: «this means the program tried to do something Miri does not
support; it does not indicate a bug in the program». Код за FFI —
территория санитайзеров. Утечку Miri сообщает так же, как LSan, и её
можно выключить `-Zmiri-ignore-leaks`, если программа теряет память
намеренно.

От системы Miri программу по умолчанию изолирует: ни файловой системы,
ни переменных окружения, ни текущего каталога. Об это спотыкаются тесты
proptest. Перед первым случаем `TestRunner` ищет файл регрессий от
текущего каталога, и `cargo miri test` останавливается на «unsupported
operation: \`getcwd\` not available when isolation is enabled».
Сохранение выключает переменная `PROPTEST_DISABLE_FAILURE_PERSISTENCE`,
но из окружения `cargo` она в Miri не попадёт — её передают флагом
`-Zmiri-env-set=PROPTEST_DISABLE_FAILURE_PERSISTENCE=1`. И тогда
два небольших свойства раздела [31.3](03-proptest.md), SLIP и CRC, по
256 случаев идут под Miri почти две минуты, а проверка одного решения
задачи A — около минуты. Свойства проверяют логику, а не `unsafe`, и им
хватает `cargo test`; в главе они помечены `#[cfg_attr(miri, ignore)]`.

## Гонки и порядки

```console
$ cargo +nightly miri run -q -p ch31 --bin san -- race
error: Undefined Behavior: Data race detected between (1) non-atomic write on thread `unnamed-1` and (2) non-atomic read on thread `unnamed-2` at alloc835+0x10
  --> code/ch31/src/bin/san.rs:74:30
   |
74 |                     unsafe { *c.0.get() += 1 };
   |                              ^^^^^^^^^^^^^^^ (2) just happened here
   |
help: and (1) occurred earlier here
  --> code/ch31/src/bin/san.rs:74:30
```

Смещение `+0x10` — то же поле `ArcInner`, что в отчёте TSan. Для очереди
со всеми `Relaxed` отчёт интереснее:

```console
$ cargo +nightly miri run -q -p ch31 --bin san -- relaxed
error: Undefined Behavior: Data race detected between (1) retag write on thread `unnamed-1` and (2) retag read of type `std::mem::MaybeUninit<u32>` on thread `main` at alloc1194+0x10
   --> code/ch31/src/spsc.rs:107:53
    |
107 |         let v = self.buf[tail].with(|slot| unsafe { (*slot).assume_init_read() });
    |                                                     ^^^^^^^ (2) just happened here
    |
help: and (1) occurred earlier here
   --> code/ch31/src/spsc.rs:96:9
    |
 96 |         self.buf[head].with_mut(|slot| unsafe { (*slot).write(v) });
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    = help: retags occur on all (re)borrows and as well as when references are copied or moved
    = help: retags permit optimizations that insert speculative reads or writes
    = help: therefore from the perspective of data races, a retag has the same implications as a read or write
```

*Retag* — момент, когда модель алиасинга выдаёт ссылке новое
разрешение: при каждом заимствовании `&*slot` и `&mut *slot`. Miri
считает его обращением к памяти, потому что компилятору разрешено
читать по ссылке заранее — `&T` обещает, что память доступна и не
меняется, пока ссылка жива. Гонку Miri нашёл раньше, чем дошло до
самого чтения значения, — на создании ссылки на слот.

Опыт `once` под Miri проходит: Miri интерпретирует и код std, и
синхронизацию внутри `Once` видит так же, как нашу. Ложной гонки, как у
TSan над неинструментированной std, у него не бывает.

Порядок потоков Miri выбирает из генератора со своим зерном.
`-Zmiri-many-seeds=0..16` запускает программу с шестнадцатью зёрнами
подряд — дешёвый способ пройти больше расписаний:

```console
$ MIRIFLAGS=-Zmiri-many-seeds=0..4 cargo +nightly miri run -q -p ch31 --bin san -- once
Trying seed: 1
999
...
```

## loom: перебор чередований

Miri проходит одно расписание за запуск. loom перебирает *все*
различимые: тело `loom::model` выполняется снова и снова, и при каждом
обращении к атомику, `UnsafeCell` или мьютексу планировщик loom решает,
какой поток пойдёт дальше и какое из допустимых моделью C11 значений
вернёт загрузка. Чтобы loom видел обращения, код должен брать атомики,
`UnsafeCell` и потоки из loom. Очередь главы 21 для этого получила
прослойку под `--cfg loom`:

```rust,ignore
{{#include ../../code/ch31/src/spsc.rs:shim}}
```

`UnsafeCell` из loom не отдаёт указатель наружу: доступ — только внутри
замыкания `with`/`with_mut`, и loom знает, когда он начался и кончился.
Своя обёртка с тем же API для обычной сборки ничего не стоит. Порядки
доступа — параметр типа, так рядом с правильной очередью живёт
ошибочная:

```rust,ignore
{{#include ../../code/ch31/src/spsc.rs:orderings}}
```

```rust,ignore
{{#include ../../code/ch31/src/spsc.rs:spsc}}
```

Модель — писатель кладёт два элемента, читатель ждёт оба:

```rust,ignore
{{#include ../../code/ch31/tests/loom.rs:model}}
```

```console
$ RUSTFLAGS="--cfg loom" cargo test -p ch31 --release --test loom --target-dir target/loom -- --nocapture
running 3 tests
AcqRel: 82 исполнений
.
thread 'all_relaxed_is_caught' (23452) panicked at .../loom-0.7.2/src/rt/location.rs:115:9:
Causality violation: Concurrent read and write accesses.
...
  15: <ch31::spsc::Spsc<u32, 3, ch31::spsc::AllRelaxed>>::pop
             at ./src/spsc.rs:107:32
  16: loom::two_items::<ch31::spsc::AllRelaxed>::{closure#0}
             at ./tests/loom.rs:28:21
...
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
```

С `AcqRel` loom прошёл 82 исполнения и во всех очередь отдала `[1, 2]`.
С `AllRelaxed` — нашёл исполнение, где читатель видит новый `head`, а
запись слота не происходит раньше его чтения: «Causality violation».
Это не «повезло с расписанием»: такое исполнение loom строит
намеренно, перебирая точки ветвления, и находит его при каждом прогоне.
Ожидание в модели — с `thread::yield_now()`, а не пустой цикл: loom
выполняет потоки по очереди, и цикл без уступки для него бесконечен —
он остановится с «Model exceeded maximum number of branches. This is
often caused by an algorithm requiring the processor to make progress,
e.g. spin locks.» Флаг `--cfg loom` меняет все
крейты, поэтому у loom свой каталог сборки, а `--release` — потому что
перебор в debug заметно медленнее.

<div class="box hood">

**Под капотом: как loom не тонет в перестановках.** Наивный перебор
всех чередований растёт факториально. loom применяет *динамическое
сокращение частичных порядков* (DPOR): две операции, которые не
зависят друг от друга, — например, загрузки разных атомиков, —
переставлять незачем, исход тот же. Выполнив исполнение, loom смотрит,
где зависимые операции разных потоков можно было выполнить в другом
порядке, и только там ставит точки ветвления для следующих
исполнений. Ещё два ограничителя — переменные окружения
`LOOM_MAX_PREEMPTIONS` (сколько раз поток может быть вытеснен
посреди работы; на практике хватает двух-трёх) и `LOOM_MAX_BRANCHES`.

</div>

<div class="box trap">

**Ловушка: модель, которую loom проходит за одно исполнение.** Та же
очередь `AllRelaxed`, но читатель забирает один раз, не дожидаясь:

```rust,ignore
{{#include ../../code/ch31/tests/loom.rs:once}}
```

Тест проходит, и исполнение одно. Ошибочное чередование — писатель
публикует `head` до того, как читатель его загрузил, — в этой модели
есть, но loom 0.7.2 до него не доходит. У каждого атомика он помнит одно
последнее обращение (`last_dependent_access` в `rt/atomic.rs`). Читатель
загрузил `head` первым, затем писатель в `push` сам загрузил `head`, и
эта загрузка затёрла обращение читателя. Когда писатель записал `head`,
последним зависимым обращением оказалось его собственное — переставлять
нечего, точка ветвления не поставлена. Вывод практический: печатать
число исполнений и требовать, чтобы оно было больше единицы, как
`acq_rel_is_sound`, и строить модели, где потоки ждут друг друга в
цикле, — тогда точек ветвления больше. И не полагаться на один
инструмент: тот же вариант очереди Miri и TSan ловят сразу.

</div>

<div class="box c">

**Как в C.** Ближе всего к Miri — Valgrind memcheck: он выполняет
машинный код на виртуальном процессоре и следит за каждым байтом. Но он
знает только «доступно — не доступно» и «инициализировано — нет»;
происхождения указателей, алиасинга и того, что `assume` нарушен, для
него нет — это правила языка, а не машины. Гонки в Valgrind ищут Helgrind
и DRD. Для перебора чередований с атомиками C11 есть исследовательские
инструменты — CDSChecker, по статье о котором устроен loom, и GenMC, — но
в обиход они не вошли. Вся цепочка от `assume` до порядка `Relaxed` в C
проверяется в лучшем случае ревью.

</div>

<div class="box check">

**Проверь себя.** 1. В опыте `relaxed` Miri сообщает о «retag read»,
а не о чтении значения. Где в `(*slot).assume_init_read()` создаётся
ссылка и почему её создание — уже обращение? 2. Опыт `once` под TSan
без `-Zbuild-std` даёт гонку, под Miri — нет. Какой из двух
инструментов прав, и почему второй ошибается? 3. Почему модель
`single_pop_model_explores_once` проверяет меньше, чем `two_items`,
хотя ошибочное чередование есть в обеих?

</div>
