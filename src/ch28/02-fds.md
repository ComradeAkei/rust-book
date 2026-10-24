# 28.2. Дескрипторы и I/O safety

<div class="box ref">

**Справка: таблица дескрипторов.** У каждого процесса есть таблица
открытых файлов: номер — индекс в ней, элемент — указатель на `struct
file` ядра, где лежат позиция, флаги и ссылка на сам файл. `open`
занимает **наименьший свободный номер**, `close` освобождает его, `dup`
и `fork` добавляют ещё одну ссылку на тот же `struct file`. Таблица общая
для всех потоков процесса. Отсюда главная опасность голого номера:
`close(fd)` два раза подряд в однопоточной программе вернёт `EBADF`, а в
многопоточной между двумя вызовами другой поток может открыть файл и
получить тот же номер — и второй `close` закроет уже его файл.

</div>

## Владение и заимствование дескриптора

В std дескриптор — три типа и два трейта из `std::os::fd`:

| Тип | Что значит |
|---|---|
| `RawFd` | `i32`, голый номер. Ничего не гарантирует |
| `OwnedFd` | дескриптор во владении: закрывается в `Drop`, ровно один раз |
| `BorrowedFd<'a>` | заимствование на время `'a`; закрыть его нельзя |
| `AsFd` | «у меня можно одолжить дескриптор»: `File`, `OwnedFd`, `TcpStream`, `Stdin`, `ChildStdout`… |
| `AsRawFd` | «у меня можно узнать номер» — только для печати и вызовов, которым нужен номер |

Функция, которой нужен любой открытый дескриптор на время вызова,
принимает `impl AsFd`:

```rust,ignore
{{#include ../../code/ch28/src/fd.rs:size}}
```

Ей можно отдать `&File`, `&OwnedFd`, `file.as_fd()` или `io::stdin()`, и
ни одним из них она не сможет распорядиться: `BorrowedFd` не закрывает
дескриптор и не переживёт владельца. Тест `size_through_any_fd`
вызывает её тремя способами. rustix и nix принимают дескрипторы так же:
`rustix::fs::fstat(fd: impl AsFd)`.

Это и называется I/O safety. Правило звучит как правило о памяти: в
безопасном коде нельзя закрыть или использовать дескриптор, которым не
владеешь и который не одолжил. Единственный путь от номера к владельцу
— `unsafe { OwnedFd::from_raw_fd(raw) }`, и обязанность «этот номер
открыт и больше ничей» берёт на себя тот, кто пишет `unsafe`.

## Закрыть на каждом пути

Копия файла:

```rust,ignore
{{#include ../../code/ch28/src/fd.rs:copy}}
```

Два `?`, ни одного `close`. Если `File::create` вернул ошибку, `from`
закроется при выходе из функции; на успешном пути закроются оба, в
обратном порядке объявления. Под strace (тест
`copy_uses_copy_file_range_and_closes_on_error`):

```console
$ strace -qq -e trace=openat,copy_file_range,close target/debug/fds copy src dst
openat(AT_FDCWD, "src", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "dst", O_WRONLY|O_CREAT|O_TRUNC|O_CLOEXEC, 0666) = 4
copy_file_range(3, NULL, 4, NULL, 1073741824, 0) = 10000
copy_file_range(3, NULL, 4, NULL, 1073741824, 0) = 0
close(4)                                = 0
close(3)                                = 0
скопировано 10000 байт
$ strace -qq -e trace=openat,close target/debug/fds copy src нет/dst
openat(AT_FDCWD, "src", O_RDONLY|O_CLOEXEC) = 3
openat(AT_FDCWD, "\320\275\320\265\321\202/dst", O_WRONLY|O_CREAT|O_TRUNC|O_CLOEXEC, 0666) = -1 ENOENT (No such file or directory)
close(3)                                = 0
ошибка: No such file or directory (os error 2)
```

Строки журнала до `"src"` опущены. Два наблюдения. Каждый файл std
открывает с `O_CLOEXEC`: дескриптор не утечёт в программу, запущенную
через `exec` (раздел [28.3](03-processes.md)), — в C этот флаг надо не
забыть самому. И `io::copy` из файла в файл не гоняет байты через
буфер в пространстве пользователя: std распознаёт пару «файл — файл» и
зовёт `copy_file_range`, где копирует ядро, а файловая система с
reflink (Btrfs, XFS) может и не копировать, а разделить блоки между
файлами. Если вызов не поддержан, std откатывается на `read` и `write`.

<div class="box c">

**Как в C.** Та же копия — `code/ch28/c/copy.c`:

```c
{{#include ../../code/ch28/c/copy.c:copy}}
```

Привычный приём — один выход через `goto done`, где закрывается всё, что
открыто. Но держится он на дисциплине: вариант с `-DLEAK` возвращается из
середины функции мимо `done`, и дескриптор источника остаётся открытым.
Скрипт `code/ch28/c/run.sh` проверяет, кто это заметит:

```console
$ bash code/ch28/c/run.sh
ok    copy.c (ok): GCC -Wall -Wextra молчит
ok    copy.c (ok): -fanalyzer молчит
ok    copy.c (ok): clang --analyze молчит
ok    copy.c (leak): GCC -Wall -Wextra молчит
ok    copy.c (leak): -fanalyzer -O0 видит утечку in
ok    copy.c (leak): -fanalyzer -O2 утечку не видит
ok    copy.c (leak): clang --analyze молчит
…
ok    copy.c (leak): valgrind — три лишних дескриптора
ok    copy.c (leak): открыт источник
```

Компилятор молчит. Анализатор GCC 13 видит утечку
(`leak of file descriptor 'in' [CWE-775] [-Wanalyzer-fd-leak]`), только
пока оптимизация выключена; clang 18 — не видит. Находит её только
запуск: три неудачные попытки подряд под `valgrind --track-fds=yes`
оставляют три открытых дескриптора.

```console
$ valgrind -q --track-fds=yes ./copy-leak src нет/dst 3
попытка 1: ошибка: No such file or directory
попытка 2: ошибка: No such file or directory
попытка 3: ошибка: No such file or directory
==20552== FILE DESCRIPTORS: 6 open (3 std) at exit.
==20552== Open file descriptor 5: src
==20552==    at 0x4981445: open (open64.c:41)
…
```

Дескрипторы, открытые ещё родителем, valgrind тоже считает и помечает
`<inherited from parent>`. Раннер CI книги, например, оставляет
открытыми два своих (142 и 145), поэтому скрипт перед запуском
закрывает программе всё, кроме 0–2.

Ещё две детали C-версии, которых нет в Rust-версии, потому что их
делают типы: `errno` первой ошибки сохраняется до `close`, который его
затёр бы, а короткая запись `write` дописывается в цикле.

</div>

<div class="box trap">

**Ловушка: ошибку `close` Rust проглатывает.** C-версия проверяет
результат `close(out)`: для файла, в который писали, это не формальность
— NFS, например, сообщает о неудачной записи на сервер только при
`close`. `Drop` у `File` вернуть ошибку не может и молча её игнорирует —
так написано в документации `File`. Если потеря записанного важна,
перед концом жизни файла зовут `file.sync_all()?`: `fsync` дожидается
записи на устройство и возвращает ошибку записи, если она была. Тот же
приём — у журнала в разделе [28.8](08-project.md), только для `mmap`:
`msync`.

</div>

## Двойное закрытие

Что будет, если нарушить обещание `from_raw_fd` — сделать двух
владельцев одного номера:

```rust,ignore
{{#include ../../code/ch28/src/bin/fds.rs:double}}
```

```console
$ cargo run -q -p ch28 --bin fds -- double-close
первый владелец закрыл 3
fatal runtime error: IO Safety violation: owned file descriptor already closed, aborting
/bin/bash: line 1: 22567 Aborted                 cargo run -q -p ch28 --bin fds -- double-close
$ echo $?
134
```

В отладочной сборке std проверяет каждый дескриптор перед `close`:

```console
$ strace -qq -e trace=fcntl,close target/debug/fds double-close
fcntl(3, F_GETFD)                       = 0x1 (flags FD_CLOEXEC)
close(3)                                = 0
fcntl(3, F_GETFD)                       = -1 EBADF (Bad file descriptor)
--- SIGABRT {si_signo=SIGABRT, si_code=SI_TKILL, si_pid=20820, si_uid=0} ---
+++ killed by SIGABRT +++
```

`EBADF` от `fcntl` значит, что номер уже закрыт: владельцев было двое, и
std останавливает процесс до того, как второй `close` закроет что-нибудь
чужое. В выпускной сборке проверки нет: второй `close` получает `EBADF`,
`Drop` его не замечает, и программа печатает «второй владелец закрыл 3».
Тесты `double_close_is_caught_in_debug_builds` и
`owned_fd_drop_checks_the_number_in_debug_builds` проверяют оба
варианта, в зависимости от того, с какой сборкой их запустили. Защиты в
обоих случаях нет — есть только `unsafe`, которое указывает, где искать.

## memfd: файл без имени

`memfd_create` создаёт файл в памяти: дескриптор есть, имени в файловой
системе нет. Его можно писать, читать, отображать (раздел
[28.5](05-mmap.md)) и передавать другому процессу. rustix возвращает его
как `OwnedFd`, и `File::from` превращает владельца в `File` без единого
`unsafe`:

```rust,ignore
{{#include ../../code/ch28/src/fd.rs:memfd}}
```

Владение переходит из rustix в std по цепочке `OwnedFd → File → OwnedFd
→ Stdio`, и ни на одном шаге номер не теряется и не удваивается.
Отдать файл ребёнку как stdin — значит отдать `Stdio` владельца:

```rust,ignore
{{#include ../../code/ch28/src/fd.rs:stdin}}
```

`wc -c` из теста `child_reads_memfd` насчитывает 1000 байт. У родителя
дескриптор закрывается вместе с `Command`, в конце выражения: `Command`
владеет `Stdio`, а ребёнок получил свою копию номера при запуске.

У файла без имени всё-таки есть ссылка в `/proc/self/fd`:

```rust,ignore
{{#include ../../code/ch28/src/fd.rs:link}}
```

Для memfd это `/memfd:ch28-link (deleted)` (тест
`memfd_has_no_name_but_has_link`): ядро показывает файл как удалённый,
потому что у него нет ни одной ссылки в каталогах. Тест `drop_closes`
проверяет, что после `drop` номер больше не указывает на этот memfd.

<div class="box trick">

**Трюк: memfd вместо временного файла.** Временный файл в `/tmp` надо
придумать по имени, создать без гонки (`O_EXCL`), удалить и не оставить
при падении. memfd не нужно ни то, ни другое: файл исчезнет, когда
закроется последний дескриптор, — в том числе при SIGKILL. Флаг
`MFD_ALLOW_SEALING` и `fcntl(F_ADD_SEALS)` позволяют ещё и запретить
изменение: получатель такого файла может отобразить его, не боясь, что
отправитель укоротит файл и вызовет SIGBUS (раздел [28.5](05-mmap.md)).

</div>

<div class="box check">

**Проверь себя.**

1. Почему `file_size` принимает `impl AsFd`, а не `&File`? Какие вызовы
   стали бы невозможны?
2. В `copy` поменяли местами строки с `from` и `to`. Изменится ли
   порядок `close` в журнале strace и почему это неважно?
3. Где в цепочке `OwnedFd → File → OwnedFd → Stdio` можно было бы
   ошибиться в C и как бы эта ошибка проявилась?

</div>
