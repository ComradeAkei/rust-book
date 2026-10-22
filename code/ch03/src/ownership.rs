//! Раздел 3.1: владение, перемещение, `Copy` и `Clone`.
//!
//! После перемещения исходная переменная недоступна, и это проверяется при
//! компиляции:
//!
//! ```compile_fail,E0382
//! fn send(frame: Vec<u8>) -> usize {
//!     frame.len()
//! }
//!
//! let frame = vec![0xA5; 64];
//! send(frame);
//! send(frame); // второй раз отдать то, чего уже нет
//! ```
//!
//! Тип с деструктором не может быть `Copy`: копия побитово дублировала бы
//! ресурс, и деструктор выполнился бы дважды.
//!
//! ```compile_fail,E0184
//! #[derive(Clone, Copy)]
//! struct Handle(u32);
//!
//! impl Drop for Handle {
//!     fn drop(&mut self) {}
//! }
//! ```
//!
//! Путь ошибки из введения, переписанный буквально: кадр отдан очереди, а
//! потом его пытаются освободить ещё раз.
//!
//! ```compile_fail,E0382
//! use ch03::ownership::{Frame, Queue};
//!
//! let mut q = Queue::default();
//! let f = Frame { id: 1, payload: vec![0x5A; 64] };
//! q.enqueue(f);
//! drop(f);
//! ```
//!
//! `Copy` можно вывести, только если все поля `Copy`:
//!
//! ```compile_fail,E0204
//! #[derive(Clone, Copy)]
//! struct Frame {
//!     id: u16,
//!     payload: Vec<u8>,
//! }
//! ```

use std::string::String;
use std::vec;
use std::vec::Vec;

// ANCHOR: move
/// Отправляет кадр и забирает его себе: после возврата буфер освобождён.
pub fn send(frame: Vec<u8>) -> usize {
    frame.len()
} // здесь вызывается деструктор `frame`: free(буфер)

/// Создаёт кадр и передаёт владение в `send`.
pub fn produce() -> usize {
    let frame = vec![0xA5; 64]; // frame владеет 64 байтами в куче
    let n = send(frame); // владение перешло в send
    // frame.len()       // error[E0382]: borrow of moved value: `frame`
    n
}
// ANCHOR_END: move

// ANCHOR: give_back
/// Функция может вернуть владение: значение снова перемещается, теперь
/// наружу.
pub fn stamp(mut frame: Vec<u8>, seq: u8) -> Vec<u8> {
    frame.insert(0, seq);
    frame
}
// ANCHOR_END: give_back

// ANCHOR: copy
/// Значение `Copy` копируется побитово, и источник остаётся доступен.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub channel: u8,
    pub value: u16,
}

/// `Clone` — явная и, возможно, дорогая копия: здесь это новый буфер в
/// куче и копирование байтов.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub id: u16,
    pub payload: Vec<u8>,
}

pub fn copy_and_clone() -> (Sample, Sample, Frame, Frame) {
    let s = Sample { channel: 3, value: 1200 };
    let t = s; // копия: s по-прежнему доступна
    let f = Frame { id: 7, payload: vec![1, 2, 3] };
    let g = f.clone(); // явная копия: вторая аллокация
    (s, t, f, g)
}
// ANCHOR_END: copy

// ANCHOR: queue
/// Очередь отправки владеет кадрами, которые в неё положили.
#[derive(Default)]
pub struct Queue {
    frames: Vec<Frame>,
}

#[derive(Debug, PartialEq)]
pub struct LinkDown;

impl Queue {
    /// Кадр перемещается в очередь: у вызывающего его больше нет.
    pub fn enqueue(&mut self, f: Frame) {
        self.frames.push(f);
    }

    /// Отправить всё и освободить: кадры уничтожаются вместе с вектором.
    pub fn flush(&mut self) -> Vec<(usize, u8)> {
        self.frames.drain(..).map(|f| (f.payload.len(), f.payload[0])).collect()
    }
}

pub fn produce_into(q: &mut Queue, seq: u8, link_up: bool) -> Result<(), LinkDown> {
    let f = Frame { id: seq as u16, payload: vec![seq; 64] };
    q.enqueue(f); // владение передано очереди
    if !link_up {
        return Err(LinkDown); // освобождать нечего: f здесь уже нет
    }
    Ok(())
}
// ANCHOR_END: queue

// ANCHOR: string_parts
/// Из чего состоит `String`: указатель на буфер в куче, ёмкость, длина.
/// Перемещение копирует эти три слова, но не сами байты.
pub fn parts(s: &String) -> (*const u8, usize, usize) {
    (s.as_ptr(), s.capacity(), s.len())
}
// ANCHOR_END: string_parts

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_into_and_out() {
        assert_eq!(produce(), 64);
        assert_eq!(stamp(vec![1, 2], 9), [9, 1, 2]);
    }

    #[test]
    fn queue_owns_frames() {
        let mut q = Queue::default();
        assert_eq!(produce_into(&mut q, 0xA5, true), Ok(()));
        assert_eq!(produce_into(&mut q, 0x5A, false), Err(LinkDown));
        assert_eq!(q.flush(), [(64, 0xA5), (64, 0x5A)]);
        assert!(q.flush().is_empty());
    }

    #[test]
    fn copy_keeps_source() {
        let (s, t, f, g) = copy_and_clone();
        assert_eq!(s, t);
        assert_eq!(f, g);
        assert_ne!(f.payload.as_ptr(), g.payload.as_ptr());
    }

    // ANCHOR: move_is_shallow
    #[test]
    fn move_copies_three_words() {
        let a = String::from("телеметрия");
        let before = parts(&a);
        let b = a; // перемещение
        assert_eq!(parts(&b), before); // тот же буфер, та же ёмкость и длина
        assert_eq!(size_of::<String>(), 3 * size_of::<usize>());
    }
    // ANCHOR_END: move_is_shallow
}
