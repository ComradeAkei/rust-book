// Общие тесты задач главы 1.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch01/tests/ch01.rs` и `solutions/ch01/tests/ch01.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов.
//
// Задача C проверяет не только функцию, но и сборку с разными профилями,
// поэтому её второй тест сам запускает cargo в отдельном каталоге
// (`target/tmp/ch01-probe`). Под Miri он пропускается, как и полный перебор
// в задаче B2: интерпретатору не по силам миллиард итераций.

/// Пакет, который подключил этот файл: заготовки или решения.
const SUT_PKG: &str = env!("CARGO_PKG_NAME");

/// Детерминированный генератор, чтобы не тянуть крейт rand.
fn xorshift64(s: &mut u64) -> u64 {
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    *s
}

/// Сколько случайных проверок делать: Miri в тысячи раз медленнее.
const RANDOM_CASES: usize = if cfg!(miri) { 300 } else { 200_000 };

mod tool {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};
    use std::time::{Duration, Instant};

    pub fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    pub fn scratch(name: &str) -> PathBuf {
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch01-{name}"))
    }

    /// `cargo build` с отдельным каталогом сборки. Внешний cargo передаёт
    /// путь к себе в переменной CARGO, так что версия совпадёт.
    pub fn cargo_build(args: &[&str], target_dir: &Path) {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let out = Command::new(cargo)
            .arg("build")
            .args(args)
            .arg("--target-dir")
            .arg(target_dir)
            .current_dir(workspace_root())
            .output()
            .expect("не удалось запустить cargo");
        assert!(
            out.status.success(),
            "cargo build {args:?}:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Запуск собранной программы с ограничением по времени.
    pub fn run(binary: &Path) -> Output {
        let mut child = Command::new(binary)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|e| panic!("{}: {e}", binary.display()));
        let start = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if start.elapsed() > Duration::from_secs(10) {
                child.kill().ok();
                panic!("{} не завершилась за 10 секунд", binary.display());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        child.wait_with_output().unwrap()
    }
}

mod task_a {
    use super::sut::a_casts::PREDICTIONS;
    use ch01::task_a::{EXPRESSIONS, actual};

    #[test]
    fn predictions_match() {
        let mut wrong = Vec::new();
        for (i, ((expr, guess), truth)) in
            EXPRESSIONS.iter().zip(PREDICTIONS).zip(actual()).enumerate()
        {
            match guess {
                None => wrong.push(format!("№{:<2} {expr:<38} ответ не заполнен", i + 1)),
                Some(guess) if guess != truth => wrong.push(format!(
                    "№{:<2} {expr:<38} ваш ответ: {guess}, на самом деле: {truth}",
                    i + 1
                )),
                Some(_) => {}
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }
}

mod task_b1 {
    use super::sut::b1_u128::*;
    use super::{RANDOM_CASES, xorshift64};

    /// Значения, на которых ломаются переносы между половинами и столбцами.
    const EDGES: [u128; 14] = [
        0,
        1,
        2,
        0xFFFF_FFFF,
        1 << 32,
        u64::MAX as u128,
        1 << 64,
        (1 << 64) + 1,
        0xFFFF_FFFF_0000_0000_FFFF_FFFF_0000_0000,
        (1 << 127) - 1,
        1 << 127,
        u128::MAX - 1,
        u128::MAX,
        0x0123_4567_89AB_CDEF_FEDC_BA98_7654_3210,
    ];

    /// Случайное число, у которого каждая 32-битная четверть с заметной
    /// вероятностью состоит из одних нулей или одних единиц: так переносы
    /// через границы четвертей случаются часто, а не раз в 2³² попыток.
    fn structured(s: &mut u64) -> u128 {
        let mut x = 0u128;
        for _ in 0..4 {
            let r = xorshift64(s);
            let quarter = match r % 4 {
                0 => 0,
                1 => u32::MAX,
                _ => (r >> 32) as u32,
            };
            x = x << 32 | quarter as u128;
        }
        x
    }

    fn pairs() -> impl Iterator<Item = (u128, u128)> {
        let mut s = 0x9E37_79B9_7F4A_7C15;
        let edges = EDGES.iter().flat_map(|&a| EDGES.iter().map(move |&b| (a, b)));
        edges.chain((0..RANDOM_CASES).map(move |_| (structured(&mut s), structured(&mut s))))
    }

    #[test]
    fn conversions_are_halves() {
        // Помощники даны в заготовке, но проверим и их: иначе тесты ниже
        // сравнивали бы не то.
        assert_eq!(U128::from_u128(1 << 64), U128 { lo: 0, hi: 1 });
        assert_eq!(U128::from_u128(7), U128 { lo: 7, hi: 0 });
        assert_eq!(U128 { lo: 1, hi: 2 }.to_u128(), (2 << 64) + 1);
    }

    #[test]
    fn add_matches_native() {
        for (a, b) in pairs() {
            let (sum, carry) = a.overflowing_add(b);
            assert_eq!(
                ch01b_add(U128::from_u128(a), U128::from_u128(b)),
                (U128::from_u128(sum), carry),
                "{a:#x} + {b:#x}"
            );
        }
    }

    #[test]
    fn mul_wide_matches_native() {
        for (a, b) in pairs() {
            let (a, b) = (a as u64, (b >> 64) as u64);
            let product = a as u128 * b as u128;
            assert_eq!(ch01b_mul_wide(a, b), U128::from_u128(product), "{a:#x} · {b:#x}");
        }
    }

    #[test]
    fn mul_matches_native() {
        for (a, b) in pairs() {
            let product = a.wrapping_mul(b);
            assert_eq!(
                ch01b_mul(U128::from_u128(a), U128::from_u128(b)),
                U128::from_u128(product),
                "{a:#x} · {b:#x}"
            );
        }
    }

    #[test]
    fn natives_are_references() {
        assert_eq!(ch01b_add_native(u128::MAX, 1), (0, true));
        assert_eq!(
            ch01b_mul_wide_native(u64::MAX, u64::MAX),
            0xFFFF_FFFF_FFFF_FFFE_0000_0000_0000_0001
        );
        assert_eq!(ch01b_mul_native(1 << 64, 1 << 64), 0);
    }
}

mod task_b2 {
    use super::sut::b2_absorb::*;
    use super::{RANDOM_CASES, xorshift64};

    fn absorbs(x: f32) -> bool {
        x + 1.0 == x
    }

    fn check(x: f32) {
        assert!(x.is_finite() && x > 0.0, "нужно положительное конечное число, а не {x}");
        assert!(absorbs(x), "{x} + 1 = {}, это не {x}", x + 1.0);
        let bits = x.to_bits();
        let prev = f32::from_bits(bits - 1);
        assert!(!absorbs(prev), "предыдущее число {prev} тоже поглощает единицу");
        // Вся двоичная октава под ответом подряд и случайная выборка ниже:
        // полный перебор стоил бы столько же, сколько решение задачи.
        let octave = if cfg!(miri) { 1 << 10 } else { 1 << 23 };
        for below in bits.saturating_sub(octave)..bits {
            let y = f32::from_bits(below);
            assert!(!absorbs(y), "{y} меньше ответа, но тоже поглощает единицу");
        }
        let mut s = 0x2545_F491_4F6C_DD1D;
        for _ in 0..RANDOM_CASES {
            let y = f32::from_bits(1 + (xorshift64(&mut s) % u64::from(bits - 1)) as u32);
            assert!(!absorbs(y), "{y} меньше ответа, но тоже поглощает единицу");
        }
    }

    #[test]
    fn analytic_answer_is_smallest() {
        check(smallest_absorbing_analytic());
    }

    #[test]
    #[cfg_attr(miri, ignore = "перебирает больше миллиарда чисел")]
    fn search_agrees_with_analysis() {
        let found = smallest_absorbing_search();
        check(found);
        assert_eq!(found.to_bits(), smallest_absorbing_analytic().to_bits());
    }
}

mod task_c {
    use super::sut::c_probe::overflow_checks_enabled;
    use super::{SUT_PKG, tool};

    #[test]
    fn matches_test_build() {
        // Профили книги не разделяют проверки переполнения и
        // debug_assertions, поэтому в тестовой сборке они совпадают.
        assert_eq!(overflow_checks_enabled(), cfg!(debug_assertions));
    }

    /// Проверки переполнения включаются отдельно от debug_assertions: пример
    /// `probe` собирается с двумя сочетаниями, которых нет в профилях по
    /// умолчанию, и печатает ответ функции.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn independent_of_debug_assertions() {
        let dir = tool::scratch("probe");
        let cases: [(&[&str], &str, &str); 2] = [
            (&["--config", "profile.dev.overflow-checks=false"], "debug", "false"),
            (&["--release", "--config", "profile.release.overflow-checks=true"], "release", "true"),
        ];
        for (flags, profile_dir, expected) in cases {
            let mut args = vec!["-p", SUT_PKG, "--example", "probe"];
            args.extend_from_slice(flags);
            tool::cargo_build(&args, &dir);
            let out = tool::run(&dir.join(profile_dir).join("examples/probe"));
            assert!(out.status.success(), "probe {flags:?} завершилась с {:?}", out.status);
            assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), expected, "сборка {flags:?}");
        }
    }
}

mod task_d {
    use super::sut::d_fma::*;
    use super::{RANDOM_CASES, xorshift64};

    #[test]
    fn functions_are_what_they_say() {
        // Обе функции даны в заготовке. На точных входах они совпадают.
        assert_eq!(ch01d_separate(1.5, 2.0, 0.25), 3.25);
        assert_eq!(ch01d_fused(1.5, 2.0, 0.25), 3.25);
    }

    #[test]
    fn witnesses_differ() {
        let (a, b, c) = DIFFER;
        assert!([a, b, c].iter().all(|x| x.is_finite()), "нужны конечные числа: {DIFFER:?}");
        let (separate, fused) = (ch01d_separate(a, b, c), ch01d_fused(a, b, c));
        assert!(separate.is_finite() && fused.is_finite(), "{separate} и {fused}");
        assert_ne!(
            separate.to_bits(),
            fused.to_bits(),
            "на {DIFFER:?} оба способа дают {separate:e}"
        );
    }

    #[test]
    fn hidden_rounding_error() {
        let (a, b, c) = CANCEL;
        assert!([a, b, c].iter().all(|x| x.is_finite()), "нужны конечные числа: {CANCEL:?}");
        assert_eq!(ch01d_separate(a, b, c), 0.0, "раздельный способ должен дать ровно ноль");
        let fused = ch01d_fused(a, b, c);
        assert_ne!(fused, 0.0, "слитный способ должен увидеть ошибку округления");
        // f32 · f32 точно помещается в f64: 24 + 24 бита мантиссы меньше 53.
        let exact = f64::from(a) * f64::from(b) + f64::from(c);
        assert_eq!(f64::from(fused), exact, "FMA обязана дать точное значение");
    }

    /// Случайное f32 с порядком в пределах 2^±40: ни произведение, ни его
    /// ошибка не выходят за диапазон и не становятся денормалами.
    fn moderate(s: &mut u64) -> f32 {
        let r = xorshift64(s);
        let sign = (r >> 63) as u32;
        let exponent = 127 - 40 + (r >> 32) as u32 % 81;
        f32::from_bits(sign << 31 | exponent << 23 | (r as u32 & 0x7F_FFFF))
    }

    #[test]
    fn product_error_is_exact() {
        let mut s = 0xD1B5_4A32_D192_ED03;
        let edges = [(1.0, 1.0), (CANCEL.0, CANCEL.1), (0.1, 10.0), (3.0, 1.0 / 3.0), (-7.0, 0.1)];
        let random = (0..RANDOM_CASES).map(|_| (moderate(&mut s), moderate(&mut s)));
        for (a, b) in edges.into_iter().chain(random) {
            let (p, e) = product_error(a, b);
            assert_eq!(
                p.to_bits(),
                (a * b).to_bits(),
                "{a:e} · {b:e}: p должно быть обычным произведением"
            );
            let exact = f64::from(a) * f64::from(b);
            assert_eq!(f64::from(p) + f64::from(e), exact, "{a:e} · {b:e} ≠ {p:e} + {e:e}");
        }
    }
}
