//! Ошибки компиляции задач B–D: снимки trybuild в tests-shared/ch24-ui,
//! общие для заготовок и решений. Файлы обращаются к макросам по имени
//! зависимости `tasks_macros` — у обоих крейтов оно одно. Обновить снимки по
//! решениям: `TRYBUILD=overwrite cargo test -p ch24-solutions --test ui`.
//!
//! Тесты запускают cargo и закреплённый rustc, поэтому их имена начинаются с
//! `rustc_`, а под Miri они не запускаются.

fn cases(glob: &str) {
    let t = trybuild::TestCases::new();
    t.compile_fail(format!("../../tests-shared/ch24-ui/{glob}"));
}

#[test]
#[cfg_attr(miri, ignore)]
fn rustc_b_errors() {
    cases("b_*.rs");
}

#[test]
#[cfg_attr(miri, ignore)]
fn rustc_c_errors() {
    cases("c_*.rs");
}

#[test]
#[cfg_attr(miri, ignore)]
fn rustc_d_errors() {
    cases("d_*.rs");
}
