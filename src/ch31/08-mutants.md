# 31.8. Кто проверит тесты: cargo-mutants

Все инструменты главы до сих пор проверяли код. Но и тесты — код, и
ошибаются они тихо: тест, который ничего не утверждает, проходит всегда.
Покрытие строк тут не помогает — строка, которую тест выполнил, но
результат которой не проверил, считается покрытой. Нужен другой вопрос:
*если код испортить, заметят ли это тесты?*

<div class="box ref">

**Справка: мутационное тестирование.** Инструмент вносит в код одну
маленькую правдоподобную ошибку — *мутанта* — и прогоняет тесты. Упали —
мутант «пойман» (caught): такую ошибку тесты заметили бы. Прошли —
мутант «выжил» (missed): в этом месте код можно сломать, и никто не
узнает. cargo-mutants разбирает исходники крейта и порождает мутантов
двух главных видов. Первый — тело функции целиком заменяется значением
её типа: `()`, `0` и `1`, `true` и `false`, `Default::default()`,
`Ok(…)` с ним же, пустой срез. Второй — оператор меняется на соседний:
`<` на `<=`, `==` и `>`, `+` на `-` и `*`, `^` на `|` и `&`, `<<` на
`>>`, `+=` на `-=` и `*=`, условие ветви `match` — на `true` и `false`.
Исходов четыре: caught, missed, timeout — тесты не кончились за отведённое
время, unviable — мутант не собрался, и тесты до него не дошли.

</div>

## Первый прогон

Разбор кадров проекта (`project/telemetry-frame`) к этой главе оброс
тестами: векторы главы 6, ошибки главы 7, поток главы 13, таблица CRC
главы 25. Сколько из них что-то проверяет, покажет cargo-mutants:

```console
$ cargo mutants -p telemetry-frame --exclude project/telemetry-frame/src/proofs.rs \
      -- --lib --test frame --test crc_table
Found 139 mutants to test
ok       Unmutated baseline in 1s build + 1s test
 INFO Auto-set test timeout to 20s
TIMEOUT  project/telemetry-frame/src/lib.rs:275:9: replace Frame<'_>::wire_len -> usize with 0 in 1s build + 20s test
TIMEOUT  project/telemetry-frame/src/lib.rs:275:20: replace + with * in Frame<'_>::wire_len in 0s build + 20s test
MISSED   project/telemetry-frame/src/lib.rs:291:19: replace < with <= in encode in 0s build + 1s test
...
MISSED   project/telemetry-frame/src/stream.rs:86:33: replace - with + in <impl Iterator for Frames<'a>>::size_hint in 0s build + 1s test
139 mutants tested in 5m: 2 missed, 60 caught, 69 unviable, 8 timeouts
```

Всё после `--` уходит `cargo test`: здесь — прежние тесты проекта, без
файла `tests/mutants.rs`, который появится ниже. Сначала cargo-mutants
прогоняет тесты на неиспорченном коде (baseline): если они падают уже
там, мутанты ничего не скажут. Время этого прогона задаёт тайм-аут для
мутантов — впятеро больше, но не меньше 20 секунд. Строки печатаются
только для выживших и зависших; полные списки — в каталоге
`mutants.out`: `caught.txt`, `missed.txt`, `timeout.txt`,
`unviable.txt`, а для каждого мутанта — его diff и журнал сборки и
тестов.

Из 139 мутантов 60 тесты поймали, 2 пропустили. Остальные 77 требуют
разбора — почти половина мутантов до тестов не дошла.

## Двое выживших

**Граница в `encode`.**

```rust,ignore
{{#include ../../project/telemetry-frame/src/lib.rs:encode}}
```

`body.len() < payload.len()` → `body.len() <= payload.len()`: мутант
отказывается кодировать в буфер ровно под кадр. Тесты проекта кодировали
в буферы с запасом — от одного байта (`HEADER_LEN + MAX_PAYLOAD + 1`) до
десятков, — а слишком маленький пробовали только на шесть байт меньше
нужного: 12 вместо 18. Ровно на границе не проверял никто. Это та самая
ошибка на единицу, ради которой в разделе [31.3](03-proptest.md)
стратегии целились в границы, и proptest её здесь нашёл бы — но свойства
`encode` с размером буфера никто не писал. Тест против мутанта
проверяет границу с обеих сторон:

```rust,ignore
{{#include ../../project/telemetry-frame/tests/mutants.rs:exact_buffer}}
```

**Формула в `size_hint`.**

```rust,ignore
// project/telemetry-frame/src/stream.rs
fn size_hint(&self) -> (usize, Option<usize>) {
    (0, Some(self.buf.len() - self.pos))
}
```

Мутант `len - pos` → `len + pos` даёт неверное, но *допустимое* число:
контракт `Iterator::size_hint` требует только, чтобы элементов было не
больше верхней границы, а `len + pos` не меньше `len - pos`. Прежний
тест (`p13_fused_and_size_hint` в `tests-shared/frame.rs`) сверял
границу точно только в начале, где `pos = 0` и обе формулы равны, а в
конце — лишь `hi >= count`. Восемь мутантов `size_hint` — замены тела на
`(0, None)`, `(1, Some(0))` и так далее и два оператора — этот тест
поймал семь.

Выживший мутант — не всегда пробел в тестах. Бывает, что изменение не
нарушает ничего, что обещано, и тогда убивать его нечем и незачем. Здесь
решает документация: итератор обещает, что каждый элемент забирает хотя
бы байт, значит, граница — ровно непрочитанный хвост, и на неё вправе
полагаться тот, кто резервирует место по верхней границе. Тест закрепляет
обещание на каждом шаге:

```rust,ignore
{{#include ../../project/telemetry-frame/tests/mutants.rs:size_hint}}
```

## Мутанты, которые не собрались

69 нежизнеспособных — больше, чем пойманных, и 55 из них — в CRC. Любой
мутант `crc16_update`, `crc16`, таблицы `CRC16_TABLE` или
`crc16_update_table` останавливает компилятор:

```text
error[E0080]: evaluation panicked: assertion failed: crc16(b"123456789") == 0x29B1
   --> project/telemetry-frame/src/lib.rs:220:15
    |
220 | const _: () = assert!(crc16(b"123456789") == 0x29B1);
    |               ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ evaluation of `_` failed here
```

Функции CRC — `const fn`, и контрольное значение из описания алгоритма
проверяется при компиляции (главы 6 и 25). Тест переехал в компилятор:
испорченный CRC не собирается, до `cargo test` дело не доходит. Мутант
таблицы ловит проверка `crc16_update_table` — таблицу считает
`const`-выражение, и её порча меняет контрольное значение.

Остальные 14 не собрались по причинам типов. `Kind`, `Frame`,
`Measurement` не реализуют `Default`, и замена тела на
`Ok(Default::default())` — ошибка E0277. Крейт `no_std`, и мутанты
`rest` с `Vec::leak(…)` не находят `Vec`. Один мутант не разобрался
синтаксически: `len as usize > MAX_PAYLOAD` → `len as usize <
MAX_PAYLOAD` компилятор читает как начало обобщённых аргументов:
«`<` is interpreted as a start of generic arguments for `usize`».

Нежизнеспособный мутант о тестах не говорит ничего. Здесь это не беда:
CRC проверен строже, чем тестом, — каждой сборкой. Но если бы проверку
`const _` однажды удалили, 55 мутантов стали бы жизнеспособными, и
ловить их пришлось бы тестам; прогон cargo-mutants после такого
изменения это и покажет.

## Тайм-ауты

Восемь мутантов зависли:

```text
project/telemetry-frame/src/lib.rs:275:9: replace Frame<'_>::wire_len -> usize with 0
project/telemetry-frame/src/lib.rs:275:20: replace + with * in Frame<'_>::wire_len
project/telemetry-frame/src/stream.rs:52:9: replace Frames<'a>::resync with ()
project/telemetry-frame/src/stream.rs:53:90: replace + with * in Frames<'a>::resync
project/telemetry-frame/src/stream.rs:67:31: replace + with - in <impl Iterator for Frames<'a>>::next
project/telemetry-frame/src/stream.rs:67:31: replace + with * in <impl Iterator for Frames<'a>>::next
project/telemetry-frame/src/stream.rs:76:36: replace + with - in <impl Iterator for Frames<'a>>::next
project/telemetry-frame/src/stream.rs:76:36: replace + with * in <impl Iterator for Frames<'a>>::next
```

Все они ломают одно: `Frames` перестаёт двигаться вперёд. `wire_len`,
равный нулю, оставляет позицию на месте; `HEADER_LEN * 0` для кадра без
нагрузки — тоже ноль; `resync`, который ничего не делает, после
испорченного заголовка снова и снова отдаёт ту же ошибку. Тест,
который проходит поток до конца, крутится бесконечно, и cargo-mutants
снимает его по тайм-ауту. Тайм-аут — тоже «тесты заметили», но дорогой:
восемь мутантов по 20 секунд — больше половины пяти минут прогона. И
повод для теста получше: тест, который сам ограничивает число шагов
(`.take(n)`) и проверяет, что их хватило, превратил бы зависание в
падение за миллисекунды. Именно это свойство — шаг продвигается хотя бы
на байт — доказывает Kani в разделе [31.10](10-project.md).

## Второй прогон

С `tests/mutants.rs`:

```console
$ cargo mutants -p telemetry-frame --exclude project/telemetry-frame/src/proofs.rs
...
139 mutants tested in 5m: 62 caught, 69 unviable, 8 timeouts
```

Код проекта не менялся — добавились только два теста. Скрипт
`code/ch31/mutants/run.sh` делает оба прогона и сверяет исходы с текстом.

## Что исключать

Доказательства Kani исключены не случайно. Без `--exclude` мутантов 143:

```text
project/telemetry-frame/src/proofs.rs:15:5: replace decode_any_buffer_up_to_24 with ()
project/telemetry-frame/src/proofs.rs:16:46: replace <= with > in decode_any_buffer_up_to_24
project/telemetry-frame/src/proofs.rs:36:5: replace frames_step_makes_progress with ()
project/telemetry-frame/src/proofs.rs:37:46: replace <= with > in frames_step_makes_progress
```

cargo-mutants идёт по объявлениям `mod` и условий `cfg` не вычисляет:
модуль `proofs` под `#[cfg(kani)]` для него такой же код, как любой
другой. Но `cargo test` этот модуль не компилирует, и все четыре
мутанта «выжили» бы — ложная тревога. То же с любым кодом, который в
тестовую сборку хоста не попадает: ветки `#[cfg(target_os = "none")]`
прошивки, функции за выключенным признаком (feature). Их исключают —
флагом `--exclude` или в `.cargo/mutants.toml`:

```toml
exclude_globs = ["project/telemetry-frame/src/proofs.rs"]
# Регулярные выражения по имени мутанта, как его печатает --list.
exclude_re = ["replace \\| with \\^ in join_bytes"]   # эквивалентный, см. ловушку
```

Модули `#[cfg(test)]` cargo-mutants пропускает сам.

## Только изменённое: `--in-diff`

Полный прогон — минуты на крейт, и для каждого запроса на слияние это
дорого. `--in-diff` берёт diff и мутирует только затронутые им строки:

```console
$ git diff main > change.diff
$ cargo mutants -p telemetry-frame --in-diff change.diff --list
project/telemetry-frame/src/stream.rs:86:9: replace <impl Iterator for Frames<'a>>::size_hint -> (usize, Option<usize>) with (0, None)
project/telemetry-frame/src/stream.rs:86:9: replace <impl Iterator for Frames<'a>>::size_hint -> (usize, Option<usize>) with (0, Some(0))
...
project/telemetry-frame/src/stream.rs:86:33: replace - with + in <impl Iterator for Frames<'a>>::size_hint
project/telemetry-frame/src/stream.rs:86:33: replace - with / in <impl Iterator for Frames<'a>>::size_hint
```

Здесь diff менял одну строку `size_hint` — и восемь мутантов вместо
139. `--list` только перечисляет, ничего не собирая; `-j 4` гоняет
четыре мутанта параллельно, каждый в своей копии дерева;
`--shard 1/4` делит мутантов между машинами CI.

<div class="box hood">

**Под капотом: копия дерева.** cargo-mutants не трогает ваши исходники.
Он копирует дерево во временный каталог — с `--gitignore true` без
того, что игнорирует git: каталоги `target` целей фаззинга весят
гигабайты, — и там для каждого мутанта переписывает один файл, собирает
тесты и запускает их, а потом возвращает файл. Каталог `target` копии
живёт весь прогон, поэтому после первой сборки каждый мутант
пересобирает только свой крейт и тех, кто от него зависит: секунда на
мутанта вместо полной сборки. Мутанты ищутся по синтаксическому дереву
(`syn`), без проверки типов, — отсюда и нежизнеспособные: что
`Default` для `Kind` нет, выясняет только компилятор. Код выхода — 0,
если все пойманы или не собрались; 2 — есть выжившие; 3 — были
тайм-ауты (он побеждает 2: в первом прогоне выжившие были, а код — 3);
4 — тесты падают уже без мутаций. Поэтому скрипт главы считает
выживших по `missed.txt`, а не по коду выхода.

</div>

<div class="box trap">

**Ловушка: ноль выживших — не ноль ошибок.** Мутанты сравнивают тесты с
кодом, каким он написан. Если код и тесты ошибаются одинаково — тест
закрепил неверное поведение, потому что его писали по коду, — любой
мутант этого кода тест уронит, и cargo-mutants будет доволен. Задача D
этой главы — ровно такой случай. Обратная сторона — *эквивалентные*
мутанты: `(hi << 8) | lo` → `(hi << 8) ^ lo` при непересекающихся битах
даёт тот же результат на любых входах, и убить такого мутанта нельзя.
Его исключают по имени (`exclude_re`) с комментарием, почему, — а не
дописывают тест, который проверяет внутренности функции вместо её
поведения.

</div>

<div class="box c">

**Как в C.** Для C и C++ есть Mull — дополнение к clang: мутации
вносятся в LLVM IR при компиляции, все мутанты попадают в один
исполняемый файл тестов, а `mull-runner` включает их по одному при
запуске. Без пересборки на каждого мутанта это быстро, но два отличия
от Rust остаются. Мутант, который вносит неопределённое поведение —
`<` на `<=` в цикле по массиву, — может пройти тесты молча, а может
упасть через раз. И проверки при компиляции почти нечем заменить:
`_Static_assert` в C принимает только константное выражение, функцию
CRC в нём не вызвать, так что все 55 мутантов CRC в C дошли бы до
тестов. Чаще же мутационного тестирования в проектах на C нет вовсе, и
мерой тестов служит покрытие `gcov` — то самое, которое не отличает
выполненную строку от проверенной.

</div>

<div class="box check">

**Проверь себя.** 1. Мутант `replace + with * in Frame<'_>::wire_len`
завис, а не упал. На каком кадре `HEADER_LEN * payload.len()` даёт
ноль и почему не падает ни одна проверка? 2. Мутант `size_hint` с
`(0, None)` пойман, а с `len + pos` — нет, хотя оба допустимы по
контракту `Iterator`. Какая строка прежнего теста отличает первый и
почему не отличает второй? 3. Почему 55 нежизнеспособных мутантов CRC
не повод для беспокойства, а 4 «выживших» в `proofs.rs` — не повод
дописывать тесты?

</div>
