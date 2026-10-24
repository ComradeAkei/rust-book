//! Раздел 28.3: процессы. `Command` запускает программу через posix_spawn,
//! пока может, и через fork и exec, когда нужен код в ребёнке до exec.

use std::io;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::process::{Child, Command, ExitStatus};

// ANCHOR: plain
/// Обычный запуск. Под strace это `clone3` с `CLONE_VM | CLONE_VFORK`:
/// glibc posix_spawn не копирует память родителя, а занимает её до exec.
pub fn run_plain(prog: &str) -> io::Result<ExitStatus> {
    Command::new(prog).status()
}
// ANCHOR_END: plain

// ANCHOR: tied
/// Ребёнок, которого ядро убьёт вместе с родителем: `PR_SET_PDEATHSIG`.
/// Метода для этого у `Command` нет, нужен `pre_exec` — и std переходит с
/// posix_spawn на fork.
pub fn spawn_tied(prog: &str, args: &[&str]) -> io::Result<Child> {
    let mut cmd = Command::new(prog);
    cmd.args(args);
    // SAFETY: замыкание выполняется в ребёнке между fork и exec, где из
    // потоков родителя остался один. prctl — системный вызов без
    // блокировок и без кучи: такой вызов здесь допустим.
    unsafe {
        cmd.pre_exec(|| {
            use rustix::process::{Signal, set_parent_process_death_signal};
            set_parent_process_death_signal(Some(Signal::KILL))?;
            Ok(())
        });
    }
    cmd.spawn()
}
// ANCHOR_END: tied

// ANCHOR: describe
/// Чем кончился процесс: кодом выхода или сигналом.
pub fn describe(status: ExitStatus) -> String {
    if let Some(code) = status.code() {
        return format!("код {code}");
    }
    match status.signal() {
        Some(sig) => {
            let name = nix::sys::signal::Signal::try_from(sig).map_or("?", |s| s.as_str());
            let core = if status.core_dumped() { ", core" } else { "" };
            format!("сигнал {name}{core}")
        }
        None => "остановлен".into(),
    }
}
// ANCHOR_END: describe

/// Строка поля `name:` из `/proc/<pid>/status`, например `State` или
/// `Threads`.
pub fn proc_status(pid: u32, name: &str) -> Option<String> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let prefix = format!("{name}:");
    text.lines().find_map(|l| l.strip_prefix(&prefix)).map(|v| v.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_and_signal() {
        assert_eq!(describe(run_plain("true").unwrap()), "код 0");
        assert_eq!(describe(run_plain("false").unwrap()), "код 1");
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        child.kill().unwrap();
        assert_eq!(describe(child.wait().unwrap()), "сигнал SIGKILL");
    }

    #[test]
    fn missing_program_is_an_error_not_a_status() {
        let err = run_plain("/нет/такой/программы").unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn tied_child_runs() {
        let mut child = spawn_tied("true", &[]).unwrap();
        assert!(child.wait().unwrap().success());
    }
}
