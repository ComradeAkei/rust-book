//! Сценарий сборки решений главы 8: таблица синуса для задачи B.
//!
//! Плавающая точка нужна только здесь, на машине сборки. В прошивку
//! попадает готовый массив `i16`, и микроконтроллеру без FPU не придётся
//! вычислять синус.
use std::env;
use std::f64::consts::FRAC_PI_2;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

// ANCHOR: table
/// Четверть периода синуса в Q15: 257 точек от 0 до π/2 включительно.
fn sine_table() -> [i16; 257] {
    let mut t = [0i16; 257];
    for (i, v) in t.iter_mut().enumerate() {
        let x = i as f64 / 256.0 * FRAC_PI_2;
        // Не больше 32767: sin(π/2) = 1.0 даёт ровно i16::MAX.
        *v = (32767.0 * x.sin()).round() as i16;
    }
    t
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    let mut out = String::from("// Сгенерировано build.rs. Не редактировать.\n\n");
    out.push_str("/// SINE[i] = round(32767 · sin(i · π/512)), i = 0..=256.\n");
    out.push_str("pub static SINE: [i16; 257] = [");
    for (i, v) in sine_table().iter().enumerate() {
        if i % 12 == 0 {
            out.push_str("\n   ");
        }
        write!(out, " {v},").unwrap();
    }
    out.push_str("\n];\n");
    let dir = env::var_os("OUT_DIR").unwrap();
    fs::write(Path::new(&dir).join("sine.rs"), out).unwrap();
}
// ANCHOR_END: table
