//! Задача C: кольцевой буфер за непрозрачным описателем для C. Rust держит
//! структуру и логику FIFO; наружу торчат только функции с C-ABI, которые
//! принимают и возвращают `RingBuffer *` как неполный (opaque) тип. Владение
//! отдаётся через `Box::into_raw` в `rb_new` и возвращается `from_raw` в
//! `rb_free` — ровно один раз. Заголовок для C — `c/ring_buffer.h` (его
//! сгенерировал бы `cbindgen`), драйвер и проверка под ASan — `c/run.sh`.

use core::ffi::c_int;

// ANCHOR: ring
/// Кольцевой буфер фиксированной ёмкости (FIFO).
pub struct RingBuffer {
    buf: Vec<i32>,
    head: usize,
    len: usize,
}

impl RingBuffer {
    /// Ёмкость не меньше 1, чтобы остаток по модулю был определён.
    pub fn with_capacity(cap: usize) -> Self {
        RingBuffer { buf: vec![0; cap.max(1)], head: 0, len: 0 }
    }

    /// Положить значение в хвост; `false`, если буфер полон.
    pub fn push(&mut self, v: i32) -> bool {
        if self.len == self.buf.len() {
            return false;
        }
        let tail = (self.head + self.len) % self.buf.len();
        self.buf[tail] = v;
        self.len += 1;
        true
    }

    /// Снять значение с головы; `None`, если буфер пуст.
    pub fn pop(&mut self) -> Option<i32> {
        if self.len == 0 {
            return None;
        }
        let v = self.buf[self.head];
        self.head = (self.head + 1) % self.buf.len();
        self.len -= 1;
        Some(v)
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}
// ANCHOR_END: ring

// ANCHOR: handle
/// Создать буфер и вернуть непрозрачный описатель (владение — у вызывающего,
/// пока он не позовёт `rb_free`).
#[unsafe(no_mangle)]
pub extern "C" fn rb_new(cap: usize) -> *mut RingBuffer {
    Box::into_raw(Box::new(RingBuffer::with_capacity(cap)))
}

/// Положить значение: 1 — успех, 0 — буфер полон.
///
/// # Safety
///
/// `h` — описатель из `rb_new`, ещё не освобождённый.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_push(h: *mut RingBuffer, v: c_int) -> c_int {
    // SAFETY: по предусловию h — живой описатель из rb_new.
    let rb = unsafe { &mut *h };
    c_int::from(rb.push(v))
}

/// Снять значение в `*out`: 1 — успех, 0 — буфер пуст.
///
/// # Safety
///
/// `h` — описатель из `rb_new`; `out` — валидный указатель на один `c_int`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_pop(h: *mut RingBuffer, out: *mut c_int) -> c_int {
    // SAFETY: h — живой описатель, out валиден по предусловию.
    let rb = unsafe { &mut *h };
    match rb.pop() {
        Some(v) => {
            // SAFETY: out валиден и выровнен по предусловию.
            unsafe { *out = v };
            1
        }
        None => 0,
    }
}

/// Текущее число элементов.
///
/// # Safety
///
/// `h` — описатель из `rb_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_len(h: *const RingBuffer) -> usize {
    // SAFETY: h — живой описатель из rb_new.
    unsafe { &*h }.len()
}

/// Освободить буфер; после вызова `h` недействителен.
///
/// # Safety
///
/// `h` — описатель из `rb_new`, ещё не освобождённый.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rb_free(h: *mut RingBuffer) {
    if !h.is_null() {
        // SAFETY: h — указатель из into_raw; возвращаем владение Box и роняем.
        drop(unsafe { Box::from_raw(h) });
    }
}
// ANCHOR_END: handle
