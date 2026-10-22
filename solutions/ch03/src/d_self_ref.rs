//! Задача 3.D. Самоссылающийся буфер приёма и что с ним делает
//! перемещение.
//!
//! Из `Pin<Box<PinnedRx>>` нельзя достать значение и переместить его: для
//! этого нужен `&mut PinnedRx`, а `Pin` его не отдаёт, пока тип `!Unpin`.
//!
//! ```compile_fail,E0507
//! use ch03_solutions::d_self_ref::PinnedRx;
//!
//! let rx = PinnedRx::new();
//! let moved: PinnedRx = *rx;
//! ```

use alloc::boxed::Box;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::ptr;

/// Ёмкость буфера.
pub const CAP: usize = 32;

#[derive(Debug, PartialEq)]
pub struct Full;

// ANCHOR: naive
/// Буфер приёма: `cursor` указывает на следующую свободную ячейку внутри
/// собственного `buf`, `home` — адрес, по которому структура лежала в
/// момент `init`.
///
/// Этот тип несостоятелен (unsound) нарочно: безопасный код может
/// переместить буфер и вызвать `push`, а та запишет по старому адресу.
/// Тесты `naive_*_ub` показывают это под Miri.
pub struct RxBuf {
    buf: [u8; CAP],
    cursor: *mut u8,
    len: usize,
    home: *const RxBuf,
}

impl RxBuf {
    pub const fn new() -> Self {
        RxBuf { buf: [0; CAP], cursor: ptr::null_mut(), len: 0, home: ptr::null() }
    }

    /// Настроить курсор на начало собственного буфера.
    pub fn init(&mut self) {
        self.cursor = self.buf.as_mut_ptr();
        self.len = 0;
        self.home = self;
    }

    pub fn push(&mut self, byte: u8) -> Result<(), Full> {
        assert!(!self.cursor.is_null(), "буфер не инициализирован");
        if self.len == CAP {
            return Err(Full);
        }
        // SAFETY: верно, только пока структура лежит там же, где в `init`,
        // и пока никто не создал `&mut RxBuf` после `init`. Ни то, ни другое
        // тип не обеспечивает: см. разбор задачи.
        unsafe {
            self.cursor.write(byte);
            self.cursor = self.cursor.add(1);
        }
        self.len += 1;
        Ok(())
    }

    pub fn received(&self) -> &[u8] {
        &self.buf[..self.len]
    }

    /// Перемещён ли буфер после `init`. Сравниваются только адреса,
    /// память по `home` не читается.
    pub fn moved(&self) -> bool {
        !self.home.is_null() && !ptr::eq(self.home, self)
    }
}

impl Default for RxBuf {
    fn default() -> Self {
        Self::new()
    }
}
// ANCHOR_END: naive

// ANCHOR: index
/// Исправление первое: вместо указателя — смещение от начала буфера.
/// Смещение не зависит от адреса структуры, и перемещать её можно сколько
/// угодно.
pub struct RxIdx {
    buf: [u8; CAP],
    len: u8,
}

impl RxIdx {
    pub const fn new() -> Self {
        RxIdx { buf: [0; CAP], len: 0 }
    }

    pub fn push(&mut self, byte: u8) -> Result<(), Full> {
        let slot = self.buf.get_mut(self.len as usize).ok_or(Full)?;
        *slot = byte;
        self.len += 1;
        Ok(())
    }

    pub fn received(&self) -> &[u8] {
        &self.buf[..self.len as usize]
    }
}

impl Default for RxIdx {
    fn default() -> Self {
        Self::new()
    }
}
// ANCHOR_END: index

// ANCHOR: pinned
/// Исправление второе: оставить указатель, но запретить перемещение.
/// Значение живёт в куче, наружу выдаётся только `Pin<Box<Self>>`, а
/// `PhantomPinned` делает тип `!Unpin`: безопасный код не получит
/// `&mut PinnedRx` и не сможет вынуть значение из-под `Pin`.
pub struct PinnedRx {
    buf: [u8; CAP],
    cursor: *mut u8,
    len: usize,
    _pin: PhantomPinned,
}

impl PinnedRx {
    pub fn new() -> Pin<Box<Self>> {
        let mut rx = Box::pin(PinnedRx {
            buf: [0; CAP],
            cursor: ptr::null_mut(),
            len: 0,
            _pin: PhantomPinned,
        });
        // SAFETY: значение не перемещается: меняем поле на месте.
        let this = unsafe { rx.as_mut().get_unchecked_mut() };
        this.cursor = this.buf.as_mut_ptr();
        rx
    }

    pub fn push(self: Pin<&mut Self>, byte: u8) -> Result<(), Full> {
        // SAFETY: значение не перемещается: пишем поля на месте.
        let this = unsafe { self.get_unchecked_mut() };
        if this.len == CAP {
            return Err(Full);
        }
        // SAFETY: значение закреплено с момента `new`, поэтому `cursor`
        // по-прежнему указывает в `this.buf` со смещением len < CAP.
        unsafe {
            this.cursor.write(byte);
            this.cursor = this.cursor.add(1);
        }
        this.len += 1;
        Ok(())
    }

    pub fn received(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}
// ANCHOR_END: pinned

// ANCHOR: asm
// Обёртки для сравнения `push` по ассемблеру. Маленькие методы без
// `#[inline]` rustc сам делает встраиваемыми в другие крейты и не
// порождает в этом, пока их никто не вызывает.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03d_push_idx(rx: &mut RxIdx, byte: u8) -> Result<(), Full> {
    rx.push(byte)
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03d_push_pinned(rx: Pin<&mut PinnedRx>, byte: u8) -> Result<(), Full> {
    rx.push(byte)
}
// ANCHOR_END: asm
