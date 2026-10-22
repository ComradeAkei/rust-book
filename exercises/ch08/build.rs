//! Сценарий сборки заготовок главы 8: таблица синуса для задачи B.
//!
//! Плавающая точка здесь разрешена: сценарий работает на машине сборки.
//! В прошивку должен попасть готовый массив `i16`.
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

/// Четверть периода синуса в Q15: SINE[i] = round(32767 · sin(i · π/512)),
/// i = 0..=256.
fn sine_table() -> [i16; 257] {
    // TODO(задача B): вычислите значения. Пока таблица из нулей, чтобы
    // крейт собирался и остальные задачи можно было проверять.
    [0; 257]
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let mut out = String::from("// Сгенерировано build.rs. Не редактировать.\n\n");
    // TODO(задача B): таблица должна лежать во flash, а не в ОЗУ.
    out.push_str("pub static SINE: [i16; 257] = [");
    for v in sine_table() {
        write!(out, "{v}, ").unwrap();
    }
    out.push_str("];\n");
    let dir = env::var_os("OUT_DIR").unwrap();
    fs::write(Path::new(&dir).join("sine.rs"), out).unwrap();
}
