//! Задача C: запечатанные трейты выводов GPIO и расширение для `u32`.
//!
//! Файл самодостаточен: тест собирает его ещё и отдельно, как библиотеку
//! `pins`, и пробует пользоваться ею из другого крейта. Не ссылайтесь из
//! него на другие модули крейта.

/// Базовый адрес GPIOA (STM32F103); порты идут через 0x400.
pub const GPIOA: usize = 0x4001_0800;

/// Вывод порта GPIO. Должен быть запечатан: выводы задаёт только этот
/// крейт.
pub trait PinId {
    /// Порт: 0 — A, 1 — B.
    const PORT: u8;
    /// Номер вывода в порту.
    const N: u8;
    /// Маска вывода в регистрах порта.
    const MASK: u32 = 1 << Self::N;
}

macro_rules! pins {
    ($($name:ident: $port:literal, $n:literal;)*) => {
        $(
            pub struct $name;
            impl PinId for $name {
                const PORT: u8 = $port;
                const N: u8 = $n;
            }
        )*
    };
}

pins! {
    PA0: 0, 0; PA1: 0, 1; PA2: 0, 2; PA3: 0, 3;
    PA4: 0, 4; PA5: 0, 5; PA6: 0, 6; PA7: 0, 7;
    PB0: 1, 0; PB1: 1, 1; PB2: 1, 2; PB3: 1, 3;
    PB4: 1, 4; PB5: 1, 5; PB6: 1, 6; PB7: 1, 7;
}

/// Адрес регистра BSRR порта, к которому относится вывод.
pub fn bsrr_addr<P: PinId>() -> usize {
    todo!("GPIOA, шаг 0x400 на порт, смещение BSRR 0x10")
}

/// Значение для BSRR: младшие 16 бит включают вывод, старшие — выключают.
pub fn bsrr<P: PinId>(high: bool) -> u32 {
    todo!()
}

/// Битовые поля регистров. Тоже должно быть запечатано.
pub trait U32Ext {
    /// Бит `n`.
    fn bit(self, n: u32) -> bool;
    /// То же число, но бит `n` равен `v`.
    fn with_bit(self, n: u32, v: bool) -> Self;
    /// Поле шириной `width` бит, начиная с бита `lsb`.
    fn field(self, lsb: u32, width: u32) -> u32;
    /// То же число, но поле заменено на `value`.
    fn with_field(self, lsb: u32, width: u32, value: u32) -> Self;
}

impl U32Ext for u32 {
    fn bit(self, n: u32) -> bool {
        todo!()
    }

    fn with_bit(self, n: u32, v: bool) -> u32 {
        todo!()
    }

    fn field(self, lsb: u32, width: u32) -> u32 {
        todo!()
    }

    fn with_field(self, lsb: u32, width: u32, value: u32) -> u32 {
        todo!()
    }
}
