//! Раздел 27.2: приоритеты исключений Cortex-M. Приоритет — байт, но
//! кристалл хранит только несколько его старших битов (у LM3S6965 — три).
//! Меньшее число — более важное исключение.

// ANCHOR: encode
/// Байт приоритета для уровня `level`, если реализовано `bits` старших
/// битов. Уровни 0, 1, 2 при трёх битах — байты 0x00, 0x20, 0x40.
pub const fn encode(level: u8, bits: u8) -> u8 {
    assert!(bits >= 1 && bits <= 8, "битов приоритета от 1 до 8");
    assert!((level as u16) < (1 << bits), "уровень не помещается в биты");
    ((level as u16) << (8 - bits)) as u8
}

/// Что прочитается из регистра приоритета после записи `value`:
/// нереализованные младшие биты читаются нулями.
pub const fn stored(value: u8, bits: u8) -> u8 {
    value & !((0xFF_u16 >> bits) as u8)
}
// ANCHOR_END: encode

// ANCHOR: preempt
/// Групповой приоритет: при поле PRIGROUP регистра AIRCR, равном
/// `prigroup`, младшие `prigroup + 1` битов байта — подприоритет, он на
/// вытеснение не влияет. После сброса PRIGROUP = 0.
pub const fn group(prio: u8, prigroup: u8) -> u8 {
    assert!(prigroup < 8);
    prio & !(((2u16 << prigroup) - 1) as u8)
}

/// Вытеснит ли исключение с приоритетом `new` код, который выполняется
/// с приоритетом `running`. Равный групповой приоритет не вытесняет:
/// такое исключение ждёт и выполняется следом.
pub const fn preempts(new: u8, running: u8, prigroup: u8) -> bool {
    group(new, prigroup) < group(running, prigroup)
}

/// Задерживает ли BASEPRI исключение с приоритетом `prio`. BASEPRI = 0 —
/// маски нет; иначе ждут все с приоритетом, численно не меньшим.
pub const fn masked(prio: u8, basepri: u8) -> bool {
    basepri != 0 && prio >= basepri
}
// ANCHOR_END: preempt

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_bits() {
        assert_eq!([encode(0, 3), encode(1, 3), encode(2, 3), encode(7, 3)], [0, 0x20, 0x40, 0xE0]);
        assert_eq!(encode(1, 8), 1);
        assert_eq!(stored(0xFF, 3), 0xE0);
        assert_eq!(stored(0x5F, 3), 0x40);
        // QEMU 8.2 хранит у lm3s6965evb все восемь битов (раздел 27.2).
        assert_eq!(stored(0xFF, 8), 0xFF);
    }

    #[test]
    fn preemption() {
        // Опыт ch27-nvic: GPIOB (0x40) вытесняет GPIOA (0x80), GPIOC (0xC0) — нет.
        assert!(preempts(0x40, 0x80, 0));
        assert!(!preempts(0xC0, 0x80, 0));
        assert!(!preempts(0x80, 0x80, 0));
        // PRIGROUP = 5: в группу идут биты 7–6, 0x40 и 0x60 — одна группа.
        assert!(preempts(0x40, 0x80, 5));
        assert!(!preempts(0x40, 0x60, 5));
        assert_eq!(group(0x7F, 5), 0x40);
    }

    #[test]
    fn basepri() {
        assert!(!masked(0x80, 0));
        assert!(masked(0x80, 0x80));
        assert!(masked(0xC0, 0x80));
        assert!(!masked(0x40, 0x80));
        assert!(masked(0x40, 0x40));
    }
}
