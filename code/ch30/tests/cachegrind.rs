//! Раздел 30.4: cachegrind — число инструкций и промахи модели кэша.
//!
//! Модель задана явно (`--I1`, `--D1`, `--LL`): без этого cachegrind берёт
//! размеры кэшей из cpuid, и цифры зависят от машины. Ir и промахи модели
//! детерминированы для одной и той же программы, поэтому тест сверяет их
//! почти точно; сами числа инструкций — наблюдение для rustc 1.97.
//! Без valgrind опыты пропускаются; `CH30_REQUIRE_VALGRIND` делает
//! пропуск ошибкой.

mod common;

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

// ANCHOR: model
const MODEL: [&str; 3] = ["--I1=32768,8,64", "--D1=32768,8,64", "--LL=8388608,16,64"];
// ANCHOR_END: model

/// События по функциям из файла cachegrind: имя функции → имя события →
/// число. Имена событий — из строки `events:`; строка стоимости — номер
/// строки исходника и счётчики, нулевые в конце строки можно не писать.
type Costs = HashMap<String, HashMap<String, u64>>;

fn by_function(out: &str) -> Costs {
    let mut events: Vec<&str> = Vec::new();
    let mut map: Costs = HashMap::new();
    let mut current = None;
    for line in out.lines() {
        if let Some(names) = line.strip_prefix("events: ") {
            events = names.split_whitespace().collect();
        } else if let Some(name) = line.strip_prefix("fn=") {
            current = Some(name.to_owned());
        } else if line.starts_with(|c: char| c.is_ascii_digit()) {
            let e = map.entry(current.clone().unwrap()).or_default();
            for (i, n) in line.split(' ').skip(1).enumerate() {
                *e.entry(events[i].to_owned()).or_default() += n.parse::<u64>().unwrap();
            }
        }
    }
    map
}

/// Прогон программы главы под cachegrind: `sim` — `--cache-sim=yes` с
/// моделью [`MODEL`] или `--branch-sim=yes`.
fn cachegrind(sim: &str, name: &str, args: &[&str]) -> Option<Costs> {
    let valgrind = common::tool("VALGRIND", "valgrind");
    assert!(valgrind.is_some() || !common::required("CH30_REQUIRE_VALGRIND"), "нужен valgrind");
    let valgrind = valgrind?;
    let file = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("ch30-cg-{name}-{}.out", args.join("-")));
    let mut cmd = Command::new(valgrind);
    cmd.arg("--tool=cachegrind");
    match sim {
        "cache" => cmd.arg("--cache-sim=yes").args(MODEL),
        "branch" => cmd.args(["--cache-sim=no", "--branch-sim=yes"]),
        _ => unreachable!(),
    };
    let out = cmd
        .arg(format!("--cachegrind-out-file={}", file.display()))
        .arg(common::release_bin(name))
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    Some(by_function(&std::fs::read_to_string(file).unwrap()))
}

fn near(x: u64, want: u64, tol: f64) -> bool {
    (x as f64 - want as f64).abs() <= want as f64 * tol
}

#[test]
fn rows_and_cols_same_instructions_different_misses() {
    let Some(rows) = cachegrind("cache", "matrix", &["rows", "2000"]) else { return };
    let cols = cachegrind("cache", "matrix", &["cols", "2000"]).unwrap();
    let r = &rows["ch30::cache::sum_rows"];
    let c = &cols["ch30::cache::sum_cols"];
    let n2 = 2000 * 2000;
    // Инструкций одинаково: тот же цикл, другой порядок индексов.
    assert!(near(r["Ir"], c["Ir"], 0.001), "{r:?} {c:?}");
    // По строкам — промах на каждую строку кэша, восемь чисел f64.
    assert!(near(r["D1mr"], n2 / 8, 0.01), "{r:?}");
    // По столбцам — промах D1 на каждое обращение, но 2000 строк кэша
    // одного прохода (128 КиБ) помещаются в LL: в LL промахов столько же.
    assert!(near(c["D1mr"], n2, 0.01), "{c:?}");
    assert!(c["DLmr"] <= r["DLmr"], "{c:?} {r:?}");
}

#[test]
fn power_of_two_stride_defeats_the_model_cache() {
    let Some(cols) = cachegrind("cache", "matrix", &["cols", "2048"]) else { return };
    let c = &cols["ch30::cache::sum_cols"];
    // Шаг 16 КиБ: в модели с простым индексом набора все строки столбца
    // попадают в 32 набора LL из 8192 — 512 строк вместо 2048. Промах LL
    // на каждое обращение.
    assert!(near(c["DLmr"], 2048 * 2048, 0.01), "{c:?}");
}

#[test]
fn observed_instructions_per_byte_of_profile_phases() {
    let Some(f) = cachegrind("cache", "profile", &["4"]) else { return };
    let bytes = 4 << 20;
    let per_byte = |name: &str| f[name]["Ir"] as f64 / bytes as f64;
    let (crc, hist, noise) = (
        per_byte("ch30::hot::crc_bitwise"),
        per_byte("ch30::hot::histogram"),
        per_byte("ch30::hot::noise"),
    );
    eprintln!("Ir на байт: crc {crc:.2}, histogram {hist:.2}, noise {noise:.2}");
    // CRC: LLVM заменил восемь шагов на байт таблицей (раздел 25.7),
    // цикл развёрнут вдвое — 15 инструкций на два байта.
    assert!((7.0..8.0).contains(&crc), "{crc}");
    assert!((2.0..4.0).contains(&hist), "{hist}");
    assert!(noise > crc, "{noise}");
}

#[test]
fn branch_model_sees_mispredictions() {
    let Some(rb) = cachegrind("branch", "branches", &["random", "branchy"]) else { return };
    let sb = cachegrind("branch", "branches", &["sorted", "branchy"]).unwrap();
    let rl = cachegrind("branch", "branches", &["random", "branchless"]).unwrap();
    // Восемь вызовов (прогрев и семь выборок) по 4 МиБ.
    let bytes = 8 << 22;
    let miss = |c: &Costs, f: &str| c[f]["Bcm"];
    // Случайные байты: половина условных переходов по данным — мимо.
    let random = miss(&rb, "ch30::branches::select_branchy") as f64 / bytes as f64;
    assert!((0.45..0.55).contains(&random), "{random}");
    assert!(miss(&sb, "ch30::branches::select_branchy") < 1000);
    assert!(miss(&rl, "ch30::branches::select_branchless") < 1000);
    // Без перехода — больше инструкций на байт.
    let ir = |c: &Costs, f: &str| c[f]["Ir"] as f64 / bytes as f64;
    let (a, b) =
        (ir(&rb, "ch30::branches::select_branchy"), ir(&rl, "ch30::branches::select_branchless"));
    eprintln!("Ir на байт: с ветвлением {a:.2}, без {b:.2}");
    assert!(b > a, "{a} {b}");
}

#[test]
fn layout_misses_follow_bytes_read() {
    let Some(c) = cachegrind("cache", "memory", &["layout"]) else { return };
    // Двенадцать проходов по 4 Ми записей: прогрев и одиннадцать выборок.
    let records = 12u64 << 22;
    let aos = c["ch30::memory::sum_aos"]["D1mr"];
    let soa = c["ch30::memory::sum_soa"]["D1mr"];
    // Запись 32 байта — две на строку; значение i32 — шестнадцать.
    assert!(near(aos, records / 2, 0.02), "{aos}");
    assert!(near(soa, records / 16, 0.02), "{soa}");
}
