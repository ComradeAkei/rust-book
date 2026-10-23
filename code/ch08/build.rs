//! Сценарий сборки главы 8 (раздел 8.5). Работает на машине, где идёт
//! сборка, до компиляции крейта:
//!
//! - превращает `sensors.csv` в таблицу `SENSORS` на Rust;
//! - записывает, для какой архитектуры работал он сам и для какой
//!   собирается крейт, — это разные вещи при сборке под Cortex-M.
//!
//! Результат кладётся в `OUT_DIR`, крейт подключает его через `include!`.
use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

// ANCHOR: sensors
fn sensors(csv: &str) -> String {
    let mut rows = Vec::new();
    for (n, line) in csv.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("id,") {
            continue;
        }
        let f: Vec<&str> = line.split(',').map(str::trim).collect();
        let [id, name, unit, scale] = f[..] else {
            panic!("sensors.csv:{}: нужно 4 поля, а не {}", n + 1, f.len());
        };
        let id: u16 = id.parse().unwrap_or_else(|e| panic!("sensors.csv:{}: id: {e}", n + 1));
        let scale: u32 = scale.parse().unwrap_or_else(|e| panic!("sensors.csv:{}: {e}", n + 1));
        assert!(id != 0, "sensors.csv:{}: датчика 0 не бывает", n + 1);
        rows.push((id, name.to_string(), unit.to_string(), scale));
    }
    let mut out = String::from("// Сгенерировано build.rs из sensors.csv. Не редактировать.\n\n");
    writeln!(out, "pub static SENSORS: [SensorInfo; {}] = [", rows.len()).unwrap();
    for (id, name, unit, scale) in &rows {
        writeln!(
            out,
            "    SensorInfo {{ id: {id}, name: {name:?}, unit: {unit:?}, scale: {scale} }},"
        )
        .unwrap();
    }
    out.push_str("];\n\npub const fn sensor(id: u16) -> Option<&'static SensorInfo> {\n");
    out.push_str("    match id {\n");
    for (i, (id, ..)) in rows.iter().enumerate() {
        writeln!(out, "        {id} => Some(&SENSORS[{i}]),").unwrap();
    }
    out.push_str("        _ => None,\n    }\n}\n");
    out
}
// ANCHOR_END: sensors

fn main() {
    // ANCHOR: main
    // Без этой строки cargo перезапускает build.rs при изменении любого
    // файла пакета; с ней — только при изменении sensors.csv и самого
    // build.rs.
    println!("cargo::rerun-if-changed=sensors.csv");
    println!("cargo::rerun-if-changed=build.rs");

    let out = env::var_os("OUT_DIR").unwrap();
    let csv = fs::read_to_string("sensors.csv").unwrap();
    fs::write(Path::new(&out).join("sensors.rs"), sensors(&csv)).unwrap();

    // cfg! в build.rs описывает машину сборки, а не цель: сценарий —
    // обычная программа для хоста. Цель приходит в переменных CARGO_CFG_*.
    let info = format!(
        "pub const BUILD_RS_ARCH: &str = {:?};\n\
         pub const BUILD_RS_CFG_ARM: bool = {};\n\
         pub const TARGET_ARCH: &str = {:?};\n",
        env::consts::ARCH,
        cfg!(target_arch = "arm"),
        env::var("CARGO_CFG_TARGET_ARCH").unwrap(),
    );
    fs::write(Path::new(&out).join("build_info.rs"), info).unwrap();
    // ANCHOR_END: main
}
