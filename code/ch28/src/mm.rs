//! Раздел 28.5: mmap. Отображение файла как `&[u8]`, анонимная память с
//! обычными и огромными страницами и счётчик страничных сбоев потока.

use rustix::mm::{Advice, MapFlags, ProtFlags, madvise, mmap, mmap_anonymous, munmap};
use std::fs::File;
use std::io;
use std::ops::{Deref, DerefMut};
use std::os::fd::AsFd;
use std::ptr::NonNull;

// ANCHOR: mapped
/// Отображение, которое снимается в `Drop`. Пустое отображение mmap не
/// создаёт (EINVAL), поэтому длина 0 — висячий указатель без вызова.
pub struct Mapped {
    ptr: NonNull<u8>,
    len: usize,
}

impl Mapped {
    /// Файл целиком, только для чтения.
    ///
    /// # Safety
    ///
    /// Пока отображение живо, файл не должен меняться и укорачиваться ни
    /// этим, ни другим процессом. Изменение нарушит обещание `&[u8]`, что
    /// байты не меняются, а укорачивание превратит чтение за новым концом
    /// в SIGBUS.
    pub unsafe fn file(file: &File) -> io::Result<Mapped> {
        let len = file.metadata()?.len() as usize;
        if len == 0 {
            return Ok(Mapped { ptr: NonNull::dangling(), len });
        }
        // SAFETY: новое отображение не пересекается с памятью программы;
        // неизменность файла обещал вызывающий.
        let p = unsafe {
            mmap(std::ptr::null_mut(), len, ProtFlags::READ, MapFlags::PRIVATE, file.as_fd(), 0)?
        };
        Ok(Mapped { ptr: NonNull::new(p.cast()).unwrap(), len })
    }

    /// Анонимная память, обнулённая ядром. `huge` — попросить огромные
    /// страницы (THP) через `madvise(MADV_HUGEPAGE)`; без `huge` —
    /// отказаться от них (`MADV_NOHUGEPAGE`): в режиме THP `always` ядро
    /// дало бы огромные страницы и без просьбы.
    pub fn anonymous(len: usize, huge: bool) -> io::Result<Mapped> {
        assert!(len > 0);
        // SAFETY: анонимное отображение — новая память, ни с чем не
        // пересекается.
        let p = unsafe {
            mmap_anonymous(
                std::ptr::null_mut(),
                len,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::PRIVATE,
            )?
        };
        let advice = if huge { Advice::LinuxHugepage } else { Advice::LinuxNoHugepage };
        // SAFETY: совет ядру о только что созданном отображении.
        unsafe { madvise(p, len, advice)? };
        Ok(Mapped { ptr: NonNull::new(p.cast()).unwrap(), len })
    }

    pub fn addr(&self) -> usize {
        self.ptr.as_ptr() as usize
    }
}

impl Deref for Mapped {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        // SAFETY: `len` байт по `ptr` отображены, пока жив `self`.
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl DerefMut for Mapped {
    /// Только для анонимной памяти: отображение файла — PROT_READ, и
    /// запись в него — SIGSEGV. Конструктор `file` небезопасен и в этом.
    fn deref_mut(&mut self) -> &mut [u8] {
        // SAFETY: как в `deref`, и `&mut self` — единственный доступ.
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl Drop for Mapped {
    fn drop(&mut self) {
        if self.len > 0 {
            // SAFETY: отображение создано в конструкторе и больше нигде не
            // используется: ссылки из Deref не переживают `self`.
            unsafe { munmap(self.ptr.as_ptr().cast(), self.len).ok() };
        }
    }
}
// ANCHOR_END: mapped

// ANCHOR: faults
/// Страничные сбои текущего потока: (minor, major). Minor — страница уже
/// в памяти или её дало ядро, major — пришлось читать с диска.
pub fn page_faults() -> (i64, i64) {
    use nix::sys::resource::{UsageWho, getrusage};
    let u = getrusage(UsageWho::RUSAGE_THREAD).expect("getrusage");
    (u.minor_page_faults(), u.major_page_faults())
}

/// Сколько minor-сбоев стоило `f`.
pub fn minor_faults_of<R>(f: impl FnOnce() -> R) -> (R, i64) {
    let before = page_faults().0;
    let r = f();
    (r, page_faults().0 - before)
}
// ANCHOR_END: faults

// ANCHOR: touch
/// Записать по байту в каждую страницу размера `page`.
pub fn touch(mem: &mut [u8], page: usize) {
    for i in (0..mem.len()).step_by(page) {
        // SAFETY: указатель — от `&mut` на элемент среза. volatile — чтобы
        // компилятор не выбросил запись в память, которую никто не читает.
        unsafe { std::ptr::write_volatile(&mut mem[i], 1) };
    }
}
// ANCHOR_END: touch

/// Сумма байтов с шагом страницы — чтение, которое трогает каждую
/// страницу.
pub fn sum_pages(mem: &[u8], page: usize) -> u64 {
    (0..mem.len()).step_by(page).map(|i| u64::from(mem[i])).sum()
}

/// `AnonHugePages` из /proc/self/smaps для отображения с началом `addr`,
/// в КиБ.
pub fn anon_huge_kib(addr: usize) -> Option<u64> {
    let smaps = std::fs::read_to_string("/proc/self/smaps").ok()?;
    let head = format!("{addr:x}-");
    // Поле AnonHugePages есть у каждого отображения, и первое после
    // заголовка — наше.
    smaps
        .lines()
        .skip_while(|l| !l.starts_with(&head))
        .find_map(|l| l.strip_prefix("AnonHugePages:"))
        .and_then(|v| v.trim().trim_end_matches("kB").trim().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const MIB: usize = 1 << 20;

    #[test]
    fn small_pages_fault_once_each() {
        let mut m = Mapped::anonymous(16 * MIB, false).unwrap();
        let ((), faults) = minor_faults_of(|| touch(&mut m, 4096));
        // Ровно 4096 страниц; на стек и прочее — немного сверху.
        assert!((4096..4096 + 64).contains(&faults), "{faults}");
    }

    #[test]
    fn file_mapping_reads_file() {
        let path = std::env::temp_dir().join(format!("ch28-mm-{}", std::process::id()));
        let mut f = File::create(&path).unwrap();
        f.write_all(b"hello, mmap").unwrap();
        let f = File::open(&path).unwrap();
        // SAFETY: файл наш и до конца теста не меняется.
        let m = unsafe { Mapped::file(&f) }.unwrap();
        assert_eq!(&m[..], b"hello, mmap");
        drop(m);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn empty_file_maps_to_empty_slice() {
        let f = tempfile_empty();
        // SAFETY: файл пуст и ничей.
        let m = unsafe { Mapped::file(&f) }.unwrap();
        assert!(m.is_empty());
    }

    fn tempfile_empty() -> File {
        File::from(rustix::fs::memfd_create("ch28-empty", rustix::fs::MemfdFlags::CLOEXEC).unwrap())
    }
}
