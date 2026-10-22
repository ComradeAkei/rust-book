//! Задача 3.D. Самоссылающийся буфер приёма и что с ним делает
//! перемещение.

use alloc::boxed::Box;
use core::marker::PhantomPinned;
use core::pin::Pin;
use core::ptr;

/// Ёмкость буфера.
pub const CAP: usize = 32;

#[derive(Debug, PartialEq)]
pub struct Full;

/// Буфер приёма: курсор — сырой указатель на следующую свободную ячейку
/// внутри собственного `buf`. Поля, кроме `buf`, выберите сами.
pub struct RxBuf {
    buf: [u8; CAP],
}

impl RxBuf {
    pub const fn new() -> Self {
        RxBuf { buf: [0; CAP] }
    }

    /// Настроить курсор на начало собственного буфера.
    pub fn init(&mut self) {
        todo!()
    }

    /// Записать байт по курсору и сдвинуть курсор.
    pub fn push(&mut self, byte: u8) -> Result<(), Full> {
        todo!()
    }

    pub fn received(&self) -> &[u8] {
        todo!()
    }

    /// Перемещён ли буфер после `init`. До `init` — нет.
    pub fn moved(&self) -> bool {
        todo!()
    }
}

impl Default for RxBuf {
    fn default() -> Self {
        Self::new()
    }
}

/// Исправление первое: никаких указателей внутрь себя. Не больше
/// `CAP + 1` байт.
pub struct RxIdx {
    buf: [u8; CAP],
}

impl RxIdx {
    pub const fn new() -> Self {
        RxIdx { buf: [0; CAP] }
    }

    pub fn push(&mut self, byte: u8) -> Result<(), Full> {
        todo!()
    }

    pub fn received(&self) -> &[u8] {
        todo!()
    }
}

impl Default for RxIdx {
    fn default() -> Self {
        Self::new()
    }
}

/// Исправление второе: указатель остаётся, но перемещать значение нельзя.
pub struct PinnedRx {
    buf: [u8; CAP],
    _pin: PhantomPinned,
}

impl PinnedRx {
    pub fn new() -> Pin<Box<Self>> {
        todo!()
    }

    pub fn push(self: Pin<&mut Self>, byte: u8) -> Result<(), Full> {
        todo!()
    }

    pub fn received(&self) -> &[u8] {
        todo!()
    }
}
