//! Init внутри виртуальной машины.
//!
//! Программа гостя — статически собранный двоичный файл, который ядро
//! запускает из initramfs как `/init`, процесс номер 1. Ни оболочки, ни
//! `insmod`, ни `mount` в образе нет: всё, что нужно опыту, программа
//! делает системными вызовами сама. Процесс 1 не должен завершаться —
//! ядро ответит паникой «Attempted to kill init!», — поэтому каждый
//! сценарий кончается [`poweroff`].

use crate::kmsg::{Record, parse_record};
use nix::errno::Errno;
use nix::kmod::{DeleteModuleFlags, ModuleInitFlags, delete_module, finit_module};
use nix::mount::{MsFlags, mount};
use nix::sys::wait::{WaitStatus, waitpid};
use nix::unistd::{ForkResult, fork};
use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::time::{Duration, Instant};

// ANCHOR: setup
/// Файловые системы, без которых опыты не обходятся: /proc, /sys,
/// devtmpfs (узлы устройств создаёт само ядро) и debugfs с tracefs.
pub fn setup() -> io::Result<()> {
    let fs = [
        ("proc", "/proc"),
        ("sysfs", "/sys"),
        ("devtmpfs", "/dev"),
        ("debugfs", "/sys/kernel/debug"),
        ("tracefs", "/sys/kernel/tracing"),
    ];
    for (kind, dir) in fs {
        fs::create_dir_all(dir)?;
        // Ядро без debugfs или tracefs ответит ENODEV — такие опыты
        // проверят это сами.
        match mount(Some(kind), dir, Some(kind), MsFlags::empty(), None::<&str>) {
            Ok(()) | Err(Errno::ENODEV) | Err(Errno::EBUSY) => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}
// ANCHOR_END: setup

// ANCHOR: insmod
/// Загружает модуль: `finit_module(2)` — тот же вызов, что делает insmod.
/// Параметры — строка `имя=значение имя=значение`, как в командной строке.
pub fn insmod(path: impl AsRef<Path>, params: &str) -> io::Result<()> {
    let file = File::open(path)?;
    let params = CString::new(params).map_err(io::Error::other)?;
    finit_module(&file, &params, ModuleInitFlags::empty())?;
    Ok(())
}

/// Выгружает модуль: `delete_module(2)`. Без O_NONBLOCK вызов ждал бы,
/// пока у модуля не кончатся пользователи.
pub fn rmmod(name: &str) -> io::Result<()> {
    let name = CString::new(name).map_err(io::Error::other)?;
    delete_module(&name, DeleteModuleFlags::O_NONBLOCK)?;
    Ok(())
}
// ANCHOR_END: insmod

// ANCHOR: kmsg
/// Читатель `/dev/kmsg`. Каждый открытый дескриптор читает журнал со
/// своей позиции: первый `drain` вернёт всё, что ядро напечатало с
/// загрузки, следующие — только новое.
pub struct Kmsg(File);

impl Kmsg {
    pub fn open() -> io::Result<Kmsg> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(nix::fcntl::OFlag::O_NONBLOCK.bits())
            .open("/dev/kmsg")?;
        Ok(Kmsg(file))
    }

    /// Все записи, которые уже есть. Неблокирующий `read` вернёт EAGAIN,
    /// когда записи кончатся, и EPIPE, если наша позиция успела
    /// вытесниться из кольцевого буфера, — тогда читаем дальше.
    pub fn drain(&mut self) -> Vec<Record> {
        let mut out = Vec::new();
        let mut buf = vec![0u8; 8192];
        loop {
            match self.0.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend(parse_record(&buf[..n])),
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.raw_os_error() == Some(nix::libc::EPIPE) => continue,
                Err(e) => panic!("/dev/kmsg: {e}"),
            }
        }
        out
    }

    /// Ждёт запись, текст которой содержит `needle`, не дольше `timeout`.
    /// Возвращает всё прочитанное за это время.
    pub fn wait_for(&mut self, needle: &str, timeout: Duration) -> Vec<Record> {
        let start = Instant::now();
        let mut seen = Vec::new();
        loop {
            let batch = self.drain();
            let found = batch.iter().any(|r| r.text.contains(needle));
            seen.extend(batch);
            if found || start.elapsed() > timeout {
                return seen;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}
// ANCHOR_END: kmsg

/// Выполняет `f` в дочернем процессе и возвращает, чем он кончился. Опыт,
/// который может уронить процесс — oops в модуле убивает текущую задачу,
/// — нельзя ставить в процессе 1: его гибель — паника ядра.
pub fn in_child(f: impl FnOnce() -> i32) -> WaitStatus {
    drain_console();
    // SAFETY: гость однопоточный — других потоков, которые держали бы
    // блокировки в момент fork, нет.
    match unsafe { fork() }.expect("fork") {
        ForkResult::Child => std::process::exit(f()),
        ForkResult::Parent { child } => waitpid(child, None).expect("waitpid"),
    }
}

/// Ждёт, пока последовательный порт допечатает вывод init. `write` в
/// консоль возвращается, как только байты легли в буфер tty, а сообщения
/// ядра идут на порт мимо этого буфера. Без ожидания отклик модуля или
/// сообщение о перезагрузке рвёт строку посередине: `### ch29: кон[
/// 6.37] ACPI: PM: …`, и `ец` — уже на следующей строке.
pub fn drain_console() {
    use std::io::Write;
    io::stdout().flush().ok();
    let _ = nix::sys::termios::tcdrain(io::stdout());
}

/// Конец сценария: сбросить буферы и перезагрузить машину. QEMU запущен с
/// `-no-reboot`, поэтому перезагрузка для него — выход.
pub fn poweroff() -> ! {
    drain_console();
    nix::unistd::sync();
    let _ = nix::sys::reboot::reboot(nix::sys::reboot::RebootMode::RB_AUTOBOOT);
    unreachable!("reboot вернулся");
}
