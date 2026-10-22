// Общие тесты задач главы 7.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch07/tests/ch07.rs` и `solutions/ch07/tests/ch07.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A собирает фрагменты компилятором, задача C — программу
// `code/ch07/c-size` через cargo, задача D запускает программу `d_double` и
// повторно сам тестовый бинарник: часть опытов завершает процесс через
// abort. Под Miri эти тесты пропускаются.

use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Каталог крейта с задачами: заготовки или решения.
fn crate_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Каталог для файлов, которые тест создаёт сам.
fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("{}-{name}", env!("CARGO_PKG_NAME")));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

const SIGABRT: i32 = 6;

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_chains::{Verdict, answers};

    /// Собрать текст как библиотеку: успех и текст ошибок.
    fn compile(name: &str, src: &str) -> (bool, String) {
        let dir = tmp("a");
        let file = dir.join(format!("{name}.rs"));
        std::fs::write(&file, src).unwrap();
        let out = Command::new("rustc")
            .current_dir(crate_dir())
            .env_remove("RUSTUP_TOOLCHAIN")
            .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata"])
            .args(["-A", "warnings", "-o"])
            .arg(dir.join(format!("{name}.rmeta")))
            .arg(&file)
            .output()
            .expect("не удалось запустить rustc");
        (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
    }

    fn fragment(n: usize) -> String {
        let p = crate_dir().join(format!("../../code/ch07/fragments/a{n}.rs"));
        std::fs::read_to_string(p).unwrap()
    }

    fn with(src: &str, impls: &[&str]) -> String {
        let mut s = src.to_string();
        for i in impls {
            s.push('\n');
            s.push_str(i);
            s.push('\n');
        }
        s
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_a_verdicts() {
        let mut wrong = Vec::new();
        for (i, verdict) in answers().iter().enumerate() {
            let n = i + 1;
            let src = fragment(n);
            let (ok, err) = compile(&format!("a{n}"), &src);
            match verdict {
                Verdict::Compiles if ok => {}
                Verdict::Compiles => wrong.push(format!("a{n}: не компилируется:\n{err}")),
                _ if ok => wrong.push(format!("a{n}: компилируется как есть")),
                Verdict::Never => {
                    if !err.contains("the `?` operator can only be used on") {
                        wrong.push(format!("a{n}: эту ошибку исправляет реализация трейта:\n{err}"));
                    }
                }
                Verdict::Missing(impls) => {
                    assert!(!impls.is_empty(), "a{n}: пустой список реализаций");
                    let (ok, err) = compile(&format!("a{n}_fixed"), &with(&src, impls));
                    if !ok {
                        wrong.push(format!("a{n}: с вашими реализациями не компилируется:\n{err}"));
                        continue;
                    }
                    for skip in 0..impls.len() {
                        if impls.len() == 1 {
                            break;
                        }
                        let rest: Vec<&str> = impls
                            .iter()
                            .enumerate()
                            .filter(|&(j, _)| j != skip)
                            .map(|(_, s)| *s)
                            .collect();
                        if compile(&format!("a{n}_less"), &with(&src, &rest)).0 {
                            wrong.push(format!("a{n}: лишняя реализация: {}", impls[skip]));
                        }
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use std::error::Error;
    use sut::b_config::{ConfigError, Sensor, StationConfig, load, parse, report};

    const GOOD: &str = "\
# станция №3
[uart]
baud = 57600

[sensor.1]
period_ms = 100

[sensor.7]   ; давление
period_ms = \"250\"
";

    fn err(src: &str) -> String {
        report(&parse(src).unwrap_err())
    }

    #[test]
    fn task_b_good_config() {
        let cfg = parse(GOOD).unwrap();
        let want = StationConfig {
            baud: 57600,
            sensors: vec![Sensor { id: 1, period_ms: 100 }, Sensor { id: 7, period_ms: 250 }],
        };
        assert_eq!(cfg, want);
        let cfg = parse("[sensor.65535]\nperiod_ms=10\n").unwrap();
        assert_eq!(cfg.baud, 115200, "скорость по умолчанию");
        assert_eq!(cfg.sensors, [Sensor { id: 65535, period_ms: 10 }]);
        assert_eq!(parse("").unwrap().sensors, []);
    }

    #[test]
    fn task_b_syntax_errors() {
        let cases = [
            ("[uart", "строка 1, столбец 1: нет закрывающей ]"),
            ("[uart]\nba$d = 1", "строка 2, столбец 3: недопустимое имя"),
            ("[uart]\n\nbaud 9600", "строка 3, столбец 6: нет знака ="),
            ("[uart]\nname = \"abc", "строка 2, столбец 8: нет закрывающей кавычки"),
            ("[uart] x", "строка 1, столбец 8: лишние символы в конце строки"),
        ];
        for (src, why) in cases {
            assert_eq!(err(src), format!("ошибка: синтаксическая ошибка\n  причина: {why}\n"), "{src:?}");
        }
    }

    #[test]
    fn task_b_semantic_errors() {
        let cases = [
            ("baud = 9600", 1, "ключ baud вне секции"),
            ("[uart]\n[can]", 2, "неизвестная секция [can]"),
            ("[sensor.0]", 1, "неизвестная секция [sensor.0]"),
            ("[sensor.x1]", 1, "неизвестная секция [sensor.x1]"),
            ("[uart]\nparity = even", 2, "неизвестный ключ parity в секции [uart]"),
            ("[sensor.2]\nbaud = 9600", 2, "неизвестный ключ baud в секции [sensor.2]"),
            ("[uart]\nbaud = 1200", 2, "скорость 1200 не поддерживается"),
            ("[sensor.3]\nperiod_ms = 5", 2, "period_ms = 5 вне допустимого диапазона"),
            ("[sensor.3]\nperiod_ms = 60001", 2, "period_ms = 60001 вне допустимого диапазона"),
            ("[sensor.4]\nperiod_ms = 10\n\n[sensor.4]", 4, "датчик 4 описан дважды"),
            ("# x\n[sensor.5]\n[uart]", 2, "в секции [sensor.5] нет ключа period_ms"),
            ("[uart]\n\n[sensor.6]\n", 3, "в секции [sensor.6] нет ключа period_ms"),
        ];
        for (src, line, why) in cases {
            let want = format!("ошибка: ошибка в строке {line}\n  причина: {why}\n");
            assert_eq!(err(src), want, "{src:?}");
        }
    }

    #[test]
    fn task_b_number_chain() {
        let e = parse("[uart]\nbaud = 9600\n[sensor.9]\n  period_ms = 1x0\n").unwrap_err();
        assert_eq!(
            report(&e),
            "ошибка: ошибка в строке 4\n  \
             причина: значение period_ms не число\n  \
             причина: invalid digit found in string\n"
        );
        let mut cur: &(dyn Error + 'static) = &e;
        while let Some(next) = cur.source() {
            cur = next;
        }
        assert!(cur.downcast_ref::<std::num::ParseIntError>().is_some(), "внизу цепочки ParseIntError");
        let e = err("[uart]\nbaud =\n");
        assert!(e.ends_with("причина: cannot parse integer from empty string\n"), "{e}");
    }

    #[test]
    #[cfg_attr(miri, ignore = "обращается к файловой системе")]
    fn task_b_files() {
        let e = load(Path::new("/nonexistent/station.ini")).unwrap_err();
        assert_eq!(
            report(&e),
            "ошибка: не удалось прочитать /nonexistent/station.ini\n  \
             причина: No such file or directory (os error 2)\n"
        );
        let io = e.source().unwrap().downcast_ref::<std::io::Error>().unwrap();
        assert_eq!(io.kind(), std::io::ErrorKind::NotFound);

        let path = tmp("b").join("station.ini");
        std::fs::write(&path, "[sensor.2]\nperiod_ms = 7ms\n").unwrap();
        assert_eq!(
            report(&load(&path).unwrap_err()),
            format!(
                "ошибка: {}: конфигурация отвергнута\n  \
                 причина: ошибка в строке 2\n  \
                 причина: значение period_ms не число\n  \
                 причина: invalid digit found in string\n",
                path.display()
            )
        );
        std::fs::write(&path, GOOD).unwrap();
        assert_eq!(load(&path).unwrap().sensors.len(), 2);
    }

    /// Сообщение звена не повторяет сообщение причины: иначе отчёт
    /// печатает одно и то же дважды.
    #[test]
    #[cfg_attr(miri, ignore = "обращается к файловой системе")]
    fn task_b_no_repeated_messages() {
        let path = tmp("b").join("twice.ini");
        std::fs::write(&path, "[sensor.2]\nperiod_ms = 7ms\n").unwrap();
        let errors = [
            parse("[uart").unwrap_err(),
            parse("[uart]\nbaud = x").unwrap_err(),
            load(&path).unwrap_err(),
            load(Path::new("/nonexistent/x.ini")).unwrap_err(),
        ];
        for e in &errors {
            let mut cur: &(dyn Error + 'static) = e;
            while let Some(next) = cur.source() {
                let (a, b) = (cur.to_string(), next.to_string());
                assert!(!a.contains(&b), "«{a}» повторяет причину «{b}»");
                cur = next;
            }
        }
    }

    #[test]
    fn task_b_error_type_is_small_and_sendable() {
        fn sendable<E: Error + Send + Sync + 'static>() {}
        sendable::<ConfigError>();
        let size = std::mem::size_of::<ConfigError>();
        assert!(size <= 40, "ConfigError занимает {size} байт, нужно не больше 40");
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use sut::c_cost::{ABORT, Overflow, Sizes, UNWIND, ch07c_scale, sum_code, sum_result};

    /// Размеры секций ELF64 по именам: разбор заголовка и таблицы секций.
    fn sections(path: &Path) -> Vec<(String, u64)> {
        let f = std::fs::read(path).unwrap();
        assert_eq!(&f[..4], b"\x7fELF");
        let u16_at = |o: usize| u16::from_le_bytes(f[o..o + 2].try_into().unwrap()) as usize;
        let u32_at = |o: usize| u32::from_le_bytes(f[o..o + 4].try_into().unwrap()) as usize;
        let u64_at = |o: usize| u64::from_le_bytes(f[o..o + 8].try_into().unwrap());
        let (shoff, shentsize, shnum, shstrndx) =
            (u64_at(0x28) as usize, u16_at(0x3A), u16_at(0x3C), u16_at(0x3E));
        let sh = |i: usize| shoff + i * shentsize;
        let strtab = u64_at(sh(shstrndx) + 0x18) as usize;
        (0..shnum)
            .map(|i| {
                let name = strtab + u32_at(sh(i));
                let end = f[name..].iter().position(|&b| b == 0).unwrap();
                let name = String::from_utf8_lossy(&f[name..name + end]).into_owned();
                (name, u64_at(sh(i) + 0x20))
            })
            .collect()
    }

    fn measure(strategy: &str) -> Sizes {
        let dir = crate_dir().join("../../code/ch07/c-size");
        let target = tmp(&format!("c-{strategy}"));
        let st = Command::new(env!("CARGO"))
            .current_dir(&dir)
            .env_remove("RUSTUP_TOOLCHAIN")
            .env_remove("RUSTFLAGS")
            .arg("build")
            .arg("--target-dir")
            .arg(&target)
            .args(["--release", "-q", "--config"])
            .arg(format!("profile.release.panic=\"{strategy}\""))
            .status()
            .unwrap();
        assert!(st.success());
        let bin = target.join("release/ch07-c-size");
        let s = sections(&bin);
        let get = |n: &str| s.iter().find(|(name, _)| name == n).map_or(0, |&(_, sz)| sz);
        Sizes {
            file: std::fs::metadata(&bin).unwrap().len(),
            text: get(".text"),
            eh_frame: get(".eh_frame"),
            eh_frame_hdr: get(".eh_frame_hdr"),
            gcc_except_table: get(".gcc_except_table"),
        }
    }

    fn close(what: &str, yours: u64, real: u64) -> Option<String> {
        let ok = yours.abs_diff(real) * 10 <= real;
        (!ok).then(|| format!("{what}: у вас {yours}, измерено {real}"))
    }

    fn compare(name: &str, yours: Sizes, real: Sizes) -> Vec<String> {
        [
            close(&format!("{name}.file"), yours.file, real.file),
            close(&format!("{name}.text"), yours.text, real.text),
            close(&format!("{name}.eh_frame"), yours.eh_frame, real.eh_frame),
            close(&format!("{name}.eh_frame_hdr"), yours.eh_frame_hdr, real.eh_frame_hdr),
            close(&format!("{name}.gcc_except_table"), yours.gcc_except_table, real.gcc_except_table),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Ваши числа должны совпасть с замером в пределах 10 %: точные
    /// размеры зависят от версии компоновщика.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_c_sizes() {
        let (unwind, abort) = (measure("unwind"), measure("abort"));
        let mut wrong = compare("UNWIND", UNWIND, unwind);
        wrong.extend(compare("ABORT", ABORT, abort));
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert!(abort.eh_frame <= unwind.eh_frame && abort.file <= unwind.file);
    }

    #[test]
    fn task_c_result_and_code_agree() {
        let mut x = 0x2545_f491u32;
        let mut samples = Vec::new();
        for _ in 0..1000 {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            samples.push(x % 50_000);
        }
        for gain in [0, 1, 7, 85_000, 90_000, u32::MAX] {
            let mut out = 0;
            let code = sum_code(&samples, gain, &mut out);
            match sum_result(&samples, gain) {
                Ok(s) => assert_eq!((code, out), (0, s), "gain = {gain}"),
                Err(Overflow) => assert_eq!(code, -1, "gain = {gain}"),
            }
        }
        assert_eq!(ch07c_scale(70_000, 70_000), Err(Overflow));
        assert_eq!(sum_result(&[1, 2, 3], 10), Ok(60));
        assert_eq!(sum_result(&[], 10), Ok(0));
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};
    use sut::d_writer::{Buffered, CAP, Sink, SinkError, Unsent};

    #[test]
    #[cfg_attr(miri, ignore = "запускает процессы")]
    fn task_d_double_panic_aborts() {
        let out = Command::new(env!("CARGO_BIN_EXE_d_double"))
            .env_remove("RUST_BACKTRACE")
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.signal(), Some(SIGABRT), "ожидали abort:\n{err}");
        assert!(err.contains("panic in a destructor during cleanup"), "{err}");
    }

    /// Приёмник для опытов: принимает не больше `chunk` байт за вызов,
    /// после `fail_after` принятых байт отвечает ошибкой, а с `explode`
    /// паникует при любом вызове.
    #[derive(Debug)]
    struct TestSink {
        got: Vec<u8>,
        chunk: usize,
        fail_after: usize,
        explode: bool,
    }

    impl TestSink {
        fn new(chunk: usize) -> Self {
            TestSink { got: Vec::new(), chunk, fail_after: usize::MAX, explode: false }
        }
    }

    impl Sink for TestSink {
        fn send(&mut self, data: &[u8]) -> Result<usize, SinkError> {
            if self.explode {
                panic!("приёмник взорвался");
            }
            let room = self.fail_after - self.got.len();
            if room == 0 {
                return Err(SinkError::Disconnected);
            }
            let n = data.len().min(self.chunk).min(room);
            self.got.extend_from_slice(&data[..n]);
            Ok(n)
        }
    }

    fn data(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 + i / 256) as u8).collect()
    }

    #[test]
    fn task_d_writes_everything_in_order() {
        let want = data(1000);
        let mut w = Buffered::new(TestSink::new(7));
        let mut rest = &want[..];
        let mut step = 1;
        while !rest.is_empty() {
            let n = step.min(rest.len());
            w.write(&rest[..n]).unwrap();
            rest = &rest[n..];
            step = step * 3 % 101 + 1;
        }
        let sink = w.finish().unwrap();
        assert_eq!(sink.got, want);
    }

    #[test]
    fn task_d_buffer_holds_until_full() {
        let mut w = Buffered::new(TestSink::new(usize::MAX));
        w.write(&data(CAP)).unwrap();
        w.write(&[1]).unwrap();
        w.flush().unwrap();
        let sink = w.finish().unwrap();
        assert_eq!(sink.got.len(), CAP + 1);
    }

    #[test]
    fn task_d_finish_reports_unsent() {
        let mut sink = TestSink::new(5);
        sink.fail_after = 12;
        let mut w = Buffered::new(sink);
        w.write(&data(40)).unwrap();
        let Err(Unsent { sink, pending, error }) = w.finish() else {
            panic!("finish должен вернуть ошибку");
        };
        assert_eq!(error, SinkError::Disconnected);
        assert_eq!(sink.got, data(12));
        assert_eq!(pending, 28);
    }

    #[test]
    fn task_d_drop_flushes_and_ignores_errors() {
        let mut sink = TestSink::new(4);
        sink.fail_after = 3;
        let r = catch_unwind(AssertUnwindSafe(|| {
            let mut w = Buffered::new(sink);
            w.write(b"abcdef").unwrap();
        }));
        assert!(r.is_ok(), "деструктор не должен паниковать из-за ошибки приёмника");
    }

    /// Запускает тест `name` этого же бинарника отдельным процессом с
    /// переменной окружения `CH07_D_CHILD`: так abort в опыте не роняет
    /// остальные тесты. Возвращает статус и stderr ребёнка.
    fn child(name: &str) -> (std::process::ExitStatus, String) {
        let out = Command::new(std::env::current_exe().unwrap())
            .args([name, "--exact", "--nocapture", "--test-threads=1"])
            .env("CH07_D_CHILD", "1")
            .env_remove("RUST_BACKTRACE")
            .output()
            .unwrap();
        (out.status, String::from_utf8_lossy(&out.stderr).into_owned())
    }

    fn in_child() -> bool {
        std::env::var_os("CH07_D_CHILD").is_some()
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает процессы")]
    fn task_d_drop_during_unwind() {
        if in_child() {
            let mut sink = TestSink::new(4);
            sink.fail_after = 0;
            let mut w = Buffered::new(sink);
            w.write(b"abc").unwrap();
            panic!("паника при живом Buffered");
        }
        let (st, err) = child("task_d::task_d_drop_during_unwind");
        assert_ne!(st.signal(), Some(SIGABRT), "abort:\n{err}");
        assert!(err.contains("паника при живом Buffered"), "{err}");
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает процессы")]
    fn task_d_sink_panics() {
        if in_child() {
            let mut sink = TestSink::new(4);
            sink.explode = true;
            let mut w = Buffered::new(sink);
            w.write(&data(CAP + 1)).unwrap();
            unreachable!("send должен был запаниковать");
        }
        let (st, err) = child("task_d::task_d_sink_panics");
        assert_ne!(st.signal(), Some(SIGABRT), "abort:\n{err}");
        assert!(err.contains("приёмник взорвался"), "{err}");
        assert!(!err.contains("unreachable"), "{err}");
    }
}
