// Общие тесты задач главы 0.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch00/tests/ch00.rs` и `solutions/ch00/tests/ch00.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов.
//
// Задачи A и B проверяют не код, а результат сборки, поэтому тесты сами
// запускают cargo и читают получившиеся файлы. Каталог сборки отдельный
// (`target/tmp/ch00-*`), чтобы не мешать внешнему cargo. Под Miri такие тесты
// пропускаются: интерпретатор не запускает внешние процессы.

/// Каталог крейта, который подключил этот файл: заготовки или решения.
/// Задача B ищет там `tiny.toml` и крейт `hello-nostd`.
const SUT_DIR: &str = env!("CARGO_MANIFEST_DIR");

mod tool {
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};
    use std::time::{Duration, Instant};

    pub fn workspace_root() -> PathBuf {
        Path::new(super::SUT_DIR).join("../..")
    }

    pub fn scratch(name: &str) -> PathBuf {
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch00-{name}"))
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

    /// Таблица символов через `nm` из binutils или `llvm-nm`: строки вида
    /// «адрес тип имя», только символы кода (типы t и T).
    pub fn text_symbols(binary: &Path) -> Vec<String> {
        for nm in ["nm", "llvm-nm"] {
            if let Ok(out) = Command::new(nm).arg(binary).output() {
                assert!(out.status.success(), "{nm} {}", binary.display());
                return String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .filter_map(|l| {
                        let mut it = l.split_whitespace();
                        let (_, kind, name) = (it.next()?, it.next()?, it.next()?);
                        matches!(kind, "t" | "T").then(|| name.to_owned())
                    })
                    .collect();
            }
        }
        panic!("нужен nm из binutils или llvm-nm из LLVM: см. приложение D");
    }

    /// Есть ли в таблице функция `name` из крейта примера. Подходит и для
    /// схемы манглинга v0 (`_RNv...8checksum`), и для старой (`_ZN...8checksum17h...E`).
    pub fn has_fn(symbols: &[String], name: &str) -> bool {
        let needle = format!("{}{}", name.len(), name);
        symbols
            .iter()
            .any(|s| s == name || s.ends_with(&needle) || s.contains(&format!("{needle}17h")))
    }

    /// Запуск собранной программы с ограничением по времени: заготовка с
    /// ошибкой может зависнуть, и тест не должен висеть вместе с ней.
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
    use super::sut::a_vanish::SURVIVES;
    use super::tool;
    use ch00::task_a::FUNCTIONS;

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo и nm")]
    fn predictions_match_release_binary() {
        let dir = tool::scratch("vanish");
        tool::cargo_build(&["--release", "-p", "ch00", "--example", "vanish"], &dir);
        let symbols = tool::text_symbols(&dir.join("release/examples/vanish"));
        // Санитарная проверка: если не нашлась даже main, сломан сам поиск.
        assert!(tool::has_fn(&symbols, "main"), "в таблице символов нет функций примера");

        let mut wrong = Vec::new();
        for (i, (name, answer)) in FUNCTIONS.iter().zip(SURVIVES).enumerate() {
            let Some(answer) = answer else {
                wrong.push(format!("№{} {name:<16} ответ не заполнен", i + 1));
                continue;
            };
            let truth = tool::has_fn(&symbols, name);
            if truth != answer {
                let word = |b: bool| if b { "есть" } else { "нет" };
                wrong.push(format!(
                    "№{} {name:<16} ваш ответ: {}, на самом деле: {}",
                    i + 1,
                    word(answer),
                    word(truth)
                ));
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }
}

mod task_b {
    use super::tool;
    use std::path::Path;

    /// Без флагов release-сборка hello весит около 435 КиБ. Порог требует
    /// убрать символы и включить LTO, ни один флаг в одиночку его не проходит.
    const STD_LIMIT: u64 = 300_000;
    /// Программа без std и без main из std укладывается в несколько КиБ.
    const NOSTD_LIMIT: u64 = 16 * 1024;

    fn check_hello(binary: &Path, limit: u64) {
        let out = tool::run(binary);
        assert!(out.status.success(), "программа завершилась с {:?}", out.status);
        assert_eq!(String::from_utf8_lossy(&out.stdout), "Hello, world!\n");
        let size = std::fs::metadata(binary).unwrap().len();
        assert!(size <= limit, "{}: {size} байт, нужно не больше {limit}", binary.display());
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn tiny_profile() {
        let dir = tool::scratch("tiny");
        let config = Path::new(super::SUT_DIR).join("tiny.toml");
        let config = config.to_str().unwrap();
        tool::cargo_build(
            &["--release", "-p", "ch00", "--example", "hello", "--config", config],
            &dir,
        );
        check_hello(&dir.join("release/examples/hello"), STD_LIMIT);
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn hello_without_std() {
        let dir = tool::scratch("nostd");
        let manifest = Path::new(super::SUT_DIR).join("hello-nostd/Cargo.toml");
        tool::cargo_build(&["--release", "--manifest-path", manifest.to_str().unwrap()], &dir);
        check_hello(&dir.join("release/hello-nostd"), NOSTD_LIMIT);
    }
}

mod task_c {
    use super::sut::c_sum::*;

    /// Детерминированный генератор, чтобы не тянуть крейт rand.
    fn xorshift(n: usize, mut s: u32) -> Vec<u32> {
        (0..n)
            .map(|_| {
                s ^= s << 13;
                s ^= s >> 17;
                s ^= s << 5;
                s
            })
            .collect()
    }

    #[test]
    fn empty_and_single() {
        assert_eq!(ch00c_sum_wrapping(&[]), 0);
        assert_eq!(ch00c_sum_wide(&[]), 0);
        assert_eq!(ch00c_sum_checked(&[]), Some(0));
        assert_eq!(ch00c_sum_wrapping(&[42]), 42);
        assert_eq!(ch00c_sum_wide(&[u32::MAX]), u32::MAX as u64);
        assert_eq!(ch00c_sum_checked(&[u32::MAX]), Some(u32::MAX));
    }

    #[test]
    fn overflow_boundary() {
        assert_eq!(ch00c_sum_wrapping(&[u32::MAX, 2]), 1);
        assert_eq!(ch00c_sum_wide(&[u32::MAX, 2]), 0x1_0000_0001);
        assert_eq!(ch00c_sum_checked(&[u32::MAX - 1, 1]), Some(u32::MAX));
        assert_eq!(ch00c_sum_checked(&[u32::MAX, 1]), None);
        // Переполнение посередине: итог снова «влез бы» по модулю 2³², но
        // точная сумма больше u32::MAX.
        assert_eq!(ch00c_sum_checked(&[u32::MAX, 1, u32::MAX]), None);
    }

    #[test]
    fn all_lengths_up_to_67() {
        // Длины вокруг ширины векторного цикла и его развёртки: хвост
        // обрабатывается отдельной веткой, и ошибки обычно прячутся там.
        let data = xorshift(67, 0x1234_5678);
        for n in 0..=data.len() {
            let xs = &data[..n];
            let exact: u64 = xs.iter().map(|&x| x as u64).sum();
            assert_eq!(ch00c_sum_wide(xs), exact, "n = {n}");
            assert_eq!(ch00c_sum_wrapping(xs), exact as u32, "n = {n}");
            assert_eq!(ch00c_sum_checked(xs), u32::try_from(exact).ok(), "n = {n}");
        }
    }

    #[test]
    fn small_values_never_overflow() {
        let xs: Vec<u32> = xorshift(1000, 7).into_iter().map(|x| x >> 12).collect();
        let exact: u64 = xs.iter().map(|&x| x as u64).sum();
        assert!(exact <= u32::MAX as u64);
        assert_eq!(ch00c_sum_checked(&xs), Some(exact as u32));
    }
}
