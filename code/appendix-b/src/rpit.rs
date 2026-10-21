//! Раздел B.3: захват времён жизни в `impl Trait` и `use<..>`.

// ANCHOR: rpit
/// В 2024 скрытый тип `impl Trait` захватывает все параметры из области
/// видимости, включая время жизни среза. Здесь так и нужно: итератор
/// читает срез. В 2021 эта функция не компилировалась.
pub fn bytes(v: &[u8]) -> impl Iterator<Item = u8> {
    v.iter().copied()
}

/// Итератору по индексам сам срез не нужен. `use<>` говорит, что тип не
/// захватывает ничего, и после вызова срез можно менять.
pub fn indices(v: &[u8]) -> impl Iterator<Item = usize> + use<> {
    0..v.len()
}
// ANCHOR_END: rpit

// ANCHOR: binding_modes
/// Увеличенная копия значения, если оно есть.
pub fn bumped(opt: &Option<u32>) -> Option<u32> {
    // В 2021 писали `if let Some(mut x) = opt`, и `mut` молча превращал
    // привязку по ссылке в копию. В 2024 это ошибка: образец ссылки
    // пишется явно, и тогда `mut x` снова означает копию.
    if let &Some(mut x) = opt {
        x += 1;
        Some(x)
    } else {
        None
    }
}
// ANCHOR_END: binding_modes

/// `Future` и `IntoFuture` в 2024 входят в прелюдию: импорт не нужен.
pub fn is_future<F: Future>(_: &F) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_rules() {
        let mut buf = vec![1u8, 2, 3];
        assert_eq!(bytes(&buf).sum::<u8>(), 6);
        let it = indices(&buf);
        buf.push(4); // `it` не держит заимствование `buf`
        assert_eq!(it.count(), 3);
    }

    #[test]
    fn explicit_reference_pattern() {
        assert_eq!(bumped(&Some(5)), Some(6));
        assert_eq!(bumped(&None), None);
    }

    #[test]
    fn future_in_prelude() {
        assert!(is_future(&async {}));
    }
}
