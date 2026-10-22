//! Раздел 5.7: какие времена жизни захватывает `impl Trait`.
//!
//! В редакции 2024 возвращаемый `impl Trait` захватывает все параметры
//! времени жизни функции. Итератор из `section_all` считается
//! заимствующим и имя секции, поэтому временная строка с именем должна
//! жить, пока жив итератор:
//!
//! ```compile_fail,E0716
//! use ch05::capture::{Config, section_all};
//!
//! let cfg = Config::parse("[uart]\nbaud=9600\n");
//! let vals = section_all(&cfg, &String::from("uart"));
//! assert_eq!(vals.count(), 1);
//! ```

use alloc::string::String;
use alloc::vec::Vec;

/// Конфигурация, разобранная на строки: без пустых строк и комментариев.
pub struct Config {
    lines: Vec<String>,
}

impl Config {
    pub fn parse(text: &str) -> Config {
        let lines = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect();
        Config { lines }
    }

    /// Строки секции `[name]`: от её заголовка до следующего.
    fn range(&self, name: &str) -> core::ops::Range<usize> {
        let is_header = |l: &String| l.starts_with('[') && l.ends_with(']');
        let Some(start) = self.lines.iter().position(|l| {
            is_header(l) && l.strip_prefix('[').and_then(|l| l.strip_suffix(']')) == Some(name)
        }) else {
            return 0..0;
        };
        let len = self.lines[start + 1..].iter().position(is_header);
        start + 1..len.map_or(self.lines.len(), |n| start + 1 + n)
    }
}

// ANCHOR: capture
/// Строки секции. Имя нужно только для поиска границ, но возвращаемый тип
/// по правилам редакции 2024 захватывает и `'n`.
pub fn section_all<'c, 'n>(cfg: &'c Config, name: &'n str) -> impl Iterator<Item = &'c str> {
    cfg.lines[cfg.range(name)].iter().map(String::as_str)
}

/// `use<'c>` перечисляет захваченное явно: итератор держит только
/// конфигурацию, имя можно отдать временной строкой.
pub fn section<'c>(cfg: &'c Config, name: &str) -> impl Iterator<Item = &'c str> + use<'c> {
    cfg.lines[cfg.range(name)].iter().map(String::as_str)
}
// ANCHOR_END: capture

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::vec::Vec;

    const TEXT: &str = "# станция\n[uart]\nbaud=9600\nmode=8N1\n\n[spi]\nclock=1000000\n";

    #[test]
    fn sections() {
        let cfg = Config::parse(TEXT);
        let uart: Vec<_> = section(&cfg, &format!("ua{}", "rt")).collect();
        assert_eq!(uart, ["baud=9600", "mode=8N1"]);
        let name = String::from("spi");
        assert_eq!(section_all(&cfg, &name).collect::<Vec<_>>(), ["clock=1000000"]);
        assert_eq!(section(&cfg, "i2c").count(), 0);
    }
}
