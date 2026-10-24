//! Программы главы: вывод, который приводит текст. Времена зависят от
//! машины и проверяются с большим запасом (группа `observed_`); числа,
//! которые от машины не зависят, — точно.

mod common;

use std::process::Command;
use std::sync::Mutex;

/// Программы меряют время — по одной за раз, чтобы не мешать друг другу,
/// и без огромных страниц, как при замерах текста
/// ([`common::without_thp`]).
fn run(name: &str, args: &[&str]) -> String {
    static SERIAL: Mutex<()> = Mutex::new(());
    let _one = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let out = common::without_thp(&mut Command::new(common::release_bin(name)))
        .args(args)
        .output()
        .unwrap();
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
    // При 2000 столбцы платят промахами кэша: в тексте в 2,5 раза, на
    // машине CI книги — в 1,9. При 4000 к ним добавляются промахи TLB
    // (раздел 30.4), и разница больше.
    let ns = |order: &str, n: &str, sum: &str| number_after(&run("matrix", &[order, n]), sum);
    let rows = ns("rows", "2000", "сумма 11999994,");
    let cols = ns("cols", "2000", "сумма 11999994,");
    assert!(cols > 1.5 * rows, "2000: {rows} {cols}");
    let rows = ns("rows", "4000", "сумма 47999995,");
    let cols = ns("cols", "4000", "сумма 47999995,");
    assert!(cols > 2.0 * rows, "4000: {rows} {cols}");
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
    // Сосед по машине иногда замедляет целую серию вызовов одного
    // варианта (раздел 30.9), поэтому у каждого варианта — лучшая медиана
    // из трёх запусков программы.
    let mut best = [f64::INFINITY; 3];
    let mut texts = String::new();
    for _ in 0..3 {
        let text = run("station", &["262144"]);
        let prefixes = ["store: медиана", " hash: медиана", "   x2: медиана"];
        for (b, prefix) in best.iter_mut().zip(prefixes) {
            *b = b.min(number_after(&text, prefix));
        }
        texts += &text;
    }
    let [store, hash, x2] = best;
    assert!(hash < store && x2 < 0.8 * hash, "{texts}");
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
