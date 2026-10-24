//! Init гостя для опытов главы 29: `/init сценарий`.
//!
//! Ядро запускает программу процессом 1 и передаёт ей в argv всё, что в
//! командной строке ядра стоит после `--`. Каждый сценарий печатает сеанс
//! в стиле оболочки, и QEMU выходит, когда сценарий кончился.

use ch29::guest::poweroff;
use ch29::session::Session;
use std::io;

// ANCHOR: hello
fn hello(s: &mut Session) -> io::Result<()> {
    s.insmod("ch29_hello", "")?;
    s.grep("ch29", "/proc/modules");
    s.rmmod("ch29_hello")?;
    s.insmod("ch29_hello", "times=3")?;
    s.rmmod("ch29_hello")?;
    // Ошибки: значение вне диапазона, не число, параметра нет.
    for bad in ["times=11", "times=abc", "times=-1", "count=2"] {
        s.insmod("ch29_hello", bad).ok();
    }
    Ok(())
}
// ANCHOR_END: hello

// ANCHOR: errors
fn errors(s: &mut Session) -> io::Result<()> {
    for module in ["ch29_errors", "ch29_errors_c"] {
        let dev = module.replace('_', "-");
        for fail_at in 1..=4 {
            s.insmod(module, &format!("fail_at={fail_at}")).ok();
            s.exists(&format!("/dev/{dev}"));
        }
        s.insmod(module, "")?;
        s.exists(&format!("/dev/{dev}"));
        s.exists(&format!("/sys/kernel/debug/{module}"));
        s.rmmod(module)?;
    }
    Ok(())
}
// ANCHOR_END: errors

// ANCHOR: alloc
fn alloc(s: &mut Session) -> io::Result<()> {
    s.insmod("ch29_alloc", "")?;
    s.rmmod("ch29_alloc")?;
    // Отладочное ядро ответит на это сообщением BUG, обычное — молчанием.
    s.insmod("ch29_alloc", "under_lock=1")?;
    s.rmmod("ch29_alloc")
}
// ANCHOR_END: alloc

/// Глубокая рекурсия в init: стек ядра — 16 КиБ, и за ним сторожевая
/// страница. Ядро после этого не живёт, поэтому сценарий отдельный.
fn stack(s: &mut Session) -> io::Result<()> {
    s.insmod("ch29_alloc", "stack_kib=8")?;
    s.rmmod("ch29_alloc")?;
    s.insmod("ch29_alloc", "stack_kib=32")
}

fn pin(s: &mut Session) -> io::Result<()> {
    s.insmod("ch29_pin", "")?;
    s.rmmod("ch29_pin")?;
    s.insmod("ch29_pin_c", "")?;
    s.rmmod("ch29_pin_c")
}

fn sync(s: &mut Session) -> io::Result<()> {
    s.insmod("ch29_sync", "")?;
    s.rmmod("ch29_sync")?;
    s.insmod("ch29_sync_c", "")?;
    s.rmmod("ch29_sync_c")
}

// ANCHOR: notes
nix::ioctl_read!(notes_len, b'n', 1, u64);
nix::ioctl_none!(notes_clear, b'n', 2);

fn notes_session(s: &mut Session, dev: &str) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;
    let path = format!("/dev/{dev}");
    let mut a = OpenOptions::new().read(true).write(true).open(&path)?;
    let mut b = OpenOptions::new().read(true).write(true).open(&path)?;
    s.say(&format!("write(a, \"первая\\n\"): {:?}", a.write("первая\n".as_bytes())));
    s.say(&format!("write(a, \"вторая\\n\"): {:?}", a.write("вторая\n".as_bytes())));
    let mut text = String::new();
    a.read_to_string(&mut text)?;
    s.say(&format!("read(a): {text:?}"));
    let mut other = String::new();
    b.read_to_string(&mut other)?;
    s.say(&format!("read(b): {other:?} — у b свой буфер"));
    let mut len = 0u64;
    // SAFETY: fd открыт, `len` — u64, как в _IOR('n', 1, u64).
    let r = unsafe { notes_len(a.as_raw_fd(), &mut len) };
    s.say(&format!("ioctl(a, NOTES_LEN): {r:?}, длина {len}"));
    // Невалидные адреса пользователя: невыделенная страница и адрес ядра.
    for (what, addr) in [("0x1000", 0x1000usize), ("ядро", 0xffff_ffff_8100_0000)] {
        // SAFETY: системный вызов с намеренно плохим адресом — ядро обязано
        // отказать, а не писать туда.
        let r = unsafe { nix::libc::pread(a.as_raw_fd(), addr as *mut _, 16, 0) };
        let e = io::Error::last_os_error();
        s.say(&format!(
            "pread(a, {what}): {r}{}",
            if r < 0 { format!(" ({e})") } else { String::new() }
        ));
        // SAFETY: то же для записи.
        let r = unsafe { nix::libc::write(a.as_raw_fd(), addr as *const _, 16) };
        let e = io::Error::last_os_error();
        s.say(&format!(
            "write(a, {what}): {r}{}",
            if r < 0 { format!(" ({e})") } else { String::new() }
        ));
    }
    let big = vec![b'x'; 5000];
    s.say(&format!("write(b, 5000 байт): {:?}", b.write(&big).map_err(|e| e.to_string())));
    // SAFETY: fd открыт; номер, которого устройство не знает.
    let r = unsafe { nix::libc::ioctl(a.as_raw_fd(), 0x6e63) };
    s.say(&format!("ioctl(a, 0x6e63): {r} ({})", io::Error::last_os_error()));
    // SAFETY: fd открыт.
    let r = unsafe { notes_clear(a.as_raw_fd()) };
    // SAFETY: как выше.
    let r2 = unsafe { notes_len(a.as_raw_fd(), &mut len) };
    s.say(&format!("NOTES_CLEAR: {r:?}, потом NOTES_LEN: {r2:?}, длина {len}"));
    drop(a);
    drop(b);
    s.dmesg();
    Ok(())
}

fn notes(s: &mut Session) -> io::Result<()> {
    for (module, dev) in [("ch29_notes", "ch29-notes"), ("ch29_notes_c", "ch29-notes-c")] {
        s.insmod(module, "")?;
        notes_session(s, dev)?;
        s.rmmod(module)?;
    }
    // Абстракция ядра simple_read_from_buffer и плохой адрес.
    s.insmod("ch29_notes", "simple=1")?;
    notes_session(s, "ch29-notes")?;
    s.rmmod("ch29_notes")
}
// ANCHOR_END: notes

// ANCHOR: edu
fn edu(s: &mut Session) -> io::Result<()> {
    s.cat("/sys/bus/pci/devices/0000:00:03.0/vendor");
    s.cat("/sys/bus/pci/devices/0000:00:03.0/device");
    s.cat("/sys/bus/pci/devices/0000:00:03.0/enable");
    s.insmod("ch29_edu", "")?;
    s.grep("ch29_edu", "/proc/interrupts");
    s.grep("ch29_edu", "/proc/iomem");
    s.rmmod("ch29_edu")?;
    s.grep("ch29_edu", "/proc/iomem");
    s.cat("/sys/bus/pci/devices/0000:00:03.0/enable");
    Ok(())
}
// ANCHOR_END: edu

// ANCHOR: oops
fn oops(s: &mut Session) -> io::Result<()> {
    use std::io::Write;
    s.insmod("ch29_oops", "")?;
    for cmd in ["unwrap", "index", "overflow", "locked", "locked"] {
        s.say(&format!("# echo {cmd} > /dev/ch29-oops"));
        let status = ch29::guest::in_child(|| {
            let mut f = std::fs::OpenOptions::new().write(true).open("/dev/ch29-oops").unwrap();
            match f.write_all(cmd.as_bytes()) {
                Ok(()) => 0,
                Err(e) => {
                    println!("write: {e}");
                    1
                }
            }
        });
        s.say(&format!("процесс-писатель: {status:?}"));
        s.dmesg();
    }
    s.cat("/proc/sys/kernel/tainted");
    s.rmmod("ch29_oops")
}
// ANCHOR_END: oops

/// Под отладчиком: одна команда, на которой gdb остановится в `trigger`.
fn gdb(s: &mut Session) -> io::Result<()> {
    use std::io::Write;
    s.insmod("ch29_oops", "")?;
    s.say("# echo overflow > /dev/ch29-oops");
    let status = ch29::guest::in_child(|| {
        let mut f = std::fs::OpenOptions::new().write(true).open("/dev/ch29-oops").unwrap();
        i32::from(f.write_all(b"overflow").is_err())
    });
    s.say(&format!("процесс-писатель: {status:?}"));
    s.dmesg();
    Ok(())
}

fn main() {
    let mut s = Session::start().expect("сеанс");
    let scenario = std::env::args().nth(1).unwrap_or_default();
    if scenario != "hello" {
        // Сообщение о «порче» ядра внешним модулем разобрано в 29.3.
        s.hide("loading out-of-tree module taints kernel");
    }
    let result = match scenario.as_str() {
        "hello" => hello(&mut s),
        "errors" => errors(&mut s),
        "alloc" => alloc(&mut s),
        "stack" => stack(&mut s),
        "pin" => pin(&mut s),
        "sync" => sync(&mut s),
        "notes" => notes(&mut s),
        "edu" => edu(&mut s),
        "oops" => oops(&mut s),
        "gdb" => gdb(&mut s),
        other => {
            println!("нет сценария {other:?}");
            Ok(())
        }
    };
    if let Err(e) = result {
        println!("сценарий {scenario}: {e}");
    }
    println!("{}", ch29::session::END);
    poweroff();
}
