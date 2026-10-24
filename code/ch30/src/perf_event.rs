//! Раздел 30.3: счётчики `perf_event_open` своими руками — тот же
//! интерфейс ядра, через который работает `perf stat`.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

// ANCHOR: attr
/// `struct perf_event_attr` в первой версии (`PERF_ATTR_SIZE_VER0`):
/// 64 байта. Поле `size` говорит ядру, какую версию структуры ему дали:
/// поля, которых в ней нет, оно считает нулями.
#[repr(C)]
#[derive(Default)]
struct PerfEventAttr {
    kind: u32,
    size: u32,
    config: u64,
    sample_period: u64,
    sample_type: u64,
    read_format: u64,
    /// Битовые поля C в одном `u64`: младший бит — `disabled`.
    flags: u64,
    wakeup_events: u32,
    bp_type: u32,
    config1: u64,
}

const _: () = assert!(size_of::<PerfEventAttr>() == 64);

const DISABLED: u64 = 1 << 0;
const EXCLUDE_KERNEL: u64 = 1 << 5;
const EXCLUDE_HV: u64 = 1 << 6;

const PERF_TYPE_HARDWARE: u32 = 0;
const PERF_TYPE_SOFTWARE: u32 = 1;
const PERF_FLAG_FD_CLOEXEC: libc::c_ulong = 1 << 3;

// _IO('$', n) из <linux/perf_event.h>.
const IOC_ENABLE: libc::c_ulong = 0x2400;
const IOC_DISABLE: libc::c_ulong = 0x2401;
const IOC_RESET: libc::c_ulong = 0x2403;
// ANCHOR_END: attr

// ANCHOR: event
/// События, которые умеет считать [`Counter`]. Первые четыре считает PMU
/// процессора, остальные — ядро.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Cycles,
    Instructions,
    CacheMisses,
    BranchMisses,
    TaskClock,
    PageFaults,
    ContextSwitches,
}

impl Event {
    pub const ALL: [Event; 7] = [
        Event::Cycles,
        Event::Instructions,
        Event::CacheMisses,
        Event::BranchMisses,
        Event::TaskClock,
        Event::PageFaults,
        Event::ContextSwitches,
    ];

    /// Тип и номер события из <linux/perf_event.h>.
    const fn type_config(self) -> (u32, u64) {
        match self {
            Event::Cycles => (PERF_TYPE_HARDWARE, 0),
            Event::Instructions => (PERF_TYPE_HARDWARE, 1),
            Event::CacheMisses => (PERF_TYPE_HARDWARE, 3),
            Event::BranchMisses => (PERF_TYPE_HARDWARE, 5),
            Event::TaskClock => (PERF_TYPE_SOFTWARE, 1),
            Event::PageFaults => (PERF_TYPE_SOFTWARE, 2),
            Event::ContextSwitches => (PERF_TYPE_SOFTWARE, 3),
        }
    }

    /// Имя события, как его пишет `perf stat`.
    pub const fn name(self) -> &'static str {
        match self {
            Event::Cycles => "cycles",
            Event::Instructions => "instructions",
            Event::CacheMisses => "cache-misses",
            Event::BranchMisses => "branch-misses",
            Event::TaskClock => "task-clock",
            Event::PageFaults => "page-faults",
            Event::ContextSwitches => "context-switches",
        }
    }

    pub const fn is_hardware(self) -> bool {
        self.type_config().0 == PERF_TYPE_HARDWARE
    }
}
// ANCHOR_END: event

// ANCHOR: counter
/// Счётчик одного события для текущего потока, только в пространстве
/// пользователя. Дескриптор закрывается в `Drop`.
pub struct Counter {
    fd: OwnedFd,
}

impl Counter {
    /// Открыть выключенный счётчик. Без PMU аппаратное событие даёт
    /// `ENOENT`; при `kernel.perf_event_paranoid` выше 2 (так в Ubuntu
    /// по умолчанию) обычному пользователю — `EACCES` на любое событие.
    pub fn new(event: Event) -> io::Result<Counter> {
        let (kind, config) = event.type_config();
        let attr = PerfEventAttr {
            kind,
            size: size_of::<PerfEventAttr>() as u32,
            config,
            flags: DISABLED | EXCLUDE_KERNEL | EXCLUDE_HV,
            ..PerfEventAttr::default()
        };
        // SAFETY: attr — живая структура версии VER0 с верным size, ядро
        // только читает её; pid 0 и cpu −1 — «этот поток на любом
        // процессоре», group_fd −1 — без группы.
        let fd = unsafe {
            libc::syscall(
                libc::SYS_perf_event_open,
                &raw const attr,
                0 as libc::pid_t,
                -1 as libc::c_int,
                -1 as libc::c_int,
                PERF_FLAG_FD_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: успешный вызов вернул новый дескриптор, он больше ничей.
        Ok(Counter { fd: unsafe { OwnedFd::from_raw_fd(fd as libc::c_int) } })
    }

    fn ioctl(&self, request: libc::c_ulong) -> io::Result<()> {
        // SAFETY: эти три запроса perf не принимают аргумента-указателя;
        // дескриптор жив, пока жив self.
        if unsafe { libc::ioctl(self.fd.as_raw_fd(), request, 0) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Обнулить счётчик, включить, выполнить `f`, выключить и прочитать.
    pub fn count<R>(&self, f: impl FnOnce() -> R) -> io::Result<(R, u64)> {
        self.ioctl(IOC_RESET)?;
        self.ioctl(IOC_ENABLE)?;
        let r = f();
        self.ioctl(IOC_DISABLE)?;
        let mut value = 0u64;
        // SAFETY: буфер — 8 байт живого u64; без read_format ядро отдаёт
        // ровно одно значение u64.
        let n = unsafe { libc::read(self.fd.as_raw_fd(), (&raw mut value).cast(), 8) };
        match n {
            8 => Ok((r, value)),
            -1 => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("короткое чтение счётчика")),
        }
    }
}
// ANCHOR_END: counter

// ANCHOR: touch
/// Анонимное отображение на `pages` страниц без огромных страниц
/// (`MADV_NOHUGEPAGE`): запись по байту в каждую страницу — ровно один
/// страничный сбой на страницу. Возвращает сумму прочитанного обратно.
pub fn touch_fresh_pages(pages: usize) -> io::Result<u64> {
    // SAFETY: sysconf без побочных эффектов.
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
    let len = pages * page;
    // SAFETY: новое анонимное отображение, ни с чем не пересекается.
    let p = unsafe {
        libc::mmap(
            core::ptr::null_mut(),
            len,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    if p == libc::MAP_FAILED {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: [p, p + len) — наше отображение; совет ядру память не портит.
    unsafe { libc::madvise(p, len, libc::MADV_NOHUGEPAGE) };
    let base = p.cast::<u8>();
    let mut sum = 0u64;
    for i in 0..pages {
        // SAFETY: смещение i * page < len, страница доступна на запись.
        unsafe {
            base.add(i * page).write_volatile(i as u8);
            sum += u64::from(base.add(i * page).read_volatile());
        }
    }
    // SAFETY: отображение наше и больше не используется.
    unsafe { libc::munmap(p, len) };
    Ok(sum)
}
// ANCHOR_END: touch

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_perf() {
        assert_eq!(Event::ALL.iter().filter(|e| e.is_hardware()).count(), 4);
        assert_eq!(Event::PageFaults.name(), "page-faults");
    }
}
