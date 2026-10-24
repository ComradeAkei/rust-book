//! Задача D: поиск строки в большом файле — через `read` с буфером и через
//! `mmap`, с обычными и с огромными страницами. Сам поиск у всех один:
//! первый байт иглы ищется блоками по 64 байта, затем сравнивается
//! остаток.

use rustix::mm::{Advice, MapFlags, ProtFlags, madvise, mmap, munmap};
use std::fs::File;
use std::io::{self, Read};

// ANCHOR: find
/// Первое вхождение `needle` в `hay`. Блок проверяется свёрткой без
/// раннего выхода — такой цикл LLVM векторизует; блок с кандидатом
/// разбирается побайтно.
pub fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    let (&first, rest) = needle.split_first()?;
    let last_start = hay.len().checked_sub(needle.len())?;
    let mut at = 0;
    while at <= last_start {
        let block = &hay[at..(at + 64).min(last_start + 1)];
        if block.iter().fold(false, |any, &b| any | (b == first)) {
            for (i, &b) in block.iter().enumerate() {
                if b == first && hay[at + i + 1..at + i + needle.len()] == *rest {
                    return Some(at + i);
                }
            }
        }
        at += block.len();
    }
    None
}
// ANCHOR_END: find

// ANCHOR: read
/// Поиск через `read` в буфер `buf_size` байт. Игла может лечь на границу
/// двух чтений, поэтому хвост в `needle.len() - 1` байт переносится в
/// начало буфера перед следующим чтением.
pub fn find_read(file: &mut File, needle: &[u8], buf_size: usize) -> io::Result<Option<u64>> {
    assert!(!needle.is_empty() && buf_size > 0);
    let keep = needle.len() - 1;
    let mut buf = vec![0u8; keep + buf_size];
    let mut have = 0;
    let mut base = 0u64;
    loop {
        let n = match file.read(&mut buf[have..]) {
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        };
        if n == 0 {
            return Ok(None);
        }
        have += n;
        if let Some(i) = find(&buf[..have], needle) {
            return Ok(Some(base + i as u64));
        }
        let consumed = have.saturating_sub(keep);
        buf.copy_within(consumed..have, 0);
        base += consumed as u64;
        have -= consumed;
    }
}
// ANCHOR_END: read

// ANCHOR: mmap
/// Поиск по отображению файла. `huge` — `madvise(MADV_HUGEPAGE)`: ядро
/// может отдать страничный кеш файла страницами по 2 МиБ, если файловая
/// система и сборка ядра это умеют; иначе совет ничего не меняет.
///
/// # Safety
///
/// Пока идёт поиск, файл не меняют и не укорачивают: изменение нарушит
/// неизменность `&[u8]`, укорачивание — SIGBUS.
pub unsafe fn find_mmap(file: &File, needle: &[u8], huge: bool) -> io::Result<Option<u64>> {
    let len = file.metadata()?.len() as usize;
    if len == 0 {
        return Ok(None);
    }
    // SAFETY: новое отображение только для чтения; неизменность файла
    // обещал вызывающий.
    let p =
        unsafe { mmap(std::ptr::null_mut(), len, ProtFlags::READ, MapFlags::PRIVATE, file, 0)? };
    if huge {
        // Ошибка — EINVAL у ядра без THP: тогда остаются обычные страницы.
        // SAFETY: совет о своём отображении.
        unsafe { madvise(p, len, Advice::LinuxHugepage).ok() };
    }
    // SAFETY: `len` байт отображены до munmap ниже.
    let hay = unsafe { std::slice::from_raw_parts(p.cast::<u8>(), len) };
    let found = find(hay, needle).map(|i| i as u64);
    // SAFETY: срез больше не используется.
    unsafe { munmap(p, len)? };
    Ok(found)
}
// ANCHOR_END: mmap
