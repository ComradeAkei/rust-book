//! Раздел 28.7: процесс смотрит на себя через /proc.
//!
//! - `procfs maps` — свои отображения: код программы, куча, именованная
//!   анонимная память, стек, vDSO.
//! - `procfs rss` — VmRSS до и после касания памяти и после munmap.
//! - `procfs cpus` — процессоры: /sys, маска потока и что видит std.
//! - `procfs sizes` — размер файлов /proc и /sys по stat и по чтению.

use ch28::mm::{Mapped, touch};
use ch28::procfs::{name_anonymous, parse_cpu_list, region_of, regions, rss_kib, status_field};
use std::fs;

// ANCHOR: maps
fn maps() {
    let journal = Mapped::anonymous(1 << 20, false).unwrap();
    if let Err(e) = name_anonymous(&journal, c"ch28-journal") {
        // EINVAL: ядро собрано без CONFIG_ANON_VMA_NAME.
        println!("имя для анонимной памяти: {e}");
    }
    let heap = Box::new(0u64);
    let on_stack = 0u8;
    let maps = fs::read_to_string("/proc/self/maps").unwrap();
    let probes = [
        ("код main", main as fn() as usize),
        ("Box", &*heap as *const u64 as usize),
        ("журнал", journal.addr()),
        ("локальная", &on_stack as *const u8 as usize),
    ];
    for (what, addr) in probes {
        let r = region_of(&maps, addr).unwrap();
        let name = r.path.map_or("(без имени)", |p| p.rsplit('/').next().unwrap());
        let perms = std::str::from_utf8(&r.perms).unwrap();
        println!("{what:10} {perms} {:6} КиБ  {name}", r.len() >> 10);
    }
    let vdso = regions(&maps).find(|r| r.path == Some("[vdso]")).unwrap();
    println!(
        "{:10} {} {:6} КиБ  [vdso]",
        "",
        std::str::from_utf8(&vdso.perms).unwrap(),
        vdso.len() >> 10
    );
    println!("всего отображений: {}", regions(&maps).count());
}
// ANCHOR_END: maps

// ANCHOR: rss
fn rss() {
    let mib = |kib: u64| kib as f64 / 1024.0;
    println!("в начале:            VmRSS {:6.1} МиБ", mib(rss_kib().unwrap()));
    let mut mem = Mapped::anonymous(64 << 20, false).unwrap();
    println!("после mmap 64 МиБ:   VmRSS {:6.1} МиБ", mib(rss_kib().unwrap()));
    touch(&mut mem, 4096);
    println!("после касания:       VmRSS {:6.1} МиБ", mib(rss_kib().unwrap()));
    drop(mem);
    println!("после munmap:        VmRSS {:6.1} МиБ", mib(rss_kib().unwrap()));
    let status = fs::read_to_string("/proc/self/status").unwrap();
    println!("пик, VmHWM:          {}", status_field(&status, "VmHWM").unwrap());
}
// ANCHOR_END: rss

// ANCHOR: cpus
fn cpus() {
    let online = fs::read_to_string("/sys/devices/system/cpu/online").unwrap();
    let online = parse_cpu_list(&online).unwrap();
    let status = fs::read_to_string("/proc/self/status").unwrap();
    let allowed = parse_cpu_list(status_field(&status, "Cpus_allowed_list").unwrap()).unwrap();
    let std_sees = std::thread::available_parallelism().unwrap();
    println!("в сети (/sys):              {online:?}");
    println!("разрешены потоку (status):  {allowed:?}");
    println!("available_parallelism:      {std_sees}");
}
// ANCHOR_END: cpus

// ANCHOR: sizes
fn sizes() {
    for path in ["/proc/self/maps", "/proc/self/status", "/sys/devices/system/cpu/online"] {
        let by_stat = fs::metadata(path).unwrap().len();
        let by_read = fs::read(path).unwrap().len();
        println!("{path:32} stat: {by_stat:5}, прочитано: {by_read:5}");
    }
}
// ANCHOR_END: sizes

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("maps") => maps(),
        Some("rss") => rss(),
        Some("cpus") => cpus(),
        Some("sizes") => sizes(),
        _ => {
            eprintln!("использование: procfs maps | rss | cpus | sizes");
            std::process::exit(2);
        }
    }
}
