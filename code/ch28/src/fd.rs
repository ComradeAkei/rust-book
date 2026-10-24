//! Раздел 28.2: дескрипторы во владении. `OwnedFd` закрывает дескриптор в
//! `Drop`, `BorrowedFd<'a>` — заимствование, которое не переживёт
//! владельца, `AsFd` — «всё, у чего можно одолжить дескриптор».

use std::fs::File;
use std::io::{self, Seek, SeekFrom, Write};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

// ANCHOR: size
/// Размер файла по любому дескриптору: `&File`, `&OwnedFd`, `BorrowedFd`,
/// `io::stdin()`. Функция дескриптор только заимствует и закрыть его не
/// может.
pub fn file_size(fd: impl AsFd) -> io::Result<u64> {
    Ok(rustix::fs::fstat(fd)?.st_size as u64)
}
// ANCHOR_END: size

// ANCHOR: copy
/// Копия файла. Оба дескриптора закрывает `Drop` — на успешном пути и на
/// каждом `?`.
pub fn copy(src: &Path, dst: &Path) -> io::Result<u64> {
    let mut from = File::open(src)?;
    let mut to = File::create(dst)?;
    io::copy(&mut from, &mut to)
}
// ANCHOR_END: copy

// ANCHOR: memfd
/// Анонимный файл в памяти: дескриптор есть, имени в файловой системе
/// нет. `OwnedFd` из rustix превращается в `File` без `unsafe`.
pub fn memfd_with(name: &str, data: &[u8]) -> io::Result<File> {
    let fd: OwnedFd = rustix::fs::memfd_create(name, rustix::fs::MemfdFlags::CLOEXEC)?;
    let mut file = File::from(fd);
    file.write_all(data)?;
    file.seek(SeekFrom::Start(0))?;
    Ok(file)
}
// ANCHOR_END: memfd

// ANCHOR: stdin
/// Отдать memfd дочернему процессу как stdin: `Stdio` берёт `OwnedFd` во
/// владение, и в родителе дескриптор закроется после `spawn`.
pub fn count_bytes_in_child(file: File) -> io::Result<String> {
    let out = Command::new("wc").arg("-c").stdin(Stdio::from(OwnedFd::from(file))).output()?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}
// ANCHOR_END: stdin

// ANCHOR: link
/// Куда указывает дескриптор: ссылка `/proc/self/fd/N`. Для memfd это
/// `/memfd:имя (deleted)`.
pub fn fd_target(fd: BorrowedFd<'_>) -> io::Result<PathBuf> {
    std::fs::read_link(format!("/proc/self/fd/{}", fd.as_raw_fd()))
}
// ANCHOR_END: link

/// Цель ссылки `/proc/self/fd/N` по номеру — для проверки, что номер уже
/// закрыт или достался другому файлу.
pub fn raw_target(raw: RawFd) -> Option<PathBuf> {
    std::fs::read_link(format!("/proc/self/fd/{raw}")).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_through_any_fd() {
        let f = memfd_with("ch28-size", b"0123456789").unwrap();
        assert_eq!(file_size(&f).unwrap(), 10);
        assert_eq!(file_size(f.as_fd()).unwrap(), 10);
        let owned = OwnedFd::from(f);
        assert_eq!(file_size(&owned).unwrap(), 10);
    }

    #[test]
    fn memfd_has_no_name_but_has_link() {
        let f = memfd_with("ch28-link", b"x").unwrap();
        let target = fd_target(f.as_fd()).unwrap();
        assert_eq!(target, Path::new("/memfd:ch28-link (deleted)"));
    }

    #[test]
    fn drop_closes() {
        let f = memfd_with("ch28-drop", b"x").unwrap();
        let raw = f.as_raw_fd();
        let name = Path::new("/memfd:ch28-drop (deleted)");
        assert_eq!(raw_target(raw).as_deref(), Some(name));
        drop(f);
        // Номер может уже достаться файлу другого теста, но не нашему memfd.
        assert_ne!(raw_target(raw).as_deref(), Some(name));
    }

    #[test]
    fn child_reads_memfd() {
        let f = memfd_with("ch28-child", &[b'a'; 1000]).unwrap();
        assert_eq!(count_bytes_in_child(f).unwrap(), "1000");
    }

    #[test]
    fn copy_closes_on_error() {
        let dir = std::env::temp_dir().join(format!("ch28-copy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("src");
        std::fs::write(&src, b"hello").unwrap();
        assert_eq!(copy(&src, &dir.join("dst")).unwrap(), 5);
        let err = copy(&src, &dir.join("нет/dst")).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
