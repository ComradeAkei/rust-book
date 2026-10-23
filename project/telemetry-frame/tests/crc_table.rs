//! Шаг главы 25: CRC по таблице совпадает с побитовым вариантом.

use telemetry_frame::{CRC16_TABLE, crc16, crc16_update, crc16_update_table};

/// Все однобайтовые и двухбайтовые входы при любом начальном значении
/// из выборки, плюс длинные куски. Под Miri второй байт — каждый 17-й, а
/// длинный кусок — 256 байт: полный перебор там идёт минуты.
#[test]
fn table_matches_bitwise() {
    let step = if cfg!(miri) { 17 } else { 1 };
    for init in [0xFFFF, 0x0000, 0x1D0F, 0x8000, 0x29B1] {
        for a in 0..=255u8 {
            assert_eq!(crc16_update_table(init, &[a]), crc16_update(init, &[a]));
            for b in (0..=255u8).step_by(step) {
                let d = [a, b];
                assert_eq!(crc16_update_table(init, &d), crc16_update(init, &d));
            }
        }
    }
    let len = if cfg!(miri) { 256 } else { 4096 };
    let long: Vec<u8> = (0..len).map(|i: u32| (i.wrapping_mul(2654435761) >> 24) as u8).collect();
    assert_eq!(crc16_update_table(0xFFFF, &long), crc16(&long));
}

#[test]
fn table_entries() {
    assert_eq!(CRC16_TABLE[0], 0);
    assert_eq!(CRC16_TABLE[1], 0x1021);
    assert_eq!(CRC16_TABLE[255], 0x1EF0);
}

/// Наблюдение, не гарантия: скорость обоих вариантов на машине, где
/// запущен тест. `cargo test --release -p telemetry-frame --test crc_table
/// -- --ignored --nocapture`.
#[test]
#[ignore = "замер скорости, запускать в release"]
fn observed_speed() {
    use std::hint::black_box;
    use std::time::Instant;
    let data = vec![0x5Au8; 1 << 20];
    let mb = |f: &dyn Fn(&[u8]) -> u16| {
        let t = Instant::now();
        let mut acc = 0u16;
        for _ in 0..64 {
            acc ^= f(black_box(&data));
        }
        black_box(acc);
        64.0 / t.elapsed().as_secs_f64()
    };
    let bit = mb(&|d| crc16_update(0xFFFF, d));
    let table = mb(&|d| crc16_update_table(0xFFFF, d));
    println!("побитово {bit:.0} МиБ/с, по таблице {table:.0} МиБ/с, в {:.1} раза", table / bit);
}
