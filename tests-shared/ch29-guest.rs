//! Init гостя для задач главы 29: `/init задача [аргументы]`.
//!
//! Общая программа заготовок и решений: тесты из `tests-shared/ch29.rs`
//! собирают её статически и кладут в initramfs вместе с модулями задач.
//! Сценарии печатают сеанс так же, как программа гостя примеров главы
//! (`code/ch29/src/bin/ch29-guest.rs`), а тест разбирает его.

use ch29::guest::{in_child, poweroff};
use ch29::session::{END, Session};
use nix::sys::wait::WaitStatus;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

/// Как кончился дочерний процесс: `pid 45: код 0` или `pid 45: сигнал SIGSEGV`.
fn status(st: WaitStatus) -> String {
    match st {
        WaitStatus::Exited(pid, code) => format!("pid {pid}: код {code}"),
        WaitStatus::Signaled(pid, sig, _) => format!("pid {pid}: сигнал {sig:?}"),
        other => format!("{other:?}"),
    }
}

/// Результат системного вызова одной строкой: `Ok(3)` или `Err(EINVAL)`.
fn res<T: std::fmt::Debug>(r: io::Result<T>) -> String {
    match r {
        Ok(v) => format!("Ok({v:?})"),
        Err(e) => match e.raw_os_error() {
            Some(n) => format!("Err({:?})", nix::errno::Errno::from_raw(n)),
            None => format!("Err({e})"),
        },
    }
}

// ANCHOR: a
/// 29.A: один модуль фрагмента — в своей загрузке ядра: lockdep после
/// первого отчёта выключается, KASAN по умолчанию сообщает только о первой
/// ошибке, а oops оставляет ядро «испорченным». Модуль грузит дочерний
/// процесс — его pid стоит в отчёте kmemleak о выделениях.
///
/// kmemleak называет объект потерянным, только если тот старше пяти
/// секунд и его содержимое не менялось между двумя проходами: объект,
/// в который ещё пишут, может быть жив. Поэтому — пауза и два прохода.
fn task_a(s: &mut Session, module: &str) -> io::Result<()> {
    let ko = format!("/lib/{module}.ko");
    if !Path::new(&ko).exists() {
        s.say(&format!("{ko}: нет — модуль не собрался"));
        return Ok(());
    }
    let start = Instant::now();
    let st = in_child(|| i32::from(ch29::guest::insmod(&ko, "").is_err()));
    s.say(&format!("# insmod {ko}: {}", status(st)));
    // Отчёты ядро печатает до того, как потомок вернётся из insmod.
    s.dmesg();
    if let Some(rest) = Duration::from_millis(5500).checked_sub(start.elapsed()) {
        std::thread::sleep(rest);
    }
    let kmemleak = "/sys/kernel/debug/kmemleak";
    for _ in 0..2 {
        s.say(&format!("# echo scan > {kmemleak}"));
        fs::write(kmemleak, "scan")?;
    }
    s.cat(kmemleak);
    s.dmesg();
    Ok(())
}
// ANCHOR_END: a

// ANCHOR: b
fn task_b(s: &mut Session) -> io::Result<()> {
    for params in ["", "n=0", "n=1", "n=2", "n=93"] {
        if s.insmod("ch29_b", params).is_ok() {
            s.rmmod("ch29_b")?;
        }
    }
    s.insmod("ch29_b", "n=94").ok();
    s.grep("ch29_b", "/proc/modules");
    Ok(())
}
// ANCHOR_END: b

// ANCHOR: c
fn open(dev: &str) -> io::Result<File> {
    OpenOptions::new().read(true).write(true).open(dev)
}

/// Чтение с нового открытия: то, что увидит `cat`.
fn read_fresh(dev: &str) -> String {
    let mut text = String::new();
    match open(dev).and_then(|mut f| f.read_to_string(&mut text)) {
        Ok(_) => format!("{text:?}"),
        Err(e) => res::<()>(Err(e)),
    }
}

fn write_fresh(dev: &str, data: &[u8]) -> String {
    res(open(dev).and_then(|mut f| f.write(data)))
}

/// Один и тот же сеанс для счётчика на C и на Rust: тест сравнит их.
fn counter_session(s: &mut Session, dev: &str) -> io::Result<()> {
    s.say(&format!("read: {}", read_fresh(dev)));
    let writes: [&[u8]; 12] = [
        b"5",
        b"+7\n",
        b"abc",
        b"",
        b"-1",
        b"1 ",
        b"12\n\n",
        b"000000000000000000000001",
        b"00000000000000000000001",
        b"18446744073709551616",
        b"18446744073709551615",
        b"8",
    ];
    for w in writes {
        s.say(&format!("write {:?}: {}", String::from_utf8_lossy(w), write_fresh(dev, w)));
    }
    s.say(&format!("read: {}", read_fresh(dev)));

    // Снимок: начатое чтение продолжается со старым значением.
    let mut a = open(dev)?;
    let mut first = [0u8; 1];
    s.say(&format!("a: read 1 байт: {} {:?}", res(a.read(&mut first)), first[0] as char));
    s.say(&format!("b: write \"+100\": {}", write_fresh(dev, b"+100")));
    let mut rest = String::new();
    let r = res(a.read_to_string(&mut rest));
    s.say(&format!("a: дочитать: {r} {rest:?}"));
    s.say(&format!("read: {}", read_fresh(dev)));
    // lseek у обоих устройств нет (в file_operations нет .llseek), а
    // pread с позиции 0 начинает чтение заново — с новым снимком.
    s.say(&format!("a: lseek 0: {}", res(a.seek(SeekFrom::Start(0)))));
    let mut again = [0u8; 24];
    let r = std::os::unix::fs::FileExt::read_at(&a, &mut again, 0);
    let text =
        r.as_ref().map_or(String::new(), |&n| String::from_utf8_lossy(&again[..n]).into_owned());
    s.say(&format!("a: pread с 0: {} {text:?}", res(r)));
    drop(a);

    // Одновременная запись из нескольких процессов.
    let (procs, each) = (4, 500);
    let mut kids = Vec::new();
    for _ in 0..procs {
        // SAFETY: гость однопоточный; потомок только пишет и выходит.
        match unsafe { nix::unistd::fork() }.expect("fork") {
            nix::unistd::ForkResult::Child => {
                let ok = (0..each).all(|_| open(dev).and_then(|mut f| f.write(b"1")).is_ok());
                std::process::exit(i32::from(!ok));
            }
            nix::unistd::ForkResult::Parent { child } => kids.push(child),
        }
    }
    let codes: Vec<String> = kids
        .into_iter()
        .map(|k| match nix::sys::wait::waitpid(k, None).expect("waitpid") {
            WaitStatus::Exited(_, code) => code.to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    s.say(&format!("{procs} процесса по {each} записей \"1\": коды {}", codes.join(" ")));
    s.say(&format!("read: {}", read_fresh(dev)));
    s.dmesg();
    Ok(())
}

fn task_c(s: &mut Session) -> io::Result<()> {
    for (module, dev) in
        [("ch29_counter_c", "/dev/ch29-counter-c"), ("ch29_c", "/dev/ch29-counter")]
    {
        s.say(&format!("### {module}"));
        if s.insmod(module, "").is_err() {
            continue;
        }
        counter_session(s, dev)?;
        s.rmmod(module)?;
    }
    s.say("### конец");
    Ok(())
}
// ANCHOR_END: c

// ANCHOR: d1
/// Каталог /sys/kernel/kobject_example: файлы, права и значения.
fn kobject_session(s: &mut Session) -> io::Result<()> {
    let dir = Path::new("/sys/kernel/kobject_example");
    let mut names: Vec<String> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
            .collect::<io::Result<_>>()?,
        Err(e) => {
            s.say(&format!("ls {}: {}", dir.display(), res::<()>(Err(e))));
            return Ok(());
        }
    };
    names.sort();
    s.say(&format!("ls: {}", names.join(" ")));
    for n in &names {
        let mode = fs::metadata(dir.join(n))?.permissions().mode() & 0o7777;
        s.say(&format!("{n}: права {mode:o}, значение {:?}", fs::read_to_string(dir.join(n))?));
    }
    let writes: [(&str, &[u8]); 7] = [
        ("foo", b"42"),
        ("baz", b"7\n"),
        ("bar", b"-3"),
        ("foo", b"x"),
        ("bar", b"99999999999"),
        ("baz", b"0x10"),
        ("foo", b""),
    ];
    for (n, w) in writes {
        let r = OpenOptions::new().write(true).open(dir.join(n)).and_then(|mut f| f.write(w));
        s.say(&format!("echo {:?} > {n}: {}", String::from_utf8_lossy(w), res(r)));
    }
    for n in &names {
        s.say(&format!("{n}: {:?}", fs::read_to_string(dir.join(n))?));
    }
    Ok(())
}

fn task_d1(s: &mut Session) -> io::Result<()> {
    for module in ["kobject-example", "ch29_d1"] {
        s.say(&format!("### {module}"));
        if s.insmod(module, "").is_err() {
            continue;
        }
        kobject_session(s)?;
        // В модулях ядра имя с '-' пишется через '_'.
        s.rmmod(&module.replace('-', "_"))?;
        s.exists("/sys/kernel/kobject_example");
    }
    s.say("### конец");
    Ok(())
}
// ANCHOR_END: d1

// ANCHOR: d2
const TRACING: &str = "/sys/kernel/tracing";

fn tracefs(file: &str, value: &str) -> io::Result<()> {
    fs::write(format!("{TRACING}/{file}"), value)
}

/// `n` чтений по 16 байт с позиции 0: среднее время одного pread, нс.
fn preads(f: &File, n: u32) -> io::Result<u128> {
    use std::os::unix::fs::FileExt;
    let mut buf = [0u8; 16];
    let start = Instant::now();
    for _ in 0..n {
        f.read_at(&mut buf, 0)?;
    }
    Ok(start.elapsed().as_nanos() / u128::from(n))
}

/// 29.D2: горячий путь read двух устройств записей из раздела 29.7 —
/// на Rust и на C. Сначала замер из пространства пользователя без
/// трассировки, потом function_graph по vfs_read на три уровня вглубь.
fn task_d2(s: &mut Session, calls: u32) -> io::Result<()> {
    let devs = [("ch29_notes", "/dev/ch29-notes"), ("ch29_notes_c", "/dev/ch29-notes-c")];
    for (module, _) in devs {
        s.insmod(module, "")?;
    }
    let files: Vec<File> = devs.iter().map(|(_, d)| open(d)).collect::<io::Result<_>>()?;
    for f in &files {
        (&*f).write_all(b"0123456789abcdef0123456789")?;
    }
    for ((module, _), f) in devs.iter().zip(&files) {
        preads(f, 1000)?; // прогрев
        s.say(&format!("{module}: pread 16 байт: {} нс", preads(f, 20 * calls)?));
    }
    tracefs("tracing_on", "0")?;
    tracefs("current_tracer", "function_graph")?;
    tracefs("max_graph_depth", "3")?;
    tracefs("set_graph_function", "vfs_read")?;
    tracefs("set_ftrace_pid", &std::process::id().to_string())?;
    tracefs("options/funcgraph-tail", "1")?;
    for ((module, _), f) in devs.iter().zip(&files) {
        tracefs("trace", "")?;
        tracefs("tracing_on", "1")?;
        preads(f, calls)?;
        tracefs("tracing_on", "0")?;
        s.say(&format!("### трасса {module}"));
        print!("{}", fs::read_to_string(format!("{TRACING}/trace"))?);
        s.say("### конец трассы");
    }
    tracefs("current_tracer", "nop")?;
    drop(files);
    for (module, _) in devs {
        s.rmmod(module)?;
    }
    s.dmesg();
    Ok(())
}
// ANCHOR_END: d2

fn main() {
    let mut s = Session::start().expect("сеанс");
    s.hide("loading out-of-tree module taints kernel");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let task = args.first().map(String::as_str).unwrap_or_default();
    let result = match task {
        "a" => task_a(&mut s, args.get(1).map_or("", String::as_str)),
        "b" => task_b(&mut s),
        "c" => task_c(&mut s),
        "d1" => task_d1(&mut s),
        "d2" => task_d2(&mut s, args.get(1).and_then(|n| n.parse().ok()).unwrap_or(200)),
        other => {
            println!("нет задачи {other:?}");
            Ok(())
        }
    };
    if let Err(e) = result {
        println!("задача {task}: {e}");
    }
    println!("{END}");
    poweroff();
}
