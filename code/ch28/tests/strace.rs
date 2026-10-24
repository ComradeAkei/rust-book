//! Разделы 28.1–28.3 под strace: какие системные вызовы делает программа
//! на каждом уровне — std, libc, rustix, nix — и что std делает до `main`
//! и внутри `Command`. Без strace тесты пропускаются; в CI его ставит
//! задание с тестами.

use std::path::{Path, PathBuf};
use std::process::Command;

fn has_strace() -> bool {
    Command::new("strace").arg("-V").output().is_ok_and(|o| o.status.success())
}

// ANCHOR: strace
/// Строки strace без номеров процессов: `-f` следит за детьми, `-qq`
/// убирает сообщения о подключении и выходе.
fn strace(name: &str, calls: &str, exe: &str, args: &[&str]) -> Vec<String> {
    let log: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("ch28-{name}.strace"));
    let status = Command::new("strace")
        .args(["-f", "-qq", "-o"])
        .arg(&log)
        .args(["-e", &format!("trace={calls}")])
        .arg(exe)
        .args(args)
        .output()
        .unwrap()
        .status;
    assert!(status.code().is_some(), "{name}: {status}");
    std::fs::read_to_string(&log)
        .unwrap()
        .lines()
        .map(|l| l.split_once(' ').map_or(l, |(_pid, rest)| rest).trim_start().to_owned())
        .collect()
}
// ANCHOR_END: strace

/// Путь, как его печатает strace: байты вне ASCII — восьмеричными
/// escape-последовательностями.
fn escaped(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii() { (b as char).to_string() } else { format!("\\{b:03o}") })
        .collect()
}

/// Номер первой строки, начинающейся с `prefix`, после строки `from`.
fn find(lines: &[String], from: usize, prefix: &str) -> usize {
    lines[from..]
        .iter()
        .position(|l| l.starts_with(prefix))
        .map(|i| from + i)
        .unwrap_or_else(|| panic!("нет «{prefix}» после строки {from}:\n{}", lines.join("\n")))
}

#[test]
fn std_runtime_before_main() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    // `procs` без аргументов печатает подсказку в stderr: первый write —
    // уже код main.
    let calls = "poll,rt_sigaction,sigaltstack,openat,write";
    let lines = strace("runtime", calls, env!("CARGO_BIN_EXE_procs"), &[]);
    let poll =
        find(&lines, 0, "poll([{fd=0, events=0}, {fd=1, events=0}, {fd=2, events=0}], 3, 0)");
    let pipe = find(&lines, poll, "rt_sigaction(SIGPIPE, {sa_handler=SIG_IGN");
    let maps = find(&lines, pipe, "openat(AT_FDCWD, \"/proc/self/maps\"");
    let alt = find(&lines, maps, "sigaltstack({ss_sp=0x");
    let segv = find(&lines, alt, "rt_sigaction(SIGSEGV, {sa_handler=0x");
    let bus = find(&lines, segv, "rt_sigaction(SIGBUS, {sa_handler=0x");
    let main = find(&lines, 0, "write(2, ");
    assert!(bus < main);
}

#[test]
fn closed_stdout_is_reopened_as_dev_null() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    use std::os::unix::process::CommandExt;
    let log = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch28-closed-stdout.strace");
    let mut cmd = Command::new("strace");
    cmd.args(["-qq", "-e", "trace=poll,openat,write", "-o"])
        .arg(&log)
        .arg(env!("CARGO_BIN_EXE_layers"));
    // SAFETY: close — async-signal-safe системный вызов.
    unsafe {
        cmd.pre_exec(|| {
            libc::close(1);
            Ok(())
        })
    };
    assert!(cmd.status().unwrap().success());
    let text = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let poll = lines.iter().position(|l| l.starts_with("poll(")).unwrap();
    assert!(lines[poll].ends_with("= 1 ([{fd=1, revents=POLLNVAL}])"), "{text}");
    assert!(lines[poll + 1].starts_with("openat(AT_FDCWD, \"/dev/null\", O_RDWR)"), "{text}");
    assert!(lines[poll + 1].ends_with("= 1"), "{text}");
    assert!(text.contains("write(1, \"std\\n\", 4)"), "{text}");
}

#[test]
fn one_write_per_layer_and_two_ways_to_open() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    let lines = strace("layers", "write,open,openat", env!("CARGO_BIN_EXE_layers"), &[]);
    let writes: Vec<&String> =
        lines.iter().filter(|l| l.starts_with("write(1, ")).take(4).collect();
    assert_eq!(writes.len(), 4);
    for (w, layer) in writes.iter().zip(["std", "libc", "rustix", "nix"]) {
        assert!(w.starts_with(&format!("write(1, \"{layer}\\n\", ")), "{w}");
    }
    // Открытие несуществующего файла: std, libc и nix — openat, rustix —
    // старый open с O_LARGEFILE.
    let path = escaped("/нет/такого/файла");
    let opens: Vec<&String> = lines.iter().filter(|l| l.contains(&path)).collect();
    let kinds: Vec<&str> = opens.iter().map(|l| l.split('(').next().unwrap()).collect();
    assert_eq!(kinds, ["openat", "openat", "open", "openat"], "{opens:#?}");
    assert!(opens[2].contains("O_LARGEFILE"), "{}", opens[2]);
}

#[test]
fn copy_uses_copy_file_range_and_closes_on_error() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"));
    let src = dir.join("ch28-copy-src");
    std::fs::write(&src, vec![b'x'; 10000]).unwrap();
    let dst = dir.join("ch28-copy-dst");
    let exe = env!("CARGO_BIN_EXE_fds");
    let lines = strace(
        "copy",
        "openat,copy_file_range,close",
        exe,
        &["copy", src.to_str().unwrap(), dst.to_str().unwrap()],
    );
    let at = find(&lines, 0, &format!("openat(AT_FDCWD, \"{}\"", src.display()));
    let copy = find(&lines, at, "copy_file_range(3, NULL, 4, NULL, ");
    find(&lines, copy, "close(4)");
    find(&lines, copy, "close(3)");

    // Каталога нет: второй open падает, первый дескриптор закрывает Drop.
    let missing = dir.join("нет-каталога").join("dst");
    let lines = strace(
        "copy-err",
        "openat,close",
        exe,
        &["copy", src.to_str().unwrap(), missing.to_str().unwrap()],
    );
    let at = find(&lines, 0, &format!("openat(AT_FDCWD, \"{}\"", src.display()));
    let failed = find(&lines, at + 1, "openat(AT_FDCWD, ");
    assert!(
        lines[failed].ends_with("= -1 ENOENT (No such file or directory)"),
        "{}",
        lines[failed]
    );
    find(&lines, failed, "close(3)");
}

#[test]
fn owned_fd_drop_checks_the_number_in_debug_builds() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    // Свой запуск, без `strace()`: отладочная сборка кончится SIGABRT, и
    // strace завершится тем же сигналом, без кода выхода.
    let log = Path::new(env!("CARGO_TARGET_TMPDIR")).join("ch28-double-close.strace");
    Command::new("strace")
        .args(["-qq", "-e", "trace=fcntl,close", "-o"])
        .arg(&log)
        .arg(env!("CARGO_BIN_EXE_fds"))
        .arg("double-close")
        .output()
        .unwrap();
    let text = std::fs::read_to_string(&log).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with("---")).collect();
    let tail = &lines[lines.len().saturating_sub(4)..];
    if cfg!(debug_assertions) {
        // Перед каждым close — проверка, что номер открыт; второй раз
        // она находит EBADF, и std завершает процесс до второго close.
        assert!(tail[0].starts_with("fcntl(") && tail[0].contains(", F_GETFD)"), "{text}");
        assert!(tail[1].starts_with("close("), "{text}");
        assert!(tail[2].ends_with("= -1 EBADF (Bad file descriptor)"), "{text}");
        assert!(tail[3].starts_with("+++ killed by SIGABRT"), "{text}");
    } else {
        // Без проверки: второй close получает EBADF, и Drop его молча
        // проглатывает.
        assert!(!text.contains("F_GETFD"), "{text}");
        let closes: Vec<&&str> = lines.iter().filter(|l| l.starts_with("close(")).collect();
        let last = closes[closes.len() - 2..].to_vec();
        assert_eq!(last[0].split(')').next(), last[1].split(')').next(), "{text}");
        assert!(last[1].ends_with("= -1 EBADF (Bad file descriptor)"), "{text}");
    }
}

// ANCHOR: spawn_test
#[test]
fn command_uses_posix_spawn_unless_pre_exec() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    let calls = "getppid,clone,clone3,prctl,execve";
    let lines = strace("spawn", calls, env!("CARGO_BIN_EXE_procs"), &["spawn"]);
    // Метки getppid делят вывод: обычный Command, затем с pre_exec.
    let m1 = find(&lines, 0, "getppid()");
    let m2 = find(&lines, m1 + 1, "getppid()");
    let m3 = find(&lines, m2 + 1, "getppid()");
    let plain = find(&lines, m1, "clone3({flags=CLONE_VM|CLONE_VFORK|CLONE_CLEAR_SIGHAND");
    assert!(plain < m2);
    let forked = find(
        &lines,
        m2,
        "clone(child_stack=NULL, flags=CLONE_CHILD_CLEARTID|CLONE_CHILD_SETTID|SIGCHLD",
    );
    let prctl = find(&lines, forked, "prctl(PR_SET_PDEATHSIG, SIGKILL)");
    let exec = find(&lines, prctl, "execve(");
    assert!(exec < m3);
}
// ANCHOR_END: spawn_test
