//! Разделы 14.1 и 14.5: неопределённое поведение, которое находит Miri.
//! Тесты помечены `ignore`: без Miri ошибка проходит незамеченной или
//! портит память всего тестового процесса. Запуск по одному:
//!
//! ```console
//! $ cargo +nightly miri test -p ch14 --test ub -- --ignored --exact write_through_shared_ref
//! ```

use ch14::boxes::{Config, from_raw, to_raw};
use std::cell::UnsafeCell;

// ANCHOR: shared_write
#[test]
#[ignore = "UB: запускать под Miri"]
fn write_through_shared_ref() {
    #[inline(never)]
    fn sneaky(p: *const u32) {
        // SAFETY: никакой — это и есть ошибка.
        unsafe { *(p as *mut u32) = 99 }
    }
    let a = 1;
    sneaky(&a);
}
// ANCHOR_END: shared_write

// ANCHOR: unsafecell_ref
#[test]
#[ignore = "UB: запускать под Miri"]
fn write_while_ref_inside_cell() {
    let c = UnsafeCell::new(1);
    // SAFETY: ошибка нарочно: ссылка внутрь живёт, пока значение меняют.
    let r: &i32 = unsafe { &*c.get() };
    unsafe { *c.get() = 2 };
    assert_eq!(*r, 2);
}
// ANCHOR_END: unsafecell_ref

// ANCHOR: double_from_raw
#[test]
#[ignore = "UB: запускать под Miri"]
fn double_from_raw() {
    let p = to_raw(Box::new(Config { name: "adc0".into(), period_ms: 100 }));
    // SAFETY: первый from_raw законен, второй — двойное освобождение.
    drop(unsafe { from_raw(p) });
    drop(unsafe { from_raw(p) });
}
// ANCHOR_END: double_from_raw
