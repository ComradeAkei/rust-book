//! Задача 23.C: макрос описания периферии `peripheral!`.
//!
//! Каждый регистр — метод, возвращающий типизированный [`Reg`]: тип
//! значения и право доступа (`ro`, `wo`, по умолчанию чтение и запись) —
//! параметры типа. Запись в регистр только для чтения не компилируется:
//!
//! ```compile_fail,E0599
//! ch23_solutions::peripheral! {
//!     UART0 @ 0x4000_C000 { DR: u32 @ 0x00, FR: u32 @ 0x18 ro }
//! }
//! let mut regs = [0u32; 8];
//! // SAFETY: блок регистров — массив выше.
//! let uart = unsafe { UART0::at(regs.as_mut_ptr().cast()) };
//! uart.FR().write(0); // method `write` exists, but trait bounds were not satisfied
//! ```
//!
//! Чтение из регистра только для записи — тоже:
//!
//! ```compile_fail,E0599
//! ch23_solutions::peripheral! {
//!     UART0 @ 0x4000_C000 { ICR: u32 @ 0x44 wo }
//! }
//! let mut regs = [0u32; 32];
//! let uart = unsafe { UART0::at(regs.as_mut_ptr().cast()) };
//! let _ = uart.ICR().read();
//! ```
//!
//! Неизвестное право доступа — ошибка «no rules expected identifier `rx`».
//! Указывает она на весь вызов, а не на слово: слово разбирает вложенный
//! вызов `peripheral!(@access rx)`, и rustc 1.97 показывает внешний.
//!
//! ```compile_fail
//! ch23_solutions::peripheral! {
//!     UART0 @ 0x4000_C000 { DR: u32 @ 0x00 rx }
//! }
//! ```
//!
//! Смещение, не кратное выравниванию регистра, ловит `const`-проверка:
//!
//! ```compile_fail,E0080
//! ch23_solutions::peripheral! {
//!     UART0 @ 0x4000_C000 { DR: u32 @ 0x02 }
//! }
//! ```

use core::marker::PhantomData;
use core::ptr;

// ANCHOR: reg
/// Право доступа — тип без значений; `Reg<u32, RO>` и `Reg<u32, RW>` —
/// разные типы с разными методами.
pub enum RO {}
pub enum WO {}
pub enum RW {}

pub trait Readable {}
pub trait Writable {}
impl Readable for RO {}
impl Readable for RW {}
impl Writable for WO {}
impl Writable for RW {}

/// Регистр устройства: адрес и тип значения, обращения только volatile.
pub struct Reg<T, A> {
    ptr: *mut T,
    _access: PhantomData<A>,
}

impl<T: Copy, A> Reg<T, A> {
    /// # Safety
    ///
    /// `ptr` — выровненный адрес регистра (или памяти, которая его
    /// изображает), живой всё время жизни `Reg`.
    pub const unsafe fn new(ptr: *mut T) -> Self {
        Self { ptr, _access: PhantomData }
    }
}

impl<T: Copy, A: Readable> Reg<T, A> {
    pub fn read(&self) -> T {
        // SAFETY: адрес валиден и выровнен по предусловию `new`.
        unsafe { ptr::read_volatile(self.ptr) }
    }
}

impl<T: Copy, A: Writable> Reg<T, A> {
    pub fn write(&self, value: T) {
        // SAFETY: то же, что у `read`.
        unsafe { ptr::write_volatile(self.ptr, value) }
    }
}

impl<T: Copy, A: Readable + Writable> Reg<T, A> {
    /// Прочитать, изменить, записать — два обращения, не атомарно.
    pub fn modify(&self, f: impl FnOnce(T) -> T) {
        self.write(f(self.read()));
    }
}
// ANCHOR_END: reg

// ANCHOR: solution
/// `peripheral! { UART0 @ 0x4000_C000 { DR: u32 @ 0x00, FR: u32 @ 0x18 ro } }`
///
/// Тип регистра — `ident`, а не `ty`: после `ty` нельзя поставить `@`, а
/// `u8`, `u16`, `u32` для разбора — обычные имена. Смещение — литерал:
/// после `expr` слово доступа не поставить, а литерал к тому же не даст
/// вызывающему вставить код в `unsafe` аксессора.
#[macro_export]
macro_rules! peripheral {
    // Право доступа: по умолчанию — чтение и запись.
    (@access) => { $crate::c_periph::RW };
    (@access rw) => { $crate::c_periph::RW };
    (@access ro) => { $crate::c_periph::RO };
    (@access wo) => { $crate::c_periph::WO };
    (
        $(#[$m:meta])*
        $vis:vis $name:ident @ $base:literal {
            $(
                $(#[$rm:meta])*
                $reg:ident: $ty:ident @ $off:literal $($access:ident)?
            ),* $(,)?
        }
    ) => {
        $(#[$m])*
        $vis struct $name {
            base: *mut u8,
        }

        #[allow(dead_code)]
        impl $name {
            pub const BASE: usize = $base;

            /// Блок регистров по его настоящему адресу.
            ///
            /// # Safety
            ///
            /// Только на машине, где по `BASE` это устройство, и без
            /// второго владельца: два `steal` дают две ручки на одни
            /// регистры.
            pub unsafe fn steal() -> Self {
                Self { base: ::core::ptr::with_exposed_provenance_mut(Self::BASE) }
            }

            /// Блок регистров по произвольному адресу — для тестов на
            /// обычной памяти.
            ///
            /// # Safety
            ///
            /// `base` указывает на память, покрывающую все регистры блока,
            /// выровненную под каждый, живую всё время жизни значения.
            pub const unsafe fn at(base: *mut u8) -> Self {
                Self { base }
            }

            $(
                $(#[$rm])*
                #[allow(non_snake_case)]
                pub fn $reg(&self) -> $crate::c_periph::Reg<$ty, $crate::peripheral!(@access $($access)?)> {
                    // SAFETY: смещение внутри блока и выровнено (проверка
                    // ниже); base валиден по предусловию конструктора.
                    unsafe { $crate::c_periph::Reg::new(self.base.wrapping_add($off).cast::<$ty>()) }
                }
            )*
        }

        // Проверки при компиляции — по одной на регистр.
        $(
            const _: () = ::core::assert!(
                $off % ::core::mem::align_of::<$ty>() == 0,
                "смещение регистра не кратно его выравниванию"
            );
        )*
    };
}
// ANCHOR_END: solution
