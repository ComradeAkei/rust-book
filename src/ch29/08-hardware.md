# 29.8. PCI-устройство: MMIO и прерывание

<div class="box ref">

**Справка: PCI, BAR, MMIO, MSI.** У каждого устройства PCI есть
*конфигурационное пространство* — 256 байт (у PCIe — 4 КиБ) с
идентификаторами производителя и устройства, командным регистром и
регистрами BAR (Base Address Register). BAR описывает окно,
через которое процессор говорит с устройством: его размер задаёт
устройство, а адрес назначает прошивка или ядро. Окно в физическом
адресном пространстве — это MMIO (memory-mapped I/O): чтение и запись
по этим адресам уходят не в память, а в регистры устройства. Ядро
отображает окно в свои виртуальные адреса без кеширования (`ioremap`),
и обращаться к нему надо инструкциями ровно того размера, который
ждёт регистр, без переупорядочивания и склейки — отсюда `ioread32` и
`iowrite32` вместо разыменования. Прерывания у PCI бывают двух видов.
INTx — линии, общие для нескольких устройств: обработчик каждого
спрашивает своё устройство, «не ты ли». MSI (Message Signaled
Interrupts) — запись: устройство пишет заранее заданное число по
заранее заданному адресу (на x86 — в окно локального APIC), и у
каждого вектора MSI свой обработчик, ни с кем не общий. Запись в память
— работа ведущего на шине, и без разрешённого bus mastering устройство
MSI не пошлёт.

</div>

## Устройство edu

У QEMU есть учебное устройство `edu` (`-device edu`), сделанное для
курсов по драйверам: настоящее PCI-устройство с BAR0 на 1 МиБ,
несколькими регистрами и прерыванием. Описание — `docs/specs/edu.rst` в
исходниках QEMU. Регистры, которые нужны драйверу главы:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_edu.rs:regs}}
```

`pci::Bar<SIZE>` — отображённый BAR, о котором при компиляции известно,
что в нём не меньше `SIZE` байт: от этого числа зависят проверки границ
в конце раздела. Само окно больше — 1 МиБ, — но драйвер обещает
трогать только первые 0x100 байт.

Драйвер находит устройство по таблице идентификаторов, и в ней —
единственный `unsafe` модуля:

```rust,ignore
{{#include ../../code/ch29/modules/ch29_edu.rs:vendor}}
```

`pci::Vendor` в 7.0 — тип с константами для каждого производителя из
`include/linux/pci_ids.h`, и сделать его из произвольного числа нельзя:
конструктор закрыт. 0x1234 — номер, которым QEMU помечает свои учебные
устройства; в таблице ядра его нет. Таблица драйвера попадает в модуль
как псевдоним, по которому udev и `modprobe` находят драйвер для
нового устройства:

```console
$ readelf -p .modinfo ch29_edu.ko

String dump of section '.modinfo':
  [     0]  author=Rust textbook
  [    15]  description=Driver for the QEMU edu PCI device
  [    44]  license=GPL
  [    50]  name=ch29_edu
  [    5e]  depends=
  [    67]  alias=pci:v00001234d000011E8sv*sd*bc*sc*i*
  [    92]  vermagic=7.0.0 SMP preempt mod_unload
```

`v00001234d000011E8` — производитель и устройство, звёздочки — любые
подсистема, класс и интерфейс.

## probe: что можно делать с устройством и когда

Драйвер — тип с трейтом `pci::Driver`. Ядро вызывает `probe`, когда
находит устройство из таблицы, и `unbind`, когда устройство уходит:
выгрузка модуля, горячее извлечение, запись в `unbind` в sysfs.

```rust,ignore
{{#include ../../code/ch29/modules/ch29_edu.rs:probe}}
```

Аргумент `probe` — не просто `&pci::Device`, а `&pci::Device<Core>`.
Параметр типа — *контекст* устройства, и от него зависит, какие методы
доступны:

<!-- linux: rust/kernel/device.rs -->
```rust,ignore
/// - [`CoreInternal`] => [`Core`] => [`Bound`] => [`Normal`]
…
/// The core context indicates that the [`Device<Core>`] reference's scope is limited to the bus
/// callback it appears in. It is intended to be used for synchronization purposes. Bus device
…
/// Some APIs, such as [`dma::CoherentAllocation`] or [`Devres`] rely on the [`Device`] to be bound,
/// which can be proven with the [`Bound`] device context.
```

`Device<Core>` бывает только внутри обратного вызова шины — `probe` и
`unbind`, — когда ядро держит блокировку устройства. Отсюда
`enable_device_mem` и `set_master`: они определены только для
`Device<Core>`. `Device<Bound>` значит «драйвер привязан»: отобразить
BAR, выделить векторы прерываний, запросить прерывание —
`iomap_region_sized`, `alloc_irq_vectors`, `request_irq` определены для
него. `Core` разыменовывается в `Bound`, а тот — в `Normal`, обычное
устройство без гарантий. Сохранить ссылку `&Device<Core>` нельзя — её
время жизни кончается с `probe`; в структуре драйвера лежит
`ARef<pci::Device>` — ссылка со счётчиком на `struct pci_dev`
(`pci_dev_get` при создании, `pci_dev_put` в `Drop`) — и это
`Device<Normal>`: методов `probe` через неё не вызвать.

Шаги `probe` по порядку:

1. `enable_device_mem` включает декодирование MMIO в командном регистре
   — и ядро заодно назначает устройству линию INTx: отсюда строка ACPI
   в журнале сеанса ниже;
2. `set_master` разрешает устройству быть ведущим на шине — без этого
   не будет MSI;
3. `iomap_region_sized` занимает регион BAR (`pci_request_region`, он
   появится в `/proc/iomem`), отображает его и проверяет, что он не
   меньше 0x100 байт; результат — `Devres<Bar0>` в `Arc`, о нём ниже;
4. проверка живости — запись `0x12345678` в `LIVENESS` и чтение
   инверсии;
5. один вектор MSI и обработчик на нём; `irq <-` — инициализатор поля,
   как в разделе [29.5](05-pin-init.md): `Registration` закреплена,
   потому что ядро хранит её адрес как `dev_id`;
6. блок `_: { … }` выполняется после инициализации поля `irq` и видит
   его по имени: драйвер запускает факториал с прерыванием по
   окончании, ждёт прерывания, а потом просит устройство прислать ещё
   одно — записью в `IRQ_RAISE`.

Любой `?` по дороге — и уже полученное откатывается: BAR и прерывание
роняет `Drop`, как в разделе [29.3](03-module.md), а векторы MSI
освобождает devres ядра при неудачном `probe`. На C откат в `probe` —
та же лестница `goto err_*`, что в `init` модуля. Одно исключение —
`enable_device_mem`: у `pci::Device` в 7.0 нет ни `disable_device`, ни
управляемого варианта (в C — `pcim_enable_device`), и включённое
устройство остаётся включённым — после ошибки и после `rmmod`. Ядро на
это не жалуется: комментарий в `pci_device_remove` объясняет, что
выключение устройства не любят слишком многие BIOS и мосты PCI.

## Devres: ресурс, который отзывают

Отображённый BAR живёт, пока устройство привязано к драйверу. А ссылки
на него могут жить дольше: в работе в очереди, в открытом файле, в
таймере. На C для этого есть `devm_*` и `pcim_*` — ресурсы, которые
ядро освобождает само при отвязке, — но указатель, сохранённый в
другом месте, остаётся и указывает на снятое отображение. `Devres<T>`
в Rust решает это по-другому:

<!-- linux: rust/kernel/devres.rs -->
```rust,ignore
/// To achieve that [`Devres`] registers a devres callback on creation, which is called once the
/// [`Device`] is unbound, revoking access to the encapsulated resource (see also [`Revocable`]).
///
/// After the [`Devres`] has been unbound it is not possible to access the encapsulated resource
/// anymore.
```

Получить `&Bar0` из `Devres<Bar0>` можно двумя способами.
`try_access()` — откуда угодно: под RCU проверяет, не отозван ли
ресурс, и возвращает `Option` — после отвязки `None`. `access(dev)` —
без RCU и без проверки отзыва, но требует `&Device<Bound>`, и того же
устройства (иначе `EINVAL`): существование такой ссылки и есть
доказательство, что отзыва не было.
Драйвер главы зовёт `access` в `probe`, где `Device<Bound>` есть, и в
обработчике прерывания — куда его передаёт абстракция.

<div class="box c">

**Как в C.** Обработчик прерывания в C — функция с `void *`:

<!-- linux: include/linux/interrupt.h -->
```c
typedef irqreturn_t (*irq_handler_t)(int, void *);
…
request_irq(unsigned int irq, irq_handler_t handler, unsigned long flags,
	    const char *name, void *dev)
```

Последний аргумент `request_irq` ядро передаст обработчику, и тот
приведёт его к своей структуре; что эта структура жива, пока
обработчик зарегистрирован, следит программист. Регистры — `ioread32(base
+ off)`, и смещение за концом BAR компилятор пропустит: чтение уйдёт за
отображение — в чужое окно MMIO или в неотображённую страницу, и тогда
oops.

</div>

## Обработчик прерывания

```rust,ignore
{{#include ../../code/ch29/modules/ch29_edu.rs:handler}}
```

Трейт обработчика требует `Sync`:

<!-- linux: rust/kernel/irq/request.rs -->
```rust,ignore
pub trait Handler: Sync {
    /// The hard IRQ handler.
    ///
    /// This is executed in interrupt context, hence all corresponding
    /// limitations do apply.
…
    fn handle(&self, device: &Device<Bound>) -> IrqReturn;
}
```

Обработчик получает `&self` и может выполняться на любом процессоре
одновременно с `probe` и `unbind` — поэтому `Sync`. Счётчик в `Cell`
(`modules-fail/ch29_fail_irq_cell.rs`) не соберётся:

```rust,ignore
{{#include ../../code/ch29/modules-fail/ch29_fail_irq_cell.rs:cell}}
```

```text
error[E0277]: `Cell<u32>` cannot be shared between threads safely
  --> code/ch29/modules-fail/ch29_fail_irq_cell.rs:27:23
   |
27 | impl irq::Handler for CountingIrq {
   |                       ^^^^^^^^^^^ `Cell<u32>` cannot be shared between threads safely
…
note: required by a bound in `Handler`
  --> target/ch29-kernel/linux-7.0/rust/kernel/irq/request.rs:30:20
   |
30 | pub trait Handler: Sync {
   |                    ^^^^ required by this bound in `Handler`
```

Та же ошибка, что у работы в очереди в разделе [29.6](06-sync.md), — и
по той же причине. Аргумент `&Device<Bound>` абстракция даёт без
проверки: прерывание снимают раньше, чем устройство отвязывают. А
вот «контекст прерывания» из комментария типы не проверяют: `fsleep`
или `Mutex::lock` в `handle` соберутся, как `GFP_KERNEL` под
спин-блокировкой в разделе [29.4](04-memory.md).

Обработчик читает `IRQ_STATUS` — причины прерывания, битовую маску, —
подтверждает их записью в `IRQ_ACK` и копит в `causes`. Ноль причин —
прерывание не от этого устройства, `IrqReturn::None`; вектор MSI ни с
кем не общий, и этого не ждут, но тот же код работал бы и на общей
линии INTx. У атомиков крейта `kernel` в 7.0 нет `fetch_or` — только
`load`, `store`, `xchg`, `cmpxchg`, `add` и `fetch_add`, — и `causes`
обновляется чтением и записью. Гонки нет: ядро не запускает обработчик одного прерывания на
двух процессорах сразу, а `probe` читает `causes` только после того,
как дождался счётчика.

Почему у обработчика свой `Arc<Devres<Bar0>>`, а не ссылка на поле
`bar` структуры `Edu`? Обработчик лежит внутри `Registration`, а та — в
поле `irq` той же `Edu`, и ссылка на соседнее поле сделала бы структуру
самоссылающейся: времени жизни «пока жива сама структура» в Rust не
записать. Да и тип обработчика обязан быть `'static`:

<!-- linux: rust/kernel/irq/request.rs -->
```rust,ignore
pub struct Registration<T: Handler + 'static> {
    #[pin]
    inner: Devres<RegistrationInner>,

    #[pin]
    handler: T,
…
}
```

Общий `Arc` — два владельца одного BAR. Снятие обработчика —
`free_irq` в `Drop` внутренней части `Registration`, `RegistrationInner`:

<!-- linux: rust/kernel/irq/request.rs -->
```rust,ignore
        // Notice that this will block until all handlers finish executing,
        // i.e.: at no point will &self be invalid while the handler is running.
        unsafe { bindings::free_irq(self.irq, self.cookie) };
```

`inner` объявлено раньше `handler`, а поля роняются в порядке
объявления: `Drop` `Devres` отзывает и роняет `RegistrationInner` —
`free_irq`, — и только потом роняется обработчик с его клоном `Arc`.
Обработчик и BAR в нём живы, пока `free_irq` не вернулся.

## Сеанс

QEMU запускается с `-device edu`, а init гостя делает:

```rust,ignore
{{#include ../../code/ch29/src/bin/ch29-guest.rs:edu}}
```

Вывод (тест `edu_plain`):

```text
# cat /sys/bus/pci/devices/0000:00:03.0/vendor
0x1234
# cat /sys/bus/pci/devices/0000:00:03.0/device
0x11e8
# cat /sys/bus/pci/devices/0000:00:03.0/enable
0
# insmod /lib/ch29_edu.ko
info:   ACPI: \_SB_.LNKC: Enabled at IRQ 11
info:   ch29_edu 0000:00:03.0: edu 1.0, проверка: !0x12345678 = 0xedcba987
info:   ch29_edu 0000:00:03.0: 10! = 3628800 (прерываний: 1, занято: 0)
info:   ch29_edu 0000:00:03.0: IRQ_RAISE: прерываний 2, причины 0x101
# grep ch29_edu /proc/interrupts
 24:          0          2 PCI-MSI-0000:00:03.0   0-edge      ch29_edu
# grep ch29_edu /proc/iomem
    fea00000-feafffff : ch29_edu
# rmmod ch29_edu
info:   ch29_edu 0000:00:03.0: отключение: всего прерываний 2
# grep ch29_edu /proc/iomem
# cat /sys/bus/pci/devices/0000:00:03.0/enable
1
```

По строкам:

- QEMU посадил устройство в слот `00:03.0`; ядро увидело его при
  загрузке, но драйвера не было, и `vendor`/`device` в sysfs — всё, что
  о нём известно;
- строка ACPI — `enable_device_mem`: ядро назначило линию INTx 11.
  Драйвер ею не пользуется — дальше он берёт MSI;
- `edu 1.0` — версия из регистра `ID`, и устройство вернуло инверсию:
  живо;
- 10! = 3 628 800 — после ровно одного прерывания, и бит «идёт
  вычисление» уже снят;
- причины 0x101: бит 0x1 — факториал, 0x100 — то, что драйвер записал в
  `IRQ_RAISE`;
- в `/proc/interrupts` — номер прерывания в ядре (24), счётчики по
  процессорам (оба прерывания пришли на CPU 1), домен MSI устройства,
  вектор 0 и тип — фронт (`edge`): MSI — событие, а не уровень на линии;
- в `/proc/iomem` — регион BAR0 под именем, которое драйвер передал
  `iomap_region_sized`;
- после `rmmod` региона нет: `Drop` `Bar` вызвал `pci_iounmap` и
  `pci_release_region`, хотя драйвер ни разу не написал «освободи»;
- а `enable` в sysfs — счётчик включений устройства — был 0 и остался
  1: выключить устройство драйвер на Rust в 7.0 не может.

## Границы регистров — при компиляции

`read32` и `write32` у `Bar<SIZE>` проверяют смещение. Проверка — в
`io_addr_assert`:

<!-- linux: rust/kernel/io.rs -->
```rust,ignore
const fn offset_valid<U>(offset: usize, size: usize) -> bool {
    let type_size = core::mem::size_of::<U>();
    if let Some(end) = offset.checked_add(type_size) {
        end <= size && offset % type_size == 0
    } else {
        false
    }
}
…
    // Always inline to optimize out error path of `build_assert`.
    #[inline(always)]
    fn io_addr_assert<U>(&self, offset: usize) -> usize {
        build_assert!(offset_valid::<U>(offset, Self::MIN_SIZE));
```

`build_assert!` — не `assert!`. Если условие ложно, макрос вызывает
`build_error`, функцию без тела в смысле компоновки:

<!-- linux: rust/build_error.rs -->
```rust,ignore
#[inline(never)]
#[cold]
#[export_name = "rust_build_error"]
#[track_caller]
pub const fn build_error(msg: &'static str) -> ! {
    panic!("{}", msg);
}
```

Символ `rust_build_error` ядро не экспортирует:

<!-- linux: rust/exports.c -->
```c
// For modules using `rust/build_error.rs`.
#ifdef CONFIG_RUST_BUILD_ASSERT_ALLOW
EXPORT_SYMBOL_RUST_GPL(rust_build_error);
#endif
```

Когда смещение — константа, а функции встроены (`#[inline(always)]`),
оптимизатор вычисляет условие и выбрасывает вызов. Если вызов остался
— условие ложно или его не удалось вычислить, — модуль не скомпонуется.
Два модуля в `modules-fail/mmio`:

```rust,ignore
{{#include ../../code/ch29/modules-fail/mmio/ch29_fail_mmio.rs:past}}
```

```rust,ignore
{{#include ../../code/ch29/modules-fail/mmio/ch29_fail_mmio_var.rs:var}}
```

Первый читает за концом BAR, второй — регистр `ID` по смещению 0, но
`black_box` прячет от компилятора, что смещение — 0. Сборка (тест
`compile_fail_modules`):

```text
  RUSTC [M] ch29_fail_mmio.o
  RUSTC [M] ch29_fail_mmio_var.o
ch29_fail_mmio.o: warning: objtool: _RINvXs5_NtNtCs3R38tTcvFoT_6kernel5alloc4kbox… falls through to next function _RINvXsi_…
  MODPOST Module.symvers
ERROR: modpost: "rust_build_error" [ch29_fail_mmio.ko] undefined!
ERROR: modpost: "rust_build_error" [ch29_fail_mmio_var.ko] undefined!
```

rustc оба модуля собрал без ошибок: проверку довершает компоновка.
Предупреждение objtool — побочное: в первом модуле вызов безусловный,
после него в функции нет ни одной инструкции, а objtool не знает, что
`rust_build_error` не возвращается, — в его списке невозвращающихся
функций Rust только имена вида `_R…`, а у этой имя задано
`#[export_name]`. Для смещений, известных только при выполнении, есть
вариант с проверкой при выполнении:

<!-- linux: rust/kernel/io.rs -->
```rust,ignore
    /// Fallible 32-bit read with runtime bounds check.
    #[inline(always)]
    fn try_read32(&self, _offset: usize) -> Result<u32>
```

С `io.try_read32(offset)?` второй модуль собирается — тест проверяет и
это.

<div class="box trap">

**Ловушка: ошибка без адреса.** modpost называет модуль, но не строку:
`rust_build_error` мог остаться от любого `read*`/`write*` — и от
любого другого `build_assert!` крейта `kernel`. Ищите обращения с
вычисляемым смещением (индекс из цикла, номер из `ioctl`) и смещения не
меньше `SIZE` у `Bar<SIZE>`. Ещё одна причина — оптимизатор не встроил
функцию, и условие осталось невычисленным: `build_assert!` полагается
на `#[inline(always)]` всей цепочки вызовов. Параметр ядра
`CONFIG_RUST_BUILD_ASSERT_ALLOW` превращает такие проверки в панику при
выполнении, но справка Kconfig называет его запасным выходом, а не
способом собирать драйверы:

<!-- linux: lib/Kconfig.debug -->
```text
	  This should not happen, thus by default the build is aborted. However,
	  as an escape hatch, you can choose Y here to ignore them during build
	  and let the check be carried at runtime (with `panic!` being called if
	  the check fails).
```

</div>

<div class="box check">

**Проверь себя.**

1. Почему `enable_device_mem` нельзя вызвать из обработчика
   прерывания, а `Devres::access` — можно?
2. Что случится с работой в очереди, которая держит клон
   `Arc<Devres<Bar0>>` и зовёт `try_access()` после `rmmod`?
3. Драйвер хочет дать пользователю `ioctl` «прочитать регистр N».
   Почему здесь нужен `try_read32`, а не `read32`, даже если N
   проверено на `< 0x100` строкой выше?
4. Что изменится, если обработчик будет хранить `causes` в
   `SpinLock<u32>`, а не в атомике? Какой вид блокировки нужен под
   прерыванием и почему в 7.0 его нет в крейте `kernel`?

</div>
