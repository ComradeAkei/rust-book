//! Раздел 31.7: проверка моделей через Kani. Функции здесь обычные;
//! доказательства — в модуле `proofs`, который собирается только под
//! `cargo kani` (`cfg(kani)`).

// ANCHOR: midpoint
/// Середина отрезка, первая версия: так её пишут в двоичном поиске.
pub fn midpoint_v1(a: u32, b: u32) -> u32 {
    (a + b) / 2
}

/// Середина без переполнения: общие биты плюс половина различающихся.
pub fn midpoint(a: u32, b: u32) -> u32 {
    (a & b) + ((a ^ b) >> 1)
}
// ANCHOR_END: midpoint

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_values_agree() {
        for a in 0..64 {
            for b in 0..64 {
                assert_eq!(midpoint(a, b), midpoint_v1(a, b));
                assert_eq!(midpoint(a, b), a.midpoint(b));
            }
        }
        assert_eq!(midpoint(u32::MAX, u32::MAX), u32::MAX);
    }
}

#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::cmd;
    use telemetry_frame as tf;

    // ANCHOR: proof_midpoint
    /// Kani ищет входы, на которых паникует хоть одна проверка, в том числе
    /// проверка переполнения: это доказательство не проходит.
    #[kani::proof]
    fn midpoint_v1_no_overflow() {
        let (a, b): (u32, u32) = (kani::any(), kani::any());
        midpoint_v1(a, b);
    }

    /// Для всех 2⁶⁴ пар — то же, что честное деление в u64, и то же, что
    /// `u32::midpoint` из std.
    #[kani::proof]
    fn midpoint_is_exact() {
        let (a, b): (u32, u32) = (kani::any(), kani::any());
        let m = midpoint(a, b);
        assert_eq!(u64::from(m), (u64::from(a) + u64::from(b)) / 2);
        assert_eq!(m, a.midpoint(b));
    }
    // ANCHOR_END: proof_midpoint

    // ANCHOR: proof_crc
    /// Шаг табличной CRC равен восьми шагам побитовой для любого состояния
    /// и любого байта; по индукции по длине данных — для любых данных.
    #[kani::proof]
    fn crc_table_step_matches_bitwise() {
        let crc: u16 = kani::any();
        let byte: u8 = kani::any();
        assert_eq!(tf::crc16_update_table(crc, &[byte]), tf::crc16_update(crc, &[byte]));
    }
    // ANCHOR_END: proof_crc

    // ANCHOR: proof_cmd
    /// Любые байты длиной до 43 — на четыре больше самой длинной команды.
    /// Цикл CRC проходит не больше 41 байта: `unwind` с запасом.
    #[kani::proof]
    #[kani::unwind(44)]
    fn parse_v1_never_panics() {
        let buf: [u8; 43] = kani::any();
        let len: usize = kani::any_where(|&n| n <= buf.len());
        let _ = cmd::parse_v1(&buf[..len]);
    }

    #[kani::proof]
    #[kani::unwind(44)]
    fn parse_never_panics() {
        let buf: [u8; 43] = kani::any();
        let len: usize = kani::any_where(|&n| n <= buf.len());
        let _ = cmd::parse(&buf[..len]);
    }
    // ANCHOR_END: proof_cmd

    // ANCHOR: vacuous
    /// Ловушка: ограничения противоречат друг другу (`n > 43` при
    /// `n <= 43`), входов нет, и доказательство «проходит» даже для
    /// `parse_v1` с её ошибкой. `kani::cover!` показывает, что до вызова
    /// не дошло ни одно исполнение.
    #[kani::proof]
    #[kani::unwind(44)]
    fn parse_v1_vacuous() {
        let buf: [u8; 43] = kani::any();
        let len: usize = kani::any_where(|&n| n <= buf.len() && n > 43);
        kani::cover!(true, "до разбора дошло хоть одно исполнение");
        let _ = cmd::parse_v1(&buf[..len]);
    }
    // ANCHOR_END: vacuous
}
