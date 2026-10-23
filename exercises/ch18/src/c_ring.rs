//! Задача C: кольцевой буфер за непрозрачным описателем для C. Условие —
//! раздел 18.7. Логику FIFO проверит общий тест, экспорт в C — драйвер на C
//! (см. `solutions/ch18/c/`, направьте его на свой крейт).

use std::ffi::c_int;

// ANCHOR: skeleton
/// Кольцевой буфер фиксированной ёмкости (FIFO).
#[allow(dead_code)]
pub struct RingBuffer {
    buf: Vec<i32>,
    head: usize,
    len: usize,
}

impl RingBuffer {
    pub fn with_capacity(cap: usize) -> Self {
        let _ = cap;
        todo!("раздел 18.7, задача C")
    }

    /// Положить в хвост; `false`, если полон.
    pub fn push(&mut self, v: i32) -> bool {
        let _ = v;
        todo!("раздел 18.7, задача C")
    }

    /// Снять с головы; `None`, если пуст.
    pub fn pop(&mut self) -> Option<i32> {
        todo!("раздел 18.7, задача C")
    }

    pub fn len(&self) -> usize {
        todo!("раздел 18.7, задача C")
    }

    pub fn is_empty(&self) -> bool {
        todo!("раздел 18.7, задача C")
    }
}

// Экспорт в C: описатель — указатель на RingBuffer, отданный `Box::into_raw`
// в rb_new и возвращаемый `Box::from_raw` в rb_free (ровно один раз).

/// Создать буфер и вернуть непрозрачный описатель.
#[unsafe(no_mangle)]
pub extern "C" fn rb_new(cap: usize) -> *mut RingBuffer {
    let _ = cap;
    todo!("раздел 18.7, задача C")
}

/// Положить значение: 1 — успех, 0 — полон.
///
/// # Safety
///
/// `h` — описатель из `rb_new`, не освобождённый.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_push(h: *mut RingBuffer, v: c_int) -> c_int {
    let _ = (h, v);
    todo!("раздел 18.7, задача C")
}

/// Снять значение в `*out`: 1 — успех, 0 — пуст.
///
/// # Safety
///
/// `h` — описатель из `rb_new`; `out` — валидный указатель на один `c_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_pop(h: *mut RingBuffer, out: *mut c_int) -> c_int {
    let _ = (h, out);
    todo!("раздел 18.7, задача C")
}

/// Текущее число элементов.
///
/// # Safety
///
/// `h` — описатель из `rb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_len(h: *const RingBuffer) -> usize {
    let _ = h;
    todo!("раздел 18.7, задача C")
}

/// Освободить буфер; после вызова `h` недействителен.
///
/// # Safety
///
/// `h` — описатель из `rb_new`, не освобождённый.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_free(h: *mut RingBuffer) {
    let _ = h;
    todo!("раздел 18.7, задача C")
}
// ANCHOR_END: skeleton
