//! Раздел 9.5: маркерные трейты и автотрейты.
//!
//! `Copy` — обещание, что копия байтов и есть копия значения. С
//! деструктором это обещание несовместимо:
//!
//! ```compile_fail,E0184
//! #[derive(Clone, Copy)]
//! struct Guard(u8);
//! impl Drop for Guard {
//!     fn drop(&mut self) {}
//! }
//! ```
//!
//! Сырой указатель лишает тип `Send` и `Sync`, и тип со сырым указателем
//! внутри — тоже:
//!
//! ```compile_fail,E0277
//! fn needs_send<T: Send>() {}
//! needs_send::<ch09::markers::RawBuf>();
//! ```
//!
//! `Cell` можно передать в другой поток, но нельзя разделить между
//! потоками, а `static` разделён между всеми потоками и прерываниями:
//!
//! ```compile_fail,E0277
//! use core::cell::Cell;
//! static TICKS: Cell<u32> = Cell::new(0);
//! ```
//!
//! Типу-маркеру хватает `PhantomData`, чтобы стать `!Send`:
//!
//! ```compile_fail,E0277
//! fn needs_send<T: Send>() {}
//! needs_send::<ch09::markers::IrqToken>();
//! ```

use core::cell::Cell;
use core::marker::PhantomData;

// ANCHOR: copy
/// Отсчёт: копия байтов — полноценная копия, поэтому `Copy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sample {
    pub sensor: u16,
    pub value: i32,
}
// ANCHOR_END: copy

// ANCHOR: auto
/// Буфер DMA: адрес и длина. Сырой указатель снимает `Send` и `Sync`.
pub struct RawBuf {
    pub ptr: *mut u8,
    pub len: usize,
}

/// Тот же буфер с обещанием автора: владение буфером можно передать
/// другому потоку.
pub struct DmaBuf {
    ptr: *mut u8,
    len: usize,
}

// SAFETY: DmaBuf — единственный владелец памяти по ptr (конструктор
// принимает &'static mut), поэтому передача в другой поток не создаёт
// второго пути к тем же байтам. Sync не реализован: &DmaBuf из двух
// потоков позволил бы читать во время записи DMA.
unsafe impl Send for DmaBuf {}

impl DmaBuf {
    pub fn new(buf: &'static mut [u8]) -> Self {
        Self { ptr: buf.as_mut_ptr(), len: buf.len() }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn addr(&self) -> usize {
        self.ptr as usize
    }
}

/// Разрешение работать с регистрами прерываний: живёт только в том
/// контексте, где создано. `PhantomData<*const ()>` ничего не хранит, но
/// делает тип `!Send` и `!Sync`.
pub struct IrqToken {
    _not_send: PhantomData<*const ()>,
}

impl IrqToken {
    /// # Safety
    ///
    /// Вызывать только в обработчике прерывания.
    pub unsafe fn new() -> Self {
        Self { _not_send: PhantomData }
    }
}

/// Счётчик с внутренней изменяемостью: `Send`, но не `Sync`.
pub struct Ticks(pub Cell<u32>);
// ANCHOR_END: auto

// ANCHOR: sized
/// `?Sized` снимает неявное требование `Sized`: функция примет и `str`, и
/// срез, и `dyn`-объект, но только за ссылкой.
pub fn byte_len<T: ?Sized>(value: &T) -> usize {
    size_of_val(value)
}
// ANCHOR_END: sized

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sensor::{Sensor, Thermo};

    /// Компилируется, только если `T: Send`.
    fn is_send<T: Send>(_: &T) -> bool {
        true
    }

    fn is_sync<T: Sync>(_: &T) -> bool {
        true
    }

    #[test]
    fn auto_traits() {
        let s = Sample { sensor: 1, value: 2 };
        assert!(is_send(&s) && is_sync(&s));
        assert!(is_send(&Ticks(Cell::new(0))));
        assert_eq!(size_of::<IrqToken>(), 0);
        // Буфер DMA в прошивке — статический массив, как здесь.
        static mut BUF: [u8; 16] = [0; 16];
        // SAFETY: других ссылок на BUF нет: он виден только этому тесту,
        // а тест выполняется один раз.
        let b = DmaBuf::new(unsafe { (&raw mut BUF).as_mut() }.unwrap());
        assert!(is_send(&b));
        assert_eq!(b.len(), 16);
        assert!(!b.is_empty() && b.addr() != 0);
    }

    #[test]
    fn sized_and_unsized() {
        let t = Thermo { raw: 1 };
        let d: &dyn Sensor = &t;
        assert_eq!(byte_len("отсчёт"), 12);
        assert_eq!(byte_len(&[1u16, 2, 3][..]), 6);
        assert_eq!(byte_len(d), 4);
        assert_eq!(size_of::<&Thermo>(), size_of::<usize>());
        assert_eq!(size_of::<&[u16]>(), 2 * size_of::<usize>());
        assert_eq!(size_of::<&dyn Sensor>(), 2 * size_of::<usize>());
    }
}
