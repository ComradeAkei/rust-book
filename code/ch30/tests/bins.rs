//! Программы главы: вывод, который приводит текст. Времена зависят от
//! машины и проверяются с большим запасом (группа `observed_`); числа,
//! которые от машины не зависят, — точно.

mod common;

use std::process::Command;
use std::sync::Mutex;

/// Программы меряют время — по одной за раз, чтобы не мешать друг другу.
fn run(name: &str, args: &[&str]) -> String {
    static SERIAL: Mutex<()> = Mutex::new(());
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let out = Command::new(common::release_bin(name)).args(args).output().unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

/// Число после `prefix` в строке вывода.
fn number_after(text: &str, prefix: &str) -> f64 {
    let at = text.find(prefix).unwrap_or_else(|| panic!("{prefix}: {text}")) + prefix.len();
    let rest = text[at..].trim_start();
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
    rest[..end].parse().unwrap_or_else(|_| panic!("{prefix}: {rest}"))
}

#[test]
fn timing_in_debug_names_the_profile() {
    let out = Command::new(env!("CARGO_BIN_EXE_timing")).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("сборка: debug\n"), "{text}");
}

#[test]
fn observed_timing_pitfalls_in_release() {
    let text = run("timing", &[]);
    assert!(text.starts_with("сборка: release\n"), "{text}");
    // Выброшенный результат: цикла нет, на вызов — доли наносекунды.
    assert!(number_after(&text, "результат выброшен:") < 0.5, "{text}");
    assert!(number_after(&text, "результат сохранён:") > 500.0, "{text}");
    // Сумма до миллиарда стоит столько же, сколько до тысячи: формула.
    let small = number_after(&text, "sum_to(black_box(1000)):");
    let big = number_after(&text, "sum_to(black_box(1000000000)):");
    assert!(big < 10.0 && big < small * 5.0, "{text}");
}

#[test]
fn allocations_are_counted_exactly() {
    let text = run("memory", &["alloc"]);
    // format! — новая строка и её рост: два выделения на строку журнала;
    // плюс 19 удвоений общего буфера, как у write!.
    assert!(text.contains("выделений 200019, 1412458 байт"), "{text}");
    assert!(text.contains("  write!: ") && text.contains("выделений 19, 1412458 байт"), "{text}");
}

#[test]
fn observed_memory_layout_and_sharing() {
    let text = run("memory", &["layout"]);
    let (aos, soa) = (number_after(&text, "AoS:"), number_after(&text, "SoA:"));
    assert!(aos > 3.0 * soa, "{text}");
    if std::thread::available_parallelism().map_or(1, |n| n.get()) >= 2 {
        let text = run("memory", &["sharing"]);
        let near = number_after(&text, "в одной строке:");
        let apart = number_after(&text, "в разных:");
        assert!(near > 1.5 * apart, "{text}");
    }
}

#[test]
fn observed_matrix_cols_slower() {
    let rows = number_after(&run("matrix", &["rows", "2000"]), "сумма 11999994,");
    let cols = number_after(&run("matrix", &["cols", "2000"]), "сумма 11999994,");
    assert!(cols > 2.0 * rows, "{rows} {cols}");
}

#[test]
fn observed_branches_random_vs_sorted() {
    let text = run("branches", &[]);
    let random = number_after(&text, "random    branchy:");
    let sorted = number_after(&text, "sorted    branchy:");
    let branchless = number_after(&text, "random branchless:");
    assert!(random > 3.0 * sorted && random > 3.0 * branchless, "{text}");
}

#[test]
fn observed_station_speedups() {
    let text = run("station", &["262144"]);
    let store = number_after(&text, "store: медиана");
    let hash = number_after(&text, " hash: медиана");
    let x2 = number_after(&text, "   x2: медиана");
    assert!(hash < store && x2 < 0.8 * hash, "{text}");
}

#[test]
fn observed_bounds_checks_cost_little() {
    let text = run("bounds", &[]);
    let slice = number_after(&text, "hist_slice:");
    let array = number_after(&text, "hist_array:");
    // Проверка предсказуема: дороже, но не в разы.
    assert!(slice < 2.0 * array, "{text}");
    // Проверка переполнения стоит векторизации.
    let wrapping = number_after(&text, "sum_wrapping:");
    let strict = number_after(&text, "sum_strict:");
    assert!(strict > 1.5 * wrapping, "{text}");
}
