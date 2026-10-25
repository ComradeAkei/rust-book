# 29.5. Закреплённая инициализация

## Структуры, которые нельзя перемещать

Вот мьютекс ядра:

<!-- linux: include/linux/mutex_types.h -->
```c
context_lock_struct(mutex) {
	atomic_long_t		owner;
	raw_spinlock_t		wait_lock;
#ifdef CONFIG_MUTEX_SPIN_ON_OWNER
	struct optimistic_spin_queue osq; /* Spinner MCS lock */
#endif
	struct list_head	wait_list;
#ifdef CONFIG_DEBUG_MUTEXES
	void			*magic;
#endif
#ifdef CONFIG_DEBUG_LOCK_ALLOC
	struct lockdep_map	dep_map;
#endif
};
```

`wait_list` — голова двусвязного списка ожидающих задач. Пустой список
в ядре — голова, которая указывает сама на себя: `next == prev ==
&wait_list`. `magic` в отладочном ядре — указатель на сам мьютекс. Обе
вещи — адреса внутри структуры, и после `mutex_init` структура
привязана к своему адресу: её копия указывает в оригинал.

<div class="box c">

**Как в C.** `ch29_pin_c.c` инициализирует мьютекс в одной структуре и
«перемещает» её `memcpy` в другую — так, как это случайно происходит при
возврате структуры по значению, `realloc` массива или присваивании
`*a = *b`:

```c
{{#include ../../code/ch29/modules/ch29_pin_c.c:copy}}
```

Обычное ядро (тест `pin_plain`):

```text
# insmod /lib/ch29_pin_c.ko
info:   ch29_pin_c: оригинал: wait_list ffffffffa0000048, next ffffffffa0000048
info:   ch29_pin_c: копия:    wait_list ffffffffa0000018, next ffffffffa0000048
info:   ch29_pin_c: копия заблокирована и освобождена, count = 1
```

Список ожидания копии пуст по смыслу, но `next` указывает на голову
оригинала. Блокировка и разблокировка сработали: без соперников
мьютекс берётся одной атомарной операцией над `owner`, и до списка дело
не дошло. Стоило бы второй задаче встать в очередь — она встала бы в
список оригинала. Отладочное ядро замечает копию на первом же
`mutex_lock` (тест `pin_debug`):

```text
warn:   ------------[ cut here ]------------
warn:   DEBUG_LOCKS_WARN_ON(lock->magic != lock)
warn:   WARNING: kernel/locking/mutex.c:593 at __mutex_lock+0x10ed/0x1420, CPU#0: init/1
warn:   Modules linked in: ch29_pin_c(O+) [last unloaded: ch29_pin(O)]
…
warn:    init_module+0x79/0x1000 [ch29_pin_c]
…
info:   ch29_pin_c: копия заблокирована и освобождена, count = 1
```

`magic` копии — адрес оригинала. Компилятор C в этом коде ничего
подозрительного не видит: `memcpy` двух структур одного типа.

</div>

## Инициализатор вместо значения

В Rust любое значение можно переместить: присваивание, возврат из
функции, передача по значению копируют байты на новое место. Для
мьютекса ядра это неприемлемо, поэтому крейт `kernel` не даёт создать
мьютекс значением. `Mutex::new` возвращает не `Mutex<T>`, а
*инициализатор* (pin-init initializer) — `impl PinInit<Mutex<T>>`:
обещание инициализировать мьютекс там, куда укажут, и больше его не трогать.

```rust,ignore
{{#include ../../code/ch29/modules/ch29_pin.rs:journal}}
```

`#[pin_data]` и `#[pin]` объявляют, какие поля структуры закреплены:
`entries` — мьютекс, его нельзя перемещать, а `name` — обычное поле.
`try_pin_init!` собирает инициализатор всей структуры из инициализаторов
полей: `name` — значение, `entries <- …` — инициализатор, который
выполнится на месте поля. `new_mutex!` — `Mutex::new` с именем для
lockdep и статическим ключом класса блокировки: всё, что в C делает
`mutex_init`. `Journal::new` тоже возвращает инициализатор, и так —
вверх по вложенности: закреплённое поле делает закреплённой всю
структуру.

Применяют инициализатор там, где для значения уже есть место:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_pin.rs:init}}
```

`KBox::pin_init` выделяет память и выполняет инициализатор прямо в ней;
результат — `Pin<KBox<Journal>>`. `stack_try_pin_init!` делает то же на
стеке: `local` — `Pin<&mut Journal>` на переменную, которую макрос
объявил и закрепил сам. Сеанс:

```text
# insmod /lib/ch29_pin.ko
info:   ch29_pin: кадры: мьютекс в куче по адресу 0xffff888000cc83d0
info:   ch29_pin: после перемещения Pin<KBox>: 0xffff888000cc83d0
info:   ch29_pin: на стеке: мьютекс по адресу 0xffffc90000013a18, записи [7]
# rmmod ch29_pin
info:   ch29_pin: кадры: записи [1, 2]
```

`let moved = journal;` переместил `Pin<KBox<Journal>>` — восемь байт
указателя, — а журнал остался где был: адрес мьютекса тот же. Мьютекс
в куче — в прямом отображении (это `kmalloc`), на стеке — в области
vmalloc (раздел [29.4](04-memory.md)). Журнал в куче модуль хранит в
поле и печатает при выгрузке.

## Чего сделать нельзя

Получить из инициализатора значение — то, что в C делает `struct
journal j = make_journal();`:

```rust,ignore
{{#include ../../code/ch29/modules-fail/ch29_fail_pin_value.rs:value}}
```

```text
error[E0308]: mismatched types
  --> code/ch29/modules-fail/ch29_fail_pin_value.rs:33:32
   |
23 |     fn new() -> impl PinInit<Self, Error> {
   |                 ------------------------- the found opaque type
...
33 |         let journal: Journal = Journal::new();
   |                      -------   ^^^^^^^^^^^^^^ expected `Journal`, found opaque type
   |                      |
   |                      expected due to this
   |
   = note:   expected struct `Journal`
           found opaque type `impl pin_init::PinInit<Journal, kernel::error::Error>`
```

И вынуть закреплённый журнал из кучи, чтобы переместить, — то, что в C
делает `memcpy`:

```rust,ignore
{{#include ../../code/ch29/modules-fail/ch29_fail_pin_move.rs:move}}
```

```text
error[E0277]: `PhantomPinned` cannot be unpinned
    --> code/ch29/modules-fail/ch29_fail_pin_move.rs:38:52
     |
  38 |         let boxed: KBox<Journal> = Pin::into_inner(journal);
     |                                    --------------- ^^^^^^^ within `lock::_::__Unpin<'_, Vec<u32, Kmalloc>, MutexBackend>`, the trait `Unpin` is not implemented for `PhantomPinned`
     |                                    |
     |                                    required by a bound introduced by this call
     |
…
note: required because it appears within the type `lock::_::__Unpin<'_, Vec<u32, Kmalloc>, MutexBackend>`
    --> target/ch29-kernel/linux-7.0/rust/kernel/sync/lock.rs:105:1
     |
 105 | #[pin_data]
     | ^^^^^^^^^^^
     = note: required for `Lock<Vec<u32, Kmalloc>, MutexBackend>` to implement `Unpin`
…
note: required for `Journal` to implement `Unpin`
```

Цепочка в сообщении — та же, что в типах: `Journal` не `Unpin`, потому
что не `Unpin` его закреплённое поле `Mutex`, а оно — из-за
`PhantomPinned` в `Lock` (раздел [29.1](01-structure.md)). `Pin<P>`
отдаёт указатель обратно (`into_inner`) и даёт `&mut T` (`DerefMut`)
только для `T: Unpin`. Тест `compile_fail_modules` собирает оба модуля
из `modules-fail` и сверяет коды ошибок.

<div class="box hood">

**Под капотом: что такое инициализатор.** `PinInit` — `unsafe`-трейт с
одним обязательным методом:

<!-- linux: rust/pin-init/src/lib.rs -->
```rust,ignore
pub unsafe trait PinInit<T: ?Sized, E = Infallible>: Sized {
    /// Initializes `slot`.
    ///
    /// # Safety
    ///
    /// - `slot` is a valid pointer to uninitialized memory.
    /// - the caller does not touch `slot` when `Err` is returned, they are only permitted to
    ///   deallocate.
    /// - `slot` will not move until it is dropped, i.e. it will be pinned.
    unsafe fn __pinned_init(self, slot: *mut T) -> Result<(), E>;
```

Инициализатор — функция «запиши значение по этому адресу», и тот, кто
её вызывает, обещает три вещи: память не инициализирована, при ошибке
её не читают, после успеха её не перемещают. `KBox::pin_init`,
`stack_try_pin_init!` и `Arc::pin_init` выполняют эти обещания один раз,
внутри крейта, и дальше значение живёт только за `Pin`. `try_pin_init!`
пишет поля прямо в `slot`, по одному, а если поле вернуло ошибку —
вызывает `drop_in_place` для уже записанных: это и есть откат из
раздела [29.3](03-module.md). Никакой копии структуры на стеке нет, и
нет её перемещения в кучу — поэтому pin-init подходит и для структур
в несколько килобайт, которые на стек ядра не поместились бы.

</div>

<div class="box trap">

**Ловушка: `Pin` защищает от перемещения, а не от копирования байтов в
`unsafe`.** `core::ptr::read` из закреплённого значения скомпилируется —
в блоке `unsafe`, с обязанностью доказать, что так можно. Всё, что
сказано выше, — о безопасном коде.

</div>

<div class="box check">

**Проверь себя.**

1. Почему у `Journal` поле `name` — без `#[pin]`? Что изменится, если
   пометить и его?
2. `fn make() -> Journal` не написать. А `fn make() -> Pin<KBox<Journal>>`?
3. Почему в отчёте `pin_debug` мьютекс копии не прошёл проверку, а у
   мьютекса на стеке модуля `ch29_pin` ни одного предупреждения нет?

</div>
