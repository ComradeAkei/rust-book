//! Раздел 17.1: `MaybeUninit<T>` — память под значение, которое ещё не
//! инициализировано. Это честная замена «сырой памяти» из C: тип прямо
//! говорит, что внутри может не быть корректного `T`, и прочитать его
//! можно только через `assume_init`, взяв на себя ответственность. Ссылку
//! на неинициализированное брать нельзя — это уже UB, поэтому пишут через
//! сырые указатели.

use core::mem::MaybeUninit;

// ANCHOR: parts
/// Собрать значение по частям: выделить место под `Point`, записать поля
/// через сырые указатели и только когда заданы оба — объявить готовым.
pub struct Point {
    pub x: u32,
    pub y: u32,
}

pub fn assemble(x: u32, y: u32) -> Point {
    let mut uninit = MaybeUninit::<Point>::uninit();
    let p = uninit.as_mut_ptr();
    // SAFETY: p указывает на выделенную, но не инициализированную память под
    // Point. Пишем каждое поле через `&raw mut`, не создавая ссылки на
    // незаполненное. Когда оба поля заданы, значение корректно.
    unsafe {
        (&raw mut (*p).x).write(x);
        (&raw mut (*p).y).write(y);
        uninit.assume_init()
    }
}
// ANCHOR_END: parts

// ANCHOR: array
/// Инициализировать массив поэлементно, не требуя `Copy` или `Default` и
/// не заводя значения-заглушки.
pub fn squares<const N: usize>() -> [u32; N] {
    let mut arr: [MaybeUninit<u32>; N] = [const { MaybeUninit::uninit() }; N];
    for (i, slot) in arr.iter_mut().enumerate() {
        slot.write((i * i) as u32);
    }
    // SAFETY: цикл задал все N элементов. Раскладка [MaybeUninit<u32>; N]
    // совпадает с [u32; N], поэтому массив читается как готовый.
    unsafe { (arr.as_ptr() as *const [u32; N]).read() }
}
// ANCHOR_END: array

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assemble_sets_both_fields() {
        let p = assemble(3, 4);
        assert_eq!((p.x, p.y), (3, 4));
    }

    #[test]
    fn squares_initialized_elementwise() {
        assert_eq!(squares::<5>(), [0, 1, 4, 9, 16]);
    }
}
