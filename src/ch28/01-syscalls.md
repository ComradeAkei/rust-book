# 28.1. Системные вызовы: std, libc, rustix, nix

## Четыре уровня одного вызова

Системный вызов на x86-64 — инструкция `syscall`: номер в `rax`,
аргументы в регистрах, результат или `-errno` — снова в `rax`. Как это
выглядит без всякой библиотеки, показал раздел
[26.2](../ch26/02-linux.md). В обычной программе между кодом и
инструкцией стоит одна из четырёх прослоек, и все они в итоге делают
один и тот же вызов. Пишем строку в stdout четырьмя способами
(`code/ch28/src/layers.rs`).

**std.** `Stdout` — общий для процесса буфер под замком. Строка,
которая кончается переводом строки, уходит одним `write`:

```rust,ignore
{{#include ../../code/ch28/src/layers.rs:std}}
```

**libc.** Крейт `libc` — объявления функций glibc и констант из её
заголовков, без единой строки логики. Вызов — `unsafe`: компилятор не
знает, живы ли указатель и длина. Ошибка — `-1` и `errno`, короткую
запись и прерывание сигналом надо обработать самому:

```rust,ignore
{{#include ../../code/ch28/src/layers.rs:libc}}
```

**rustix.** Безопасные обёртки над вызовами Linux и других Unix.
Дескриптор — `BorrowedFd` (раздел [28.2](02-fds.md)), ошибка — `Errno`.
На Linux у rustix свой бэкенд `linux_raw`: вызов делается инструкцией
`syscall` прямо в теле функции, без glibc и без `errno`:

```rust,ignore
{{#include ../../code/ch28/src/layers.rs:rustix}}
```

**nix.** Тоже безопасные обёртки, но поверх libc: вызов идёт через glibc,
`errno` читается после него. В nix есть то, чего нет в rustix: `fork`,
`sigaction`, маски сигналов — rustix не даёт их намеренно, потому что
glibc держит своё состояние вокруг них, и обойти её значит его сломать:

```rust,ignore
{{#include ../../code/ch28/src/layers.rs:nix}}
```

Под strace все четыре — один и тот же вызов:

```console
$ strace -qq -o layers.strace -e trace=write,open,openat target/debug/layers
$ grep -v 'libgcc_s\|libc.so\|ld.so' layers.strace
openat(AT_FDCWD, "/proc/self/maps", O_RDONLY|O_CLOEXEC) = 3
write(1, "std\n", 4)                    = 4
write(1, "libc\n", 5)                   = 5
write(1, "rustix\n", 7)                 = 7
write(1, "nix\n", 4)                    = 4
openat(AT_FDCWD, "/\320\275…", O_RDONLY|O_CLOEXEC) = -1 ENOENT (No such file or directory)
openat(AT_FDCWD, "/\320\275…", O_RDONLY|O_CLOEXEC) = -1 ENOENT (No such file or directory)
open("/\320\275…", O_RDONLY|O_LARGEFILE|O_CLOEXEC) = -1 ENOENT (No such file or directory)
openat(AT_FDCWD, "/\320\275…", O_RDONLY|O_CLOEXEC) = -1 ENOENT (No such file or directory)
…
```

Путь `/нет/такого/файла` strace печатает байтами UTF-8 в восьмеричном
виде; здесь он сокращён. Тест `one_write_per_layer_and_two_ways_to_open`
из `tests/strace.rs` проверяет и четыре `write`, и разницу в открытии:
std, libc и nix открывают файл через `openat(AT_FDCWD, …)`, а rustix —
старым вызовом `open` и с флагом `O_LARGEFILE`. Ядро на 64-битной
системе ставит этот флаг каждому открытию само, rustix просто передаёт
его явно; на результат это не влияет.

## Одна ошибка — четыре значения

Вторая половина программы открывает несуществующий файл на каждом
уровне и печатает ошибку и её размер:

```rust,ignore
{{#include ../../code/ch28/src/layers.rs:errors}}
```

```console
$ cargo run -q -p ch28 --bin layers
std
libc
rustix
nix
std    8 байт: No such file or directory (os error 2)
libc   4 байт: No such file or directory (os error 2)
rustix 2 байт: No such file or directory (os error 2)
nix    4 байт: ENOENT: No such file or directory
```

Код один — ENOENT, 2. Разное — то, что вокруг него. `rustix::io::Errno`
— обёртка над `u16`: номеров ошибок в Linux меньше 4096. `nix::Errno` —
перечисление с вариантами `ENOENT`, `EINTR` и остальными, его удобно
сопоставлять в `match`. `io::Error` в std занимает 8 байт и умеет больше:
это указатель с тегом в двух младших битах. Код ошибки ОС хранится прямо
в значении, без кучи; собственное сообщение (`io::Error::new`) лежит в
`Box`, и тогда в значении — указатель на него. Все три переводятся в
`io::Error` через `From`, поэтому `?` работает на границе уровней —
этим пользуются функции из `layers.rs`.

<div class="box c">

**Как в C.** `errno` — не переменная, а макрос:
`(*__errno_location())`, адрес своей копии у каждого потока. Значение
надо забрать сразу после неудачного вызова: следующий вызов libc, даже
успешный, вправе его затереть. Поэтому в C пишут `int saved = errno;`
перед `close` на пути ошибки — пример есть в `code/ch28/c/copy.c`
(раздел [28.2](02-fds.md)). В Rust `io::Error::last_os_error()` берёт
`errno` в момент вызова и дальше не зависит от него — но вызвать её надо
тоже сразу, как в `say_libc` выше.

</div>

## Что std делает до main

Пустая программа на Rust делает до `main` несколько вызовов, которых нет
у программы на C. Тест `std_runtime_before_main` проверяет их порядок;
вот полный журнал от первого вызова после загрузки библиотек до выхода
(программа `procs` без аргументов печатает подсказку и выходит с кодом
2):

```console
$ strace -qq target/debug/procs 2>&1 | sed -n '/^poll/,$p'
poll([{fd=0, events=0}, {fd=1, events=0}, {fd=2, events=0}], 3, 0) = 0 (Timeout)
rt_sigaction(SIGPIPE, {sa_handler=SIG_IGN, sa_mask=[PIPE], sa_flags=SA_RESTORER|SA_RESTART, …}, …) = 0
getrandom("\x63\xe1\x72\xb2\xa6\x0c\xab\xeb", 8, GRND_NONBLOCK) = 8
brk(NULL)                               = 0x5606e6b9a000
brk(0x5606e6bbb000)                     = 0x5606e6bbb000
openat(AT_FDCWD, "/proc/self/maps", O_RDONLY|O_CLOEXEC) = 3
prlimit64(0, RLIMIT_STACK, NULL, {rlim_cur=8192*1024, rlim_max=RLIM64_INFINITY}) = 0
fstat(3, {st_mode=S_IFREG|0444, st_size=0, ...}) = 0
read(3, "5606abac3000-5606abaed000 r--p 0"..., 1024) = 1024
read(3, "86b95b1000-7f86b9600000 r--p 001"..., 1024) = 1024
read(3, "000 00:00 0 \n7f86b983b000-7f86b9"..., 1024) = 1024
close(3)                                = 0
sched_getaffinity(21194, 32, [0 1 2 3]) = 32
rt_sigaction(SIGSEGV, NULL, {sa_handler=SIG_DFL, sa_mask=[], sa_flags=0}, 8) = 0
sigaltstack(NULL, {ss_sp=NULL, ss_flags=SS_DISABLE, ss_size=0}) = 0
mmap(NULL, 16048, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS|MAP_STACK, -1, 0) = 0x7f86b9835000
mprotect(0x7f86b9835000, 4096, PROT_NONE) = 0
sigaltstack({ss_sp=0x7f86b9836000, ss_flags=0, ss_size=11952}, NULL) = 0
gettid()                                = 21194
rt_sigaction(SIGSEGV, {sa_handler=0x5606abb3f0b0, sa_mask=[], sa_flags=SA_RESTORER|SA_ONSTACK|SA_SIGINFO, …}, NULL, 8) = 0
rt_sigaction(SIGBUS, NULL, {sa_handler=SIG_DFL, sa_mask=[], sa_flags=0}, 8) = 0
rt_sigaction(SIGBUS, {sa_handler=0x5606abb3f0b0, sa_mask=[], sa_flags=SA_RESTORER|SA_ONSTACK|SA_SIGINFO, …}, NULL, 8) = 0
write(2, "\320\270\321\201\320\277\320\276\320\273\321\214\320\267\320\276\320\262\320\260\320\275\320\270\320\265: proc"..., 60) = 60
sigaltstack({ss_sp=NULL, ss_flags=SS_DISABLE, ss_size=11952}, NULL) = 0
munmap(0x7f86b9835000, 16048)           = 0
exit_group(2)                           = ?
```

По порядку:

- **`poll` по дескрипторам 0, 1, 2** с нулевым ожиданием. Если какой-то
  из них закрыт (`POLLNVAL`), std открывает на его место `/dev/null`.
  Иначе первый же `File::open` получил бы номер 1, и `println!` писал бы
  в этот файл. Тест `closed_stdout_is_reopened_as_dev_null` закрывает
  stdout до `exec` и находит в журнале `revents=POLLNVAL` и
  `openat(AT_FDCWD, "/dev/null", O_RDWR) = 1`;
- **SIGPIPE → SIG_IGN.** Запись в канал без читателя вернёт `EPIPE`, а не
  убьёт процесс; к чему это приводит, — в разделе
  [28.4](04-signals.md);
- **`getrandom` на 8 байт и `brk`** — первый `malloc`: glibc берёт
  случайный ключ для проверок своего кеша свободных блоков и растит кучу;
- **`/proc/self/maps`, `prlimit64(RLIMIT_STACK)` и `sched_getaffinity`**
  — это glibc `pthread_getattr_np` для главного потока: границу его стека
  можно узнать только из maps и предела стека, а маску процессоров эта
  функция заполняет заодно. std спрашивает границу, чтобы знать, где
  сторожевая страница стека;
- **`sigaltstack` и обработчики SIGSEGV и SIGBUS.** При переполнении
  стека сбой происходит на сторожевой странице, и обработчику нужен
  другой стек — отдельные 16 048 байт: сторожевая страница и 11 952
  байта стека.
  Обработчик сверяет адрес сбоя со сторожевой страницей и печатает
  `thread 'main' has overflowed its stack`; любой другой SIGSEGV он
  возвращает к действию по умолчанию.

После `main` — снять запасной стек и `exit_group`: процесс целиком, а не
только поток (раздел [26.2](../ch26/02-linux.md)).

<div class="box hood">

**Под капотом: 11 952 байта.** Размер запасного стека — не константа из
заголовка. std берёт больший из `SIGSTKSZ` (8192) и `AT_MINSIGSTKSZ` из
вспомогательного вектора — минимума, который сообщает ядро. На этой
машине:

```console
$ LD_SHOW_AUXV=1 /bin/true | grep MINSIG
AT_MINSIGSTKSZ:       11952
```

Ядро кладёт на стек обработчика всё состояние процессора, сохранённое
`xsave`, а у этого Xeon есть матричные регистры AMX: одни только плитки
занимают 8 КиБ. Постоянный `SIGSTKSZ` из старых заголовков на таком
процессоре мал, и стек, выделенный по нему, ядро переполнило бы.

</div>

## Цена вызова и vDSO

Сколько стоит переход в ядро и обратно, меряет программа `syscost`:
каждая операция миллион раз, лучший из пяти замеров.

```rust,ignore
{{#include ../../code/ch28/src/bin/syscost.rs:cost}}
```

```console
$ cargo run -q -p ch28 --release --bin syscost
вызов функции:                    1.5 нс
getppid через rustix (asm):      95.9 нс
getppid через libc:              97.6 нс
Instant::now (vDSO):             25.0 нс
clock_gettime через syscall:    140.0 нс
vDSO быстрее вызова ядра: true
```

`getppid` — самый дешёвый вызов из возможных: ядро читает одно поле
задачи. И всё же он в 60 раз дороже вызова функции — около 200 тактов на
2,1 ГГц. Время уходит не на работу, а на переход: `syscall`, сохранение
регистров на стек ядра, защиты от Spectre на входе и выходе, `sysret`.
rustix сэкономил на PLT и `errno` пару наносекунд — на фоне перехода это
шум.

`clock_gettime` через `libc::syscall` дороже `getppid`: ядро ещё читает
часы и переводит такты в наносекунды. А `Instant::now` в пять с лишним
раз дешевле — потому что в ядро не ходит вовсе.

<div class="box ref">

**Справка: vDSO.** Ядро отображает в каждый процесс маленькую
разделяемую библиотеку — vDSO, «virtual dynamic shared object», 8 КиБ
кода (её видно в `/proc/self/maps` как `[vdso]`, раздел
[28.7](07-proc.md)), — и рядом страницу данных `[vvar]`, куда само пишет
параметры часов. `clock_gettime` из glibc находит в vDSO функцию
`__vdso_clock_gettime`: она читает счётчик тактов инструкцией `rdtsc` и
параметры из `[vvar]` под последовательным замком (seqlock) и считает
время в пространстве пользователя. В ядро она уходит, только если
источник часов не годится для чтения из пользовательского режима
(`/sys/devices/system/clocksource/clocksource0/current_clocksource` —
`tsc` здесь). Так же через vDSO работают `gettimeofday`, `time` и
`getcpu`. Адрес vDSO glibc узнаёт из вспомогательного вектора,
`AT_SYSINFO_EHDR` (раздел [26.2](../ch26/02-linux.md)).

</div>

Вывод: системный вызов — это сотня наносекунд, сколько бы ни делал сам
вызов. Поэтому `BufWriter` из главы [15](../ch15/index.md) копит байты и
пишет одним `write`, поэтому `read` с буфером в 64 КиБ быстрее побайтового
в тысячи раз, и поэтому `Instant::now` можно звать в горячем цикле, а
`getpid` — лучше не надо. Точные числа зависят от процессора, ядра и
защит; тест `vdso_is_cheaper_than_a_syscall` проверяет только
соотношение.

<div class="box check">

**Проверь себя.**

1. Почему `say_libc` проверяет `n < 0`, а `say_rustix` — нет? Где в
   rustix спрятано это сравнение?
2. Почему std проверяет три стандартных дескриптора одним `poll`, а не
   тремя `fcntl(F_GETFD)`? Оцените разницу по замерам `syscost`.
3. Почему rustix не даёт `fork`, хотя это самый обычный системный вызов?

</div>
