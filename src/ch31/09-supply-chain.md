# 31.9. Цепочка поставок: cargo-deny, cargo-audit, cargo-vet

В `Cargo.lock` учебника 253 крейта: 114 — главы, задачи и проект, 139
пришли с crates.io. Тесты, фаззер и Kani этой главы проверяют наш код;
про чужой нужно знать другое. Можно ли его распространять вместе с
нашим — какие у него лицензии? Нет ли в нём известных уязвимостей и не
заброшен ли он? Откуда он пришёл? Кто его читал? И что из него выполняется
ещё до того, как мы запустим свою программу? Всё это вместе — *цепочка
поставок* (supply chain): путь чужого кода в нашу сборку.

<div class="box ref">

**Справка: что приходит с зависимостью.** Крейт с crates.io — архив
исходников. `Cargo.lock` закрепляет точную версию каждого крейта и
контрольную сумму архива; cargo сверяет её при каждой загрузке, так что
подменить опубликованную версию нельзя — можно только опубликовать
новую. Версию можно *отозвать* (yank): новые `Cargo.lock` её не выберут,
старые продолжат собираться. Об уязвимостях и заброшенных крейтах
сообщает база RustSec — репозиторий на GitHub, по записи на случай, с
идентификатором `RUSTSEC-год-номер` и диапазоном затронутых версий. Её
читают два инструмента: *cargo-audit* — только она и `Cargo.lock`;
*cargo-deny* — она плюс политика лицензий, источников и запретов. Третий,
*cargo-vet*, базы не читает: он ведёт учёт того, кто какую версию какого
крейта прочитал.

</div>

## Политика: `deny.toml`

cargo-deny проверяет граф зависимостей против файла политики. У
учебника он один — `deny.toml` в корне — на пять рабочих пространств:
корень, прошивки и три набора целей фаззинга главы 31.

```toml
{{#include ../../deny.toml:graph}}
```

Граф строится для тех целей, под которые учебник собирается:
зависимость только для Windows в проверку не попадёт, зато попадёт
всё, что нужно прошивкам под Cortex-M.

```toml
{{#include ../../deny.toml:advisories}}
```

Каждое исключение — с причиной. `bare-metal` заброшен, но приходит
через `cortex-m` 0.7 (главы 27 и прошивки), и замены нет: исключение
записано на одну запись RustSec, а не на весь крейт и не на все
заброшенные сразу.

```toml
{{#include ../../deny.toml:licenses}}
```

Список разрешённых лицензий — короткий, и исключение на NCSA сделано для
одного крейта: libFuzzer попадает только в цели фаззинга, которые мы не
распространяем. `confidence-threshold` — насколько текст файла лицензии
должен совпасть с эталоном, если в `Cargo.toml` лицензия не указана
выражением SPDX.

```toml
{{#include ../../deny.toml:sources}}
```

С политикой:

```console
$ cargo deny check
...
warning[unmatched-bypass]: crate build bypass was not encountered
    ┌─ deny.toml:130:10
    │
130 │ crate = "ch31"
    │          ━━━━ unmatched bypass
advisories ok, bans ok, licenses ok, sources ok
```

Предупреждения «не встретилось» — цена одной политики на пять
пространств: обход для `ch31` нужен в целях фаззинга, где `ch31` —
зависимость, а в корне он член рабочего пространства, и cargo-deny его
не проверяет. Без политики — с пустым файлом настроек — те же
зависимости не проходят:

```console
$ cargo deny --config empty.toml check
error[rejected]: failed to satisfy license requirements
   ┌─ ~/.cargo/registry/src/index.crates.io-…/aho-corasick-1.1.5/Cargo.toml:40:12
   │
40 │ license = "Unlicense OR MIT"
   │            ━━━━━━━━━────━━━
   │            │            │
   │            │            rejected: license is not explicitly allowed
   │            rejected: license is not explicitly allowed
...
error[unmaintained]: bare-metal is deprecated
  ┌─ Cargo.lock:9:1
  │
9 │ bare-metal 0.2.5 registry+https://github.com/rust-lang/crates.io-index
  │ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ unmaintained advisory detected
  │
  ├ ID: RUSTSEC-2026-0110
  ├ Advisory: https://rustsec.org/advisories/RUSTSEC-2026-0110
  ├ The `bare-metal` crate has been deprecated and archived.
  ...
  ├ Solution: No safe upgrade is available!
  ├ bare-metal v0.2.5
    └── cortex-m v0.7.10
        ├── ch27-hal v0.1.0
...
advisories FAILED, bans ok, licenses FAILED, sources ok
```

196 отказов по лицензиям: по умолчанию не разрешено ничего, даже MIT.
Код выхода — 5, битовая маска проваленных проверок: advisories — 1,
bans — 2, licenses — 4, sources — 8. Под отчётом о `bare-metal` —
дерево: через кого он пришёл. Это и есть ответ на вопрос «можно ли от
него избавиться».

## Код, который выполняется при сборке

Зависимость выполняет свой код задолго до того, как вы вызовете её
функцию. Сценарий сборки `build.rs` — программа, которую cargo собирает
и запускает перед сборкой крейта. Процедурный макрос — библиотека,
которую компилятор загружает в себя и вызывает при разборе вашего кода.
Опыт — два безобидных крейта:

```rust,ignore
{{#include ../../code/ch31/supply/build/dep/build.rs:build}}
```

```rust,ignore
{{#include ../../code/ch31/supply/build/mac/src/lib.rs:mac}}
```

```rust,ignore
{{#include ../../code/ch31/supply/build/app/src/main.rs:app}}
```

`cargo metadata` — то, на чём строят граф cargo-deny и редакторы, —
читает только `Cargo.toml` и `Cargo.lock`, и следов не остаётся. Хватает
`cargo check` — проверки типов без сборки исполняемого файла:

```console
$ cargo check
   Compiling ch31-supply-dep v0.1.0 (…/build/dep)
   Compiling ch31-supply-mac v0.1.0 (…/build/mac)
    Checking ch31-supply-app v0.1.0 (…/build/app)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.25s
$ cat build-rs-ran proc-macro-ran
сценарий сборки, uid 0процедурный макрос
```

uid 0 — потому что скрипт главы гоняли в контейнере от root; у вас там
будет ваш uid. Оба крейта выполнили свой код с правами того, кто
набрал команду, и записали файлы туда, куда им вздумалось. То же
происходит, когда редактор с rust-analyzer открывает проект: он
запускает сценарии сборки и процедурные макросы, чтобы знать, что они
порождают. Открыть чужой репозиторий в редакторе — уже выполнить его
код.

Правило `[bans.build]` политики — список крейтов, которым сценарий
сборки разрешён:

```toml
{{#include ../../deny.toml:bans}}
```

Каждая запись — чтение `build.rs`: большинство спрашивают у rustc версию
и включают `cfg`, `cortex-m-rt` кладёт сценарий компоновки, libfuzzer-sys
собирает C++. Новый крейт со сценарием сборки останавливает проверку:

```console
$ cargo deny --manifest-path build/app/Cargo.toml --config deny.toml check bans
error[build-script-not-allowed]: crate 'ch31-supply-dep = 0.1.0' has a build script but is not allowed to have one
 ├ path = '$crate/build.rs'
 ├ checksum = '68090367dd0a6d0d92ab4500a421bc08d3473f369b2f204de98c4aebf5de813f'
 ├ ch31-supply-dep v0.1.0
   └── ch31-supply-app v0.1.0
...
bans FAILED
```

Процедурный макрос `ch31-supply-mac` этот список не останавливает: он
— о сценариях сборки. К процедурным макросам относятся только правила
`executables` и `interpreted` — никаких готовых исполняемых файлов и
скриптов в исходниках крейтов, чей код выполняется при сборке. Скрипты
CI самого `zerocopy` в его архиве лежат, но сборка их не запускает: для
них — обходы с перечислением путей. Контрольная сумма в отчёте — та,
которую можно записать в обход (`build-script = "…"`): пока сценарий не
изменился, cargo-deny считает его прочитанным и не ищет в крейте
исполняемых файлов.

## cargo audit

```console
$ cargo audit
    Fetching advisory database from `https://github.com/RustSec/advisory-db.git`
      Loaded 1352 security advisories (from ~/.cargo/advisory-db)
    Updating crates.io index
    Scanning Cargo.lock for vulnerabilities (253 crate dependencies)
Crate:     bare-metal
Version:   0.2.5
Warning:   unmaintained
Title:     bare-metal is deprecated
Date:      2026-04-23
ID:        RUSTSEC-2026-0110
URL:       https://rustsec.org/advisories/RUSTSEC-2026-0110
warning: 1 allowed warning found
```

Уязвимостей нет, заброшенный крейт — предупреждение, код выхода 0.
Чтобы увидеть уязвимость, достаточно `Cargo.lock` со старой версией —
такой, какой cargo записал бы для `smallvec = "=1.6.0"`:

```bash
{{#include ../../code/ch31/supply/run.sh:old_lock}}
```

```console
$ cargo audit -f old/Cargo.lock
...
    Scanning old/Cargo.lock for vulnerabilities (2 crate dependencies)
Crate:     smallvec
Version:   1.6.0
Title:     Buffer overflow in SmallVec::insert_many
Date:      2021-01-08
ID:        RUSTSEC-2021-0003
URL:       https://rustsec.org/advisories/RUSTSEC-2021-0003
Severity:  9.8 (critical)
Solution:  Upgrade to >=0.6.14, <1.0.0 OR >=1.6.1

error: 1 vulnerability found!
```

Уязвимый крейт при этом не скачан и не собран: cargo audit читает
только `Cargo.lock` и сравнивает версии с диапазонами из базы. Поэтому
он быстр и годится для проверки по расписанию: база обновляется
каждый день, и вчера чистый `Cargo.lock` сегодня может оказаться
уязвимым без единого изменения в репозитории. По той же причине его
можно навести на `Cargo.lock` чужого проекта, ничего не запуская.
Исполняемый файл, собранный `cargo auditable build`, несёт список своих
зависимостей в отдельной секции, и `cargo audit bin` проверяет уже его.

Чем cargo audit отличается от раздела `advisories` cargo-deny? База
одна и та же. cargo-deny добавляет политику — исключения с причинами,
уровень для заброшенных крейтов, отозванные версии — и работает в одном
прогоне с лицензиями и запретами. cargo audit проще: исключения — флагом
`--ignore` или в `.cargo/audit.toml`, а `--deny warnings` делает ошибкой
и заброшенные крейты. В CI учебника работают оба: cargo-deny — с
политикой, cargo audit — как отдельная проверка того же `Cargo.lock`
без неё.

## cargo vet: кто это читал

База RustSec знает только о найденном. Вопрос «читал ли кто-нибудь этот
код» она не задаёт. cargo-vet ведёт учёт: в каталоге `supply-chain`
репозитория записано, какие версии каких крейтов кто проверил и по
какому критерию. Встроенных критериев два: `safe-to-run` — код можно
запускать на машине разработчика (сборка и тесты), и `safe-to-deploy` —
можно ставить к пользователям, строже. Опыт — крейт с одной
зависимостью:

```console
$ cargo vet init
$ cargo vet
Vetting Succeeded (1 exempted)
```

`init` вписывает все текущие зависимости в исключения
(`exemptions`) — с чего-то надо начать, — и проверка проходит. Новая
зависимость в исключения не попадает:

```console
$ cargo add memchr@=2.8.3 --no-default-features
$ cargo vet
Vetting Failed!

1 unvetted dependencies:
  memchr:2.8.3 missing ["safe-to-deploy"]

recommended audits for safe-to-deploy:
    Command                         Publisher   Used By   Audit Size
    cargo vet inspect memchr 2.8.3  BurntSushi  ch31-vet  16310 lines

estimated audit backlog: 16310 lines

Use |cargo vet certify| to record the audits.
```

16 310 строк — столько придётся прочитать, чтобы честно поставить
подпись под одной небольшой зависимостью. `cargo vet inspect` открывает
исходники именно той версии, что в `Cargo.lock`, а `cargo vet certify`
записывает результат:

```console
$ cargo vet certify memchr 2.8.3 --criteria safe-to-deploy \
      --who "Читатель главы 31" --notes "пример записи: что прочитано и что проверено"
$ cargo vet
Vetting Succeeded (1 fully audited, 1 exempted)
$ cat supply-chain/audits.toml

# cargo-vet audits file

[[audits.memchr]]
who = "Читатель главы 31"
criteria = "safe-to-deploy"
version = "2.8.3"
notes = "пример записи: что прочитано и что проверено"
```

Читать всё самому никто не обязан. Аудиты можно импортировать —
`[imports.<имя>]` с адресом чужого `audits.toml` в
`supply-chain/config.toml`: свои записи публикуют Mozilla, Google,
Bytecode Alliance и другие. А при обновлении проверяют не всю новую
версию, а разницу: `cargo vet diff memchr 2.8.3 2.8.4` показывает
только изменённое, и `certify` с двумя версиями записывает аудит
дельты. Опыт целиком — в `code/ch31/supply/run.sh`; самому учебнику
каталог `supply-chain` не заведён: исключений в нём было бы 139, и
учёт начинался бы с нуля.

<div class="box hood">

**Под капотом: что читает cargo-deny.** Граф — из `cargo metadata`,
который не выполняет ни сценариев сборки, ни макросов, с разрешением
признаков (features) для перечисленных целей. Лицензии — из поля
`license` в `Cargo.toml` каждого крейта, выражением SPDX (`MIT OR
Apache-2.0`), а если поля нет — из файлов лицензий в архиве, которые
сравниваются с эталонными текстами. Архивы к этому времени уже скачаны:
`cargo metadata` скачивает их, чтобы прочитать `Cargo.toml`. Для
`[bans.build]` cargo-deny смотрит в цели крейта: цель вида
`custom-build` — сценарий сборки, `proc-macro` — макрос, — и обходит
файлы архива в поисках исполняемых: ELF, Mach-O, PE — по первым 16
байтам, скрипты — по строке `#!` в начале. База RustSec — клон
репозитория в `~/.cargo/advisory-db`; каждый запуск начинается с
`git fetch`, а `cargo audit -n` и `cargo deny --offline` работают по уже
скачанной копии.

</div>

<div class="box trap">

**Ловушка: проверили один `Cargo.lock`, а собрали другой.** Проверки
главы смотрят на `Cargo.lock` репозитория — и только на него.
`cargo install` без `--locked` его не читает и берёт свежие совместимые
версии, сборка в CI без `--locked` может молча обновить файл. У
библиотеки `Cargo.lock` вообще не доходит до пользователей: каждый, кто
от неё зависит, разрешает версии в своём `Cargo.lock`, и ваш чистый
отчёт о нём ничего не говорит. И RustSec знает только о том, о чём
сообщили: уязвимость в копии библиотеки C, которую крейт собирает из
своих исходников (как [`libz-sys`][g-sys-crate] с зашитым zlib), попадёт в базу,
только если кто-то запишет её на сам крейт.

</div>

<div class="box c">

**Как в C.** Ни `Cargo.lock`, ни общего реестра: зависимости — пакеты
дистрибутива (их уязвимости отслеживает сам дистрибутив), копии
исходников в репозитории, подмодули git, `FetchContent` в CMake. Какая
версия zlib на самом деле вошла в сборку, часто не скажет никто, кроме
сборочного журнала. Лицензии ищут сканерами по тексту (ScanCode, REUSE
с метками SPDX в файлах), уязвимости — по базе CVE и номеру версии,
если его удалось узнать; OSV-Scanner понимает и `Cargo.lock`, и
зависимости C из ряда сборочных систем. А код, выполняемый при сборке,
в C — это `./configure`, макросы m4 и `Makefile`: тысячи строк shell,
которые никто не читает. Именно там в 2024 году спрятали закладку в
xz-utils (CVE-2024-3094): в выпущенных архивах — не в git — макрос
`build-to-host.m4` при сборке доставал из «тестовых» файлов код, который
встраивался в liblzma и через неё — в sshd. Политики «кому разрешён
сценарий сборки» для `./configure` нет: сценарий сборки есть у всех.

</div>

<div class="box check">

**Проверь себя.** 1. Почему `cargo deny` без политики завершился с
кодом 5? Какой код был бы, если бы отказали только источники? 2. Почему
`cargo metadata` на чужом репозитории безопасен, а открыть тот же
репозиторий в редакторе с rust-analyzer — нет? 3. cargo audit на
`Cargo.lock` учебника вчера был чист, а сегодня нашёл уязвимость. Что
изменилось, если в репозиторий никто ничего не отправлял? 4. Новая
версия `memchr` — плюс сорок строк к прочитанной 2.8.3. Сколько строк
нужно прочитать, чтобы обновиться с подписью cargo-vet?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-sys-crate]: ../appendix-f.md#sys-crate "-sys-крейт (sys crate): Крейт с сырыми объявлениями C-библиотеки: extern-блоки один к одному с заголовком, часто от bindgen, и build.rs, который находит или собирает библиотеку и линкует её."
