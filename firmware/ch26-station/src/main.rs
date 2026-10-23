//! Глава 26, сквозной проект (раздел 26.7): прошивка станции без
//! cortex-m-rt. Среда выполнения — ch26-bare, формат кадра — крейт
//! telemetry-frame, тот же, что разбирает хост. Станция «измеряет»
//! два датчика, кодирует кадры, печатает их байты через semihosting и
//! разбирает обратно.
//!
//! ```text
//! cd firmware && cargo run --release -p ch26-station
//! ```
#![no_std]
#![no_main]

use ch26_bare::{entry, sh};
use core::num::NonZeroU16;
use telemetry_frame::{
    CRC16_TABLE, HEADER_LEN, Kind, Measurement, crc16_update_table, decode, encode,
};

entry!(main);

/// Показание датчика в момент `t` мс: пила с периодом 1 с, у второго
/// датчика — со сдвигом и в обратную сторону. Вместо АЦП — арифметика:
/// драйверы и таймер появятся в главе 27.
fn sample(sensor: u16, t: u32) -> i32 {
    let phase = (t % 1000) as i32;
    if sensor == 1 { 2000 + phase } else { -500 - phase / 2 }
}

// ANCHOR: main
fn main() -> ! {
    sh::put("станция: среда ch26-bare, кадры telemetry-frame\n");
    // Всё состояние — на стеке: изменяемых static у этой среды нет.
    let mut seq = 0u8;
    let (mut sent, mut decoded, mut table_ok) = (0u32, 0u32, 0u32);
    for t in (0..1000).step_by(250) {
        for sensor in 1..=2 {
            let m = Measurement {
                sensor: NonZeroU16::new(sensor).unwrap(),
                timestamp_ms: t,
                value: sample(sensor, t),
            };
            let mut buf = [0u8; HEADER_LEN + Measurement::WIRE_LEN];
            let Ok(n) = encode(Kind::Measurement, seq, &m.to_bytes(), &mut buf) else {
                sh::put("кадр не поместился\n");
                sh::exit(false);
            };
            sent += 1;
            if seq < 2 {
                sh::put("кадр ");
                sh::dec(seq.into());
                sh::put(":");
                for b in &buf[..n] {
                    sh::put(" ");
                    hex2(*b);
                }
                sh::put("\n");
            }
            // Обратно, как это сделает хост.
            let back = match decode(&buf[..n]) {
                Ok(f) => f.header.seq == seq && Measurement::from_bytes(f.payload) == Ok(m),
                Err(_) => false,
            };
            decoded += u32::from(back);
            // CRC по таблице из .rodata — тот же, что побитовый в encode.
            let mut crc = crc16_update_table(0xFFFF, &buf[..6]);
            crc = crc16_update_table(crc, &buf[HEADER_LEN..n]);
            if crc.to_le_bytes() == buf[6..8] {
                table_ok += 1;
            }
            seq = seq.wrapping_add(1);
        }
    }
    sh::put("кадров ");
    sh::dec(sent);
    sh::put(", разобрано обратно ");
    sh::dec(decoded);
    sh::put(", CRC по таблице совпал ");
    sh::dec(table_ok);
    sh::put("\n");
    sh::put("CRC16_TABLE по адресу ");
    sh::hex(CRC16_TABLE.as_ptr() as u32);
    sh::put(" — во flash, код запуска её не трогал\n");
    sh::exit(sent == 8 && decoded == sent && table_ok == sent)
}
// ANCHOR_END: main

/// Байт двумя шестнадцатеричными цифрами.
fn hex2(b: u8) {
    let d = b"0123456789abcdef";
    let s = [d[(b >> 4) as usize], d[(b & 0xF) as usize]];
    // Цифры ASCII.
    sh::put(core::str::from_utf8(&s).unwrap_or("??"));
}
