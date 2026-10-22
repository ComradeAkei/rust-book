// Общие тесты задач главы 3.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch03/tests/ch03.rs` и `solutions/ch03/tests/ch03.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов.
//
// Задача C проверяется по LLVM IR и ассемблеру, которые тест получает сам,
// запуская cargo в отдельном каталоге (`target/tmp/ch03-…`); под Miri она
// пропускается. Тесты задачи D с пометкой «UB» по умолчанию не запускаются:
// это неопределённое поведение, которое нужно увидеть под Miri.

mod task_a {
    use super::sut::a_drop_order::PREDICTIONS;
    use ch03::drop_order::take_log;
    use ch03::task_a::FRAGMENTS;
    use std::panic;

    #[test]
    fn predictions_match() {
        let mut wrong = Vec::new();
        for (i, (f, guess)) in FRAGMENTS.iter().zip(PREDICTIONS).enumerate() {
            // Фрагмент №7 паникует нарочно: ловим панику и сравниваем журнал.
            let _ = panic::catch_unwind(f);
            let actual = take_log();
            if guess.is_empty() {
                wrong.push(format!("№{} ответ не заполнен", i + 1));
            } else if guess != actual.as_slice() {
                wrong.push(format!(
                    "№{}  ваш ответ: {guess:?}\n    на самом деле: {actual:?}",
                    i + 1
                ));
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }
}

mod task_b {
    use super::sut::b_scope_guard::{ScopeGuard, Transaction};
    use ch03::regs::Regs;
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    /// Значение, которое считает, сколько раз его уничтожили.
    struct Counted<'a>(&'a Cell<u32>);

    impl Drop for Counted<'_> {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn runs_on_scope_exit_once() {
        let calls = Cell::new(0);
        {
            let mut g = ScopeGuard::new(5, |v| calls.set(calls.get() + v));
            *g += 1; // DerefMut
            assert_eq!(*g, 6); // Deref
            assert_eq!(calls.get(), 0);
        }
        assert_eq!(calls.get(), 6);
    }

    #[test]
    fn runs_on_early_return() {
        let calls = Cell::new(0);
        let f = |fail: bool| -> Result<(), ()> {
            let _g = ScopeGuard::new((), |()| calls.set(calls.get() + 1));
            if fail {
                return Err(());
            }
            Ok(())
        };
        assert_eq!(f(true), Err(()));
        assert_eq!(f(false), Ok(()));
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn runs_on_panic() {
        let calls = Cell::new(0);
        let r = catch_unwind(AssertUnwindSafe(|| {
            let _g = ScopeGuard::new(1, |v| calls.set(calls.get() + v));
            panic!("отказ посреди области");
        }));
        assert!(r.is_err());
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn into_inner_defuses() {
        let calls = Cell::new(0);
        let drops = Cell::new(0);
        let g = ScopeGuard::new(Counted(&drops), |_| calls.set(calls.get() + 1));
        let v = ScopeGuard::into_inner(g);
        assert_eq!((calls.get(), drops.get()), (0, 0));
        drop(v);
        assert_eq!((calls.get(), drops.get()), (0, 1));
    }

    #[test]
    fn value_and_closure_dropped_exactly_once() {
        let value_drops = Cell::new(0);
        let closure_drops = Cell::new(0);
        {
            let captured = Counted(&closure_drops);
            let _g = ScopeGuard::new(Counted(&value_drops), move |_v| {
                let _keep = &captured;
            });
        }
        assert_eq!((value_drops.get(), closure_drops.get()), (1, 1));
        let g = ScopeGuard::new(Counted(&value_drops), {
            let captured = Counted(&closure_drops);
            move |_v| {
                let _keep = &captured;
            }
        });
        drop(ScopeGuard::into_inner(g));
        assert_eq!((value_drops.get(), closure_drops.get()), (2, 2));
    }

    fn zst_guard() -> ScopeGuard<u64, impl FnOnce(u64)> {
        ScopeGuard::new(7, |_| {})
    }

    #[test]
    fn no_extra_bytes() {
        assert_eq!(size_of::<ScopeGuard<u64, fn(u64)>>(), 16);
        assert_eq!(size_of_val(&zst_guard()), size_of::<u64>(), "страж с пустым замыканием");
        assert_eq!(size_of::<ScopeGuard<(), fn(())>>(), size_of::<fn(())>());
    }

    fn setup() -> Regs {
        let mut r = Regs::new();
        r.write(0, 1);
        r.write(1, 2);
        r
    }

    #[test]
    fn rollback_without_commit() {
        let mut r = setup();
        {
            let mut t = Transaction::begin(&mut r);
            t.write(1, 20);
            t.write(5, 50);
            t.write(1, 21);
            assert_eq!(t.read(1), 21);
        }
        assert_eq!((r.read(0), r.read(1), r.read(5)), (1, 2, 0));
        // Откат пишет каждый регистр один раз, в обратном порядке первого
        // изменения.
        assert_eq!(r.history(), [(0, 1), (1, 2), (1, 20), (5, 50), (1, 21), (5, 0), (1, 2)]);
    }

    #[test]
    fn commit_keeps_changes() {
        let mut r = setup();
        let mut t = Transaction::begin(&mut r);
        t.write(3, 30);
        t.write(0, 10);
        t.commit();
        assert_eq!((r.read(0), r.read(3)), (10, 30));
        assert_eq!(r.history(), [(0, 1), (1, 2), (3, 30), (0, 10)]);
    }

    fn configure(r: &mut Regs, baud: u32) -> Result<(), &'static str> {
        let mut t = Transaction::begin(r);
        t.write(0, 0); // выключить устройство на время настройки
        t.write(2, baud);
        if baud == 0 {
            return Err("нулевая скорость");
        }
        t.write(0, 1);
        t.commit();
        Ok(())
    }

    #[test]
    fn rollback_on_early_return() {
        let mut r = setup();
        assert_eq!(configure(&mut r, 0), Err("нулевая скорость"));
        assert_eq!((r.read(0), r.read(2)), (1, 0));
        assert_eq!(&r.history()[2..], [(0, 0), (2, 0), (2, 0), (0, 1)]);
        assert_eq!(configure(&mut r, 115_200), Ok(()));
        assert_eq!((r.read(0), r.read(2)), (1, 115_200));
    }

    #[test]
    fn rollback_on_panic() {
        let mut r = setup();
        let res = catch_unwind(AssertUnwindSafe(|| {
            let mut t = Transaction::begin(&mut r);
            t.write(7, 70);
            t.write(1, 0);
            let _ = 1 / t.read(1); // деление на ноль: паника
            t.commit();
        }));
        assert!(res.is_err());
        assert_eq!((r.read(1), r.read(7)), (2, 0));
    }
}

mod task_c {
    use super::sut::c_abi::{
        THUMB_C, THUMB_MEMCPY_FROM, THUMB_RUST, Thresholds, X86_64_C, X86_64_MEMCPY_FROM,
        X86_64_RUST,
    };
    use ch03::task_c::MAX_WORDS;
    use std::path::Path;
    use std::process::Command;

    const X86: &str = "x86_64-unknown-linux-gnu";
    const THUMB: &str = "thumbv7em-none-eabihf";

    /// Пробный крейт: для каждого N структура из N слов, функции с ней в
    /// аргументе и в результате для обоих ABI, и вызывающий, которому
    /// приходится делать копию (вызываемая функция меняет свой аргумент).
    fn probe_source() -> String {
        let mut s = String::from("#![no_std]\nuse core::hint::black_box;\n");
        for n in 1..=MAX_WORDS {
            s += &format!(
                "#[repr(C)] pub struct W{n}(pub [u32; {n}]);\n\
                 #[inline(never)] #[unsafe(no_mangle)] pub fn arg_rust_{n}(mut s: W{n}) -> u32 \
                 {{ s.0.reverse(); black_box(&mut s); s.0[0] }}\n\
                 #[inline(never)] #[unsafe(no_mangle)] pub extern \"C\" fn arg_c_{n}(mut s: W{n}) -> u32 \
                 {{ s.0.reverse(); black_box(&mut s); s.0[0] }}\n\
                 #[inline(never)] #[unsafe(no_mangle)] pub fn ret_rust_{n}(x: u32) -> W{n} \
                 {{ W{n}([x; {n}]) }}\n\
                 #[inline(never)] #[unsafe(no_mangle)] pub extern \"C\" fn ret_c_{n}(x: u32) -> W{n} \
                 {{ W{n}([x; {n}]) }}\n\
                 #[inline(never)] #[unsafe(no_mangle)] pub fn call_rust_{n}(s: &W{n}) -> u32 \
                 {{ arg_rust_{n}(W{n}(s.0)) }}\n\
                 #[inline(never)] #[unsafe(no_mangle)] pub fn call_c_{n}(s: &W{n}) -> u32 \
                 {{ arg_c_{n}(W{n}(s.0)) }}\n"
            );
        }
        s
    }

    struct Output {
        ir: String,
        asm: String,
    }

    /// Собирает пробный крейт для цели и возвращает LLVM IR и ассемблер.
    fn compile(target: &str) -> Output {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch03-abi-{target}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"ch03-abi\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
             [lib]\npath = \"lib.rs\"\n\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), probe_source()).unwrap();
        let (ir, asm) = (dir.join("out.ll"), dir.join("out.s"));
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let out = Command::new(cargo)
            .args(["rustc", "--target-dir"])
            .arg(dir.join("target"))
            .args(["--release", "--lib", "--target", target, "--"])
            .arg(format!("--emit=llvm-ir={},asm={}", ir.display(), asm.display()))
            .args(["-C", "codegen-units=1"])
            .args(if target == X86 { &["-C", "llvm-args=-x86-asm-syntax=intel"][..] } else { &[] })
            .current_dir(&dir)
            .env_remove("RUSTFLAGS")
            .output()
            .expect("не удалось запустить cargo");
        assert!(
            out.status.success(),
            "сборка для {target}:\n{}\nНужна цель: rustup target add {target}",
            String::from_utf8_lossy(&out.stderr)
        );
        Output {
            ir: std::fs::read_to_string(ir).unwrap(),
            asm: std::fs::read_to_string(asm).unwrap(),
        }
    }

    /// Строка `define` функции из LLVM IR. Одинаковые функции
    /// MergeFunctions превращает в псевдоним (`@a = ... alias ..., ptr @b`):
    /// тогда берём определение оригинала.
    fn define<'a>(ir: &'a str, name: &str) -> &'a str {
        let alias = format!("@{name} = ");
        if let Some(l) = ir.lines().find(|l| l.starts_with(&alias) && l.contains(" alias ")) {
            let target = l.rsplit("ptr @").next().unwrap().trim();
            return define(ir, target);
        }
        let at = format!("@{name}(");
        ir.lines()
            .find(|l| l.starts_with("define") && l.contains(&at))
            .unwrap_or_else(|| panic!("{name} нет в IR"))
    }

    /// Тело функции в ассемблере. MergeFunctions может сделать одну функцию
    /// псевдонимом другой (`a = b`): тогда берём тело оригинала.
    fn body<'a>(asm: &'a str, name: &str) -> Vec<&'a str> {
        let alias = format!("\n{name} = ");
        if let Some(i) = asm.find(&alias) {
            let target = asm[i + alias.len()..].lines().next().unwrap().trim();
            return body(asm, target);
        }
        let head = format!("\n{name}:\n");
        let start = asm.find(&head).unwrap_or_else(|| panic!("{name} нет в ассемблере"));
        let rest = &asm[start + head.len()..];
        rest[..rest.find(".Lfunc_end").unwrap()]
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('.') && !l.starts_with('@'))
            .collect()
    }

    /// Аргумент идёт в регистрах, если в IR это не указатель на копию
    /// (Rust ABI) и не `byval` (C на x86_64). На Cortex-M для C всё
    /// решает раскладка AAPCS: смотрим, кладёт ли вызывающий что-нибудь на
    /// стек перед вызовом.
    fn arg_in_regs(o: &Output, target: &str, abi: &str, n: usize) -> (bool, String) {
        if target == THUMB && abi == "c" {
            let caller = body(&o.asm, &format!("call_c_{n}"));
            let stack: Vec<_> =
                caller.iter().filter(|l| l.starts_with("str") && l.contains("[sp")).collect();
            return (stack.is_empty(), format!("{stack:?}"));
        }
        let params = params(define(&o.ir, &format!("arg_{abi}_{n}")));
        (!params.contains("ptr"), format!("({params})"))
    }

    /// Список параметров из строки `define ... @имя(параметры) ...`.
    fn params(define: &str) -> &str {
        let start = define.find("@").unwrap();
        let open = start + define[start..].find('(').unwrap();
        let mut depth = 0;
        for (i, c) in define[open..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' if depth == 1 => return &define[open + 1..open + i],
                ')' => depth -= 1,
                _ => {}
            }
        }
        panic!("нет списка параметров: {define}")
    }

    fn ret_in_regs(o: &Output, abi: &str, n: usize) -> (bool, String) {
        let params = params(define(&o.ir, &format!("ret_{abi}_{n}")));
        (!params.contains("sret"), format!("({params})"))
    }

    fn calls_memcpy(o: &Output, n: usize) -> bool {
        body(&o.asm, &format!("call_rust_{n}")).iter().any(|l| l.contains("memcpy"))
    }

    fn check_target(target: &str, rust: Thresholds, c: Thresholds, memcpy: usize) -> Vec<String> {
        let o = compile(target);
        let mut wrong = Vec::new();
        for (abi, t) in [("rust", rust), ("c", c)] {
            for n in 1..=MAX_WORDS {
                let size = 4 * n;
                let (actual, why) = arg_in_regs(&o, target, abi, n);
                if actual != (size <= t.arg_in_regs) {
                    wrong.push(format!(
                        "{target}, ABI {abi}, аргумент {size} байт: по вашему порогу {} — \
                         на самом деле {}\n    {why}",
                        t.arg_in_regs,
                        if actual { "в регистрах" } else { "не в регистрах" }
                    ));
                    break;
                }
            }
            for n in 1..=MAX_WORDS {
                let size = 4 * n;
                let (actual, why) = ret_in_regs(&o, abi, n);
                if actual != (size <= t.ret_in_regs) {
                    wrong.push(format!(
                        "{target}, ABI {abi}, результат {size} байт: по вашему порогу {} — \
                         на самом деле {}\n    {why}",
                        t.ret_in_regs,
                        if actual { "в регистрах" } else { "через память" }
                    ));
                    break;
                }
            }
        }
        for n in 1..=MAX_WORDS {
            let size = 4 * n;
            let actual = calls_memcpy(&o, n);
            if actual != (size >= memcpy) {
                wrong.push(format!(
                    "{target}: копия {size} байт для call_rust_{n}: по вашему порогу {memcpy} — \
                     на самом деле {}\n    {}",
                    if actual { "вызов memcpy" } else { "без memcpy" },
                    body(&o.asm, &format!("call_rust_{n}")).join("\n    ")
                ));
                break;
            }
        }
        wrong
    }

    fn filled(t: Thresholds) -> bool {
        t.arg_in_regs != 0 && t.ret_in_regs != 0
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn x86_64() {
        assert!(filled(X86_64_RUST) && filled(X86_64_C) && X86_64_MEMCPY_FROM != 0, "ответы не заполнены");
        let wrong = check_target(X86, X86_64_RUST, X86_64_C, X86_64_MEMCPY_FROM);
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn thumbv7em() {
        assert!(filled(THUMB_RUST) && filled(THUMB_C) && THUMB_MEMCPY_FROM != 0, "ответы не заполнены");
        let wrong = check_target(THUMB, THUMB_RUST, THUMB_C, THUMB_MEMCPY_FROM);
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }
}

mod task_d {
    use super::sut::d_self_ref::{CAP, Full, PinnedRx, RxBuf, RxIdx};

    #[test]
    fn naive_notices_move() {
        let mut a = RxBuf::new();
        assert!(!a.moved(), "до init перемещать нечего");
        a.init();
        assert!(!a.moved());
        let boxed = Box::new(a);
        assert!(boxed.moved(), "буфер переехал в кучу");
        let v = [RxBuf::new()];
        assert!(!v[0].moved(), "без init буфер ни на что не ссылается");
    }

    /// Под Miri: курсор указывает в кадр функции, которой уже нет.
    #[test]
    #[ignore = "UB: запускать под Miri"]
    fn naive_returned_ub() {
        fn make() -> RxBuf {
            let mut rx = RxBuf::new();
            rx.init();
            rx
        }
        let mut rx = make();
        rx.push(0x7E).unwrap();
        assert_eq!(rx.received(), [0x7E]);
    }

    /// Под Miri: курсор получен из `&mut` в `init`, а `push` создаёт новый
    /// `&mut`, и права старого кончаются. Перемещать ничего не нужно.
    #[test]
    #[ignore = "UB: запускать под Miri"]
    fn naive_in_place_ub() {
        let mut rx = RxBuf::new();
        rx.init();
        rx.push(1).unwrap();
        assert_eq!(rx.received(), [1]);
    }

    fn fill<F: FnMut(u8) -> Result<(), Full>>(mut push: F) {
        for b in 0..CAP as u8 {
            assert_eq!(push(b), Ok(()));
        }
        assert_eq!(push(0xFF), Err(Full));
    }

    #[test]
    fn index_survives_moves() {
        fn make() -> RxIdx {
            let mut rx = RxIdx::new();
            rx.push(1).unwrap();
            rx
        }
        let rx = make();
        let mut boxed = Box::new(rx);
        boxed.push(2).unwrap();
        let mut v = [*boxed]; // из кучи обратно на стек
        v[0].push(3).unwrap();
        assert_eq!(v[0].received(), [1, 2, 3]);
        let mut rx = RxIdx::new();
        fill(|b| rx.push(b));
        assert_eq!(rx.received().len(), CAP);
    }

    #[test]
    fn pinned_survives_moves() {
        let mut rx = PinnedRx::new();
        rx.as_mut().push(1).unwrap();
        let mut v = vec![rx]; // перемещается указатель, не буфер
        v[0].as_mut().push(2).unwrap();
        let mut rx = v.pop().unwrap();
        rx.as_mut().push(3).unwrap();
        assert_eq!(rx.received(), [1, 2, 3]);
        let mut rx = PinnedRx::new();
        fill(|b| rx.as_mut().push(b));
        assert_eq!(rx.received().len(), CAP);
    }

    #[test]
    fn index_is_smaller() {
        assert!(size_of::<RxIdx>() < size_of::<RxBuf>());
        assert!(size_of::<RxIdx>() <= CAP + 1);
    }
}
