//! Шаг главы 31: доказательства Kani для разбора кадров. Модуль
//! собирается только под `cargo kani` (`cfg(kani)`); в прошивку и в
//! обычную сборку хоста он не попадает.

use crate::stream::Frames;
use crate::{decode, encode};

// ANCHOR: decode
/// Любой буфер длиной до 24 байт — это кадры с нагрузкой до 16 байт и
/// все их порчи. `decode` не паникует, принятый кадр лежит внутри
/// буфера и кодируется обратно в те же байты.
#[kani::proof]
#[kani::unwind(25)]
fn decode_any_buffer_up_to_24() {
    let buf: [u8; 24] = kani::any();
    let len = kani::any_where(|&n: &usize| n <= buf.len());
    if let Ok(f) = decode(&buf[..len]) {
        assert!(f.wire_len() <= len);
        let mut out = [0u8; 24];
        let n = encode(f.header.kind, f.header.seq, f.payload, &mut out).unwrap();
        assert!(out[..n] == buf[..n]);
    }
}
// ANCHOR_END: decode

// ANCHOR: frames
/// Один шаг `Frames` с начала любого буфера до 24 байт: не паникует, а
/// если отдал элемент — сдвинулся вперёд, но не за конец. Следующий шаг
/// итератора зависит только от непрочитанного хвоста, а хвост — тоже
/// буфер до 24 байт. Значит, по индукции разбор любого такого буфера
/// кончается, и каждый шаг забирает хотя бы байт. Развернуть весь цикл
/// Kani тоже может, но уже для 16 байт CBMC не закончил и за 28 минут.
#[kani::proof]
#[kani::unwind(25)]
fn frames_step_makes_progress() {
    let buf: [u8; 24] = kani::any();
    let len = kani::any_where(|&n: &usize| n <= buf.len());
    let mut it = Frames::until_eof(&buf[..len]);
    match it.next() {
        Some(item) => {
            let at = match item {
                Ok((at, _)) => at,
                Err(e) => e.offset,
            };
            assert!(at == 0);
            assert!(it.position() > 0 && it.position() <= len);
        }
        None => assert!(len == 0),
    }
}
// ANCHOR_END: frames
