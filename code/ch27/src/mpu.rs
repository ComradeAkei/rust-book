//! Раздел 27.7: регион MPU (PMSAv7, Cortex-M3/M4). Регион задают два
//! регистра: RBAR — начало и номер, RASR — размер, права и включение.
//! Размер — степень двойки от 32 байт, начало выровнено на размер.

// ANCHOR: mpu
/// Права доступа, поле AP регистра RASR.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Access {
    /// Никакого доступа: любое обращение — MemManage.
    None = 0b000,
    /// Чтение и запись только в привилегированном режиме.
    Privileged = 0b001,
    /// Чтение и запись в любом режиме.
    Full = 0b011,
    /// Только чтение в любом режиме.
    ReadOnly = 0b110,
}

/// Значение RBAR: начало `base` региона `region` размером `1 << size_log2`
/// байт и бит VALID, чтобы номер региона взялся из этой же записи.
pub const fn rbar(base: u32, size_log2: u8, region: u8) -> u32 {
    assert!(region < 8, "у Cortex-M3 восемь регионов");
    assert!(size_log2 >= 5 && size_log2 <= 31, "регион от 32 байт");
    assert!(base & ((1 << size_log2) - 1) == 0, "начало не выровнено на размер");
    base | 1 << 4 | region as u32
}

/// Значение RASR: регион включён, размер `1 << size_log2` байт, права
/// `ap`, `xn` — запрет выполнения кода из региона.
pub const fn rasr(size_log2: u8, ap: Access, xn: bool) -> u32 {
    assert!(size_log2 >= 5 && size_log2 <= 31, "регион от 32 байт");
    (xn as u32) << 28 | (ap as u32) << 24 | ((size_log2 - 1) as u32) << 1 | 1
}
// ANCHOR_END: mpu

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_region() {
        // 32 байта без доступа и без выполнения — сторож стека в ch27-fault.
        assert_eq!(rasr(5, Access::None, true), 0x1000_0009);
        assert_eq!(rbar(0x2000_0020, 5, 0), 0x2000_0030);
        assert_eq!(rasr(14, Access::Full, false), 0x0300_001B);
        assert_eq!(rbar(0x2000_4000, 14, 7), 0x2000_4017);
    }

    #[test]
    #[should_panic(expected = "не выровнено")]
    fn misaligned_base() {
        rbar(0x2000_0010, 5, 0);
    }
}
