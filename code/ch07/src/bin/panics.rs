//! Опыты разделов 7.5 и 7.7: что делает паника, когда её ловят, когда она
//! случается во время другой паники и когда пересекает границу `extern "C"`.
//!
//! Запуск: `cargo run -p ch07 --bin panics -- <опыт>`. Часть опытов
//! завершает процесс через abort, поэтому каждый — отдельный запуск.

use std::panic::{self, AssertUnwindSafe};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;

// ANCHOR: loud
/// Сообщает о своём уничтожении в stderr.
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        eprintln!("drop {}", self.0);
    }
}
// ANCHOR_END: loud

// ANCHOR: catch
fn catch() {
    let r = panic::catch_unwind(|| {
        let _a = Loud("a");
        let v = [1, 2, 3];
        let i = v.len() + std::hint::black_box(4);
        v[i] // паника: индекс вне массива
    });
    let payload = r.unwrap_err();
    let msg = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("?");
    eprintln!("поймали: {msg}");
}
// ANCHOR_END: catch

// ANCHOR: hook
fn hook() {
    panic::set_hook(Box::new(|info| {
        let loc = info.location().unwrap();
        let msg = info.payload_as_str().unwrap_or("?");
        eprintln!("[станция] паника в {}:{}: {msg}", loc.file(), loc.line());
    }));
    let _ = panic::catch_unwind(|| panic!("датчик {} не отвечает", 3));
    let _ = panic::take_hook(); // вернуть стандартный обработчик
    let _ = panic::catch_unwind(|| panic!("снова со стандартным хуком"));
}
// ANCHOR_END: hook

// ANCHOR: double
/// Деструктор, который паникует: например, сбрасывает буфер на диск и
/// получает ошибку.
struct Flusher;

impl Drop for Flusher {
    fn drop(&mut self) {
        eprintln!("drop Flusher: сброс буфера");
        panic!("сброс не удался");
    }
}

fn double() {
    let _f = Flusher;
    let _a = Loud("a");
    panic!("первая паника");
}
// ANCHOR_END: double

fn drop_only() {
    let _f = Flusher;
    eprintln!("выходим из функции без паники");
}

// ANCHOR: ffi
#[inline(never)] // чтобы функцию было видно в ассемблере (раздел 7.7)
extern "C" fn callback_c(x: i32) -> i32 {
    if x < 0 {
        panic!("отрицательный отсчёт {x}");
    }
    x * 2
}

#[inline(never)]
extern "C-unwind" fn callback_c_unwind(x: i32) -> i32 {
    if x < 0 {
        panic!("отрицательный отсчёт {x}");
    }
    x * 2
}

fn ffi_c() {
    let _a = Loud("a");
    let r = panic::catch_unwind(|| callback_c(std::hint::black_box(-1)));
    eprintln!("сюда не дойдём: {}", r.is_err());
}

fn ffi_c_unwind() {
    let _a = Loud("a");
    let r = panic::catch_unwind(|| callback_c_unwind(std::hint::black_box(-1)));
    eprintln!("поймали: {}", r.is_err());
}
// ANCHOR_END: ffi

// ANCHOR: track
/// Номер регистра в окне 0..16. Паника укажет на вызывающего.
#[track_caller]
fn reg(n: usize) -> usize {
    assert!(n < 16, "регистр {n} вне окна");
    n
}

fn track() {
    let _ = reg(3);
    let _ = reg(std::hint::black_box(40));
}
// ANCHOR_END: track

// ANCHOR: thread
fn thread() {
    let samples = Arc::new(Mutex::new(vec![1, 2, 3]));
    let s = Arc::clone(&samples);
    let h = thread::spawn(move || {
        let mut v = s.lock().unwrap();
        v.push(4);
        panic!("поток обработки упал, держа замок");
    });
    eprintln!("join: {:?}", h.join().map_err(|_| "поток паниковал"));
    match samples.lock() {
        Ok(v) => eprintln!("данные: {v:?}"),
        Err(poisoned) => eprintln!("замок отравлен, данные: {:?}", poisoned.into_inner()),
    }
}
// ANCHOR_END: thread

fn resume() {
    let r = panic::catch_unwind(AssertUnwindSafe(|| -> u32 { panic!("ушла дальше") }));
    eprintln!("поймали и отпускаем");
    if let Err(p) = r {
        panic::resume_unwind(p);
    }
}

fn main() -> ExitCode {
    let what = std::env::args().nth(1).unwrap_or_default();
    match what.as_str() {
        "catch" => catch(),
        "hook" => hook(),
        "double" => double(),
        "drop-only" => drop_only(),
        "ffi-c" => ffi_c(),
        "ffi-c-unwind" => ffi_c_unwind(),
        "track" => track(),
        "thread" => thread(),
        "resume" => resume(),
        _ => {
            eprintln!("опыты: catch hook double drop-only ffi-c ffi-c-unwind track thread resume");
            return ExitCode::FAILURE;
        }
    }
    eprintln!("main завершился");
    ExitCode::SUCCESS
}
