//! Раздел 14.1: `Box::new` большого массива. В отладочной сборке массив
//! сначала строится на стеке, и 16 МиБ не помещаются в 8 МиБ стека
//! главного потока; в release LLVM строит его сразу в куче. `new_zeroed`
//! и `vec!` выделяют память в куче в обеих сборках. Массив из `from_fn`
//! (`seq`) строится на стеке и в release.
//!
//! Запуск: `cargo run -p ch14 --example bigbox [new|zeroed|vec|seq]`.
use ch14::boxes::{BIG, big_from_vec, big_zeroed};

#[inline(never)]
fn by_new() -> Box<[u8; BIG]> {
    Box::new([0; BIG])
}

#[inline(never)]
fn by_from_fn() -> Box<[u32; BIG / 4]> {
    Box::new(core::array::from_fn(|i| i as u32))
}

fn main() {
    let how = std::env::args().nth(1).unwrap_or_else(|| "new".into());
    let sum: u64 = match how.as_str() {
        "zeroed" => big_zeroed().iter().map(|&b| u64::from(b)).sum(),
        "vec" => big_from_vec(BIG).iter().map(|&b| u64::from(b)).sum(),
        "seq" => by_from_fn().iter().map(|&b| u64::from(b)).sum(),
        _ => by_new().iter().map(|&b| u64::from(b)).sum(),
    };
    println!("{how}: {} МиБ в куче, сумма {sum}", BIG >> 20);
}
