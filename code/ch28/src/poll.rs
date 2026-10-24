//! Раздел 28.6: epoll. Один поток ждёт сразу много дескрипторов и читает
//! тот, в котором есть данные.

use rustix::buffer::spare_capacity;
use rustix::event::epoll::{self, CreateFlags, EventData, EventFlags};
use rustix::io::Errno;
use std::io;
use std::os::fd::OwnedFd;
use std::time::{Duration, Instant};

// ANCHOR: drain
/// Читать все дескрипторы, пока каждый не закроется. Возвращает куски в
/// порядке прихода: (номер источника, байты).
pub fn drain_all(sources: &[OwnedFd]) -> io::Result<Vec<(usize, Vec<u8>)>> {
    let ep = epoll::create(CreateFlags::CLOEXEC)?;
    for (i, fd) in sources.iter().enumerate() {
        epoll::add(&ep, fd, EventData::new_u64(i as u64), EventFlags::IN)?;
    }
    let mut open = sources.len();
    let mut chunks = Vec::new();
    let mut events = Vec::with_capacity(8);
    let mut buf = [0u8; 4096];
    while open > 0 {
        events.clear();
        match epoll::wait(&ep, spare_capacity(&mut events), None) {
            Ok(_) => {}
            // Сигнал прерывает epoll_wait и при SA_RESTART (раздел 28.4):
            // событий нет, ждём снова.
            Err(Errno::INTR) => continue,
            Err(e) => return Err(e.into()),
        }
        for ev in &events {
            let i = ev.data.u64() as usize;
            // Уровневый режим: если прочитаем не всё, epoll сообщит снова.
            match rustix::io::read(&sources[i], &mut buf)? {
                0 => {
                    // Конец потока: писатель закрыл свой конец (EPOLLHUP).
                    epoll::delete(&ep, &sources[i])?;
                    open -= 1;
                }
                n => chunks.push((i, buf[..n].to_vec())),
            }
        }
    }
    Ok(chunks)
}
// ANCHOR_END: drain

// ANCHOR: edge
/// Сколько событий сообщит epoll, если на каждое читать по одному байту,
/// а в канале лежит `pending` байт. `edge` — флаг EPOLLET.
pub fn events_for_one_byte_reads(pending: usize, edge: bool) -> io::Result<usize> {
    let (r, w) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC)?;
    rustix::io::write(&w, &vec![b'x'; pending])?;
    let ep = epoll::create(CreateFlags::CLOEXEC)?;
    let flags = if edge { EventFlags::IN | EventFlags::ET } else { EventFlags::IN };
    epoll::add(&ep, &r, EventData::new_u64(0), flags)?;
    let mut events = Vec::with_capacity(1);
    let mut reported = 0;
    loop {
        events.clear();
        // 50 мс тишины — событий больше не будет. Прерванное сигналом
        // ожидание (EINTR) продолжаем на остаток срока, а не заново.
        let deadline = Instant::now() + Duration::from_millis(50);
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let timeout = rustix::time::Timespec::try_from(left).unwrap();
            match epoll::wait(&ep, spare_capacity(&mut events), Some(&timeout)) {
                Ok(_) => break,
                Err(Errno::INTR) => continue,
                Err(e) => return Err(e.into()),
            }
        }
        if events.is_empty() {
            return Ok(reported);
        }
        reported += 1;
        let mut byte = [0u8];
        match rustix::io::read(&r, &mut byte) {
            Ok(_) | Err(Errno::AGAIN) => {}
            Err(e) => return Err(e.into()),
        }
    }
}
// ANCHOR_END: edge

// ANCHOR: uring
/// Есть ли io_uring: `io_uring_setup` на 8 записей. Ядро само выбирает
/// размеры колец и пишет их в `struct io_uring_params` (120 байт):
/// `sq_entries` по смещению 0, `cq_entries` — по смещению 4.
pub fn io_uring_probe() -> io::Result<(u32, u32)> {
    let mut params = [0u32; 30];
    // SAFETY: буфер размером со struct io_uring_params, ядро пишет в него.
    let fd = unsafe { libc::syscall(libc::SYS_io_uring_setup, 8, params.as_mut_ptr()) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: успешный io_uring_setup вернул новый дескриптор, он наш.
    drop(unsafe { <OwnedFd as std::os::fd::FromRawFd>::from_raw_fd(fd as i32) });
    Ok((params[0], params[1]))
}
// ANCHOR_END: uring

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn drains_in_arrival_order() {
        let mut readers = Vec::new();
        let mut writers = Vec::new();
        for _ in 0..3 {
            let (r, w) = rustix::pipe::pipe_with(rustix::pipe::PipeFlags::CLOEXEC).unwrap();
            readers.push(r);
            writers.push(w);
        }
        let t = thread::spawn(move || {
            // Порядок записи: 2, 0, 1 — с паузами, чтобы куски не слились.
            for i in [2, 0, 1] {
                rustix::io::write(&writers[i], format!("от {i}").as_bytes()).unwrap();
                thread::sleep(Duration::from_millis(30));
            }
            drop(writers);
        });
        let chunks = drain_all(&readers).unwrap();
        t.join().unwrap();
        let order: Vec<usize> = chunks.iter().map(|c| c.0).collect();
        assert_eq!(order, [2, 0, 1]);
        assert_eq!(chunks[0].1, "от 2".as_bytes());
    }

    #[test]
    fn io_uring_rings_if_available() {
        // В контейнерах io_uring бывает запрещён (EPERM, ENOSYS): тогда
        // проверять нечего.
        if let Ok((sq, cq)) = io_uring_probe() {
            assert_eq!((sq, cq), (8, 16));
        }
    }

    #[test]
    fn level_reports_until_empty_edge_reports_once() {
        assert_eq!(events_for_one_byte_reads(5, false).unwrap(), 5);
        assert_eq!(events_for_one_byte_reads(5, true).unwrap(), 1);
    }
}
