//! Раздел 2.4: `if let`, `while let`, `let else`, цепочки условий,
//! `matches!`.

// ANCHOR: if_let
/// Номер канала из команды вида `ch3`. `if let` — `match` с одной
/// интересной веткой.
pub fn channel(cmd: &[u8]) -> Option<u8> {
    if let [b'c', b'h', d @ b'0'..=b'7'] = cmd { Some(d - b'0') } else { None }
}
// ANCHOR_END: if_let

// ANCHOR: while_let
/// Сумма значений в записях TLV: тег, длина, данные. `while let` со
/// срезовым образцом разбирает записи, пока они есть; хвост короче
/// заголовка или длины — ошибка.
pub fn tlv_sum(mut buf: &[u8]) -> Option<u32> {
    let mut sum = 0u32;
    while let [_tag, len, rest @ ..] = buf {
        let len = usize::from(*len);
        if rest.len() < len {
            return None;
        }
        let (value, tail) = rest.split_at(len);
        sum += value.iter().map(|&b| u32::from(b)).sum::<u32>();
        buf = tail;
    }
    if buf.is_empty() { Some(sum) } else { None }
}
// ANCHOR_END: while_let

// ANCHOR: let_else
/// Ошибки разбора строки настройки.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingError {
    NoEquals,
    BadNumber,
}

/// Строка настройки `ключ = значение`. `let else` разбирает значение или
/// уходит из функции: ветка `else` обязана иметь тип `!`.
pub fn parse_setting(line: &str) -> Result<(&str, u32), SettingError> {
    let Some((key, value)) = line.split_once('=') else {
        return Err(SettingError::NoEquals);
    };
    let Ok(value) = value.trim().parse::<u32>() else {
        return Err(SettingError::BadNumber);
    };
    Ok((key.trim(), value))
}
// ANCHOR_END: let_else

// ANCHOR: chains
/// Команда `baud=<скорость>` с проверкой по списку. Цепочка из
/// `let`-условий и обычных условий через `&&` — edition 2024.
pub fn baud_command(line: &str) -> Option<u32> {
    if let Some((key, value)) = line.split_once('=')
        && key == "baud"
        && let Ok(rate) = value.parse::<u32>()
        && matches!(rate, 9600 | 19_200 | 38_400 | 57_600 | 115_200)
    {
        Some(rate)
    } else {
        None
    }
}
// ANCHOR_END: chains

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels() {
        assert_eq!(channel(b"ch3"), Some(3));
        assert_eq!(channel(b"ch8"), None);
        assert_eq!(channel(b"ch"), None);
    }

    #[test]
    fn tlv() {
        assert_eq!(tlv_sum(&[1, 2, 10, 20, 2, 0, 3, 1, 5]), Some(35));
        assert_eq!(tlv_sum(&[]), Some(0));
        assert_eq!(tlv_sum(&[1, 3, 10]), None);
        assert_eq!(tlv_sum(&[1, 0, 7]), None);
    }

    #[test]
    fn settings() {
        assert_eq!(parse_setting("period = 500"), Ok(("period", 500)));
        assert_eq!(parse_setting("period"), Err(SettingError::NoEquals));
        assert_eq!(parse_setting("period = fast"), Err(SettingError::BadNumber));
    }

    #[test]
    fn baud() {
        assert_eq!(baud_command("baud=115200"), Some(115_200));
        assert_eq!(baud_command("baud=1200"), None);
        assert_eq!(baud_command("speed=9600"), None);
        assert_eq!(baud_command("baud=fast"), None);
    }
}
