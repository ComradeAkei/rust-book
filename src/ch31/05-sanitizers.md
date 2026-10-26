# 31.5. Санитайзеры: ASan, LeakSanitizer, TSan

<div class="box ref">

**Справка: как устроены санитайзеры.** AddressSanitizer (ASan) — две
части. Компилятор перед каждым чтением и записью вставляет проверку
*теневой памяти*: на каждые 8 байт программы — байт тени по адресу
`(addr >> 3) + 0x7fff8000`, и в нём записано, сколько из этих 8 байт
доступно. Рантайм подменяет `malloc` и `free`: вокруг каждого блока
кладёт отравленные «[красные зоны][g-redzone]», а освобождённый блок держит в
карантине, не отдавая повторно, — поэтому выход за конец и обращение
после освобождения попадают в отравленную тень. Цена — примерно вдвое
медленнее и втрое больше памяти. [LeakSanitizer][g-lsan] (LSan) при выходе из
программы обходит память, как сборщик мусора, и сообщает о блоках, на
которые не осталось указателей; в сборке с ASan он включён по умолчанию.
[ThreadSanitizer][g-tsan] (TSan) помнит для каждых 8 байт последние обращения с
векторными часами потоков и строит отношение «происходит раньше» из
синхронизации: мьютексов, `join`, атомарных операций с их порядками. Два
обращения к одному месту из разных потоков, хотя бы одно — запись, и
ни одно не происходит раньше другого — гонка данных, даже если в этом
прогоне они не совпали по времени. TSan замедляет в 5–15 раз.

</div>

Безопасному коду Rust санитайзеры не нужны: всё, что они ловят,
компилятор там уже запретил. Нужны они там, где правила держит
программист: в `unsafe`, по ту сторону FFI и в коде, который строит
синхронизацию из [атомиков][g-atomic]. Программа `src/bin/san.rs` — по опыту на
каждый случай, и каждый без санитайзера отрабатывает «нормально».

## Сборка

Санитайзеры — флаг nightly-компилятора `-Zsanitizer=…`, тот же рантайм
из LLVM, что у clang:

```console
$ RUSTFLAGS=-Zsanitizer=address cargo +nightly build -Zbuild-std \
>     --target x86_64-unknown-linux-gnu --release --bin san
```

`--target` обязателен, даже если цель совпадает с хостом: с ним cargo не
передаёт `RUSTFLAGS` сценариям сборки и [процедурным макросам][g-proc-macro], а они
выполняются при сборке и инструментировать их незачем. `-Zbuild-std`
пересобирает std с тем же флагом — для ASan это желательно (ошибки
внутри std тоже видны), для TSan обязательно, о чём ниже. Тест
`tests/sanitizers.rs` делает то же из кода и сверяет отчёты:

```rust,ignore
{{#include ../../code/ch31/tests/sanitizers.rs:build}}
```

## Обращение после освобождения

[Сырой указатель][g-raw-pointer] на элемент вектора пережил перераспределение буфера:

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:uaf}}
```

Без санитайзера программа печатает `первый элемент: 1` и в debug, и в
release — и не потому, что байты старого блока ещё на месте. Блок здесь
никуда не делся: `realloc` из glibc нашёл место прямо за ним и расширил
его, адрес буфера не изменился, и указатель остался верным по
случайности. Ошибка от этого не исчезла — `extend` вправе перенести
буфер, и при другой истории кучи перенесёт. `realloc` из ASan переносит
блок всегда, а старый держит в карантине, — поэтому ошибка видна при
каждом запуске:

```console
$ target/.../release/san uaf
=================================================================
==19872==ERROR: AddressSanitizer: heap-use-after-free on address 0x7b6de9de0030 at pc 0x56089122e3b1 bp 0x7fff40780f90 sp 0x7fff40780f88
READ of size 4 at 0x7b6de9de0030 thread T0
    #0 0x56089122e3b0 in san::uaf code/ch31/src/bin/san.rs:21:58
    #1 0x56089122e3b0 in san::main code/ch31/src/bin/san.rs:144:18
...
0x7b6de9de0030 is located 0 bytes inside of 12-byte region [0x7b6de9de0030,0x7b6de9de003c)
freed by thread T0 here:
    #0 0x5608911f92f1 in realloc (target/.../release/san+0x1112f1)
    #1 0x56089122fd1e in alloc::alloc::realloc_nonnull library/alloc/src/alloc.rs:260:14
...
previously allocated by thread T0 here:
    #0 0x5608911f8ee4 in malloc (target/.../release/san+0x110ee4)
...
```

В отчёте три стека: где прочитали, где освободили, где выделили. Блок —
12 байт, три `u32` из `vec![1, 2, 3]`; освободил его `realloc` внутри
`extend`. Код выхода — 1.

## Выход за границу

`get_unchecked` снимает проверку границ, индекс приходит «снаружи»:

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:overflow}}
```

Тут без санитайзера два исхода. В [профиле][g-profile] dev `get_unchecked` сам
проверяет своё предусловие (`ub_checks` включены вместе с
`debug_assertions`) и прерывает программу:

```console
$ target/debug/san overflow
thread 'main' (20469) panicked at code/ch31/src/bin/san.rs:33:41:
unsafe precondition(s) violated: slice::get_unchecked requires that the index is within the slice
...
thread caused non-unwinding panic. aborting.
```

В release проверки нет, и программа печатает `v[16] = 0` — байт из
соседней памяти кучи. С ASan в release:

```console
$ target/.../release/san overflow
==19893==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x7b4f4ffe0040 at pc 0x55e08902634d bp 0x7ffc94a4a6f0 sp 0x7ffc94a4a6e8
READ of size 1 at 0x7b4f4ffe0040 thread T0
    #0 0x55e08902634c in san::overflow code/ch31/src/bin/san.rs:33:38
...
0x7b4f4ffe0040 is located 0 bytes after 16-byte region [0x7b4f4ffe0030,0x7b4f4ffe0040)
...
SUMMARY: AddressSanitizer: heap-buffer-overflow code/ch31/src/bin/san.rs:33:38 in san::overflow
```

«0 bytes after 16-byte region» — первый байт за концом блока, то есть
`v[16]`.

## Ошибка в C за FFI

Функция на C пишет на байт больше, чем обещает контракт:

```c
{{#include ../../code/ch31/c/fill.c:fill}}
```

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:ffi}}
```

Без санитайзера — `последний: 15`: семнадцатый байт лёг в хвост блока,
который аллокатор выделил с запасом, и никто этого не заметил. Чтобы
ASan увидел запись, проверки должны стоять в коде на C — их вставляет
компилятор C, а не rustc. Сценарий сборки об этом не заботится:

```rust,ignore
{{#include ../../code/ch31/build.rs:build_rs}}
```

cc 1.10 видит `address` в переменной `CARGO_CFG_SANITIZE` (cargo задаёт
её на nightly из `-Zsanitizer`) и сам добавляет компилятору C
`-fsanitize=address` — на x86-64 Linux и для GCC, и для clang, объекты
обоих работают с рантаймом из LLVM rustc. Здесь `cc` — GCC 13:

```console
$ RUSTFLAGS=-Zsanitizer=address cargo +nightly build -vv --target x86_64-unknown-linux-gnu --release --bin san
...
[ch31 0.1.0] running: "cc" "-O3" "-ffunction-sections" "-fdata-sections" "-fPIC" "-g" "-gdwarf-4" "-fno-omit-frame-pointer" "-mno-omit-leaf-frame-pointer" "-m64" "-fsanitize=address" "-Wall" "-Wextra" "-o" ... "-c" "c/fill.c"
```

```console
$ target/.../release/san ffi
==19903==ERROR: AddressSanitizer: heap-buffer-overflow on address 0x7bc776de0040 at pc 0x555591900f0e bp 0x7fff46695340 sp 0x7fff46695338
WRITE of size 1 at 0x7bc776de0040 thread T0
    #0 0x555591900f0d in fill_counter code/ch31/c/fill.c:9:16
    #1 0x5555918fc679 in san::ffi code/ch31/src/bin/san.rs:46:14
...
0x7bc776de0040 is located 0 bytes after 16-byte region [0x7bc776de0030,0x7bc776de0040)
allocated by thread T0 here:
    #0 0x5555918c80c2 in calloc (target/.../release/san+0x1110c2)
...
SUMMARY: AddressSanitizer: heap-buffer-overflow code/ch31/c/fill.c:9:16 in fill_counter
```

Стек проходит через границу языков: строка C, затем строка Rust, откуда
её вызвали; выделил вектор `calloc` — `vec![0u8; 16]` просит обнулённую
память. С `CFLAGS=-fno-sanitize=address` флаг отменяется, C собирается
без проверок, и та же программа под ASan снова печатает `последний: 15`
с кодом 0: рантайм перехватывает `malloc` и `memcpy`, но простую запись
`buf[i] = …` в неинструментированном коде не видит.

## Утечка

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:leak}}
```

Утечка в Rust не нарушает безопасности — `mem::forget` и `Box::leak`
не `unsafe`, — но LSan её находит:

```console
$ target/.../release/san leak
вектор забыт

=================================================================
==19914==ERROR: LeakSanitizer: detected memory leaks

Direct leak of 1000 byte(s) in 1 object(s) allocated from:
    #0 0x55a76a2590c2 in calloc (target/.../release/san+0x1110c2)
...
SUMMARY: AddressSanitizer: 1000 byte(s) leaked in 1 allocation(s).
```

Проверка идёт при выходе, после `main`: строка программы напечатана
раньше отчёта. «Direct» — на блок не указывает ничто; «Indirect» были
бы блоки, на которые указывают только утёкшие.

```rust,ignore
{{#include ../../code/ch31/tests/sanitizers.rs:asan_checks}}
```

## Гонка данных

`unsafe impl Sync` разрешает делиться значением, которое для этого не
предназначено, — и два потока увеличивают один счётчик без
синхронизации:

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:race}}
```

Без санитайзера в debug — `счётчик: 143768 из 200000`, `132019`,
`139824`: увеличения теряются. В release — `200000 из 200000` три раза
из трёх: LLVM свернул цикл в одну инструкцию `addq $0x186a0,0x10(%rdi)`
(0x186a0 = 100 000), окно гонки сузилось до неё, и ошибка исчезла из
виду. С TSan:

```console
$ target/.../release/san race
==================
WARNING: ThreadSanitizer: data race (pid=20285)
  Write of size 8 at 0x720800000030 by thread T2:
    #0 san::race::{closure#0}::{closure#0} code/ch31/src/bin/san.rs:74:30 (san+0xf5c2f)
...
  Previous write of size 8 at 0x720800000030 by thread T1:
    #0 san::race::{closure#0}::{closure#0} code/ch31/src/bin/san.rs:74:30 (san+0xf5c2f)
...
  Location is heap block of size 24 at 0x720800000020 allocated by main thread:
...
    #9 <alloc::rcs::arc::Arc<san::Racy>>::new library/alloc/src/rcs/arc.rs:441:25 (san+0xf9a54)
    #10 san::race code/ch31/src/bin/san.rs:67:19 (san+0xf9a54)
...
SUMMARY: ThreadSanitizer: data race code/ch31/src/bin/san.rs:74:30 in san::race::{closure#0}::{closure#0}
==================
счётчик: 200000 из 200000
ThreadSanitizer: reported 1 warnings
```

Счётчик под TSan сошёлся — а гонка найдена: TSan судит не по результату,
а по тому, упорядочены ли обращения. Блок в 24 байта — `ArcInner`: два
счётчика ссылок и `Racy`, поле на смещении 16. Код выхода — 66, так TSan
отмечает найденные гонки.

## Порядки атомарных операций

Очередь [SPSC][g-spsc] главы 21 в варианте раздела 31.6: порядки доступа — параметр
типа, и рядом с правильной очередью живёт вариант, где всё [`Relaxed`][g-memory-ordering].

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:spsc}}
```

С `AcqRel` TSan молчит: `сумма: 49995000`, код 0. С `AllRelaxed` сумма
та же — на x86-64 обычные `mov` и так упорядочены, — но:

```console
$ target/.../release/san relaxed
==================
WARNING: ThreadSanitizer: data race (pid=20295)
  Read of size 4 at 0x721000000010 by main thread:
    #0 core::ptr::read::<u32> library/core/src/ptr/mod.rs:1754:9 (san+0xf7af7)
...
    #3 <ch31::spsc::Spsc<u32, 8, ch31::spsc::AllRelaxed>>::pop::{closure#0} code/ch31/src/spsc.rs:113:61
...
  Previous write of size 4 at 0x721000000010 by thread T1:
    #0 <core::mem::maybe_uninit::MaybeUninit<u32>>::write library/core/src/mem/maybe_uninit.rs:575:9
    #1 <ch31::spsc::Spsc<u32, 8, ch31::spsc::AllRelaxed>>::push::{closure#0} code/ch31/src/spsc.rs:99:57
...
SUMMARY: ThreadSanitizer: data race library/core/src/ptr/mod.rs:1754:9 in core::ptr::read::<u32>
==================
сумма: 49995000
ThreadSanitizer: reported 1 warnings
```

Читатель увидел новый `head`, но `Relaxed`-загрузка не создаёт
отношения «происходит раньше» с записью слота у писателя — значит,
чтение слота с этой записью не упорядочено. На x86-64 это сойдёт с рук
до первой перестановки компилятором, на Cortex-M7 или ARMv8 — до первой
перестановки процессором. TSan видит ошибку по модели памяти, а не по
железу, на котором запущен.

```rust,ignore
{{#include ../../code/ch31/tests/sanitizers.rs:tsan_checks}}
```

## Почему TSan нужна `-Zbuild-std`

ASan собирается и поверх готовой std: тест `without_build_std` находит
`heap-use-after-free` и без пересборки. TSan nightly собирать отказывается:

```console
$ RUSTFLAGS=-Zsanitizer=thread cargo +nightly build --target x86_64-unknown-linux-gnu --release --bin san
error: mixing `-Zsanitizer` will cause an ABI mismatch in crate `ch31`
  |
  = help: the `-Zsanitizer` flag modifies the ABI so Rust crates compiled with different values of this flag cannot be used together safely
  = note: `-Zsanitizer=thread` in this crate is incompatible with `-Zsanitizer` being unset in dependency `std`
  = help: unset `-Zsanitizer` in this crate or set `-Zsanitizer=thread` in `std`
  = help: if you are sure this will not cause problems, you may use `-Cunsafe-allow-abi-mismatch=sanitizer` to silence this error
...
```

Та же ошибка — для `core`, `alloc`, `compiler_builtins` и остальных
крейтов std. Для TSan это не формальность: часть синхронизации, которую
он должен видеть, живёт в скомпилированной std. [Обобщённый][g-generic] и
`#[inline]` код std — `Arc`, атомики, большая часть `Mutex` — порождается
в нашем крейте и инструментирован, а необобщённый — нет. Опыт `once`
ошибки не содержит:

```rust,ignore
{{#include ../../code/ch31/src/bin/san.rs:once}}
```

С `-Zbuild-std` TSan молчит: `999`, код 0. Флаг
`-Cunsafe-allow-abi-mismatch=sanitizer` снимает ошибку сборки, и
программа собирается поверх готовой std, — тогда TSan находит гонку,
которой нет:

```rust,ignore
{{#include ../../code/ch31/tests/sanitizers.rs:mismatch}}
```

В отчёте — чтение полей `Vec` в `TABLE` основным потоком и запись их
потоком T1 в `MaybeUninit::write` внутри `OnceLock::initialize`. Эти
функции обобщённые и инструментированы. А публикует значение
Release-запись состояния в `std::sys::sync::once::futex::Once::call` —
`call_once_force` передаёт туда [замыкание][g-closure] как `&mut dyn FnMut`, функция
необобщённая и скомпилирована в std без TSan. `get` читает состояние
Acquire-загрузкой, но TSan не знает записи, с которой её связать. Опыты
`race`, `spsc` и `relaxed` в такой сборке дают те же вердикты, что с
`-Zbuild-std`, — ложный отчёт зависит от того, через какой код std прошла
синхронизация. Поэтому TSan — только с `-Zbuild-std`. Так же он нашёл
ошибку в [приложении C](../appendix-c.md) — `Rc` внутри [`Send`][g-send-sync]-типа.

<div class="box hood">

**Под капотом: что вставляет `-Zsanitizer=address`.** Перед чтением
4 байт по адресу `p` — загрузка байта тени `*((p >> 3) + 0x7fff8000)`;
ноль — все 8 байт доступны, и проверка на этом кончается, иначе
сравнение младших бит `p` с числом доступных байт и вызов
`__asan_report_load4`. Локальные массивы, адрес которых утекает,
переезжают в кадр с красными зонами — или в «поддельный стек» в куче,
если включено обнаружение обращений после возврата. Для глобальных
переменных компилятор добавляет зоны в секцию данных и регистрирует их
при старте. Всё это видно в `cargo asm` (раздел
[0.3](../ch00/03-asm.md)) — проверка тени занимает несколько инструкций
на каждое обращение.

</div>

<div class="box trap">

**Ловушка: санитайзер видит только выполненное.** Отчёта нет — значит,
ошибки не было *в этом прогоне на этих входах*. Ошибку в ветке, до
которой тесты не дошли, ASan не найдёт; поэтому его ставят под фаззер
(раздел [31.4](04-fuzz.md)), а не только под тесты. И не всё
выполненное: ASan следит за границами блоков, а не полей — запись за
массив внутри структуры, как калибровка на девять точек в C, остаётся
невидимой. TSan находит гонку, только если оба обращения выполнились и
второе застало первое в тени — там помнятся несколько последних
обращений к каждым 8 байтам; гонку в редкой ветке он пропустит так же.

Застать мало — нужен ещё стек прежнего обращения. TSan восстанавливает
его, проигрывая журнал событий потока, а журнал конечен (длину задаёт
`TSAN_OPTIONS=history_size` — сколько частей журнала добавить к трём
обязательным, по умолчанию 0). Если события в журнале уже нет, отчёт
выбрасывается молча. В первой версии опыта `spsc` ожидание было пустым
циклом со `spin_loop`, и на загруженной машине TSan о `relaxed` молчал: из
24 одновременных прогонов на четырёх ядрах гонку назвали три, на одном
ядре (`taskset -c 0`) — ни один. Писатель заполнял очередь и до конца
кванта крутил `push` — две загрузки индексов за оборот, каждая — событие
журнала, — и запись слота вытеснялась раньше, чем до неё добирался
читатель. С `history_size=4` на одном ядре отчёт появлялся в трёх прогонах
из четырёх, с `7` — в четырёх. Теперь ожидание — `thread::yield_now()`:
квант сразу уходит другому потоку, и под той же нагрузкой гонку называют
24 прогона из 24.

</div>

Есть и другие санитайзеры: `-Zsanitizer=memory` ([MemorySanitizer][g-msan],
чтение неинициализированной памяти; требует, чтобы инструментирован был
весь код, включая C), `hwaddress` (ASan на тегах в старшем байте
указателя, AArch64), `cfi` и `kcfi` (проверка целей косвенных вызовов).
Для `thumbv7m-none-eabi` rustc не поддерживает ни одного — в описании
цели (`rustc --print target-spec-json`) нет поля
`supported-sanitizers`, а у `x86_64-unknown-linux-gnu` их девять. Поэтому
логику прошивки, вынесенную за трейт, как в разделе
[31.2](02-no-std.md), запускают под санитайзерами на хосте.

<div class="box c">

**Как в C.** Санитайзеры пришли из C и C++: `-fsanitize=address`,
`-fsanitize=thread`, `-fsanitize=undefined` у GCC и clang — те же
рантаймы. Разница в том, где их применять. В C любая строка может
выйти за границы или прочитать освобождённое, и санитайзер нужен для
всего кода. В Rust — для блоков `unsafe`, кода за FFI и самодельной
синхронизации: искать есть где, но это видно по `grep unsafe`.
[UBSan][g-ubsan] в Rust почти не нужен: переполнение знаковых, сдвиг на ширину,
невыровненный доступ — в безопасном коде либо паника (в debug), либо
определённое поведение, а в `unsafe` неопределённое поведение лучше
ищет Miri (раздел [31.6](06-miri-loom.md)).

</div>

<div class="box check">

**Проверь себя.** 1. Почему `san uaf` без санитайзера печатает `1`, а не
мусор, и что изменится, если перед `extend` выделить ещё один небольшой
вектор? 2. Под TSan счётчик сошёлся, а отчёт есть. Что именно TSan счёл
гонкой? 3. Что увидит ASan в опыте `ffi`, если `fill.c` собрать без
`-fsanitize=address`, и почему? 4. Почему вариант очереди `AllRelaxed`
проходит на x86-64 миллион раз подряд и всё же ошибочен?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-atomic]: ../appendix-f.md#atomic "Атомик (atomic): Тип из core::sync::atomic (AtomicU32, AtomicBool и другие), операции над которым неделимы и упорядочены по выбранной модели памяти."
[g-closure]: ../appendix-f.md#closure "Замыкание (closure): Функция вместе со своим контекстом — захваченными переменными."
[g-generic]: ../appendix-f.md#generic "Обобщение (generic): Функция или тип с параметрами-типами: fn max(a: T, b: T) -> T."
[g-lsan]: ../appendix-f.md#lsan "LSan (LeakSanitizer): Санитайзер утечек: при выходе из программы ищет блоки кучи, на которые не осталось указателей."
[g-memory-ordering]: ../appendix-f.md#memory-ordering "Порядок памяти (memory ordering): Аргумент Ordering атомарной операции: какие из соседних обращений к памяти другие потоки обязаны увидеть до неё или после."
[g-msan]: ../appendix-f.md#msan "MSan (MemorySanitizer): Санитайзер чтения неинициализированной памяти."
[g-proc-macro]: ../appendix-f.md#proc-macro "Процедурный макрос (procedural macro): Функция на Rust, которую компилятор вызывает при компиляции: на входе поток токенов, на выходе поток токенов."
[g-profile]: ../appendix-f.md#profile "Профиль (profile): Именованный набор параметров компиляции cargo: уровень оптимизации, отладочная информация, проверки переполнения, стратегия паники."
[g-raw-pointer]: ../appendix-f.md#raw-pointer "Сырой указатель (raw pointer): Указатель *const T или *mut T без гарантий ссылки: он может быть нулевым, висячим, невыровненным."
[g-redzone]: ../appendix-f.md#redzone "Красная зона (redzone): Байты вокруг блока кучи, которые санитайзер помечает недоступными: обращение к ним — выход за границу блока."
[g-send-sync]: ../appendix-f.md#send-sync "Send, Sync: Автотрейты потокобезопасности."
[g-spsc]: ../appendix-f.md#spsc "SPSC (single-producer single-consumer): Очередь с одним писателем и одним читателем."
[g-tsan]: ../appendix-f.md#tsan "TSan (ThreadSanitizer): Санитайзер гонок данных: следит за обращениями к памяти из разных потоков и сообщает о парах конфликтующих доступов без синхронизации."
[g-ubsan]: ../appendix-f.md#ubsan "UBSan (UndefinedBehaviorSanitizer): Санитайзер GCC и Clang, который ловит неопределённое поведение C во время выполнения: знаковое переполнение, сдвиг на ширину типа и больше, деление на ноль, невыровненный указатель."
