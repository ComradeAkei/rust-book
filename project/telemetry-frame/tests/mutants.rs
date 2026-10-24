//! Шаг главы 31 (раздел 31.8): тесты против мутантов, которых пропустили
//! прежние тесты проекта. `cargo mutants -p telemetry-frame` нашёл двух
//! выживших; код проекта не менялся — добавились только эти проверки.

use telemetry_frame::stream::Frames;
use telemetry_frame::{FrameError, HEADER_LEN, Kind, MAX_PAYLOAD, decode, encode};

// ANCHOR: exact_buffer
/// Мутант `encode`: `body.len() < payload.len()` → `<=`. Тесты кодировали
/// в буфер с запасом, а буфер ровно под кадр не пробовал никто. Граница
/// проверяется с обеих сторон: ровно `need` байт — можно, на байт меньше —
/// нельзя.
#[test]
fn encode_fits_exact_buffer() {
    for len in [0, 1, 10, MAX_PAYLOAD] {
        let payload = vec![0x5A; len];
        let need = HEADER_LEN + len;
        let mut out = vec![0; need];
        assert_eq!(encode(Kind::Event, 3, &payload, &mut out), Ok(need));
        assert_eq!(decode(&out).unwrap().payload, &payload[..]);
        let have = need - 1;
        assert_eq!(
            encode(Kind::Event, 3, &payload, &mut out[..have]),
            Err(FrameError::BufferTooSmall { need, have })
        );
    }
}
// ANCHOR_END: exact_buffer

// ANCHOR: size_hint
/// Мутант `size_hint`: `len - pos` → `len + pos`. Такая граница сверху
/// тоже верна — элементов не больше, чем байтов, — поэтому прежний тест
/// её пропускал: на позиции 0 обе формулы равны, а дальше он проверял
/// только `hi >= count`. Документация обещает точнее: граница — ровно
/// непрочитанный хвост.
#[test]
fn size_hint_is_rest_of_buffer() {
    let mut buf = [0u8; 2 * HEADER_LEN + 3];
    let n = encode(Kind::Ack, 1, &[], &mut buf).unwrap();
    encode(Kind::Ack, 2, &[], &mut buf[n..]).unwrap();
    let mut it = Frames::until_eof(&buf);
    assert_eq!(it.size_hint(), (0, Some(buf.len())));
    while it.next().is_some() {
        assert_eq!(it.size_hint(), (0, Some(it.rest().len())));
    }
    assert_eq!(it.position(), buf.len());
}
// ANCHOR_END: size_hint
