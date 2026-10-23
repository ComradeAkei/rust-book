//! Глава 13 на Cortex-M0, M3 и M4F: итератор `Frames` из шага П13 разбирает
//! поток с мусором, испорченным кадром и оборванным хвостом; тот же поток,
//! принятый кусками через кольцевой буфер `Ring` из раздела 13.4, даёт те
//! же кадры с теми же смещениями; итератор по срезу занимает два слова.
//!
//! ```text
//! cd firmware && cargo run --release -p ch13-iter
//! ```
#![no_std]
#![no_main]

use ch13::custom::{Ring, RingIter};
use core::{iter, slice};
use cortex_m_rt::entry;
use cortex_m_semihosting::{debug, hprintln};
use telemetry_frame::stream::{Frames, stats};
use telemetry_frame::{Frame, Kind, RxError, encode};

/// Кадр: смещение в потоке, тип и номер.
type Seen = (usize, Kind, u8);

/// Поток: мусор, измерение, событие с испорченным CRC, ещё мусор,
/// подтверждение и первые пять байт следующего кадра.
fn stream(out: &mut [u8; 64]) -> usize {
    let mut n = 0;
    let junk = |out: &mut [u8; 64], n: &mut usize, bytes: &[u8]| {
        out[*n..*n + bytes.len()].copy_from_slice(bytes);
        *n += bytes.len();
    };
    junk(out, &mut n, &[0x00, 0x13, 0x37]);
    n += encode(Kind::Measurement, 1, &[0x10, 0x27, 0x00, 0x00], &mut out[n..]).unwrap();
    n += encode(Kind::Event, 2, &[7, 7], &mut out[n..]).unwrap();
    out[n - 1] ^= 0x10;
    junk(out, &mut n, &[0x5a, 0x5a]);
    n += encode(Kind::Ack, 3, &[], &mut out[n..]).unwrap();
    let mut tail = [0; 16];
    let t = encode(Kind::Measurement, 4, &[1, 2], &mut tail).unwrap();
    junk(out, &mut n, &tail[..t.min(5)]);
    n
}

// ANCHOR: chunks
/// Приём кусками по `piece` байт: прерывание кладёт байты в кольцевой
/// буфер, основной цикл перекладывает их в линейный `rx` и разбирает. Что
/// разобрано, сдвигается из `rx` через `copy_within`, неполный кадр ждёт
/// продолжения. Возвращает число кадров и ошибок.
fn by_pieces(input: &[u8], piece: usize, seen: &mut [Seen; 8]) -> (usize, usize) {
    let mut ring = Ring::<16>::new();
    let mut rx = [0u8; 32]; // не меньше самого длинного кадра потока
    let (mut len, mut base) = (0, 0);
    let (mut frames, mut errors) = (0, 0);
    // Тип элемента написан полностью: с `Result<_, _>` компилятор выберет
    // одно время жизни кадра для всех вызовов, и заём `rx` протянется
    // через `copy_within` — ошибка E0502.
    let mut take = |at: usize, item: Result<(usize, Frame<'_>), RxError>| match item {
        Ok((i, f)) => {
            seen[frames] = (at + i, f.header.kind, f.header.seq);
            frames += 1;
        }
        Err(_) => errors += 1,
    };
    for chunk in input.chunks(piece) {
        for &b in chunk {
            ring.push(b); // так работает обработчик прерывания UART
        }
        // `zip` сначала берёт место в `rx`: когда места нет, байт из
        // кольца не забирается и не теряется.
        let moved = rx[len..].iter_mut().zip(iter::from_fn(|| ring.pop())).map(|(d, b)| *d = b);
        len += moved.count();
        let mut it = Frames::new(&rx[..len]);
        it.by_ref().for_each(|item| take(base, item));
        let used = it.position();
        rx.copy_within(used..len, 0);
        (len, base) = (len - used, base + used);
    }
    // Тайм-аут приёма: продолжения не будет.
    Frames::until_eof(&rx[..len]).for_each(|item| take(base, item));
    (frames, errors)
}
// ANCHOR_END: chunks

// ANCHOR: main
#[entry]
fn main() -> ! {
    let mut buf = [0u8; 64];
    let n = stream(&mut buf);
    let input = &buf[..n];

    hprintln!("поток {} байт, целиком:", n);
    let mut whole = [(0, Kind::Ack, 0); 8];
    let mut frames = 0;
    for item in Frames::until_eof(input) {
        match item {
            Ok((at, f)) => {
                whole[frames] = (at, f.header.kind, f.header.seq);
                frames += 1;
                hprintln!("  {:2}: {:?}, номер {}", at, f.header.kind, f.header.seq);
            }
            Err(e) => hprintln!("  {:2}: {}", e.offset, e.error),
        }
    }
    let s = stats(input);
    hprintln!("кадров {:?}, ошибок {}, мусора {} байт", s.frames, s.errors, s.junk);

    for piece in [1, 5, 7] {
        let mut seen = [(0, Kind::Ack, 0); 8];
        let (got, errors) = by_pieces(input, piece, &mut seen);
        let same = whole[..frames].iter().zip(&seen[..got]).filter(|(a, b)| a == b).count();
        hprintln!(
            "кусками по {}: кадров {}, совпали {} из {}, ошибок {}",
            piece,
            got,
            same,
            frames,
            errors
        );
    }

    hprintln!(
        "slice::Iter<u8>: {} байт, RingIter: {} байт, Frames: {} байт",
        size_of::<slice::Iter<'static, u8>>(),
        size_of::<RingIter<'static>>(),
        size_of::<Frames<'static>>()
    );
    debug::exit(debug::EXIT_SUCCESS);
    loop {}
}
// ANCHOR_END: main

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    debug::exit(debug::EXIT_FAILURE);
    loop {}
}
