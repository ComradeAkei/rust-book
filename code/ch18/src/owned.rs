//! Раздел 18.4: владение через границу. У C нет ни `Drop`, ни времён жизни:
//! договор «кто выделил, тот и освобождает» держится только на дисциплине.
//! `Box::into_raw` снимает ответственность за освобождение с Rust и отдаёт
//! сырой указатель; `Box::from_raw` возвращает её ровно один раз. На этом
//! строят и передачу владения в C, и экспорт Rust-объекта как непрозрачного
//! описателя (opaque handle), который C держит, но внутрь не заглядывает.

use core::ffi::c_void;

unsafe extern "C" {
    fn c_hold(p: *mut c_void) -> *mut c_void;
}

// ANCHOR: roundtrip
/// Отдать владение Rust-значением «в C» и забрать обратно. Пока указатель у
/// C, Rust про него забыл: утечки нет только потому, что мы заберём его
/// назад и восстановим `Box`.
pub fn roundtrip(value: String) -> String {
    let raw = Box::into_raw(Box::new(value)) as *mut c_void;
    // SAFETY: c_hold лишь хранит и возвращает тот же указатель, не
    // разыменовывая и ничего не освобождая, — raw по-прежнему наш живой Box.
    let back = unsafe { c_hold(raw) } as *mut String;
    // SAFETY: back — тот самый указатель из into_raw; владение возвращаем
    // один раз, значение забираем и роняем Box.
    *unsafe { Box::from_raw(back) }
}
// ANCHOR_END: roundtrip

// ANCHOR: export
/// Экспорт в обратную сторону: функции с C-ABI и неискажёнными именами,
/// которые вызывает C. Непрозрачный описатель — это указатель на Rust-значение,
/// отданное через `into_raw`; C видит лишь `void *` и не знает его устройства.
/// Создаёт счётчик и возвращает описатель.
#[unsafe(no_mangle)]
pub extern "C" fn ch18_counter_new() -> *mut c_void {
    Box::into_raw(Box::new(0u64)) as *mut c_void
}

/// Увеличить счётчик на 1 и вернуть новое значение.
///
/// # Safety
///
/// `h` — описатель из `ch18_counter_new`, ещё не освобождённый.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ch18_counter_bump(h: *mut c_void) -> u64 {
    // SAFETY: по предусловию h — живой указатель из ch18_counter_new,
    // единственный доступ к счётчику идёт через него.
    let c = unsafe { &mut *(h as *mut u64) };
    *c += 1;
    *c
}

/// Освободить счётчик. После вызова `h` недействителен.
///
/// # Safety
///
/// `h` получен из `ch18_counter_new` и ещё не освобождался.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ch18_counter_free(h: *mut c_void) {
    if !h.is_null() {
        // SAFETY: h — указатель из into_raw; возвращаем владение Box и роняем.
        drop(unsafe { Box::from_raw(h as *mut u64) });
    }
}
// ANCHOR_END: export

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(miri, ignore = "вызывает C")]
    fn roundtrip_preserves_value() {
        let s = roundtrip(String::from("через границу и обратно"));
        assert_eq!(s, "через границу и обратно");
    }
}
