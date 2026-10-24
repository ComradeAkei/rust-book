//! Раздел 30.6: ветвление против кода без ветвлений.

// ANCHOR: select
/// Отобрать байты не меньше 128 в `out`: запись только для отобранных —
/// условный переход на каждом байте. Возвращает, сколько отобрано.
#[inline(never)]
pub fn select_branchy(data: &[u8], out: &mut Vec<u8>) -> usize {
    out.clear();
    for &b in data {
        if b >= 128 {
            out.push(b);
        }
    }
    out.len()
}

/// То же без перехода: байт пишется всегда, а позиция сдвигается на
/// `(b >= 128) as usize`. Лишние записи затирает следующий байт.
#[inline(never)]
pub fn select_branchless(data: &[u8], out: &mut Vec<u8>) -> usize {
    out.clear();
    out.resize(data.len(), 0);
    let mut n = 0;
    for (i, &b) in data.iter().enumerate() {
        // n <= i < data.len() == out.len(): индекс всегда в границах.
        out[n.min(i)] = b;
        n += usize::from(b >= 128);
    }
    out.truncate(n);
    n
}
// ANCHOR_END: select

// ANCHOR: count
/// Подсчёт без записи: здесь LLVM сам обходится без перехода.
#[inline(never)]
pub fn count_big(data: &[u8]) -> usize {
    data.iter().filter(|&&b| b >= 128).count()
}
// ANCHOR_END: count

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_selection() {
        let data: Vec<u8> = (0..10_000u32).map(|i| (i * 7919 % 256) as u8).collect();
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let n = select_branchy(&data, &mut a);
        assert_eq!(select_branchless(&data, &mut b), n);
        assert_eq!(a, b);
        assert_eq!(count_big(&data), n);
    }
}
