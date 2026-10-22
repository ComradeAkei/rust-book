// Общие тесты задач главы 10.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch10/tests/ch10.rs` и `solutions/ch10/tests/ch10.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задачи A, B, C и D собирают пробы закреплённым rustc и cargo: A —
// фрагменты из code/ch10/fragments, B — крейт для хоста, C — для
// Cortex-M4F, C и D — пробы против самого крейта. Под Miri эти тесты
// пропускаются, как и всё, что зовёт qsort из libc; остальное — обычный
// код и под Miri идёт.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

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

/// Коды ошибок из вывода rustc: `error[E0277]` → `E0277`.
fn codes(err: &str) -> Vec<&str> {
    err.match_indices("error[E").map(|(i, _)| &err[i + 6..i + 11]).collect()
}

/// Имя библиотеки крейта в именах файлов: `ch10-solutions` → `ch10_solutions`.
fn lib_name() -> String {
    env!("CARGO_PKG_NAME").replace('-', "_")
}

/// Собрать крейт задач закреплённым cargo в отдельный каталог. cargo через
/// прокси rustup, а не env!("CARGO"): прокси прочтёт rust-toolchain.toml и
/// передаст выбор дальше, иначе embedded-hal из реестра, где
/// rust-toolchain.toml нет, собрал бы тулчейн по умолчанию.
fn cargo_build(name: &str, args: &[&str]) -> PathBuf {
    let target = tmp(name);
    let out = pinned("cargo", crate_dir())
        .args(["build", "--lib", "--target-dir"])
        .arg(&target)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "cargo build {args:?}:\n{}", text(&out.stderr));
    target
}

/// Разбор ELF (32 и 64 бита, little-endian) и архивов ar: символы с
/// адресом и размером. Хватает и для объектных файлов в .rlib,
/// и для собранной программы.
mod elf {
    include!("elf.rs");
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use std::collections::BTreeSet;

    /// Сколько разных тел функций с `checksum` в пути оказалось в программе
    /// из фрагмента `a{i}.rs`.
    fn measured(i: usize) -> usize {
        let file = crate_dir().join(format!("../../code/ch10/fragments/a{i}.rs"));
        let src = std::fs::read_to_string(&file).unwrap();
        let extra: Vec<&str> = src
            .lines()
            .filter_map(|l| l.strip_prefix("// rustc: "))
            .flat_map(str::split_whitespace)
            .collect();
        let exe = tmp("a").join(format!("a{i}"));
        let out = pinned("rustc", crate_dir())
            .args(["--edition", "2024", "-C", "opt-level=3", "-A", "warnings", "-o"])
            .arg(&exe)
            .args(&extra)
            .arg(&file)
            .output()
            .unwrap();
        assert!(out.status.success(), "a{i}.rs не собирается:\n{}", text(&out.stderr));
        let bytes = std::fs::read(&exe).unwrap();
        let elf = elf::parse(&bytes);
        // Схема v0: имя пути записано с длиной, `8checksum`.
        let bodies: BTreeSet<u64> = elf
            .symbols
            .iter()
            .filter(|s| s.func && s.shndx != 0 && s.size > 0 && s.name.contains("8checksum"))
            .map(|s| s.value)
            .collect();
        bodies.len()
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_a_copies() {
        let answers = sut::a_copies::copies();
        let actual: Vec<usize> = std::thread::scope(|s| {
            let jobs: Vec<_> = (1..=6).map(|i| s.spawn(move || measured(i))).collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        let wrong: Vec<String> = (0..6)
            .filter(|&i| answers[i] != actual[i])
            .map(|i| format!("a{}: ответ {}, а в программе {}", i + 1, answers[i], actual[i]))
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use core::cmp::Ordering;
    use sut::b_sort::{order, sort_libc, sort_rust};

    /// Псевдослучайные u32 во всём диапазоне: xorshift32.
    fn data(n: usize, mut x: u32) -> Vec<u32> {
        (0..n)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                x
            })
            .collect()
    }

    #[test]
    fn task_b_order() {
        let cases = [
            (0, 0, Ordering::Equal),
            (0, 1, Ordering::Less),
            (1, 0, Ordering::Greater),
            // Разность 0x8000_0001 в int — отрицательное число: на этой паре
            // компаратор «a - b» ошибается.
            (0x8000_0001, 0, Ordering::Greater),
            (0, u32::MAX, Ordering::Less),
            (u32::MAX, u32::MAX, Ordering::Equal),
            (0x7fff_ffff, 0x8000_0000, Ordering::Less),
        ];
        for (a, b, want) in cases {
            assert_eq!(order(a, b), want, "cmp_u32({a:#x}, {b:#x})");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "qsort из libc Miri не исполняет")]
    fn task_b_libc() {
        for seed in [1, 12345, 0xdead_beef] {
            let mut v = data(10_000, seed);
            let mut want = v.clone();
            want.sort();
            sort_libc(&mut v);
            assert_eq!(v, want, "qsort, зерно {seed}");
        }
        let mut dup = vec![3, 1, 3, 0, u32::MAX, 1];
        sort_libc(&mut dup);
        assert_eq!(dup, [0, 1, 1, 3, 3, u32::MAX]);
        let mut empty: [u32; 0] = [];
        sort_libc(&mut empty);
        let mut one = [7];
        sort_libc(&mut one);
        assert_eq!(one, [7]);
    }

    #[test]
    fn task_b_rust() {
        let mut v = data(1_000, 777);
        let mut want = v.clone();
        want.sort();
        sort_rust(&mut v);
        assert_eq!(v, want);
        let mut empty: [u32; 0] = [];
        sort_rust(&mut empty);
    }

    /// В release-сборке для хоста: `qsort` — внешний символ, `cmp_u32` —
    /// отдельная функция, которую вызывают по адресу, а замыкание из
    /// `sort_rust` отдельной функцией не существует — оно встроено в
    /// мономорфизованную сортировку.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_b_inlined() {
        let target = cargo_build("b-release", &["--release"]);
        let rlib = target.join("release").join(format!("lib{}.rlib", lib_name()));
        let symbols = elf::rlib_symbols(&std::fs::read(rlib).unwrap());
        let defined = |pred: &dyn Fn(&str) -> bool| {
            symbols.iter().any(|s| s.func && s.shndx != 0 && pred(&s.name))
        };
        assert!(
            symbols.iter().any(|s| s.name == "qsort" && s.shndx == 0),
            "sort_libc не вызывает qsort из libc"
        );
        assert!(defined(&|n| n.contains("7cmp_u32")), "cmp_u32 не отдельная функция");
        assert!(defined(&|n| n.contains("6b_sort9sort_rust")), "sort_rust не найдена");
        // Замыкание в схеме v0: `_RNC` + путь функции, где оно написано.
        assert!(
            !defined(&|n| n.starts_with("_RNC") && n.contains("6b_sort9sort_rust")),
            "замыкание из sort_rust осталось отдельной функцией: сравнение не встроено"
        );
    }
}

// ---------------------------------------------------------------- задачи C и D: пробы

/// Пробы против самого крейта: отладочная сборка в отдельном каталоге,
/// затем rustc с `--extern sut=…` и `--extern embedded_hal=…`.
mod probes {
    use super::*;

    struct Deps {
        rlib: PathBuf,
        deps: PathBuf,
        hal: PathBuf,
    }

    fn deps() -> &'static Deps {
        static DEPS: OnceLock<Deps> = OnceLock::new();
        DEPS.get_or_init(|| {
            let target = cargo_build("probes", &[]);
            let deps = target.join("debug/deps");
            let hal = std::fs::read_dir(&deps)
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| {
                    let n = p.file_name().unwrap().to_string_lossy();
                    n.starts_with("libembedded_hal-") && n.ends_with(".rlib")
                })
                .expect("нет libembedded_hal-*.rlib");
            Deps { rlib: target.join("debug").join(format!("lib{}.rlib", lib_name())), deps, hal }
        })
    }

    /// Собрать пробу; вернуть коды ошибок (пусто — собралась).
    pub fn probe(name: &str, src: &str) -> Vec<String> {
        let d = deps();
        let dir = tmp("probes");
        let file = dir.join(format!("{name}.rs"));
        std::fs::write(&file, src).unwrap();
        let out = pinned("rustc", crate_dir())
            .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata"])
            .args(["--error-format", "short", "-A", "warnings", "--crate-name", name, "--out-dir"])
            .arg(&dir)
            .arg("-L")
            .arg(format!("dependency={}", d.deps.display()))
            .arg("--extern")
            .arg(format!("sut={}", d.rlib.display()))
            .arg("--extern")
            .arg(format!("embedded_hal={}", d.hal.display()))
            .arg(&file)
            .output()
            .unwrap();
        let err = text(&out.stderr);
        let mut found: Vec<String> = codes(&err).into_iter().map(String::from).collect();
        if !out.status.success() && found.is_empty() {
            panic!("проба {name} не собралась без кода ошибки:\n{err}");
        }
        found.dedup();
        found
    }

    /// Прогнать пробы параллельно и сравнить с ожиданием.
    pub fn check(cases: &[(&str, &str, &[&str])]) {
        let results: Vec<Vec<String>> = std::thread::scope(|s| {
            let jobs: Vec<_> =
                cases.iter().map(|(name, src, _)| s.spawn(move || probe(name, src))).collect();
            jobs.into_iter().map(|j| j.join().unwrap()).collect()
        });
        let wrong: Vec<String> = cases
            .iter()
            .zip(&results)
            .filter(|((_, _, want), got)| got.iter().map(String::as_str).ne(want.iter().copied()))
            .map(|((name, _, want), got)| format!("{name}: ждали {want:?}, а rustc говорит {got:?}"))
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use sut::c_matrix::Matrix;

    #[test]
    fn task_c_math() {
        let a = Matrix([[1, 2, 3], [4, 5, 6]]);
        let b = Matrix([[7, 8], [9, 10], [11, 12]]);
        assert_eq!(a.matmul(&b), Matrix([[58, 64], [139, 154]]));
        assert_eq!(a * b, Matrix([[58, 64], [139, 154]]));
        assert_eq!(b.matmul(&a), Matrix([[39, 54, 69], [49, 68, 87], [59, 82, 105]]));
        assert_eq!(a.transpose(), Matrix([[1, 4], [2, 5], [3, 6]]));
        assert_eq!(a.apply([1, 0, -1]), [-2, -2]);
        assert_eq!(a + a, Matrix([[2, 4, 6], [8, 10, 12]]));
        assert_eq!(Matrix::<3, 3>::identity(), Matrix([[1, 0, 0], [0, 1, 0], [0, 0, 1]]));
        assert_eq!(Matrix::<4, 4>::identity().trace(), 4);
        let sq = Matrix([[2, 1], [1, 3]]);
        assert_eq!(sq.matmul(&Matrix::<2, 2>::identity()), sq);
        assert_eq!(Matrix::<2, 3>::from_fn(|i, j| (10 * i + j) as i32), Matrix([[0, 1, 2], [10, 11, 12]]));
        assert_eq!(Matrix::<1, 1>::ZERO, Matrix([[0]]));
        // Переполнение — по модулю 2³², без паники и в отладочной сборке.
        let big = Matrix([[i32::MAX, i32::MAX]]);
        assert_eq!(big.matmul(&Matrix([[2], [0]])), Matrix([[-2]]));
        assert_eq!(big + big, Matrix([[-2, -2]]));
        let mut out = Matrix::<2, 2>::ZERO;
        sut::c_matrix::ch10c_mul2(&sq, &sq, &mut out);
        assert_eq!(out, Matrix([[5, 5], [5, 10]]));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_c_types() {
        const USE: &str = "use sut::c_matrix::Matrix;\n";
        let cases: [(&str, String, &[&str]); 7] = [
            (
                "c_ok",
                format!("{USE}pub fn f(a: Matrix<2, 3>, b: Matrix<3, 4>) -> Matrix<2, 4> {{ a.matmul(&b) }}"),
                &[],
            ),
            (
                "c_ok_ops",
                format!("{USE}pub fn f(a: Matrix<2, 3>, b: Matrix<3, 2>) -> Matrix<2, 2> {{ a * b + Matrix::identity() }}"),
                &[],
            ),
            (
                "c_ok_transpose",
                format!("{USE}pub fn f(a: Matrix<2, 3>) -> Matrix<3, 2> {{ a.transpose() }}"),
                &[],
            ),
            (
                "c_mismatch",
                format!("{USE}pub fn f(a: Matrix<2, 3>, b: Matrix<2, 3>) -> Matrix<2, 3> {{ a.matmul(&b) }}"),
                &["E0308"],
            ),
            (
                "c_mismatch_op",
                format!("{USE}pub fn f(a: Matrix<2, 3>, b: Matrix<2, 3>) {{ let _ = a * b; }}"),
                &["E0308"],
            ),
            (
                "c_add_dims",
                format!("{USE}pub fn f(a: Matrix<2, 3>, b: Matrix<3, 2>) {{ let _ = a + b; }}"),
                &["E0308"],
            ),
            (
                "c_identity_rect",
                format!("{USE}pub fn f() -> Matrix<2, 3> {{ Matrix::<2, 3>::identity() }}"),
                &["E0599"],
            ),
        ];
        let cases: Vec<(&str, &str, &[&str])> =
            cases.iter().map(|(n, s, w)| (*n, s.as_str(), *w)).collect();
        probes::check(&cases);
    }

    /// Пять размеров — пять разных копий `matmul` в коде для Cortex-M4F.
    /// Размеры печатаются для отчёта: `cargo test -- --nocapture task_c_sizes`.
    #[test]
    #[cfg_attr(miri, ignore = "запускает cargo")]
    fn task_c_sizes() {
        let t = "thumbv7em-none-eabihf";
        let target = cargo_build("c-thumb", &["--release", "--target", t]);
        let rlib = target.join(t).join("release").join(format!("lib{}.rlib", lib_name()));
        let symbols = elf::rlib_symbols(&std::fs::read(rlib).unwrap());
        let mut sizes = Vec::new();
        for n in [2, 3, 4, 6, 8] {
            let name = format!("ch10c_mul{n}");
            let s = symbols
                .iter()
                .find(|s| s.name == name && s.shndx != 0)
                .unwrap_or_else(|| panic!("нет функции {name}"));
            sizes.push((n, s.size));
        }
        for (n, size) in &sizes {
            eprintln!("{t}: ch10c_mul{n} — {size} байт");
        }
        let mut distinct: Vec<u64> = sizes.iter().map(|&(_, s)| s).collect();
        distinct.dedup();
        assert!(distinct.len() > 1, "у всех размеров один и тот же код: {sizes:?}");
        assert!(sizes.iter().all(|&(_, s)| s > 4), "функции пусты: {sizes:?}");
    }
}

// ---------------------------------------------------------------- задача D

mod task_d {
    use super::*;
    use embedded_hal::delay::DelayNs;
    use embedded_hal::spi::{ErrorKind, ErrorType, Operation, SpiDevice};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;
    use sut::d_flash::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct SimError;

    impl embedded_hal::spi::Error for SimError {
        fn kind(&self) -> ErrorKind {
            ErrorKind::Other
        }
    }

    /// Модель W25Q128: 16 МиБ, стёртые байты — 0xFF, запись только
    /// сбрасывает биты и заворачивается внутри страницы, BUSY держится
    /// заданное число опросов. Журнал — строка на транзакцию или паузу.
    struct Chip {
        mem: BTreeMap<u32, u8>,
        id: [u8; 3],
        wel: bool,
        busy: u32,
        busy_polls: u32,
        asleep: bool,
        /// Номер транзакции, которая вернёт ошибку шины.
        fail_at: Option<usize>,
        count: usize,
        log: Vec<String>,
    }

    #[derive(Clone)]
    struct Sim(Rc<RefCell<Chip>>);

    struct Delay(Rc<RefCell<Chip>>);

    /// Для `unwrap` у `Flash::new`: ошибка возвращает и шину.
    impl std::fmt::Debug for Sim {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("Sim")
        }
    }

    impl Sim {
        fn new() -> (Sim, Delay) {
            let chip = Chip {
                mem: BTreeMap::new(),
                id: [0xEF, 0x40, 0x18],
                wel: false,
                busy: 0,
                busy_polls: 2,
                asleep: false,
                fail_at: None,
                count: 0,
                log: Vec::new(),
            };
            let rc = Rc::new(RefCell::new(chip));
            (Sim(rc.clone()), Delay(rc))
        }

        fn chip(&self) -> std::cell::RefMut<'_, Chip> {
            self.0.borrow_mut()
        }

        /// Журнал с прошлого вызова.
        fn take(&self) -> Vec<String> {
            std::mem::take(&mut self.chip().log)
        }

        fn byte(&self, addr: u32) -> u8 {
            *self.0.borrow().mem.get(&addr).unwrap_or(&0xFF)
        }
    }

    impl DelayNs for Delay {
        fn delay_ns(&mut self, ns: u32) {
            self.0.borrow_mut().log.push(format!("пауза {ns} нс"));
        }
    }

    impl ErrorType for Sim {
        type Error = SimError;
    }

    impl SpiDevice for Sim {
        fn transaction(&mut self, ops: &mut [Operation<'_, u8>]) -> Result<(), SimError> {
            let mut c = self.0.borrow_mut();
            let n = c.count;
            c.count += 1;
            if c.fail_at == Some(n) {
                c.log.push("ошибка шины".into());
                return Err(SimError);
            }
            // Первый Write — команда и адрес, дальше данные и чтение.
            let mut head: Vec<u8> = Vec::new();
            let mut data: Vec<u8> = Vec::new();
            let mut reads = 0;
            for (i, op) in ops.iter().enumerate() {
                match op {
                    Operation::Write(w) if i == 0 => head.extend_from_slice(w),
                    Operation::Write(w) => data.extend_from_slice(w),
                    Operation::Read(r) => reads += r.len(),
                    _ => panic!("модель понимает только Write и Read"),
                }
            }
            let cmd = head[0];
            let with_addr = matches!(cmd, 0x02 | 0x03 | 0x20);
            assert_eq!(head.len(), if with_addr { 4 } else { 1 }, "команда {cmd:02x}: {head:02x?}");
            let addr = if with_addr { u32::from_be_bytes([0, head[1], head[2], head[3]]) } else { 0 };
            let mut out: Vec<u8> = vec![0xFF; reads];
            let mut note = String::new();
            if c.asleep && cmd != 0xAB {
                note = " (спит)".into();
            } else {
                match cmd {
                    0xAB => c.asleep = false,
                    0xB9 => c.asleep = true,
                    0x9F => out[..3].copy_from_slice(&c.id),
                    0x05 => {
                        for b in out.iter_mut() {
                            *b = u8::from(c.wel) << 1 | u8::from(c.busy > 0);
                            if c.busy > 0 {
                                c.busy -= 1;
                                if c.busy == 0 {
                                    c.wel = false;
                                }
                            }
                        }
                    }
                    0x06 | 0x04 if c.busy > 0 => note = " (занята)".into(),
                    0x06 => c.wel = true,
                    0x04 => c.wel = false,
                    0x03 => {
                        for (i, b) in out.iter_mut().enumerate() {
                            let a = (addr + i as u32) % (1 << 24);
                            *b = *c.mem.get(&a).unwrap_or(&0xFF);
                        }
                    }
                    0x02 | 0x20 if !c.wel || c.busy > 0 => note = " (без WEL)".into(),
                    0x02 => {
                        let page = addr & !0xFF;
                        for (i, &b) in data.iter().enumerate() {
                            let a = page | ((addr + i as u32) & 0xFF);
                            let old = *c.mem.get(&a).unwrap_or(&0xFF);
                            c.mem.insert(a, old & b);
                        }
                        c.busy = c.busy_polls;
                    }
                    0x20 => {
                        let base = addr & !0xFFF;
                        c.mem.retain(|&a, _| a & !0xFFF != base);
                        c.busy = c.busy_polls;
                    }
                    _ => note = " (не понята)".into(),
                }
            }
            let mut line = format!("{cmd:02x}");
            if with_addr {
                line += &format!(" {addr:06x}");
            }
            if !data.is_empty() {
                line += &format!(" +{}", data.len());
            }
            if reads > 0 && reads <= 3 {
                let hex: Vec<String> = out.iter().map(|b| format!("{b:02x}")).collect();
                line += &format!(" → {}", hex.join(" "));
            } else if reads > 3 {
                line += &format!(" → {reads} байт");
            }
            line += &note;
            c.log.push(line);
            drop(c);
            let mut pos = 0;
            for op in ops.iter_mut() {
                if let Operation::Read(r) = op {
                    let len = r.len();
                    r.copy_from_slice(&out[pos..pos + len]);
                    pos += len;
                }
            }
            Ok(())
        }
    }

    fn ready() -> (Flash<Sim, Ready>, Sim, Delay) {
        let (sim, mut delay) = Sim::new();
        let flash = Flash::new(sim.clone(), &mut delay).unwrap();
        sim.take();
        (flash, sim, delay)
    }

    #[test]
    fn task_d_new() {
        let (sim, mut delay) = Sim::new();
        let flash = Flash::new(sim.clone(), &mut delay).unwrap();
        assert_eq!(sim.take(), ["ab", "пауза 3000 нс", "9f → ef 40 18", "05 → 00"]);
        drop(flash.free());

        let (sim, mut delay) = Sim::new();
        sim.chip().id = [0xC2, 0x20, 0x18];
        let Err((_, e)) = Flash::new(sim.clone(), &mut delay) else { panic!("чужой ID принят") };
        assert_eq!(e, Error::WrongId([0xC2, 0x20, 0x18]));
        assert_eq!(sim.take(), ["ab", "пауза 3000 нс", "9f → c2 20 18"]);

        let (sim, mut delay) = Sim::new();
        {
            let mut c = sim.chip();
            c.wel = true;
            c.busy = 5;
        }
        let Err((_, e)) = Flash::new(sim.clone(), &mut delay) else { panic!("занятая принята") };
        assert_eq!(e, Error::StillBusy);
        assert_eq!(sim.take(), ["ab", "пауза 3000 нс", "9f → ef 40 18", "05 → 03"]);

        let (sim, mut delay) = Sim::new();
        sim.chip().fail_at = Some(1);
        let Err((_, e)) = Flash::new(sim.clone(), &mut delay) else { panic!("ошибка шины потеряна") };
        assert_eq!(e, Error::Spi(SimError));
    }

    #[test]
    fn task_d_write_cycle() {
        let (flash, sim, _) = ready();
        let we = flash.write_enable().unwrap();
        assert_eq!(sim.take(), ["06", "05 → 02"]);
        let busy = we.program(0x0100, &[0x12, 0x34, 0x56, 0x78]).unwrap();
        assert_eq!(sim.take(), ["02 000100 +4"]);
        let Poll::Pending(busy) = busy.poll().unwrap() else { panic!("запись кончилась сразу") };
        let mut flash = busy.wait(10).unwrap();
        assert_eq!(sim.take(), ["05 → 03", "05 → 03", "05 → 00"]);
        let mut buf = [0; 5];
        flash.read(0x0100, &mut buf).unwrap();
        assert_eq!(sim.take(), ["03 000100 → 5 байт"]);
        assert_eq!(buf, [0x12, 0x34, 0x56, 0x78, 0xFF]);
        let mut id = [0; 2];
        flash.read(0xFF_FFFE, &mut id).unwrap();
        assert_eq!(flash.read(0xFF_FFFF, &mut id), Err(Error::OutOfRange));
    }

    #[test]
    fn task_d_page_and_sector() {
        let (flash, sim, _) = ready();
        let we = flash.write_enable().unwrap();
        sim.take();
        // 16 байт с 0xF8: хвост пересёк бы границу страницы и завернулся.
        let failed = we.program(0xF8, &[0; 16]).unwrap_err();
        assert_eq!(failed.error, Error::PageOverflow);
        let failed = failed.flash.program(0x10, &[]).unwrap_err();
        assert_eq!(failed.error, Error::PageOverflow);
        let failed = failed.flash.program(0, &[0; 257]).unwrap_err();
        assert_eq!(failed.error, Error::PageOverflow);
        let failed = failed.flash.erase_sector(0x0800).unwrap_err();
        assert_eq!(failed.error, Error::Unaligned);
        assert!(sim.take().is_empty(), "проверки — до обмена по шине");
        // Ровно до границы страницы — можно.
        let busy = failed.flash.program(0xF8, &[0; 8]).unwrap();
        let flash = busy.wait(10).unwrap();
        assert_eq!(sim.byte(0xF8), 0);
        assert_eq!(sim.byte(0x100), 0xFF);
        sim.take();
        let busy = flash.write_enable().unwrap().erase_sector(0).unwrap();
        assert_eq!(sim.take(), ["06", "05 → 02", "20 000000"]);
        let flash = busy.wait(10).unwrap();
        assert_eq!(sim.byte(0xF8), 0xFF);
        let flash = flash.write_enable().unwrap().disable().unwrap();
        sim.take();
        let _ = flash.write_enable().unwrap();
        assert_eq!(sim.take(), ["06", "05 → 02"]);
    }

    #[test]
    fn task_d_disable() {
        let (flash, sim, _) = ready();
        let flash = flash.write_enable().unwrap().disable().unwrap();
        assert_eq!(sim.take(), ["06", "05 → 02", "04"]);
        assert!(!sim.chip().wel);
        drop(flash);
    }

    #[test]
    fn task_d_timeout() {
        let (flash, sim, _) = ready();
        sim.chip().busy_polls = 7;
        let busy = flash.write_enable().unwrap().program(0, &[0]).unwrap();
        sim.take();
        let failed = busy.wait(5).unwrap_err();
        assert_eq!(failed.error, Error::Timeout);
        assert_eq!(sim.take().len(), 5);
        // Драйвер не потерян: можно ждать дальше.
        let _ready = failed.flash.wait(5).unwrap();
        assert_eq!(sim.take(), ["05 → 03", "05 → 03", "05 → 00"]);
    }

    #[test]
    fn task_d_sleep() {
        let (flash, sim, mut delay) = ready();
        let asleep = flash.sleep(&mut delay).unwrap();
        assert_eq!(sim.take(), ["b9", "пауза 3000 нс"]);
        assert!(sim.chip().asleep);
        let mut flash = asleep.wake(&mut delay).unwrap();
        assert_eq!(sim.take(), ["ab", "пауза 3000 нс"]);
        let mut b = [0];
        flash.read(0, &mut b).unwrap();
        assert_eq!(sim.take(), ["03 000000 → ff"]);
    }

    #[test]
    fn task_d_bus_errors() {
        let (flash, sim, mut delay) = ready();
        // 0x06 прошла, чтение состояния — нет: драйвер остаётся в Ready.
        let n = sim.chip().count;
        sim.chip().fail_at = Some(n + 1);
        let failed = flash.write_enable().unwrap_err();
        assert_eq!(failed.error, Error::Spi(SimError));
        let flash: Flash<Sim, Ready> = failed.flash;
        // WEL не установился: микросхема занята, и 0x06 она пропускает.
        {
            let mut c = sim.chip();
            c.wel = false;
            c.busy = 1;
        }
        sim.take();
        let failed = flash.write_enable().unwrap_err();
        assert_eq!(failed.error, Error::WriteNotEnabled);
        assert_eq!(sim.take(), ["06 (занята)", "05 → 01"]);
        let n = sim.chip().count;
        sim.chip().fail_at = Some(n);
        let failed = failed.flash.sleep(&mut delay).unwrap_err();
        assert_eq!(failed.error, Error::Spi(SimError));
        assert!(!sim.chip().asleep);
    }

    #[test]
    fn task_d_write_page() {
        let (flash, sim, _) = ready();
        let flash = write_page(flash, 0x2000, b"telemetry", 10).unwrap();
        assert_eq!(sim.take(), ["06", "05 → 02", "02 002000 +9", "05 → 03", "05 → 03", "05 → 00"]);
        let Err((any, e)) = write_page(flash, 0x20FF, b"xy", 10) else { panic!("пересечение страницы принято") };
        assert_eq!((any.state(), e), ("WriteEnabled", Error::PageOverflow));
        let AnyFlash::WriteEnabled(we) = any else { unreachable!() };
        let flash = we.disable().unwrap();
        sim.chip().busy_polls = 50;
        let Err((any, e)) = write_page(flash, 0x2100, b"z", 3) else { panic!("таймаут потерян") };
        assert_eq!((any.state(), e), ("Busy", Error::Timeout));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_d_types() {
        const USE: &str = "use embedded_hal::spi::SpiDevice;\nuse embedded_hal::delay::DelayNs;\nuse sut::d_flash::*;\n";
        let cases: [(&str, String, &[&str]); 8] = [
            (
                "d_ok",
                format!(
                    "{USE}pub fn f<S: SpiDevice>(f: Flash<S, Ready>) -> Flash<S, Busy> {{\n\
                     f.write_enable().ok().unwrap().program(0, &[1]).ok().unwrap() }}"
                ),
                &[],
            ),
            (
                "d_program_ready",
                format!("{USE}pub fn f<S: SpiDevice>(f: Flash<S, Ready>) {{ let _ = f.program(0, &[1]); }}"),
                &["E0599"],
            ),
            (
                "d_read_busy",
                format!("{USE}pub fn f<S: SpiDevice>(mut f: Flash<S, Busy>) {{ let _ = f.read(0, &mut [0]); }}"),
                &["E0599"],
            ),
            (
                "d_sleep_enabled",
                format!(
                    "{USE}pub fn f<S: SpiDevice, D: DelayNs>(f: Flash<S, WriteEnabled>, d: &mut D) {{ let _ = f.sleep(d); }}"
                ),
                &["E0599"],
            ),
            (
                "d_wake_ready",
                format!("{USE}pub fn f<S: SpiDevice, D: DelayNs>(f: Flash<S, Ready>, d: &mut D) {{ let _ = f.wake(d); }}"),
                &["E0599"],
            ),
            (
                "d_moved",
                format!(
                    "{USE}pub fn f<S: SpiDevice>(mut f: Flash<S, Ready>) {{\n\
                     let _we = f.write_enable();\n let _ = f.read(0, &mut [0]); }}"
                ),
                &["E0382"],
            ),
            (
                "d_own_state",
                format!("{USE}pub enum Mine {{}}\nimpl State for Mine {{}}"),
                &["E0277"],
            ),
            (
                "d_forge",
                format!(
                    "{USE}pub fn f<S: SpiDevice>(spi: S) -> Flash<S, Busy> {{\n\
                     Flash {{ spi, _state: core::marker::PhantomData }} }}"
                ),
                &["E0451"],
            ),
        ];
        let cases: Vec<(&str, &str, &[&str])> =
            cases.iter().map(|(n, s, w)| (*n, s.as_str(), *w)).collect();
        probes::check(&cases);
    }
}
