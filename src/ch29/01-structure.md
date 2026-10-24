# 29.1. Rust for Linux: привязки, помощники, крейт kernel

Поддержка Rust вошла в основную ветку ядра в версии 6.1 (2022). Ядро
по-прежнему написано на C; Rust — второй язык для драйверов и модулей,
и всё, что модуль на Rust делает, он делает вызовами того же C-API, что
и модуль на C. Между модулем и этим API — три слоя, и все три лежат в
каталоге `rust/` исходников ядра.

<div class="box ref">

**Справка: модуль ядра.** Модуль — объектный файл `.ko`, который ядро
загружает во время работы вызовом `finit_module(2)` (так делает
`insmod`) и выгружает вызовом `delete_module(2)` (`rmmod`). Загрузчик
ядра размещает код модуля в отдельной области адресов, связывает его
неразрешённые символы с символами, которые ядро экспортировало
(`EXPORT_SYMBOL`), и вызывает функцию инициализации модуля. Модуль
работает с полными правами ядра: общая память, общие структуры, никакой
защиты одного модуля от другого. Ошибка в модуле — ошибка ядра.

</div>

## Привязки: bindgen над заголовками C

Первый слой — крейт `bindings`. Его исходник генерирует при сборке ядра
bindgen: читает заголовки из `rust/bindings/bindings_helper.h` и пишет
объявления Rust для всех функций, структур и констант, до которых
дотянулся. Для конфигурации ядра этой главы получается
`rust/bindings/bindings_generated.rs` на 117 258 строк. Вот как в нём
выглядит misc-устройство:

<!-- linux: build/rust/bindings/bindings_generated.rs -->
```rust,ignore
#[repr(C)]
#[derive(Copy, Clone, MaybeZeroable)]
pub struct miscdevice {
    pub minor: ffi::c_int,
    pub name: *const ffi::c_char,
    pub fops: *const file_operations,
    pub list: list_head,
    pub parent: *mut device,
    pub this_device: *mut device,
    pub groups: *mut *const attribute_group,
    pub nodename: *const ffi::c_char,
    pub mode: umode_t,
}
…
extern "C" {
    pub fn misc_register(misc: *mut miscdevice) -> ffi::c_int;
}
```

Это C, записанный синтаксисом Rust: `#[repr(C)]`, сырые указатели, код
ошибки в `c_int`. Вызов `misc_register` — `unsafe`, и все правила C
остаются правилами: структура должна жить, пока устройство
зарегистрировано, `name` — указывать на строку с нулём в конце,
`misc_deregister` — быть вызвана ровно один раз. Писать модуль прямо на
привязках можно (`kernel::bindings` открыт), но это тот же C без единой
проверки — плюс `unsafe` на каждой строке.

## Помощники: то, чего нет в объектном коде

Не всё из заголовков C можно вызвать по имени. `static inline`-функции
и макросы не оставляют символа, который можно было бы позвать из
другого языка, а ядро держит в заголовках очень многое: `kmalloc`,
`spin_lock`, `refcount_inc`, `copy_to_user`. Для них в `rust/helpers/`
лежат 57 файлов на C с обёртками — настоящими функциями, которые
вызывают макрос или inline-функцию:

<!-- linux: rust/helpers/mutex.c -->
```c
__rust_helper void rust_helper_mutex_lock(struct mutex *lock)
{
	mutex_lock(lock);
}
```

bindgen генерирует для них объявления под исходным именем, с
`#[link_name]`, — и код на Rust зовёт `bindings::mutex_lock`, не зная,
чем это имя было в C:

<!-- linux: build/rust/bindings/bindings_helpers_generated.rs -->
```rust,ignore
extern "C" {
    #[link_name="rust_helper_mutex_lock"]
    pub fn mutex_lock(lock: *mut mutex);
}
```

Зачем обёртка для `mutex_lock`, которая в C — обычная функция? Не
всегда обычная. С отладкой блокировок (`CONFIG_DEBUG_LOCK_ALLOC`, её
включает lockdep) это макрос:

<!-- linux: include/linux/mutex.h -->
```c
#define mutex_lock(lock) mutex_lock_nested(lock, 0)
…
extern void mutex_lock(struct mutex *lock) __acquires(lock);
```

Привязка к функции из `bindings_generated.rs` есть только в обычном
ядре, привязка к помощнику — всегда, и крейт `bindings` устроен так,
что первая, если есть, перекрывает вторую. Набор привязок, таким
образом, зависит от конфигурации ядра: два ядра этой главы, обычное и
отладочное, дают модулю разные `bindings`.

<div class="box hood">

**Под капотом: цена помощника.** Помощник — вызов настоящей функции там,
где C подставил бы тело на месте: `copy_to_user` в C —
`static __always_inline`, а из Rust это вызов `rust_helper_copy_to_user`,
внутри которого уже подставлен `copy_to_user`. Каждый помощник помечен
`__rust_helper` — заготовкой для будущего встраивания помощников в код
Rust, — но в 7.0 это пустой макрос (`#define __rust_helper` в
`rust/helpers/helpers.c`), и в ассемблере модулей видны вызовы
`rust_helper_*`. Их цену в горячем пути меряет задача
[29.D2](10-tasks.md).

</div>

## Крейт kernel: правила в типах

Третий слой — крейт `kernel`: 146 файлов и 47 882 строки в ядре 7.0.
Это безопасные абстракции над привязками: каждая обещает, что
безопасный код с ней не нарушит правил C-API, — и обоснование каждого
`unsafe` внутри неё написано рядом. Вот мьютекс и спин-блокировка ядра,
один тип на оба случая:

<!-- linux: rust/kernel/sync/lock.rs -->
```rust,ignore
#[repr(C)]
#[pin_data]
pub struct Lock<T: ?Sized, B: Backend> {
    /// The kernel lock object.
    #[pin]
    state: Opaque<B::State>,

    /// Some locks are known to be self-referential (e.g., mutexes), while others are architecture
    /// or config defined (e.g., spinlocks). So we conservatively require them to be pinned in case
    /// some architecture uses self-references now or in the future.
    #[pin]
    _pin: PhantomPinned,

    /// The data protected by the lock.
    #[pin]
    pub(crate) data: UnsafeCell<T>,
}

// SAFETY: `Lock` can be transferred across thread boundaries iff the data it protects can.
unsafe impl<T: ?Sized + Send, B: Backend> Send for Lock<T, B> {}

// SAFETY: `Lock` serialises the interior mutability it provides, so it is `Sync` as long as the
// data it protects is `Send`.
unsafe impl<T: ?Sized + Send, B: Backend> Sync for Lock<T, B> {}
```

Три правила C-API стали частью типа. Данные лежат в блокировке, а не
рядом с ней: добраться до `data` можно только через охранника, которого
возвращает `lock()` (раздел [29.6](06-sync.md)). `Opaque<B::State>` —
`struct mutex` или `spinlock_t`, о содержимом которых Rust ничего не
предполагает: их меняет C. `PhantomPinned` делает тип не-`Unpin`:
однажды инициализированную блокировку нельзя переместить (раздел
[29.5](05-pin-init.md)). `Mutex<T>` и `SpinLock<T>` — этот же `Lock` с
разными `Backend`.

Рядом с `kernel` живут ещё несколько крейтов: `pin_init` —
инициализация на месте; `macros` — процедурные макросы `module!`,
`#[vtable]`, `#[pin_data]` (они собираются как библиотеки для машины
сборки, `rust/libmacros.so`); `ffi` — типы C (`ffi::c_int`); и `core`
— тот же `core`, что у любой программы без std (глава
[26](../ch26/index.md)), только собранный из исходников `rust-src` под
цель ядра.

## Что попадает в модуль

Модуль на Rust компилируется против метаданных этих крейтов
(`rust/libkernel.rmeta` — 5,4 МБ, `rust/libcore.rmeta` — 60 МБ в
каталоге сборки). Обычные функции крейтов уже скомпилированы в ядро и
экспортированы: сборка генерирует `rust/exports_kernel_generated.h` со
строками вида

<!-- linux: build/rust/exports_kernel_generated.h -->
```text
EXPORT_SYMBOL_RUST_GPL(_RINvCscUF0uj1CVC4_8pin_init14pin_init_scopeINtNtCs3R38tTcvFoT_6kernel6devres6DevresNtNtBK_9auxiliary12RegistrationENtNtBK_5error5ErrorNCNvMsc_B1l_B1j_3new0INtNtCsi1tY5tVYnWh_4core6result6ResultBF_B1P_EEBK_);
```

— 670 символов из `kernel`, 630 из `core`, 1414 из `bindings` и 388
помощников для этой конфигурации. Обобщённый код экспортировать нельзя:
его мономорфизирует компилятор модуля, и он оказывается в самом `.ko`.
В символах `ch29_hello.ko` из раздела [29.3](03-module.md) — методы
`MaybeUninit`, `SetOnce` и `ModuleParamAccess` из `core` и `kernel`,
специализированные под типы модуля:

```console
$ llvm-nm ch29_hello.ko | grep ' T '
0000000000000000 T _RNvMs1_NtCs3R38tTcvFoT_6kernel12module_paramINtB5_17ModuleParamAccessmE5valueCsgBrfut3XapP_10ch29_hello
0000000000000023 T _RNvMs1_NtNtCsi1tY5tVYnWh_4core3mem12maybe_uninitINtB5_11MaybeUninitNtCsgBrfut3XapP_10ch29_hello5HelloE16assume_init_dropB15_
…
```

Имена — в схеме искажения v0 (`_R…`): в ней есть крейт, модуль, тип и
параметры обобщения, и `rustfilt` или `llvm-cxxfilt` превращают её
обратно в путь Rust. Отчёты ядра печатают эти имена как есть (раздел
[29.9](09-oops.md)).

Экспорт — `EXPORT_SYMBOL_GPL`: символы Rust доступны только модулям с
лицензией, совместимой с GPL. Модуль с `license: "Proprietary"`
загрузчик не свяжет с ними.

<div class="box trap">

**Ловушка: глава привязана к ядру 7.0.** Внутренний API ядра не
стабилен по принципу (`Documentation/process/stable-api-nonsense.rst`):
функции переименовывают, а их пользователей исправляют в том же
изменении. Абстракции Rust моложе C-API и меняются быстрее: в 7.0
`ARef` живёт в `kernel::sync::aref`, а `kernel::types::ARef` остался
реэкспортом на время переезда. Примеры главы собираются ядром 7.0 и
только им; с другим ядром часть из них не соберётся.

То же с компилятором. Ядро генерирует для rustc описание цели
(`target.json`), и его формат — нестабильный интерфейс rustc. Модуль
нужно собирать тем rustc, которым собрано ядро: метаданные крейта
`kernel` другая версия не прочитает. А rustc новее ядра может не
прочитать даже описание цели — так ответил rustc 1.101.0-nightly от
2026-10-21 на ядро 7.0:

```console
$ RUSTUP_TOOLCHAIN=nightly make -C target/ch29-kernel/build M=$PWD MO=out LLVM=1
  RUSTC [M] ch29_hello.o
error: error loading target specification: rustc-abi: invalid rustc abi: 'x86-softfloat'. allowed values: 'softfloat', 'x86-sse2', 'powerpc-spe', 'sparc-v8plus' at line 3 column 32
```

Поэтому `build.sh` берёт версию rustc из `rust-toolchain.toml` книги, а
обвязка главы при сборке модулей — из `CONFIG_RUSTC_VERSION_TEXT`
собранного ядра.

</div>

<div class="box c">

**Как в C.** Модуль на C компилируется против заголовков и того же
`.config`: структура `struct mutex` в обычном и отладочном ядре разного
размера, и модуль, собранный для одного, другому не подходит — отсюда
`vermagic` в каждом `.ko` (раздел [29.3](03-module.md)). Для Rust
добавляется ещё одно условие — та же версия компилятора, — зато
отпадает целый класс ошибок: привязки генерируются из тех же заголовков
при каждой сборке и не могут разойтись с C молча.

</div>

<div class="box check">

**Проверь себя.**

1. Почему `copy_to_user` нельзя объявить в `extern "C"` и вызвать
   напрямую, а `misc_register` — можно?
2. Почему набор функций в `kernel::bindings` разный у обычного и
   отладочного ядра главы?
3. Где окажется машинный код `KVec::<u32>::push`, если модуль вызывает
   его: в ядре или в `.ko`? А код `Error::to_errno`?

</div>
