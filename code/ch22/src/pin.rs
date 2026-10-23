//! Раздел 22.3: самоссылающиеся автоматы, `Pin` и `Unpin`.
//!
//! async-блок, одалживающий свою же локальную переменную через `.await`,
//! превращается в автомат, который хранит и переменную, и ссылку на неё —
//! *указатель внутрь самого себя*. Переместить такой автомат в памяти значит
//! оставить ссылку смотреть на старое место. `Pin<&mut T>` — обещание, что
//! `T` больше не сдвинется, пока не будет уничтожен; для типов `!Unpin`
//! безопасный код получить из `Pin` перемещаемый `&mut T` не может.
//!
//! Перемещать закреплённое значение нельзя — компилятор не даст ни
//! вытащить его из `Pin`, ни взять `&mut` (через который работает
//! `mem::swap`):
//!
//! ```compile_fail,E0507
//! use ch22::pin::SelfRef;
//! let p = core::pin::pin!(SelfRef::new(*b"pin!"));
//! let moved: SelfRef = *p; // переместить из Pin нельзя
//! ```
//!
//! ```compile_fail,E0596
//! use ch22::pin::SelfRef;
//! let mut a = core::pin::pin!(SelfRef::new(*b"left"));
//! let mut b = core::pin::pin!(SelfRef::new(*b"rght"));
//! core::mem::swap(&mut *a, &mut *b); // &mut для !Unpin из Pin не взять
//! ```
//!
//! `Pin::new` — безопасный конструктор только для `Unpin`-типов: им
//! закрепление ничего не обещает, их можно двигать и после.
//!
//! ```compile_fail,E0277
//! use ch22::pin::SelfRef;
//! let mut s = SelfRef::new(*b"nope");
//! let p = core::pin::Pin::new(&mut s); // SelfRef: !Unpin
//! ```
//!
//! А async-блок, держащий ссылку на свою переменную через `.await`, сам
//! `!Unpin` — компилятор вывел это из устройства автомата:
//!
//! ```compile_fail,E0277
//! fn needs_unpin<T: Unpin>(_: &T) {}
//! let fut = async {
//!     let v = [1u8, 2, 3];
//!     let r = &v; // ссылка внутрь будущего автомата
//!     ch22::future::yield_now().await;
//!     r.len()
//! };
//! needs_unpin(&fut);
//! ```

use core::marker::PhantomPinned;
use core::pin::Pin;
use core::ptr;

// ANCHOR: selfref
/// Значение с указателем на собственное поле — то же, что хранит автомат
/// async-блока с `let r = &v;` через `.await`. `PhantomPinned` делает тип
/// `!Unpin`: закрепив, его уже не сдвинуть безопасным кодом.
pub struct SelfRef {
    data: [u8; 4],
    ptr: *const u8, // после init смотрит на data[0]
    _pin: PhantomPinned,
}

impl SelfRef {
    pub const fn new(data: [u8; 4]) -> Self {
        SelfRef { data, ptr: ptr::null(), _pin: PhantomPinned }
    }

    /// Завести внутренний указатель. Принимает только закреплённое значение:
    /// после этого вызова сдвигать его нельзя — указатель устареет.
    pub fn init(self: Pin<&mut Self>) {
        // SAFETY: мы не перемещаем *this, а лишь записываем поле ptr.
        let this = unsafe { self.get_unchecked_mut() };
        this.ptr = this.data.as_ptr();
    }

    /// Прочитать первый байт через внутренний указатель. Безопасно, потому
    /// что `Pin` гарантирует: со времени `init` значение не двигалось.
    pub fn first(self: Pin<&Self>) -> u8 {
        assert!(self.points_to_self(), "init не вызван");
        // SAFETY: ptr указывает на self.data, а self закреплён и жив.
        unsafe { *self.ptr }
    }

    /// Смотрит ли указатель на поле *этого* экземпляра. Только сравнение
    /// адресов — разыменования нет, поэтому безопасно и для сдвинутой копии.
    pub fn points_to_self(&self) -> bool {
        ptr::eq(self.ptr, self.data.as_ptr())
    }
}
// ANCHOR_END: selfref

// ANCHOR: unpinned
/// Та же структура без `Pin` и без `PhantomPinned`: её можно завести и
/// переместить, и внутренний указатель останется смотреть на старое место.
pub struct Unpinned {
    pub data: [u8; 4],
    ptr: *const u8,
}

impl Unpinned {
    pub fn new(data: [u8; 4]) -> Self {
        Unpinned { data, ptr: ptr::null() }
    }
    pub fn init(&mut self) {
        self.ptr = self.data.as_ptr();
    }
    pub fn points_to_self(&self) -> bool {
        ptr::eq(self.ptr, self.data.as_ptr())
    }
}

// ANCHOR_END: unpinned

#[cfg(test)]
mod tests {
    use super::*;
    use core::pin::pin;

    #[test]
    fn pinned_self_reference_stays_valid() {
        let mut s = pin!(SelfRef::new(*b"Rust"));
        s.as_mut().init();
        assert!(s.points_to_self());
        assert_eq!(s.as_ref().first(), b'R');
    }

    // ANCHOR: moved
    #[test]
    fn moving_breaks_self_reference() {
        let mut u = Unpinned::new(*b"move");
        u.init();
        assert!(u.points_to_self());
        // Переезд в кучу: значение копируется по новому адресу, а ptr по-
        // прежнему смотрит в старый слот стека. Разыменовать его теперь —
        // неопределённое поведение, поэтому только сравниваем адреса.
        let moved = Box::new(u);
        assert!(!moved.points_to_self());
    }
    // ANCHOR_END: moved

    #[test]
    fn box_pin_pins_on_heap() {
        let mut b = Box::pin(SelfRef::new(*b"heap"));
        b.as_mut().init();
        let before = b.as_ref().get_ref() as *const SelfRef;
        let b2 = b; // перемещается Box (указатель), а не само значение
        assert_eq!(b2.as_ref().get_ref() as *const SelfRef, before);
        assert_eq!(b2.as_ref().first(), b'h');
    }
}
