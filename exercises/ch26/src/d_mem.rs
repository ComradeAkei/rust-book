//! Задача D: свои `memcpy`, `memmove`, `memset` и функции ARM EABI.
//!
//! На голом железе (`target_os = "none"`) функции экспортируются под
//! своими именами С и заменяют слабые определения из compiler_builtins.
//! На хосте это обычные функции Rust: экспорт под именем `memcpy` в
//! тестовой программе подменил бы `memcpy` из glibc для всего процесса.

// ANCHOR: skeleton
// ANCHOR: three
/// `memcpy`: скопировать `n` байт, вернуть `dest`.
///
/// # Safety
///
/// Области `[dest, dest + n)` и `[src, src + n)` допустимы и не
/// перекрываются.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let _ = (dest, src, n);
    todo!("задача D: memcpy")
}

/// `memmove`: как `memcpy`, но области могут перекрываться.
///
/// # Safety
///
/// Области `[dest, dest + n)` и `[src, src + n)` допустимы.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let _ = (dest, src, n);
    todo!("задача D: memmove")
}

/// `memset`: заполнить `n` байт младшим байтом `c`, вернуть `dest`.
///
/// # Safety
///
/// Область `[dest, dest + n)` допустима.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    let _ = (dest, c, n);
    todo!("задача D: memset")
}
// ANCHOR_END: three

// TODO(задача D): функции ARM EABI. Компилятор под Cortex-M зовёт их
// вместо memcpy и memset. Обратите внимание на порядок аргументов
// __aeabi_memset: длина вторая, значение третье.

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memcpy(dest: *mut u8, src: *const u8, n: usize) {
    let _ = (dest, src, n);
    todo!("задача D: __aeabi_memcpy")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memcpy4(dest: *mut u8, src: *const u8, n: usize) {
    let _ = (dest, src, n);
    todo!("задача D: __aeabi_memcpy4")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memcpy8(dest: *mut u8, src: *const u8, n: usize) {
    let _ = (dest, src, n);
    todo!("задача D: __aeabi_memcpy8")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memmove(dest: *mut u8, src: *const u8, n: usize) {
    let _ = (dest, src, n);
    todo!("задача D: __aeabi_memmove")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memmove4(dest: *mut u8, src: *const u8, n: usize) {
    let _ = (dest, src, n);
    todo!("задача D: __aeabi_memmove4")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memmove8(dest: *mut u8, src: *const u8, n: usize) {
    let _ = (dest, src, n);
    todo!("задача D: __aeabi_memmove8")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memset(dest: *mut u8, n: usize, c: i32) {
    let _ = (dest, n, c);
    todo!("задача D: __aeabi_memset")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memset4(dest: *mut u8, n: usize, c: i32) {
    let _ = (dest, n, c);
    todo!("задача D: __aeabi_memset4")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memset8(dest: *mut u8, n: usize, c: i32) {
    let _ = (dest, n, c);
    todo!("задача D: __aeabi_memset8")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memclr(dest: *mut u8, n: usize) {
    let _ = (dest, n);
    todo!("задача D: __aeabi_memclr")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memclr4(dest: *mut u8, n: usize) {
    let _ = (dest, n);
    todo!("задача D: __aeabi_memclr4")
}

/// # Safety
///
/// Как у соответствующей функции C; суффикс 4 или 8 обещает выравнивание.
#[cfg_attr(target_os = "none", unsafe(no_mangle))]
pub unsafe extern "C" fn __aeabi_memclr8(dest: *mut u8, n: usize) {
    let _ = (dest, n);
    todo!("задача D: __aeabi_memclr8")
}
// ANCHOR_END: skeleton
