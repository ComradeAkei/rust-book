//! Опыты главы 29 в QEMU. Каждый тест собирает (один раз) модули из
//! `modules/` и программу гостя, грузит ядро с initramfs и проверяет
//! вывод сеанса. Без собранного ядра тесты пропускаются.

use ch29::host::{Flavor, Initramfs, Kernel, Qemu, guest_binary, lines_with, repo_root};
use ch29::session::{guest_part, without_console};
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

fn tmp(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch29-{name}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// initramfs с программой гостя и всеми модулями главы для ядра `flavor`.
fn image(kernel: &Kernel) -> PathBuf {
    static GUEST: OnceLock<PathBuf> = OnceLock::new();
    static PLAIN: OnceLock<PathBuf> = OnceLock::new();
    static DEBUG: OnceLock<PathBuf> = OnceLock::new();
    let guest = GUEST.get_or_init(|| guest_binary("ch29", "ch29-guest", &tmp("guest-target")));
    let cell = if kernel.flavor == Flavor::Plain { &PLAIN } else { &DEBUG };
    cell.get_or_init(|| {
        let name = format!("{:?}", kernel.flavor).to_lowercase();
        let out = tmp(&format!("modules-{name}"));
        let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("modules");
        // CLIPPY=1: модули главы обязаны проходить линты ядра без
        // единого предупреждения — как код, который примут в ядро.
        match kernel.make_modules_with(&src, &out, &["CLIPPY=1"]) {
            Ok(log) => assert!(!log.contains("warning:"), "предупреждения:\n{log}"),
            Err(log) => panic!("модули не собрались:\n{log}"),
        }
        let mut img = Initramfs::new(guest);
        let mut kos: Vec<_> = std::fs::read_dir(&out)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "ko"))
            .collect();
        kos.sort();
        for ko in &kos {
            img = img.lib(ko);
        }
        let cpio = out.join("initramfs.cpio");
        img.write(kernel, &cpio).unwrap();
        cpio
    })
    .clone()
}

/// Весь вывод последовательного порта; `None` — ядра нет.
fn run_serial(flavor: Flavor, scenario: &str, setup: impl FnOnce(&mut Qemu)) -> Option<String> {
    let kernel = Kernel::or_skip(flavor)?;
    let initrd = image(&kernel);
    let mut q = Qemu::new(&kernel, &initrd, &[scenario]);
    setup(&mut q);
    Some(q.run().unwrap())
}

/// Вывод сценария гостя; `None` — ядра нет.
fn run(flavor: Flavor, scenario: &str, setup: impl FnOnce(&mut Qemu)) -> Option<String> {
    let serial = run_serial(flavor, scenario, setup)?;
    let body = guest_part(&serial).unwrap_or_else(|| panic!("гость не дошёл до конца:\n{serial}"));
    let body = without_console(body);
    println!("{body}");
    Some(body)
}

// ANCHOR: hello
#[test]
fn hello_module_with_parameter() {
    let Some(out) = run(Flavor::Plain, "hello", |_| {}) else { return };
    assert!(out.contains("warn:   ch29_hello: loading out-of-tree module taints kernel."), "{out}");
    assert!(out.contains("info:   ch29_hello: привет #1\n"), "{out}");
    assert!(out.contains("info:   ch29_hello: привет #3\n"), "{out}");
    assert!(out.contains("пока, было приветствий: 3"), "{out}");
    assert!(out.contains("err:    ch29_hello: times = 11: не больше 10"), "{out}");
}
// ANCHOR_END: hello

/// Строки сеанса с префиксом `info:   <модуль>: `, без него.
fn info<'s>(out: &'s str, module: &str) -> Vec<&'s str> {
    lines_with(out, &format!("info:   {module}: "))
}

// ANCHOR: errors
#[test]
fn errors_roll_back_in_reverse_order() {
    let Some(out) = run(Flavor::Plain, "errors", |_| {}) else { return };
    let errnos = ["ENOMEM", "EBUSY", "ENOENT", "EINVAL"];
    let codes = ["-12", "-16", "-2", "-22"];
    let os = ["(os error 12)", "(os error 16)", "(os error 2)", "(os error 22)"];
    for (module, dev, fail) in [
        ("ch29_errors", "/dev/ch29-errors", errnos),
        ("ch29_errors_c", "/dev/ch29-errors-c", codes),
    ] {
        let held = ["буфер", dev, "каталог debugfs"];
        for step in 1..=4 {
            // Отказ на шаге N освобождает N-1 ресурс — в обратном порядке.
            let freed: Vec<String> =
                held[..step - 1].iter().rev().map(|r| format!("освобождаю: {r}")).collect();
            let mut want = vec![format!("шаг {step}: отказ {}", fail[step - 1])];
            want.extend(freed);
            let session =
                out.split(&format!("# insmod /lib/{module}.ko fail_at={step}\n")).nth(1).unwrap();
            let session = session.split("# ").next().unwrap();
            assert!(session.contains(os[step - 1]), "{session}");
            assert_eq!(info(session, module), want, "{module}, шаг {step}");
        }
        let unload = out.split(&format!("# rmmod {module}\n")).nth(1).unwrap();
        let unload: Vec<&str> = info(unload, module).into_iter().take(3).collect();
        let mut order: Vec<String> = held.iter().map(|r| format!("освобождаю: {r}")).collect();
        if module == "ch29_errors_c" {
            // goto-цепочка в exit — всегда обратный порядок.
            order.reverse();
        }
        // В Rust поля структуры модуля освобождаются в порядке объявления.
        assert_eq!(unload, order, "{module}: выгрузка");
        assert!(out.contains(&format!("# ls -d {dev}: есть")), "{out}");
    }
}
// ANCHOR_END: errors

/// Число после `prefix` в строке вывода: `0x…` — шестнадцатеричное.
fn number_after(out: &str, prefix: &str) -> u64 {
    let rest = out.split(prefix).nth(1).unwrap_or_else(|| panic!("нет {prefix:?}:\n{out}"));
    let word = rest.split(|c: char| !c.is_ascii_alphanumeric()).next().unwrap();
    match word.strip_prefix("0x") {
        Some(hex) => u64::from_str_radix(hex, 16).unwrap(),
        None => word.parse().unwrap(),
    }
}

/// Шестнадцатеричное число без `0x` после `prefix`: так печатают `%px` в
/// C и отчёты ядра.
fn hex_after(out: &str, prefix: &str) -> u64 {
    let rest = out.split(prefix).nth(1).unwrap_or_else(|| panic!("нет {prefix:?}:\n{out}"));
    let word = rest.split(|c: char| !c.is_ascii_hexdigit()).next().unwrap();
    u64::from_str_radix(word, 16).unwrap()
}

// ANCHOR: layout
/// Области адресов ядра x86-64 с четырёхуровневыми таблицами страниц —
/// Documentation/arch/x86/x86_64/mm.rst. KASLR в ядре главы выключен, и
/// области начинаются точно с этих адресов.
const DIRECT_MAP: Range<u64> = 0xffff_8880_0000_0000..0xffff_c880_0000_0000;
const VMALLOC: Range<u64> = 0xffff_c900_0000_0000..0xffff_e900_0000_0000;
const MODULES: Range<u64> = 0xffff_ffff_a000_0000..0xffff_ffff_ff00_0000;
// ANCHOR_END: layout

// ANCHOR: alloc
#[test]
fn alloc_plain() {
    let Some(out) = run(Flavor::Plain, "alloc", |_| {}) else { return };
    let i = info(&out, "ch29_alloc");
    assert_eq!(i[0], "size_of: KBox<Point> 8, Option<KBox<Point>> 8, KVec<u8> 24, Flags 4");
    assert_eq!(i[2], "KVec: [1, 4, 9, 16, 25], ёмкость 8");
    assert_eq!(i[3], "KVec на 1 ТиБ: Some(AllocError)");
    assert_eq!(i[4], "64 МиБ: KVec Some(AllocError), KVVec None");
    // Стек задачи — тоже из vmalloc (VMAP_STACK).
    let big = number_after(&out, "KVVec по адресу ");
    let small = number_after(&out, "KVVec на 4 КиБ по адресу ");
    let shown = number_after(&out, "тот же указатель через {:p}: ");
    assert!(VMALLOC.contains(&big), "{big:#x}: 64 МиБ — из vmalloc");
    assert!(DIRECT_MAP.contains(&small), "{small:#x}: 4 КиБ — от kmalloc");
    assert_ne!(shown, big, "{{:p}} печатает не указатель, а адрес ссылки на стеке");
    assert!(VMALLOC.contains(&shown), "{shown:#x}: ссылка лежит на стеке");
    // Обычное ядро не замечает GFP_KERNEL под спин-блокировкой.
    assert!(out.contains("GFP_KERNEL под спин-блокировкой: вернулось"), "{out}");
    assert!(!out.contains("BUG:"), "{out}");
}

#[test]
fn alloc_debug() {
    let Some(out) = run(Flavor::Debug, "alloc", |_| {}) else { return };
    let at = out.find("# insmod /lib/ch29_alloc.ko under_lock=1").unwrap();
    let (first, second) = out.split_at(at);
    assert!(!first.contains("BUG:"), "GFP_ATOMIC — не ошибка:\n{first}");
    assert!(
        second.contains("err:    BUG: sleeping function called from invalid context"),
        "{second}"
    );
    assert!(second.contains("1 lock held by init/1:"), "{second}");
    assert!(second.contains("(ch29_alloc){"), "lockdep называет блокировку по имени: {second}");
    // Отчёт — предупреждение, а не остановка: выделение вернулось.
    assert!(second.contains("GFP_KERNEL под спин-блокировкой: вернулось"), "{second}");
}
// ANCHOR_END: alloc

// ANCHOR: pin
#[test]
fn pin_plain() {
    let Some(out) = run(Flavor::Plain, "pin", |_| {}) else { return };
    let heap = number_after(&out, "мьютекс в куче по адресу ");
    let moved = number_after(&out, "после перемещения Pin<KBox>: ");
    let stack = number_after(&out, "на стеке: мьютекс по адресу ");
    assert_eq!(heap, moved, "Pin<KBox> перемещает указатель, а не мьютекс");
    assert!(DIRECT_MAP.contains(&heap), "{heap:#x}: KBox — от kmalloc");
    assert!(VMALLOC.contains(&stack), "{stack:#x}: стек — vmalloc");
    assert!(out.contains("на стеке: мьютекс по адресу"), "{out}");
    assert!(out.contains("записи [7]") && out.contains("кадры: записи [1, 2]"), "{out}");
    // Копия мьютекса в C: её список ожидания указывает на оригинал.
    let orig = hex_after(&out, "оригинал: wait_list ");
    let copy_next = hex_after(&out, "копия:    wait_list ");
    let next = hex_after(&out, &format!("копия:    wait_list {copy_next:x}, next "));
    assert_eq!(next, orig, "{out}");
    assert!(MODULES.contains(&orig), "{orig:#x}: статика модуля");
    assert!(!out.contains("DEBUG_LOCKS_WARN_ON"), "обычное ядро копию не замечает");
}

#[test]
fn pin_debug() {
    let Some(out) = run(Flavor::Debug, "pin", |_| {}) else { return };
    assert!(out.contains("warn:   DEBUG_LOCKS_WARN_ON(lock->magic != lock)"), "{out}");
    assert!(out.contains("init_module+") && out.contains("[ch29_pin_c]"), "{out}");
    let rust = out.split("# insmod /lib/ch29_pin_c.ko").next().unwrap();
    assert!(!rust.contains("WARNING"), "в Rust-модуле копии нет:\n{rust}");
}
// ANCHOR_END: pin

// ANCHOR: sync
#[test]
fn sync_plain() {
    let Some(out) = run(Flavor::Plain, "sync", |_| {}) else { return };
    assert!(out.contains("под SpinLock: 400000, Atomic: 400000, ожидалось 400000"), "{out}");
    let racy = number_after(&out, "без синхронизации: ");
    assert!(racy <= 400_000, "{out}");
    assert!(out.contains("под spinlock: 400000, atomic64: 400000"), "{out}");
    // C раскладывает работы по процессорам сам: второй обязан быть.
    assert!(out.contains("ch29_sync_c: работа 1: CPU 1"), "{out}");
    println!("гонка в C: {racy} из 400000, потеряно {}", 400_000 - racy);
}
// ANCHOR_END: sync

// ANCHOR: notes
#[test]
fn notes_plain() {
    let Some(out) = run(Flavor::Plain, "notes", |_| {}) else { return };
    let mut sessions = out.split("# insmod ").skip(1);
    let (rust, c, simple) =
        (sessions.next().unwrap(), sessions.next().unwrap(), sessions.next().unwrap());
    let common = [
        "read(a): \"первая\\nвторая\\n\"",
        "read(b): \"\" — у b свой буфер",
        "ioctl(a, NOTES_LEN): Ok(0), длина 26",
        "write(a, 0x1000): -1 (Bad address (os error 14))",
        "pread(a, ядро): -1 (Bad address (os error 14))",
        "write(a, ядро): -1 (Bad address (os error 14))",
        "write(b, 5000 байт): Err(\"No space left on device (os error 28)\")",
        "ioctl(a, 0x6e63): -1 (Inappropriate ioctl for device (os error 25))",
        "NOTES_CLEAR: Ok(0), потом NOTES_LEN: Ok(0), длина 0",
    ];
    for s in [rust, c, simple] {
        for line in common {
            assert!(s.contains(line), "нет {line:?}:\n{s}");
        }
    }
    // Своя копия в пользователя и C: плохой адрес — EFAULT.
    assert!(rust.contains("pread(a, 0x1000): -1 (Bad address (os error 14))"), "{rust}");
    assert!(c.contains("pread(a, 0x1000): -1 (Bad address (os error 14))"), "{c}");
    // simple_read_from_buffer ядра 7.0 молча возвращает 0.
    assert!(simple.contains("pread(a, 0x1000): 0\n"), "{simple}");
}
// ANCHOR_END: notes

// ANCHOR: edu
#[test]
fn edu_plain() {
    let Some(out) = run(Flavor::Plain, "edu", |q| q.devices.push("edu".into())) else { return };
    assert!(out.contains("# cat /sys/bus/pci/devices/0000:00:03.0/vendor\n0x1234\n"), "{out}");
    assert!(out.contains("0x11e8"), "{out}");
    let dev = "info:   ch29_edu 0000:00:03.0: ";
    assert!(out.contains(&format!("{dev}edu 1.0, проверка: !0x12345678 = 0xedcba987")), "{out}");
    assert!(out.contains(&format!("{dev}10! = 3628800")), "{out}");
    assert!(out.contains(&format!("{dev}IRQ_RAISE: прерываний 2, причины 0x101")), "{out}");
    assert!(out.contains("PCI-MSI-0000:00:03.0") && out.contains("ch29_edu\n"), "{out}");
    assert!(out.contains(" : ch29_edu\n# rmmod ch29_edu"), "{out}");
    // До загрузки устройство выключено.
    assert!(out.contains("0x11e8\n# cat /sys/bus/pci/devices/0000:00:03.0/enable\n0\n"), "{out}");
    // После выгрузки регион MMIO свободен, а устройство осталось
    // включённым: у `pci::Device` в 7.0 нет `disable_device`.
    let tail = "# grep ch29_edu /proc/iomem\n# cat /sys/bus/pci/devices/0000:00:03.0/enable\n1\n";
    assert!(out.ends_with(tail), "{out}");
}
// ANCHOR_END: edu

// ANCHOR: oops
#[test]
fn oops_plain() {
    let Some(out) = run(Flavor::Plain, "oops", |_| {}) else { return };
    let reports = [
        ("unwrap", "called `Option::unwrap()` on a `None` value"),
        ("index", "index out of bounds: the len is 3 but the index is 5"),
        ("overflow", "attempt to add with overflow"),
    ];
    for (n, (cmd, msg)) in reports.iter().enumerate() {
        let part = out.split(&format!("# echo {cmd} > /dev/ch29-oops\n")).nth(1).unwrap();
        let part = part.split("# ").next().unwrap();
        assert!(part.contains("процесс-писатель: Signaled("), "{part}");
        assert!(part.contains("SIGSEGV"), "{part}");
        assert!(part.contains("emerg:  rust_kernel: panicked at ch29_oops.rs:"), "{part}");
        assert!(part.contains(msg), "{part}");
        assert!(part.contains("crit:   kernel BUG at rust/helpers/bug.c:"), "{part}");
        assert!(part.contains(&format!("Oops: invalid opcode: 0000 [#{}] SMP", n + 1)), "{part}");
        // Имена в трассе — искажённые по схеме v0 (раздел 29.9).
        assert!(part.contains("_RNv") && part.contains("ch29_oops7trigger"), "{part}");
    }
    // Сводка регистров после «end trace» — всегда регистры первого oops:
    // у второго свой RSP в отчёте и чужой в сводке.
    let rsp = |cmd: &str| -> Vec<String> {
        let part = out.split(&format!("# echo {cmd} > /dev/ch29-oops\n")).nth(1).unwrap();
        let part = part.split("# ").next().unwrap();
        part.lines().filter(|l| l.contains("RSP: 0018:")).map(str::to_string).collect()
    };
    let (first, second) = (rsp("unwrap"), rsp("index"));
    assert_eq!(first.len(), 2, "{first:?}");
    assert_eq!(first[0], first[1]);
    assert_eq!(second.len(), 2, "{second:?}");
    assert_ne!(second[0], first[0]);
    assert_eq!(second[1], first[0]);
    // Паника с захваченным мьютексом: раскрутки нет, и охранник так и
    // не отпустил мьютекс.
    let mut locked = out.split("# echo locked > /dev/ch29-oops\n").skip(1);
    let first = locked.next().unwrap().split("# ").next().unwrap();
    assert!(first.contains("паника с захваченным мьютексом"), "{first}");
    assert!(first.contains("Oops: invalid opcode: 0000 [#4] SMP"), "{first}");
    let second = locked.next().unwrap().split("# ").next().unwrap();
    assert!(second.contains("write: Device or resource busy (os error 16)"), "{second}");
    assert!(second.contains("процесс-писатель: Exited(Pid("), "{second}");
    assert!(second.contains("охранник прошлой паники не отпущен"), "{second}");
    assert!(!second.contains("Oops"), "{second}");
    // D — ядро уже падало, O — внешний модуль: 128 + 4096.
    assert!(out.contains("# cat /proc/sys/kernel/tainted\n4224\n"), "{out}");
}
// ANCHOR_END: oops

// ANCHOR: decode
/// Трасса первого oops через scripts/decode_stacktrace.sh: имена Rust
/// разобраны, строки — нет. В трассе адрес возврата — инструкция после
/// вызова, и скрипт ядра 7.0 отдаёт его addr2line как есть: строка
/// соседняя. Адрес на байт раньше — внутри вызова, и строка верная.
#[test]
fn oops_trace_lines() {
    let Some(out) = run(Flavor::Plain, "oops", |_| {}) else { return };
    let kernel = Kernel::or_skip(Flavor::Plain).unwrap();
    let modules = tmp("modules-plain");
    let first = out.split("# echo unwrap > /dev/ch29-oops\n").nth(1).unwrap();
    let trace: String = first
        .lines()
        .skip_while(|l| !l.ends_with("Call Trace:"))
        .take_while(|l| !l.ends_with("</TASK>"))
        .map(|l| format!("{}\n", l.strip_prefix("warn:   ").unwrap_or(l)))
        .collect();
    let file = modules.join("oops-trace.txt");
    std::fs::write(&file, &trace).unwrap();
    let root = repo_root().canonicalize().unwrap();
    let decoded = std::process::Command::new("bash")
        .arg(kernel.build.join("source/scripts/decode_stacktrace.sh"))
        .arg(kernel.build.join("vmlinux"))
        .arg(&root)
        .arg(&modules)
        .env("LLVM", "1")
        .stdin(std::fs::File::open(&file).unwrap())
        .output()
        .unwrap();
    let decoded = String::from_utf8_lossy(&decoded.stdout).into_owned();
    println!("{decoded}");
    // Строка `trigger` по версии скрипта и строка, где на самом деле unwrap.
    let shown = decoded.split(" ch29_oops::trigger (code/ch29/modules/ch29_oops.rs:").nth(1);
    let shown: usize = shown.unwrap().split(')').next().unwrap().parse().unwrap();
    let source = std::fs::read_to_string(root.join("code/ch29/modules/ch29_oops.rs")).unwrap();
    let unwrap_at = 1 + source.lines().position(|l| l.contains("found.unwrap()")).unwrap();
    assert_ne!(shown, unwrap_at, "{decoded}");
    let write_iter = "<ch29_oops::Oops as kernel::miscdevice::MiscDevice>::write_iter (";
    assert!(decoded.contains(write_iter), "{decoded}");

    // Смещение `trigger+0x157/0x174` из трассы и начало `trigger` в .ko.
    let at = trace.split("ch29_oops7trigger+0x").nth(1).unwrap();
    let off = u64::from_str_radix(at.split('/').next().unwrap(), 16).unwrap();
    let ko = modules.join("ch29_oops.ko");
    let nm = std::process::Command::new("llvm-nm").arg(&ko).output().unwrap();
    let nm = String::from_utf8(nm.stdout).unwrap();
    let base = nm.lines().find(|l| l.ends_with("ch29_oops7trigger")).unwrap();
    let base = u64::from_str_radix(base.split(' ').next().unwrap(), 16).unwrap();
    let line = |addr: u64| {
        let o = std::process::Command::new("llvm-addr2line")
            .arg("-i")
            .arg("-e")
            .arg(&ko)
            .arg(format!("{addr:#x}"))
            .output()
            .unwrap();
        String::from_utf8(o.stdout).unwrap()
    };
    let src = root.join("code/ch29/modules/ch29_oops.rs").display().to_string();
    let plain = line(base + off);
    assert!(plain.trim_end().ends_with(&format!("{src}:{shown}")), "{plain}");
    let minus1 = line(base + off - 1);
    println!("{off:#x}: {base:#x}\n{minus1}");
    assert!(minus1.trim_end().ends_with(&format!("{src}:{unwrap_at}")), "{minus1}");
    assert!(minus1.contains("core/src/option.rs:"), "{minus1}");
}
// ANCHOR_END: decode

// ANCHOR: stack
#[test]
fn stack_plain() {
    let Some(serial) = run_serial(Flavor::Plain, "stack", |_| {}) else { return };
    println!("{serial}");
    assert!(serial.contains("info:   ch29_alloc: стек: сумма 36"), "8 КиБ помещаются:\n{serial}");
    let hit = serial.lines().find(|l| l.contains("BUG: TASK stack guard page was hit at")).unwrap();
    // «stack is A..B»: стек задачи — 16 КиБ, промах — ниже его начала.
    let range = hit.split("(stack is ").nth(1).unwrap().trim_end_matches(')');
    let (lo, hi) = range.split_once("..").unwrap();
    let (lo, hi) = (u64::from_str_radix(lo, 16).unwrap(), u64::from_str_radix(hi, 16).unwrap());
    assert_eq!(hi - lo, 16 << 10, "{hit}");
    assert!(VMALLOC.contains(&lo), "{hit}");
    let at = hex_after(hit, "was hit at ");
    assert!(at < lo && lo - at <= 4096, "{hit}");
    assert!(serial.contains("_RNvCs") && serial.contains("ch29_alloc4burn"), "{serial}");
    assert!(serial.contains("Kernel panic - not syncing"), "{serial}");
    assert!(guest_part(&serial).is_none(), "после паники сценарий не продолжается");
}
// ANCHOR_END: stack

// ANCHOR: gdb
/// Сценарий команд gdb: символы ядра, скрипты ядра для gdb, остановка в
/// функции модуля, которого при подключении ещё нет.
///
/// QEMU с `-S` стоит на первой инструкции BIOS: ядра в памяти ещё нет, и
/// скрипты ядра читать нечего. Поэтому сначала — аппаратная точка
/// останова (`hbreak`; программную под KVM затёр бы распаковщик bzImage)
/// на do_init_module: модуль уже в памяти и в списке модулей, его init
/// ещё не вызван. Там `lx-symbols` подгружает символы модуля.
///
/// `lx-symbols` ищет файлы модулей в каталогах-аргументах и в текущем.
/// В ядре 7.0 вариант с аргументом падает — в scripts/gdb/linux/symbols.py
/// стоит `p.append(...)` вместо `self.module_paths.append(...)`, — поэтому
/// gdb запускается из каталога модулей, а команда — без аргументов.
///
/// Скрипты ядра вычисляют выражения на текущем языке gdb. В кадре Rust
/// имя `modules` ищется как путь Rust и не находится — `lx-lsmod` молча
/// печатает пустой список; после `set language c` список на месте.
// ANCHOR: script
fn gdb_script(kernel: &Kernel, port: u16) -> String {
    let build = kernel.build.display();
    format!(
        "set pagination off
set confirm off
add-auto-load-safe-path {build}
file {build}/vmlinux
source {build}/vmlinux-gdb.py
target remote 127.0.0.1:{port}
hbreak do_init_module
continue
delete
lx-symbols
hbreak ch29_oops::trigger
continue
echo ### bt\\n
bt
echo ### info args\\n
info args
echo ### info locals\\n
info locals
echo ### info symbol\\n
info symbol $pc
echo ### lx-lsmod\\n
lx-lsmod
echo ### show language\\n
show language
set language c
echo ### lx-lsmod, язык C\\n
lx-lsmod
echo ### конец\\n
detach
"
    )
}
// ANCHOR_END: script

#[test]
fn gdb_stops_in_module_function() {
    let Some(kernel) = Kernel::or_skip(Flavor::Plain) else { return };
    if std::process::Command::new("gdb").arg("--version").output().is_err() {
        assert!(std::env::var_os("CH29_REQUIRE_KERNEL").is_none(), "нет gdb");
        eprintln!("пропуск: нет gdb");
        return;
    }
    let initrd = image(&kernel);
    let modules = tmp("modules-plain");
    // Свободный порт: займём и отпустим.
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let mut q = Qemu::new(&kernel, &initrd, &["gdb"]);
    // -S: процессор стоит, пока отладчик не скажет continue.
    q.extra = vec!["-gdb".into(), format!("tcp:127.0.0.1:{port}"), "-S".into()];
    let script = modules.join("ch29.gdb");
    std::fs::write(&script, gdb_script(&kernel, port)).unwrap();
    let (serial, gdb) = std::thread::scope(|sc| {
        let qemu = sc.spawn(|| q.run());
        // QEMU открывает порт не сразу.
        let start = std::time::Instant::now();
        while std::net::TcpStream::connect(("127.0.0.1", port)).is_err() {
            assert!(start.elapsed().as_secs() < 30, "QEMU не открыл порт {port}");
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let gdb = std::process::Command::new("gdb")
            .current_dir(&modules)
            .args(["-batch", "-nx", "-x"])
            .arg(&script)
            .output()
            .expect("gdb");
        (qemu.join().unwrap().unwrap(), gdb)
    });
    let out =
        String::from_utf8_lossy(&gdb.stdout).into_owned() + &String::from_utf8_lossy(&gdb.stderr);
    println!("{out}");
    let body = without_console(guest_part(&serial).expect("гость не дошёл до конца"));
    println!("{body}");
    // Точка 1 — do_init_module, её удалили; вторая — в модуле.
    assert!(out.contains("hit Breakpoint 2, ch29_oops::trigger (cmd="), "{out}");
    let part = |name: &str| {
        let head = format!("### {name}\n");
        out.split(head.as_str()).nth(1).unwrap().split("### ").next().unwrap().to_string()
    };
    let bt = part("bt");
    assert!(bt.starts_with("#0  ch29_oops::trigger"), "{bt}");
    // Метод MiscDevice модуля (номер impl зависит от макросов модуля),
    // обёртка крейта kernel, потом C-часть VFS.
    let frames = ["ch29_oops::{impl#", "}::write_iter (_kiocb=", "MiscdeviceVTable", "vfs_write"];
    for f in frames {
        assert!(bt.contains(f), "нет {f}: {bt}");
    }
    // b"overflow" — срез байтов, как его печатает gdb.
    assert!(part("info args").contains("cmd = &[u8] [111, 118, 101, 114, 102, 108, 111, 119]"));
    assert!(part("info symbol").contains("in section .text of"), "{out}");
    let rows = |t: String| t.lines().skip(1).filter(|l| !l.trim().is_empty()).count();
    assert_eq!(rows(part("lx-lsmod")), 0, "{out}");
    assert!(part("show language").contains("currently rust"), "{out}");
    let lsmod = part("lx-lsmod, язык C");
    assert_eq!(rows(lsmod.clone()), 1, "{out}");
    // Первый модуль ложится в начало области модулей (KASLR выключен).
    assert!(lsmod.contains(&format!("{:#x} ch29_oops", MODULES.start)), "{lsmod}");
    // После detach модуль доделал своё: паника и oops.
    assert!(body.contains("attempt to add with overflow"), "{body}");
}
// ANCHOR_END: gdb

/// Секция `.modinfo` модулей — ровно как вывод `readelf` в тексте главы:
/// у `ch29_hello` (29.3) и `ch29_edu` (29.8), с псевдонимом из таблицы
/// устройств.
#[test]
fn modinfo_matches_text() {
    let Some(kernel) = Kernel::or_skip(Flavor::Plain) else { return };
    image(&kernel);
    let dir = tmp("modules-plain");
    for (page, module) in [("03-module.md", "ch29_hello"), ("08-hardware.md", "ch29_edu")] {
        let cmd = format!("$ readelf -p .modinfo {module}.ko\n");
        let text = std::fs::read_to_string(repo_root().join("src/ch29").join(page)).unwrap();
        let (_, after) = text.split_once(&cmd).unwrap_or_else(|| panic!("{page}: нет {cmd:?}"));
        let (block, _) = after.split_once("```").unwrap();
        let out = std::process::Command::new("readelf")
            .args(["-p", ".modinfo"])
            .arg(dir.join(format!("{module}.ko")))
            .output()
            .unwrap();
        assert!(out.status.success(), "readelf: {out:?}");
        let out = String::from_utf8(out.stdout).unwrap();
        let norm = |s: &str| s.trim().lines().map(str::trim_end).collect::<Vec<_>>().join("\n");
        assert_eq!(norm(block), norm(&out), "{page}");
    }
}

/// Фрагменты исходников ядра, которые цитирует текст главы. Перед блоком
/// кода стоит `<!-- linux: путь -->` — путь в исходниках ядра или, с
/// `build/` в начале, в каталоге сборки, — и каждая непустая строка
/// блока, кроме `…`, обязана найтись в этом файле.
#[test]
fn quoted_kernel_sources_match() {
    let Some(kernel) = Kernel::or_skip(Flavor::Plain) else { return };
    let mut checked = 0;
    let mut pages: Vec<PathBuf> = std::fs::read_dir(repo_root().join("src/ch29"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    pages.sort();
    for page in pages {
        let text = std::fs::read_to_string(&page).unwrap();
        let mut rest = text.as_str();
        while let Some(at) = rest.find("<!-- linux: ") {
            let (file, after) = rest[at + 12..].split_once(" -->").unwrap();
            // Блок: строка с ``` и языком, тело, закрывающая ```.
            let body = after.split_once("```").unwrap().1.split_once('\n').unwrap().1;
            let (block, tail) = body.split_once("```").unwrap();
            let path = match file.strip_prefix("build/") {
                Some(f) => kernel.build.join(f),
                None => kernel.build.join("source").join(file),
            };
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {file}: {e}", page.display()));
            let lines: Vec<&str> = source.lines().map(str::trim).collect();
            for l in block.lines().map(str::trim).filter(|l| !l.is_empty() && *l != "…") {
                assert!(lines.contains(&l), "{}: в {file} нет строки {l:?}", page.display());
            }
            checked += 1;
            rest = tail;
        }
    }
    assert!(checked > 0, "в тексте главы нет цитат из ядра");
}

/// Модули из `modules-fail` не собираются, и каждый — со своей ошибкой.
/// `make -k` собирает все четыре, не останавливаясь на первой ошибке.
#[test]
fn compile_fail_modules() {
    let Some(kernel) = Kernel::or_skip(Flavor::Plain) else { return };
    let out = tmp("modules-fail");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("modules-fail");
    let log = kernel.make_modules_with(&src, &out, &["-k"]).expect_err("модули собрались");
    // `error[E0308]: …`, а через строку `--> …/файл.rs:строка:столбец`.
    let mut found = Vec::new();
    let lines: Vec<&str> = log.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        let Some(code) = l.strip_prefix("error[").and_then(|r| r.split_once(']')) else {
            continue;
        };
        let file = lines[i + 1].trim_start().strip_prefix("--> ").unwrap();
        let name = Path::new(file.split(':').next().unwrap()).file_stem().unwrap();
        found.push((name.to_str().unwrap().to_string(), code.0.to_string()));
    }
    found.sort();
    let expected = [
        ("ch29_fail_irq_cell", "E0277"),
        ("ch29_fail_pin_move", "E0277"),
        ("ch29_fail_pin_value", "E0308"),
        ("ch29_fail_racy_cell", "E0277"),
        ("ch29_fail_racy_plain", "E0594"),
    ];
    let expected: Vec<_> = expected.iter().map(|(f, c)| (f.to_string(), c.to_string())).collect();
    assert_eq!(found, expected, "{log}");
    assert!(log.contains("`PhantomPinned` cannot be unpinned"), "{log}");
    assert!(log.contains("`Cell<u64>` cannot be shared between threads safely"), "{log}");
    assert!(log.contains("cannot assign to data in dereference of `Arc<Shared>`"), "{log}");
    assert!(log.contains("required by a bound in `Handler`"), "{log}");

    // Плавающая точка: rustc соберёт, а modpost не найдёт функций soft-float.
    let out = tmp("modules-fail-float");
    let log = kernel.make_modules_with(&src.join("float"), &out, &[]).expect_err("f32 собрался");
    for sym in ["__mulsf3", "__fixunssfsi"] {
        let line = format!("ERROR: modpost: \"{sym}\" [ch29_fail_float.ko] undefined!");
        assert!(log.contains(&line), "{log}");
    }
    assert!(!log.contains("error["), "rustc ошибок не нашёл:\n{log}");

    // Регистр за концом BAR и смещение, неизвестное при компиляции:
    // проверку `build_assert!` довершает modpost.
    let out = tmp("modules-fail-mmio");
    let log = kernel.make_modules_with(&src.join("mmio"), &out, &[]).expect_err("BAR+0x100");
    for m in ["ch29_fail_mmio", "ch29_fail_mmio_var"] {
        let line = format!("ERROR: modpost: \"rust_build_error\" [{m}.ko] undefined!");
        assert!(log.contains(&line), "{log}");
    }
    assert!(!log.contains("error["), "rustc ошибок не нашёл:\n{log}");

    // То же смещение через `try_read32` — проверка при выполнении, и
    // модуль собирается.
    let var = std::fs::read_to_string(src.join("mmio/ch29_fail_mmio_var.rs")).unwrap();
    let fixed = var.replace("io.read32(offset);", "io.try_read32(offset)?;");
    assert_ne!(fixed, var);
    let dir = tmp("modules-try-read");
    std::fs::write(dir.join("ch29_fail_mmio_var.rs"), fixed).unwrap();
    std::fs::write(dir.join("Kbuild"), "obj-m += ch29_fail_mmio_var.o\n").unwrap();
    let out = tmp("modules-try-read-out");
    if let Err(log) = kernel.make_modules_with(&dir, &out, &[]) {
        panic!("try_read32 не собрался:\n{log}");
    }
    assert!(out.join("ch29_fail_mmio_var.ko").is_file());
}

/// Заготовки задач (`exercises/ch29/modules`) задач не решают, но обязаны
/// собираться — с теми же линтами ядра, что и решения.
#[test]
fn exercise_modules_build() {
    let Some(kernel) = Kernel::or_skip(Flavor::Plain) else { return };
    let out = tmp("exercises-modules");
    let src = repo_root().join("exercises/ch29/modules");
    match kernel.make_modules_with(&src, &out, &["CLIPPY=1"]) {
        Ok(log) => assert!(!log.contains("warning:"), "предупреждения:\n{log}"),
        Err(log) => panic!("заготовки не собрались:\n{log}"),
    }
    for m in ["ch29_b", "ch29_c", "ch29_d1"] {
        assert!(out.join(format!("{m}.ko")).is_file(), "нет {m}.ko");
    }
}
