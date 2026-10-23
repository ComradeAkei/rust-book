//! Задача B: `split_first_mut` вручную. Заёмщик не позволяет взять `&mut`
//! на `s[0]` и `&mut s[1..]` одновременно, хотя памяти они не делят. Весь
//! `unsafe` заперт внутри; снаружи сигнатура безопасна, а непересечение
//! отрезков доказано.

// ANCHOR: solution
/// Первый элемент и хвост среза как непересекающиеся `&mut`. То же, что
/// `slice::split_first_mut`, но написано руками.
pub fn split_first_mut<T>(s: &mut [T]) -> Option<(&mut T, &mut [T])> {
    let len = s.len();
    if len == 0 {
        return None;
    }
    let p: *mut T = s.as_mut_ptr();
    // SAFETY: срез непуст, поэтому p указывает на живой, выровненный,
    // инициализированный s[0], а p.add(1) — на начало хвоста длины len-1
    // в границах того же выделения. Отрезки [0, 1) и [1, len) не
    // пересекаются, значит две &mut не алиасят. Исходный &mut s больше не
    // используется, пока живут выданные ссылки.
    unsafe {
        let head = &mut *p;
        let tail = core::slice::from_raw_parts_mut(p.add(1), len - 1);
        Some((head, tail))
    }
}
// ANCHOR_END: solution
