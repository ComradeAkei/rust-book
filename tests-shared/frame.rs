// Общие тесты формата кадра сквозного проекта (шаги глав 6 и 7; тесты
// задачи П7 начинаются с `p7_`).
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

// ---------------------------------------------------------- задача П7

#[test]
fn p7_messages() {
    let cases = [
        (FrameError::Short { need: 18, have: 5 }, "кадр не полон: нужно 18 байт, есть 5"),
        (FrameError::BufferTooSmall { need: 18, have: 12 }, "буфер мал: нужно 18 байт, есть 12"),
        (FrameError::BadMagic(0x5a), "неверный маркер 0x5a"),
        (FrameError::BadVersion(2), "версия 2 не поддерживается"),
        (FrameError::BadKind(9), "неизвестный тип кадра 9"),
        (FrameError::TooLong(241), "нагрузка 241 байт длиннее 240"),
        (
            FrameError::BadCrc { stored: 0x73e5, computed: 0x0a1b },
            "CRC не совпадает: в кадре 0x73e5, посчитано 0x0a1b",
        ),
        (FrameError::BadPayload, "нагрузка не соответствует типу кадра"),
    ];
    for (e, msg) in cases {
        assert_eq!(e.to_string(), msg);
        let dyn_err: &dyn core::error::Error = &e;
        assert!(dyn_err.source().is_none(), "{e:?}: у ошибки разбора нет причины");
    }
}

#[test]
fn p7_recovery() {
    assert_eq!(FrameError::Short { need: 18, have: 5 }.recovery(), Recovery::NeedMore(13));
    // Несогласованная ошибка, собранная вручную, не должна ни паниковать,
    // ни заставлять приёмник ждать почти usize::MAX байт.
    assert_eq!(FrameError::Short { need: 1, have: 5 }.recovery(), Recovery::NeedMore(0));
    for e in [
        FrameError::BadMagic(0),
        FrameError::BadVersion(7),
        FrameError::BadKind(0),
        FrameError::TooLong(1000),
        FrameError::BadCrc { stored: 1, computed: 2 },
    ] {
        assert_eq!(e.recovery(), Recovery::Resync, "{e:?}");
    }
    assert_eq!(FrameError::BadPayload.recovery(), Recovery::Drop);
    assert_eq!(FrameError::BufferTooSmall { need: 1, have: 0 }.recovery(), Recovery::Reject);
}

#[test]
fn p7_context_in_stream() {
    let mut stream = [0u8; 40];
    stream[..8].copy_from_slice(&ACK);
    stream[8..26].copy_from_slice(&MEAS);
    stream[26..34].copy_from_slice(&ACK);
    stream[33] ^= 0x01; // порча CRC третьего кадра
    assert_eq!(decode_at(&stream, 0).unwrap().header.kind, Kind::Ack);
    assert_eq!(decode_at(&stream, 8).unwrap().payload, &MEAS[8..]);
    let e = decode_at(&stream, 26).unwrap_err();
    assert_eq!(e.offset, 26);
    assert_eq!(e.error.recovery(), Recovery::Resync);
    assert_eq!(e.to_string(), "кадр со смещения 26 отвергнут");
    let src = core::error::Error::source(&e).unwrap();
    assert_eq!(src.to_string(), e.error.to_string());
    assert_eq!(src.downcast_ref::<FrameError>(), Some(&e.error));
    // Порча длины выглядит как неполный кадр: 8 + 64 байта вместо 8.
    // Приёмник будет ждать, пока не сработает тайм-аут (главы 12–13).
    let mut bad_len = stream;
    bad_len[30] ^= 0x40;
    let e = decode_at(&bad_len, 26).unwrap_err();
    assert_eq!(e.error, FrameError::Short { need: 72, have: 14 });
    assert_eq!(e.error.recovery(), Recovery::NeedMore(58));
    // Смещение за концом потока — неполный кадр, а не паника.
    let e = decode_at(&stream, 100).unwrap_err();
    assert_eq!(e.error, FrameError::Short { need: HEADER_LEN, have: 0 });
    let e = decode_at(&stream[..30], 26).unwrap_err();
    assert_eq!(e.error.recovery(), Recovery::NeedMore(4));
}

#[test]
fn p7_errors_are_small() {
    assert!(core::mem::size_of::<FrameError>() <= 24);
    assert!(core::mem::size_of::<RxError>() <= 32);
}
