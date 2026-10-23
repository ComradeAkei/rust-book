// Общие тесты задач главы 11.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch11/tests/ch11.rs` и `solutions/ch11/tests/ch11.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A сверяет ответы ещё и при сборке для Cortex-M3, задача B — код
// двух вызовов в ассемблере; обе собирают пробы закреплённым cargo. Под
// Miri эти тесты пропускаются, как и чтение таблицы методов в задаче D:
// Miri таблицу памятью не считает.

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

/// Имя библиотеки крейта в именах файлов: `ch11-solutions` → `ch11_solutions`.
fn lib_name() -> String {
    env!("CARGO_PKG_NAME").replace('-', "_")
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use std::rc::Rc;
    use sut::a_sizes::{Rect, Shape, Square, pointee_bytes, words};

    const NAMES: [&str; 12] = [
        "&dyn Shape",
        "Box<dyn Shape>",
        "&str",
        "Box<[u8]>",
        "Rc<dyn Shape>",
        "&&dyn Shape",
        "Option<Box<dyn Shape>>",
        "[&dyn Shape; 3]",
        "&[&dyn Shape; 3]",
        "&[&dyn Shape]",
        "fn(&dyn Shape) -> u32",
        "Option<(&dyn Shape, u8)>",
    ];

    fn measured_words() -> [usize; 12] {
        let w = size_of::<usize>();
        [
            size_of::<&dyn Shape>(),
            size_of::<Box<dyn Shape>>(),
            size_of::<&str>(),
            size_of::<Box<[u8]>>(),
            size_of::<Rc<dyn Shape>>(),
            size_of::<&&dyn Shape>(),
            size_of::<Option<Box<dyn Shape>>>(),
            size_of::<[&dyn Shape; 3]>(),
            size_of::<&[&dyn Shape; 3]>(),
            size_of::<&[&dyn Shape]>(),
            size_of::<fn(&dyn Shape) -> u32>(),
            size_of::<Option<(&dyn Shape, u8)>>(),
        ]
        .map(|b| b / w)
    }

    #[test]
    fn task_a_words() {
        let answers = words();
        let actual = measured_words();
        let wrong: Vec<String> = (0..12)
            .filter(|&i| answers[i] != actual[i])
            .map(|i| format!("{i}. {}: ответ {}, на деле {}", NAMES[i], answers[i], actual[i]))
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn task_a_pointees() {
        let sq: &dyn Shape = &Square(3);
        let rect: &dyn Shape = &Rect { w: 3, h: 4 };
        let bytes: Box<[u8]> = vec![0u8; 5].into_boxed_slice();
        let squares = [Square(1), Square(2), Square(3)];
        let rc: Rc<dyn Shape> = Rc::new(Rect { w: 1, h: 1 });
        let actual = [
            size_of_val(sq),
            size_of_val(rect),
            size_of_val("дом"),
            size_of_val(&*bytes),
            size_of_val(&squares[..]),
            size_of_val(&*rc),
        ];
        let answers = pointee_bytes();
        let wrong: Vec<String> = (0..6)
            .filter(|&i| answers[i] != actual[i])
            .map(|i| format!("{i}: ответ {}, на деле {}", answers[i], actual[i]))
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
        assert_eq!(sq.area() + rect.area(), 21);
    }

    /// Те же ответы при сборке для Cortex-M3, где слово — 4 байта:
    /// проверки в `const` вычисляет компилятор цели.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_a_thumb() {
        let dir = tmp("a-thumb");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let manifest = format!(
            "[package]\nname = \"a-thumb\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
             [dependencies]\nsut = {{ package = \"{}\", path = {:?} }}\n\n[workspace]\n",
            env!("CARGO_PKG_NAME"),
            crate_dir()
        );
        std::fs::write(dir.join("Cargo.toml"), manifest).unwrap();
        let mut src = String::from(
            "#![no_std]\nextern crate alloc;\nuse alloc::{boxed::Box, rc::Rc};\n\
             use sut::a_sizes::*;\nconst W: usize = size_of::<usize>();\n\
             const A: [usize; 12] = words();\nconst P: [usize; 6] = pointee_bytes();\n",
        );
        for (i, name) in NAMES.iter().enumerate() {
            src += &format!(
                "const _: () = assert!(size_of::<{name}>() == A[{i}] * W, \"{i}. {name}\");\n"
            );
        }
        src += "const _: () = assert!(size_of_val::<dyn Shape>(&Square(3)) == P[0], \"0\");\n\
                const _: () = assert!(size_of_val::<dyn Shape>(&Rect { w: 3, h: 4 }) == P[1], \"1\");\n\
                const _: () = assert!(size_of_val(\"дом\") == P[2], \"2\");\n\
                const _: () = assert!(size_of_val::<[Square]>(&[Square(1), Square(2), Square(3)]) == P[4], \"4\");\n\
                pub fn keep(_: Box<[u8]>, _: Rc<dyn Shape>) {}\n";
        std::fs::write(dir.join("src/lib.rs"), src).unwrap();
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
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;
    use sut::b_vtable::{
        Counter, Sensor, SensorBox, SensorRef, Thermo, ch11_b_read_dyn, ch11_b_read_raw,
    };

    #[test]
    fn task_b_calls() {
        let t = Thermo { raw: 344 };
        let c = Counter { pulses: 7 };
        let refs = [SensorRef::new(&t), SensorRef::new(&c)];
        let sum: i32 = refs.iter().map(|r| r.read()).sum();
        assert_eq!(sum, 2157);
        assert_eq!(refs.map(|r| r.name()), ["термодатчик", "датчик"]);
        assert_eq!(refs.map(|r| (r.size(), r.align())), [(2, 2), (4, 4)]);
        assert!(refs[0].vtable().drop.is_none(), "у Thermo нет деструктора");
        assert_eq!(ch11_b_read_raw(refs[0]), ch11_b_read_dyn(&t));
        assert_eq!(ch11_b_read_raw(SensorRef::new(&Thermo { raw: -16 })), -100);
    }

    #[test]
    fn task_b_one_table_per_type() {
        let (a, b) = (Thermo { raw: 1 }, Thermo { raw: 2 });
        let c = Counter { pulses: 3 };
        let (ta, tb, tc) =
            (SensorRef::new(&a).vtable(), SensorRef::new(&b).vtable(), SensorRef::new(&c).vtable());
        assert!(std::ptr::eq(ta, tb), "таблица для Thermo должна быть одна");
        assert!(!std::ptr::eq(ta, tc));
    }

    /// Датчик, который отмечает свой `drop`.
    struct Tracked {
        drops: Rc<Cell<u32>>,
        level: u64,
    }

    impl Sensor for Tracked {
        fn read(&self) -> i32 {
            self.level as i32
        }
    }

    impl Drop for Tracked {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    struct Nothing;

    impl Sensor for Nothing {
        fn read(&self) -> i32 {
            -1
        }
    }

    #[repr(align(32))]
    struct Aligned([u8; 40]);

    impl Sensor for Aligned {
        fn read(&self) -> i32 {
            self.0.iter().map(|&b| i32::from(b)).sum()
        }
    }

    #[test]
    fn task_b_box_drop() {
        let drops = Rc::new(Cell::new(0));
        {
            let b = SensorBox::new(Tracked { drops: drops.clone(), level: 42 });
            assert_eq!(b.as_ref().read(), 42);
            assert_eq!(b.as_ref().size(), size_of::<Tracked>());
            assert!(b.as_ref().vtable().drop.is_some());
            assert_eq!(drops.get(), 0);
        }
        assert_eq!(drops.get(), 1, "деструктор должен отработать ровно раз");
        // Тип нулевого размера и выравнивание больше слова: Box::new их
        // не выделяет или выделяет с особым выравниванием, и dealloc должен
        // получить то же самое.
        let z = SensorBox::new(Nothing);
        assert_eq!((z.as_ref().read(), z.as_ref().size()), (-1, 0));
        let a = SensorBox::new(Aligned([1; 40]));
        assert_eq!((a.as_ref().read(), a.as_ref().size(), a.as_ref().align()), (40, 64, 32));
        let list: Vec<SensorBox> =
            (0..10).map(|i| SensorBox::new(Tracked { drops: drops.clone(), level: i })).collect();
        assert_eq!(list.iter().map(|b| b.as_ref().read()).sum::<i32>(), 45);
        drop(list);
        assert_eq!(drops.get(), 11);
    }

    /// Ассемблер библиотеки задач, собранной в release: тело функции
    /// `name` без директив, меток и комментариев. Если LLVM сделал из
    /// функции псевдоним другой, берётся тело той.
    pub(super) fn body(asm: &str, name: &str) -> Vec<String> {
        if let Some(target) = asm.lines().find_map(|l| {
            l.trim().strip_prefix(&format!(".set {name}, ")).map(|t| t.trim().to_string())
        }) {
            return body(asm, &target);
        }
        let start = asm.lines().position(|l| l == format!("{name}:")).unwrap_or_else(|| {
            panic!("в ассемблере нет функции {name}");
        });
        asm.lines()
            .skip(start + 1)
            .take_while(|l| !l.starts_with(".Lfunc_end"))
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('.') && !l.starts_with('#'))
            .filter(|l| !l.starts_with('@') && !l.ends_with(':'))
            .map(String::from)
            .collect()
    }

    pub(super) fn emit_asm(target: &str) -> String {
        let dir = tmp(&format!("b-asm-{target}"));
        let mut cmd = pinned("cargo", crate_dir());
        cmd.args(["rustc", "--release", "--lib", "--target", target, "--target-dir"])
            .arg(&dir)
            .args(["--", "--emit", "asm", "-C", "codegen-units=1"]);
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

    /// Вызов через самодельную таблицу компилируется в те же инструкции,
    /// что и через `dyn`: на x86-64 — один `jmp` по слову 3 таблицы, на
    /// Cortex-M3 — загрузка по смещению 12 и `bx`.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_b_same_code() {
        for (target, needle) in [
            ("x86_64-unknown-linux-gnu", "jmp qword ptr [rsi + 24]"),
            ("thumbv7m-none-eabi", "ldr r1, [r1, #12]"),
        ] {
            let asm = emit_asm(target);
            let raw = body(&asm, "ch11_b_read_raw");
            let dynamic = body(&asm, "ch11_b_read_dyn");
            assert_eq!(raw, dynamic, "{target}: код вызовов различается");
            assert!(
                dynamic.iter().any(|l| l.replace('\t', " ") == needle),
                "{target}: в ch11_b_read_dyn нет «{needle}»: {dynamic:?}"
            );
        }
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::sut;
    use sut::c_dispatch::{
        Eight, Family, Op, OpK, SixtyFour, Two, build_dyn, build_enum, formula, kinds, make_dyn,
        run, xorshift,
    };

    /// Перестановка из условия, написанная заново.
    fn reference_kinds(n: u8, len: usize, sorted: bool) -> Vec<u8> {
        let per = len / usize::from(n);
        let mut k: Vec<u8> = (0..len).map(|i| (i / per) as u8).collect();
        if !sorted {
            let mut s = 1;
            for i in (1..len).rev() {
                let j = xorshift(&mut s) as usize % (i + 1);
                k.swap(i, j);
            }
        }
        k
    }

    fn reference_sum(kinds: &[u8], x: u32) -> u32 {
        kinds
            .iter()
            .enumerate()
            .fold(0u32, |s, (i, &k)| s.wrapping_add(formula(u32::from(k), i as u32, x)))
    }

    #[test]
    fn task_c_kinds() {
        for (n, len) in [(2u8, 16usize), (8, 64), (64, 4096)] {
            let sorted = kinds(n, len, true);
            assert_eq!(sorted, reference_kinds(n, len, true), "n = {n}, по порядку");
            let shuffled = kinds(n, len, false);
            assert_eq!(shuffled, reference_kinds(n, len, false), "n = {n}, вперемешку");
            let mut back = shuffled.clone();
            back.sort();
            assert_eq!(back, sorted, "перемешанный массив — перестановка упорядоченного");
            assert_ne!(shuffled, sorted);
        }
    }

    #[test]
    fn task_c_forwarding() {
        let b: Box<dyn Op> = make_dyn::<3>(10);
        assert_eq!(b.apply(5), formula(3, 10, 5));
        let items: Vec<Box<dyn Op>> = vec![make_dyn::<0>(1), make_dyn::<63>(2)];
        assert_eq!(run(&items, 9), formula(0, 1, 9).wrapping_add(formula(63, 2, 9)));
        assert_eq!(
            run(&[OpK::<7> { v: 1 }, OpK::<7> { v: 2 }], 0),
            formula(7, 1, 0).wrapping_add(formula(7, 2, 0))
        );
        assert_eq!(run::<OpK<1>>(&[], 3), 0);
    }

    fn check<F: Family>(len: usize) {
        for sorted in [true, false] {
            let k = kinds(F::N, len, sorted);
            for x in [0, 7, u32::MAX] {
                let want = reference_sum(&k, x);
                assert_eq!(run(&build_dyn::<F>(&k), x), want, "dyn, {} типов", F::N);
                assert_eq!(run(&build_enum::<F>(&k), x), want, "enum, {} типов", F::N);
                assert_eq!(F::run_groups(&F::groups(&k), x), want, "группы, {} типов", F::N);
            }
        }
    }

    #[test]
    fn task_c_sums() {
        check::<Two>(64);
        check::<Eight>(512);
        check::<SixtyFour>(4096);
    }

    /// Бенчмарк меряет то, что заявлено: в цикле для `Box<dyn Op>` остаётся
    /// косвенный вызов, а для однородной группы вызовов нет вовсе.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_c_calls() {
        let asm = super::task_b::emit_asm("x86_64-unknown-linux-gnu");
        let dynamic = super::task_b::body(&asm, "ch11_c_run_dyn");
        assert!(
            dynamic
                .iter()
                .any(|l| l.starts_with("call") && l.contains("ptr [") && !l.contains("rip")),
            "в ch11_c_run_dyn нет вызова через таблицу: {dynamic:?}"
        );
        let fixed = super::task_b::body(&asm, "ch11_c_run_static");
        assert!(
            !fixed.iter().any(|l| l.starts_with("call")),
            "в ch11_c_run_static остался вызов: {fixed:?}"
        );
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::sut;
    use std::alloc::Layout;
    use sut::d_metadata::{
        Byte, Line, Named, Sensor, Unit, Wide, raw_has_drop, raw_size_align, split, std_layout,
        std_size_align,
    };

    /// Объекты задачи и что о них известно из типов.
    fn objects() -> Vec<(Box<dyn Sensor>, Layout, bool)> {
        vec![
            (Box::new(Unit), Layout::new::<Unit>(), false),
            (Box::new(Byte(7)), Layout::new::<Byte>(), false),
            (Box::new(Wide(u64::MAX)), Layout::new::<Wide>(), false),
            (Box::new(Line([1; 64])), Layout::new::<Line>(), false),
            (Box::new(Named("датчик".into())), Layout::new::<Named>(), true),
        ]
    }

    #[test]
    fn task_d_std() {
        for (s, layout, _) in objects() {
            assert_eq!(std_size_align(&*s), (layout.size(), layout.align()));
            assert_eq!(std_layout(&*s), layout);
        }
        assert_eq!(std_layout(&Line([0; 64])).align(), 64);
    }

    #[test]
    fn task_d_split() {
        for (s, _, _) in objects() {
            let (data, vtable) = split(&*s);
            assert_eq!(data, &*s as *const dyn Sensor as *const ());
            assert!(!vtable.is_null());
        }
        // Наблюдение: rustc строит одну таблицу на тип в единице
        // кодогенерации. Miri нарочно выдаёт разные таблицы одному типу:
        // язык единственности не обещает.
        let (a, b) = (Byte(1), Byte(2));
        #[cfg(not(miri))]
        assert_eq!(split(&a).1, split(&b).1, "два объекта одного типа — одна таблица");
        assert_eq!(split(&a).0, &a as *const Byte as *const ());
        assert_eq!(split(&b).0, &b as *const Byte as *const ());
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri таблицу методов памятью не считает")]
    fn task_d_raw() {
        for (s, layout, drops) in objects() {
            assert_eq!(raw_size_align(&*s), (layout.size(), layout.align()), "read = {}", s.read());
            assert_eq!(raw_size_align(&*s), std_size_align(&*s));
            assert_eq!(raw_has_drop(&*s), drops, "read = {}", s.read());
        }
    }
}
