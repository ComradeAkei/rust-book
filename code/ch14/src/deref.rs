//! Раздел 14.2: `Deref` и `DerefMut`.
//!
//! Свой умный указатель — теневая копия регистров, которая знает, менялась
//! ли она; разыменование и автоматическое разыменование в вызовах
//! методов; приведения `&Box<String>` и `&Rc<String>` к `&str`.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use core::ops::{Deref, DerefMut};

// ANCHOR: shadow
/// Теневая копия регистров периферии в ОЗУ. Читать её можно как само
/// значение, а любое изменяемое заимствование помечает копию грязной, и
/// `flush` пишет в устройство, только если было что писать.
pub struct Shadow<T> {
    value: T,
    dirty: bool,
}

impl<T> Shadow<T> {
    pub const fn new(value: T) -> Self {
        Self { value, dirty: false }
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Отдаёт значение функции `write`, если оно менялось с прошлого раза.
    pub fn flush(&mut self, write: impl FnOnce(&T)) -> bool {
        let dirty = core::mem::replace(&mut self.dirty, false);
        if dirty {
            write(&self.value);
        }
        dirty
    }
}

impl<T> Deref for Shadow<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Shadow<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.dirty = true;
        &mut self.value
    }
}
// ANCHOR_END: shadow

// ANCHOR: uart
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Uart {
    pub baud: u32,
    pub parity: bool,
}

impl Uart {
    /// Делитель тактовой частоты для 16-кратной передискретизации.
    pub fn divisor(&self, clock_hz: u32) -> u32 {
        clock_hz / (16 * self.baud)
    }

    pub fn set_baud(&mut self, baud: u32) {
        self.baud = baud;
    }
}
// ANCHOR_END: uart

// ANCHOR: use_shadow
/// Сколько раз пришлось писать в устройство за четыре шага.
pub fn shadow_writes() -> [bool; 4] {
    let mut uart = Shadow::new(Uart { baud: 115_200, parity: false });
    let mut writes = [false; 4];
    let flush = |u: &mut Shadow<Uart>| u.flush(|_regs| { /* запись в регистры */ });

    let _div = uart.divisor(16_000_000); // &self: Deref, копия чистая
    writes[0] = flush(&mut uart);
    uart.parity = true; // поле через DerefMut
    writes[1] = flush(&mut uart);
    uart.set_baud(9600); // &mut self: DerefMut
    writes[2] = flush(&mut uart);
    let _same = &mut *uart; // заимствование без записи — всё равно грязно
    writes[3] = flush(&mut uart);
    writes
}
// ANCHOR_END: use_shadow

// ANCHOR: coerce
pub fn name_len(s: &str) -> usize {
    s.len()
}

/// Каждый вызов приводит ссылку к `&str` цепочкой `deref`.
pub fn coercions() -> [usize; 4] {
    let owned = String::from("adc0");
    let boxed: Box<String> = Box::new("adc12".into());
    let shared: Rc<String> = Rc::new("temp".into());
    let shadow = Shadow::new(String::from("pressure"));
    [
        name_len(&owned),  // &String → &str
        name_len(&boxed),  // &Box<String> → &String → &str
        name_len(&shared), // &Rc<String> → &String → &str
        name_len(&shadow), // &Shadow<String> → &String → &str
    ]
}
// ANCHOR_END: coerce

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shadow_marks_every_mutable_borrow() {
        assert_eq!(shadow_writes(), [false, true, true, true]);
    }

    #[test]
    fn shadow_reads_like_the_value() {
        let u = Shadow::new(Uart { baud: 115_200, parity: false });
        assert_eq!(u.divisor(16_000_000), 8);
        assert_eq!(*u, Uart { baud: 115_200, parity: false });
        assert!(!u.is_dirty());
    }

    #[test]
    fn coercions_reach_str() {
        assert_eq!(coercions(), [4, 5, 4, 8]);
    }
}
