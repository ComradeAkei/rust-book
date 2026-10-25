// Общие тесты задач главы 28.
//
// Файл не является крейтом: его подключают через `include!` тесты из
// `exercises/ch28/tests/ch28.rs` и `solutions/ch28/tests/ch28.rs`, заранее
// объявив `use <крейт> as sut;`.
//
// Задача A запускает программу `ch28-ops` под strace (без strace тест
// пропускается), B сравнивает вывод с GNU ls, C запускает оболочку
// `ch28-shell` со сценарием на stdin и на псевдотерминале, D сверяет
// поиск через read и mmap с наивным поиском.

use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Каталог для файлов, которые тест создаёт сам; пустой.
fn tmp(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("{}-{name}", env!("CARGO_PKG_NAME")));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ---------------------------------------------------------------- A

fn has_strace() -> bool {
    Command::new("strace").arg("-V").output().is_ok_and(|o| o.status.success())
}

/// Имена вызовов каждой операции `ch28-ops`: журнал `strace -f`, поделённый
/// метками `getppid`. Строки `<... x resumed>` повторяют уже учтённый
/// вызов, строки `---` — доставка сигнала. Программа запускается один раз
/// на все тесты.
fn observed() -> &'static [Vec<String>] {
    static OPS: std::sync::OnceLock<Vec<Vec<String>>> = std::sync::OnceLock::new();
    OPS.get_or_init(trace_ops)
}

fn trace_ops() -> Vec<Vec<String>> {
    let log = tmp("a").join("ops.strace");
    let status = Command::new("strace")
        .args(["-f", "-qq", "-o"])
        .arg(&log)
        .arg(env!("CARGO_BIN_EXE_ch28-ops"))
        .stdout(Stdio::piped())
        .status()
        .unwrap();
    assert!(status.success());
    let mut ops: Vec<Vec<String>> = Vec::new();
    for line in std::fs::read_to_string(&log).unwrap().lines() {
        let call = line.split_once(' ').map_or(line, |(_pid, rest)| rest).trim_start();
        if call.starts_with("<...") || call.starts_with("---") || call.starts_with("+++") {
            continue;
        }
        let name = call.split('(').next().unwrap();
        if name == "getppid" {
            ops.push(Vec::new());
        } else if let Some(op) = ops.last_mut() {
            op.push(name.to_owned());
        }
    }
    // Шесть операций между семью метками; после последней — выход.
    assert_eq!(ops.len(), 7, "{ops:?}");
    ops.truncate(6);
    ops
}

/// Без этих вызовов операция невозможна — их надо предсказать.
const KEY_CALLS: [&[&str]; 6] =
    [&["write"], &["openat"], &["read"], &["clone3", "execve", "wait4"], &["clone3", "futex"], &[]];

#[test]
fn a_predictions_match_strace() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    let seen = observed();
    for (k, &op) in sut::a_strace::OPS.iter().enumerate() {
        let predicted = sut::a_strace::predict(op);
        for call in predicted {
            assert!(seen[k].iter().any(|s| s == call), "{op:?}: предсказан {call}, а в журнале {:?}", seen[k]);
        }
        for key in KEY_CALLS[k] {
            assert!(predicted.contains(key), "{op:?}: не предсказан {key}; в журнале {:?}", seen[k]);
        }
    }
}

#[test]
fn a_instant_now_makes_no_syscalls() {
    if !has_strace() {
        return eprintln!("strace не найден — пропуск");
    }
    assert_eq!(observed()[5], Vec::<String>::new());
}

// ---------------------------------------------------------------- B

/// Каталог со всеми видами записей задачи: файлы с разными правами и
/// размерами, жёсткая ссылка, подкаталоги, символические ссылки, FIFO и
/// имя с пробелом. Время изменения у всех — заданное, у `.` и `..` тоже.
fn b_tree() -> PathBuf {
    let root = tmp("b");
    let dir = root.join("listed");
    std::fs::create_dir(&dir).unwrap();
    let file = |name: &str, len: usize, mode: u32| {
        let p = dir.join(name);
        std::fs::write(&p, vec![b'x'; len]).unwrap();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    file("a.txt", 5, 0o644);
    file(".hidden", 0, 0o600);
    file("Zeta", 1000, 0o640);
    file("run", 123_456, 0o4755);
    file("с пробелом", 3, 0o664);
    std::fs::hard_link(dir.join("a.txt"), dir.join("hard")).unwrap();
    for (name, mode) in [("bin", 0o755), ("shared", 0o1777), ("private", 0o700)] {
        std::fs::create_dir(dir.join(name)).unwrap();
        std::fs::set_permissions(dir.join(name), std::fs::Permissions::from_mode(mode)).unwrap();
    }
    std::os::unix::fs::symlink("a.txt", dir.join("link")).unwrap();
    std::os::unix::fs::symlink("нет такого", dir.join("dangling")).unwrap();
    rustix::fs::mkfifoat(rustix::fs::CWD, dir.join("fifo"), rustix::fs::Mode::from_raw_mode(0o644)).unwrap();

    // 2024-02-29 13:37 UTC и дальше по 1 ч 1 мин; каталоги — последними,
    // потому что запись в каталог меняет его mtime.
    let names = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().path()).collect::<Vec<_>>();
    for (i, p) in names.iter().chain([&dir, &root]).enumerate() {
        let t = rustix::fs::Timespec { tv_sec: 1_709_213_820 + 3660 * i as i64, tv_nsec: 0 };
        let times = rustix::fs::Timestamps { last_access: t, last_modification: t };
        rustix::fs::utimensat(rustix::fs::CWD, p, &times, rustix::fs::AtFlags::SYMLINK_NOFOLLOW).unwrap();
    }
    dir
}

#[test]
fn b_ls_matches_gnu_ls() {
    let dir = b_tree();
    let ls = Command::new("ls")
        .args(["-la", "--time-style=+%Y-%m-%d %H:%M"])
        .arg(&dir)
        .env("LC_ALL", "C")
        .env("TZ", "UTC")
        .output()
        .unwrap();
    assert!(ls.status.success());
    let expected = String::from_utf8(ls.stdout).unwrap();
    let ours = sut::b_ls::list(&dir).unwrap();
    assert_eq!(ours, expected, "\nнаш вывод:\n{ours}\nls:\n{expected}");
}

#[test]
fn b_ls_of_missing_dir_is_an_error() {
    let e = sut::b_ls::list(&tmp("b-missing").join("нет")).unwrap_err();
    assert_eq!(e.kind(), std::io::ErrorKind::NotFound);
}

// ---------------------------------------------------------------- C

use sut::c_shell::{Job, ParseError, Redirect, Stage, parse};

fn stage(argv: &[&str]) -> Stage {
    Stage { argv: argv.iter().map(|s| s.to_string()).collect(), input: None, output: None }
}

#[test]
fn c_parse_pipelines_redirects_and_quotes() {
    let mut grep = stage(&["grep", "x"]);
    grep.output = Some(Redirect { path: "out".into(), append: false });
    assert_eq!(parse("ls -l | grep x > out &"), Ok(Some(Job { stages: vec![stage(&["ls", "-l"]), grep], background: true })));

    let mut cat = stage(&["cat"]);
    cat.input = Some("in".into());
    cat.output = Some(Redirect { path: "log".into(), append: true });
    assert_eq!(parse("cat<in>>log"), Ok(Some(Job { stages: vec![cat], background: false })));

    let quoted = parse(r#"echo 'a  b' "c \"d\" \n" ''"#).unwrap().unwrap();
    assert_eq!(quoted.stages[0].argv, ["echo", "a  b", r#"c "d" \n"#, ""]);
    assert_eq!(parse("   \n"), Ok(None));
}

#[test]
fn c_parse_errors() {
    assert_eq!(parse("a | | b"), Err(ParseError::EmptyStage));
    assert_eq!(parse("| a"), Err(ParseError::EmptyStage));
    assert_eq!(parse("a |"), Err(ParseError::EmptyStage));
    assert_eq!(parse("> f"), Err(ParseError::EmptyStage));
    assert_eq!(parse("a >"), Err(ParseError::MissingTarget));
    assert_eq!(parse("a < | b"), Err(ParseError::MissingTarget));
    assert_eq!(parse("a & b"), Err(ParseError::AmpersandNotLast));
    assert_eq!(parse("echo 'x"), Err(ParseError::Unterminated));
    assert_eq!(parse(r#"echo "x\"#), Err(ParseError::Unterminated));
}

/// Оболочка со сценарием на stdin.
fn shell(script: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ch28-shell"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[test]
fn c_pipelines_and_redirects() {
    let out = tmp("c-pipe").join("out");
    let out = out.display();
    let o = shell(&format!(
        "printf 'b\\na\\nc\\n' | sort | tr a-z A-Z > {out}\ncat < {out}\necho ещё >> {out}\ncat {out} | wc -l\n"
    ));
    assert_eq!(text(&o.stdout), "A\nB\nC\n4\n");
    assert_eq!(text(&o.stderr), "");
    assert_eq!(o.status.code(), Some(0));
}

#[test]
fn c_statuses_and_errors() {
    let o = shell("false\nnosuch-cmd | cat\nsh -c 'kill -TERM $$'\na | | b\n");
    let err = text(&o.stderr);
    let lines: Vec<&str> = err.lines().collect();
    assert_eq!(
        lines,
        [
            "ch28-shell: [код 1]",
            "ch28-shell: nosuch-cmd: команда не найдена",
            "ch28-shell: [сигнал SIGTERM]",
            "ch28-shell: пустая команда",
        ]
    );
    // Код оболочки — код последней строки: ошибка разбора, 2.
    assert_eq!(o.status.code(), Some(2));
    let o = shell("sh -c 'exit 5'\n");
    assert_eq!(o.status.code(), Some(5));
    let o = shell("exit 7\necho не выполнится\n");
    assert_eq!((o.status.code(), text(&o.stdout)), (Some(7), String::new()));
}

#[test]
fn c_builtin_cd_changes_the_shell_itself() {
    // pwd печатает путь без символических ссылок: target бывает ссылкой.
    let dir = std::fs::canonicalize(tmp("c-cd")).unwrap();
    let o = shell(&format!("cd {}\npwd\ncd {}/нет\n", dir.display(), dir.display()));
    assert_eq!(text(&o.stdout), format!("{}\n", dir.display()));
    assert!(text(&o.stderr).starts_with("ch28-shell: cd: "), "{}", text(&o.stderr));
}

#[test]
fn c_background_job_does_not_block() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_ch28-shell"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    child.stdin.take().unwrap().write_all("sleep 5 &\necho сразу\n".as_bytes()).unwrap();
    let mut err = BufReader::new(child.stderr.take().unwrap());
    let mut first = String::new();
    err.read_line(&mut first).unwrap();
    let pgid: i32 = first.trim().trim_start_matches('[').trim_end_matches(']').parse().unwrap_or_else(|_| panic!("{first}"));
    // Оболочка выходит сразу. Каналы stdout и stderr при этом открыты:
    // их унаследовал sleep, и читатель увидит конец потока только после
    // него — поэтому ждём процесс, а не wait_with_output.
    assert!(child.wait().unwrap().success());
    assert!(start.elapsed() < Duration::from_secs(3), "оболочка ждала фоновое задание");
    // Фоновое задание — своя группа: её можно убить целиком.
    nix::sys::signal::killpg(nix::unistd::Pid::from_raw(pgid), nix::sys::signal::Signal::SIGKILL).unwrap();
    let mut out = String::new();
    child.stdout.take().unwrap().read_to_string(&mut out).unwrap();
    assert_eq!(out, "сразу\n");
}

/// Дети процесса `pid` по /proc/*/stat: (pid, имя).
fn children_of(pid: u32) -> Vec<(u32, String)> {
    let mut kids = Vec::new();
    for entry in std::fs::read_dir("/proc").unwrap().flatten() {
        let Ok(stat) = std::fs::read_to_string(entry.path().join("stat")) else { continue };
        // pid (имя) состояние ppid ...: имя может содержать пробелы и
        // скобки, поэтому — по последней `)`.
        let (Some(open), Some(close)) = (stat.find('('), stat.rfind(')')) else { continue };
        let mut rest = stat[close + 2..].split(' ');
        let ppid = rest.nth(1).and_then(|p| p.parse::<u32>().ok());
        if ppid == Some(pid) {
            kids.push((stat[..open].trim().parse().unwrap(), stat[open + 1..close].to_owned()));
        }
    }
    kids
}

/// Ведущая сторона псевдотерминала: что напечатала оболочка.
struct Term {
    master: OwnedFd,
    seen: String,
}

impl Term {
    fn send(&mut self, s: &str) {
        rustix::io::write(&self.master, s.as_bytes()).unwrap();
    }

    /// Читать, пока в выводе не появится `want`; вывод до него и сам
    /// `want` забываются.
    fn expect(&mut self, want: &str) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !self.seen.contains(want) {
            let left = deadline.saturating_duration_since(Instant::now());
            assert!(!left.is_zero(), "нет «{want}» в выводе терминала: {:?}", self.seen);
            let mut fds = [rustix::event::PollFd::new(&self.master, rustix::event::PollFlags::IN)];
            let timeout = rustix::event::Timespec::try_from(left).unwrap();
            if rustix::event::poll(&mut fds, Some(&timeout)).unwrap() == 0 {
                continue;
            }
            let mut buf = [0u8; 1024];
            // EIO: ведомая сторона закрыта — оболочка вышла.
            let n = rustix::io::read(&self.master, &mut buf).unwrap_or(0);
            assert!(n > 0, "терминал закрыт, нет «{want}»: {:?}", self.seen);
            self.seen += &String::from_utf8_lossy(&buf[..n]);
        }
        let end = self.seen.find(want).unwrap() + want.len();
        self.seen.drain(..end);
    }
}

#[test]
fn c_ctrl_c_interrupts_only_the_job() {
    use rustix::pty::{OpenptFlags, grantpt, openpt, ptsname, unlockpt};
    let master = openpt(OpenptFlags::RDWR | OpenptFlags::NOCTTY | OpenptFlags::CLOEXEC).unwrap();
    grantpt(&master).unwrap();
    unlockpt(&master).unwrap();
    let name = ptsname(&master, Vec::new()).unwrap();
    let slave = File::options()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY)
        .open(OsStr::from_bytes(name.as_bytes()))
        .unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_ch28-shell"));
    cmd.stdin(slave.try_clone().unwrap()).stdout(slave.try_clone().unwrap()).stderr(slave);
    // SAFETY: setsid и ioctl — async-signal-safe системные вызовы.
    unsafe {
        cmd.pre_exec(|| {
            // Новый сеанс, и терминал становится управляющим: теперь
            // Ctrl-C на нём шлёт SIGINT группе переднего плана.
            rustix::process::setsid()?;
            rustix::process::ioctl_tiocsctty(rustix::stdio::stdin())?;
            Ok(())
        })
    };
    let mut shell = cmd.spawn().unwrap();
    drop(cmd);
    let mut term = Term { master, seen: String::new() };

    term.expect("$ ");
    term.send("sleep 30\n");
    let start = Instant::now();
    while !children_of(shell.id()).iter().any(|(_, name)| name == "sleep") {
        assert!(start.elapsed() < Duration::from_secs(10), "sleep не запустился");
        std::thread::sleep(Duration::from_millis(10));
    }
    term.send("\x03");
    term.expect("ch28-shell: [сигнал SIGINT]");
    term.expect("$ ");
    // Оболочка пережила Ctrl-C и снова владеет терминалом.
    term.send("echo жив\n");
    term.expect("жив\r\n");
    term.expect("$ ");
    term.send("exit 3\n");
    assert_eq!(shell.wait().unwrap().code(), Some(3));
}

// ---------------------------------------------------------------- D

use sut::d_search::{find_mmap, find_read};

/// Детерминированные байты из маленького алфавита: в них много почти
/// совпадений с иглой.
fn d_data(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            b"abc"[(x % 3) as usize]
        })
        .collect()
}

fn naive(hay: &[u8], needle: &[u8]) -> Option<u64> {
    hay.windows(needle.len()).position(|w| w == needle).map(|i| i as u64)
}

fn d_file(name: &str, data: &[u8]) -> (PathBuf, File) {
    let path = tmp(&format!("d-{name}")).join(name);
    std::fs::write(&path, data).unwrap();
    let file = File::open(&path).unwrap();
    (path, file)
}

#[test]
fn d_read_and_mmap_agree_with_naive_search() {
    let data = d_data(20_000, 0x2805);
    let (_path, mut file) = d_file("agree", &data);
    for needle in [&b"a"[..], b"cab", b"abcabc", b"aaaaaaa", b"ccccccccccccc", b"abcabcabcabcabcabc"] {
        let want = naive(&data, needle);
        for buf in [1, 2, 3, 5, 7, 64, 4096] {
            use std::io::Seek;
            file.rewind().unwrap();
            assert_eq!(find_read(&mut file, needle, buf).unwrap(), want, "{needle:?}, буфер {buf}");
        }
        for huge in [false, true] {
            // SAFETY: файл теста никто не меняет.
            assert_eq!(unsafe { find_mmap(&file, needle, huge) }.unwrap(), want, "{needle:?}");
        }
    }
}

#[test]
fn d_needle_across_buffer_boundary() {
    // Игла начинается за 3 байта до конца первого буфера в 4096 байт.
    let mut data = vec![b'.'; 10_000];
    let needle = "граница".as_bytes();
    data[4093..4093 + needle.len()].copy_from_slice(needle);
    let (_path, mut file) = d_file("boundary", &data);
    assert_eq!(find_read(&mut file, needle, 4096).unwrap(), Some(4093));
    // SAFETY: как выше.
    assert_eq!(unsafe { find_mmap(&file, needle, false) }.unwrap(), Some(4093));
    let (_path, mut file) = d_file("end", b"...xyz");
    assert_eq!(find_read(&mut file, b"xyz", 2).unwrap(), Some(3));
    // SAFETY: как выше.
    assert_eq!(unsafe { find_mmap(&file, b"xyz", false) }.unwrap(), Some(3));
}

#[test]
fn d_empty_file_and_long_needle() {
    let (_path, mut file) = d_file("empty", b"");
    assert_eq!(find_read(&mut file, b"x", 16).unwrap(), None);
    // SAFETY: как выше.
    assert_eq!(unsafe { find_mmap(&file, b"x", false) }.unwrap(), None);
    let (_path, mut file) = d_file("short", b"abc");
    assert_eq!(find_read(&mut file, b"abcd", 1).unwrap(), None);
    // SAFETY: как выше.
    assert_eq!(unsafe { find_mmap(&file, b"abcd", true) }.unwrap(), None);
}
