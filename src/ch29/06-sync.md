# 29.6. Блокировки, атомики и счётчики ссылок

## Четыре работы и общий счётчик

Опыт: четыре работы в очереди ядра (workqueue) по сто тысяч раз
увеличивают общий счётчик. Работа — функция, которую поток ядра
выполнит позже, на каком-нибудь процессоре; несколько работ могут идти
одновременно на разных процессорах — это обычная многопоточность
ядра. Состояние, общее для работ:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_sync.rs:shared}}
```

Два счётчика — под спин-блокировкой и атомарный, — счётчик завершённых
работ и `Completion`, на котором `init` модуля ждёт, пока работы
кончатся. Сами работы:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_sync.rs:run}}
```

`Arc::pin_init` выделяет память под `Shared` и инициализирует её на
месте: в структуре спин-блокировка и `Completion`, их нельзя
перемещать (раздел [29.5](05-pin-init.md)). Каждая работа получает свой
клон `Arc` — [замыкание][g-closure] `move` забирает его, — и `try_spawn` кладёт
замыкание в очередь `system_unbound`, работы которой не привязаны к
процессору. Сеанс (тест `sync_plain`):

```text
# insmod /lib/ch29_sync.ko
info:   ch29_sync: работа 1: CPU 0
info:   ch29_sync: работа 0: CPU 1
info:   ch29_sync: работа 3: CPU 1
info:   ch29_sync: работа 2: CPU 1
info:   ch29_sync: под SpinLock: 400000, Atomic: 400000, ожидалось 400000
```

Оба счётчика сошлись, и работы шли на обоих процессорах.

## То же на C — и третий счётчик

```c
{{#include ../../code/ch29/modules/ch29_sync_c.c:racy}}
```

Здесь работы раскладываются по процессорам явно, `queue_work_on`, — и
у них есть третий счётчик, без синхронизации. `READ_ONCE` и `WRITE_ONCE`
не делают его безопасным: они только запрещают компилятору склеить,
выкинуть или разорвать обращения. Чтение и запись — два отдельных
обращения, и между ними другой процессор успевает записать своё:

```text
# insmod /lib/ch29_sync_c.ko
info:   ch29_sync_c: работа 0: CPU 0
info:   ch29_sync_c: работа 1: CPU 1
info:   ch29_sync_c: работа 2: CPU 0
info:   ch29_sync_c: работа 3: CPU 1
info:   ch29_sync_c: без синхронизации: 384806, под spinlock: 400000, atomic64: 400000, ожидалось 400000
```

Потеряно 15 194 увеличения. Число меняется от запуска к запуску —
тест проверяет только, что оно не больше 400 000.

## Почему такого счётчика нет в Rust

Его нельзя написать. Счётчик — поле общего состояния, а `Arc` даёт
работам только общую ссылку `&Shared`. Записать через неё — ошибка:

```rust,ignore
{{#include ../../code/ch29/modules-fail/ch29_fail_racy_plain.rs:plain}}
```

```text
error[E0594]: cannot assign to data in dereference of `Arc<Shared>`
  --> code/ch29/modules-fail/ch29_fail_racy_plain.rs:31:21
   |
31 |                     s.plain += 1;
   |                     ^^^^^^^^^^^^ cannot assign
   |
   = help: trait `DerefMut` is required to modify through a dereference, but it is not implemented for `Arc<Shared>`
```

Обойти через [внутреннюю изменяемость][g-interior-mut] — `Cell`, который разрешает запись
по общей ссылке, — тоже нельзя:

```rust,ignore
{{#include ../../code/ch29/modules-fail/ch29_fail_racy_cell.rs:cell}}
```

```text
error[E0277]: `Cell<u64>` cannot be shared between threads safely
   --> code/ch29/modules-fail/ch29_fail_racy_cell.rs:26:63
…
    = help: the trait `Sync` is not implemented for `Cell<u64>`
    = note: if you want to do aliasing and mutation between multiple threads, use `std::sync::RwLock` or `std::sync::atomic::AtomicU64` instead
    = note: required for `Arc<Cell<u64>>` to implement `Send`
note: required because it's used within this closure
…
note: required by a bound in `Queue::try_spawn`
   --> target/ch29-kernel/linux-7.0/rust/kernel/workqueue.rs:334:35
    |
334 |     pub fn try_spawn<T: 'static + Send + FnOnce()>(
    |                                   ^^^^ required by this bound in `Queue::try_spawn`
```

`try_spawn` требует от замыкания `Send`: работа выполнится в другом
потоке ядра. Замыкание держит `Arc<Cell<u64>>`, а `Arc<T>` — `Send`
только для `T: Sync + Send`. Это те же правила, что у `std::thread::spawn`
(глава [20](../ch20/index.md)): `Send` и `Sync` — обычные трейты, крейт
`kernel` расставил их на своих типах, и компилятор проверяет их для
работ в очереди ядра, обработчиков прерываний (раздел
[29.8](08-hardware.md)) и самого модуля (`Module: Sync + Send`, раздел
[29.3](03-module.md)) так же, как для потоков std.

<div class="box trap">

**Ловушка: подсказки rustc — из std.** В сообщении E0277 компилятор
советует `std::sync::RwLock` и `std::sync::atomic::AtomicU64`. В ядре
std нет; советы читаются как «блокировка или [атомик][g-atomic]», а типы — из
крейта `kernel`: `kernel::sync::{Mutex, SpinLock}` и
`kernel::sync::atomic::Atomic<u64>`.

</div>

## Мьютекс и спин-блокировка

Оба — `Lock<T, B>` из раздела [29.1](01-structure.md): данные внутри,
`lock()` возвращает охранника, через которого к ним и обращаются, а
`Drop` охранника отпускает блокировку. Забыть `unlock` на пути ошибки
нельзя — его нет; вернуть из функции ссылку на данные, пережившую
охранника, не даст [проверка заимствований][g-borrow-checker].

Разница между ними — в том, что делает ждущий:

| | `Mutex<T>` | `SpinLock<T>` |
|---|---|---|
| Под капотом | `struct mutex` | `spinlock_t` |
| Ждущий | засыпает (иногда сначала недолго крутится) | крутится в цикле на своём процессоре |
| Держатель | может спать | в атомарном контексте: спать нельзя |
| Где брать | только в контексте процесса | и в атомарном контексте |

Данные, общие с обработчиком прерывания, в C защищают
`spin_lock_irqsave` — спин-блокировкой с запретом прерываний на своём
процессоре, иначе прерывание посреди критической секции будет вечно
ждать блокировку, которую держит прерванный код. В крейте `kernel` 7.0
такого вида блокировки нет (в `rust/kernel/sync/lock/` — только
`mutex.rs`, `spinlock.rs` и `global.rs`), и обработчик из раздела
[29.8](08-hardware.md) обходится атомиками.

Здесь работы держат блокировку на одну операцию, и спин-блокировка
дешевле: ждать придётся наносекунды, засыпать дороже. Мьютекс — когда
под блокировкой можно спать: выделять с `GFP_KERNEL`, копировать в
пространство пользователя, ждать устройство (раздел
[29.7](07-chardev.md)). Чего тип не проверяет — сна под спин-блокировкой
(раздел [29.4](04-memory.md)) и порядка взятия двух блокировок: обе
ошибки остаются отладочному ядру и lockdep (задача [29.A](10-tasks.md)).

## Атомики ядра

`Atomic<u64>` — не `core::sync::atomic::AtomicU64`, а обёртка над
`atomic64_t` ядра, и порядок доступа у неё — не [`Ordering::SeqCst`][g-memory-ordering] и
компания, а модель памяти ядра LKMM (`tools/memory-model/`). Порядки —
типы-аргументы: `Relaxed`, `Acquire`, `Release` и `Full`:

<!-- linux: rust/kernel/sync/atomic/ordering.rs -->
```rust,ignore
//! - [`Full`] means "fully-ordered", that is:
//!   - It provides ordering between all the preceding memory accesses and the annotated operation.
//!   - It provides ordering between the annotated operation and all the following memory accesses.
//!   - It provides ordering between all the preceding memory accesses and all the following memory
//!     accesses.
//!   - All the orderings are the same strength as a full memory barrier (i.e. `smp_mb()`).
```

`add(1, Relaxed)` — то же, что `atomic64_add` в C: атомарно, но без
упорядочения с другими обращениями. Для счётчика этого достаточно:
итог прочитает `init` после `wait_for_completion`, а завершение работы
и пробуждение ждущего уже упорядочивают всё, что было до них.
`fetch_add(1, Full)` на счётчике завершённых работ — как
`atomic_inc_return`: полный [барьер][g-memory-barrier] с обеих сторон. Почему атомики ядра
не взяты из `core`: код на Rust и на C работает с одними и теми же
переменными, и правила для них должны быть одни — правила ядра, со
всеми его барьерами и зависимостями адресов.

## Счётчики ссылок: Arc и ARef

`kernel::sync::Arc` похож на `std::sync::Arc` и отличается в том, что
важно ядру:

<!-- linux: rust/kernel/sync/arc.rs -->
```rust,ignore
//! It is different from the standard library's [`Arc`] in a few ways:
//! 1. It is backed by the kernel's [`Refcount`] type.
//! 2. It does not support weak references, which allows it to be half the size.
//! 3. It saturates the reference count instead of aborting when it goes over a threshold.
//! 4. It does not provide a `get_mut` method, so the ref counted object is pinned.
//! 5. The object in [`Arc`] is pinned implicitly.
```

`Refcount` — `refcount_t` ядра: при переполнении счётчик «насыщается»
и объект навсегда остаётся в памяти (утечка вместо use-after-free), а
ядро печатает предупреждение. Слабых ссылок нет, и счётчик один.
Объект в `Arc` закреплён: `Arc::pin_init` инициализирует его на месте, и
`Shared` с блокировкой внутри живёт в `Arc` спокойно.

`Arc` [владеет][g-ownership] объектом, который создал Rust. У объектов, которые создал
C и считает сам, — устройств, файлов, задач — свой счётчик внутри
структуры C (`kref`, `get_device`/`put_device`). Для них —
`ARef<T>`: [умный указатель][g-smart-pointer], который в `Clone` зовёт функцию «взять
ссылку» этого типа, а в `Drop` — «отпустить». Драйвер из раздела
[29.8](08-hardware.md) держит так PCI-устройство: `ARef<pci::Device>`.
Ручной `put_device` на каждом пути выхода — то, что в C забывают, —
превращается в `Drop`.

<div class="box check">

**Проверь себя.**

1. Замените в `ch29_sync` `SpinLock` на `Mutex`. Соберётся ли модуль?
   Изменится ли результат?
2. Почему `atomic.add(1, Relaxed)` достаточно, если четыре работы
   выполняются одновременно? Что гарантирует, что `init` прочитает
   400000, а не меньше?
3. В C счётчик без синхронизации собрался. Какой инструмент ядра нашёл
   бы гонку? (Подсказка: `CONFIG_KCSAN`.)

</div>

<!-- Ссылки на глоссарий (приложение F); обновляет tools/check-terms.py --refs -->
[g-atomic]: ../appendix-f.md#atomic "Атомик (atomic): Тип из core::sync::atomic (AtomicU32, AtomicBool и другие), операции над которым неделимы и упорядочены по выбранной модели памяти."
[g-borrow-checker]: ../appendix-f.md#borrow-checker "Проверка заимствований (borrow checker): Часть компилятора, которая доказывает, что каждое заимствование не переживает владельца и не пересекается с конфликтующим."
[g-closure]: ../appendix-f.md#closure "Замыкание (closure): Функция вместе со своим контекстом — захваченными переменными."
[g-interior-mut]: ../appendix-f.md#interior-mut "Внутренняя изменяемость (interior mutability): Изменение значения через разделяемую ссылку &."
[g-memory-barrier]: ../appendix-f.md#memory-barrier "Барьер памяти (memory barrier, fence): Инструкция или указание компилятору, которые запрещают переставлять обращения к памяти через себя."
[g-memory-ordering]: ../appendix-f.md#memory-ordering "Порядок памяти (memory ordering): Аргумент Ordering атомарной операции: какие из соседних обращений к памяти другие потоки обязаны увидеть до неё или после."
[g-ownership]: ../appendix-f.md#ownership "Владение (ownership): Правило Rust: у каждого значения ровно один владелец — переменная, поле или элемент коллекции, — и он отвечает за уничтожение значения."
[g-smart-pointer]: ../appendix-f.md#smart-pointer "Умный указатель (smart pointer): Тип, который ведёт себя как указатель (реализует Deref) и сам управляет тем, на что указывает: Box владеет значением в куче, Rc и Arc считают ссылки, MutexGuard держит мьютекс."
