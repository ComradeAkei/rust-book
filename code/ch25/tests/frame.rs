//! Раздел 25.5: кадр, записанный литералом `hex!`, разбирает настоящий
//! декодер сквозного проекта — CRC и поля совпадают.

use ch25::tables::MEASUREMENT_FRAME;
use telemetry_frame::{Kind, decode};

#[test]
fn hex_literal_is_a_valid_frame() {
    let frame = decode(&MEASUREMENT_FRAME).unwrap();
    assert_eq!(frame.header.kind, Kind::Measurement);
    assert_eq!(frame.header.seq, 7);
    assert_eq!(frame.payload, [0x12, 0x34]);
    let f = MEASUREMENT_FRAME;
    let crc = telemetry_frame::crc16_update(telemetry_frame::crc16(&f[..6]), &f[8..]);
    assert_eq!(crc, 0x9BD2);
    assert_eq!(crc.to_le_bytes(), [f[6], f[7]]);
}
