// Общие тесты задач главы 2.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch02/tests/ch02.rs` и `solutions/ch02/tests/ch02.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов. Тесты задачи D лежат в `ch02-d.rs`: их же
// подключают крейты задачи D в edition 2021.
//
// Задача A проверяется по ассемблеру, который тест получает сам, запуская
// cargo в отдельном каталоге (`target/tmp/ch02-…`). Задача B сверяет
// декодер с дизассемблером: с готовым корпусом `ch02-thumb.txt` всегда и с
// вашей собственной прошивкой по `--ignored`. Тесты, которые запускают
// внешние программы или ждут взаимной блокировки, под Miri пропускаются.

mod tool {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    pub fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    pub fn scratch(name: &str) -> PathBuf {
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch02-{name}"))
    }

    /// Запуск cargo с отдельным каталогом сборки. Внешний cargo передаёт
    /// путь к себе в переменной CARGO, так что версия совпадёт. RUSTFLAGS
    /// убирается: для прошивок она заменила бы флаги из
    /// `firmware/.cargo/config.toml`.
    pub fn cargo(args: &[&str], dir: &Path, target_dir: &Path) {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let (command, rest) = args.split_first().unwrap();
        let out = Command::new(cargo)
            .arg(command)
            .arg("--target-dir")
            .arg(target_dir)
            .args(rest)
            .current_dir(dir)
            .env_remove("RUSTFLAGS")
            .output()
            .expect("не удалось запустить cargo");
        assert!(out.status.success(), "cargo {args:?}:\n{}", String::from_utf8_lossy(&out.stderr));
    }

    /// Крейт-библиотека из одного файла `lib.rs` с ассемблером в `out.s`.
    pub fn compile_asm(name: &str, lib_rs: &str, extra: &[&str]) -> String {
        let dir = scratch(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"ch02-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
                 [lib]\npath = \"lib.rs\"\n\n[workspace]\n"
            ),
        )
        .unwrap();
        std::fs::write(dir.join("lib.rs"), lib_rs).unwrap();
        let asm = dir.join("out.s");
        let emit = format!("--emit=asm={}", asm.display());
        let mut args = vec!["rustc", "--release", "--lib", "--", &emit];
        args.extend(["-C", "codegen-units=1", "-C", "llvm-args=-x86-asm-syntax=intel"]);
        args.extend(extra);
        cargo(&args, &dir, &dir.join("target"));
        std::fs::read_to_string(asm).unwrap()
    }

    /// `llvm-objdump`: из переменной LLVM_OBJDUMP, из PATH (с номером
    /// версии или без) или из компонента llvm-tools текущего тулчейна.
    pub fn llvm_objdump() -> PathBuf {
        if let Some(p) = std::env::var_os("LLVM_OBJDUMP") {
            return p.into();
        }
        let works = |p: &Path| Command::new(p).arg("--version").output().is_ok();
        for v in std::iter::once(String::new()).chain((14..=30).rev().map(|v| format!("-{v}"))) {
            let p = PathBuf::from(format!("llvm-objdump{v}"));
            if works(&p) {
                return p;
            }
        }
        let rustc = |arg: &str| {
            let out = Command::new("rustc").arg(arg).output().unwrap();
            String::from_utf8(out.stdout).unwrap()
        };
        let host =
            rustc("-vV").lines().find_map(|l| l.strip_prefix("host: ").map(str::to_owned)).unwrap();
        let p = Path::new(rustc("--print=sysroot").trim())
            .join("lib/rustlib")
            .join(host)
            .join("bin/llvm-objdump");
        assert!(
            works(&p),
            "llvm-objdump не найден: поставьте пакет llvm (приложение D) \
             или выполните `rustup component add llvm-tools`"
        );
        p
    }
}

mod task_a {
    use super::sut::a_tables::{MIN_CASES, MIN_DENSITY, MIN_DENSITY_SIZE, PREDICTIONS};
    use super::tool;
    use ch02::task_a::Lowering;

    const FUNCTIONS: [&str; 5] = [
        "ch02a_method",
        "ch02a_flash_time",
        "ch02a_uart_reg",
        "ch02a_digits",
        "ch02a_level_letter",
    ];

    /// Инструкции функции `name` из листинга в синтаксисе Intel.
    fn instructions(asm: &str, name: &str) -> Vec<String> {
        let head = format!("\n{name}:\n");
        let start = asm.find(&head).unwrap_or_else(|| panic!("{name} нет в листинге"));
        let body = &asm[start + head.len()..];
        let body = &body[..body.find(".Lfunc_end").unwrap()];
        body.lines()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .filter(|l| l.starts_with(|c: char| c.is_ascii_lowercase()))
            .collect()
    }

    /// Способ по признакам из раздела 2.5. Метки таблиц стоят в
    /// операндах инструкций, поэтому ищем их во всей функции.
    fn classify(instrs: &[String]) -> Lowering {
        let any = |f: &dyn Fn(&str) -> bool| instrs.iter().any(|i| f(i));
        if any(&|i| i.contains(".LJTI")) {
            Lowering::JumpTable
        } else if any(&|i| i.contains(".Lswitch.table")) {
            Lowering::MemoryTable
        } else if any(&|i| {
            i.starts_with("bt ")
                || ["shr ", "shl ", "sar "].iter().any(|s| i.starts_with(s)) && i.ends_with(", cl")
        }) {
            Lowering::RegisterTable
        } else if any(&|i| i.starts_with('j') && !i.starts_with("jmp")) {
            Lowering::Compares
        } else {
            Lowering::Formula
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn predictions_match() {
        if !cfg!(target_arch = "x86_64") {
            eprintln!("задача A проверяется по ассемблеру x86_64, а хост другой");
            return;
        }
        let path = tool::workspace_root().join("code/ch02/src/task_a.rs");
        let lib =
            format!("#![no_std]\n#[path = {:?}]\npub mod task_a;\n", path.canonicalize().unwrap());
        let asm = tool::compile_asm("task-a", &lib, &[]);
        let mut wrong = Vec::new();
        for (i, (name, guess)) in FUNCTIONS.iter().zip(PREDICTIONS).enumerate() {
            let instrs = instructions(&asm, name);
            let actual = classify(&instrs);
            match guess {
                None => wrong.push(format!("№{} {name:<20} ответ не заполнен", i + 1)),
                Some(g) if g != actual => wrong.push(format!(
                    "№{} {name:<20} ваш ответ: {g:?}, на самом деле: {actual:?}\n    {}",
                    i + 1,
                    instrs.join("\n    ")
                )),
                Some(_) => {}
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }

    /// Функции `jt_<n>_<r>`: `n` ветвей со значениями 0, 1, …, n − 2 и
    /// r − 1, то есть плотность n / r. Каждая ветвь вызывает свою внешнюю
    /// функцию, так что таблицей значений не обойтись.
    fn density_probe() -> (String, Vec<(u32, u32)>) {
        let mut src = String::from("#![no_std]\nunsafe extern \"C\" {\n");
        for i in 0..5 {
            src += &format!("    fn e{i}();\n");
        }
        src += "}\n";
        let mut shapes = Vec::new();
        for n in [3u32, 4] {
            for r in n..=60 {
                src += &format!(
                    "#[unsafe(no_mangle)]\npub fn jt_{n}_{r}(x: u32) {{\n    match x {{\n"
                );
                let values = (0..n - 1).chain([r - 1]);
                for (i, v) in values.enumerate() {
                    src += &format!("        {v} => unsafe {{ e{i}() }},\n");
                }
                src += "        _ => unsafe { e4() },\n    }\n}\n";
                shapes.push((n, r));
            }
        }
        (src, shapes)
    }

    fn check_density(level: &str, min_cases: u32, density: u32) -> Vec<String> {
        let (src, shapes) = density_probe();
        let opt = format!("opt-level={level}");
        let asm = tool::compile_asm(&format!("density-{level}"), &src, &["-C", &opt]);
        let mut wrong = Vec::new();
        for (n, r) in shapes {
            let actual =
                instructions(&asm, &format!("jt_{n}_{r}")).iter().any(|i| i.contains(".LJTI"));
            let predicted = n >= min_cases && n * 100 >= r * density;
            if predicted != actual {
                wrong.push(format!(
                    "opt-level={level}: {n} ветви на диапазоне {r} (плотность {:.1} %): \
                     по вашим порогам таблица {}, на самом деле {}",
                    100.0 * f64::from(n) / f64::from(r),
                    if predicted { "есть" } else { "нет" },
                    if actual { "есть" } else { "нет" },
                ));
            }
        }
        wrong
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn density_thresholds() {
        if !cfg!(target_arch = "x86_64") {
            eprintln!("задача A проверяется по ассемблеру x86_64, а хост другой");
            return;
        }
        // В заготовке пороги нулевые, и тест должен упасть, а не сломать
        // сборку, поэтому проверка во время выполнения, а не в `const`.
        let thresholds = [MIN_CASES, MIN_DENSITY, MIN_DENSITY_SIZE];
        assert!(!thresholds.contains(&0), "пороги не заполнены");
        let mut wrong = check_density("3", MIN_CASES, MIN_DENSITY);
        wrong.extend(check_density("s", MIN_CASES, MIN_DENSITY_SIZE));
        let shown = wrong.len().min(10);
        assert!(
            wrong.is_empty(),
            "\nрасхождений: {}\n{}\n",
            wrong.len(),
            wrong[..shown].join("\n")
        );
    }
}

mod task_b {
    use super::sut::b_thumb::decode;
    use super::tool;
    use ch02::thumb::Disasm;

    /// Строка дизассемблера с 16-битной инструкцией.
    struct Line {
        addr: u32,
        hw: u16,
        mnemonic: String,
        text: String,
    }

    /// Разбирает вывод `llvm-objdump -d` или корпус в том же формате.
    /// 32-битные инструкции и данные пропускаются.
    fn parse(dump: &str) -> Vec<Line> {
        let mut lines = Vec::new();
        for raw in dump.lines() {
            let raw = raw.trim_start();
            if raw.starts_with('#') {
                continue;
            }
            let Some((addr, rest)) = raw.split_once(": ") else { continue };
            let Ok(addr) = u32::from_str_radix(addr, 16) else { continue };
            let Some((code, asm)) = rest.split_once('\t') else { continue };
            let code = code.trim_end();
            if code.len() != 4 {
                continue; // 32-битная инструкция или байты данных
            }
            let Ok(hw) = u16::from_str_radix(code, 16) else { continue };
            let (mnemonic, operands) = asm.split_once('\t').unwrap_or((asm, ""));
            let operands = operands.split(" @").next().unwrap();
            let operands = match operands.find(" <") {
                Some(i) => &operands[..i],
                None => operands,
            };
            let operands = operands.trim();
            let text = if operands.is_empty() {
                mnemonic.to_string()
            } else {
                format!("{mnemonic}\t{operands}")
            };
            lines.push(Line { addr, hw, mnemonic: mnemonic.to_string(), text });
        }
        lines
    }

    /// Мнемоники подмножества: MOV, ADD, SUB, B, LDR.
    fn in_subset(mnemonic: &str) -> bool {
        const CONDS: [&str; 14] =
            ["eq", "ne", "hs", "lo", "mi", "pl", "vs", "vc", "hi", "ls", "ge", "lt", "gt", "le"];
        matches!(mnemonic, "movs" | "mov" | "adds" | "add" | "subs" | "sub" | "b" | "ldr")
            || mnemonic.strip_prefix('b').is_some_and(|c| CONDS.contains(&c))
    }

    /// Сверяет декодер со строками дизассемблера, возвращает число
    /// проверенных инструкций подмножества.
    fn check(lines: &[Line]) -> usize {
        let mut wrong = Vec::new();
        let mut checked = 0;
        for l in lines {
            let got = decode(l.hw).map(|insn| Disasm { insn, addr: l.addr }.to_string());
            let expected = in_subset(&l.mnemonic).then(|| l.text.clone());
            checked += usize::from(expected.is_some());
            if got != expected {
                wrong.push(format!(
                    "{:6x}: {:04x}  ожидалось {:<28} декодер: {}",
                    l.addr,
                    l.hw,
                    expected.unwrap_or_else(|| format!("None ({})", l.text)),
                    got.unwrap_or_else(|| "None".into())
                ));
            }
        }
        let shown = wrong.len().min(30);
        assert!(
            wrong.is_empty(),
            "\nошибок: {} из {} строк\n{}\n",
            wrong.len(),
            lines.len(),
            wrong[..shown].join("\n")
        );
        checked
    }

    #[test]
    fn committed_corpus() {
        let lines = parse(include_str!("ch02-thumb.txt"));
        assert!(lines.len() > 1000, "корпус не прочитан: {} строк", lines.len());
        let checked = check(&lines);
        assert!(checked > 700, "в корпусе мало инструкций подмножества: {checked}");
    }

    /// Прошивки главы 27 — только для Cortex-M3 и старше (BASEPRI, CFSR,
    /// MPU ARMv7-M, RTIC), прошивка главы 30 — тоже: у ARMv6-M нет
    /// счётчика тактов DWT. Тот же список исключает clippy для Cortex-M0 в
    /// `.github/workflows/ci.yml`.
    const M3_ONLY: [&str; 9] = [
        "ch27-core",
        "ch27-nvic",
        "ch27-periph",
        "ch27-irq",
        "ch27-fault",
        "ch27-rtic",
        "ch27-station",
        "ch27-tasks",
        "ch30-cycles",
    ];

    /// Собирает все прошивки из `firmware/` под Cortex-M0 и сверяет
    /// декодер с их дизассемблером.
    #[test]
    #[ignore = "собирает прошивки и запускает llvm-objdump"]
    fn own_firmware() {
        let target = "thumbv6m-none-eabi";
        let dir = tool::scratch("firmware");
        let mut args = vec!["build", "--release", "--target", target, "--workspace"];
        for name in M3_ONLY {
            args.extend(["--exclude", name]);
        }
        tool::cargo(&args, &tool::workspace_root().join("firmware"), &dir);
        let objdump = tool::llvm_objdump();
        let mut lines = Vec::new();
        let mut elves = Vec::new();
        for entry in std::fs::read_dir(dir.join(target).join("release")).unwrap() {
            let path = entry.unwrap().path();
            let is_elf =
                path.is_file() && std::fs::read(&path).is_ok_and(|b| b.starts_with(b"\x7fELF"));
            if !is_elf {
                continue;
            }
            let out = std::process::Command::new(&objdump).arg("-d").arg(&path).output().unwrap();
            assert!(
                out.status.success(),
                "{}: {}",
                path.display(),
                String::from_utf8_lossy(&out.stderr)
            );
            lines.extend(parse(&String::from_utf8(out.stdout).unwrap()));
            elves.push(path.file_name().unwrap().to_string_lossy().into_owned());
        }
        assert!(!elves.is_empty(), "прошивки не найдены");
        let checked = check(&lines);
        eprintln!(
            "прошивки {elves:?}: {} инструкций, из них в подмножестве {checked}",
            lines.len()
        );
    }
}

mod task_c {
    use super::sut::c_lexer::lex;
    use super::sut::c_regex::lex_regex;
    use ch02::lexer::generate::{self, Rng};
    use ch02::lexer::{Kind, LexError, Op, Token};

    const PROGRAMS: u64 = if cfg!(miri) { 2 } else { 300 };

    fn t(kind: Kind, start: usize, end: usize) -> Token {
        Token { kind, start, end }
    }

    #[test]
    fn hand_written() {
        use Kind::{Ident, Int, Str};
        let op = Kind::Op;
        let cases: &[(&str, &[Token])] = &[
            ("", &[]),
            (
                "x = 0x1F;",
                &[t(Ident, 0, 1), t(op(Op::Assign), 2, 3), t(Int(31), 4, 8), t(op(Op::Semi), 8, 9)],
            ),
            ("a->b", &[t(Ident, 0, 1), t(op(Op::Arrow), 1, 3), t(Ident, 3, 4)]),
            (
                "a-->b",
                &[t(Ident, 0, 1), t(op(Op::Minus), 1, 2), t(op(Op::Arrow), 2, 4), t(Ident, 4, 5)],
            ),
            ("1.5", &[t(Int(1), 0, 1), t(op(Op::Dot), 1, 2), t(Int(5), 2, 3)]),
            ("0", &[t(Int(0), 0, 1)]),
            ("007_", &[t(Int(7), 0, 4)]),
            ("0b_1010_", &[t(Int(10), 0, 8)]),
            ("18446744073709551615", &[t(Int(u64::MAX), 0, 20)]),
            ("0xFFFF_FFFF_FFFF_FFFF", &[t(Int(u64::MAX), 0, 21)]),
            ("a/b", &[t(Ident, 0, 1), t(op(Op::Slash), 1, 2), t(Ident, 2, 3)]),
            ("a/**/b//c\nd", &[t(Ident, 0, 1), t(Ident, 5, 6), t(Ident, 10, 11)]),
            ("/***/x", &[t(Ident, 5, 6)]),
            ("/*/ */x", &[t(Ident, 6, 7)]),
            ("// конец", &[]),
            ("\"\"", &[t(Str, 0, 2)]),
            ("\"а\\x41\\\"\\\\\"", &[t(Str, 0, 12)]),
            (
                "<<=>>=!=!",
                &[
                    t(op(Op::Shl), 0, 2),
                    t(op(Op::Assign), 2, 3),
                    t(op(Op::Shr), 3, 5),
                    t(op(Op::Assign), 5, 6),
                    t(op(Op::Ne), 6, 8),
                    t(op(Op::Bang), 8, 9),
                ],
            ),
            (
                "&&&|||",
                &[
                    t(op(Op::AndAnd), 0, 2),
                    t(op(Op::Amp), 2, 3),
                    t(op(Op::OrOr), 3, 5),
                    t(op(Op::Pipe), 5, 6),
                ],
            ),
            (
                "f(x)[0]{}",
                &[
                    t(Ident, 0, 1),
                    t(op(Op::LParen), 1, 2),
                    t(Ident, 2, 3),
                    t(op(Op::RParen), 3, 4),
                    t(op(Op::LBracket), 4, 5),
                    t(Int(0), 5, 6),
                    t(op(Op::RBracket), 6, 7),
                    t(op(Op::LBrace), 7, 8),
                    t(op(Op::RBrace), 8, 9),
                ],
            ),
            (
                "_9:%*+,",
                &[
                    t(Ident, 0, 2),
                    t(op(Op::Colon), 2, 3),
                    t(op(Op::Percent), 3, 4),
                    t(op(Op::Star), 4, 5),
                    t(op(Op::Plus), 5, 6),
                    t(op(Op::Comma), 6, 7),
                ],
            ),
        ];
        for (src, expected) in cases {
            assert_eq!(lex(src).as_deref(), Ok(*expected), "автомат на {src:?}");
            if !cfg!(miri) {
                assert_eq!(lex_regex(src).as_deref(), Ok(*expected), "regex на {src:?}");
            }
        }
    }

    /// Ошибочные входы и первая ошибка, которую должен вернуть автомат.
    const ERRORS: &[(&str, LexError)] = &[
        ("a @ b", LexError::UnexpectedChar(2)),
        ("x = ё", LexError::UnexpectedChar(4)),
        ("\"abc", LexError::UnterminatedString(0)),
        ("x \"ab\ncd\"", LexError::UnterminatedString(2)),
        ("\"ab\\", LexError::UnterminatedString(0)),
        ("\"ab\\q\"", LexError::BadEscape(3)),
        ("\"\\x4g\"", LexError::BadEscape(1)),
        ("\"\\x4", LexError::UnterminatedString(0)),
        ("/* abc", LexError::UnterminatedComment(0)),
        ("a /* b *", LexError::UnterminatedComment(2)),
        ("12ab", LexError::BadDigit(2)),
        ("0b102", LexError::BadDigit(4)),
        ("0x", LexError::BadDigit(2)),
        ("0x_ ", LexError::BadDigit(3)),
        ("0xg", LexError::BadDigit(2)),
        ("0X1", LexError::BadDigit(1)),
        ("1_000x", LexError::BadDigit(5)),
        ("18446744073709551616", LexError::Overflow(0)),
        ("x 0x1_0000_0000_0000_0000", LexError::Overflow(2)),
        (
            "0b1_0000000000000000000000000000000000000000000000000000000000000000",
            LexError::Overflow(0),
        ),
    ];

    #[test]
    fn errors() {
        for &(src, err) in ERRORS {
            assert_eq!(lex(src), Err(err), "автомат на {src:?}");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "regex под Miri слишком медленный")]
    fn regex_rejects_errors() {
        for &(src, _) in ERRORS {
            assert!(
                lex_regex(src).is_err(),
                "regex принял ошибочный вход {src:?}: {:?}",
                lex_regex(src)
            );
        }
    }

    #[test]
    fn random_programs() {
        for seed in 0..PROGRAMS {
            let (src, expected) = generate::program(seed, 200);
            assert_eq!(lex(&src), Ok(expected), "автомат, программа №{seed}:\n{src}");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "regex под Miri слишком медленный")]
    fn regex_random_programs() {
        for seed in 0..PROGRAMS {
            let (src, expected) = generate::program(seed, 200);
            assert_eq!(lex_regex(&src), Ok(expected), "regex, программа №{seed}:\n{src}");
        }
    }

    /// Программа с одним испорченным символом: оба лексера должны
    /// одинаково решить, ошибка это или нет, и выдать одни и те же токены.
    #[test]
    #[cfg_attr(miri, ignore = "regex под Miri слишком медленный")]
    fn mutations_agree() {
        const NOISE: [char; 16] =
            ['"', '\\', '/', '*', '\n', '0', 'x', 'b', '_', 'g', '@', 'ё', ' ', '=', '-', '>'];
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        for seed in 0..2000 {
            let (src, _) = generate::program(seed, 30);
            let mut chars: Vec<char> = src.chars().collect();
            let at = rng.below(chars.len() as u64) as usize;
            chars[at] = NOISE[rng.below(NOISE.len() as u64) as usize];
            let src: String = chars.into_iter().collect();
            let (a, b) = (lex(&src), lex_regex(&src));
            match (&a, &b) {
                (Ok(x), Ok(y)) => assert_eq!(x, y, "разные токены на {src:?}"),
                (Err(_), Err(_)) => {}
                _ => panic!("автомат: {a:?}\nregex: {b:?}\nвход: {src:?}"),
            }
        }
    }
}

mod task_d {
    include!("ch02-d.rs");
}
