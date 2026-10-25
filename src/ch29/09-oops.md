# 29.9. Паника, oops и отладчик

<div class="box ref">

**Справка: oops и паника ядра.** Когда процессор ловит исключение в
режиме ядра — обращение по плохому адресу, неверную инструкцию, — ядро
не может вернуть ошибку: некому. Оно печатает *oops* — отчёт с
регистрами и стеком вызовов — и убивает задачу, в контексте которой
случилось исключение, как будто та получила `SIGSEGV`. Остальное ядро
продолжает работать, но ему уже нельзя вполне доверять: убитая задача
могла держать блокировки и оставить структуры наполовину изменёнными.
Поэтому ядро помечает себя «испорченным» (taint) флагом D. Если
исключение случилось там, где убивать некого — в обработчике
прерывания, в задаче `init`, — или если задан `panic_on_oops`, oops
переходит в *панику ядра* (kernel panic): машина останавливается.
`BUG()` — способ сказать «здесь ядро сломано» нарочно: на x86 это
инструкция `ud2`, неверная по определению.

</div>

## Паника Rust — это oops

Модуль `ch29_oops` регистрирует `/dev/ch29-oops`, и запись в него —
команда для функции, которая паникует по-разному:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_oops.rs:trigger}}
```

`black_box` не даёт оптимизатору вычислить ответ при компиляции:
иначе rustc увидел бы гарантированную панику и, например, выход за
границы массива известной длины отверг бы ещё при сборке. Init гостя
пишет команды из дочернего процесса — его ядро и убьёт:

```rust,ignore
{{#include ../../code/ch29/src/bin/ch29-guest.rs:oops}}
```

Первая команда (тест `oops_plain`; часть регистров опущена):

```text
# echo unwrap > /dev/ch29-oops
процесс-писатель: Signaled(Pid(34), SIGSEGV, false)
info:   ch29_oops: команда: unwrap
emerg:  rust_kernel: panicked at ch29_oops.rs:58:22:
called `Option::unwrap()` on a `None` value
warn:   ------------[ cut here ]------------
crit:   kernel BUG at rust/helpers/bug.c:7!
warn:   Oops: invalid opcode: 0000 [#1] SMP
warn:   CPU: 0 UID: 0 PID: 34 Comm: init Tainted: G           O        7.0.0 #1 PREEMPTLAZY 
warn:   Tainted: [O]=OOT_MODULE
warn:   RIP: 0010:rust_helper_BUG+0x5/0x10
warn:   Code: 0f 1f 44 00 00 0f 1f 44 00 00 48 8d 87 f0 00 00 00 c3 0f 1f 00 0f 1f 44 00 00 48 8d 87 10 ff ff ff c3 0f 1f 00 0f 1f 44 00 00 <0f> 0b 66 0f 1f 84 00 00 00 00 00 0f 1f 44 00 00 89 f8 85 ff 75 01
warn:   RSP: 0018:ffffc900001abd28 EFLAGS: 00000286
…
warn:   Call Trace:
warn:    <TASK>
warn:    _RNvCs9hJ03s5DiqP_7___rustc17rust_begin_unwind+0x43/0x43
warn:    ? _RNvXsM_NtNtCs3R38tTcvFoT_6kernel6device8propertyAxj1_NtB5_8Property25read_from_fwnode_propertyB9_+0x4b/0x4b
warn:    _RNvNtCsi1tY5tVYnWh_4core9panicking9panic_fmt+0x25/0x25
warn:    _RNvNtCsi1tY5tVYnWh_4core9panicking5panic+0xe/0xe
warn:    _RNvNtCsi1tY5tVYnWh_4core6option13unwrap_failed+0x15/0x15
warn:    _RNvCskVWChMwOrSO_9ch29_oops7trigger+0x1e9/0x21e [ch29_oops]
warn:    _RNvXs2_CskVWChMwOrSO_9ch29_oopsNtB5_4OopsNtNtCs3R38tTcvFoT_6kernel10miscdevice10MiscDevice10write_iter+0xf7/0x147 [ch29_oops]
warn:    ? _RNvXs_CskVWChMwOrSO_9ch29_oopsNtB4_6LOCKEDNtNtNtNtCs3R38tTcvFoT_6kernel4sync4lock6global17GlobalLockBackend14get_lock_class+0x8/0x8 [ch29_oops]
warn:    _RNvMs2_NtCs3R38tTcvFoT_6kernel10miscdeviceINtB5_16MiscdeviceVTableNtCskVWChMwOrSO_9ch29_oops4OopsE10write_iterB14_+0x1d/0x54 [ch29_oops]
warn:    vfs_write+0x1ac/0x240
warn:    ksys_write+0x61/0xb0
warn:    do_syscall_64+0x78/0x210
warn:    entry_SYSCALL_64_after_hwframe+0x4b/0x53
…
warn:    </TASK>
warn:   Modules linked in: ch29_oops(O)
warn:   ---[ end trace 0000000000000000 ]---
warn:   RIP: 0010:rust_helper_BUG+0x5/0x10
…
```

Цепочка такая. `unwrap` на `None` зовёт `core::option::unwrap_failed`,
та — механизм паники `core`, а он — [обработчик паники][g-panic-handler], который крейт
`kernel` определяет для всего ядра:

<!-- linux: rust/kernel/lib.rs -->
```rust,ignore
#[panic_handler]
fn panic(info: &core::panic::PanicInfo<'_>) -> ! {
    pr_emerg!("{}\n", info);
    // SAFETY: FFI call.
    unsafe { bindings::BUG() };
}
```

`pr_emerg!` печатает место и сообщение паники — отсюда `rust_kernel:
panicked at ch29_oops.rs:58:22` (`rust_kernel` — имя модуля крейта
`kernel` в журнале), — а `BUG()` в Rust недоступен как макрос и идёт
через помощника:

<!-- linux: rust/helpers/bug.c -->
```c
__rust_helper __noreturn void rust_helper_BUG(void)
{
	BUG();
}
```

`<0f> 0b` в строке `Code:` — та самая `ud2`, на которой стоит `RIP`.
Исключение «неверная инструкция» ядро узнаёт как `BUG()` по таблице
мест, печатает `kernel BUG at rust/helpers/bug.c:7!` — файл и строку
помощника, а не модуля, — и делает oops. Писатель убит `SIGSEGV`, init
ничего не заметил.

Переполнение в `overflow` — тоже паника: проверки переполнения в ядре
по умолчанию включены, в отличие от сборки `--release` в пространстве
пользователя, и для всего кода Rust, включая `core`:

<!-- linux: lib/Kconfig.debug -->
```text
config RUST_OVERFLOW_CHECKS
	bool "Overflow checks"
	default y
…
	  overflow. When overflow-checks are enabled, a Rust panic will occur
	  on overflow.
```

## Убит процесс, а не ядро

Сеанс продолжается: `index` и `overflow` дают oops `[#2]` и `[#3]`, с
тем же путём через `rust_helper_BUG`. В строке `Tainted` второго oops
появляется D:

```text
warn:   Oops: invalid opcode: 0000 [#2] SMP
warn:   CPU: 0 UID: 0 PID: 36 Comm: init Tainted: G      D    O        7.0.0 #1 PREEMPTLAZY 
warn:   Tainted: [D]=DIE, [O]=OOT_MODULE
…
# cat /proc/sys/kernel/tainted
4224
```

`/proc/sys/kernel/tainted` — битовая маска флагов:

<!-- linux: include/linux/panic.h -->
```c
#define TAINT_DIE			7
…
#define TAINT_OOT_MODULE		12
```

2⁷ + 2¹² = 128 + 4096 = 4224: «ядро уже падало» и «загружен внешний
модуль». Отчёт об ошибке с такого ядра разработчики ядра читают с
поправкой: после первого oops состояние уже могло быть испорчено.

Что значит «убита задача» для Rust, показывает последняя команда.
`locked` берёт мьютекс и паникует, не отпустив его:

```text
# echo locked > /dev/ch29-oops
процесс-писатель: Signaled(Pid(38), SIGSEGV, false)
info:   ch29_oops: команда: locked
emerg:  rust_kernel: panicked at ch29_oops.rs:68:13:
паника с захваченным мьютексом
…
warn:   Oops: invalid opcode: 0000 [#4] SMP
…
# echo locked > /dev/ch29-oops
write: Device or resource busy (os error 16)
процесс-писатель: Exited(Pid(39), 1)
info:   ch29_oops: команда: locked
info:   ch29_oops: мьютекс занят: охранник прошлой паники не отпущен
```

Охранник `_guard` так и не выполнил `Drop`: паника в ядре не
раскручивает стек. Код Rust ядра собирается с `-Cpanic=abort`:

<!-- linux: Makefile -->
```text
KBUILD_RUSTFLAGS := $(rust_common_flags) \
		    -Cpanic=abort -Cembed-bitcode=n -Clto=n \
```

— а обработчик паники возвращает `!` и кончается `BUG()`. Дальше
задачу снимает `oops_end`, и стек она не раскручивает, а сбрасывает:

<!-- linux: arch/x86/kernel/dumpstack.c -->
```c
	/*
	 * We're not going to return, but we might be on an IST stack or
	 * have very little stack space left.  Rewind the stack and kill
	 * the task.
…
	rewind_stack_and_make_dead(signr);
```

Весь код Rust выше по стеку пропущен: ни одного `Drop`. Модуль главы берёт
мьютекс через `try_lock` и видит его занятым; обычный `lock()` уснул бы
навсегда, и вместе с ним — каждый следующий писатель.

<div class="box trap">

**Ловушка: «паника — это безопасно».** В пространстве пользователя
паника по умолчанию раскручивает стек, отпускает `MutexGuard` (помечая
мьютекс [отравленным][g-poisoning]) и роняет всё по дороге — программа может
поймать её `catch_unwind` или хотя бы умереть чисто. В ядре нет ничего
из этого: паника — это `BUG()`, утечка всего, что держал код, и
блокировки, захваченные навсегда. Паника в модуле — ошибка того же
веса, что разыменование `NULL` в C. Отсюда стиль крейта `kernel`: `Result`
вместо `unwrap`, `try_*`-методы, `checked_*`-арифметика там, где
переполнение возможно.

</div>

<div class="box trap">

**Ловушка: сводка регистров — от первого oops.** После строки `end
trace` ядро повторяет `RIP` и регистры — «на случай, если отчёт уехал с
экрана». Но это регистры *первого* oops с момента загрузки, а не
текущего:

<!-- linux: arch/x86/kernel/dumpstack.c -->
```c
	/* Save the regs of the first oops for the executive summary later. */
	if (!die_counter)
		exec_summary_regs = *regs;
```

В сеансе это видно по `RSP`: у oops `[#2]` в отчёте
`ffffc900001bbd08`, а в сводке после него — `ffffc900001abd28`, `RSP`
oops `[#1]` (тест сверяет и это). Регистры управления (`CR0`…`CR4`)
`__show_regs` читает заново, и они в сводке — текущие. Разбирая второй
и следующие oops, читайте регистры до `end trace`.

</div>

Переполнение стека ядра проходит тот же путь через `die` и
`oops_end`, только исключение — от сторожевой страницы под стеком
задачи, а не от `ud2`, и кончается путь иначе. Обработчик двойного
сбоя входит в ядро как NMI, `oops_end` видит контекст прерывания и
вместо `rewind_stack_and_make_dead` вызывает `panic`; отчёт — в разделе
[29.4](04-memory.md).

## Имена и строки в трассе

Имена функций Rust в трассе — искажённые (mangled) по схеме v0: ядро
собирает Rust с `-Csymbol-mangling-version=v0` и печатает имена из
`kallsyms` как есть. `_RNvCskVWChMwOrSO_9ch29_oops7trigger` читается
так: `_R` — символ Rust, `Nv` — имя в пространстве значений, `C…_` —
корень крейта с хешем-различителем, дальше идентификаторы с длиной
впереди: `9ch29_oops`, `7trigger`. То есть `ch29_oops::trigger`.

После имени — смещение и длина функции: `trigger+0x1e9/0x21e`. Это
адрес *возврата* — инструкция после вызова `unwrap_failed`. У
`overflow` в трассе `trigger+0x21e/0x21e`: смещение равно длине, адрес
возврата указывает за конец функции, потому что вызов паники —
последняя её инструкция и возврата после него нет. Строки со знаком `?`
— адреса из кода ядра, которые нашлись в стеке, но которые раскрутчик
ORC не считает настоящими кадрами: обычно это мусор от прошлых вызовов.

Расшифровать трассу помогает скрипт ядра `scripts/decode_stacktrace.sh`:
он разбирает имена (через `llvm-cxxfilt`) и по отладочной информации
`vmlinux` и `.ko` подставляет файл и строку. Отладочная информация в ядре
главы есть: `CONFIG_DEBUG_INFO_DWARF5`. Трасса первого oops (тест
`oops_trace_lines`; пути к исходникам `core` сокращены):

```console
$ LLVM=1 bash target/ch29-kernel/linux-7.0/scripts/decode_stacktrace.sh \
    target/ch29-kernel/build/vmlinux "$PWD" target/tmp/ch29-modules-plain < trace.txt
Call Trace:
<TASK>
__rustc::rust_begin_unwind (target/ch29-kernel/linux-7.0/rust/kernel/pid_namespace.rs:36)
? <[i64; 1] as kernel::device::property::Property>::read_from_fwnode_property (target/ch29-kernel/linux-7.0/rust/kernel/fmt.rs:47 target/ch29-kernel/linux-7.0/rust/kernel/fmt.rs:54)
core::panicking::panic_fmt (…/library/core/src/cell/lazy.rs:382)
core::panicking::panic (…/library/core/src/panicking.rs:60)
core::option::unwrap_failed (…/library/core/src/result.rs:1866)
 ch29_oops::trigger (code/ch29/modules/ch29_oops.rs:68) ch29_oops
 <ch29_oops::Oops as kernel::miscdevice::MiscDevice>::write_iter (…/library/core/src/result.rs:2173 code/ch29/modules/ch29_oops.rs:91) ch29_oops
…
vfs_write (target/ch29-kernel/linux-7.0/fs/read_write.c:596 target/ch29-kernel/linux-7.0/fs/read_write.c:688)
ksys_write (target/ch29-kernel/linux-7.0/fs/read_write.c:740)
```

Имена теперь читаемые, а строки — нет. `trigger` упал на строке 58
(`found.unwrap()`), а скрипт показал 68 — `panic!` из ветки `locked`.
`rust_begin_unwind` оказался в `pid_namespace.rs`, а `unwrap_failed` —
в `result.rs`, хотя он в `option.rs`. Скрипт отдаёт `addr2line` адрес
возврата как есть, а адрес возврата — уже *следующая* инструкция: после
вызова, который не возвращается, это код другой ветки или вовсе другой
функции. Строку вызова даёт адрес на байт раньше. Начало `trigger` в
`.ko` — 0xd5 (`llvm-nm`), значит, 0xd5 + 0x1e9 − 1 = 0x2bd:

```console
$ llvm-addr2line -i -e ch29_oops.ko 0x2bd
…/library/core/src/option.rs:1013
…/code/ch29/modules/ch29_oops.rs:58
```

`-i` показывает и встроенные функции: `Option::unwrap` из `option.rs`
встроен в `trigger`, а вызвана паника со строки 58.

## Модуль в gdb

QEMU умеет быть сервером gdb: `-gdb tcp:127.0.0.1:порт` открывает порт,
`-S` держит процессор до команды отладчика. Ядро главы собрано с
`CONFIG_GDB_SCRIPTS`: в каталоге сборки лежит `vmlinux-gdb.py` с
командами `lx-*`, которые понимают структуры ядра. Сценарий gdb из
теста `gdb_stops_in_module_function`:

```rust,ignore
{{#include ../../code/ch29/tests/kernel.rs:script}}
```

Порядок важен. Когда gdb подключается, ядра ещё нет в памяти — процессор
стоит на первой инструкции BIOS, — и модуля `ch29_oops` тем более:
адреса `trigger` не знает никто. Поэтому сначала точка останова на
`do_init_module`: к этому моменту модуль загружен и в списке модулей, а
его `init` ещё не вызван. Точка аппаратная (`hbreak`, отладочные
регистры процессора): программную, `int3` в памяти, под KVM затёр бы
распаковщик ядра, когда кладёт ядро на место, а тест идёт и под KVM.
`lx-symbols` находит загруженные модули, ищет их `.ko` в текущем
каталоге и подгружает символы. Теперь можно ставить точку на функцию
модуля по имени Rust — `ch29_oops::trigger`.

Вывод gdb (пути сокращены до репозитория):

```text
Hardware assisted breakpoint 1 at 0xffffffff812ce9a0: file target/ch29-kernel/linux-7.0/kernel/module/main.c, line 3018.

Thread 1 hit Breakpoint 1, do_init_module (mod=<optimized out>) at target/ch29-kernel/linux-7.0/kernel/module/main.c:3018
3018	{
loading vmlinux
scanning for modules in target/tmp/ch29-modules-plain
loading @0xffffffffa0000000: target/tmp/ch29-modules-plain/ch29_oops.ko
Hardware assisted breakpoint 2 at 0xffffffffa00000e7: file code/ch29/modules/ch29_oops.rs, line 54.

Thread 1 hit Breakpoint 2, ch29_oops::trigger (cmd=...) at code/ch29/modules/ch29_oops.rs:54
54	    let table = [10u32, 20, 30];
### bt
#0  ch29_oops::trigger (cmd=...) at code/ch29/modules/ch29_oops.rs:54
#1  0xffffffffa000051d in ch29_oops::{impl#4}::write_iter (_kiocb=..., iov=<optimized out>) at code/ch29/modules/ch29_oops.rs:91
#2  0xffffffffa000036d in kernel::miscdevice::MiscdeviceVTable<ch29_oops::Oops>::write_iter<ch29_oops::Oops> (kiocb=<optimized out>, iter=0xffffc900001abde0) at target/ch29-kernel/linux-7.0/rust/kernel/miscdevice.rs:292
#3  0xffffffff8138f4ec in new_sync_write (filp=0xffff888000b9b240, buf=0x7f0d9b68e810 "overflowDeadlock\000", len=8, ppos=0xffffc900001abef8) at target/ch29-kernel/linux-7.0/fs/read_write.c:595
#4  vfs_write (file=0xffff888000b9b240, buf=0x7f0d9b68e810 "overflowDeadlock\000", count=8, pos=0xffffc900001abef8) at target/ch29-kernel/linux-7.0/fs/read_write.c:688
#5  0xffffffff8138f6b1 in ksys_write (fd=<optimized out>, buf=0x7f0d9b68e810 "overflowDeadlock\000", count=8) at target/ch29-kernel/linux-7.0/fs/read_write.c:740
#6  0xffffffff81535378 in do_syscall_x64 (regs=0xffffc900001abf58, nr=<optimized out>) at target/ch29-kernel/linux-7.0/arch/x86/entry/syscall_64.c:63
#7  do_syscall_64 (regs=0xffffc900001abf58, nr=<optimized out>) at target/ch29-kernel/linux-7.0/arch/x86/entry/syscall_64.c:94
#8  0xffffffff810000ac in entry_SYSCALL_64 () at target/ch29-kernel/linux-7.0/arch/x86/entry/entry_64.S:121
…
### info args
cmd = &[u8] [111, 118, 101, 114, 102, 108, 111, 119]
### info locals
No locals.
### info symbol
ch29_oops::trigger + 18 in section .text of target/tmp/ch29-modules-plain/ch29_oops.ko
### lx-lsmod
Address            Module                  Size  Used by
### show language
The current source language is "auto; currently rust".
Warning: the current language does not match this frame.
### lx-lsmod, язык C
Address            Module                  Size  Used by
0xffffffffa0000000 ch29_oops              16384  0
### конец
```

Что здесь видно:

- `bt` идёт через границу языков без швов: `trigger` и метод
  `MiscDevice` модуля, обёртка `MiscdeviceVTable` из крейта `kernel` —
  та самая `extern "C"`-функция из таблицы `file_operations`, — потом
  VFS на C и вход системного вызова. Кадры после `entry_SYSCALL_64`
  (опущены) — мусор: дальше стек пространства пользователя, которого
  раскрутчик gdb не понимает;
- один и тот же буфер на C и на Rust. В кадре `vfs_write` это `char *buf`
  и отдельно `count=8`, и gdb печатает строку до нуля:
  `"overflowDeadlock\000"` — восемь байт команды и чужие байты за ними
  (соседняя строка в `.rodata` программы гостя). В кадре Rust это
  `cmd = &[u8]` — [срез][g-slice] с длиной, и gdb печатает ровно восемь байт;
- `ch29_oops::{impl#4}::write_iter` — так gdb называет метод в
  `impl MiscDevice for Oops`: по номеру блока `impl` в модуле. rustc
  нумерует и блоки, которые развернули макросы: когда в модуль добавили
  `global_lock!`, номер сдвинулся с 3 на 4;
- `info locals` пуст: о `table` и `found` в отладочной информации
  ничего нет — оптимизатор держит их в регистрах или выбросил, а без
  оптимизации ядро и модули не собираются.

После `detach` гость продолжает работу, и `overflow` даёт обычный oops.

<div class="box trap">

**Ловушка: `lx-lsmod` пуст.** В кадре Rust `lx-lsmod` печатает только
заголовок — без ошибки, будто модулей нет. Скрипты ядра вычисляют
выражения вроде `modules` на текущем языке gdb, а язык в кадре Rust —
Rust: имя ищется как путь Rust и не находится. После `set language c`
список на месте. Тот же совет — для любых команд `lx-*`, которые
молча возвращают пустое: в кадре Rust сначала `set language c`.

</div>

<div class="box trap">

**Ловушка: `lx-symbols` с аргументом.** Справка команды обещает, что
каталоги поиска модулей можно передать аргументами, но в 7.0 вариант с
аргументом падает с `AttributeError`:

<!-- linux: scripts/gdb/linux/symbols.py -->
```python
        for p in arg.split():
            if p == "-bpf":
                monitor_bpf = True
            else:
                p.append(os.path.abspath(os.path.expanduser(p)))
        self.module_paths.append(os.getcwd())
```

`p` — строка, у неё нет `append`; должно было быть
`self.module_paths.append(...)`. Текущий каталог в список попадает
всегда, поэтому тест запускает gdb из каталога модулей и зовёт
`lx-symbols` без аргументов.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `kernel BUG at` называет `rust/helpers/bug.c`, а не файл
   модуля, и где в отчёте искать место паники?
2. После oops в `write_iter` модуль остался загруженным и `rmmod`
   прошёл. Всегда ли так? Что было бы, если бы паника случилась в
   `init` модуля?
3. В трассе `overflow` — `trigger+0x21e/0x21e`. Какую строку покажет
   для этого адреса `decode_stacktrace.sh` и почему ей нельзя верить?
4. Почему точку останова на `trigger` нельзя поставить сразу после
   подключения gdb, а на `do_init_module` — можно?

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-panic-handler]: ../appendix-f.md#panic-handler "Обработчик паники (panic handler): Функция с атрибутом #[panic_handler], которую core вызывает при панике в программе без std."
[g-poisoning]: ../appendix-f.md#poisoning "Отравление (poisoning): Пометка Mutex или RwLock, что поток запаниковал, держа замок: следующий lock возвращает Err(PoisonError), и данные можно забрать через into_inner, приняв, что они могли остаться наполовину изменёнными."
[g-slice]: ../appendix-f.md#slice "Срез (slice): Непрерывная последовательность элементов, длина которой известна только во время выполнения: [T], а str — срез байт UTF-8."
