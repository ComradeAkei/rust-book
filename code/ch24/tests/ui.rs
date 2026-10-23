//! 24.5: ошибки компиляции, которые выдают макросы главы, — снимками
//! trybuild. Каждый файл tests/ui/*.rs компилируется отдельным крейтом, и
//! сообщения компилятора сравниваются с соседним .stderr. Обновить снимки:
//! `TRYBUILD=overwrite cargo test -p ch24 --test ui`.
//!
//! Тест запускает cargo и закреплённый rustc 1.97, поэтому его имя
//! начинается с `rustc_` (задание раскладки в CI его пропускает), а под
//! Miri он не запускается.

// ANCHOR: ui
#[test]
#[cfg_attr(miri, ignore)]
fn rustc_ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
// ANCHOR_END: ui
