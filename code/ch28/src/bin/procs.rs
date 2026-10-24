//! Раздел 28.3: опыты с процессами.
//!
//! - `procs spawn` — `Command` через posix_spawn и через fork; между
//!   запусками — `getppid` как метка для strace.
//! - `procs fork-mutex` — fork, пока другой поток держит мьютекс.
//! - `procs tied` — ребёнок с `PR_SET_PDEATHSIG`; родитель печатает его
//!   pid и выходит.
//! - `procs mask` — заблокированный в родителе SIGTERM у ребёнка.

use ch28::process::{proc_status, run_plain, spawn_tied};
use nix::sys::signal::{SigSet, Signal};
use std::os::unix::process::{CommandExt, parent_id};
use std::process::Command;
use std::sync::{Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

fn spawn_paths() {
    let _ = parent_id();
    run_plain("true").unwrap();
    let _ = parent_id();
    spawn_tied("true", &[]).unwrap().wait().unwrap();
    let _ = parent_id();
}

// ANCHOR: fork_mutex
static TABLE: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn fork_mutex() {
    let (locked, wait) = mpsc::channel();
    let holder = thread::spawn(move || {
        let mut table = TABLE.lock().unwrap();
        locked.send(()).unwrap();
        thread::sleep(Duration::from_millis(1500));
        table.push(1);
    });
    wait.recv().unwrap(); // мьютекс занят другим потоком

    use nix::unistd::{ForkResult, fork};
    // SAFETY: нарушено намеренно, чтобы показать зависание: в ребёнке
    // вызывается lock, а это не async-signal-safe.
    match unsafe { fork() }.unwrap() {
        ForkResult::Child => {
            let n = TABLE.lock().map_or(0, |t| t.len());
            // SAFETY: _exit не запускает деструкторы и atexit родителя.
            unsafe { libc::_exit(n as i32) }
        }
        ForkResult::Parent { child } => {
            let pid = child.as_raw() as u32;
            thread::sleep(Duration::from_millis(300));
            let threads = proc_status(pid, "Threads").unwrap_or_default();
            let state = proc_status(pid, "State").unwrap_or_default();
            println!("ребёнок {pid}: потоков {threads}, состояние {state}");
            holder.join().unwrap();
            println!("родитель: поток отпустил мьютекс, в таблице {}", TABLE.lock().unwrap().len());
            let start = Instant::now();
            use nix::sys::wait::{WaitPidFlag, WaitStatus, waitpid};
            while start.elapsed() < Duration::from_millis(500) {
                if waitpid(child, Some(WaitPidFlag::WNOHANG)).unwrap() != WaitStatus::StillAlive {
                    println!("ребёнок вышел сам");
                    return;
                }
                thread::sleep(Duration::from_millis(20));
            }
            println!("ребёнок всё ещё ждёт мьютекс: отпустить его некому");
            nix::sys::signal::kill(child, nix::sys::signal::Signal::SIGKILL).unwrap();
            waitpid(child, None).unwrap();
        }
    }
}
// ANCHOR_END: fork_mutex

#[allow(clippy::zombie_processes)] // ребёнка не ждут нарочно: его убьёт ядро
fn tied() {
    let child = spawn_tied("sleep", &["30"]).unwrap();
    println!("{}", child.id());
    // Родитель выходит, не дожидаясь ребёнка: ядро пошлёт тому SIGKILL.
}

// ANCHOR: mask
/// Маску сигналов ребёнок наследует: std не сбрасывает её ни в
/// posix_spawn, ни в fork. Ребёнок — grep, который печатает строку SigBlk
/// из своего /proc/self/status.
fn mask() {
    SigSet::from(Signal::SIGTERM).thread_block().unwrap();
    let sigblk = |cmd: &mut Command| {
        let out = cmd.args(["SigBlk", "/proc/self/status"]).output().unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    println!("обычный Command:     {}", sigblk(&mut Command::new("grep")));
    let mut cleared = Command::new("grep");
    // SAFETY: sigprocmask — async-signal-safe системный вызов, без кучи.
    unsafe {
        cleared.pre_exec(|| {
            SigSet::empty().thread_set_mask()?;
            Ok(())
        });
    }
    println!("pre_exec с маской []: {}", sigblk(&mut cleared));
}
// ANCHOR_END: mask

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("spawn") => spawn_paths(),
        Some("fork-mutex") => fork_mutex(),
        Some("tied") => tied(),
        Some("mask") => mask(),
        _ => {
            eprintln!("использование: procs spawn | fork-mutex | tied | mask");
            std::process::exit(2);
        }
    }
}
