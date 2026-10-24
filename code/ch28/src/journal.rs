//! Раздел 28.8: журнал кадров станции в файле, отображённом через
//! MAP_SHARED. Запись — обычная запись в память, без write: страницы
//! принадлежат страничному кешу ядра, и кадр, зафиксированный в журнале,
//! переживает даже SIGKILL процесса. Переживёт ли он отключение питания,
//! решает `msync`.

use rustix::fs::{FallocateFlags, FlockOperation, fallocate, flock};
use rustix::mm::{MapFlags, MsyncFlags, ProtFlags, mmap, msync, munmap};
use std::fs::File;
use std::io;
use std::path::Path;
use std::ptr::NonNull;
use std::sync::atomic::{AtomicU64, Ordering};
use telemetry_frame::{FrameError, Kind, encode, stream::Frames};

// ANCHOR: layout
/// Заголовок файла — 64 байта: маркер, число зафиксированных байт кадров
/// и ёмкость. Числа — little-endian, как поля кадра. Дальше — кадры
/// подряд, тот же поток, что разбирает [`Frames`] из главы 13.
pub const MAGIC: [u8; 8] = *b"TLMJRN01";
pub const HEADER: usize = 64;
const USED_AT: usize = 8;
const CAPACITY_AT: usize = 16;
// ANCHOR_END: layout

// ANCHOR: journal
pub struct Journal {
    base: NonNull<u8>,
    /// Длина отображения и файла: заголовок и ёмкость.
    len: usize,
    frames: usize,
    /// Дескриптор держит flock: второй писатель того же файла получит
    /// EWOULDBLOCK. Для отображения дескриптор не нужен — оно держит
    /// файл само.
    _lock: File,
}

impl Journal {
    /// Открыть журнал или создать новый на `capacity` байт кадров.
    ///
    /// # Safety
    ///
    /// Пока журнал открыт, файл не укорачивают и не пишут в обход
    /// журнала. flock от второго журнала защищает, но он рекомендательный:
    /// `truncate` или `dd` его не спросят. Укороченный файл — SIGBUS при
    /// записи, чужая запись — гонка данных с нашими `&mut [u8]`.
    pub unsafe fn open(path: &Path, capacity: usize) -> io::Result<Journal> {
        let file =
            File::options().read(true).write(true).create(true).truncate(false).open(path)?;
        flock(&file, FlockOperation::NonBlockingLockExclusive)?;
        let len = HEADER + capacity;
        let fresh = file.metadata()?.len() == 0;
        if fresh {
            // Блоки на диске — сразу. Без этого файл разреженный, и на
            // полном диске запись в страницу отображения кончится SIGBUS,
            // а не ошибкой ENOSPC.
            fallocate(&file, FallocateFlags::empty(), 0, len as u64)?;
        } else if file.metadata()?.len() != len as u64 {
            return Err(invalid("размер файла не совпадает с ёмкостью журнала"));
        }
        // SAFETY: новое отображение ни с чем не пересекается; файл длиной
        // `len`, и укорачивать его не будут — обещание вызывающего.
        let p = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                ProtFlags::READ | ProtFlags::WRITE,
                MapFlags::SHARED,
                &file,
                0,
            )?
        };
        let mut j = Journal { base: NonNull::new(p.cast()).unwrap(), len, frames: 0, _lock: file };
        if fresh {
            j.header_mut()[..8].copy_from_slice(&MAGIC);
            j.header_mut()[CAPACITY_AT..CAPACITY_AT + 8]
                .copy_from_slice(&(capacity as u64).to_le_bytes());
        } else {
            j.check(capacity)?;
        }
        Ok(j)
    }

    /// Проверить заголовок чужого журнала и посчитать его кадры.
    fn check(&mut self, capacity: usize) -> io::Result<()> {
        let h = self.header_mut();
        let stored_capacity =
            u64::from_le_bytes(h[CAPACITY_AT..CAPACITY_AT + 8].try_into().unwrap());
        if h[..8] != MAGIC || stored_capacity != capacity as u64 {
            return Err(invalid("не журнал кадров или другая ёмкость"));
        }
        if self.used() > capacity {
            return Err(invalid("счётчик байт больше ёмкости"));
        }
        let mut frames = 0;
        for item in Frames::until_eof(self.committed()) {
            item.map_err(|e| invalid(&format!("испорченный кадр: {e}")))?;
            frames += 1;
        }
        self.frames = frames;
        Ok(())
    }

    fn header_mut(&mut self) -> &mut [u8] {
        // SAFETY: первые HEADER байт отображения; `&mut self` — единственный
        // доступ из этого процесса.
        unsafe { std::slice::from_raw_parts_mut(self.base.as_ptr(), HEADER) }
    }

    /// Счётчик зафиксированных байт. Атомарный: его читает и другой
    /// процесс, отобразивший тот же файл.
    fn used_counter(&self) -> &AtomicU64 {
        // SAFETY: смещение 8 от начала страницы выровнено на 8; к этим
        // байтам обращаются только как к AtomicU64.
        unsafe { &*self.base.as_ptr().add(USED_AT).cast::<AtomicU64>() }
    }

    fn used(&self) -> usize {
        u64::from_le(self.used_counter().load(Ordering::Acquire)) as usize
    }

    /// Байты зафиксированных кадров.
    pub fn committed(&self) -> &[u8] {
        // SAFETY: `used` байт после заголовка записаны и больше не
        // меняются: журнал только дописывается.
        unsafe { std::slice::from_raw_parts(self.base.as_ptr().add(HEADER), self.used()) }
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn capacity(&self) -> usize {
        self.len - HEADER
    }
}
// ANCHOR_END: journal

// ANCHOR: push
impl Journal {
    /// Дописать кадр. Номер кадра — счётчик кадров по модулю 256.
    pub fn push(&mut self, kind: Kind, payload: &[u8]) -> io::Result<u8> {
        let at = self.used();
        let seq = self.frames as u8;
        // SAFETY: незафиксированный хвост после `at` — только наш.
        let free = unsafe {
            std::slice::from_raw_parts_mut(
                self.base.as_ptr().add(HEADER + at),
                self.capacity() - at,
            )
        };
        let n = encode(kind, seq, payload, free).map_err(|e| match e {
            FrameError::BufferTooSmall { .. } => {
                io::Error::new(io::ErrorKind::StorageFull, "журнал полон")
            }
            e => io::Error::new(io::ErrorKind::InvalidInput, e),
        })?;
        // Фиксация — после байтов кадра. Release: другой процесс, который
        // прочитает новый счётчик с Acquire, увидит и сам кадр. Если
        // процесс убьют до этой строки, кадра в журнале нет, а его байты
        // перезапишет следующий push.
        self.used_counter().store(((at + n) as u64).to_le(), Ordering::Release);
        self.frames += 1;
        Ok(seq)
    }

    /// Записать грязные страницы на диск и дождаться конца записи. Без
    /// этого ядро запишет их само, когда сочтёт нужным (обычно за 30 с).
    pub fn sync(&self) -> io::Result<()> {
        // SAFETY: адрес и длина — всё наше отображение.
        unsafe { msync(self.base.as_ptr().cast(), self.len, MsyncFlags::SYNC)? };
        Ok(())
    }
}

impl Drop for Journal {
    fn drop(&mut self) {
        // SAFETY: отображение создано в `open`; ссылки из `committed` не
        // переживают `self`.
        unsafe { munmap(self.base.as_ptr().cast(), self.len).ok() };
    }
}
// ANCHOR_END: push

// ANCHOR: read
/// Зафиксированные кадры журнала, прочитанные через `read`, без mmap. Тот
/// же страничный кеш: что писатель записал в память, `read` видит сразу,
/// даже если писателя убили и `msync` не было.
pub fn read_committed(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = std::fs::read(path)?;
    if bytes.len() < HEADER || bytes[..8] != MAGIC {
        return Err(invalid("не журнал кадров"));
    }
    let used = u64::from_le_bytes(bytes[USED_AT..USED_AT + 8].try_into().unwrap()) as usize;
    if HEADER + used > bytes.len() {
        return Err(invalid("счётчик байт за концом файла"));
    }
    bytes.truncate(HEADER + used);
    bytes.drain(..HEADER);
    Ok(bytes)
}
// ANCHOR_END: read

fn invalid(msg: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, msg.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use telemetry_frame::stream::stats;

    fn path(name: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("ch28-journal-{name}-{}", std::process::id()));
        std::fs::remove_file(&p).ok();
        p
    }

    #[test]
    fn frames_survive_reopen_and_are_seen_by_read() {
        let p = path("reopen");
        // SAFETY: файл наш, кроме журнала его никто не трогает.
        let mut j = unsafe { Journal::open(&p, 4096) }.unwrap();
        assert_eq!(j.push(Kind::Measurement, &[1; 10]).unwrap(), 0);
        assert_eq!(j.push(Kind::Event, b"boot").unwrap(), 1);
        // Пока журнал открыт, read видит те же байты.
        assert_eq!(read_committed(&p).unwrap(), j.committed());
        drop(j);
        // SAFETY: как выше.
        let mut j = unsafe { Journal::open(&p, 4096) }.unwrap();
        assert_eq!(j.frames(), 2);
        assert_eq!(j.push(Kind::Measurement, &[2; 10]).unwrap(), 2);
        let s = stats(j.committed());
        assert_eq!((s.frames, s.errors, s.junk), ([2, 1, 0], 0, 0));
        drop(j);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn second_writer_is_refused() {
        let p = path("lock");
        // SAFETY: как выше.
        let _j = unsafe { Journal::open(&p, 4096) }.unwrap();
        // SAFETY: второй журнал не откроется — проверка этого.
        let e = unsafe { Journal::open(&p, 4096) }.err().unwrap();
        assert_eq!(e.kind(), io::ErrorKind::WouldBlock);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn full_journal_refuses_and_keeps_frames() {
        let p = path("full");
        // SAFETY: как выше.
        let mut j = unsafe { Journal::open(&p, 40) }.unwrap();
        j.push(Kind::Measurement, &[0; 10]).unwrap(); // 18 байт
        j.push(Kind::Measurement, &[0; 10]).unwrap(); // 36 байт
        let e = j.push(Kind::Measurement, &[0; 10]).unwrap_err();
        assert_eq!(e.kind(), io::ErrorKind::StorageFull);
        assert_eq!((j.frames(), j.committed().len()), (2, 36));
        drop(j);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn uncommitted_bytes_are_not_frames() {
        let p = path("torn");
        // SAFETY: как выше.
        let mut j = unsafe { Journal::open(&p, 4096) }.unwrap();
        j.push(Kind::Measurement, &[1; 10]).unwrap();
        // Процесс «убит» посреди push: байты кадра есть, счётчика нет.
        // SAFETY: хвост после зафиксированных байт — наш.
        let tail = unsafe { std::slice::from_raw_parts_mut(j.base.as_ptr().add(HEADER + 18), 18) };
        encode(Kind::Measurement, 1, &[9; 10], tail).unwrap();
        drop(j);
        assert_eq!(read_committed(&p).unwrap().len(), 18);
        // SAFETY: как выше.
        let mut j = unsafe { Journal::open(&p, 4096) }.unwrap();
        assert_eq!(j.frames(), 1);
        j.push(Kind::Event, b"ok").unwrap();
        let s = stats(j.committed());
        assert_eq!((s.frames, s.errors, s.junk), ([1, 1, 0], 0, 0));
        drop(j);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn foreign_file_is_rejected() {
        let p = path("foreign");
        std::fs::write(&p, vec![0u8; HEADER + 4096]).unwrap();
        // SAFETY: как выше.
        let e = unsafe { Journal::open(&p, 4096) }.err().unwrap();
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        std::fs::remove_file(&p).unwrap();
    }
}
