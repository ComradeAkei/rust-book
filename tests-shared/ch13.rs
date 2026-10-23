// Общие тесты задач главы 13.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch13/tests/ch13.rs` и `solutions/ch13/tests/ch13.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задачи C и D проверяют ассемблер, собранный закреплённым cargo, и под
// Miri пропускаются. Задача D считает обращения к распределителю: для
// этого тестовая программа ставит свой глобальный распределитель, который
// считает вызовы в каждом потоке отдельно, чтобы параллельные тесты не
// мешали друг другу.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Каталог крейта с задачами: заготовки или решения.
fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Каталог для файлов, которые тест создаёт сам.
fn tmp(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-{name}", env!("CARGO_PKG_NAME")));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Команда с окружением закреплённого тулчейна: rustup выберет версию по
/// rust-toolchain.toml, а не по тулчейну, которым запущен сам тест.
fn pinned(program: &str, dir: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env("CARGO_TERM_COLOR", "never");
    cmd
}

fn text(out: &[u8]) -> String {
    String::from_utf8_lossy(out).into_owned()
}

/// Имя библиотеки крейта в именах файлов: `ch13-solutions` → `ch13_solutions`.
fn lib_name() -> String {
    env!("CARGO_PKG_NAME").replace('-', "_")
}

/// Ассемблер библиотеки задач, собранной в release для `target`.
fn emit_asm(target: &str) -> String {
    let dir = tmp(&format!("asm-{target}"));
    let mut cmd = pinned("cargo", crate_dir());
    cmd.args(["rustc", "--release", "--lib", "--target", target, "--target-dir"]).arg(&dir);
    if target.starts_with("thumb") {
        cmd.arg("--no-default-features");
    }
    cmd.args(["--", "--emit", "asm", "-C", "codegen-units=1"]);
    if target.starts_with("x86_64") {
        cmd.args(["-C", "llvm-args=-x86-asm-syntax=intel"]);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "cargo rustc --target {target}:\n{}", text(&out.stderr));
    let deps = dir.join(target).join("release/deps");
    let prefix = format!("{}-", lib_name());
    let file = std::fs::read_dir(&deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(&prefix) && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции с `no_mangle` по имени. Если LLVM сделал из неё
/// псевдоним другой, одинаковой, — `name = other` или `.set name, other`,
/// — берётся тело той.
fn body(asm: &str, name: &str) -> Vec<String> {
    let label = format!("{name}:");
    let Some(start) = asm.lines().position(|l| l == label) else {
        let (assign, set) = (format!("{name} = "), format!(".set {name}, "));
        let target = asm
            .lines()
            .find_map(|l| l.trim().strip_prefix(&assign).or_else(|| l.trim().strip_prefix(&set)))
            .unwrap_or_else(|| panic!("в ассемблере нет функции {name}"));
        return body(asm, target.trim());
    };
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(|l| l.trim().replace('\t', " "))
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
        .collect()
}

/// Сколько инструкций начинается с `prefix`.
fn count(body: &[String], prefix: &str) -> usize {
    body.iter().filter(|l| l.starts_with(prefix)).count()
}

/// Псевдослучайные числа xorshift32: тесты воспроизводимы.
fn rng(seed: u32) -> impl FnMut() -> u32 {
    let mut s = seed | 1;
    move || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s
    }
}

// ------------------------------------------------- распределитель-счётчик

thread_local! {
    /// Вызовы alloc и realloc в этом потоке.
    static CALLS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

struct Counting;

// SAFETY: все вызовы передаются системному распределителю как есть.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = CALLS.try_with(|c| c.set((c.get().0 + 1, c.get().1)));
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let _ = CALLS.try_with(|c| c.set((c.get().0, c.get().1 + 1)));
        // SAFETY: контракт realloc тот же, что у вызывающего.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Сколько раз `f` вызвала alloc и realloc в этом потоке.
fn allocs<T>(f: impl FnOnce() -> T) -> ((usize, usize), T) {
    let before = CALLS.with(Cell::get);
    let r = f();
    let after = CALLS.with(Cell::get);
    ((after.0 - before.0, after.1 - before.1), r)
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::sut::a_chains::{measured, predicted};

    const CHAINS: [&str; 8] = [
        "map(f).find(|&x| x > 50)",
        "map(f).rev().take(2).sum()",
        "map(f).count()",
        "map(f).len()",
        "map(f).take_while(|&x| x < 50).count()",
        "map(f).skip_while(|&x| x < 50).count()",
        "map(f).map(… Err(x) при x > 85 …).sum::<Result<_, _>>()",
        "filter(нечётные).map(f).last()",
    ];

    #[test]
    fn task_a_chains() {
        let answers = predicted();
        let actual = measured();
        let wrong: Vec<String> = (0..8)
            .filter(|&i| answers[i] != actual[i])
            .map(|i| {
                format!(
                    "{}. {}: ответ (результат {}, вызовов {}), на деле ({}, {})",
                    i + 1,
                    CHAINS[i],
                    answers[i].0,
                    answers[i].1,
                    actual[i].0,
                    actual[i].1
                )
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use sut::b_iters::{
        Bits, END, ESC, ESC_END, ESC_ESC, Primes, SEGMENT, Slip, SlipError, SlipFrame,
    };

    fn naive_bits(x: u32) -> Vec<u32> {
        (0..32).filter(|i| x >> i & 1 == 1).collect()
    }

    #[test]
    fn task_b_bits() {
        let mut next = rng(7);
        let step = if cfg!(miri) { 997 } else { 1 };
        let random = if cfg!(miri) { 40 } else { 2000 };
        let values = (0..=0xffffu32).step_by(step).chain((0..random).map(|_| next())).chain([
            0,
            1,
            1 << 31,
            u32::MAX,
            0x8000_0001,
        ]);
        for x in values {
            let want = naive_bits(x);
            assert_eq!(Bits(x).collect::<Vec<_>>(), want, "{x:#x}");
            assert_eq!(
                Bits(x).rev().collect::<Vec<_>>(),
                want.iter().rev().copied().collect::<Vec<_>>()
            );
            assert_eq!(Bits(x).len(), want.len(), "{x:#x}");
            assert_eq!(Bits(x).size_hint(), (want.len(), Some(want.len())));
        }
        // С двух концов вперемешку: каждый бит ровно один раз.
        let mut b = Bits(0b1011_0110);
        assert_eq!((b.next(), b.next_back(), b.next(), b.len()), (Some(1), Some(7), Some(2), 2));
        assert_eq!(
            (b.next_back(), b.next(), b.next(), b.next_back()),
            (Some(5), Some(4), None, None)
        );
        fn fused<I: std::iter::FusedIterator + DoubleEndedIterator + ExactSizeIterator>(_: &I) {}
        fused(&Bits(1));
    }

    fn is_prime(n: u32) -> bool {
        n >= 2 && (2..).take_while(|d: &u64| d * d <= u64::from(n)).all(|d| u64::from(n) % d != 0)
    }

    #[test]
    fn task_b_primes_small() {
        let n = if cfg!(miri) { 60 } else { 3000 };
        let want: Vec<u32> = (0..).filter(|&k| is_prime(k)).take(n).collect();
        assert_eq!(Primes::new().take(n).collect::<Vec<_>>(), want);
        for from in [0, 1, 2, 3, 4, 5, 90, 97, 98] {
            let want: Vec<u32> = (from..).filter(|&k| is_prime(k)).take(10).collect();
            assert_eq!(Primes::starting_at(from).take(10).collect::<Vec<_>>(), want, "с {from}");
        }
    }

    /// Счёт простых до миллиона и граница сегмента; лениво: простых для
    /// просеивания нужно не больше, чем до корня из пройденного.
    #[test]
    #[cfg_attr(miri, ignore = "долго под Miri")]
    fn task_b_primes_count() {
        let mut p = Primes::new();
        assert_eq!(p.by_ref().take_while(|&k| k < 1_000_000).count(), 78_498);
        assert!(p.base_len() <= 200, "держит {} простых", p.base_len());
        let edge = 2 * SEGMENT as u32 + 1;
        let want: Vec<u32> = (edge - 200..).filter(|&k| is_prime(k)).take(30).collect();
        assert_eq!(Primes::starting_at(edge - 200).take(30).collect::<Vec<_>>(), want);
    }

    /// У верхней границы `u32`: последние простые и конец без
    /// переполнения. Простых для просеивания здесь уже 6541, и под Miri
    /// одно их построение занимает минуты.
    #[test]
    #[cfg_attr(miri, ignore = "долго под Miri")]
    fn task_b_primes_top() {
        let from = u32::MAX - 400;
        let want: Vec<u32> = (from..=u32::MAX).filter(|&k| is_prime(k)).collect();
        let mut p = Primes::starting_at(from);
        assert_eq!(p.by_ref().collect::<Vec<_>>(), want);
        assert_eq!(want.last(), Some(&4_294_967_291));
        assert_eq!((p.next(), p.next()), (None, None));
        assert_eq!(Primes::starting_at(u32::MAX).next(), None);
    }

    /// Кодер SLIP для тестов: END в начале и в конце, END и ESC
    /// экранированы.
    fn slip_encode(payload: &[u8], out: &mut Vec<u8>) {
        out.push(END);
        for &b in payload {
            match b {
                END => out.extend([ESC, ESC_END]),
                ESC => out.extend([ESC, ESC_ESC]),
                b => out.push(b),
            }
        }
        out.push(END);
    }

    fn decode(f: SlipFrame<'_>) -> Result<Vec<u8>, SlipError> {
        f.bytes().collect()
    }

    #[test]
    fn task_b_slip_roundtrip() {
        let mut next = rng(13);
        let mut stream = Vec::new();
        let mut want = Vec::new();
        for _ in 0..if cfg!(miri) { 10 } else { 300 } {
            let len = 1 + next() as usize % 40;
            // Байты END и ESC встречаются часто: пусть экранирование работает.
            let p: Vec<u8> = (0..len)
                .map(|_| match next() % 5 {
                    0 => END,
                    1 => ESC,
                    _ => next() as u8,
                })
                .collect();
            slip_encode(&p, &mut stream);
            want.push(p);
        }
        let got: Vec<Vec<u8>> = Slip::new(&stream).map(|f| decode(f).unwrap()).collect();
        assert_eq!(got, want);
        let mut s = Slip::new(&stream);
        assert_eq!(s.by_ref().count(), want.len());
        assert!(s.rest().is_empty());
        assert!(s.next().is_none());
    }

    #[test]
    fn task_b_slip_edges() {
        // END подряд — пустые кадры, их нет; хвост без END — в rest.
        let stream = [END, END, 1, 2, END, END, END, 3, ESC, ESC_END, END, 9, 9];
        let mut s = Slip::new(&stream);
        assert_eq!(s.next(), Some(SlipFrame(&[1, 2])));
        let f = s.next().unwrap();
        assert_eq!(f, SlipFrame(&[3, ESC, ESC_END]));
        assert_eq!(decode(f), Ok(vec![3, END]));
        assert_eq!(s.next(), None);
        assert_eq!(s.rest(), [9, 9]);
        assert_eq!(Slip::new(&[1, 2, 3]).next(), None);
        assert_eq!(Slip::new(&[1, 2, 3]).rest(), [1, 2, 3]);
        // Ошибки экранирования: итератор кадра кончается на первой.
        let bad = SlipFrame(&[1, ESC, 0x42, 2]);
        assert_eq!(bad.bytes().collect::<Vec<_>>(), [Ok(1), Err(SlipError::BadEscape(0x42))]);
        let trailing = SlipFrame(&[1, ESC]);
        assert_eq!(decode(trailing), Err(SlipError::TrailingEscape));
        let mut it = trailing.bytes();
        assert_eq!(
            (it.next(), it.next(), it.next(), it.next()),
            (Some(Ok(1)), Some(Err(SlipError::TrailingEscape)), None, None)
        );
    }

    #[test]
    fn task_b_slip_size_hint() {
        let frames: [&[u8]; 6] = [
            &[],
            &[1],
            &[ESC, ESC_END],
            &[ESC, 0x42, 1, 2, 3],
            &[1, 2, ESC, ESC_ESC, 3],
            &[ESC, ESC_END, ESC, ESC_ESC],
        ];
        for raw in frames {
            let mut it = SlipFrame(raw).bytes();
            loop {
                let (lo, hi) = it.size_hint();
                let rest = it.clone().count();
                assert!(
                    lo <= rest && rest <= hi.unwrap(),
                    "{raw:02x?}: {lo}..{hi:?}, осталось {rest}"
                );
                if it.next().is_none() {
                    break;
                }
            }
        }
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use sut::c_dot::{ch13c_dot_f32, ch13c_dot_f32_lanes, ch13c_dot_i32};

    #[test]
    fn task_c_values() {
        let mut next = rng(21);
        for n in [0, 1, 7, 8, 9, 31, 64, 100, 1000] {
            let a: Vec<i32> = (0..n).map(|_| next() as i32).collect();
            let b: Vec<i32> = (0..n + 3).map(|_| next() as i32).collect();
            let want = a.iter().zip(&b).fold(0i32, |s, (x, y)| s.wrapping_add(x.wrapping_mul(*y)));
            assert_eq!(ch13c_dot_i32(&a, &b), want, "n = {n}");
            assert_eq!(ch13c_dot_i32(&b, &a), want, "n = {n}, длины наоборот");
            let fa: Vec<f32> = (0..n).map(|_| (next() % 2001) as f32 / 1000.0 - 1.0).collect();
            let fb: Vec<f32> = (0..n).map(|_| (next() % 2001) as f32 / 1000.0 - 1.0).collect();
            let exact: f64 = fa.iter().zip(&fb).map(|(x, y)| f64::from(*x) * f64::from(*y)).sum();
            // Порядок сложений по элементам: ровно то же, что цикл.
            let mut seq = -0.0f32;
            for (x, y) in fa.iter().zip(&fb) {
                seq += x * y;
            }
            assert_eq!(ch13c_dot_f32(&fa, &fb).to_bits(), seq.to_bits(), "n = {n}");
            // Другой порядок даёт другое округление, но в пределах ошибки.
            let lanes = ch13c_dot_f32_lanes(&fa, &fb);
            let bound = 1e-6 * n as f64 + 1e-6;
            assert!((f64::from(lanes) - exact).abs() <= bound, "n = {n}: {lanes} и {exact}");
            assert_eq!(
                ch13c_dot_f32_lanes(&fa, &fb[..n / 2]),
                ch13c_dot_f32_lanes(&fa[..n / 2], &fb)
            );
        }
    }

    /// `i32` векторизуется, последовательная сумма `f32` — нет, восемь
    /// сумм — да.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_c_vectorized() {
        let a = emit_asm("x86_64-unknown-linux-gnu");
        let i = body(&a, "ch13c_dot_i32");
        assert!(count(&i, "paddd") >= 2, "ch13c_dot_i32 не векторизована: {i:?}");
        let f = body(&a, "ch13c_dot_f32");
        assert!(count(&f, "addss") >= 1, "{f:?}");
        assert_eq!(count(&f, "addps"), 0, "сумма f32 по порядку не может быть векторной: {f:?}");
        let l = body(&a, "ch13c_dot_f32_lanes");
        assert!(count(&l, "addps") >= 1 && count(&l, "mulps") >= 1, "ch13c_dot_f32_lanes: {l:?}");
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use sut::d_vector::{
        CHAINS, ch13d_broken, ch13d_fixed, multiples_of_3_once, parse_once, predicted_allocs,
    };

    #[test]
    fn task_d_same_sum() {
        let mut next = rng(5);
        for n in [0, 1, 2, 3, 15, 16, 17, 100, 1001] {
            let x: Vec<u32> = (0..n).map(|_| next()).collect();
            let want = x.iter().fold(0u32, |s, v| s.wrapping_add(*v));
            assert_eq!(ch13d_broken(&x), want, "n = {n}");
            assert_eq!(ch13d_fixed(&x), want, "n = {n}");
        }
    }

    /// Векторная сумма — `paddd` на x86-64: в `ch13d_broken` её нет, в
    /// `ch13d_fixed` есть.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_d_vectorization() {
        let a = emit_asm("x86_64-unknown-linux-gnu");
        let broken = body(&a, "ch13d_broken");
        let fixed = body(&a, "ch13d_fixed");
        assert_eq!(count(&broken, "paddd"), 0, "ch13d_broken векторизована: {broken:?}");
        assert!(count(&fixed, "paddd") >= 2, "ch13d_fixed не векторизована: {fixed:?}");
    }

    #[test]
    fn task_d_allocs() {
        let answers = predicted_allocs();
        let mut wrong = Vec::new();
        for (i, (name, run)) in CHAINS.iter().enumerate() {
            let v: Vec<u32> = (0..1000).collect();
            let (got, ()) = allocs(|| run(v));
            if got != answers[i] {
                wrong.push(format!(
                    "{}. {name}: ответ alloc {}, realloc {}; на деле {}, {}",
                    i + 1,
                    answers[i].0,
                    answers[i].1,
                    got.0,
                    got.1
                ));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn task_d_once() {
        let x: Vec<u32> = (0..1000).collect();
        let (calls, v) = allocs(|| multiples_of_3_once(&x));
        assert_eq!(v, (0..1000).step_by(3).collect::<Vec<u32>>());
        assert_eq!(calls, (1, 0), "одна аллокация и ни одного realloc");
        assert_eq!(v.capacity(), v.len(), "без лишней ёмкости");
        let words: Vec<String> = (0..500).map(|i| (i * 7).to_string()).collect();
        let refs: Vec<&str> = words.iter().map(String::as_str).collect();
        let (calls, r) = allocs(|| parse_once(&refs));
        assert_eq!(r.unwrap(), (0..500).map(|i| i * 7).collect::<Vec<u16>>());
        assert_eq!(calls, (1, 0));
        assert!(parse_once(&["1", "x"]).is_err());
        assert!(parse_once(&["70000"]).is_err());
        assert_eq!(parse_once(&[]), Ok(vec![]));
    }
}
