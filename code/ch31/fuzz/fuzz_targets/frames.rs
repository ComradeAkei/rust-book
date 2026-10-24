// ANCHOR: all
#![no_main]

use libfuzzer_sys::fuzz_target;
use telemetry_frame::stream::Frames;
use telemetry_frame::{HEADER_LEN, MAX_PAYLOAD, decode, encode};

// Шаг главы 31: разбор кадров проекта на любых байтах. Свойства те же,
// что в доказательствах Kani, но без предела в 24 байта.
fuzz_target!(|buf: &[u8]| {
    if let Ok(f) = decode(buf) {
        let mut out = [0; HEADER_LEN + MAX_PAYLOAD];
        let n = encode(f.header.kind, f.header.seq, f.payload, &mut out).unwrap();
        assert_eq!(&out[..n], &buf[..n]);
    }
    // Каждый элемент начинается там, где кончился предыдущий, и забирает
    // хотя бы байт; кадр из потока — тот же, что decode с его смещения.
    let mut it = Frames::until_eof(buf);
    let mut pos = 0;
    while let Some(item) = it.next() {
        let at = match item {
            Ok((at, f)) => {
                assert_eq!(decode(&buf[at..]), Ok(f));
                at
            }
            Err(e) => e.offset,
        };
        assert!(at == pos && it.position() > at && it.position() <= buf.len());
        pos = it.position();
    }
    assert_eq!(pos, buf.len());
});
// ANCHOR_END: all
