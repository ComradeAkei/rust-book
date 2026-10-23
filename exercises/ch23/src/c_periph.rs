//! Задача C: макрос описания периферии `peripheral!`.
//!
//! Образец разбирает вход, но пока выдаёт каждому регистру право `RW`, а
//! чтение и запись — заглушки. Допишите: `Reg::read`, `write`, `modify`
//! (volatile), выбор права по слову `ro`/`wo`/`rw` (без слова — `RW`),
//! `steal` и проверку выравнивания смещения при компиляции. Пока право
//! одно, doc-тесты ниже компилируются — и потому падают.
//!
//! Каждый регистр — метод, возвращающий типизированный [`Reg`]: тип
//! значения и право доступа (`ro`, `wo`, по умолчанию чтение и запись) —
//! параметры типа. Запись в регистр только для чтения не компилируется:
//!
//! ```compile_fail,E0599
//! ch23_exercises::peripheral! {
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
//! ch23_exercises::peripheral! {
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
//! ch23_exercises::peripheral! {
//!     UART0 @ 0x4000_C000 { DR: u32 @ 0x00 rx }
//! }
//! ```
//!
//! Смещение, не кратное выравниванию регистра, ловит `const`-проверка:
//!
//! ```compile_fail,E0080
//! ch23_exercises::peripheral! {
//!     UART0 @ 0x4000_C000 { DR: u32 @ 0x02 }
//! }
//! ```

use core::marker::PhantomData;
use core::ptr;

// ANCHOR: skeleton_reg
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
        todo!("раздел 23.7, задача C")
    }
}

impl<T: Copy, A: Writable> Reg<T, A> {
    pub fn write(&self, value: T) {
        todo!("раздел 23.7, задача C")
    }
}

impl<T: Copy, A: Readable + Writable> Reg<T, A> {
    /// Прочитать, изменить, записать — два обращения, не атомарно.
    pub fn modify(&self, f: impl FnOnce(T) -> T) {
        todo!("раздел 23.7, задача C")
    }
}
// ANCHOR_END: skeleton_reg

// ANCHOR: skeleton
#[macro_export]
macro_rules! peripheral {
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
            /// второго владельца.
            pub unsafe fn steal() -> Self {
                ::core::todo!("раздел 23.7, задача C")
            }

            /// Блок регистров по произвольному адресу — для тестов.
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
                pub fn $reg(&self) -> $crate::c_periph::Reg<$ty, $crate::c_periph::RW> {
                    ::core::todo!("раздел 23.7, задача C: право доступа по слову ro/wo/rw")
                }
            )*
        }
    };
}
// ANCHOR_END: skeleton
