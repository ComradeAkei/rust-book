//! Сценарий сборки решений главы 25: бюджет ОЗУ для задачи D.
//!
//! Объём ОЗУ берётся из той же карты памяти, по которой компонуются
//! прошивки (firmware/memory.x), минус запас под стек. Результат уходит
//! в переменную окружения компилятора `CH25_RAM_BUDGET`, и библиотека
//! читает её через `env!`. Переменная окружения с тем же именем при
//! сборке заменяет вычисленное значение — так тест проверяет, что
//! превышение бюджета останавливает сборку.

// ANCHOR: build
use std::env;
use std::fs;
use std::path::Path;

/// Запас под стек: столько ОЗУ буферам драйверов не достаётся.
const STACK_RESERVE: u64 = 4096;

/// «16K» → 16384, «1M» → 1048576, «0x4000» → 16384.
fn parse_length(s: &str) -> u64 {
    let s = s.trim();
    let (digits, scale) = match s.as_bytes().last() {
        Some(b'K') => (&s[..s.len() - 1], 1024),
        Some(b'M') => (&s[..s.len() - 1], 1024 * 1024),
        _ => (s, 1),
    };
    let n = match digits.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16),
        None => digits.parse(),
    };
    n.unwrap_or_else(|e| panic!("memory.x: длина ОЗУ {s:?} не число: {e}")) * scale
}

/// Длина области `RAM` из блока MEMORY скрипта компоновщика.
fn ram_length(script: &str) -> u64 {
    let line = script
        .lines()
        .map(str::trim)
        .find(|l| l.starts_with("RAM") && l.contains("LENGTH"))
        .expect("memory.x: нет строки области RAM");
    let (_, len) = line.split_once("LENGTH").unwrap();
    let len = len.trim_start().trim_start_matches('=');
    parse_length(len.split(',').next().unwrap())
}

fn main() {
    let memory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../firmware/memory.x");
    println!("cargo::rerun-if-changed={}", memory.display());
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=CH25_RAM_BUDGET");
    let budget = match env::var("CH25_RAM_BUDGET") {
        Ok(v) => v,
        Err(_) => {
            let script =
                fs::read_to_string(&memory).unwrap_or_else(|e| panic!("{}: {e}", memory.display()));
            (ram_length(&script) - STACK_RESERVE).to_string()
        }
    };
    println!("cargo::rustc-env=CH25_RAM_BUDGET={budget}");
}
// ANCHOR_END: build
