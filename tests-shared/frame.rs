// Общие тесты формата кадра сквозного проекта (шаги глав 6, 7 и 12;
// тесты задачи П7 начинаются с `p7_`, задачи П12 — с `p12_`).
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

// ---------------------------------------------------------- задача П12

/// Входы для сравнения двух `decode`: эталонные кадры и все их префиксы,
/// все однобитовые порчи, все значения каждого байта заголовка, кадры
/// предельной длины и псевдослучайные буферы.
fn p12_inputs() -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut tail = MEAS.to_vec();
    tail.extend_from_slice(&ACK);
    for v in [&MEAS[..], &ACK[..], &tail[..]] {
        for n in 0..=v.len() {
            out.push(v[..n].to_vec());
        }
        for bit in 0..v.len() * 8 {
            let mut b = v.to_vec();
            b[bit / 8] ^= 1 << (bit % 8);
            out.push(b);
        }
        for pos in 0..HEADER_LEN.min(v.len()) {
            for x in 0..=255u8 {
                let mut b = v.to_vec();
                b[pos] = x;
                out.push(b);
            }
        }
    }
    let mut buf = [0u8; HEADER_LEN + MAX_PAYLOAD + 1];
    let n = encode(Kind::Event, 9, &[0x5a; MAX_PAYLOAD], &mut buf).unwrap();
    out.push(buf[..n].to_vec());
    out.push(buf[..n - 1].to_vec());
    let mut long = buf[..n].to_vec();
    long[4] = 241;
    out.push(long);
    let mut seed = 0x2545_f491_u32;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed
    };
    for i in 0..3000 {
        let len = next() as usize % 40;
        let mut b: Vec<u8> = (0..len).map(|_| next() as u8).collect();
        // Половина буферов с верным началом, чтобы разбор шёл глубже.
        if i % 2 == 0 && len >= 3 {
            b[0] = MAGIC;
            b[1] = VERSION;
            b[2] = 1 + (next() % 3) as u8;
            if len >= 6 {
                b[4] = (next() % 24) as u8;
                b[5] = 0;
            }
        }
        out.push(b);
    }
    out
}

#[test]
fn p12_same_as_decode() {
    let inputs = p12_inputs();
    // Под Miri каждый сороковой вход: иначе тест идёт минуты.
    let step = if cfg!(miri) { 41 } else { 1 };
    for b in inputs.iter().step_by(step) {
        assert_eq!(parse::decode(b), decode(b), "вход {b:02x?}");
    }
    // Среди входов есть все исходы разбора. Это свойство самих входов, а
    // не кода, и под Miri его не пересчитывают: вышло бы ещё десять тысяч
    // разборов.
    if cfg!(miri) {
        return;
    }
    let outcomes: std::collections::BTreeSet<String> = inputs
        .iter()
        .map(|b| match decode(b) {
            Ok(_) => "Ok".to_string(),
            Err(e) => format!("{e:?}").split([' ', '(']).next().unwrap().to_string(),
        })
        .collect();
    assert_eq!(
        outcomes.into_iter().collect::<Vec<_>>(),
        ["BadCrc", "BadKind", "BadMagic", "BadVersion", "Ok", "Short", "TooLong"]
    );
}

#[test]
fn p12_combinators() {
    use parse::{byte, expect, le_u16, le_u32, map, take, then, try_map};
    assert_eq!(take(2)(&[1, 2, 3][..]), Ok((&[1, 2][..], &[3][..])));
    assert_eq!(take(4)(&[1, 2, 3][..]), Err(FrameError::Short { need: 4, have: 3 }));
    assert_eq!(take(0)(&[][..]), Ok((&[][..], &[][..])));
    assert_eq!(byte()(&[7, 8][..]), Ok((7, &[8][..])));
    assert_eq!(byte()(&[][..]), Err(FrameError::Short { need: 1, have: 0 }));
    let magic = expect(MAGIC, FrameError::BadMagic);
    assert_eq!(magic(&[MAGIC, 1][..]), Ok((MAGIC, &[1][..])));
    assert_eq!(magic(&[0x5a][..]), Err(FrameError::BadMagic(0x5a)));
    assert_eq!(le_u16()(&[0x34, 0x12, 9][..]), Ok((0x1234, &[9][..])));
    assert_eq!(le_u32()(&[1, 0, 0, 0x80][..]), Ok((0x8000_0001, &[][..])));
    assert_eq!(le_u32()(&[1, 0][..]), Err(FrameError::Short { need: 4, have: 2 }));
    assert_eq!(map(byte(), |b| b * 2)(&[21][..]), Ok((42, &[][..])));
    let kind = try_map(byte(), Kind::try_from);
    assert_eq!(kind(&[3][..]), Ok((Kind::Ack, &[][..])));
    assert_eq!(kind(&[9][..]), Err(FrameError::BadKind(9)));
    // Длина, а за ней столько байт.
    let counted = then(byte(), |&n| take(usize::from(n)));
    assert_eq!(counted(&[2, 7, 8, 9][..]), Ok(((2, &[7, 8][..]), &[9][..])));
    assert_eq!(counted(&[3, 7][..]), Err(FrameError::Short { need: 3, have: 1 }));
}

#[test]
fn p12_frame_leaves_rest() {
    let mut two = MEAS.to_vec();
    two.extend_from_slice(&ACK);
    let (f, rest) = parse::frame()(&two).unwrap();
    assert_eq!(f, decode(&MEAS).unwrap());
    assert_eq!(rest, &ACK[..]);
    assert_eq!(parse::decode(rest).unwrap().header.seq, 255);
}

#[test]
fn p12_measurement() {
    let payload = &MEAS[HEADER_LEN..];
    assert_eq!(parse::measurement_from(payload), Ok(meas()));
    let (m, rest) = parse::measurement()(&MEAS[HEADER_LEN..]).unwrap();
    assert_eq!((m, rest), (meas(), &[][..]));
    let mut zero = payload.to_vec();
    zero[0] = 0;
    zero[1] = 0;
    let mut longer = payload.to_vec();
    longer.push(0);
    for b in [&zero[..], &longer[..], &payload[..9], &[][..]] {
        assert_eq!(parse::measurement_from(b), Measurement::from_bytes(b), "{b:02x?}");
        assert_eq!(parse::measurement_from(b), Err(FrameError::BadPayload));
    }
    for v in [i32::MIN, -1, 0, i32::MAX] {
        let m = Measurement { value: v, ..meas() };
        assert_eq!(parse::measurement_from(&m.to_bytes()), Ok(m));
    }
}

/// Весь разбор встроен в `parse::decode`: ни вызовов по адресу, ни
/// вызовов других функций и замыканий модуля `parse`.
#[test]
#[cfg_attr(miri, ignore = "запускает cargo")]
fn p12_inlined() {
    use std::process::Command;
    assert_eq!(parse::decode(&ACK), decode(&ACK), "сначала разбор должен работать");
    let lib = env!("CARGO_PKG_NAME").replace('-', "_");
    let label_part = format!("{}{lib}5parse6decode:", lib.len());
    for target in ["x86_64-unknown-linux-gnu", "thumbv7m-none-eabi"] {
        let dir = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("{}-p12-asm-{target}", env!("CARGO_PKG_NAME")));
        let mut cmd = Command::new("cargo");
        cmd.current_dir(env!("CARGO_MANIFEST_DIR"))
            .env_remove("RUSTUP_TOOLCHAIN")
            .env_remove("RUSTFLAGS")
            .args(["rustc", "--release", "--lib", "--target", target, "--target-dir"])
            .arg(&dir)
            .args(["--", "--emit", "asm", "-C", "codegen-units=1"]);
        if target.starts_with("x86_64") {
            cmd.args(["-C", "llvm-args=-x86-asm-syntax=intel"]);
        }
        let out = cmd.output().unwrap();
        assert!(out.status.success(), "{target}: {}", String::from_utf8_lossy(&out.stderr));
        let deps = dir.join(target).join("release/deps");
        let file = std::fs::read_dir(&deps)
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                let n = p.file_name().unwrap().to_string_lossy();
                n.starts_with(&format!("{lib}-")) && n.ends_with(".s")
            })
            .expect("нет файла .s");
        let asm = std::fs::read_to_string(file).unwrap();
        let start = asm
            .lines()
            .position(|l| l.ends_with(&label_part) && !l.starts_with('.'))
            .unwrap_or_else(|| panic!("{target}: нет функции parse::decode"));
        let body: Vec<String> = asm
            .lines()
            .skip(start + 1)
            .take_while(|l| !l.starts_with(".Lfunc_end"))
            .map(|l| l.trim().replace('\t', " "))
            .filter(|l| !l.is_empty() && !l.starts_with(['.', '#', '@']) && !l.ends_with(':'))
            .collect();
        let bad: Vec<&String> = body
            .iter()
            .filter(|l| {
                let call = l.starts_with("call") || l.starts_with("bl ") || l.starts_with("blx");
                let operand = l.split_once(' ').map_or("", |(_, o)| o.trim());
                let indirect = operand.starts_with('r')
                    || operand.contains("ptr [") && !operand.contains("rip");
                call && (indirect || operand.contains("5parse"))
            })
            .collect();
        assert!(bad.is_empty(), "{target}: в parse::decode остались вызовы {bad:?}");
    }
}
