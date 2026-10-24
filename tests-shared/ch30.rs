// Общие тесты задач главы 30.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch30/tests/ch30.rs` и `solutions/ch30/tests/ch30.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Замеры идут в release-сборке программы `ch30-task`
// (tests-shared/ch30-task.rs): тест собирает её сам. Детерминированные
// проверки — число выделений памяти, число инструкций и промахи модели
// кэша и предсказателя cachegrind, ассемблер; время проверяют только тесты
// `observed_`, с большим запасом. Без valgrind опыты под cachegrind
// пропускаются, `CH30_REQUIRE_VALGRIND` делает пропуск ошибкой. Ассемблер
// задачи D2 — наблюдение для rustc 1.97 (префикс `rustc_`).

use std::collections::HashMap;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Каталог для сборок теста: свой у заготовок и у решений.
fn tmp(name: &str) -> PathBuf {
    Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-{name}", env!("CARGO_PKG_NAME")))
}

/// Release-сборка программы `ch30-task` этого крейта, один раз на прогон.
fn task_bin() -> &'static Path {
    static BIN: OnceLock<PathBuf> = OnceLock::new();
    BIN.get_or_init(|| {
        let dir = tmp("release");
        let st = Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_remove("RUSTFLAGS")
            .args(["build", "--release", "--bin", "ch30-task", "--target-dir"])
            .arg(&dir)
            .status()
            .unwrap();
        assert!(st.success());
        dir.join("release/ch30-task")
    })
}

/// Замерам времени не должны мешать ни другие замеры, ни cachegrind, ни
/// сборка: внешние программы тест запускает по одной.
fn serial() -> MutexGuard<'static, ()> {
    static SERIAL: Mutex<()> = Mutex::new(());
    SERIAL.lock().unwrap_or_else(|e| e.into_inner())
}

/// Вывод `ch30-task` с аргументами `args`. Программа идёт без огромных
/// страниц (THP), как при замерах разборов: `prctl(PR_SET_THP_DISABLE)`
/// перед `exec`. В режиме THP `always` 64 МиБ задачи A5 легли бы на
/// страницы по 2 МиБ — десятки сбоев вместо 32 768, — и холодный проход
/// бывал бы медленнее прогретого всего в два-три раза.
fn task(args: &[&str]) -> String {
    let bin = task_bin();
    let _one = serial();
    let mut cmd = Command::new(bin);
    // SAFETY: между fork и exec — только системный вызов, без выделений
    // памяти и блокировок.
    unsafe {
        cmd.pre_exec(|| {
            let (on, zero): (libc::c_ulong, libc::c_ulong) = (1, 0);
            match libc::prctl(libc::PR_SET_THP_DISABLE, on, zero, zero, zero) {
                0 => Ok(()),
                _ => Err(std::io::Error::last_os_error()),
            }
        })
    };
    let out = cmd.args(args).output().unwrap();
    assert!(out.status.success(), "{args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap()
}

/// Число после `prefix` в строке вывода.
fn number_after(text: &str, prefix: &str) -> f64 {
    let at = text.find(prefix).unwrap_or_else(|| panic!("{prefix}: {text}")) + prefix.len();
    let rest = text[at..].trim_start();
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
    rest[..end].parse().unwrap_or_else(|_| panic!("{prefix}: {rest}"))
}

// ---------------------------------------------------------------- cachegrind

/// Модель кэша — та же, что в опытах главы (раздел 30.4).
const MODEL: [&str; 3] = ["--I1=32768,8,64", "--D1=32768,8,64", "--LL=8388608,16,64"];

/// События по функциям: имя функции → имя события → число.
type Costs = HashMap<String, HashMap<String, u64>>;

fn by_function(out: &str) -> Costs {
    let mut events: Vec<&str> = Vec::new();
    let mut map: Costs = HashMap::new();
    let mut current = None;
    for line in out.lines() {
        if let Some(names) = line.strip_prefix("events: ") {
            events = names.split_whitespace().collect();
        } else if let Some(name) = line.strip_prefix("fn=") {
            current = Some(name.to_owned());
        } else if line.starts_with(|c: char| c.is_ascii_digit()) {
            let e = map.entry(current.clone().unwrap()).or_default();
            for (i, n) in line.split(' ').skip(1).enumerate() {
                *e.entry(events[i].to_owned()).or_default() += n.parse::<u64>().unwrap();
            }
        }
    }
    map
}

/// `ch30-task` под cachegrind: `sim` — `ir` (только инструкции), `cache`
/// (модель [`MODEL`]) или `branch` (модель предсказателя).
fn cachegrind(sim: &str, args: &[&str]) -> Option<Costs> {
    let valgrind =
        std::env::var_os("VALGRIND").map_or_else(|| PathBuf::from("valgrind"), PathBuf::from);
    let ok = Command::new(&valgrind).arg("--version").output().is_ok_and(|o| o.status.success());
    if !ok {
        assert!(std::env::var_os("CH30_REQUIRE_VALGRIND").is_none(), "нужен valgrind");
        eprintln!("valgrind не найден: опыт пропущен (путь можно задать в VALGRIND)");
        return None;
    }
    let bin = task_bin();
    let _one = serial();
    let dir = tmp("cachegrind");
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(format!("{sim}-{}.out", args.join("-")));
    let mut cmd = Command::new(valgrind);
    cmd.arg("--tool=cachegrind");
    match sim {
        "ir" => cmd.arg("--cache-sim=no"),
        "cache" => cmd.arg("--cache-sim=yes").args(MODEL),
        "branch" => cmd.args(["--cache-sim=no", "--branch-sim=yes"]),
        _ => unreachable!(),
    };
    let out = cmd
        .arg(format!("--cachegrind-out-file={}", file.display()))
        .arg(bin)
        .args(args)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    Some(by_function(&std::fs::read_to_string(file).unwrap()))
}

/// Сумма события по всей программе.
fn total(costs: &Costs, event: &str) -> u64 {
    costs.values().map(|e| e.get(event).copied().unwrap_or(0)).sum()
}

/// Событие функции, имя которой кончается на `suffix`.
fn of(costs: &Costs, suffix: &str, event: &str) -> u64 {
    let (_, e) = costs
        .iter()
        .find(|(name, _)| name.ends_with(suffix))
        .unwrap_or_else(|| panic!("нет функции …{suffix}: она встроена?"));
    e.get(event).copied().unwrap_or(0)
}

// ---------------------------------------------------------------- A

// ANCHOR: a
use sut::a_bench::Mistake;

#[test]
fn task_a_mistakes() {
    use Mistake::*;
    let want = [SetupInTimedRegion, DiscardedResult, DebugBuild, ConstantFolding, ColdStart];
    assert_eq!(sut::a_bench::mistakes(), want);
}

// ANCHOR: a_checks
/// Выделения памяти за один вызов замера на `iters` повторов.
fn allocations(name: &str, iters: u32) -> f64 {
    number_after(&task(&["a", name, &iters.to_string()]), "выделений")
}

#[test]
fn task_a1_setup_out_of_timed_region() {
    // У bench_1 вход готовится в каждом повторе, у исправленного — один раз.
    assert_eq!(allocations("bench_1", 20) - allocations("bench_1", 10), 10.0);
    assert_eq!(allocations("fixed_1", 20), allocations("fixed_1", 10));
}

/// Инструкций на один повтор замера: разница двух прогонов под cachegrind.
/// Считаются только функции задачи и программы: печать числа с плавающей
/// точкой идёт разными путями для разных чисел, и её инструкции от
/// прогона к прогону меняются.
fn ir_per_iter(name: &str) -> Option<f64> {
    let ir = |iters: &str| -> Option<u64> {
        let c = cachegrind("ir", &["a", name, iters])?;
        let ours =
            c.iter().filter(|(f, _)| f.contains("a_bench::") || f.starts_with("ch30_task::"));
        Some(ours.map(|(_, e)| e.get("Ir").copied().unwrap_or(0)).sum())
    };
    let (short, long) = (ir("1000")?, ir("2000")?);
    Some((long as f64 - short as f64) / 1000.0)
}

#[test]
fn task_a2_a4_work_is_not_thrown_away() {
    let Some(discarded) = ir_per_iter("bench_2") else { return };
    // Выброшенный хеш — ни одной инструкции на повтор; настоящий — по
    // несколько на каждый из 4096 байт.
    assert!(discarded < 1.0, "bench_2: {discarded} инструкций на повтор");
    let fixed = ir_per_iter("fixed_2").unwrap();
    assert!(fixed > 2.0 * 4096.0, "fixed_2: {fixed} инструкций на повтор");
    // Хеш ключа, известного при компиляции, — готовое число.
    let folded = ir_per_iter("bench_4").unwrap();
    assert!(folded < 16.0, "bench_4: {folded} инструкций на повтор");
    let fixed = ir_per_iter("fixed_4").unwrap();
    assert!(fixed > 2.0 * 64.0, "fixed_4: {fixed} инструкций на повтор");
}

#[test]
fn task_a3_no_comparison_in_debug() {
    // Тесты собраны в профиле dev: исправленный замер отказывается мерить.
    let in_test = sut::a_bench::fixed_3(1);
    assert_eq!(in_test.is_none(), cfg!(debug_assertions), "{in_test:?}");
    let release = number_after(&task(&["a", "fixed_3", "10"]), "fixed_3:");
    assert!(release.is_finite() && release > 0.0, "{release}");
}

#[test]
fn observed_task_a_numbers() {
    let a = |name: &str, iters: &str| number_after(&task(&["a", name, iters]), &format!("{name}:"));
    let (setup, hash) = (a("bench_1", "100"), a("fixed_1", "100"));
    eprintln!("A1: с подготовкой {setup:.3} нс на байт, без неё {hash:.3}");
    assert!(setup > 1.5 * hash);
    // В debug перебор по индексам и итератор различаются, в release — нет.
    let debug = sut::a_bench::bench_3(20);
    let release = a("fixed_3", "1000");
    eprintln!("A3: индексы / итератор — debug {debug:.2}, release {release:.2}");
    if cfg!(debug_assertions) {
        assert!(debug > 1.5, "{debug}");
    }
    assert!((0.5..2.0).contains(&release), "{release}");
    let (cold, warm) = (a("bench_5", "1"), a("fixed_5", "5"));
    eprintln!("A5: холодный {cold:.3} нс на байт, прогретый {warm:.3}");
    assert!(cold > 3.0 * warm);
}
// ANCHOR_END: a_checks
// ANCHOR_END: a

// ---------------------------------------------------------------- B

// ANCHOR: b
use sut::b_particles::{Particles, cloud, energy_aos, step_aos};

#[test]
fn task_b_same_particles() {
    for n in [0, 1, 7, 1000] {
        let ps = cloud(n);
        let mut soa = Particles::from_aos(&ps);
        assert_eq!(soa.to_aos(), ps);
        // Та же сумма в том же порядке — то же число до бита.
        assert_eq!(soa.energy().to_bits(), energy_aos(&ps).to_bits());
        let mut aos = ps.clone();
        for _ in 0..3 {
            step_aos(&mut aos, 0.25);
            soa.step(0.25);
        }
        assert_eq!(soa.to_aos(), aos);
    }
}

#[test]
fn task_b_cache_misses() {
    let n = "262144";
    let Some(aos) = cachegrind("cache", &["b", "aos", n, "1"]) else { return };
    let soa = cachegrind("cache", &["b", "soa", n, "1"]).unwrap();
    let misses = |c: &Costs, f: &str| of(c, f, "D1mr") + of(c, f, "D1mw");
    // Энергии нужны 16 байт частицы из 48, шагу — 24 из 48: столбцы
    // приносят в кэш только их.
    let (ea, es) = (misses(&aos, "::energy_aos"), misses(&soa, "Particles>::energy"));
    let (sa, ss) = (misses(&aos, "::step_aos"), misses(&soa, "Particles>::step"));
    eprintln!("промахи D1: energy AoS {ea}, SoA {es}; step AoS {sa}, SoA {ss}");
    assert!(2 * es < ea, "energy: AoS {ea}, SoA {es}");
    assert!(3 * ss < 2 * sa, "step: AoS {sa}, SoA {ss}");
}

#[test]
fn observed_task_b_soa_faster() {
    let aos = task(&["b", "aos"]);
    let soa = task(&["b", "soa"]);
    eprint!("{aos}{soa}");
    let (ea, es) = (number_after(&aos, "energy"), number_after(&soa, "energy"));
    let (sa, ss) = (number_after(&aos, "step"), number_after(&soa, "step"));
    // Энергия столбцами упирается в цепочку сложений f64, строками — в
    // память: насколько выигрыш больше полутора раз, зависит от машины.
    assert!(1.5 * es < ea, "energy: AoS {ea}, SoA {es}");
    assert!(1.3 * ss < sa, "step: AoS {sa}, SoA {ss}");
}
// ANCHOR_END: b

// ---------------------------------------------------------------- C

// ANCHOR: c
use sut::c_ngrams::{text, top_fast, top_naive};

#[test]
fn task_c_same_top() {
    let bytes: Vec<u8> = (0..=255u8).cycle().take(3000).collect();
    let texts: [&[u8]; 6] =
        [b"", b"ab", b"aaaaaaaaaaaa", b"abababab abab ba", &bytes, &text(50_000)];
    for t in texts {
        for n in 1..=8 {
            for k in [0, 1, 5, 1000] {
                assert_eq!(top_fast(t, n, k), top_naive(t, n, k), "n = {n}, k = {k}, {t:?}");
            }
        }
    }
}

#[test]
fn task_c_ten_times_fewer_instructions() {
    let len = (4 << 20).to_string();
    let Some(base) = cachegrind("ir", &["c", "text", &len]) else { return };
    let base = total(&base, "Ir");
    for n in ["3", "5"] {
        let ir = |v: &str| total(&cachegrind("ir", &["c", v, &len, n]).unwrap(), "Ir") - base;
        let (naive, fast) = (ir("naive"), ir("fast"));
        let per = |x: u64| x as f64 / (4 << 20) as f64;
        eprintln!(
            "n = {n}: инструкций на байт — наивно {:.1}, быстро {:.1}",
            per(naive),
            per(fast)
        );
        assert!(fast * 10 <= naive, "n = {n}: {naive} против {fast}");
    }
}

#[test]
fn observed_task_c_ten_times_faster() {
    let len = (16 << 20).to_string();
    // Один прогон — один замер без прогрева: лучший из трёх, шум только
    // добавляет время.
    let best = |v: &str, n: &str| {
        (0..3)
            .map(|_| number_after(&task(&["c", v, &len, n]), &format!("{v}:")))
            .fold(f64::INFINITY, f64::min)
    };
    for n in ["3", "5"] {
        let (naive, fast) = (best("naive", n), best("fast", n));
        eprintln!("n = {n}: наивно {naive:.2} нс на байт, быстро {fast:.2}");
        assert!(fast * 8.0 < naive, "n = {n}: {naive} против {fast}");
    }
}
// ANCHOR_END: c

// ---------------------------------------------------------------- D

// ANCHOR: d1
use sut::d_branches::{Codegen, lower_bound_branchless, lower_bound_branchy, predictions};

#[test]
fn task_d1_lower_bound() {
    for n in 0..40u32 {
        // Повторы: каждое значение встречается до трёх раз подряд.
        let v: Vec<u32> = (0..n).map(|i| i / 3 * 2 + 1).collect();
        for x in (0..n + 4).chain([u32::MAX]) {
            let want = v.partition_point(|&e| e < x);
            assert_eq!(lower_bound_branchy(&v, x), want, "branchy: n = {n}, x = {x}");
            assert_eq!(lower_bound_branchless(&v, x), want, "branchless: n = {n}, x = {x}");
        }
    }
}

#[test]
fn task_d1_branch_model() {
    // Один замер и прогрев по 100 000 запросов, и ещё один проход для
    // суммы — 300 000 вызовов.
    let calls = 300_000.0;
    let bcm = |variant: &str, order: &str| -> Option<f64> {
        let c = cachegrind("branch", &["d", variant, order, "1024", "100000", "1"])?;
        Some(of(&c, &format!("::lower_bound_{variant}"), "Bcm") as f64 / calls)
    };
    let Some(random) = bcm("branchy", "random") else { return };
    let sorted = bcm("branchy", "sorted").unwrap();
    let flat = bcm("branchless", "random").unwrap();
    eprintln!(
        "промахов на запрос: с ветвлением {random:.2} (по порядку {sorted:.2}), без {flat:.2}"
    );
    // Десять шагов, на случайных запросах угадать можно половину.
    assert!(random > 4.0, "{random}");
    assert!(sorted < 0.7 * random, "{sorted}");
    // Без ветвления промахивается только выход из цикла.
    assert!(flat < 1.2, "{flat}");
}
// ANCHOR_END: d1

// ANCHOR: d2
/// Функции задачи D2 в том порядке, в каком их перечисляет `predictions`.
const D2: [&str; 9] = [
    "max_of",
    "first_or_zero",
    "div_or_zero",
    "call_if",
    "clamp_byte",
    "heavy_if",
    "mul_if",
    "nonzero_and_even",
    "get_or",
];

/// Ассемблер библиотеки этого крейта: release, синтаксис Intel.
fn asm() -> String {
    let _one = serial();
    let dir = tmp("asm");
    let out = Command::new("cargo")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("RUSTUP_TOOLCHAIN")
        .env_remove("RUSTFLAGS")
        .args(["rustc", "--release", "--lib", "--target-dir"])
        .arg(&dir)
        .args(["--", "--emit", "asm", "-C", "codegen-units=1", "-C", "debuginfo=0"])
        .args(["-C", "llvm-args=-x86-asm-syntax=intel"])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
    let prefix = format!("{}-", env!("CARGO_PKG_NAME").replace('-', "_"));
    let file = std::fs::read_dir(dir.join("release/deps"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with(&prefix) && n.ends_with(".s")
        })
        .expect("нет файла .s");
    std::fs::read_to_string(file).unwrap()
}

/// Инструкции функции `name` модуля `d_branches`.
fn body<'a>(asm: &'a str, name: &str) -> Vec<&'a str> {
    let label = format!("10d_branches{}{name}:", name.len());
    let start = asm
        .lines()
        .position(|l| l.starts_with("_R") && l.ends_with(&label))
        .unwrap_or_else(|| panic!("нет функции {name}: она встроена?"));
    asm.lines()
        .skip(start + 1)
        .take_while(|l| !l.starts_with(".Lfunc_end"))
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(['.', '#']))
        .collect()
}

#[test]
fn task_d2_rustc_predictions() {
    let asm = asm();
    let seen: Vec<Codegen> = D2
        .iter()
        .map(|name| {
            let b = body(&asm, name);
            // Условный переход — любой j<cc>, кроме безусловного jmp.
            let branch = b.iter().any(|l| l.starts_with('j') && !l.starts_with("jmp"));
            eprintln!("{name}: {}", b.join("; "));
            if branch { Codegen::Branch } else { Codegen::Branchless }
        })
        .collect();
    assert_eq!(predictions().to_vec(), seen);
}
// ANCHOR_END: d2

// ANCHOR: d3
#[test]
fn observed_task_d3_when_branches_win() {
    let ns = |variant: &str, order: &str, n: &str, q: &str| {
        let out = task(&["d", variant, order, n, q]);
        eprint!("{out}");
        number_after(&out, ":")
    };
    // Небольшой массив, случайные запросы: промахи предсказателя.
    let (branchy, flat) =
        (ns("branchy", "random", "1024", "1000000"), ns("branchless", "random", "1024", "1000000"));
    assert!(2.0 * flat < branchy, "{branchy} против {flat}");
    // Запросы по порядку: переход угадан, и с ветвлением поиск в разы
    // быстрее, чем на случайных; без ветвления — около того же времени.
    let random = ns("branchy", "random", "65536", "1000000");
    let sorted = ns("branchy", "sorted", "65536", "1000000");
    ns("branchless", "sorted", "65536", "1000000");
    assert!(2.0 * sorted < random, "{random} против {sorted}");
    // Массив больше кэша: время уходит на промахи кэша, и выигрыш кода без
    // ветвлений пропадает.
    let (branchy, flat) = (
        ns("branchy", "random", "16777216", "200000"),
        ns("branchless", "random", "16777216", "200000"),
    );
    assert!(flat > 0.8 * branchy, "{branchy} против {flat}");
    // Запросы цепочкой: следующий поиск ждёт ответа на этот, и соседние
    // запросы больше не перекрываются. Переход процессор угадывает и идёт
    // дальше, не дожидаясь загрузки, по которой тот решается; cmov ждёт её
    // на каждом шаге. На запросах по порядку угадан почти каждый переход, на
    // большом массиве и случайных — половина, но и половина загрузок,
    // начатых заранее, окупает промахи.
    let (branchy, flat) = (
        ns("branchy", "sorted-chain", "1024", "1000000"),
        ns("branchless", "sorted-chain", "1024", "1000000"),
    );
    assert!(flat > 1.5 * branchy, "{branchy} против {flat}");
    let (branchy, flat) = (
        ns("branchy", "random-chain", "16777216", "200000"),
        ns("branchless", "random-chain", "16777216", "200000"),
    );
    assert!(flat > 1.5 * branchy, "{branchy} против {flat}");
}
// ANCHOR_END: d3
