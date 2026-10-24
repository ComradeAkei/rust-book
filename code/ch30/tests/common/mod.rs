//! Общее для тестов главы 30: release-сборка программ и поиск perf и
//! valgrind. Профилировщик и симулятор кэша меряют release: в debug
//! профиль — это профиль отладочных проверок, а не программы.

#![allow(dead_code)]

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Каталог release-сборки программ главы: собирается один раз на прогон.
pub fn release_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ch30-release");
        let st = Command::new("cargo")
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_remove("RUSTFLAGS")
            .args(["build", "--release", "--bins", "--target-dir"])
            .arg(&out)
            .status()
            .unwrap();
        assert!(st.success());
        out.join("release")
    })
}

/// Путь к release-программе главы.
pub fn release_bin(name: &str) -> PathBuf {
    release_dir().join(name)
}

// ANCHOR: no_thp
/// Команда, процессу которой ядро не даст огромных страниц (THP):
/// `prctl(PR_SET_THP_DISABLE)` в дочернем процессе перед `exec`. Запрет
/// наследуется через fork и exec — действует и на программу под `perf`.
/// Числа текста сняты при THP в режиме `madvise`, где программам главы,
/// которые огромных страниц не просят, их не дают. В режиме `always` —
/// так настроена, например, машина CI книги — сбой был бы один на 2 МиБ,
/// а не на 4 КиБ, и замеры холодной памяти дали бы другие числа.
pub fn without_thp(cmd: &mut Command) -> &mut Command {
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
    }
}
// ANCHOR_END: no_thp

/// Работающая программа: `PERF` из окружения или `perf` из `PATH`, если
/// `--version` у неё проходит. В Ubuntu `/usr/bin/perf` — обёртка,
/// которая отказывается работать с ядром, для которого нет пакета
/// linux-tools; тогда путь к настоящему perf задают в `PERF`.
pub fn tool(env: &str, name: &str) -> Option<PathBuf> {
    let path = std::env::var_os(env).map_or_else(|| PathBuf::from(name), PathBuf::from);
    let ok = Command::new(&path).arg("--version").output().is_ok_and(|o| o.status.success());
    if ok {
        Some(path)
    } else {
        eprintln!("{name} не найден или не работает: опыт пропущен (путь можно задать в {env})");
        None
    }
}

/// Нужен ли инструмент обязательно: в CI пропуск опыта — ошибка.
pub fn required(var: &str) -> bool {
    std::env::var_os(var).is_some()
}
