//! Задача C: IPv4-адрес из строки — разбор при компиляции и ошибка сборки
//! вместо ошибки при запуске.

use core::net::Ipv4Addr;

/// Чем строка не похожа на адрес.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpError {
    /// Октетов не четыре.
    Count,
    /// Пустой октет: «1..2.3», точка в начале или в конце.
    Empty,
    /// В октете не десятичная цифра.
    Digit,
    /// Октет больше 255.
    Range,
    /// Ведущий ноль: «010».
    LeadingZero,
}

/// Разбор записи «a.b.c.d».
pub const fn parse_ipv4(s: &str) -> Result<[u8; 4], IpError> {
    // TODO(задача C)
    let _ = s;
    todo!()
}

impl IpError {
    /// Паника с текстом ошибки: «IPv4: нужно ровно четыре октета»,
    /// «IPv4: пустой октет», «IPv4: в октете не десятичная цифра»,
    /// «IPv4: октет больше 255», «IPv4: ведущий ноль в октете».
    pub const fn fail(self) -> ! {
        // TODO(задача C)
        todo!()
    }
}

/// Адрес или паника с текстом ошибки.
pub const fn ipv4(s: &str) -> Ipv4Addr {
    // TODO(задача C)
    let _ = s;
    todo!()
}

// ANCHOR: macro
/// `ip!("10.0.0.1")` — адрес, разобранный при компиляции.
#[macro_export]
macro_rules! ip {
    ($s:literal) => {
        // TODO(задача C): сейчас адрес разбирается при выполнении.
        $crate::c_ip::ipv4($s)
    };
}
// ANCHOR_END: macro
