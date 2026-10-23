//! Задача C: запечатанные трейты выводов GPIO и расширение для `u32`.
//!
//! Файл самодостаточен: тест собирает его ещё и отдельно, как библиотеку
//! `pins`, и пробует пользоваться ею из другого крейта.

// ANCHOR: seal
mod private {
    /// Печать для `PinId` и `U32Ext`. Модуль приватный: другой крейт не
    /// может назвать `Sealed`, а значит, не может и реализовать.
    pub trait Sealed {}
}
// ANCHOR_END: seal

/// Базовый адрес GPIOA (STM32F103); порты идут через 0x400.
pub const GPIOA: usize = 0x4001_0800;

// ANCHOR: pins
/// Вывод порта GPIO. Список выводов закрыт: их задаёт только этот крейт.
pub trait PinId: private::Sealed {
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
            impl private::Sealed for $name {}
            impl PinId for $name {
                const PORT: u8 = $port;
                const N: u8 = $n;
            }
        )*
    };
}
// ANCHOR_END: pins

pins! {
    PA0: 0, 0; PA1: 0, 1; PA2: 0, 2; PA3: 0, 3;
    PA4: 0, 4; PA5: 0, 5; PA6: 0, 6; PA7: 0, 7;
    PB0: 1, 0; PB1: 1, 1; PB2: 1, 2; PB3: 1, 3;
    PB4: 1, 4; PB5: 1, 5; PB6: 1, 6; PB7: 1, 7;
}

// ANCHOR: bsrr
/// Адрес регистра BSRR порта, к которому относится вывод.
pub fn bsrr_addr<P: PinId>() -> usize {
    GPIOA + 0x400 * usize::from(P::PORT) + 0x10
}

/// Значение для BSRR: младшие 16 бит включают вывод, старшие — выключают.
/// Запись в BSRR атомарна: другие выводы порта она не трогает.
pub fn bsrr<P: PinId>(high: bool) -> u32 {
    if high { P::MASK } else { P::MASK << 16 }
}
// ANCHOR_END: bsrr

// ANCHOR: ext
/// Битовые поля регистров. Расширение тоже запечатано: новые методы с
/// реализацией по умолчанию не столкнутся с чужими реализациями.
pub trait U32Ext: private::Sealed {
    /// Бит `n`.
    fn bit(self, n: u32) -> bool;
    /// То же число, но бит `n` равен `v`.
    fn with_bit(self, n: u32, v: bool) -> Self;
    /// Поле шириной `width` бит, начиная с бита `lsb`.
    fn field(self, lsb: u32, width: u32) -> u32;
    /// То же число, но поле заменено на `value`.
    fn with_field(self, lsb: u32, width: u32, value: u32) -> Self;
}

impl private::Sealed for u32 {}

/// Маска поля, прижатая к нулевому биту. `1 << width` при ширине 32
/// переполнилось бы, поэтому маску сдвигаем вправо.
fn mask(lsb: u32, width: u32) -> u32 {
    assert!(
        width > 0 && lsb.checked_add(width).is_some_and(|end| end <= 32),
        "поле из {width} бит с бита {lsb} не помещается в u32"
    );
    u32::MAX >> (32 - width)
}

impl U32Ext for u32 {
    fn bit(self, n: u32) -> bool {
        self.field(n, 1) != 0
    }

    fn with_bit(self, n: u32, v: bool) -> u32 {
        self.with_field(n, 1, u32::from(v))
    }

    fn field(self, lsb: u32, width: u32) -> u32 {
        // Маску считаем до сдвига: она же проверяет, что сдвиг меньше 32.
        let m = mask(lsb, width);
        self >> lsb & m
    }

    fn with_field(self, lsb: u32, width: u32, value: u32) -> u32 {
        let m = mask(lsb, width);
        assert!(value <= m, "значение {value:#x} не помещается в {width} бит");
        self & !(m << lsb) | value << lsb
    }
}
// ANCHOR_END: ext
