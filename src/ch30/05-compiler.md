# 30.5. Что умеет компилятор: встраивание, LTO, `#[cold]`, PGO, `target-cpu`

Раздел [10.6](../ch10/06-inlining.md) объяснил правила: обобщённые
функции и функции с `#[inline]` везут свой MIR в метаданных крейта и
встраиваются в чужой крейт, остальные — только самые маленькие листовые
или при LTO. Здесь — как это выглядит в коде и что из этого стоит
времени.

## Встраивание через границу крейта

Крейт `ch30-dep` играет чужую библиотеку:

```rust,ignore
{{#include ../../code/ch30-dep/src/lib.rs:dep}}
```

Крейт главы зовёт каждую функцию из своей, не встраиваемой:

```rust,ignore
{{#include ../../code/ch30/src/inlining.rs:callers}}
```

Одну функцию, как везде в книге, покажет `cargo asm` (раздел
[0.3](../ch00/03-asm.md)): `cargo asm -p ch30 --lib
ch30::inlining::call_sum 0` — номер нужен, потому что под это имя
подходят ещё две функции. Тесты главы стороннего инструмента не
требуют и снимают весь крейт сразу, через `cargo rustc` — ключи после
`--` уходят rustc только этого крейта:

```console
$ cargo rustc -q -p ch30 --release --lib -- --emit asm -C codegen-units=1 -C debuginfo=0 -C llvm-args=-x86-asm-syntax=intel
$ llvm-cxxfilt < target/release/deps/ch30-*.s | less
```

Синтаксис Intel, отладочная информация выключена, чтобы в листинге не
было директив `.loc`, имена пропущены через `llvm-cxxfilt`. Так сняты
и листинги раздела 30.1. Функции `ch30::inlining` (тест
`rustc_cross_crate_inlining`):

```text
ch30::inlining::call_scale:
        lea eax, [rdi + 2*rdi]
        inc eax
        ret

ch30::inlining::call_sum:
        jmp qword ptr [rip + ch30_dep::scaled_sum@GOTPCREL]
```

`scale` — лист без вызовов, и rustc сам отдал её MIR в метаданные:
`x * 3 + 1` встроено как `lea` и `inc`. `scaled_sum` внутри зовёт
итератор и `scale` — автоматическое правило её не берёт, и `call_sum`
— переход в чужой крейт (`jmp`, а не `call`: вызов последний, и его
заменили переходом). Внутри `ch30-dep` её тело векторизовано, но
`call_sum` об этом ничего не знает и для короткого среза заплатит за
вызов. С `#[inline]` — то же тело, но в нашем крейте:

```text
ch30::inlining::call_sum_inline:
        test rsi, rsi
        je .LBB66_1
        cmp rsi, 8
        jae .LBB66_5
        ...
.LBB66_6:
        movdqu xmm3, xmmword ptr [rdi + rdx]
        movdqu xmm4, xmmword ptr [rdi + rdx + 16]
        paddd xmm1, xmm3
        paddd xmm3, xmm3
        paddd xmm1, xmm3
        paddd xmm0, xmm4
        paddd xmm4, xmm4
        paddd xmm0, xmm4
        psubd xmm1, xmm2
        psubd xmm0, xmm2
        add rdx, 32
        cmp rax, rdx
        jne .LBB66_6
```

Восемь чисел за оборот: `x + 2x` двумя `paddd` и `+1` как вычитание
вектора из единиц (`pcmpeqd xmm2, xmm2` дал все биты 1, то есть −1 в
каждом слове). А обобщённая версия встроилась в точно такое же тело, и
LLVM это заметил:

```text
_RNvNtCs67eQXqKoQEU_4ch308inlining16call_sum_generic = _RNvNtCs67eQXqKoQEU_4ch308inlining15call_sum_inline
```

Две функции с одинаковым машинным кодом слиты в одну: второе имя —
псевдоним первого. Это оптимизация слияния функций (MergeFunctions),
и в замерах она подстерегает: две «разные» функции, которые сравнивают,
могут оказаться одной.

## LTO и единицы кодогенерации

LTO (link-time optimization) откладывает оптимизацию до компоновки:
LLVM получает все крейты программы сразу и встраивает через любые
границы. Профиль для этого — несколько строк в корневом `Cargo.toml`:

```toml
# Раздел 30.5: release с LTO по всей программе и одной единицей кодогенерации.
# Профиль наследует release и меняет только то, что меряет глава.
[profile.release-lto]
inherits = "release"
lto = "fat"
codegen-units = 1
```

`codegen-units = 1` — второй рычаг: release по умолчанию делит крейт на
16 частей, чтобы собирать их параллельно, и между частями встраивание
слабее. Программа `inlining` в этом профиле:

```console
$ cargo build -q -p ch30 --profile release-lto --bin inlining
$ cargo rustc -q -p ch30 --profile release-lto --bin inlining -- --emit asm -C llvm-args=-x86-asm-syntax=intel
```

```text
ch30::inlining::call_sum:
        test rsi, rsi
        je .LBB22_1
        cmp rsi, 8
        jae .LBB22_5
        ...
.LBB22_6:
        movdqu xmm3, xmmword ptr [rdi + rdx]
        movdqu xmm4, xmmword ptr [rdi + rdx + 16]
        paddd xmm1, xmm3
        paddd xmm3, xmm3
        paddd xmm1, xmm3
```

`call_sum` без всякого `#[inline]` теперь такой же, как
`call_sum_inline`: `--emit asm` у программы с LTO выдаёт код после LTO
(файл `target/release-lto/deps/inlining-*.s`, тест
`rustc_lto_inlines_the_other_crate`). Бинарник — 2,1 МБ вместо 5,2 МБ
(оба с отладочной информацией, `debug = true` в release корневого
`Cargo.toml`), но код сократился мало: секция `.text` — 242 КБ против
253 КБ. Ушла в основном отладочная информация, секции `.debug_*` — 1,8
МБ вместо 4,7. Без LTO компоновщик берёт готовые объектные файлы std:
невызываемые функции он выбрасывает (`--gc-sections`), а их DWARF
оставляет. С LTO код std проходит через LLVM вместе с нашим — rlib
std везут и биткод, — и отладочная информация пишется только для того,
что осталось. Цена — время сборки: вся оптимизация в одном потоке в
конце. Выигрыш во времени бывает большим там, где горячий путь
проходит через много маленьких функций разных крейтов. На станции
(раздел [30.9](09-project.md)) LTO ускорил быстрые варианты приёма на
10–15 %: разбор кадра, который в release остаётся отдельной функцией,
с LTO встроен в цикл приёма.

<div class="box hood">

**Под капотом: thin и fat.** `lto = "fat"` сливает модули всех крейтов
в один и оптимизирует целиком. `lto = "thin"` оставляет модули
раздельными, но даёт каждому сводку о других и встраивает по ней;
параллельно и почти так же хорошо. Без `lto` в профиле release cargo
всё равно включает «локальный thin LTO» — между единицами
кодогенерации одного крейта, но не между крейтами; при одной единице
или `opt-level = 0` делать ему нечего, и его нет. `lto = "off"`
выключает и его.

</div>

<div class="box trap">

**Ловушка: листинг не того бинарника.** Если профиль не задал
`codegen-units` явно, rustc при `--emit asm` молча берёт одну единицу
кодогенерации: так весь крейт ложится в один файл `.s`. С явным
`-C codegen-units=16` файлов шестнадцать, по одному на часть. Значит,
листинг обычного release собран не так, как сам release из шестнадцати
частей, и встраивание между частями в них может различаться. Тесты
главы задают одну единицу явно и смотрят на неё; чтобы увидеть код
именно того, что запускается, дизассемблируют сам бинарник:
`objdump -d --no-show-raw-insn -M intel target/release/inlining`.

</div>

## `#[cold]` и `cold_path`

Ветка, которая почти никогда не выполняется, должна стоять в стороне:
горячий путь — подряд, без взятых переходов, плотно в кэше команд.

```rust,ignore
{{#include ../../code/ch30/src/inlining.rs:cold}}
```

```text
ch30::inlining::check_plain:
        cmp edi, 1000
        jbe .LBB64_1
        push rax
        call ch30::inlining::report
        xor eax, eax
        add rsp, 8
        ret
.LBB64_1:
        lea eax, [rdi + 2*rdi]
        ret

ch30::inlining::check_cold:
        cmp edi, 1000
        ja .LBB62_2
        lea eax, [rdi + 4*rdi]
        ret
.LBB62_2:
        push rax
        call ch30::inlining::report_cold
        ...

ch30::inlining::check_hint:
        cmp edi, 1000
        ja .LBB63_2
        lea eax, [8*rdi]
        sub eax, edi
        ret
.LBB63_2:
        ...
```

Без подсказки LLVM не знает, какая ветка частая, и поставил подряд
ветку ошибки: горячий путь `check_plain` — взятый переход `jbe` и
`lea` где-то ниже. Вызов `#[cold]`-функции делает холодной и ветку, из
которой она вызвана: в `check_cold` подряд идёт `x * 5` и `ret`, а
ошибка — за переходом `ja`. `std::hint::cold_path()` (стабилен с
Rust 1.95) делает то же прямо в ветке, без отдельной функции. Тест
`rustc_cold_paths`.

Множители у трёх функций разные (3, 5, 7) не случайно: с одинаковыми
телами LLVM слил бы функции в одну, как обобщённую выше, — и
раскладка «холодной» функции оказалась бы раскладкой псевдонима.

На время одиночного вызова раскладка почти не влияет: предсказатель
переходов угадывает и взятый, и невзятый переход одинаково хорошо. Она
важна для кэша команд и пропускной способности фронтенда, когда
горячего кода много, — то есть в больших программах, а не в
микрозамерах.

## PGO: раскладка по профилю

Подсказки можно не писать, а собрать: PGO (profile-guided optimization)
собирает программу со счётчиками переходов, запускает её на типичной
нагрузке и собирает ещё раз, раскладывая код по профилю. Нагрузка
программы `inlining` — миллион вызовов `check_*` со значениями в
диапазоне. Скрипт `code/ch30/pgo/run.sh`:

```bash
{{#include ../../code/ch30/pgo/run.sh:steps}}
```

`llvm-profdata` должен быть той же версии LLVM, что у rustc, — его даёт
`rustup component add llvm-tools` (в
`$(rustc --print sysroot)/lib/rustlib/<цель>/bin`).

```console
$ bash code/ch30/pgo/run.sh
профиль: 2 файл(а) .profraw, 141600 байт .profdata
--- check_plain без профиля
cmp edi, 1000
jbe .LBB64_1
push rax
call _RNvNtCs67eQXqKoQEU_4ch308inlining6report
xor eax, eax
add rsp, 8
ret
.LBB64_1:
lea eax, [rdi + 2*rdi]
ret
--- check_plain с профилем
cmp edi, 1000
ja .LBB62_2
lea eax, [rdi + 2*rdi]
ret
.LBB62_2:
push rax
call _RNvNtCs67eQXqKoQEU_4ch308inlining11report_cold
xor eax, eax
add rsp, 8
ret
ok
```

Файлов `.profraw` два: второй записал сценарий сборки (`build.rs`)
зависимости. `RUSTFLAGS` без `--target` действуют и на сценарии сборки,
так что он тоже собран со счётчиками и записал их при запуске;
профилю это не мешает — функций сценария в библиотеке нет.

С профилем `check_plain` разложен как `check_cold`: ветка ошибки ни разу
не выполнилась, и LLVM унёс её за переход. Заодно `report` пропала:
профиль сделал её холодной, её код совпал с `report_cold`, и функции
слились — `check_plain` теперь зовёт `report_cold`. Скрипт проверяет
оба перехода и идёт в CI.

Профиль привязан к коду: единицы кодогенерации и отладочная информация
должны совпадать во всех сборках, иначе LLVM отбрасывает счётчики
функций с предупреждением «function control flow change detected (hash
mismatch)» — поэтому скрипт задаёт `-Ccodegen-units=1` во всех трёх. На
станции (раздел 30.9) PGO остался в пределах шума: его
сильная сторона — большие программы со сложным ветвлением, компиляторы
и базы данных, где горячих путей много и угадать их подсказками
вручную нельзя. Так, с PGO собирают сам rustc для x86-64 Linux.

## `target-cpu`: новые инструкции

По умолчанию rustc собирает под базовый x86-64 — SSE2, без AVX. В нём
нет умножения 64-битных чисел вектором, и сумма квадратов из раздела
30.1 собирает его из 32-битных половин:

```text
.LBB69_6:
        movdqu xmm2, xmmword ptr [rdi + rdx]
        movdqu xmm3, xmmword ptr [rdi + rdx + 16]
        movdqa xmm4, xmm2
        psrlq xmm4, 32
        pmuludq xmm4, xmm2
        psllq xmm4, 33
        pmuludq xmm2, xmm2
        paddq xmm1, xmm2
        paddq xmm1, xmm4
        ...
```

x² для x = 2³²·h + l — это l² + 2·(h·l)·2³² по модулю 2⁶⁴: `pmuludq`
перемножает младшие половины, сдвиг на 33 — это «·2·2³²». С
`-C target-cpu=emeraldrapids` (это `native` машины, где снята глава;
тест задаёт имя явно, чтобы не зависеть от машины CI):

```text
.LBB69_8:
        vmovdqu ymm4, ymmword ptr [rdi + r8]
        vmovdqu ymm5, ymmword ptr [rdi + r8 + 32]
        vmovdqu ymm6, ymmword ptr [rdi + r8 + 64]
        vmovdqu ymm7, ymmword ptr [rdi + r8 + 96]
        vpmullq ymm4, ymm4, ymm4
        vpmullq ymm5, ymm5, ymm5
        vpaddq ymm0, ymm4, ymm0
        vpaddq ymm1, ymm5, ymm1
```

`vpmullq` из AVX-512DQ (с расширением VL — и на 256-битных регистрах)
умножает четыре 64-битных числа одной инструкцией, оборот — 16 чисел.
Регистры — `ymm`, 256 бит, а не `zmm`: в настройках LLVM для этих
процессоров стоит признак `prefer-256-bit` — память о том, что ядра
Intel прежних поколений на 512-битных векторах заметно снижали
частоту. С `-C target-feature=-prefer-256-bit` в том же цикле
появляются `vpmullq zmm4, zmm4, zmm4` (rustc предупредит, что признак
нестабилен, но передаст его LLVM). Тест `rustc_native_avx512` сверяет,
что по умолчанию `zmm` нет.

```console
$ RUSTFLAGS="-C target-cpu=native" cargo build -q -p ch30 --release --bin timing --target-dir target/native
$ for i in 1 2 3; do target/release/timing | grep sum_squares; target/native/release/timing | grep sum_squares; done
sum_squares, 10 000 чисел: медиана 3745 нс (мин 3215, MAD 313), 0.375 нс на число
sum_squares, 10 000 чисел: медиана 2940 нс (мин 2308, MAD 190), 0.294 нс на число
sum_squares, 10 000 чисел: медиана 6191 нс (мин 5492, MAD 300), 0.619 нс на число
sum_squares, 10 000 чисел: медиана 3008 нс (мин 2608, MAD 194), 0.301 нс на число
sum_squares, 10 000 чисел: медиана 5076 нс (мин 4804, MAD 116), 0.508 нс на число
sum_squares, 10 000 чисел: медиана 2858 нс (мин 1694, MAD 338), 0.286 нс на число
```

Базовая сборка — 0,38–0,62 нс на число и шумит, с AVX-512 — 0,29–0,30.
Выигрыш — от 1,3 до 2 раз, в зависимости от того, на какую полосу
шума попал базовый прогон; три прогона подряд, а не один, здесь и
показывают, сколько стоит одиночный замер.

<div class="box trap">

**Ловушка: `native` — это эта машина.** Программа с
`-C target-cpu=native` на процессоре без AVX-512 упадёт с SIGILL на
первой же такой инструкции — и под valgrind тоже (раздел
[30.4](04-cachegrind.md)). Для раздачи собирают под уровень
(`-C target-cpu=x86-64-v3` — AVX2, FMA, BMI2: Intel с Haswell 2013
года, AMD с Excavator 2015-го, но не дешёвые Pentium, Celeron и Atom
тех лет) или
выбирают реализацию при выполнении по `is_x86_feature_detected!`
(раздел [19.4](../ch19/04-dispatch.md)).

</div>

<div class="box c">

**Как в C.** Всё это есть и в GCC и clang, иногда под теми же именами:
`static inline` в заголовке — способ дать тело функции каждому файлу,
как `#[inline]` даёт MIR каждому крейту; `-flto`;
`__attribute__((cold))` и `__builtin_expect(x, 0)` (в C++20 ещё
`[[unlikely]]`; в C23 такого атрибута нет); `-fprofile-generate` и
`-fprofile-use`;
`-march=native`. Разница в умолчаниях: cargo всегда знает весь граф
крейтов, поэтому LTO включается одной строкой профиля, а не правкой
каждого Makefile, и `#[inline]` на функции библиотеки не требует
переносить её в заголовок.

</div>

<div class="box check">

**Проверь себя.** 1. Почему `call_sum` — `jmp`, а не `call`, и что
изменилось бы, если бы после вызова `scaled_sum` стояло ещё сложение?
2. С профилем `report` слилась с `report_cold`. Почему без профиля они
не слились, хотя тела у них одинаковые в исходнике?

</div>
