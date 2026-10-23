// Общие тесты задач главы 12.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch12/tests/ch12.rs` и `solutions/ch12/tests/ch12.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A сверяет ответы ещё и при сборке для Cortex-M3, задача C — код
// парсеров в ассемблере, задача D — где именно компилятор сообщает об
// ошибке; все три собирают пробы закреплённым cargo и под Miri
// пропускаются.

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

/// Имя библиотеки крейта в именах файлов: `ch12-solutions` → `ch12_solutions`.
fn lib_name() -> String {
    env!("CARGO_PKG_NAME").replace('-', "_")
}

/// Пробный крейт в `tmp(name)`, который зависит от крейта задач с
/// возможностями `features` и состоит из `src`.
fn probe(name: &str, default_features: bool, features: &[&str], src: &str) -> PathBuf {
    let dir = tmp(name);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let manifest = format!(
        "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\nsut = {{ package = \"{}\", path = {:?}, default-features = {default_features}, \
         features = {features:?} }}\n\n[workspace]\n",
        env!("CARGO_PKG_NAME"),
        crate_dir()
    );
    std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
    std::fs::write(dir.join("src/lib.rs"), src).unwrap();
    dir
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use sut::a_sizes::{measured, predicted};

    const NAMES: [&str; 8] = [
        "|| 7u8",
        "|| x",
        "move || x",
        "move || (a, b)",
        "|| s.len()",
        "|| r.value + 1",
        "move || r.value + r.id as i32",
        "move || inner() + 1",
    ];

    #[test]
    fn task_a_host() {
        let answers = predicted(size_of::<usize>());
        let actual = measured();
        let wrong: Vec<String> = (0..8)
            .filter(|&i| answers[i] != actual[i])
            .map(|i| format!("c{i} = {}: ответ {}, на деле {}", NAMES[i], answers[i], actual[i]))
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// Те же замыкания при сборке для Cortex-M3, где указатель — 4 байта:
    /// `measured` и проверки в `const` вычисляет компилятор цели.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_a_thumb() {
        let mut src = String::from(
            "#![no_std]\nuse sut::a_sizes::{measured, predicted};\n\
             const M: [usize; 8] = measured();\nconst P: [usize; 8] = predicted(4);\n",
        );
        for (i, name) in NAMES.iter().enumerate() {
            src += &format!("const _: () = assert!(M[{i}] == P[{i}], \"c{i} = {name}\");\n");
        }
        let dir = probe("a-thumb", false, &[], &src);
        let out = pinned("cargo", &dir)
            .args(["build", "--target", "thumbv7m-none-eabi", "--target-dir"])
            .arg(dir.join("target"))
            .output()
            .unwrap();
        assert!(out.status.success(), "ответы не сошлись для Cortex-M3:\n{}", text(&out.stderr));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::sut;
    use std::cell::RefCell;
    use sut::b_registry::{Chain, Event, Full, Handlers, Registry};

    const EVENTS: [Event; 6] = [
        Event::Button(true),
        Event::Sample(100),
        Event::Tick(1),
        Event::Button(false),
        Event::Sample(-30),
        Event::Button(true),
    ];

    #[test]
    fn task_b_registry() {
        let mut presses = 0u32;
        let mut sum = 0i32;
        let mut last_tick = 0u32;
        let mut on_button = |e| {
            if e == Event::Button(true) {
                presses += 1;
            }
        };
        let mut on_sample = |e| {
            if let Event::Sample(v) = e {
                sum += i32::from(v);
            }
        };
        let mut on_tick = |e| {
            if let Event::Tick(t) = e {
                last_tick = t;
            }
        };
        {
            let mut r = Registry::<'_, 4>::new();
            assert_eq!(r.subscribe(&mut on_button), Ok(0));
            assert_eq!(r.subscribe(&mut on_sample), Ok(1));
            assert_eq!(r.subscribe(&mut on_tick), Ok(2));
            let calls: usize = EVENTS.iter().map(|&e| r.emit(e)).sum();
            assert_eq!(calls, 18, "каждое событие получают три обработчика");
            // Освобождённая ячейка возвращает заём и занимается снова.
            let h = r.unsubscribe(1).expect("ячейка 1 занята");
            h(Event::Sample(1));
            assert!(r.unsubscribe(1).is_none(), "ячейка 1 уже свободна");
            assert!(r.unsubscribe(9).is_none(), "ячейки 9 нет");
            assert_eq!(r.emit(Event::Sample(1000)), 2);
            assert_eq!(r.subscribe(h), Ok(1));
            assert_eq!(r.emit(Event::Tick(7)), 3);
        }
        assert_eq!((presses, sum, last_tick), (2, 71, 7));
    }

    #[test]
    fn task_b_registry_full() {
        let mut a = |_| {};
        let mut b = |_| {};
        let mut c = |_| {};
        let mut r = Registry::<'_, 2>::default();
        assert_eq!(r.emit(Event::Tick(0)), 0);
        assert_eq!(r.subscribe(&mut a), Ok(0));
        assert_eq!(r.subscribe(&mut b), Ok(1));
        assert_eq!(r.subscribe(&mut c), Err(Full));
        // Ссылка на объект трейта и `None` в нише: два слова на ячейку.
        assert_eq!(size_of::<Registry<'_, 2>>(), 2 * 2 * size_of::<usize>());
    }

    #[test]
    fn task_b_chain() {
        let log = RefCell::new(Vec::new());
        let mut chain = Chain::new()
            .with(|e| log.borrow_mut().push(("первый", e)))
            .with(|e| log.borrow_mut().push(("второй", e)));
        assert_eq!(chain.emit(Event::Tick(5)), 2);
        assert_eq!(chain.emit(Event::Button(true)), 2);
        assert_eq!(
            *log.borrow(),
            [
                ("первый", Event::Tick(5)),
                ("второй", Event::Tick(5)),
                ("первый", Event::Button(true)),
                ("второй", Event::Button(true))
            ],
            "обработчики вызываются в порядке добавления"
        );
        assert_eq!(Chain::default().emit(Event::Tick(0)), 0);
    }

    /// Обработчики владеют своим состоянием: реестр из трёх замыканий
    /// занимает ровно то, что они захватили, без таблиц и `Option`.
    #[test]
    fn task_b_chain_owns_state() {
        let report = RefCell::new(Vec::new());
        let counter = |kind: u8| {
            // Номер и счётчик — один кортеж, и замыкание захватывает его
            // целиком вместе со ссылкой на отчёт: восемь байт и слово при
            // любом порядке полей, в том числе под -Zrandomize-layout.
            let mut state = (kind, 0u32);
            let report = &report;
            move |e: Event| {
                let (kind, n) = &mut state;
                let mine = matches!(
                    (*kind, e),
                    (0, Event::Button(_)) | (1, Event::Sample(_)) | (2, Event::Tick(_))
                );
                *n += u32::from(mine);
                if e == Event::Tick(99) {
                    report.borrow_mut().push((*kind, *n));
                }
            }
        };
        let mut chain = Chain::new().with(counter(0)).with(counter(1)).with(counter(2));
        for e in EVENTS {
            assert_eq!(chain.emit(e), 3);
        }
        let mut handlers = chain.into_handlers();
        assert_eq!(handlers.emit(Event::Tick(99)), 3);
        assert_eq!(*report.borrow(), [(0, 3), (1, 2), (2, 2)]);
        let ((((), a), b), c) = handlers;
        let w = size_of::<usize>();
        assert_eq!(size_of_val(&a) + size_of_val(&b) + size_of_val(&c), 3 * (8 + w));
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use sut::c_parser::{
        Csq, Reply, alt, and_then, cmgl, cops, csq, left, line, many, map, number, pair,
        parse_cmgl, parse_cops, parse_csq, quoted, reply, right, tag,
    };

    #[test]
    fn task_c_primitives() {
        assert_eq!(tag("OK")("OK\r\n"), Some(("OK", "\r\n")));
        assert_eq!(tag("OK")("ERROR"), None);
        assert_eq!(tag("")("x"), Some(("", "x")));
        assert_eq!(number()("23,99"), Some((23, ",99")));
        assert_eq!(number()("4294967295"), Some((u32::MAX, "")));
        assert_eq!(number()("4294967296"), None, "переполнение — не число");
        assert_eq!(number()(",1"), None);
        assert_eq!(number()(""), None);
        assert_eq!(number()("٣"), None, "только ASCII-цифры");
        assert_eq!(quoted()("\"MegaFon\",1"), Some(("MegaFon", ",1")));
        assert_eq!(quoted()("\"\""), Some(("", "")));
        assert_eq!(quoted()("\"незакрытая"), None);
        assert_eq!(quoted()("Мегафон"), None);
    }

    #[test]
    fn task_c_combinators() {
        assert_eq!(map(number(), |n| n * 2)("21x"), Some((42, "x")));
        let small = and_then(number(), |n| (n < 10).then_some(n as u8));
        assert_eq!(small("7"), Some((7, "")));
        assert_eq!(small("70"), None);
        assert_eq!(pair(number(), tag(","))("1,2"), Some(((1, ","), "2")));
        assert_eq!(pair(number(), tag(","))("1;2"), None);
        assert_eq!(left(number(), tag(","))("1,2"), Some((1, "2")));
        assert_eq!(right(tag("+"), number())("+5"), Some((5, "")));
        let ok_or_error = alt(tag("OK"), tag("ERROR"));
        assert_eq!(ok_or_error("ERROR"), Some(("ERROR", "")));
        assert_eq!(ok_or_error("BUSY"), None);
        let digits = many(left(number(), tag(",")), 0, |acc, n| acc * 10 + n);
        assert_eq!(digits("1,2,3,x"), Some((123, "x")));
        assert_eq!(digits("x"), Some((0, "x")), "ни разу — тоже успех");
        let empty = many(tag(""), 0u32, |n, _| n + 1);
        assert_eq!(empty("abc"), Some((0, "abc")), "пустое совпадение не повторяется");
        assert_eq!(line(tag("OK"))("OK\r\n"), Some(("OK", "")));
        assert_eq!(line(tag("OK"))("OK"), None);
    }

    #[test]
    fn task_c_at_replies() {
        assert_eq!(
            parse_csq("\r\n+CSQ: 23,99\r\n\r\nOK\r\n"),
            Some(Reply::Data(Csq { rssi: 23, ber: 99 }))
        );
        assert_eq!(
            parse_csq("\r\n+CSQ: 99,7\r\n\r\nOK\r\n"),
            Some(Reply::Data(Csq { rssi: 99, ber: 7 }))
        );
        assert_eq!(parse_csq("\r\n+CSQ: 32,0\r\n\r\nOK\r\n"), None, "уровень 32 не бывает");
        assert_eq!(parse_csq("\r\n+CSQ: 20,8\r\n\r\nOK\r\n"), None, "ошибки 8 не бывает");
        assert_eq!(parse_csq("\r\n+CSQ: 23,99\r\n"), None, "нет OK");
        assert_eq!(parse_csq("\r\n+CSQ: 23,99\r\n\r\nOK\r\nмусор"), None, "остаток");
        assert_eq!(parse_csq("\r\nERROR\r\n"), Some(Reply::Error));
        assert_eq!(parse_csq("\r\n+CME ERROR: 10\r\n"), Some(Reply::Cme(10)));
        assert_eq!(
            parse_cops("\r\n+COPS: 0,0,\"MegaFon\"\r\n\r\nOK\r\n"),
            Some(Reply::Data("MegaFon"))
        );
        assert_eq!(parse_cops("\r\n+COPS: 0\r\n\r\nOK\r\n"), None);
        assert_eq!(parse_cops("\r\n+CME ERROR: 30\r\n"), Some(Reply::Cme(30)));
        let list = "\r\n+CMGL: 1,\"REC READ\"\r\n+CMGL: 3,\"REC UNREAD\"\r\n\r\nOK\r\n";
        assert_eq!(parse_cmgl(list), Some(Reply::Data(0b1010)));
        assert_eq!(parse_cmgl("\r\nOK\r\n"), Some(Reply::Data(0)), "пустой список");
        assert_eq!(parse_cmgl("\r\n+CMGL: 32,\"REC READ\"\r\n\r\nOK\r\n"), None);
        // Части тоже парсеры и складываются дальше.
        assert_eq!(reply(cops())("\r\nERROR\r\n"), Some((Reply::Error, "")));
        assert!(csq()("+CSQ: 1,2\r\n").is_some());
        assert_eq!(cmgl()("+CMGL: 0,\"x\"\r\n+CMGL: 2,\"y\"\r\nOK"), Some((0b101, "OK")));
    }

    /// Ассемблер библиотеки задач, собранной в release для `target`.
    pub(super) fn emit_asm(target: &str) -> String {
        let dir = tmp(&format!("c-asm-{target}"));
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

    /// Тело функции, метка которой содержит все `parts`. Если LLVM сделал
    /// из функции псевдоним другой, берётся тело той.
    fn body(asm: &str, parts: &[&str]) -> Vec<String> {
        let label = asm
            .lines()
            .find(|l| {
                l.ends_with(':') && !l.starts_with('.') && parts.iter().all(|p| l.contains(p))
            })
            .unwrap_or_else(|| panic!("в ассемблере нет функции {parts:?}"));
        let name = label.trim_end_matches(':');
        if let Some(target) = asm.lines().find_map(|l| {
            l.trim().strip_prefix(&format!(".set {name}, ")).map(|t| t.trim().to_string())
        }) {
            return body(asm, &[&target]);
        }
        let start = asm.lines().position(|l| l == label).unwrap();
        asm.lines()
            .skip(start + 1)
            .take_while(|l| !l.starts_with(".Lfunc_end"))
            .map(|l| l.trim().replace('\t', " "))
            .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
            .collect()
    }

    /// Вызов по адресу из регистра или из памяти, кроме загрузки адреса
    /// известной функции через GOT (`[rip + имя@GOTPCREL]`).
    fn indirect(l: &str) -> bool {
        let x86 = l.starts_with("call") || l.starts_with("jmp");
        let arm = l.starts_with("blx") || l.starts_with("bx") && !l.starts_with("bx lr");
        let operand = l.split_once(' ').map_or("", |(_, o)| o.trim());
        (x86 && (operand.starts_with('r') || operand.contains("ptr [") && !operand.contains("rip")))
            || (arm && operand.starts_with('r') && operand != "lr")
    }

    /// Весь разбор встроен: в `parse_csq` и `parse_cmgl` нет ни одного
    /// вызова по адресу — ни через `dyn`, ни через указатель на функцию.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_c_no_indirect_calls() {
        // Пустая заготовка тоже обходится без вызовов по адресу: сначала
        // парсер должен работать.
        assert_eq!(parse_csq("\r\nERROR\r\n"), Some(Reply::Error));
        assert_eq!(parse_cmgl("\r\nOK\r\n"), Some(Reply::Data(0)));
        for target in ["x86_64-unknown-linux-gnu", "thumbv7m-none-eabi"] {
            let asm = emit_asm(target);
            for f in ["9parse_csq", "10parse_cmgl"] {
                let b = body(&asm, &["c_parser", f]);
                let bad: Vec<&String> = b.iter().filter(|l| indirect(l)).collect();
                assert!(bad.is_empty(), "{target}, {f}: вызовы по адресу {bad:?}");
            }
        }
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use std::cell::Cell;
    use std::pin::pin;
    use std::rc::Rc;
    use std::task::{Context, Poll, Waker};
    use sut::d_send::{Job, make_job, make_job_send, make_local_rc, make_task, run_in_thread};

    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = pin!(f);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
        }
    }

    #[test]
    fn task_d_runs() {
        let stats = Rc::new(Cell::new(0));
        assert_eq!(run_in_thread(make_job(Job { id: 7, stats: stats.clone() })), 703);
        assert_eq!(run_in_thread(make_job_send(Job { id: 2, stats: stats.clone() })), 203);
        assert_eq!(stats.get(), 0, "без возможностей задачи статистику никто не трогает");
        let mut local = make_job(Job { id: 1, stats: stats.clone() });
        assert_eq!([local(), local()], [101, 102]);
        assert_eq!(block_on(make_task(5)), 6);
        assert_eq!(run_in_thread(make_local_rc(1)), 3);
    }

    const FAR_AWAY: &str = "use std::cell::Cell;\nuse std::rc::Rc;\n\
        use sut::d_send::{Job, make_job, make_local_rc, make_task, run_in_thread};\n\
        fn is_send<T: Send>(_: &T) {}\n\
        pub fn far_away() -> u32 {\n    \
            let job = Job { id: 7, stats: Rc::new(Cell::new(0)) };\n    \
            run_in_thread(make_job(job))\n}\n\
        pub fn futures() {\n    is_send(&make_task(1));\n    is_send(&make_local_rc(1));\n}\n";

    fn check(name: &str, features: &[&str]) -> (bool, String) {
        let dir = probe(name, true, features, FAR_AWAY);
        let out = pinned("cargo", &dir)
            .args(["check", "--target-dir"])
            .arg(dir.join("target"))
            .output()
            .unwrap();
        (out.status.success(), text(&out.stderr))
    }

    /// Без возможностей пробный крейт компилируется, с `d-break` — нет, и
    /// ошибки показывают на пробный крейт: сигнатура `make_job` не обещала
    /// ничего, и её не в чем упрекнуть.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_d_far_away() {
        let (ok, err) = check("d-probe-ok", &[]);
        assert!(ok, "без возможностей задачи пробный крейт должен собираться:\n{err}");
        let (ok, err) = check("d-probe-break", &["d-break"]);
        assert!(!ok, "с d-break замыкание и future должны перестать быть Send");
        assert!(err.contains("`Rc<Cell<u32>>` cannot be sent between threads safely"), "{err}");
        assert!(err.contains("required because it's used within this closure"), "{err}");
        assert!(err.contains("`Rc<u32>` cannot be sent between threads safely"), "{err}");
        assert!(err.contains("required because it's used within this `async` fn body"), "{err}");
        let sites: Vec<&str> = err.lines().filter(|l| l.trim_start().starts_with("--> ")).collect();
        assert!(
            sites.iter().filter(|l| l.contains("src/lib.rs")).count() >= 2,
            "обе ошибки — в пробном крейте: {sites:?}"
        );
        assert!(
            !err.contains("make_local_rc"),
            "Rc внутри тела замыкания не влияет на Send:\n{err}"
        );
    }

    /// С `d-break-send` ломается сама библиотека: `+ Send` в сигнатуре
    /// `make_job_send` проверяется там, где функция определена.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_d_promise_in_signature() {
        let (ok, err) = check("d-probe-send", &["d-break-send"]);
        assert!(!ok, "с d-break-send make_job_send должна перестать компилироваться");
        let first = err.lines().find(|l| l.trim_start().starts_with("--> ")).unwrap_or("");
        assert!(first.contains("d_send.rs"), "ошибка должна быть в d_send.rs: {first}\n{err}");
        assert!(err.contains("cannot be sent between threads safely"), "{err}");
        assert!(err.contains("make_job_send"), "{err}");
    }
}
