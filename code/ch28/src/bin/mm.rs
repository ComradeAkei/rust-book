//! Раздел 28.5: опыты с mmap.
//!
//! - `mm faults` — сколько страничных сбоев стоит первое касание памяти:
//!   анонимная память с обычными и огромными страницами, файл.
//! - `mm sigbus` — отображённый файл укоротили.
//! - `mm shared` — memfd с MAP_SHARED у двух процессов.
//! - `mm shared-child` — сторона ребёнка: stdin — тот же memfd.

use ch28::mm::{Mapped, anon_huge_kib, minor_faults_of, sum_pages, touch};
use rustix::mm::{MapFlags, ProtFlags, mmap, munmap};
use std::fs::File;
use std::io::Write;
use std::os::fd::AsFd;

const MIB: usize = 1 << 20;

// ANCHOR: faults
fn faults() {
    let len = 64 * MIB;
    let mut small = Mapped::anonymous(len, false).unwrap();
    let ((), n) = minor_faults_of(|| touch(&mut small, 4096));
    println!("анонимная, 4 КиБ:  {n:5} сбоев на {} страниц", len / 4096);

    let mut huge = Mapped::anonymous(len, true).unwrap();
    let ((), n) = minor_faults_of(|| touch(&mut huge, 4096));
    let thp = anon_huge_kib(huge.addr()).unwrap_or(0);
    println!("анонимная, THP:    {n:5} сбоев, AnonHugePages {} МиБ", thp / 1024);

    let path = std::env::temp_dir().join(format!("ch28-faults-{}", std::process::id()));
    File::create(&path).unwrap().write_all(&vec![7u8; len]).unwrap();
    let file = File::open(&path).unwrap();
    // SAFETY: файл создан здесь и до конца опыта не меняется.
    let mapped = unsafe { Mapped::file(&file) }.unwrap();
    let (sum, n) = minor_faults_of(|| sum_pages(&mapped, 4096));
    assert_eq!(sum, 7 * (len / 4096) as u64);
    println!("файл, чтение:      {n:5} сбоев на {} страниц", len / 4096);

    // SAFETY: как выше; MAP_POPULATE заполняет таблицы страниц в самом mmap.
    let p = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            ProtFlags::READ,
            MapFlags::PRIVATE | MapFlags::POPULATE,
            file.as_fd(),
            0,
        )
        .unwrap()
    };
    // SAFETY: `len` байт отображены до munmap ниже.
    let populated = unsafe { std::slice::from_raw_parts(p.cast::<u8>(), len) };
    let (_, n) = minor_faults_of(|| sum_pages(populated, 4096));
    println!("файл, MAP_POPULATE: {n:4} сбоев");
    // SAFETY: отображение создано выше, срез дальше не используется.
    unsafe { munmap(p, len).unwrap() };
    std::fs::remove_file(&path).unwrap();
}
// ANCHOR_END: faults

// ANCHOR: sigbus
fn sigbus() {
    let path = std::env::temp_dir().join(format!("ch28-sigbus-{}", std::process::id()));
    File::create(&path).unwrap().write_all(&[1u8; 8192]).unwrap();
    let file = File::open(&path).unwrap();
    // SAFETY: обещание нарушено намеренно — файл укоротят ниже.
    let m = unsafe { Mapped::file(&file) }.unwrap();
    println!("до: m[4096] = {}", m[4096]);
    File::create(&path).unwrap(); // O_TRUNC: длина 0
    std::fs::remove_file(&path).unwrap();
    println!("после укорачивания: m[4096] = …");
    std::io::stdout().flush().unwrap();
    println!("{}", m[4096]); // SIGBUS: за концом файла страниц нет
}
// ANCHOR_END: sigbus

// ANCHOR: shared
/// Отображение memfd на `len` байт с MAP_SHARED: запись видят все, кто
/// отобразил тот же файл.
fn map_shared(fd: impl AsFd, len: usize) -> &'static mut [u8] {
    // SAFETY: новое отображение; живёт до конца процесса.
    unsafe {
        let p = mmap(
            std::ptr::null_mut(),
            len,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::SHARED,
            fd,
            0,
        )
        .unwrap();
        std::slice::from_raw_parts_mut(p.cast(), len)
    }
}

fn shared() {
    let memfd = rustix::fs::memfd_create("ch28-shared", rustix::fs::MemfdFlags::CLOEXEC).unwrap();
    rustix::fs::ftruncate(&memfd, 4096).unwrap();
    let mem = map_shared(&memfd, 4096);
    mem[0] = 41;
    let exe = std::env::current_exe().unwrap();
    let file = File::from(memfd);
    let status = std::process::Command::new(exe)
        .arg("shared-child")
        .stdin(file.try_clone().unwrap())
        .status()
        .unwrap();
    assert!(status.success());
    println!("родитель после ребёнка: mem[0] = {}", mem[0]);
}

fn shared_child() {
    let stdin = std::io::stdin();
    let mem = map_shared(stdin.as_fd(), 4096);
    println!("ребёнок видит mem[0] = {}, пишет {}", mem[0], mem[0] + 1);
    mem[0] += 1;
}
// ANCHOR_END: shared

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("faults") => faults(),
        Some("sigbus") => sigbus(),
        Some("shared") => shared(),
        Some("shared-child") => shared_child(),
        _ => {
            eprintln!("использование: mm faults | sigbus | shared");
            std::process::exit(2);
        }
    }
}
