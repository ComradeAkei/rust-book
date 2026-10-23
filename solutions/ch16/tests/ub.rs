//! Задачи A, C и D под Miri. Каждый тест здесь либо содержит неопределённое
//! поведение (A, C), либо зависит от модели алиасинга (D), поэтому все они
//! помечены `#[ignore]`: обычный `cargo test` их не выполняет. Под Miri их
//! гоняют поодиночке — скриптом `code/ch16/ub/run.sh` или вручную:
//!
//! ```text
//! cargo +nightly miri test -p ch16-solutions --test ch16-ub a_bounds -- --ignored
//! ```
//!
//! Для каждого фрагмента A и C Miri обязан сообщить об ошибке. Для D вердикт
//! противоположный в двух моделях: под Stacked Borrows — ошибка, под
//! `-Zmiri-tree-borrows` — тест проходит.
//!
//! Разбор каждого пункта — в приложении E.

use std::hint::black_box;

// ----- Задача A: каталог неопределённого поведения -----

// A1. Чтение за границей среза: `get_unchecked` не проверяет индекс, и
// предусловие (индекс в границах) здесь нарушено.
#[test]
#[ignore = "UB: out-of-bounds; запускать под Miri поодиночке"]
fn a_bounds() {
    // ANCHOR: a_bounds
    let v = [1u8, 2, 3];
    // SAFETY: НЕТ — индекс 5 вне [0, 3). Это и есть UB, его ловит Miri.
    let out = unsafe { *v.get_unchecked(5) };
    // ANCHOR_END: a_bounds
    black_box(out);
}

// A2. Обращение к освобождённой памяти: указатель переживает своё
// выделение.
#[test]
#[ignore = "UB: use-after-free; запускать под Miri поодиночке"]
fn a_use_after_free() {
    // ANCHOR: a_uaf
    let p: *mut i32 = Box::into_raw(Box::new(7));
    // SAFETY: возвращаем Box и роняем его — память освобождена.
    unsafe { drop(Box::from_raw(p)) };
    // SAFETY: НЕТ — p указывает на освобождённую память. UB.
    let v = unsafe { *p };
    // ANCHOR_END: a_uaf
    black_box(v);
}

// A3. Значение, недопустимое для типа: `bool` — это строго 0 или 1, байт 2
// не является корректным bool, и само его появление — UB.
#[test]
#[ignore = "UB: invalid value; запускать под Miri поодиночке"]
#[allow(invalid_value)]
fn a_invalid_bool() {
    // ANCHOR: a_invalid
    let byte: u8 = 2;
    // SAFETY: НЕТ — 2 не является корректным bool. UB уже в этой строке.
    let b: bool = unsafe { *(&byte as *const u8 as *const bool) };
    // ANCHOR_END: a_invalid
    black_box(b);
}

// A4. Нарушение алиасинга: две живые `&mut` на одну ячейку. Это UB в обеих
// моделях — в отличие от задачи D, где модели расходятся.
#[test]
#[ignore = "UB: aliasing; запускать под Miri поодиночке"]
fn a_aliasing() {
    // ANCHOR: a_alias
    let mut x = 0i32;
    let p: *mut i32 = &mut x;
    // SAFETY: НЕТ — a и b обе живы и обе &mut на x. UB.
    let a: &mut i32 = unsafe { &mut *p };
    let b: &mut i32 = unsafe { &mut *p };
    *a = 1;
    *b = 2;
    // ANCHOR_END: a_alias
    black_box(x);
}

// A5. Чтение неинициализированной памяти: `assume_init` обещает, что байты
// заданы, но они не заданы.
#[test]
#[ignore = "UB: uninitialized memory; запускать под Miri поодиночке"]
#[allow(invalid_value, clippy::uninit_assumed_init)]
fn a_uninit() {
    // ANCHOR: a_uninit
    // SAFETY: НЕТ — u32 не инициализирован, а мы обещаем обратное. UB.
    let u: u32 = unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    // ANCHOR_END: a_uninit
    black_box(u);
}

// A6. Доступ по невыровненному адресу: `u32` требует выравнивания 4, а
// адрес смещён на 1 байт.
#[test]
#[ignore = "UB: misaligned access; запускать под Miri поодиночке"]
fn a_misaligned() {
    // ANCHOR: a_align
    let bytes = [0u8; 8];
    // SAFETY: НЕТ — bytes.as_ptr()+1 не кратен 4, а u32 требует кратности.
    let p = unsafe { bytes.as_ptr().add(1) } as *const u32;
    let v = unsafe { *p };
    // ANCHOR_END: a_align
    black_box(v);
}

// ----- Задача C: unsound-API даёт висячую ссылку -----

// ANCHOR: c_unsound
/// ВАРИАНТ С ОШИБКОЙ (не использовать). Сигнатура обещает `'static`, но
/// `s` живёт не вечно: transmute растягивает время жизни на пустом месте.
/// API получается *unsound* — вызывающий, не нарушив ничего, получит
/// висячую ссылку.
fn leak_view(s: &str) -> &'static str {
    // Это не SAFETY, а анти-пример: ничто не гарантирует 'static.
    unsafe { std::mem::transmute::<&str, &'static str>(s) }
}
// ANCHOR_END: c_unsound

#[test]
#[ignore = "UB: use-after-free из unsound-API; запускать под Miri поодиночке"]
fn c_unsound_dangles() {
    // ANCHOR: c_dangle
    let dangling: &'static str = {
        let owned = String::from("i2c");
        leak_view(&owned) // «'static» ссылка внутрь owned
    }; // owned уничтожен здесь, строка освобождена
    // SAFETY: НЕТ — читаем освобождённую память. Честная замена (StrBuf::view)
    // связывает время жизни с буфером и этого не допускает.
    black_box(dangling.len());
    // ANCHOR_END: c_dangle
}

// ----- Задача D: Tree Borrows принимает, Stacked Borrows — нет -----

#[test]
#[ignore = "зависит от модели: SB — ошибка, TB — проходит; под Miri поодиночке"]
fn d_tree_ok_stack_ub() {
    // Под -Zmiri-tree-borrows проходит и возвращает 3; под Stacked Borrows
    // Miri сообщает об ошибке на последней записи через сырой указатель.
    assert_eq!(ch16_solutions::d_models::tree_ok_stack_ub(), 3);
}
