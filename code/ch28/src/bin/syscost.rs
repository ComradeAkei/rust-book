//! Раздел 28.1: сколько стоит системный вызов и сколько — vDSO.
//!
//! Каждая операция выполняется миллион раз, из пяти замеров берётся
//! лучший: так меньше шума от вытеснения и прерываний. Числа зависят от
//! процессора, ядра и защит от Spectre и Meltdown; важно соотношение.

use std::hint::black_box;
use std::time::{Duration, Instant};

const N: u32 = 1_000_000;

/// Лучшее из пяти время одной операции, нс.
fn per_call(mut op: impl FnMut()) -> f64 {
    let best = (0..5)
        .map(|_| {
            let t = Instant::now();
            for _ in 0..N {
                op();
            }
            t.elapsed()
        })
        .min()
        .unwrap_or(Duration::ZERO);
    best.as_nanos() as f64 / f64::from(N)
}

// ANCHOR: cost
#[inline(never)]
fn plain_call(x: u32) -> u32 {
    black_box(x) + 1
}

fn main() {
    let call = per_call(|| {
        black_box(plain_call(black_box(1)));
    });
    // getppid — самый дешёвый вызов: ядро только читает поле задачи.
    let rustix = per_call(|| {
        black_box(rustix::process::getppid());
    });
    // SAFETY: getppid без аргументов, ошибок у него нет.
    let libc = per_call(|| {
        black_box(unsafe { libc::getppid() });
    });
    // clock_gettime через vDSO: код ядра в адресном пространстве
    // процесса читает часы без перехода в ядро.
    let vdso = per_call(|| {
        black_box(Instant::now());
    });
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: указатель на живую структуру; тот же вызов, но настоящим
    // переходом в ядро, мимо vDSO.
    let sys = per_call(|| unsafe {
        libc::syscall(libc::SYS_clock_gettime, libc::CLOCK_MONOTONIC, &mut ts);
        black_box(&ts);
    });
    println!("вызов функции:                 {call:6.1} нс");
    println!("getppid через rustix (asm):    {rustix:6.1} нс");
    println!("getppid через libc:            {libc:6.1} нс");
    println!("Instant::now (vDSO):           {vdso:6.1} нс");
    println!("clock_gettime через syscall:   {sys:6.1} нс");
    println!("vDSO быстрее вызова ядра: {}", vdso < sys);
}
// ANCHOR_END: cost
