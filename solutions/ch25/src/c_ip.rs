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
    /// Ведущий ноль: `inet_aton` прочтёт «010» как восьмеричное 8, поэтому
    /// такую запись не принимают ни здесь, ни `str::parse::<Ipv4Addr>`.
    LeadingZero,
}

// ANCHOR: parse
/// Разбор записи «a.b.c.d». `const fn`: работает и при выполнении, и при
/// компиляции, поэтому без `for`, итераторов и `?` — только `while`,
/// индексы и `return`.
pub const fn parse_ipv4(s: &str) -> Result<[u8; 4], IpError> {
    let b = s.as_bytes();
    let mut octets = [0u8; 4];
    let mut n = 0; // сколько октетов уже записано
    let mut value = 0u32;
    let mut digits = 0;
    let mut i = 0;
    // Лишний шаг при i == b.len() закрывает последний октет.
    while i <= b.len() {
        if i == b.len() || b[i] == b'.' {
            if digits == 0 {
                return Err(IpError::Empty);
            }
            if n == 4 {
                return Err(IpError::Count);
            }
            octets[n] = value as u8;
            n += 1;
            value = 0;
            digits = 0;
        } else {
            let c = b[i];
            if !c.is_ascii_digit() {
                return Err(IpError::Digit);
            }
            if digits == 1 && value == 0 {
                return Err(IpError::LeadingZero);
            }
            value = value * 10 + (c - b'0') as u32;
            digits += 1;
            // Проверка на каждой цифре: value не больше 2559 и не
            // переполняется, сколько бы цифр ни было.
            if value > 255 {
                return Err(IpError::Range);
            }
        }
        i += 1;
    }
    if n == 4 { Ok(octets) } else { Err(IpError::Count) }
}
// ANCHOR_END: parse

// ANCHOR: fail
impl IpError {
    /// Паника с текстом ошибки. В константе `panic!` не форматирует сам
    /// `IpError` (раздел 25.3), поэтому по литералу на вариант.
    pub const fn fail(self) -> ! {
        match self {
            IpError::Count => panic!("IPv4: нужно ровно четыре октета"),
            IpError::Empty => panic!("IPv4: пустой октет"),
            IpError::Digit => panic!("IPv4: в октете не десятичная цифра"),
            IpError::Range => panic!("IPv4: октет больше 255"),
            IpError::LeadingZero => panic!("IPv4: ведущий ноль в октете"),
        }
    }
}

/// Адрес или паника. При выполнении паника — при запуске, в константе —
/// ошибка сборки E0080 с тем же текстом.
pub const fn ipv4(s: &str) -> Ipv4Addr {
    match parse_ipv4(s) {
        Ok(o) => Ipv4Addr::new(o[0], o[1], o[2], o[3]),
        Err(e) => e.fail(),
    }
}
// ANCHOR_END: fail

// ANCHOR: macro
/// `ip!("10.0.0.1")` — адрес, разобранный при компиляции. Блок `const`
/// заставляет вычислить `ipv4` при сборке, даже когда макрос стоит в теле
/// обычной функции: опечатка в адресе — ошибка сборки, а не паника в поле.
#[macro_export]
macro_rules! ip {
    ($s:literal) => {
        const { $crate::c_ip::ipv4($s) }
    };
}
// ANCHOR_END: macro
