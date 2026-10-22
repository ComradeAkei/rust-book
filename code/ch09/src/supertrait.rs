//! Раздел 9.3: супертрейты и методы по умолчанию.
//!
//! Супертрейт — требование, а не наследование: реализовать `Probe` можно
//! только для типа, у которого уже есть `Named` и `Debug`.
//!
//! ```compile_fail,E0277
//! use ch09::supertrait::{Probe, ProbeError};
//! #[derive(Debug)]
//! struct Nameless;
//! impl Probe for Nameless {
//!     fn probe(&mut self) -> Result<u8, ProbeError> { Ok(0) }
//! }
//! ```
//!
//! Обязательный метод нельзя пропустить, метод по умолчанию — можно.
//!
//! ```compile_fail,E0046
//! use ch09::sensor::Sensor;
//! struct Silent;
//! impl Sensor for Silent {}
//! ```

use core::fmt;

// ANCHOR: supertraits
/// Имя устройства на шине.
pub trait Named {
    fn name(&self) -> &'static str;
}

/// Почему устройство не ответило.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeError {
    NoAck,
    WrongId(u8),
}

/// Опрос устройства. Чтобы реализовать `Probe`, тип должен уже уметь
/// назвать себя и печататься через `{:?}`.
pub trait Probe: Named + fmt::Debug {
    /// Прочитать идентификатор устройства.
    fn probe(&mut self) -> Result<u8, ProbeError>;

    /// Строка для журнала. Метод по умолчанию пользуется методами
    /// супертрейтов так же, как своими.
    fn report(&mut self, out: &mut dyn fmt::Write) -> fmt::Result {
        match self.probe() {
            Ok(id) => write!(out, "{}: id {id:#04x}", self.name()),
            Err(e) => write!(out, "{}: {e:?}, состояние {self:?}", self.name()),
        }
    }
}
// ANCHOR_END: supertraits

// ANCHOR: impls
#[derive(Debug)]
pub struct Bmp {
    pub addr: u8,
}

impl Named for Bmp {
    fn name(&self) -> &'static str {
        "BMP280"
    }
}

impl Probe for Bmp {
    fn probe(&mut self) -> Result<u8, ProbeError> {
        if self.addr == 0x76 { Ok(0x58) } else { Err(ProbeError::NoAck) }
    }
}

/// Датчик, у которого свой отчёт: метод по умолчанию заменён.
#[derive(Debug)]
pub struct Eeprom;

impl Named for Eeprom {
    fn name(&self) -> &'static str {
        "24C02"
    }
}

impl Probe for Eeprom {
    fn probe(&mut self) -> Result<u8, ProbeError> {
        Err(ProbeError::WrongId(0xff))
    }

    fn report(&mut self, out: &mut dyn fmt::Write) -> fmt::Result {
        out.write_str("24C02: идентификатора нет, это память")
    }
}
// ANCHOR_END: impls

// ANCHOR: bound
/// Ограничение `T: Probe` открывает и методы супертрейтов.
pub fn first_name<T: Probe>(devices: &[T]) -> Option<&'static str> {
    devices.first().map(|d| d.name())
}
// ANCHOR_END: bound

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    #[test]
    fn default_and_override() {
        let mut s = String::new();
        Bmp { addr: 0x76 }.report(&mut s).unwrap();
        s.push('\n');
        Bmp { addr: 0x77 }.report(&mut s).unwrap();
        s.push('\n');
        Eeprom.report(&mut s).unwrap();
        assert_eq!(
            s,
            "BMP280: id 0x58\n\
             BMP280: NoAck, состояние Bmp { addr: 119 }\n\
             24C02: идентификатора нет, это память"
        );
        assert_eq!(first_name(&[Bmp { addr: 1 }]), Some("BMP280"));
    }
}
