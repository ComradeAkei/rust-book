//! Задача D: поиск строки в большом файле — через `read` с буфером и через
//! `mmap`, с обычными и с огромными страницами. Замер — программа
//! `ch28-search`.

use std::fs::File;
use std::io;

// ANCHOR: api
/// Поиск через `read` в буфер `buf_size` байт: смещение первого
/// вхождения `needle` от начала файла. Игла может лечь на границу двух
/// чтений.
pub fn find_read(file: &mut File, needle: &[u8], buf_size: usize) -> io::Result<Option<u64>> {
    let _ = (file, needle, buf_size);
    todo!("задача D: find_read")
}

/// Поиск по отображению файла. `huge` — попросить у ядра огромные
/// страницы через `madvise(MADV_HUGEPAGE)`.
///
/// # Safety
///
/// Пока идёт поиск, файл не меняют и не укорачивают.
pub unsafe fn find_mmap(file: &File, needle: &[u8], huge: bool) -> io::Result<Option<u64>> {
    let _ = (file, needle, huge);
    todo!("задача D: find_mmap")
}
// ANCHOR_END: api
