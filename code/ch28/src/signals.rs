//! Раздел 28.4: сигналы. Обработчик, который только поднимает флаг, и
//! signalfd, который превращает сигнал в чтение дескриптора.

use nix::sys::signal::{SaFlags, SigAction, SigHandler, SigSet, Signal, sigaction};
use nix::sys::signalfd::{SfdFlags, SignalFd};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ANCHOR: flag
static STOP: AtomicBool = AtomicBool::new(false);
static LAST: AtomicU32 = AtomicU32::new(0);

/// Обработчик выполняется посреди любой инструкции программы, в том числе
/// внутри malloc или под чужим мьютексом. Поэтому здесь только то, что
/// async-signal-safe: две атомарные записи.
extern "C" fn on_stop(sig: libc::c_int) {
    LAST.store(sig as u32, Ordering::Relaxed);
    STOP.store(true, Ordering::Relaxed);
}

/// SIGTERM и SIGINT поднимают флаг; программа проверяет его сама.
pub fn install_stop_flag() -> nix::Result<&'static AtomicBool> {
    let action = SigAction::new(SigHandler::Handler(on_stop), SaFlags::SA_RESTART, SigSet::empty());
    for sig in [Signal::SIGTERM, Signal::SIGINT] {
        // SAFETY: обработчик делает только атомарные записи.
        unsafe { sigaction(sig, &action)? };
    }
    Ok(&STOP)
}

/// Номер последнего сигнала, поднявшего флаг.
pub fn last_signal() -> u32 {
    LAST.load(Ordering::Relaxed)
}
// ANCHOR_END: flag

// ANCHOR: signalfd
/// Сигналы — в дескриптор. Сначала их надо заблокировать, иначе ядро
/// доставит их обычным способом. Маску наследуют потоки, созданные после
/// вызова, поэтому вызывать — до первого `thread::spawn`.
pub fn signal_fd(signals: &[Signal]) -> nix::Result<SignalFd> {
    let mut mask = SigSet::empty();
    for &s in signals {
        mask.add(s);
    }
    mask.thread_block()?;
    SignalFd::with_flags(&mask, SfdFlags::SFD_CLOEXEC)
}
// ANCHOR_END: signalfd
