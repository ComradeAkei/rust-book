# 29.7. Символьное устройство и граница с пространством пользователя

<div class="box ref">

**Справка: граница пространств.** Указатель, который пользователь
передал в `read(fd, buf, n)`, — адрес в *его* адресном пространстве.
Ядро не может просто разыменовать его, по трём причинам. Адрес может
быть каким угодно — не отображённым, указывающим в ядро (`write(fd,
0xffffffff81000000, 16)` — попытка прочитать код ядра чужими руками).
Страница может быть выгружена и прийти с диска только после страничного
сбоя — а это сон. И процессор сам не пускает: с SMAP (Supervisor Mode
Access Prevention, x86) обращение ядра к странице пользователя —
исключение, если перед ним не выполнена инструкция `stac`. Поэтому все
обращения идут через `copy_to_user` и `copy_from_user`: они проверяют,
что адрес — пользователя (`access_ok`), открывают доступ, копируют и
перехватывают сбой. Возвращают они число *не*скопированных байтов, а
код ошибки — `EFAULT` — драйвер должен сделать из этого числа сам.

</div>

## misc-устройство

Символьное устройство — файл в `/dev`, у которого вместо данных на
диске — функции драйвера: `open`, `read`, `write`, `ioctl`, `release`.
misc-устройство — самый простой их вид: один старший номер на всех,
младший выдаётся при регистрации, узел в `/dev` создаёт ядро.
`ch29_notes` регистрирует `/dev/ch29-notes`, и у каждого открытия свой
буфер заметок: `write` дописывает в него, `read` читает с позиции файла,
`ioctl` сообщает длину и очищает буфер.

В C операции — таблица указателей на функции `struct file_operations`,
а состояние открытого файла — `void *private_data`. В Rust — трейт
`MiscDevice`, и состояние — тип:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_notes.rs:open}}
```

`type Ptr` — что абстракция положит в `file->private_data`: здесь
закреплённый `KBox<Notes>`. `open` возвращает его, и крейт `kernel`
превращает его в указатель (`into_foreign`); в `read_iter`, `write_iter`
и `ioctl` модуль получает обратно `&Notes`, а в `release` абстракция
забирает владение (`from_foreign`) и роняет `KBox` — отсюда сообщение
`PinnedDrop` «закрыт, в буфере … байт». Ни одного приведения `void *` в
модуле нет.

Таблицу `file_operations` для `Notes` строит крейт `kernel`, а
`#[vtable]` на `impl` сообщает ему, какие методы реализованы:

<!-- linux: rust/kernel/miscdevice.rs -->
```rust,ignore
    const VTABLE: bindings::file_operations = bindings::file_operations {
        open: Some(Self::open),
        release: Some(Self::release),
        mmap: if T::HAS_MMAP { Some(Self::mmap) } else { None },
        read_iter: if T::HAS_READ_ITER {
            Some(Self::read_iter)
        } else {
            None
        },
…
        ..pin_init::zeroed()
    };
```

`HAS_READ_ITER` и остальные константы генерирует `#[vtable]`: `true` для
методов, которые есть в `impl`. Таблица — константа, вычисленная при
компиляции, и для нереализованных методов в ней `NULL` — как у
драйвера на C, где неназванные поля инициализатора равны нулю. Каждый
`Some(Self::read_iter)` — функция `extern "C"`, которая переводит
указатели C в типы Rust, вызывает метод модуля и переводит `Result`
обратно в число: `Err(e)` — `e.to_errno()`.

## read и write

```rust,ignore
{{#include ../../code/ch29/modules/ch29_notes.rs:read}}
```

Абстракция misc-устройства в 7.0 даёт не `read(file, buf, count, ppos)`
с указателем пользователя, а `read_iter`: `Kiocb` — запрос с позицией
файла, `IovIterDest` — куда копировать, один буфер или вектор буферов
`readv`. Указателя пользователя модуль не видит вовсе:
`copy_to_iter(&[u8])` копирует через границу и возвращает, сколько
скопировал. Ноль при непустых источнике и приёмнике значит, что адрес
пользователя недоступен, и модуль возвращает `EFAULT`.

```rust,ignore
{{#include ../../code/ch29/modules/ch29_notes.rs:write}}
```

`copy_from_iter_vec` дописывает данные пользователя в `KVVec` и сам
выделяет место — с флагами, которые ему передали. Буфер — под
мьютексом: копирование из пространства пользователя может спать
(страничный сбой), и спин-блокировка здесь была бы ошибкой из раздела
[29.4](04-memory.md).

## ioctl

```rust,ignore
{{#include ../../code/ch29/modules/ch29_notes.rs:ioctl}}
```

```rust,ignore
{{#include ../../code/ch29/modules/ch29_notes.rs:ioctl_fn}}
```

Номера команд `_IOR` и `_IO` вычисляются при компиляции — `const fn`
вместо макросов C — и кодируют направление, размер аргумента и
«магическое» число устройства, как одноимённые макросы. Аргумент
`ioctl` — `usize`, который для `NOTES_LEN` — адрес пользователя:
`UserSlice::new(UserPtr::from_addr(arg), 8).writer()` — восемь байт у
пользователя, куда можно только писать, и `write(&len)` копирует туда
`u64` (типы, которые можно копировать байтами, отмечены трейтом
`AsBytes`). Неизвестная команда — `ENOTTY`, как требует соглашение.

## Сеанс: Rust и C

Init открывает устройство дважды и проверяет всё, включая плохие
адреса:

```rust,ignore
{{#include ../../code/ch29/src/bin/ch29-guest.rs:notes}}
```

Вывод для `ch29_notes` (тест `notes_plain`):

```text
# insmod /lib/ch29_notes.ko
write(a, "первая\n"): Ok(13)
write(a, "вторая\n"): Ok(13)
read(a): "первая\nвторая\n"
read(b): "" — у b свой буфер
ioctl(a, NOTES_LEN): Ok(0), длина 26
pread(a, 0x1000): -1 (Bad address (os error 14))
write(a, 0x1000): -1 (Bad address (os error 14))
pread(a, ядро): -1 (Bad address (os error 14))
write(a, ядро): -1 (Bad address (os error 14))
write(b, 5000 байт): Err("No space left on device (os error 28)")
ioctl(a, 0x6e63): -1 (Inappropriate ioctl for device (os error 25))
NOTES_CLEAR: Ok(0), потом NOTES_LEN: Ok(0), длина 0
info:   ch29_notes: закрыт, в буфере 0 байт
info:   ch29_notes: закрыт, в буфере 0 байт
# rmmod ch29_notes
```

«первая\n» — 13 байт: кириллица в UTF-8 по два байта на букву. Адрес
`0x1000` — неотображённая страница, `0xffffffff81000000` — код ядра:
`EFAULT` в обе стороны. `read` с позиции файла: init читает до конца,
и второй `read` вернул 0. В `/dev/ch29-notes` нельзя сделать `lseek` — у
misc-устройства в 7.0 нет метода `llseek`, и ядро снимает с такого файла
`FMODE_LSEEK` (`lseek` вернёт `ESPIPE`), — зато `pread` с явной позицией
работает.

<div class="box c">

**Как в C.** `ch29_notes_c.c` — то же устройство:

```c
{{#include ../../code/ch29/modules/ch29_notes_c.c:fops}}
```

Сеанс `/dev/ch29-notes-c` — строка в строку тот же: тест сверяет оба
устройства одним списком ожиданий. Различие — в том, что держится на
внимании программиста. `private_data` — `void *`, и каждая функция
приводит его к `struct notes *` сама. `mutex_unlock` стоит перед каждым
выходом из `notes_write`, и для этого понадобился `goto out`.
`copy_from_user` возвращает число, и превратить его в `-EFAULT` надо не
забыть. `release` обязан освободить то, что выделил `open`, и вызвать
`mutex_destroy`. Задача [29.C](10-tasks.md) — такое же сравнение на
устройстве-счётчике, с подсчётом строк и путей выхода по ошибке.

</div>

## Ошибка в абстракции: simple_read_from_buffer

Чтение «всего файла из буфера» — частый случай, и для него в C есть
`simple_read_from_buffer`, а в крейте `kernel` —
`IovIterDest::simple_read_from_buffer`. С параметром `simple=1` модуль
читает через неё (первые строки `read_iter`). Сеанс тот же, кроме одной
строки:

```text
# insmod /lib/ch29_notes.ko simple=1
…
pread(a, 0x1000): 0
write(a, 0x1000): -1 (Bad address (os error 14))
…
```

`pread` по плохому адресу вернул не `EFAULT`, а 0 — конец файла.
Программа, которая читает в цикле до нуля, решит, что файл пуст. Причина
видна в исходнике абстракции:

<!-- linux: rust/kernel/iov.rs -->
```rust,ignore
    pub fn simple_read_from_buffer(&mut self, ppos: &mut i64, contents: &[u8]) -> Result<usize> {
        if *ppos < 0 {
            return Err(EINVAL);
        }
        let Ok(pos) = usize::try_from(*ppos) else {
            return Ok(0);
        };
        if pos >= contents.len() {
            return Ok(0);
        }

        // BOUNDS: We just checked that `pos < contents.len()` above.
        let num_written = self.copy_to_iter(&contents[pos..]);

        // OVERFLOW: `pos+num_written <= contents.len() <= isize::MAX <= i64::MAX`.
        *ppos = (pos + num_written) as i64;

        Ok(num_written)
    }
```

`copy_to_iter` вернул 0 — ни одного байта, — и функция вернула `Ok(0)`.
В C ровно на этом месте проверка:

<!-- linux: fs/libfs.c -->
```c
	ret = copy_to_user(to, from + pos, count);
	if (ret == count)
		return -EFAULT;
```

<div class="box trap">

**Ловушка: безопасный код — не значит правильный.** В
`simple_read_from_buffer` нет ни `unsafe`, ни нарушения памяти: ошибка
— в логике перевода протокола C («скопировано 0 из n — это `EFAULT`»).
Типы этого не проверяют. Абстракции крейта `kernel` — такой же код, как
любой другой; когда поведение важно, проверяйте его опытом, как здесь.
Модуль главы по умолчанию проверяет ноль сам.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `Notes::buf` — `Mutex`, а не `SpinLock`?
2. Что вернёт `read` на `/dev/ch29-notes`, если пользователь передаст
   буфер, у которого первая страница отображена, а вторая — нет, и
   заметок больше страницы?
3. Почему для `NOTES_LEN` нужен `UserSlice`, а не `*(arg as *mut u64) =
   len` в `unsafe`?

</div>
