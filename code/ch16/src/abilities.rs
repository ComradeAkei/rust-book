//! Раздел 16.1: пять возможностей, которые даёт `unsafe`, и ничего сверх
//! них. `unsafe` не выключает проверки заимствования и не разрешает гонки
//! — он лишь позволяет: разыменовать сырой указатель, вызвать `unsafe fn`,
//! обратиться к `static mut`, реализовать `unsafe trait` и прочитать поле
//! `union`. Каждая возможность — здесь, в наименьшем виде.
//!
//! Всё это запрещено в безопасном коде, и компилятор это ловит. Разыменовать
//! сырой указатель:
//!
//! ```compile_fail,E0133
//! let x = 5;
//! let p = &x as *const i32;
//! let _v = *p; // dereference of raw pointer is unsafe
//! ```
//!
//! Вызвать `unsafe fn` без `unsafe`:
//!
//! ```compile_fail,E0133
//! unsafe fn danger() -> u32 { 1 }
//! let _ = danger(); // call to unsafe function is unsafe
//! ```
//!
//! В edition 2024 ссылку на `static mut` нельзя взять даже внутри `unsafe` —
//! поэтому к счётчику ниже обращаются через `&raw mut`:
//!
//! ```compile_fail
//! static mut C: u32 = 0;
//! let _r = unsafe { &C }; // creating a shared reference to mutable static
//! ```

// ANCHOR: raw
/// 1. Разыменование сырого указателя. Создать `*const`/`*mut` и считать
///    его адрес можно в безопасном коде; разыменовать — только в `unsafe`.
pub fn deref_raw(v: &[i32]) -> Option<i32> {
    let p: *const i32 = v.as_ptr();
    if v.is_empty() {
        return None;
    }
    // SAFETY: p получен из непустого среза, адрес первого элемента жив и
    // выровнен, значение инициализировано.
    Some(unsafe { *p })
}
// ANCHOR_END: raw

// ANCHOR: unsafe_fn
/// 2. Вызов `unsafe fn`. Сама функция обещает корректность только при
///    соблюдении предусловия, записанного в её документации.
///
/// # Safety
///
/// `i < slice.len()`.
pub unsafe fn get_unchecked(slice: &[i32], i: usize) -> i32 {
    debug_assert!(i < slice.len());
    // SAFETY: предусловие функции — i в границах; переложено на вызывающего.
    unsafe { *slice.get_unchecked(i) }
}

/// Безопасная обёртка: проверяет предусловие и только тогда зовёт.
pub fn second(slice: &[i32]) -> Option<i32> {
    if slice.len() > 1 {
        // SAFETY: длина больше 1, значит индекс 1 в границах.
        Some(unsafe { get_unchecked(slice, 1) })
    } else {
        None
    }
}
// ANCHOR_END: unsafe_fn

// ANCHOR: static_mut
/// 3. Доступ к `static mut`. В edition 2024 ссылка на `static mut`
///    запрещена (это слишком легко превращается в две `&mut`); берут сырой
///    указатель через `&raw mut` и работают через него.
static mut COUNTER: u32 = 0;

/// Увеличить счётчик. Вызывать только из одного потока: общего `static mut`
/// без синхронизации хватает на счётчик прерываний, но не на гонку.
///
/// # Safety
///
/// Нет одновременного доступа к `COUNTER` из другого потока или прерывания.
pub unsafe fn bump_counter() -> u32 {
    // SAFETY: &raw mut не создаёт ссылки; одновременного доступа нет по
    // предусловию. Читаем и пишем через указатель.
    unsafe {
        let p = &raw mut COUNTER;
        *p += 1;
        *p
    }
}
// ANCHOR_END: static_mut

// ANCHOR: unsafe_trait
/// 4. Реализация `unsafe trait`. Трейт объявлен `unsafe`, потому что от
///    корректности его реализации зависит безопасность чужого кода. Здесь —
///    маркер «тип можно побайтно обнулить».
///
/// # Safety
///
/// Реализатор обещает, что все нули — корректное значение типа.
pub unsafe trait Zeroable: Sized {
    fn zeroed() -> Self {
        // SAFETY: реализатор пообещал, что нули — корректное значение.
        unsafe { core::mem::zeroed() }
    }
}

// SAFETY: для целых и их массивов нули — корректное значение.
unsafe impl Zeroable for u32 {}
// SAFETY: нуль корректен для каждого элемента, значит и для массива.
unsafe impl<T: Zeroable, const N: usize> Zeroable for [T; N] {}
// ANCHOR_END: unsafe_trait

// ANCHOR: union
/// 5. Чтение поля `union`. Все поля лежат по одному адресу; компилятор не
///    знает, какое из них записано последним, поэтому чтение — `unsafe`.
#[repr(C)]
pub union Reg {
    pub word: u32,
    pub bytes: [u8; 4],
}

/// Разложить слово на байты через union (как объединение в C).
pub fn word_bytes(word: u32) -> [u8; 4] {
    let r = Reg { word };
    // SAFETY: записали word, читаем bytes того же размера; любое битовое
    // представление u32 — корректный [u8; 4].
    unsafe { r.bytes }
}
// ANCHOR_END: union

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_abilities() {
        assert_eq!(deref_raw(&[7, 8]), Some(7));
        assert_eq!(deref_raw(&[]), None);
        assert_eq!(second(&[1, 2, 3]), Some(2));
        assert_eq!(second(&[1]), None);
        // SAFETY: тест однопоточный, другого доступа к COUNTER нет.
        unsafe {
            assert_eq!(bump_counter(), 1);
            assert_eq!(bump_counter(), 2);
        }
        assert_eq!(<[u32; 4]>::zeroed(), [0; 4]);
        assert_eq!(word_bytes(u32::from_le_bytes([1, 2, 3, 4])), [1, 2, 3, 4]);
    }
}
