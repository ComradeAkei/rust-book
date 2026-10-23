//! Введение и разделы 15.3, 15.6 и 15.7: примеры, которые глава
//! запускает. Тесты собирают их закреплённым cargo в release и сверяют
//! вывод с текстом.
//!
//! - `registry`: сценарии `table.c` на `HashMap`; число сравнений ключей с
//!   хешем K&R — следствие одинаковых хешей и детерминировано, с SipHash
//!   — случайно, но мало.
//! - `registry order`: порядок обхода разный у двух таблиц и от запуска к
//!   запуску — наблюдение, тест назван `cargo_observed_`.
//! - `swiss`: управляющие байты таблицы с FxHash — раскладка hashbrown,
//!   наблюдение.
//! - `trace`: обращения к распределителю, наблюдение.
//! - `growth` под strace: brk, переход на mmap с 128 КиБ, mremap и
//!   подвижный порог glibc. Без strace тест пропускается.
//!
//! В задании CI со случайной раскладкой тесты пропускаются по префиксу
//! `cargo_`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn target() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch15-run")
}

fn cargo(args: &[&str]) -> Output {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    Command::new("cargo")
        .current_dir(dir)
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .env_remove("RUST_BACKTRACE")
        .args(args)
        .arg("--target-dir")
        .arg(target())
        .output()
        .unwrap()
}

fn run(example: &str, args: &[&str]) -> String {
    let mut a = vec!["run", "-q", "--release", "--example", example, "--"];
    a.extend_from_slice(args);
    let o = cargo(&a);
    let text = String::from_utf8_lossy(&o.stdout).into_owned();
    assert!(o.status.success(), "{text}{}", String::from_utf8_lossy(&o.stderr));
    text
}

/// Число перед словом `сравнений` в строке, которая начинается с `prefix`.
fn comparisons(out: &str, prefix: &str) -> f64 {
    let line = out.lines().find(|l| l.starts_with(prefix)).unwrap_or_else(|| panic!("{out}"));
    let before = line.split(" сравнений").next().unwrap();
    before.rsplit(' ').next().unwrap().parse().unwrap()
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_registry() {
    let out = run("registry", &[]);
    assert!(out.starts_with("adc0: измерений 1 (ожидалось 1)\nBB: найден\n"), "{out}");
    assert_eq!(comparisons(&out, "K&R    , имена из Aa/BB: 8192 штук,"), 4095.5);
    assert_eq!(comparisons(&out, "K&R    , обычные имена:  8192 штук,"), 13.1);
    assert!(comparisons(&out, "SipHash, имена из Aa/BB: 8192 штук,") < 1.0, "{out}");
    assert!(comparisons(&out, "SipHash, обычные имена:  8192 штук,") < 1.0, "{out}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_observed_hash_order() {
    let runs: Vec<String> = (0..3).map(|_| run("registry", &["order"])).collect();
    let orders: Vec<&str> = runs.iter().flat_map(|r| r.lines()).collect();
    assert_eq!(orders.len(), 6);
    for o in &orders {
        let mut keys: Vec<u32> =
            o.trim_matches(['[', ']']).split(", ").map(|k| k.parse().unwrap()).collect();
        keys.sort_unstable();
        assert!(keys.iter().copied().eq(0..16), "{o}");
    }
    // Шесть порядков из 16! возможных: совпасть все они могут лишь случайно.
    assert!(orders.iter().any(|o| *o != orders[0]), "{orders:?}");
    assert!(runs.iter().any(|r| r.lines().next() != r.lines().nth(1)), "{runs:?}");
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_observed_swiss() {
    let out = run("swiss", &[]);
    let want = "\
HashMap<u64, u64>, 15 ключей: ёмкость 28, корзин 32, блок 560 байт = 32 × 16 + 32 + 16
управление: 00 2d 5b ff ff 45 72 ff ff 5c ff 0a ff 74 ff 21 ff ff ff 39 67 ff ff 50 7e ff ff 68 ff 16 ff ff
хвост:      00 2d 5b ff ff 45 72 ff ff 5c ff 0a ff 74 ff 21 (копия первых 16)
ключ  300: h1 =  9, h2 = 5c, байт 5c в позиции 9
ключ  700: h1 =  1, h2 = 2d, байт 2d в позиции 1
ключ 1100: h1 = 24, h2 = 7e, байт 7e в позиции 24

20 имён из Aa/BB, хеш K&R: корзин 32, у всех h1 = 31, h2 = 00
управление: 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff ff ff ff ff ff ff ff ff 00
без BBAaBBAaAa: 00 00 00 00 80 00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff ff ff ff ff ff ff ff ff 00
удалено помечено 80, остальные 19 находятся: true
";
    assert_eq!(out, want);
}

#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn cargo_observed_trace() {
    let out = run("trace", &[]);
    let want = "\
vec![1u32, 2, 3]                 alloc 12/4
vec![0u64; 1000]                 alloc_zeroed 8000/8
Vec<u32>, 9 раз push             alloc 16/4, realloc 16→32/4, realloc 32→64/4
String::from(\"датчик\")           alloc 12/1
format!(\"{}={}\", s, v)           alloc 8/1, realloc 8→16/1
Vec<()>, 1000 раз push           ничего
HashMap::new()                   ничего
HashMap<u32, u32>, 1 insert      alloc 52/16
BTreeMap<u32, u32>, 12 insert    alloc 96/4, alloc 104/8, alloc 200/8, alloc 104/8, dealloc 96/4
into_boxed_slice при len < cap   alloc 100/1, realloc 100→4/1
drop(Vec<u32> на 16)             dealloc 64/4
";
    assert_eq!(out, want);
}

/// Системные вызовы памяти и строки вывода примера `growth`: strace пишет
/// `write(1, "…")` с восьмеричными escape-последовательностями, поэтому
/// строки вывода берутся из самого примера, а из strace — только порядок.
fn traced_growth(arg: &str) -> Option<Vec<String>> {
    if Command::new("strace").arg("-V").output().is_err() {
        eprintln!("strace не найден, тест пропущен");
        return None;
    }
    let o = cargo(&["build", "-q", "--release", "--example", "growth"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let exe = target().join("release/examples/growth");
    let log = target().join(format!("growth-{arg}.strace"));
    let o = Command::new("strace")
        .args(["-qq", "-e", "trace=brk,mmap,mremap,munmap,write", "-o"])
        .arg(&log)
        .arg(&exe)
        .arg(arg)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let mut out = String::from_utf8(o.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect::<Vec<_>>()
        .into_iter();
    let trace = std::fs::read_to_string(log).unwrap();
    // Вызовы до первой строки вывода — запуск процесса.
    let mut seen = Vec::new();
    let mut started = false;
    for l in trace.lines() {
        if l.starts_with("write(1,") {
            started = true;
            seen.push(out.next().unwrap());
        } else if started {
            seen.push(l.split(" = ").next().unwrap().trim_end().to_string());
        }
    }
    Some(seen)
}

#[test]
#[cfg_attr(miri, ignore = "запускает strace")]
fn cargo_observed_growth_strace() {
    let Some(s) = traced_growth("once") else { return };
    let at =
        |cap: &str| s.iter().position(|l| l.contains(cap) && l.starts_with("ёмкость")).unwrap();
    // До 128 КиБ буфер растёт на вершине кучи без системных вызовов.
    assert!(s[at("       8:")..at("  131072:")].iter().all(|l| l.starts_with("ёмкость")), "{s:#?}");
    assert!(s[at("      16:")..=at("  131072:")].iter().all(|l| l.ends_with("на месте")), "{s:#?}");
    // 256 КиБ — отдельное отображение, дальше mremap.
    assert_eq!(
        s[at("  262144:") - 1],
        "mmap(NULL, 266240, PROT_READ|PROT_WRITE, MAP_PRIVATE|MAP_ANONYMOUS, -1, 0)"
    );
    for (cap, old, new) in [("  524288:", 266240, 528384), (" 4194304:", 2101248, 4198400)] {
        let l = &s[at(cap) - 1];
        assert!(
            l.starts_with("mremap(") && l.ends_with(&format!(", {old}, {new}, MREMAP_MAYMOVE)")),
            "{l}"
        );
    }
    assert_eq!(s.iter().find(|l| l.starts_with("переездов")).unwrap(), "переездов: 6");

    let Some(s) = traced_growth("twice") else { return };
    let second = s.iter().position(|l| l == "--- второй раз").unwrap();
    assert!(
        s[..second].iter().any(|l| l.starts_with("munmap(") && l.ends_with(", 4198400)")),
        "{s:#?}"
    );
    // После освобождения блока в 4 МиБ порог mmap поднят: второй рост — в
    // куче через brk, и почти всегда на месте.
    let rest = &s[second..];
    assert!(!rest.iter().any(|l| l.starts_with("mmap(") || l.starts_with("mremap(")), "{rest:#?}");
    assert!(rest.iter().filter(|l| l.starts_with("brk(")).count() >= 3, "{rest:#?}");
    assert_eq!(rest.iter().find(|l| l.starts_with("переездов")).unwrap(), "переездов: 1");
}
