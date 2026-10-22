//! Раздел 7.2: комбинаторы `Option` и `Result` вместо вложенных `match`.
//! Замыкания разбираются в главе 12, здесь достаточно знать, что `|x| …` —
//! анонимная функция.

// ANCHOR: lookup
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupError {
    NoSensor(u16),
    /// Период опроса меньше 10 мс датчик не выдерживает.
    TooFast {
        id: u16,
        period_ms: u32,
    },
}

/// Период опроса датчика `id` по таблице `(id, период)`.
pub fn period(table: &[(u16, u32)], id: u16) -> Result<u32, LookupError> {
    let &(_, p) = table.iter().find(|(i, _)| *i == id).ok_or(LookupError::NoSensor(id))?;
    Some(p).filter(|&p| p >= 10).ok_or(LookupError::TooFast { id, period_ms: p })
}

/// То же на `match`: так выглядит код без комбинаторов.
pub fn period_match(table: &[(u16, u32)], id: u16) -> Result<u32, LookupError> {
    let mut found = None;
    for &(i, p) in table {
        if i == id {
            found = Some(p);
            break;
        }
    }
    match found {
        None => Err(LookupError::NoSensor(id)),
        Some(p) if p < 10 => Err(LookupError::TooFast { id, period_ms: p }),
        Some(p) => Ok(p),
    }
}
// ANCHOR_END: lookup

// ANCHOR: transpose
/// Необязательное поле конфигурации: отсутствие — не ошибка, а неверное
/// значение — ошибка. `Option<Result<…>>` переворачивается в
/// `Result<Option<…>>`, и `?` срабатывает только на ошибке.
pub fn optional_gain(field: Option<&str>) -> Result<Option<u8>, core::num::ParseIntError> {
    let gain = field.map(str::parse::<u8>).transpose()?;
    Ok(gain)
}
// ANCHOR_END: transpose

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combinators_match_match() {
        let t = [(1, 100), (2, 5), (7, 10)];
        for id in 0..10 {
            assert_eq!(period(&t, id), period_match(&t, id));
        }
        assert_eq!(period(&t, 1), Ok(100));
        assert_eq!(period(&t, 2), Err(LookupError::TooFast { id: 2, period_ms: 5 }));
        assert_eq!(period(&t, 3), Err(LookupError::NoSensor(3)));
    }

    #[test]
    fn transpose_optional_field() {
        assert_eq!(optional_gain(None), Ok(None));
        assert_eq!(optional_gain(Some("16")), Ok(Some(16)));
        assert!(optional_gain(Some("300")).is_err());
    }
}
