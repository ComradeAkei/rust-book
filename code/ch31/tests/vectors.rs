//! Раздел 31.1: тест со своим `main` (`harness = false` в Cargo.toml).
//! libtest здесь нет: бинарник читает таблицу векторов, печатает отчёт
//! сам, а вердикт для cargo — код выхода.

use ch31::basics::Window;
use std::process::ExitCode;

// ANCHOR: harness
/// Среднее окна размера `n`; размер — параметр типа, поэтому выбор из
/// заранее собранных вариантов.
fn mean(n: usize, input: &str) -> Option<Option<i32>> {
    Some(match n {
        1 => input.parse::<Window<1>>().ok()?.mean(),
        2 => input.parse::<Window<2>>().ok()?.mean(),
        3 => input.parse::<Window<3>>().ok()?.mean(),
        4 => input.parse::<Window<4>>().ok()?.mean(),
        _ => return None,
    })
}

fn main() -> ExitCode {
    let table = include_str!("vectors.txt");
    let (mut passed, mut failed) = (0, 0);
    for (no, line) in table.lines().enumerate().filter(|(_, l)| !l.starts_with('#')) {
        let f: Vec<&str> = line.split('|').map(str::trim).collect();
        let n: usize = f[0].parse().expect("размер окна");
        let want = (f[2] != "-").then(|| f[2].parse::<i32>().expect("среднее"));
        let got = mean(n, f[1]);
        if got == Some(want) {
            passed += 1;
        } else {
            failed += 1;
            println!("FAIL vectors.txt:{}: {line} — получено {got:?}", no + 1);
        }
    }
    println!("векторы окна: {passed} совпали, {failed} нет");
    if failed == 0 { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}
// ANCHOR_END: harness
