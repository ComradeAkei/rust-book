//! Записи журнала ядра из `/dev/kmsg`.
//!
//! Одно чтение из `/dev/kmsg` — одна запись:
//! `уровень,номер,микросекунды,флаги;текст\n`, за которой могут идти
//! строки продолжения ` КЛЮЧ=значение\n`. Байты текста меньше 0x20 и
//! от 0x7f ядро записывает как `\xNN` — в том числе каждый байт кириллицы
//! в UTF-8. `dmesg` из util-linux раскодирует их обратно, и
//! [`parse_record`] делает то же самое.

/// Запись журнала.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// Уровень: 0 — KERN_EMERG … 7 — KERN_DEBUG.
    pub level: u8,
    /// Источник: 0 — ядро, 1 и дальше — записи из пространства пользователя.
    pub facility: u8,
    /// Порядковый номер записи.
    pub seq: u64,
    /// Время от загрузки, мкс.
    pub usec: u64,
    /// Текст без префикса модуля и перевода строки.
    pub text: String,
}

// ANCHOR: parse
/// Разбирает одну запись. `None` — не похоже на запись `/dev/kmsg`.
pub fn parse_record(raw: &[u8]) -> Option<Record> {
    let semi = raw.iter().position(|&b| b == b';')?;
    let header = std::str::from_utf8(&raw[..semi]).ok()?;
    let mut fields = header.split(',');
    let prefix: u32 = fields.next()?.parse().ok()?;
    let seq = fields.next()?.parse().ok()?;
    let usec = fields.next()?.parse().ok()?;
    // Первая строка — текст, остальные — продолжения « КЛЮЧ=значение».
    let body = &raw[semi + 1..];
    let end = body.iter().position(|&b| b == b'\n').unwrap_or(body.len());
    Some(Record {
        level: (prefix & 7) as u8,
        facility: (prefix >> 3) as u8,
        seq,
        usec,
        text: String::from_utf8_lossy(&unescape(&body[..end])).into_owned(),
    })
}

/// `\xNN` → байт; всё остальное как есть.
pub fn unescape(s: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        if s[i] == b'\\' && s.get(i + 1) == Some(&b'x') {
            let hex = s.get(i + 2..i + 4).and_then(|h| std::str::from_utf8(h).ok());
            if let Some(b) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(b);
                i += 4;
                continue;
            }
        }
        out.push(s[i]);
        i += 1;
    }
    out
}
// ANCHOR_END: parse

/// Уровни печати ядра — как `KERN_*` в `include/linux/kern_levels.h`.
pub fn level_name(level: u8) -> &'static str {
    match level {
        0 => "emerg",
        1 => "alert",
        2 => "crit",
        3 => "err",
        4 => "warn",
        5 => "notice",
        6 => "info",
        _ => "debug",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_header_and_unescapes_utf8() {
        // Запись `pr_info!("привет #0\n")` модуля ch29_hello в том виде, в
        // каком её отдаёт /dev/kmsg; номер и время — для примера.
        let raw = b"6,512,1251240,-;ch29_hello: \\xd0\\xbf\\xd1\\x80\\xd0\\xb8\\xd0\\xb2\\xd0\\xb5\\xd1\\x82 #0\n";
        let r = parse_record(raw).unwrap();
        assert_eq!((r.level, r.facility, r.seq, r.usec), (6, 0, 512, 1251240));
        assert_eq!(r.text, "ch29_hello: привет #0");
    }

    #[test]
    fn continuation_lines_are_not_text() {
        let raw = b"4,7,10,-;ch29: tainted\n SUBSYSTEM=module\n DEVICE=+module:ch29\n";
        let r = parse_record(raw).unwrap();
        assert_eq!(r.text, "ch29: tainted");
        assert_eq!(level_name(r.level), "warn");
    }

    #[test]
    fn user_facility_and_garbage() {
        // Запись в /dev/kmsg из пространства пользователя: LOG_USER = 1.
        let r = parse_record(b"14,9,20,-;init: hello\n").unwrap();
        assert_eq!((r.level, r.facility), (6, 1));
        assert_eq!(parse_record(b"no header here"), None);
        assert_eq!(unescape(b"\\x4"), b"\\x4", "обрезанная последовательность — как есть");
        assert_eq!(unescape(b"a\\xzzb"), b"a\\xzzb");
    }
}
