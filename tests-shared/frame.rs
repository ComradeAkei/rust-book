// Общие тесты формата кадра сквозного проекта (шаг главы 6).
//
// Подключаются через `include!` из `project/telemetry-frame/tests/frame.rs`
// и `exercises/project-frame/tests/frame.rs` после `use <крейт> as sut;`.
// Векторы посчитаны независимо, на Python (binascii.crc_hqx с init 0xFFFF).

use core::num::NonZeroU16;
use sut::*;

/// Измерение {датчик 7, 1000 мс, -42}, kind = 1, seq = 5.
const MEAS: [u8; 18] = [
    0xa5, 0x01, 0x01, 0x05, 0x0a, 0x00, 0xe5, 0x73, 0x07, 0x00, 0xe8, 0x03, 0x00, 0x00, 0xd6, 0xff,
    0xff, 0xff,
];

/// Подтверждение без нагрузки, seq = 255.
const ACK: [u8; 8] = [0xa5, 0x01, 0x03, 0xff, 0x00, 0x00, 0xd7, 0x52];

fn meas() -> Measurement {
    Measurement { sensor: NonZeroU16::new(7).unwrap(), timestamp_ms: 1000, value: -42 }
}

#[test]
fn crc_check_value() {
    assert_eq!(crc16(b"123456789"), 0x29B1);
    assert_eq!(crc16(b""), 0xFFFF);
    // Пошаговый расчёт совпадает с расчётом за один проход.
    assert_eq!(crc16_update(crc16(b"1234"), b"56789"), 0x29B1);
}

#[test]
fn encode_matches_vectors() {
    let mut out = [0u8; 64];
    let n = encode(Kind::Measurement, 5, &meas().to_bytes(), &mut out).unwrap();
    assert_eq!(&out[..n], &MEAS);
    let n = encode(Kind::Ack, 255, &[], &mut out).unwrap();
    assert_eq!(&out[..n], &ACK);
}

#[test]
fn decode_matches_vectors() {
    let f = decode(&MEAS).unwrap();
    assert_eq!(f.header, Header { kind: Kind::Measurement, seq: 5, len: 10 });
    assert_eq!(Measurement::from_bytes(f.payload), Ok(meas()));
    assert_eq!(f.wire_len(), MEAS.len());
    let f = decode(&ACK).unwrap();
    assert_eq!(f.header.kind, Kind::Ack);
    assert!(f.payload.is_empty());
}

#[test]
fn stream_of_frames() {
    let mut stream = Vec::new();
    stream.extend_from_slice(&MEAS);
    stream.extend_from_slice(&ACK);
    stream.extend_from_slice(&MEAS);
    let mut rest = &stream[..];
    let mut kinds = Vec::new();
    while !rest.is_empty() {
        let f = decode(rest).unwrap();
        kinds.push(f.header.kind);
        rest = &rest[f.wire_len()..];
    }
    assert_eq!(kinds, [Kind::Measurement, Kind::Ack, Kind::Measurement]);
}

#[test]
fn every_single_bit_flip_is_caught() {
    // CRC-16 обнаруживает любую одиночную ошибку. Проверяем все 144 бита.
    for byte in 0..MEAS.len() {
        for bit in 0..8 {
            let mut f = MEAS;
            f[byte] ^= 1 << bit;
            assert!(decode(&f).is_err(), "не замечена ошибка в байте {byte}, бит {bit}");
        }
    }
}

#[test]
fn decode_errors() {
    assert_eq!(decode(&MEAS[..7]), Err(FrameError::Short { need: 8, have: 7 }));
    assert_eq!(decode(&MEAS[..17]), Err(FrameError::Short { need: 18, have: 17 }));
    let mut f = MEAS;
    f[0] = 0x5A;
    assert_eq!(decode(&f), Err(FrameError::BadMagic(0x5A)));
    let mut f = MEAS;
    f[1] = 2;
    assert_eq!(decode(&f), Err(FrameError::BadVersion(2)));
    let mut f = MEAS;
    f[2] = 9;
    assert_eq!(decode(&f), Err(FrameError::BadKind(9)));
    let mut f = MEAS;
    f[4] = 241;
    assert_eq!(decode(&f), Err(FrameError::TooLong(241)));
    let mut f = MEAS;
    f[17] ^= 0x01;
    assert!(matches!(decode(&f), Err(FrameError::BadCrc { stored: 0x73e5, .. })));
}

#[test]
fn encode_errors() {
    let mut small = [0u8; 12];
    assert_eq!(
        encode(Kind::Measurement, 0, &meas().to_bytes(), &mut small),
        Err(FrameError::BufferTooSmall { need: 18, have: 12 })
    );
    let big = [0u8; 241];
    let mut out = [0u8; 300];
    assert_eq!(encode(Kind::Event, 0, &big, &mut out), Err(FrameError::TooLong(241)));
    let max = [0x55u8; 240];
    let n = encode(Kind::Event, 1, &max, &mut out).unwrap();
    assert_eq!(decode(&out[..n]).unwrap().payload, &max[..]);
}

#[test]
fn measurement_payload() {
    assert_eq!(Measurement::from_bytes(&[0; 9]), Err(FrameError::BadPayload));
    assert_eq!(Measurement::from_bytes(&[0; 10]), Err(FrameError::BadPayload));
    assert_eq!(core::mem::size_of::<Option<Measurement>>(), core::mem::size_of::<Measurement>());
    assert_eq!(core::mem::size_of::<HeaderWire>(), HEADER_LEN);
    assert_eq!(core::mem::align_of::<HeaderWire>(), 1);
}
