//! Разделы 14.4 и 14.5: цена операций одного потока — `clone` и `drop` у
//! `Rc` и `Arc`, инкремент через `Cell` и через `RefCell`.
//!
//! Запуск: `cargo run --release -p ch14 --example bench [повторов]`.
//! Время меняется от машины к машине и от запуска к запуску.
use std::cell::{Cell, RefCell};
use std::hint::black_box;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;

/// Наносекунд на операцию: лучший из пяти замеров по `reps` повторов.
fn ns(reps: u32, mut f: impl FnMut()) -> f64 {
    (0..5)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..reps {
                f();
            }
            t.elapsed().as_secs_f64() * 1e9 / f64::from(reps)
        })
        .fold(f64::INFINITY, f64::min)
}

fn main() {
    let reps = std::env::args().nth(1).map_or(20_000_000, |s| s.parse().unwrap());
    let rc = Rc::new(1u32);
    let arc = Arc::new(1u32);
    let cell = Cell::new(0u32);
    let refcell = RefCell::new(0u32);

    let r = [
        ("Rc::clone + drop", ns(reps, || drop(black_box(Rc::clone(black_box(&rc)))))),
        ("Arc::clone + drop", ns(reps, || drop(black_box(Arc::clone(black_box(&arc)))))),
        ("Cell: +1", ns(reps, || black_box(&cell).set(cell.get() + 1))),
        ("RefCell: +1", ns(reps, || *black_box(&refcell).borrow_mut() += 1)),
    ];
    for (name, t) in r {
        println!("{name:18} {t:6.2} нс");
    }
}
