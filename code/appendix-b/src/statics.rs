//! Раздел B.1: `unsafe fn`, `static mut` и окружение процесса.

use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

// ANCHOR: unsafe_op
/// Чтение регистра по адресу.
///
/// # Safety
///
/// `p` указывает на выровненное `u32`, доступное для чтения.
pub unsafe fn read_reg(p: *const u32) -> u32 {
    // В 2024 тело `unsafe fn` безопасно по умолчанию: каждая небезопасная
    // операция требует своего блока и своего обоснования.
    // SAFETY: условия на `p` вызывающий принял по контракту функции.
    unsafe { p.read_volatile() }
}
// ANCHOR_END: unsafe_op

// ANCHOR: static_mut
/// Как было принято писать до 2024: счётчик тиков в `static mut`.
static mut LEGACY_TICKS: u32 = 0;

/// # Safety
///
/// Не вызывается одновременно из нескольких потоков или из прерывания.
pub unsafe fn legacy_tick() -> u32 {
    // Ссылка `&mut LEGACY_TICKS` в 2024 запрещена. Сырой указатель
    // разрешён: он не обещает, что значение никто не меняет.
    let p = &raw mut LEGACY_TICKS;
    // SAFETY: одновременных вызовов нет по контракту функции.
    unsafe {
        *p += 1;
        *p
    }
}

/// Как стоит писать: атомарный счётчик без `unsafe`.
pub static TICKS: AtomicU32 = AtomicU32::new(0);

pub fn tick() -> u32 {
    TICKS.fetch_add(1, Ordering::Relaxed) + 1
}
// ANCHOR_END: static_mut

// ANCHOR: env
/// Переменная окружения для дочернего процесса. Окружение текущего
/// процесса не меняется, поэтому `unsafe` не нужен.
pub fn run_with_env(var: &str, value: &str) -> std::io::Result<String> {
    let script = format!("printf %s \"${var}\"");
    let out = Command::new("sh").arg("-c").arg(script).env(var, value).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
// ANCHOR_END: env

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_through_pointer() {
        let reg = 0x5A5A_0001u32;
        // SAFETY: указатель на локальную переменную, выровнен и инициализирован.
        assert_eq!(unsafe { read_reg(&reg) }, 0x5A5A_0001);
    }

    #[test]
    fn counters() {
        // SAFETY: legacy_tick вызывается только в этом тесте и только здесь.
        let (a, b) = unsafe { (legacy_tick(), legacy_tick()) };
        assert_eq!(b, a + 1);
        let t = tick();
        assert!(t >= 1);
    }

    #[test]
    #[cfg_attr(miri, ignore = "Miri не запускает дочерние процессы")]
    fn child_environment() {
        assert_eq!(run_with_env("TELEMETRY_PORT", "4242").unwrap(), "4242");
        assert_eq!(std::env::var_os("TELEMETRY_PORT"), None);
    }
}
