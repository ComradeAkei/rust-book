// Общие тесты задач главы 29.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch29/tests/ch29.rs` и `solutions/ch29/tests/ch29.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Модули задач B, C и D1 крейт держит в `modules/`; тесты собирают их
// системой сборки ядра (с CLIPPY=1 — линтами ядра) вместе с C-фрагментами
// задачи A, их переводами, эталоном задачи C и модулями записей из
// раздела 29.7 для задачи D2. Всё это вместе с программой гостя
// `ch29-task-guest` ложится в initramfs, и каждый сценарий — своя
// загрузка ядра в QEMU. Без собранного ядра (code/ch29/kernel/build.sh)
// такие тесты пропускаются; разбор трассы задачи D2 проверяется и без него.

use ch29::host::{Flavor, Initramfs, Kernel, Qemu, guest_binary};
use ch29::session::{guest_part, without_console};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn tmp(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-{name}", env!("CARGO_PKG_NAME")));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Что собралось для одного ядра.
struct Built {
    initrd: PathBuf,
    /// Журнал сборки `modules/` крейта: `Err` — не собралось.
    tasks: Result<String, String>,
    /// Журналы сборки переводов фрагментов A на Rust, 1–6.
    a_rust: Vec<Result<String, String>>,
}

/// Собирает каталог модулей и добавляет готовые `.ko` в список.
fn make(kernel: &Kernel, src: &Path, name: &str, kos: &mut Vec<PathBuf>) -> Result<String, String> {
    let out = tmp(&format!("{name}-{:?}", kernel.flavor).to_lowercase());
    let r = kernel.make_modules_with(src, &out, &["CLIPPY=1"]);
    if r.is_ok() {
        let mut found: Vec<PathBuf> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "ko"))
            .collect();
        found.sort();
        kos.extend(found);
    }
    r
}

fn build(kernel: &Kernel) -> &'static Built {
    static GUEST: OnceLock<PathBuf> = OnceLock::new();
    static PLAIN: OnceLock<Built> = OnceLock::new();
    static DEBUG: OnceLock<Built> = OnceLock::new();
    let guest = GUEST.get_or_init(|| {
        guest_binary(env!("CARGO_PKG_NAME"), "ch29-task-guest", &tmp("guest-target"))
    });
    let cell = if kernel.flavor == Flavor::Plain { &PLAIN } else { &DEBUG };
    cell.get_or_init(|| {
        let shared = manifest("../../tests-shared/ch29");
        let mut kos = Vec::new();
        let tasks = make(kernel, &manifest("modules"), "tasks", &mut kos);
        for (dir, name) in [(shared.join("a/c"), "a-c"), (shared.join("c-ref"), "c-ref")] {
            if let Err(log) = make(kernel, &dir, name, &mut kos) {
                panic!("{name} не собрался:\n{log}");
            }
        }
        if let Err(log) = make(kernel, &manifest("../../code/ch29/modules"), "chapter", &mut kos) {
            panic!("модули главы не собрались:\n{log}");
        }
        let a_rust = (1..=6)
            .map(|i| {
                make(kernel, &shared.join(format!("a/rust/a{i}")), &format!("a{i}-rs"), &mut kos)
            })
            .collect();
        // Образец ядра для задачи D1: samples/kobject/kobject-example.c.
        kos.push(kernel.build.join("samples/kobject/kobject-example.ko"));
        let mut img = Initramfs::new(guest);
        for ko in &kos {
            img = img.lib(ko);
        }
        let initrd =
            tmp(&format!("initramfs-{:?}", kernel.flavor).to_lowercase()).join("initramfs.cpio");
        img.write(kernel, &initrd).unwrap();
        Built { initrd, tasks, a_rust }
    })
}

/// Сеанс гостя без консоли ядра; `None` — ядра нет.
fn run(flavor: Flavor, args: &[&str]) -> Option<(String, &'static Built)> {
    let kernel = Kernel::or_skip(flavor)?;
    let built = build(&kernel);
    let serial = Qemu::new(&kernel, &built.initrd, args).run().unwrap();
    let body = guest_part(&serial).unwrap_or_else(|| panic!("гость не дошёл до конца:\n{serial}"));
    let body = without_console(body);
    println!("{body}");
    Some((body, built))
}

/// Модули крейта собрались, и clippy ядра не нашёл ничего в `file`.
fn assert_clean(built: &Built, file: &str) {
    let log = built.tasks.as_ref().unwrap_or_else(|log| panic!("modules/ не собрались:\n{log}"));
    let warned: Vec<&str> = log.lines().filter(|l| l.contains("-->") && l.contains(file)).collect();
    assert!(warned.is_empty(), "предупреждения clippy в {file}:\n{log}");
}

/// Строки сеанса между `### from` и следующей строкой `###`.
fn section<'s>(out: &'s str, from: &str) -> &'s str {
    let start =
        out.find(&format!("### {from}\n")).unwrap_or_else(|| panic!("нет раздела {from}:\n{out}"));
    let body = &out[start + from.len() + 5..];
    &body[..body.find("### ").unwrap_or(body.len())]
}

// ---------------------------------------------------------------- A

use sut::a_bugs::{Bug, InRust, verdicts};

// ANCHOR: a_reports
/// По каким строкам отчёта отладочное ядро узнаёт ошибку. Сон в
/// атомарном контексте ловят в двух местах: `might_sleep()` в начале
/// функций, которые могут спать, и проверка в самом планировщике.
fn report(bug: Bug) -> &'static [&'static str] {
    match bug {
        Bug::LeakOnError => &["unreferenced object"],
        Bug::UseAfterFree => &["BUG: KASAN: slab-use-after-free"],
        Bug::MissingUnlock => &["WARNING: lock held when returning to user space!"],
        Bug::SleepInAtomic => {
            &["BUG: sleeping function called from invalid context", "BUG: scheduling while atomic"]
        }
        Bug::OutOfBounds => &["BUG: KASAN: slab-out-of-bounds"],
        Bug::LockOrder => &["WARNING: possible circular locking dependency detected"],
    }
}

fn reported(out: &str, bug: Bug) -> bool {
    report(bug).iter().any(|r| out.contains(r))
}
// ANCHOR_END: a_reports

/// Сеанс задачи A для одного модуля: отчёты ядра и найденные утечки.
/// Утечку приписываем модулю, только если её выделил процесс, который
/// грузил модуль: kmemleak печатает pid выделившей задачи.
fn a_session(out: &str) -> String {
    let pid = out
        .split("# insmod ")
        .nth(1)
        .and_then(|l| l.split("pid ").nth(1))
        .and_then(|l| l.split(':').next())
        .unwrap_or_else(|| panic!("нет pid загрузчика:\n{out}"));
    let comm = format!("pid {pid},");
    // Отчёт kmemleak: «unreferenced object …», ниже «comm "init", pid N, …».
    let mut kept = String::new();
    let mut lines = out.lines().peekable();
    while let Some(l) = lines.next() {
        if l.starts_with("unreferenced object") {
            let owner = lines.peek().is_some_and(|n| n.contains(&comm));
            if !owner {
                continue;
            }
        }
        kept.push_str(l);
        kept.push('\n');
    }
    kept
}

// ANCHOR: a
#[test]
fn task_a_fragments_on_debug_kernel() {
    let answers = verdicts();
    let mut observed = BTreeMap::new();
    for (i, v) in answers.iter().enumerate() {
        let n = i + 1;
        let Some((c, built)) = run(Flavor::Debug, &["a", &format!("ch29_a{n}")]) else { return };
        let c = a_session(&c);
        assert!(reported(&c, v.bug), "фрагмент {n}: в отчёте C нет {:?}:\n{c}", report(v.bug));
        let rust = match &built.a_rust[i] {
            Err(log) if log.contains("error[E") => InRust::CompileError,
            Err(log) => panic!("перевод {n} не собрался не по своей ошибке:\n{log}"),
            Ok(_) => {
                let (out, _) = run(Flavor::Debug, &["a", &format!("ch29_a{n}_rs")]).unwrap();
                let out = a_session(&out);
                if out.contains("rust_kernel: panicked at") {
                    InRust::Panic
                } else if reported(&out, v.bug) {
                    InRust::Remains
                } else {
                    InRust::Gone
                }
            }
        };
        observed.insert(n, (v.bug, rust));
    }
    for (n, (bug, rust)) in &observed {
        println!("фрагмент {n}: {bug:?}, в Rust: {rust:?}");
    }
    for (i, v) in answers.iter().enumerate() {
        assert_eq!(v.rust, observed[&(i + 1)].1, "фрагмент {}: {:?}", i + 1, v.bug);
    }
}
// ANCHOR_END: a

// ---------------------------------------------------------------- B

// ANCHOR: b
#[test]
fn task_b_module_with_parameter() {
    let Some((out, built)) = run(Flavor::Plain, &["b"]) else { return };
    assert_clean(built, "ch29_b.rs");
    for (params, line) in [
        ("", "info:   ch29_b: fib(10) = 55\n"),
        (" n=0", "info:   ch29_b: fib(0) = 0\n"),
        (" n=1", "info:   ch29_b: fib(1) = 1\n"),
        (" n=2", "info:   ch29_b: fib(2) = 1\n"),
        (" n=93", "info:   ch29_b: fib(93) = 12200160415121876738\n"),
    ] {
        let part = out
            .split(&format!("# insmod /lib/ch29_b.ko{params}\n"))
            .nth(1)
            .unwrap_or_else(|| panic!("{out}"));
        assert!(part.starts_with(line), "insmod{params}: ждём {line:?}:\n{part}");
    }
    let part = out.split("# insmod /lib/ch29_b.ko n=94\n").nth(1).unwrap();
    assert!(
        part.starts_with("insmod: Value too large for defined data type (os error 75)\n"),
        "{part}"
    );
    assert!(part.contains("err:    ch29_b: "), "отказ объясняется в журнале: {part}");
    // Модуль с ошибкой в init не остаётся загруженным.
    assert!(out.ends_with("# grep ch29_b /proc/modules\n"), "{out}");
}
// ANCHOR_END: b

// ---------------------------------------------------------------- C

/// Строки сеанса счётчика без имён модуля и устройства.
fn counter_lines(out: &str, module: &str) -> String {
    section(out, module)
        .replace("/dev/ch29-counter-c", "/dev/X")
        .replace("/dev/ch29-counter", "/dev/X")
        .replace(module, "X")
}

/// Строки кода без пустых и комментариев. Блочный комментарий
/// отслеживается до `*/`: строка кода тоже может начинаться с `*`
/// (`*counter = …`), и по первому символу её от продолжения комментария
/// не отличить.
fn code<'s>(text: &'s str, comment: &str) -> Vec<&'s str> {
    let mut in_block = false;
    text.lines()
        .map(str::trim)
        .filter(|l| {
            if in_block {
                in_block = !l.contains("*/");
                return false;
            }
            if l.starts_with("/*") {
                in_block = !l.contains("*/");
                return false;
            }
            !l.is_empty() && !l.starts_with(comment)
        })
        .collect()
}

// ANCHOR: c
#[test]
fn task_c_counter_matches_c_reference() {
    let Some((out, built)) = run(Flavor::Plain, &["c"]) else { return };
    assert_clean(built, "ch29_c.rs");
    let c = counter_lines(&out, "ch29_counter_c");
    // Эталон ведёт себя так, как сказано в условии.
    for line in [
        "read: \"0\\n\"",
        "write \"5\": Ok(1)",
        "write \"+7\\n\": Ok(3)",
        "write \"abc\": Err(EINVAL)",
        "write \"-1\": Err(EINVAL)",
        "write \"000000000000000000000001\": Err(EINVAL)",
        "write \"00000000000000000000001\": Ok(23)",
        "write \"18446744073709551616\": Err(ERANGE)",
        "write \"18446744073709551615\": Err(EOVERFLOW)",
        "read: \"21\\n\"",
        "a: read 1 байт: Ok(1) '2'",
        "a: дочитать: Ok(2) \"1\\n\"",
        "read: \"121\\n\"",
        "a: lseek 0: Err(ESPIPE)",
        "a: pread с 0: Ok(4) \"121\\n\"",
        "4 процесса по 500 записей \"1\": коды 0 0 0 0",
        "read: \"2121\\n\"",
    ] {
        assert!(c.contains(&format!("{line}\n")), "эталон: нет {line:?}:\n{c}");
    }
    let rust = counter_lines(&out, "ch29_c");
    assert_eq!(rust, c, "счётчик на Rust ведёт себя не так, как эталон на C");

    let rs = std::fs::read_to_string(manifest("modules/ch29_c.rs")).unwrap();
    let cs = std::fs::read_to_string(manifest("../../tests-shared/ch29/c-ref/ch29_counter_c.c"))
        .unwrap();
    let (rs_code, cs_code) = (code(&rs, "//"), code(&cs, "//"));
    // Строки, где функция может вернуть ошибку: `?` и `Err(` в Rust,
    // `return -E…`, `return ret` и `goto` в C.
    let rs_err = rs_code.iter().filter(|l| l.contains('?') || l.contains("Err(")).count();
    let cs_err = cs_code
        .iter()
        .filter(|l| l.contains("return -E") || l.contains("return ret") || l.contains("goto "))
        .count();
    println!("строк кода: Rust {}, C {}", rs_code.len(), cs_code.len());
    println!("строк с выходом по ошибке: Rust {rs_err}, C {cs_err}");
}
// ANCHOR_END: c

// ---------------------------------------------------------------- D1

// ANCHOR: d1
#[test]
fn task_d1_kobject_matches_sample() {
    let Some((out, built)) = run(Flavor::Plain, &["d1"]) else { return };
    assert_clean(built, "ch29_d1.rs");
    let c = section(&out, "kobject-example");
    for line in [
        "ls: bar baz foo",
        "foo: права 664, значение \"0\\n\"",
        "echo \"42\" > foo: Ok(2)",
        "echo \"x\" > foo: Err(EINVAL)",
        "echo \"99999999999\" > bar: Err(ERANGE)",
        "foo: \"42\\n\"",
        "echo \"0x10\" > baz: Err(EINVAL)",
        "baz: \"7\\n\"",
        "bar: \"-3\\n\"",
        "# ls -d /sys/kernel/kobject_example: нет",
    ] {
        assert!(c.contains(&format!("{line}\n")), "образец: нет {line:?}:\n{c}");
    }
    let rust = section(&out, "ch29_d1");
    let normalize = |s: &str| {
        s.replace("/lib/ch29_d1.ko", "/lib/X.ko")
            .replace("/lib/kobject-example.ko", "/lib/X.ko")
            .replace("rmmod ch29_d1", "rmmod X")
            .replace("rmmod kobject_example", "rmmod X")
    };
    assert_eq!(normalize(rust), normalize(c), "модуль на Rust ведёт себя не так, как образец");
}
// ANCHOR_END: d1

// ---------------------------------------------------------------- D2

use sut::d2_trace::{durations, instructions, median};

#[test]
fn task_d2_trace_parser() {
    let trace = "\
# tracer: function_graph
#
 1)               |  vfs_read() {
 1)   0.311 us    |    rw_verify_area();
 1)               |    new_sync_read() {
 1)   1.020 us    |      _copy_to_iter();
 1)   3.500 us    |    } /* new_sync_read */
 1) + 12.904 us   |  } /* vfs_read */
 0)   0.250 us    |  } /* vfs_read */
";
    let c = sut::d2_trace::calls(trace);
    let names: Vec<(&str, usize)> = c.iter().map(|c| (c.function.as_str(), c.depth)).collect();
    assert_eq!(
        names,
        [
            ("rw_verify_area", 1),
            ("_copy_to_iter", 2),
            ("new_sync_read", 1),
            ("vfs_read", 0),
            ("vfs_read", 0)
        ]
    );
    assert_eq!(durations(trace, "vfs_read"), [12.904, 0.25]);
    assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
    assert_eq!(median(&[4.0, 1.0, 2.0, 3.0]), Some(2.5));
    assert_eq!(median(&[]), None);
}

#[test]
fn task_d2_objdump_parser() {
    let dump = "\
0000000000000000 <notes_read>:
   0:      \tnopl\t(%rax,%rax)
\t\t0000000000000001:  R_X86_64_NONE\t__fentry__-0x4
   5:      \tpushq\t%r15
  1c:      \tcallq\t0x21 <notes_read+0x21>
\t\t000000000000001d:  R_X86_64_PLT32\tmutex_lock-0x4
  4e:      \tretq

0000000000000050 <notes_write>:
  50:      \tretq
";
    assert_eq!(instructions(dump, "notes_read"), Some(4));
    assert_eq!(instructions(dump, "notes_write"), Some(1));
    assert_eq!(instructions(dump, "notes"), None);
    assert_eq!(sut::d2_trace::callees(dump, "notes_read"), ["mutex_lock"]);
}

// ANCHOR: d2
#[test]
fn task_d2_read_path() {
    let calls = 200;
    let Some((out, _)) = run(Flavor::Plain, &["d2", &calls.to_string()]) else { return };
    let mut rows = Vec::new();
    for module in ["ch29_notes", "ch29_notes_c"] {
        let trace = section(&out, &format!("трасса {module}"));
        let vfs = durations(trace, "vfs_read");
        assert_eq!(vfs.len(), calls, "{module}: вызовов vfs_read в трассе:\n{trace}");
        let pread = out.split(&format!("{module}: pread 16 байт: ")).nth(1).unwrap();
        let ns: u64 = pread.split(' ').next().unwrap().parse().unwrap();
        rows.push((module, ns, median(&vfs).unwrap()));
    }
    for (module, ns, vfs) in &rows {
        println!("{module:14} pread {ns:6} нс, vfs_read по трассе {vfs:7.3} мкс (медиана)");
    }

    // Ассемблер: сколько инструкций в обработчике read и в переходнике.
    let dir = tmp("chapter-plain");
    let dump = |ko: &str| {
        let o = std::process::Command::new("llvm-objdump")
            .args(["-d", "-r", "-C", "--no-show-raw-insn"])
            .arg(dir.join(ko))
            .output()
            .expect("llvm-objdump");
        String::from_utf8(o.stdout).unwrap()
    };
    let (rs, c) = (dump("ch29_notes.ko"), dump("ch29_notes_c.ko"));
    let c_read = instructions(&c, "notes_read").expect("notes_read");
    let shim =
        instructions(&rs, "<kernel::miscdevice::MiscdeviceVTable<ch29_notes::Notes>>::read_iter")
            .expect("переходник read_iter");
    let body =
        instructions(&rs, "<ch29_notes::Notes as kernel::miscdevice::MiscDevice>::read_iter")
            .expect("Notes::read_iter");
    println!("инструкций: C notes_read {c_read}; Rust: переходник {shim} + read_iter {body}");
    assert!(c_read > 0 && shim > 0 && body > 0);
}
// ANCHOR_END: d2
