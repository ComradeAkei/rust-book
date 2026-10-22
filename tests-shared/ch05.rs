// Общие тесты задач главы 5.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch05/tests/ch05.rs` и `solutions/ch05/tests/ch05.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов.
//
// Задачи A, C и D собирают файлы компилятором — тем же rustc, которым
// cargo собирает тесты. Под Miri эти тесты пропускаются. Задача B считает
// выделения памяти собственным глобальным распределителем.

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

/// Собрать `file` как библиотеку: успех и текст ошибок.
fn compile(file: &Path, out_dir: &Path) -> (bool, String) {
    let stem = file.file_stem().unwrap().to_str().unwrap();
    let out = Command::new("rustc")
        .current_dir(crate_dir())
        .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "-A", "warnings"])
        .arg("-o")
        .arg(out_dir.join(format!("{stem}.rmeta")))
        .arg(file)
        .output()
        .expect("не удалось запустить rustc");
    (out.status.success(), String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Метка, которую заготовки внешних файлов задач C и D несут, пока задача
/// не решена.
const STUB: &str = "ЗАГОТОВКА";

/// Глобальный распределитель, который считает выделения памяти в каждом
/// потоке отдельно: тесты идут параллельно и друг другу не мешают.
mod counting {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;

    thread_local! {
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    fn bump() {
        let _ = COUNT.try_with(|c| c.set(c.get() + 1));
    }

    pub struct Counting;

    // SAFETY: всё делает System, счётчик в потоке не выделяет память.
    unsafe impl GlobalAlloc for Counting {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            bump();
            // SAFETY: контракт GlobalAlloc::alloc передаётся как есть.
            unsafe { System.alloc(layout) }
        }

        unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
            bump();
            // SAFETY: то же для alloc_zeroed.
            unsafe { System.alloc_zeroed(layout) }
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            bump();
            // SAFETY: то же для realloc.
            unsafe { System.realloc(ptr, layout, new_size) }
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            // SAFETY: то же для dealloc.
            unsafe { System.dealloc(ptr, layout) }
        }
    }

    /// Результат `f` и число выделений памяти в этом потоке за время её
    /// работы.
    pub fn allocations<R>(f: impl FnOnce() -> R) -> (R, usize) {
        let before = COUNT.with(Cell::get);
        let r = f();
        (r, COUNT.with(Cell::get) - before)
    }
}

#[global_allocator]
static ALLOC: counting::Counting = counting::Counting;

mod task_a {
    use super::sut::a_elision::SIGNATURES;
    use super::{compile, crate_dir, tmp};

    const NAMES: [&str; 8] =
        ["trim_cr", "split_kv", "pick", "count_lines", "make_filter", "name", "lookup", "next_token"];

    /// Части через запятую верхнего уровня: запятые внутри `<>`, `()` и `[]`
    /// не делят.
    fn split_top(s: &str) -> Vec<&str> {
        let (mut out, mut depth, mut start, mut prev) = (Vec::new(), 0, 0, ' ');
        for (i, c) in s.char_indices() {
            match c {
                '(' | '[' | '<' => depth += 1,
                ')' | ']' => depth -= 1,
                '>' if prev != '-' => depth -= 1,
                ',' if depth == 0 => {
                    out.push(s[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
            prev = c;
        }
        let last = s[start..].trim();
        if !last.is_empty() {
            out.push(last);
        }
        out
    }

    /// Имена параметров сигнатуры `fn name…(…)`; приёмник — `"self"`.
    fn params(sig: &str, name: &str) -> Result<Vec<String>, String> {
        let rest = sig.split_once(&format!("fn {name}")).ok_or("нет `fn имя`")?.1;
        let open = rest.find('(').ok_or("нет списка параметров")?;
        let mut depth = 0;
        let mut close = None;
        for (i, c) in rest[open..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close.ok_or("скобки параметров не закрыты")?;
        split_top(&rest[open + 1..close])
            .into_iter()
            .map(|p| {
                if p.ends_with("self") && !p.contains(':') {
                    Ok("self".to_owned())
                } else {
                    let (n, _) = p.split_once(':').ok_or_else(|| format!("параметр без типа: {p}"))?;
                    Ok(n.trim().trim_start_matches("mut ").trim().to_owned())
                }
            })
            .collect()
    }

    /// Вызов `target` с параметрами сигнатуры `sig` (метод — через `self.`).
    fn call(sig: &str, name: &str, target: &str) -> Result<String, String> {
        let ps = params(sig, name)?;
        let method = ps.first().is_some_and(|p| p == "self");
        let args: Vec<_> = ps.iter().filter(|p| *p != "self").cloned().collect();
        let prefix = if method { "self." } else { "" };
        Ok(format!("{prefix}{target}({})", args.join(", ")))
    }

    /// Что в сигнатуре осталось неявным: ссылка без времени жизни, `'_`,
    /// `dyn` без времени жизни объекта.
    fn implicit(sig: &str) -> Option<String> {
        if sig.contains("'_") {
            return Some("`'_` — пропущенное время жизни, его нужно назвать".into());
        }
        for (i, c) in sig.char_indices() {
            if c == '&' && !sig[i + 1..].starts_with('\'') {
                return Some(format!("ссылка без времени жизни: «{}»", &sig[i..]));
            }
        }
        if let Some(i) = sig.find("dyn ") {
            if !sig[i..].contains("+ '") {
                return Some("у `dyn` не указано время жизни объекта (`+ '…`)".into());
            }
        }
        None
    }

    /// Фрагмент с двумя функциями рядом с исходной: `stud_имя` с сигнатурой
    /// студента вызывает исходную, `back_имя` с исходной сигнатурой
    /// вызывает `stud_имя`. Обе компилируются, только если сигнатуры
    /// равносильны.
    fn generate(text: &str, name: &str, answer: &str) -> Result<String, String> {
        let lines: Vec<&str> = text.lines().collect();
        let at = lines.iter().position(|l| l.contains(&format!("fn {name}("))).unwrap();
        let sig_line = lines[at];
        let indent = &sig_line[..sig_line.len() - sig_line.trim_start().len()];
        let end = at + lines[at..].iter().position(|l| *l == format!("{indent}}}")).unwrap();
        let orig = sig_line.trim().trim_start_matches("pub ").trim_end_matches('{').trim();
        let stud = answer.trim().trim_start_matches("pub ").trim();
        let head = format!("fn {name}");
        if !stud.starts_with(&head) || !stud[head.len()..].starts_with(['<', '(']) {
            return Err(format!("сигнатура должна начинаться с `{head}`: {stud}"));
        }
        let stud_name = format!("stud_{name}");
        let rename = |sig: &str, to: &str| sig.replacen(&head, &format!("fn {to}"), 1);
        let extra = [
            format!("{indent}{} {{", rename(stud, &stud_name)),
            format!("{indent}    {}", call(stud, name, name)?),
            format!("{indent}}}"),
            format!("{indent}{} {{", rename(orig, &format!("back_{name}"))),
            format!("{indent}    {}", call(orig, name, &stud_name)?),
            format!("{indent}}}"),
        ];
        let mut out: Vec<String> = lines[..=end].iter().map(|l| l.to_string()).collect();
        out.extend(extra);
        out.extend(lines[end + 1..].iter().map(|l| l.to_string()));
        Ok(out.join("\n") + "\n")
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn signatures_match() {
        let dir = crate_dir().join("../../code/ch05/fragments");
        let out = tmp("a");
        let mut wrong = Vec::new();
        for (i, (name, answer)) in NAMES.iter().zip(SIGNATURES).enumerate() {
            let file = format!("a{}.rs", i + 1);
            let text = std::fs::read_to_string(dir.join(&file)).unwrap();
            let (ok, log) = compile(&dir.join(&file), &out);
            let e0106 = !ok && log.contains("error[E0106]");
            assert!(ok || e0106, "{file} не компилируется по другой причине:\n{log}");
            let answer = answer.trim();
            if answer.is_empty() {
                wrong.push(format!("{file}: ответ не заполнен"));
            } else if answer == "E0106" || e0106 {
                if answer != "E0106" {
                    wrong.push(format!("{file}: элизия здесь не работает, исходная сигнатура — E0106"));
                } else if !e0106 {
                    wrong.push(format!("{file}: исходная сигнатура компилируется, E0106 нет"));
                }
            } else if let Some(why) = implicit(answer) {
                wrong.push(format!("{file}: {why}"));
            } else {
                match generate(&text, name, answer) {
                    Err(e) => wrong.push(format!("{file}: {e}")),
                    Ok(src) => {
                        let generated = out.join(format!("check_a{}.rs", i + 1));
                        std::fs::write(&generated, src).unwrap();
                        let (ok, log) = compile(&generated, &out);
                        if !ok {
                            wrong.push(format!("{file}: сигнатура не равносильна исходной\n{log}"));
                        }
                    }
                }
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n\n"));
    }
}

mod task_b {
    use super::counting::allocations;
    use super::sut::b_lexer::{LexError, LexErrorKind, Lexer, Token};

    const CONFIG: &str = "\
# станция телеметрии
[uart]
baud = 9600   # скорость
mode=8N1
name = \"метео; 3\"

[spi]
clock = 1000000
empty =
";

    fn tokens(src: &str) -> Vec<Result<Token<'_>, LexError>> {
        Lexer::new(src).collect()
    }

    fn err(line: u32, col: u32, kind: LexErrorKind) -> Result<Token<'static>, LexError> {
        Err(LexError { line, col, kind })
    }

    /// Срез лежит внутри исходного текста.
    fn inside(src: &str, s: &str) -> bool {
        let (base, p) = (src.as_ptr() as usize, s.as_ptr() as usize);
        base <= p && p + s.len() <= base + src.len()
    }

    #[test]
    fn example() {
        use Token::*;
        let expected = [
            Section("uart"),
            Key("baud"),
            Value("9600"),
            Key("mode"),
            Value("8N1"),
            Key("name"),
            Value("метео; 3"),
            Section("spi"),
            Key("clock"),
            Value("1000000"),
            Key("empty"),
            Value(""),
        ];
        let got: Vec<_> = tokens(CONFIG).into_iter().map(Result::unwrap).collect();
        assert_eq!(got, expected);
    }

    #[test]
    fn line_endings() {
        use Token::*;
        let got = tokens("[a]\r\nk = v \r\n\r\nx=1");
        assert_eq!(got, [Ok(Section("a")), Ok(Key("k")), Ok(Value("v")), Ok(Key("x")), Ok(Value("1"))]);
        assert!(tokens("").is_empty());
        assert!(tokens("\n\n  # только комментарий\n; и этот\n").is_empty());
    }

    #[test]
    fn errors_with_position() {
        use LexErrorKind::*;
        let src = "[uart\nbaud 9600\n=1\nba$d=1\n[]\n[ua rt]\n[spi] x\nname = \"abc\n\
                   x = \"абв\" z\n[секция]\nok = 1\n\t  speed\n";
        let expected = [
            err(1, 1, UnclosedSection),
            err(2, 6, MissingEquals),
            err(3, 1, BadName),
            err(4, 3, BadName),
            err(5, 2, BadName),
            err(6, 4, BadName),
            err(7, 7, TrailingGarbage),
            err(8, 8, UnclosedQuote),
            err(9, 11, TrailingGarbage),
            err(10, 2, BadName),
            Ok(Token::Key("ok")),
            Ok(Token::Value("1")),
            err(12, 9, MissingEquals),
        ];
        assert_eq!(tokens(src), expected);
    }

    #[test]
    fn comments_in_values() {
        use Token::*;
        let got = tokens("a = x#y\nb = x #y\nc = \"x # y\" ; z\nd = ;\ne=\"\"");
        let values: Vec<_> = got
            .into_iter()
            .filter_map(|t| match t.unwrap() {
                Value(v) => Some(v),
                _ => None,
            })
            .collect();
        assert_eq!(values, ["x#y", "x", "x # y", "", ""]);
    }

    #[test]
    fn zero_copy_and_zero_allocations() {
        let n = if cfg!(miri) { 20 } else { 2000 };
        let big: String =
            (0..n).map(|i| format!("[s{i}]\nkey{i} = значение {i} # c\nq = \"x y\"\nbad line\n")).collect();
        let ((ok, errors), allocs) = allocations(|| {
            let (mut ok, mut errors) = (0, 0);
            for t in Lexer::new(&big) {
                match t {
                    Ok(Token::Section(s) | Token::Key(s) | Token::Value(s)) => {
                        assert!(inside(&big, s), "{s:?} не указывает в исходный текст");
                        ok += 1;
                    }
                    Err(_) => errors += 1,
                }
            }
            (ok, errors)
        });
        assert_eq!((ok, errors), (5 * n, n));
        assert_eq!(allocs, 0, "лексер выделял память");
    }

    /// Генератор xorshift64: случайные конфигурации с известными лексемами.
    struct Rng(u64);

    impl Rng {
        fn below(&mut self, n: usize) -> usize {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            (self.0 % n as u64) as usize
        }

        fn pick<'a>(&mut self, xs: &[&'a str]) -> &'a str {
            xs[self.below(xs.len())]
        }

        fn name(&mut self) -> String {
            let chars: Vec<char> = "abcxyzXYZ019_.-".chars().collect();
            (0..1 + self.below(7)).map(|_| chars[self.below(chars.len())]).collect()
        }
    }

    #[derive(Debug, PartialEq)]
    enum Owned {
        Section(String),
        Key(String),
        Value(String),
    }

    fn owned(t: Token<'_>) -> Owned {
        match t {
            Token::Section(s) => Owned::Section(s.into()),
            Token::Key(s) => Owned::Key(s.into()),
            Token::Value(s) => Owned::Value(s.into()),
        }
    }

    /// Строки конфигурации и лексемы, которые из неё должны получиться.
    fn config(rng: &mut Rng) -> (String, Vec<Owned>) {
        const WORDS: [&str; 7] = ["9600", "8N1", "метео", "a#b", "x;y", "1.5e3", "uart0"];
        let (mut src, mut expected) = (String::new(), Vec::new());
        for _ in 0..rng.below(12) {
            let indent = rng.pick(&["", " ", "\t", "  "]);
            let tail = rng.pick(&["", " ", "  # комментарий", " ; c", "\t#"]);
            let eol = rng.pick(&["\n", "\r\n"]);
            match rng.below(5) {
                0 => src += rng.pick(&["", "# c", "  ; c", "   "]),
                1 => {
                    let name = rng.name();
                    src += &format!("{indent}[{name}]{tail}");
                    expected.push(Owned::Section(name));
                }
                _ => {
                    let key = rng.name();
                    let words: Vec<&str> = (0..rng.below(4)).map(|_| rng.pick(&WORDS)).collect();
                    let quoted = rng.below(3) == 0;
                    let value = if quoted { words.join(rng.pick(&[" ", " # ", " ; "])) } else { words.join(" ") };
                    let written = if quoted { format!("\"{value}\"") } else { value.clone() };
                    let eq = rng.pick(&["=", " =", "= ", " = ", "  =  "]);
                    src += &format!("{indent}{key}{eq}{written}{tail}");
                    expected.push(Owned::Key(key));
                    expected.push(Owned::Value(value));
                }
            }
            src += eol;
        }
        (src, expected)
    }

    #[test]
    fn random_configs() {
        let mut rng = Rng(0x5EED_CAFE);
        for _ in 0..if cfg!(miri) { 10 } else { 500 } {
            let (src, expected) = config(&mut rng);
            let got: Result<Vec<_>, _> = Lexer::new(&src).map(|t| t.map(owned)).collect();
            assert_eq!(got.as_ref(), Ok(&expected), "конфигурация:\n{src}");
        }
    }
}

mod task_c {
    use super::sut::c_variance::{TYPES, VARIANCE, Variance};
    use super::{STUB, compile, crate_dir, tmp};
    use std::path::Path;

    const TRUTH: [&str; 10] = [
        "&'a u8",
        "&'a mut u8",
        "&'a mut &'a u8",
        "*const &'a u8",
        "*mut &'a u8",
        "Cell<&'a u8>",
        "fn(&'a u8)",
        "fn(fn(&'a u8))",
        "Box<dyn Fn(&'a u8)>",
        "core::str::Lines<'a>",
    ];

    /// Пробные функции: `T<'b>` → `T<'a>` при `'b: 'a` компилируется, только
    /// если тип ковариантен, обратное — только если контравариантен.
    fn probe(i: usize, ty: &str, out: &Path) -> Variance {
        let long = ty.replace("'a", "'b");
        let mut accepts = [false; 2];
        for (k, (from, to)) in [(&long[..], ty), (ty, &long[..])].into_iter().enumerate() {
            let src = format!(
                "use core::cell::Cell;\npub fn probe<'a, 'b: 'a>(x: {from}) -> {to} {{\n    x\n}}\n"
            );
            let file = out.join(format!("probe{i}_{k}.rs"));
            std::fs::write(&file, src).unwrap();
            accepts[k] = compile(&file, out).0;
        }
        match accepts {
            [true, false] => Variance::Co,
            [false, true] => Variance::Contra,
            [false, false] => Variance::Inv,
            [true, true] => panic!("{ty}: не зависит от 'a"),
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn variance_table() {
        assert_eq!(TYPES, TRUTH, "список типов в задаче менять не нужно");
        let out = tmp("c");
        let mut wrong = Vec::new();
        for (i, (ty, answer)) in TRUTH.iter().zip(VARIANCE).enumerate() {
            let truth = probe(i, ty, &out);
            match answer {
                None => wrong.push(format!("{ty}: ответ не заполнен")),
                Some(a) if a != truth => wrong.push(format!("{ty}: ваш ответ {a:?}, на самом деле {truth:?}")),
                Some(_) => {}
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn covariant_and_cell() {
        let dir = crate_dir().join("variance");
        let out = tmp("c");
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).unwrap();
        for f in ["covariant.rs", "cell.rs"] {
            assert!(!read(f).contains(STUB), "variance/{f}: задача не решена (метка {STUB})");
        }
        let (ok, log) = compile(&dir.join("covariant.rs"), &out);
        assert!(ok, "variance/covariant.rs должен компилироваться:\n{log}");
        assert!(read("cell.rs").contains("Cell<"), "в variance/cell.rs нет Cell");
        let (ok, log) = compile(&dir.join("cell.rs"), &out);
        assert!(!ok, "variance/cell.rs должен не компилироваться");
        assert!(
            log.contains("Cell") && log.contains("invariant"),
            "variance/cell.rs отвергнут не из-за инвариантности Cell:\n{log}"
        );
    }
}

mod task_d {
    use super::sut::d_windows::{LendingIterator, despike, windows_mut};
    use super::{STUB, compile, crate_dir, tmp};

    fn collect(v: &mut [u32], size: usize) -> Vec<Vec<u32>> {
        let mut it = windows_mut(v, size);
        let mut out = Vec::new();
        while let Some(w) = it.next() {
            out.push(w.to_vec());
        }
        out
    }

    #[test]
    fn windows() {
        let mut v = [1, 2, 3, 4];
        assert_eq!(collect(&mut v, 2), [[1, 2], [2, 3], [3, 4]]);
        assert_eq!(collect(&mut v, 4), [[1, 2, 3, 4]]);
        assert!(collect(&mut v, 5).is_empty());
        assert!(collect(&mut [], 1).is_empty());
        assert_eq!(collect(&mut v, usize::MAX), Vec::<Vec<u32>>::new());
    }

    #[test]
    #[should_panic]
    fn zero_size_panics() {
        windows_mut(&mut [1, 2], 0);
    }

    #[test]
    fn overlapping_writes() {
        // Сумма с накоплением на месте: каждое окно видит запись предыдущего.
        let mut v = [1, 2, 3, 4];
        let mut it = windows_mut(&mut v, 2);
        while let Some(w) = it.next() {
            w[1] += w[0];
        }
        assert_eq!(v, [1, 3, 6, 10]);
    }

    #[test]
    fn despike_samples() {
        let mut s = [10, 10, 99, 10, 10, -50, 10, 11];
        assert_eq!(despike(&mut s, 20), 2);
        assert_eq!(s, [10, 10, 10, 10, 10, 10, 10, 11]);
        let mut extreme = [i32::MIN, i32::MAX, i32::MIN];
        assert_eq!(despike(&mut extreme, 0), 1);
        assert_eq!(extreme, [i32::MIN; 3]);
        assert_eq!(despike(&mut [1, 2], 0), 0);
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn iterator_is_rejected() {
        let file = crate_dir().join("windows/iterator.rs");
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(!text.contains(STUB), "windows/iterator.rs: задача не решена (метка {STUB})");
        assert!(text.contains("Iterator for WindowsMut"), "нужна реализация Iterator for WindowsMut");
        let (ok, log) = compile(&file, &tmp("d"));
        assert!(!ok, "windows/iterator.rs скомпилировался");
        assert!(
            log.contains("lifetime may not live long enough"),
            "windows/iterator.rs отвергнут по другой причине:\n{log}"
        );
    }
}
