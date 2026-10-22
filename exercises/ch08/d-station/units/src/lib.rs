//! Температура с датчика станции в шкале, выбранной при сборке.
use core::fmt;

#[cfg(all(feature = "celsius", feature = "fahrenheit"))]
compile_error!("units: включите одну шкалу, celsius или fahrenheit");

#[cfg(not(any(feature = "celsius", feature = "fahrenheit")))]
compile_error!("units: выберите шкалу, celsius или fahrenheit");

/// Отсчёт термодатчика: сотые доли градуса Цельсия.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Temp(pub i32);

#[cfg(feature = "celsius")]
const UNIT: &str = "°C";
#[cfg(feature = "fahrenheit")]
const UNIT: &str = "°F";

impl Temp {
    /// Значение в выбранной шкале, в сотых долях градуса.
    pub fn hundredths(self) -> i32 {
        #[cfg(feature = "celsius")]
        let v = self.0;
        #[cfg(feature = "fahrenheit")]
        let v = self.0 * 9 / 5 + 3200;
        v
    }
}

impl fmt::Display for Temp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let v = self.hundredths();
        let sign = if v < 0 { "-" } else { "" };
        let v = v.unsigned_abs();
        write!(f, "{sign}{}.{:02} {UNIT}", v / 100, v % 100)
    }
}
