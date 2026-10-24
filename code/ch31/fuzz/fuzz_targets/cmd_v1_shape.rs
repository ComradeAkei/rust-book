// ANCHOR: all
#![no_main]

use arbitrary::Arbitrary;
use ch31::cmd::{SYNC, crc8, parse_v1};
use libfuzzer_sys::fuzz_target;

/// Форма команды: код и поля нагрузки. Длину, число точек и CRC
/// вычисляет обёртка — согласовывать их фаззеру не нужно.
#[derive(Arbitrary, Debug)]
enum Shape {
    Ping,
    SetPeriod { sensor: u16, period_ms: u16 },
    Calibrate { sensor: u16, points: Vec<[u8; 4]> },
    Reset,
}

fuzz_target!(|shape: Shape| {
    let (cmd, payload) = match shape {
        Shape::Ping => (1, vec![]),
        Shape::SetPeriod { sensor, period_ms } => {
            (2, [sensor.to_le_bytes(), period_ms.to_le_bytes()].concat())
        }
        Shape::Calibrate { sensor, points } => {
            let mut p = sensor.to_le_bytes().to_vec();
            p.push(points.len() as u8);
            p.extend(points.iter().flatten());
            (3, p)
        }
        Shape::Reset => (4, vec![]),
    };
    let Ok(len) = u8::try_from(payload.len()) else { return };
    let mut wire = [&[SYNC, cmd, len][..], &payload].concat();
    wire.push(crc8(&wire[1..]));
    let _ = parse_v1(&wire);
});
// ANCHOR_END: all
