//! Задача D: как средняя длина пробы линейного пробирования растёт с
//! загрузкой, что с ней делают надгробия и почему `std::HashMap`
//! (SwissTable) не вырождается.
//!
//! ```text
//! cargo run --release -p ch15-solutions --example probes
//! ```

use ch15_solutions::d_table::{filled, probe_stats};
use std::time::Instant;

fn main() {
    let cap = 1 << 16;
    println!("таблица на {cap} ячеек, случайные ключи");
    println!("загрузка  средняя проба  макс  время поиска, нс");
    for &load in &[0.3, 0.5, 0.7, 0.8, 0.9, 0.95] {
        let (t, _) = filled(cap, load, 1);
        let (avg, max) = probe_stats(&t);
        let keys: Vec<u64> = t.keys().copied().collect();
        let start = Instant::now();
        let mut hits = 0u64;
        for _ in 0..4 {
            for k in &keys {
                hits += t.get(k).is_some() as u64;
            }
        }
        let ns = start.elapsed().as_nanos() as f64 / (4 * keys.len().max(1)) as f64;
        std::hint::black_box(hits);
        println!("{load:>7.2}  {avg:>12.2}  {max:>4}  {ns:>14.1}");
    }
}
