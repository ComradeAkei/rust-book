// Общие тесты задач главы 4.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch04/tests/ch04.rs` и `solutions/ch04/tests/ch04.rs`, заранее
// объявив `use <крейт> as sut;`. Так заготовки и решения проверяются одним
// и тем же набором тестов.
//
// Задачи A, B и D запускают компиляторы: rustc версии из
// rust-toolchain.toml, nightly для Polonius, clang или arm-none-eabi-gcc и
// cc для C. Под Miri эти тесты пропускаются. Тестам с `polonius` и
// `task_b` в имени нужен rustc из rust-toolchain.toml, поэтому в задании CI,
// где установлен только nightly, их пропускают.

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

/// rustc закреплённой версии (`None`) или тулчейна rustup. Переменную
/// `RUSTUP_TOOLCHAIN` убираем: под `cargo +nightly` она подменила бы версию
/// из rust-toolchain.toml.
fn rustc(toolchain: Option<&str>) -> Command {
    let mut cmd = match toolchain {
        Some(t) => {
            let mut c = Command::new("rustup");
            c.args(["run", t, "rustc"]);
            c
        }
        None => Command::new("rustc"),
    };
    cmd.env_remove("RUSTUP_TOOLCHAIN").current_dir(crate_dir());
    cmd
}

/// Собрать `file` как библиотеку и вернуть коды ошибок `E…` по порядку
/// появления. `None`, если такого тулчейна нет.
fn error_codes(
    toolchain: Option<&str>,
    file: &Path,
    out_dir: &Path,
    extra: &[&str],
) -> Option<Vec<String>> {
    let stem = file.file_stem().unwrap().to_str().unwrap();
    let out = rustc(toolchain)
        .args(["--edition", "2024", "--crate-type", "lib", "--emit", "metadata", "-o"])
        .arg(out_dir.join(format!("{stem}.rmeta")))
        .args(extra)
        .arg(file)
        .output()
        .expect("не удалось запустить rustc");
    if toolchain.is_some() && !out.status.success() && out.stderr.starts_with(b"error: toolchain") {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stderr);
    let mut codes: Vec<String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("error[").and_then(|r| r.get(..5)).map(str::to_owned))
        .collect();
    codes.dedup();
    assert_eq!(out.status.success(), codes.is_empty(), "{}:\n{text}", file.display());
    Some(codes)
}

mod task_a {
    use super::sut::a_borrowck::PREDICTIONS;
    use super::{crate_dir, error_codes, tmp};

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn predictions_match() {
        let dir = crate_dir().join("../../code/ch04/fragments");
        let out = tmp("a");
        let mut wrong = Vec::new();
        for (i, guess) in PREDICTIONS.iter().enumerate() {
            let name = format!("a{:02}.rs", i + 1);
            let codes = error_codes(None, &dir.join(&name), &out, &[]).unwrap();
            let actual = codes.first().map_or("ok", String::as_str);
            if guess.is_empty() {
                wrong.push(format!("{name}: ответ не заполнен"));
            } else if *guess != actual {
                wrong.push(format!("{name}: ваш ответ {guess}, на самом деле {actual}"));
            }
        }
        assert!(wrong.is_empty(), "\n{}\n", wrong.join("\n"));
    }
}

mod task_b {
    use super::sut::b_add_twice::{ALIASED, LDR, STR, ch04b_add_twice};
    use super::{crate_dir, rustc, tmp};
    use std::path::Path;
    use std::process::Command;

    #[test]
    fn adds_twice() {
        let mut a = 1;
        ch04b_add_twice(&mut a, &2);
        assert_eq!(a, 5);
    }

    /// Инструкции функции из ассемблера rustc, clang или GCC: строки от
    /// метки до `.Lfunc_end` или `.size`, без директив, меток и комментариев.
    fn body<'a>(asm: &'a str, name: &str) -> Vec<&'a str> {
        let head = format!("{name}:");
        let mut lines = asm.lines().map(str::trim);
        assert!(lines.any(|l| l == head), "{name} нет в ассемблере:\n{asm}");
        lines
            .take_while(|l| !l.starts_with(".Lfunc_end") && !l.starts_with(".size"))
            .filter(|l| !l.is_empty() && !l.starts_with(['.', '@']) && !l.ends_with(':'))
            .collect()
    }

    /// Сколько раз встречается мнемоника, с суффиксами ширины `.w`/`.n` и
    /// без них.
    fn count(body: &[&str], insn: &str) -> usize {
        body.iter()
            .filter_map(|l| l.split_whitespace().next())
            .filter(|m| m.trim_end_matches(".w").trim_end_matches(".n") == insn)
            .count()
    }

    fn check(what: &str, i: usize, body: &[&str]) {
        assert!(LDR[i] > 0 && STR[i] > 0, "{what}: ответ не заполнен");
        assert_eq!(
            (LDR[i], STR[i]),
            (count(body, "ldr"), count(body, "str")),
            "{what} на thumbv7m: (ldr, str) в ответе и на самом деле\n{}",
            body.join("\n")
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn task_b_rust_loads() {
        let dir = tmp("b");
        let root = dir.join("root.rs");
        let src = crate_dir().join("src/b_add_twice.rs");
        std::fs::write(&root, format!("#![no_std]\n#[path = {src:?}]\npub mod b;\n")).unwrap();
        let asm = dir.join("b.s");
        let out = rustc(None)
            .args(["--edition", "2024", "--crate-type", "lib", "-O"])
            .args(["--target", "thumbv7m-none-eabi", "--emit"])
            .arg(format!("asm={}", asm.display()))
            .arg(&root)
            .output()
            .expect("не удалось запустить rustc");
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let asm = std::fs::read_to_string(asm).unwrap();
        check("Rust", 0, &body(&asm, "ch04b_add_twice"));
    }

    /// Ассемблер C-файла для Cortex-M3: clang, а если его нет,
    /// arm-none-eabi-gcc. `None`, если нет ни того, ни другого.
    fn c_for_thumb(src: &Path, out: &Path) -> Option<String> {
        let compilers: [(&str, &[&str]); 2] = [
            ("clang", &["--target=thumbv7m-none-eabi", "-mcpu=cortex-m3"]),
            ("arm-none-eabi-gcc", &["-mcpu=cortex-m3", "-mthumb"]),
        ];
        for (cc, flags) in compilers {
            let Ok(o) = Command::new(cc).args(flags).args(["-O2", "-S", "-o"]).arg(out).arg(src).output()
            else {
                continue;
            };
            assert!(o.status.success(), "{cc}:\n{}", String::from_utf8_lossy(&o.stderr));
            return Some(std::fs::read_to_string(out).unwrap());
        }
        None
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает компилятор C")]
    fn task_b_c_loads() {
        let src = crate_dir().join("c/add_twice.c");
        let Some(asm) = c_for_thumb(&src, &tmp("b").join("c.s")) else {
            eprintln!("нет ни clang, ни arm-none-eabi-gcc: проверка C пропущена");
            return;
        };
        check("C без restrict", 1, &body(&asm, "add_twice"));
        check("C с restrict", 2, &body(&asm, "add_twice_restrict"));
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает компилятор C")]
    fn task_b_c_aliased() {
        let dir = tmp("b");
        let main = dir.join("main.c");
        std::fs::write(
            &main,
            "#include <stdio.h>\n\
             void add_twice(int *a, const int *b);\n\
             void add_twice_restrict(int *a, const int *b);\n\
             int main(void)\n{\n    int x = 1, y = 1;\n    add_twice(&x, &x);\n    \
             add_twice_restrict(&y, &y);\n    printf(\"%d %d\\n\", x, y);\n    return 0;\n}\n",
        )
        .unwrap();
        let exe = dir.join("aliased");
        let cc = std::env::var("CC").unwrap_or_else(|_| "cc".into());
        let o = Command::new(&cc)
            .args(["-O2", "-o"])
            .arg(&exe)
            .arg(crate_dir().join("c/add_twice.c"))
            .arg(&main)
            .output()
            .unwrap_or_else(|e| panic!("не удалось запустить {cc}: {e}"));
        assert!(o.status.success(), "{cc}:\n{}", String::from_utf8_lossy(&o.stderr));
        let run = Command::new(&exe).output().unwrap();
        let got: Vec<i32> = String::from_utf8(run.stdout)
            .unwrap()
            .split_whitespace()
            .map(|s| s.parse().unwrap())
            .collect();
        assert_eq!(ALIASED[..], got[..], "x после add_twice(&x, &x): ответ и на самом деле");
    }
}

mod task_c {
    use super::sut::c_build_graph::{Graph, TaskId};

    fn graph(rules: &[(&str, &[&str])]) -> Graph {
        let mut g = Graph::new();
        for (task, deps) in rules {
            let t = g.task(task);
            for d in *deps {
                let d = g.task(d);
                g.depends(t, d);
            }
        }
        g
    }

    /// Каждая задача в порядке ровно один раз и после всех своих
    /// зависимостей.
    fn check_order(g: &Graph, order: &[TaskId]) {
        assert_eq!(order.len(), g.len(), "в порядке не все задачи");
        let mut pos = vec![usize::MAX; g.len()];
        for (i, t) in order.iter().enumerate() {
            assert_eq!(pos[t.0 as usize], usize::MAX, "{} в порядке дважды", g.name(*t));
            pos[t.0 as usize] = i;
        }
        for &t in order {
            for &d in g.deps(t) {
                assert!(pos[d.0 as usize] < pos[t.0 as usize], "{} раньше {}", g.name(t), g.name(d));
            }
        }
    }

    /// Каждая задача цикла зависит от следующей, последняя — от первой,
    /// повторов нет.
    fn check_cycle(g: &Graph, cycle: &[TaskId]) {
        assert!(!cycle.is_empty(), "пустой цикл");
        for (i, &t) in cycle.iter().enumerate() {
            let next = cycle[(i + 1) % cycle.len()];
            assert!(g.deps(t).contains(&next), "{} не зависит от {}", g.name(t), g.name(next));
        }
        let mut uniq = cycle.to_vec();
        uniq.sort();
        uniq.dedup();
        assert_eq!(uniq.len(), cycle.len(), "задачи в цикле повторяются");
    }

    #[test]
    fn firmware_build() {
        let mut g = graph(&[
            ("firmware.elf", &["main.o", "hal.o", "link.x"]),
            ("main.o", &["main.c", "hal.h"]),
            ("hal.o", &["hal.c", "hal.h"]),
            ("hal.h", &["regs.h"]),
        ]);
        assert_eq!(g.len(), 8);
        let hal = g.task("hal.h");
        assert_eq!(g.task("hal.h"), hal, "повторный task вернул другой номер");
        assert_eq!(g.len(), 8);
        assert_eq!(g.name(hal), "hal.h");
        let order = g.build_order().expect("циклов нет");
        check_order(&g, &order);
        assert_eq!(g.name(*order.last().unwrap()), "firmware.elf");
    }

    #[test]
    fn finds_cycle() {
        // gen.h пишет программа gen, которая сама включает gen.h.
        let g = graph(&[
            ("app", &["main.o"]),
            ("main.o", &["main.c", "gen.h"]),
            ("gen.h", &["gen"]),
            ("gen", &["gen.o"]),
            ("gen.o", &["gen.c", "gen.h"]),
        ]);
        let cycle = g.build_order().expect_err("цикл gen.h → gen → gen.o → gen.h");
        check_cycle(&g, &cycle);
        let mut names: Vec<&str> = cycle.iter().map(|&t| g.name(t)).collect();
        names.sort();
        assert_eq!(names, ["gen", "gen.h", "gen.o"]);
    }

    #[test]
    fn self_dependency_and_empty() {
        let g = graph(&[("a", &["a"])]);
        let a = TaskId(0);
        assert_eq!(g.build_order(), Err(vec![a]));
        assert_eq!(Graph::new().build_order(), Ok(vec![]));
    }

    const BIG: u32 = if cfg!(miri) { 1_000 } else { 100_000 };

    /// Цепочка из 100 000 задач. Рекурсивный обход переполнил бы стек
    /// тестового потока (2 МиБ).
    #[test]
    fn long_chain() {
        let mut g = Graph::new();
        let ids: Vec<TaskId> = (0..BIG).map(|i| g.task(&format!("t{i}"))).collect();
        for w in ids.windows(2) {
            g.depends(w[0], w[1]);
        }
        let order = g.build_order().expect("циклов нет");
        check_order(&g, &order);
        assert_eq!(order[0], *ids.last().unwrap());
        g.depends(*ids.last().unwrap(), ids[0]);
        let cycle = g.build_order().expect_err("цепочка замкнута");
        assert_eq!(cycle.len(), BIG as usize);
        check_cycle(&g, &cycle);
    }

    /// Только арена и номера: ни небезопасного кода, ни счётчиков ссылок.
    /// Комментарии не проверяются.
    #[test]
    fn arena_only() {
        let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/c_build_graph.rs"));
        for (n, line) in src.lines().enumerate() {
            let code = line.split("//").next().unwrap();
            for word in ["unsafe", "Rc<", "Rc::", "RefCell", "Weak<", "*const ", "*mut "] {
                assert!(!code.contains(word), "строка {}: {word}\n{line}", n + 1);
            }
        }
    }
}

mod task_d {
    use super::sut::d_polonius::{Node, Slot, acquire, push_back};
    use super::{crate_dir, error_codes, tmp};

    #[test]
    fn acquire_reuses_free_slot() {
        let mut pool: Vec<Slot> = (0..3).map(|i| Slot { busy: i != 1, data: [i; 16] }).collect();
        let s = acquire(&mut pool);
        assert!(s.busy);
        assert_eq!(s.data[0], 1, "свободен был слот 1");
        assert_eq!(pool.len(), 3);
        let s = acquire(&mut pool);
        assert!(s.busy);
        s.data[0] = 9;
        assert_eq!(pool.len(), 4, "свободных не было: нужен новый слот");
        assert_eq!(pool[3].data[0], 9);
        assert!(pool[3].busy);
    }

    #[test]
    fn push_back_appends() {
        let mut head = Node { val: 1, next: None };
        push_back(&mut head, 2);
        push_back(&mut head, 3);
        let mut vals = Vec::new();
        let mut cur = Some(&head);
        while let Some(n) = cur {
            vals.push(n.val);
            cur = n.next.as_deref();
        }
        assert_eq!(vals, [1, 2, 3]);
    }

    /// Коды ошибок для `polonius/<name>.rs`, собранного вместе с
    /// определениями `Slot` и `Node`.
    fn natural(name: &str, toolchain: Option<&str>, extra: &[&str]) -> Option<Vec<String>> {
        let dir = tmp("d");
        let root = dir.join(format!("{name}.rs"));
        let src = crate_dir().join(format!("polonius/{name}.rs"));
        std::fs::write(
            &root,
            format!(
                "pub struct Slot {{ pub busy: bool, pub data: [u8; 16] }}\n\
                 pub struct Node {{ pub val: u32, pub next: Option<Box<Node>> }}\n\
                 #[path = {src:?}]\npub mod natural;\n"
            ),
        )
        .unwrap();
        error_codes(toolchain, &root, &dir, extra)
    }

    const BORROW_ERRORS: [&str; 5] = ["E0499", "E0502", "E0503", "E0505", "E0506"];

    fn stable_rejects(name: &str) {
        let codes = natural(name, None, &[]).unwrap();
        assert!(
            !codes.is_empty() && codes.iter().all(|c| BORROW_ERRORS.contains(&c.as_str())),
            "rustc из rust-toolchain.toml должен отвергнуть polonius/{name}.rs только \
             ошибками заимствования, а выдал {codes:?}"
        );
    }

    fn polonius(name: &str) -> Option<Vec<String>> {
        let codes = natural(name, Some("nightly"), &["-Zpolonius=next"]);
        if codes.is_none() {
            eprintln!("nightly не установлен: проверка Polonius пропущена");
        }
        codes
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn polonius_accepts_acquire() {
        stable_rejects("acquire");
        if let Some(codes) = polonius("acquire") {
            assert!(codes.is_empty(), "Polonius должен принять polonius/acquire.rs: {codes:?}");
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "запускает rustc")]
    fn polonius_rejects_push_back() {
        stable_rejects("push_back");
        if let Some(codes) = polonius("push_back") {
            assert!(!codes.is_empty(), "Polonius принял polonius/push_back.rs");
        }
    }
}
