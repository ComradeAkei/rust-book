//! Раздел 13.5: время на элемент для пар функций, которые глава сравнивает
//! в ассемблере: `for` и `fold` по `chain`, сведение каналов по индексам,
//! через `zip` и с AVX2, пики блоков через `chunks` и `chunks_exact`.
//!
//! Запуск: `cargo run --release -p ch13 --example bench [повторов]`.
//! Время меняется от машины к машине и от запуска к запуску.
use ch13::codegen::*;
use std::hint::black_box;
use std::time::Instant;

const N: usize = 4096;

/// Наносекунд на элемент: лучший из пяти замеров по `reps` повторов.
fn ns(reps: u32, elems: usize, mut f: impl FnMut()) -> f64 {
    (0..5)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..reps {
                f();
            }
            t.elapsed().as_secs_f64() * 1e9 / f64::from(reps) / elems as f64
        })
        .fold(f64::INFINITY, f64::min)
}

fn main() {
    let reps = std::env::args().nth(1).map_or(20_000, |s| s.parse().unwrap());
    let mut s = 1u32;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (s >> 16) as i16
    };
    let a: Vec<i16> = (0..N).map(|_| next()).collect();
    let b: Vec<i16> = (0..N).map(|_| next()).collect();
    let w: Vec<u32> = a.iter().map(|&x| x as u32).collect();
    let (w1, w2) = w.split_at(N / 2);
    let mut out = vec![0i16; N];
    let mut peaks = vec![0i16; N / 16];

    let chain_for = ns(reps, N, || {
        black_box(ch13_chain_for(black_box(w1), black_box(w2)));
    });
    let chain_fold = ns(reps, N, || {
        black_box(ch13_chain_fold(black_box(w1), black_box(w2)));
    });
    println!("chain, for:          {chain_for:.3} нс на элемент");
    println!("chain, fold:         {chain_fold:.3} нс на элемент");

    let mut run = |f: fn(&mut [i16], &[i16], &[i16])| {
        ns(reps, N, || f(black_box(&mut out), black_box(&a), black_box(&b)))
    };
    let index = run(ch13_mix_index);
    let zip = run(ch13_mix_zip);
    let best = run(mix);
    println!("сведение, индексы:   {index:.3} нс на отсчёт");
    println!("сведение, zip:       {zip:.3} нс на отсчёт");
    println!("сведение, AVX2:      {best:.3} нс на отсчёт (если процессор умеет)");

    let chunks = ns(reps, N, || ch13_peaks_chunks(black_box(&a), black_box(&mut peaks)));
    let exact = ns(reps, N, || ch13_peaks_exact(black_box(&a), black_box(&mut peaks)));
    println!("пики, chunks:        {chunks:.3} нс на отсчёт");
    println!("пики, chunks_exact:  {exact:.3} нс на отсчёт");
}
