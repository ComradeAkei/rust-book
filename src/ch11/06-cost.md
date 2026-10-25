# 11.6. Цена косвенного вызова

## Цикл опроса

Станция опрашивает датчики, которые пришли снаружи, и типы их
неизвестны:

```rust
{{#include ../../code/ch11/src/cost.rs:poll}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_poll_dyn
ch11_poll_dyn:
	…
.LBB32_3:
	mov rdi, qword ptr [r14]
	mov rax, qword ptr [r14 + 8]
	call qword ptr [rax + 24]
	add ebp, eax
	add r14, 16
	cmp r14, rbx
	jne .LBB32_3
	…
$ cargo asm -p ch11 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch11_poll_dyn
ch11_poll_dyn:
	…
.LBB25_2:
	ldr r0, [r6, #8]!
	ldr r1, [r6, #4]
	ldr r1, [r1, #12]
	blx r1
	add r4, r0
	add.w r0, r6, #8
	cmp r0, r5
	bne .LBB25_2
	…
```

На каждый датчик — две загрузки из среза (адрес данных и таблица лежат
рядом, в одной строке кэша), загрузка адреса `read` из таблицы и вызов.
Вынести адрес из цикла, как в `scan` из раздела
[11.4](04-objects.md), здесь нельзя: у каждого элемента своя таблица.
Встроить `read` тоже нельзя, и вместе с ним теряются все оптимизации
поверх него: [векторизация][g-vectorization] суммы, вынос общих вычислений.

В `fops.c` из введения к главе на один вызов приходится три загрузки:
указатель на объект из массива, `d->ops` из самого объекта и `read` из
таблицы. Средняя загрузка трогает память объекта ещё до вызова, и если
объекты разбросаны по куче, промах кэша случается раньше, чем начинает
работать метод. В Rust таблица едет в указателе, и до вызова память
объекта не нужна.

## Предсказатель переходов

На x86-64 главная цена — не загрузки, а `call` по регистру. Процессор
выбирает инструкции на десятки тактов вперёд и не может ждать, пока
загрузка из таблицы вернёт адрес. Блок предсказания косвенных переходов
угадывает адрес по адресу самой инструкции и истории последних
переходов, конвейер выполняет предсказанный метод, а при ошибке
сбрасывает работу и начинает заново. Это десятки тактов: по замерам
задачи C на машине, где сняты числа главы, один промах обходится
примерно в 9 нс.

Замеры из введения показывают это на C: 1.45 нс на вызов, когда объекты
лежат по типам, и 6.33 нс вперемешку. Модель предсказателя `valgrind`
насчитала 0.2 % промахов в первом случае — две смены типа за проход по
тысяче объектов — и 51.3 % во втором: при двух равновероятных типах
угадать следующий без закономерности можно в половине случаев. Rust
здесь ничем не отличается, `call qword ptr [rax + 24]` предсказывается
так же. Задача C из раздела [11.8](08-tasks.md) меряет `dyn`,
перечисление и обобщения на 2, 8 и 64 типах по порядку и вперемешку.

Отсюда практическое правило: если объектов много, а типов мало, держать
объекты одного типа рядом. Это может быть сортировка по типу при
построении списка или отдельный вектор на каждый тип — тогда `dyn` не
нужен вовсе.

На Cortex-M0, M3 и M4 предсказателя косвенных переходов нет, и цена не
зависит от порядка. По технической документации Cortex-M3 `BLX` по
регистру занимает 1 + P тактов, где P — перезаполнение конвейера, от
одного до трёх, а загрузка адреса из таблицы во flash добавляет [такты
ожидания][g-wait-states] памяти. Зато невозможность встроить `read` стоит там столько
же, сколько везде.

## Девиртуализация

Косвенный вызов остаётся косвенным, только если LLVM не знает, какая
таблица в указателе. Когда знает — вызов становится прямым и
встраивается:

```rust
{{#include ../../code/ch11/src/cost.rs:devirt}}
```

```console
$ cargo asm -p ch11 --lib --simplify ch11_devirt_local
ch11_devirt_local:
	mov eax, 2150
	ret
$ cargo asm -p ch11 --lib --simplify ch11_devirt_static
ch11_devirt_static:
	mov eax, 2162
	ret
$ cargo asm -p ch11 --lib --simplify --no-default-features --target thumbv7m-none-eabi ch11_devirt_static
ch11_devirt_static:
	push {r7, lr}
	mov r7, sp
	movw r0, #2162
	pop {r7, pc}
$ cargo asm -p ch11 --lib --simplify ch11_devirt_hidden
ch11_devirt_hidden:
	sub rsp, 24
	mov word ptr [rsp + 6], 344
	lea rax, [rsp + 6]
	mov qword ptr [rsp + 8], rax
	lea rax, [rip + .Lanon.da1beaeebe5f420a6faa69c57adbc3f3.5]
	mov qword ptr [rsp + 16], rax
	lea rax, [rsp + 8]
	#APP
	#NO_APP
	mov rdi, qword ptr [rsp + 8]
	mov rax, qword ptr [rsp + 16]
	call qword ptr [rax + 24]
	add rsp, 24
	ret
```

В `ch11_devirt_local` объект и приведение в одной функции: таблица —
константа, слово 3 в ней — `ch11_read_thermo`, вызов встроен, и
`344 * 100 / 16` посчитано при компиляции. В `ch11_devirt_static`
константы всё: массив `SENSORS`, объекты и таблицы, и сумма двух
датчиков, 2150 + 12, тоже сложилась при компиляции. [`black_box`][g-black-box] в
`ch11_devirt_hidden` делает вид, что читает и меняет толстый указатель
(`#APP` и `#NO_APP` — пустая ассемблерная вставка), и после него LLVM
уже не знает, что за таблица в `[rsp + 16]`.

`black_box` изображает то, что в настоящей программе происходит само.
Девиртуализации нет, если указатель пришёл из другого крейта без LTO,
из массива, заполненного во время работы, через `static mut` или по
[FFI][g-ffi]. Есть — если после встраивания LLVM видит, откуда взялась таблица:
`ch11_scan_max` из раздела [11.4](04-objects.md) встроила `scan` и
вызов замыкания. В прошивках с `lto = true` (раздел
[0.2](../ch00/02-cargo.md)) границы крейтов не мешают, и
девиртуализация случается чаще, чем можно ожидать по исходнику.

<div class="box trap">

**Ловушка.** Замер, который строит объекты рядом с циклом, меряет
девиртуализированный код: `dyn` в нём выглядит бесплатным, потому что
его там нет. Сравнивать `dyn` с обобщениями нужно на объектах, которые
оптимизатор не видит: собранных во время работы из входных данных или
пропущенных через `black_box`. Бенчмарк задачи C строит векторы
функцией `build_dyn` из перестановки, которую генерирует во время
работы, а тест `task_c_calls` проверяет ассемблер: в цикле `run` для
`Box<dyn Op>` остаётся вызов через таблицу.

</div>

## Retpoline

Предсказатель общий для всего, что исполняется на ядре процессора, и
это дыра. Spectre v2 (2018) — отравление предсказателя: атакующий
приучает его к тому, что косвенный переход по некоторому адресу ведёт к
выбранному им фрагменту кода, и жертва исполняет этот фрагмент
спекулятивно. Результат спекуляции отбрасывается, но успевает оставить
след в кэше, и по времени доступа к кэшу атакующий читает чужие данные.

*Retpoline* («return trampoline») заменяет каждый косвенный переход
переходником, где предсказатель косвенных переходов не участвует:

```console
$ gcc -O2 -fcf-protection=none -mindirect-branch=thunk -o fops-ret fops.c
$ objdump -d --no-show-raw-insn -M intel fops-ret
…
    15e0:	mov    rdi,QWORD PTR [rbx]
    15e3:	add    rbx,0x8
    15e7:	mov    rax,QWORD PTR [rdi]
    15ea:	mov    rax,QWORD PTR [rax]
    15ed:	call   1617 <__x86_indirect_thunk_rax>
…
0000000000001617 <__x86_indirect_thunk_rax>:
    1617:	call   1623 <__x86_indirect_thunk_rax+0xc>
    161c:	pause
    161e:	lfence
    1621:	jmp    161c <__x86_indirect_thunk_rax+0x5>
    1623:	mov    QWORD PTR [rsp],rax
    1627:	ret
```

Переходник вызывает свою же метку `1623`, и в стек ложится адрес
возврата `161c`. По метке `1623` адрес возврата затирается настоящей
целью из `rax`, и `ret` уходит туда. Предсказатель возвратов при этом
уверен, что `ret` вернётся в `161c`, и спекулятивное исполнение крутится
в безвредном цикле `pause; lfence`, пока процессор не узнает настоящий
адрес. Отравлять нечего, но и угадывать нечего: каждый косвенный вызов
становится гарантированным промахом. В `fops.c` с retpoline вызов стоит
14 нс в любом порядке — дороже худшего случая без защиты. Ubuntu
собирает GCC с `-fcf-protection` по умолчанию, и с ним
`-mindirect-branch` несовместим, отсюда `-fcf-protection=none`.

rustc умеет то же на nightly:

```console
$ rustc +nightly --edition 2024 --crate-type lib -O --emit asm -Zretpoline poll.rs
error: mixing `-Zretpoline` will cause an ABI mismatch in crate `poll`
  |
  = help: the `-Zretpoline` flag modifies the ABI so Rust crates compiled with different values of this flag cannot be used together safely
  = note: `-Zretpoline` being set in this crate is incompatible with `-Zretpoline` being unset in dependency `std`
  = help: unset `-Zretpoline` in this crate or set `-Zretpoline` in `std`
  = help: if you are sure this will not cause problems, you may use `-Cunsafe-allow-abi-mismatch=retpoline` to silence this error
$ rustc +nightly … -Zretpoline -Cunsafe-allow-abi-mismatch=retpoline poll.rs
poll:
	…
.LBB0_3:
	mov	rdi, qword ptr [r14 + r12]
	mov	rax, qword ptr [r14 + r12 + 8]
	mov	r11, qword ptr [rax + 24]
	call	__llvm_retpoline_r11
	…
__llvm_retpoline_r11:
	call	.Ltmp0
.LBB1_1:
	pause
	lfence
	jmp	.LBB1_1
.Ltmp0:
	mov	qword ptr [rsp], r11
	ret
```

Ошибка — не формальность. Готовая `std` собрана без retpoline, и её
косвенные вызовы — в `fmt`, в обработке паники — остались бы
незащищёнными, поэтому rustc требует собирать с флагом все крейты
программы. Ключ `-Cunsafe-allow-abi-mismatch` нужен здесь только
затем, чтобы посмотреть ассемблер. Тест `rustc_retpoline` в
`code/ch11/tests/nightly.rs` проверяет и ошибку, и переходник.

<div class="box hood">

**Под капотом.** Ядро Linux собирают с `-mindirect-branch=thunk-extern`:
переходники `__x86_indirect_thunk_*` определяет само ядро, а при
загрузке механизм `alternatives` заменяет их тем, что подходит
процессору. На процессорах с аппаратной защитой (eIBRS) переходник
становится обычным косвенным переходом, и ядро не платит за retpoline.
Виртуальная машина, на которой сняты числа этой главы, сообщает:

```console
$ cat /sys/devices/system/cpu/vulnerabilities/spectre_v2
Mitigation: Enhanced / Automatic IBRS; IBPB: conditional; PBRSB-eIBRS: SW sequence; BHI: Vulnerable
```

Для Rust в ядре есть `-Zretpoline-external-thunk`: вызовы идут через
`__x86_indirect_thunk_r11`, который подставляет ядро. `core` ядро
собирает само, с теми же флагами, так что ошибки о несовпадении [ABI][g-abi] не
возникает.

</div>

<div class="box check">

**Проверь себя.** Сколько загрузок из памяти выполняет `ch11_poll_dyn`
на один датчик и сколько из них зависят друг от друга? Почему
`ch11_devirt_static` девиртуализирована, а `ch11_poll_dyn(&SENSORS)` из
другого крейта — нет? Что быстрее на машине с retpoline: 1000 вызовов
через `dyn` вперемешку или те же вызовы по порядку типов? Зачем
переходнику `lfence`, если `pause` и так ничего не делает?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-abi]: ../appendix-f.md#abi "ABI (application binary interface): Двоичный интерфейс платформы: соглашение о вызовах, раскладка типов в памяти, имена символов."
[g-black-box]: ../appendix-f.md#black-box "black_box: Функция std::hint::black_box(x): возвращает свой аргумент, но оптимизатор не знает, что она с ним делает."
[g-ffi]: ../appendix-f.md#ffi "FFI (foreign function interface): Вызов функций, написанных на другом языке, и показ своих функций ему."
[g-vectorization]: ../appendix-f.md#vectorization "Векторизация (vectorization): Оптимизация, при которой компилятор переписывает цикл на SIMD-команды, обрабатывающие несколько элементов за раз, и добавляет скалярный хвост для остатка."
[g-wait-states]: ../appendix-f.md#wait-states "Такты ожидания (wait states): Пустые такты, которые ядро ждёт при чтении медленной флеш-памяти на высокой частоте."
