//! Задача D: свои `memcpy`, `memmove`, `memset` и функции ARM EABI.
//!
//! На голом железе (`target_os = "none"`) функции экспортируются под
//! своими именами С и заменяют слабые определения из compiler_builtins.
//! На хосте это обычные функции Rust: экспорт под именем `memcpy` в
//! тестовой программе подменил бы `memcpy` из glibc для всего процесса.
//!
//! Крейт собран с `#![no_builtins]`: без этого LLVM узнал бы в циклах
//! ниже идиомы и заменил бы их вызовами — в `__aeabi_memset` вызовом
//! самой себя (`solutions/ch26/tests/naive.rs`).

#[cfg(feature = "count")]
pub mod calls {
    //! Счётчики вызовов для прошивки ch26-tasks. На Cortex-M0 нет
    //! атомарного `fetch_add`, поэтому счёт — `load` и `store`: точен,
    //! пока функции не вызываются из прерываний.
    use core::sync::atomic::{AtomicU32, Ordering::Relaxed};

    pub static MEMCPY: AtomicU32 = AtomicU32::new(0);
    pub static MEMMOVE: AtomicU32 = AtomicU32::new(0);
    pub static MEMSET: AtomicU32 = AtomicU32::new(0);

    pub(super) fn bump(c: &AtomicU32) {
        c.store(c.load(Relaxed).wrapping_add(1), Relaxed);
    }
}

const WORD: usize = size_of::<u32>();

fn aligned(p: usize) -> bool {
    p % WORD == 0
}

// ANCHOR: memcpy
/// `memcpy`: словами, если оба адреса выровнены, иначе и в хвосте —
/// байтами.
///
/// # Safety
///
/// Области `[dest, dest + n)` и `[src, src + n)` допустимы и не
/// перекрываются.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    #[cfg(feature = "count")]
    calls::bump(&calls::MEMCPY);
    let mut i = 0;
    if aligned(dest as usize) && aligned(src as usize) {
        while i + WORD <= n {
            // SAFETY: оба адреса выровнены на 4, слово — внутри областей.
            unsafe { dest.add(i).cast::<u32>().write(src.add(i).cast::<u32>().read()) };
            i += WORD;
        }
    }
    while i < n {
        // SAFETY: i < n — внутри областей.
        unsafe { *dest.add(i) = *src.add(i) };
        i += 1;
    }
    dest
}
// ANCHOR_END: memcpy

// ANCHOR: memmove
/// `memmove`: области могут перекрываться. Если приёмник ниже источника,
/// прямой порядок не затирает непрочитанное, иначе копируем с конца.
///
/// # Safety
///
/// Области `[dest, dest + n)` и `[src, src + n)` допустимы.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    #[cfg(feature = "count")]
    calls::bump(&calls::MEMMOVE);
    if (dest as usize) <= (src as usize) {
        let mut i = 0;
        while i < n {
            // SAFETY: i < n; запись в dest[i] не трогает src[j] при j > i.
            unsafe { *dest.add(i) = *src.add(i) };
            i += 1;
        }
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            // SAFETY: i < n; запись в dest[i] не трогает src[j] при j < i.
            unsafe { *dest.add(i) = *src.add(i) };
        }
    }
    dest
}
// ANCHOR_END: memmove

// ANCHOR: memset
/// `memset`: байт `c` (от `int` берётся младший байт), словами по
/// выровненному адресу.
///
/// # Safety
///
/// Область `[dest, dest + n)` допустима.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    #[cfg(feature = "count")]
    calls::bump(&calls::MEMSET);
    let b = c as u8;
    let mut i = 0;
    if aligned(dest as usize) {
        let w = u32::from(b) * 0x0101_0101;
        while i + WORD <= n {
            // SAFETY: адрес выровнен, слово внутри области.
            unsafe { dest.add(i).cast::<u32>().write(w) };
            i += WORD;
        }
    }
    while i < n {
        // SAFETY: i < n.
        unsafe { *dest.add(i) = b };
        i += 1;
    }
    dest
}
// ANCHOR_END: memset

// ANCHOR: aeabi
// Функции ARM EABI (RTABI, раздел 4.3.4): компилятор под Cortex-M зовёт
// их вместо memcpy и memset. Суффикс 4 и 8 обещает выравнивание адресов;
// здесь его проверяет сама memcpy. У __aeabi_memset другой порядок
// аргументов, чем у memset: длина вторая, значение третье.

/// # Safety
///
/// Как у [`memcpy`].
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memcpy(dest: *mut u8, src: *const u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memcpy(dest, src, n) };
}

/// # Safety
///
/// Как у [`memcpy`], адреса выровнены на 4.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memcpy4(dest: *mut u8, src: *const u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memcpy(dest, src, n) };
}

/// # Safety
///
/// Как у [`memcpy`], адреса выровнены на 8.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memcpy8(dest: *mut u8, src: *const u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memcpy(dest, src, n) };
}

/// # Safety
///
/// Как у [`memmove`].
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memmove(dest: *mut u8, src: *const u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memmove(dest, src, n) };
}

/// # Safety
///
/// Как у [`memmove`], адреса выровнены на 4.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memmove4(dest: *mut u8, src: *const u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memmove(dest, src, n) };
}

/// # Safety
///
/// Как у [`memmove`], адреса выровнены на 8.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memmove8(dest: *mut u8, src: *const u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memmove(dest, src, n) };
}

/// # Safety
///
/// Как у [`memset`]. Порядок аргументов: адрес, длина, значение.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memset(dest: *mut u8, n: usize, c: i32) {
    // SAFETY: договор тот же.
    unsafe { memset(dest, c, n) };
}

/// # Safety
///
/// Как у [`__aeabi_memset`], адрес выровнен на 4.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memset4(dest: *mut u8, n: usize, c: i32) {
    // SAFETY: договор тот же.
    unsafe { memset(dest, c, n) };
}

/// # Safety
///
/// Как у [`__aeabi_memset`], адрес выровнен на 8.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memset8(dest: *mut u8, n: usize, c: i32) {
    // SAFETY: договор тот же.
    unsafe { memset(dest, c, n) };
}

/// # Safety
///
/// Область `[dest, dest + n)` допустима.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memclr(dest: *mut u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memset(dest, 0, n) };
}

/// # Safety
///
/// Как у [`__aeabi_memclr`], адрес выровнен на 4.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memclr4(dest: *mut u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memset(dest, 0, n) };
}

/// # Safety
///
/// Как у [`__aeabi_memclr`], адрес выровнен на 8.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memclr8(dest: *mut u8, n: usize) {
    // SAFETY: договор тот же.
    unsafe { memset(dest, 0, n) };
}
// ANCHOR_END: aeabi
