//! Раздел 5.4: вариантность и `PhantomData`.
//!
//! `&mut T` инвариантен по `T`: через `&mut &'static str` нельзя записать
//! ссылку на локальную строку, иначе после выхода из блока `name`
//! указывала бы в освобождённую память:
//!
//! ```compile_fail,E0597
//! use ch05::variance::set;
//!
//! let mut name: &'static str = "по умолчанию";
//! {
//!     let local = String::from("из конфигурации");
//!     set(&mut name, &local);
//! }
//! println!("{name}");
//! ```
//!
//! `Cell<T>` тоже инвариантен по `T`: укоротить время жизни внутри ячейки
//! нельзя, ячейка разрешает запись через `&`:
//!
//! ```compile_fail
//! use std::cell::Cell;
//!
//! fn shorten<'a>(c: &'a Cell<&'static str>) -> &'a Cell<&'a str> {
//!     c
//! }
//! ```
//!
//! Обратная замена для указателя на функцию невозможна: обработчик только
//! `&'static str` получил бы короткую ссылку и мог бы её сохранить.
//!
//! ```compile_fail
//! fn widen<'a>(f: fn(&'static str) -> usize) -> fn(&'a str) -> usize {
//!     f
//! }
//! ```
//!
//! Параметр времени жизни, который не встречается в полях, запрещён:
//! компилятор не знает его вариантности.
//!
//! ```compile_fail,E0392
//! struct DmaWindow<'a> {
//!     ptr: core::ptr::NonNull<u8>,
//!     len: usize,
//! }
//! ```
//!
//! Пока окно DMA живо, буфер нельзя ни освободить, ни прочитать:
//!
//! ```compile_fail,E0505
//! use ch05::variance::DmaWindow;
//!
//! let mut rx = vec![0u8; 16];
//! let w = DmaWindow::new(&mut rx);
//! drop(rx); // контроллер ещё пишет в буфер
//! let _ = w.finish();
//! ```

use core::marker::PhantomData;
use core::ptr::NonNull;

// ANCHOR: covariant
/// Порт из конфигурации или порт по умолчанию. Литерал `"uart0"` имеет тип
/// `&'static str` и подходит туда, где ждут `&'a str`: `&'a T` ковариантен
/// по `'a`, длинная ссылка годится вместо короткой.
pub fn port_or_default<'a>(cfg: Option<&'a str>) -> &'a str {
    cfg.unwrap_or("uart0")
}
// ANCHOR_END: covariant

// ANCHOR: invariant
/// Записывает ссылку в ячейку. Через `&mut &'a str` кладётся ссылка с
/// временем жизни `'a`, поэтому `'a` должно совпасть у обоих аргументов.
pub fn set<'a>(slot: &mut &'a str, v: &'a str) {
    *slot = v;
}
// ANCHOR_END: invariant

// ANCHOR: contra
/// Обработчик строк, принимающий ссылку любой длины жизни, годится туда,
/// где передают только `&'static str`: `fn(&'a str)` контравариантен по `'a`.
pub fn contra<'a>(f: fn(&'a str) -> usize) -> fn(&'static str) -> usize {
    f
}
// ANCHOR_END: contra

// ANCHOR: phantom
/// Буфер, отданный каналу DMA. В регистры контроллера уходят адрес и длина,
/// ссылки в полях нет. `PhantomData<&'a mut [u8]>` сообщает компилятору,
/// что окно держит буфер так же, как держала бы `&'a mut [u8]`.
pub struct DmaWindow<'a> {
    ptr: NonNull<u8>,
    len: usize,
    _buf: PhantomData<&'a mut [u8]>,
}

impl<'a> DmaWindow<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        DmaWindow { len: buf.len(), ptr: NonNull::from(buf).cast(), _buf: PhantomData }
    }

    /// Значение для регистра адреса памяти канала.
    pub fn addr(&self) -> usize {
        self.ptr.as_ptr() as usize
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Передача закончена: буфер возвращается программе на тот же срок `'a`.
    pub fn finish(self) -> &'a mut [u8] {
        // SAFETY: окно создано из `&'a mut [u8]` и до своего уничтожения
        // держит это заимствование (поле `_buf`), других путей к буферу нет.
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}
// ANCHOR_END: phantom

/// Модель контроллера: пишет принятые байты по адресу из окна, как писал
/// бы канал DMA в память.
#[cfg(test)]
fn dma_receive(w: &DmaWindow<'_>, data: &[u8]) {
    let n = data.len().min(w.len);
    // SAFETY: окно описывает `w.len` байт уникально заимствованного буфера.
    unsafe { core::ptr::copy_nonoverlapping(data.as_ptr(), w.ptr.as_ptr(), n) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;
    use alloc::vec;

    #[test]
    fn covariance() {
        let cfg = String::from("uart2");
        assert_eq!(port_or_default(Some(&cfg)), "uart2");
        assert_eq!(port_or_default(None), "uart0");
    }

    #[test]
    fn invariance() {
        let mut name: &str = "по умолчанию";
        let local = String::from("из конфигурации");
        set(&mut name, &local); // `name` здесь сама локальная: срок общий
        assert_eq!(name, "из конфигурации");
    }

    #[test]
    fn contravariance() {
        fn count(s: &str) -> usize {
            s.chars().count()
        }
        let f: fn(&'static str) -> usize = contra(count);
        assert_eq!(f("uart1"), 5);
    }

    #[test]
    fn dma_window() {
        let mut rx = vec![0u8; 8];
        let addr = rx.as_ptr() as usize;
        let w = DmaWindow::new(&mut rx);
        assert_eq!((w.addr(), w.len(), w.is_empty()), (addr, 8, false));
        dma_receive(&w, b"\x01\x02T=21");
        let buf = w.finish();
        buf[0] = 0xAA;
        assert_eq!(rx, b"\xAA\x02T=21\0\0");
    }
}
