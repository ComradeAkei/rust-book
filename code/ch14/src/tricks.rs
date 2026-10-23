//! Раздел 14.8: трюки и ловушки.

use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

// ANCHOR: which_clone
/// `rc.clone()` клонирует указатель, а не строку: метод `Rc::clone`
/// найден раньше, чем `String::clone` за разыменованием.
pub fn which_clone() -> (usize, bool) {
    let rc = Rc::new(String::from("adc0"));
    let a = rc.clone(); // Rc::clone: тот же блок, счётчик 2
    let b: String = (*rc).clone(); // String::clone: новая строка
    let count = Rc::strong_count(&rc);
    drop(a);
    (count, b.as_ptr() == rc.as_ptr())
}
// ANCHOR_END: which_clone

/// В редакции 2021 временный `Ref` из условия `if let` жил до конца всего
/// `if let`, и та же функция паниковала в `else`:
///
/// ```edition2021,should_panic
/// use std::cell::RefCell;
/// fn first_or_insert(chs: &RefCell<Vec<u8>>) -> u8 {
///     if let Some(&ch) = chs.borrow().first() {
///         ch
///     } else {
///         chs.borrow_mut().push(0);
///         0
///     }
/// }
/// first_or_insert(&RefCell::new(vec![]));
/// ```
// ANCHOR: scrutinee
/// Первый канал списка; если список пуст — добавить канал 0.
pub fn first_or_insert(chs: &RefCell<Vec<u8>>) -> u8 {
    if let Some(&ch) = chs.borrow().first() {
        ch
    } else {
        chs.borrow_mut().push(0); // редакция 2024: Ref уже уничтожен
        0
    }
}

/// То же через `match`: временное значение условия живёт до конца
/// всего `match`, и `borrow_mut` в ветке паникует.
pub fn first_or_insert_match(chs: &RefCell<Vec<u8>>) -> u8 {
    match chs.borrow().first() {
        Some(&ch) => ch,
        None => {
            chs.borrow_mut().push(0);
            0
        }
    }
}
// ANCHOR_END: scrutinee

// ANCHOR: for_head
/// Удвоить каждый канал по индексу. Временный `Ref` из заголовка `for`
/// живёт, пока идёт весь цикл, и `borrow_mut` в теле паникует на первом
/// же шаге.
pub fn double_all_broken(chs: &RefCell<Vec<u8>>) {
    for i in 0..chs.borrow().len() {
        chs.borrow_mut()[i] *= 2;
    }
}

pub fn double_all(chs: &RefCell<Vec<u8>>) {
    let n = chs.borrow().len();
    for i in 0..n {
        chs.borrow_mut()[i] *= 2;
    }
}
// ANCHOR_END: for_head

// ANCHOR: rc_slice
/// Два блока: `RcInner` с вектором внутри и буфер вектора. Чтение
/// отсчёта — два перехода по указателю.
pub fn shared_vec(samples: Vec<i16>) -> Rc<Vec<i16>> {
    Rc::new(samples)
}

/// Один блок: счётчики и отсчёты подряд, `Rc<[i16]>` — толстый
/// указатель с длиной. Отсчёты копируются из вектора, а его буфер
/// освобождается.
pub fn shared_slice(samples: Vec<i16>) -> Rc<[i16]> {
    Rc::from(samples)
}
// ANCHOR_END: rc_slice

// ANCHOR: ptr_eq
#[derive(PartialEq, Debug)]
pub struct Config {
    pub period_ms: u32,
}

/// `==` у `Rc` сравнивает значения, `ptr_eq` — адреса.
pub fn same(a: &Rc<Config>, b: &Rc<Config>) -> (bool, bool) {
    (a == b, Rc::ptr_eq(a, b))
}
// ANCHOR_END: ptr_eq

// ANCHOR: cells_slice
/// Удержание пиков: каждый отсчёт становится максимумом из себя и
/// предыдущего. У `&mut [T]` нет `windows` — окна пересекаются, и
/// изменяемых ссылок было бы две на один элемент. У среза ячеек есть, и
/// писать можно в любое окно.
pub fn hold_peaks(samples: &mut [i16]) {
    let cells = Cell::from_mut(samples).as_slice_of_cells();
    for w in cells.windows(2) {
        w[1].set(w[1].get().max(w[0].get()));
    }
}
// ANCHOR_END: cells_slice

#[cfg(feature = "std")]
// ANCHOR: into_inner
/// Каждый из `n` потоков отдаёт свой клон и пытается забрать значение.
/// `into_inner` обещает, что ровно один получит `Some`. С `try_unwrap`
/// так нельзя: два потока могут одновременно увидеть счётчик 2, оба
/// получить `Err`, оба уничтожить свои клоны — и значение не достанется
/// никому.
pub fn takers(n: usize) -> usize {
    let value = alloc::sync::Arc::new(String::from("калибровка"));
    let clones: Vec<_> = (0..n).map(|_| alloc::sync::Arc::clone(&value)).collect();
    drop(value);
    let handles: Vec<_> = clones
        .into_iter()
        .map(|c| std::thread::spawn(move || alloc::sync::Arc::into_inner(c).is_some()))
        .collect();
    handles.into_iter().map(|h| h.join().unwrap()).filter(|&got| got).count()
}
// ANCHOR_END: into_inner

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn method_clone_is_rc_clone() {
        assert_eq!(which_clone(), (2, false));
    }

    #[test]
    fn if_let_releases_borrow_before_else() {
        let chs = RefCell::new(vec![]);
        assert_eq!(first_or_insert(&chs), 0);
        assert_eq!(first_or_insert(&chs), 0);
        assert_eq!(*chs.borrow(), [0]);
        assert_eq!(first_or_insert_match(&RefCell::new(vec![5])), 5);
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn match_keeps_borrow_to_the_end() {
        first_or_insert_match(&RefCell::new(vec![]));
    }

    #[test]
    #[should_panic(expected = "RefCell already borrowed")]
    fn for_head_keeps_borrow() {
        double_all_broken(&RefCell::new(vec![1, 2]));
    }

    #[test]
    fn for_with_length_first() {
        let chs = RefCell::new(vec![1, 2]);
        double_all(&chs);
        assert_eq!(*chs.borrow(), [2, 4]);
    }

    #[test]
    fn rc_slice_and_vec_hold_the_same() {
        let v = shared_vec(vec![1, 2, 3]);
        let s = shared_slice(vec![1, 2, 3]);
        assert_eq!(&v[..], &s[..]);
        assert_eq!(core::mem::size_of_val(&v), core::mem::size_of::<usize>());
        assert_eq!(core::mem::size_of_val(&s), 2 * core::mem::size_of::<usize>());
    }

    #[test]
    fn eq_compares_values() {
        let a = Rc::new(Config { period_ms: 100 });
        let b = Rc::new(Config { period_ms: 100 });
        assert_eq!(same(&a, &b), (true, false));
        assert_eq!(same(&a, &Rc::clone(&a)), (true, true));
    }

    #[test]
    fn peaks_are_held() {
        let mut s = [3, 1, 4, 1, 5, 9, 2, 6];
        hold_peaks(&mut s);
        assert_eq!(s, [3, 3, 4, 4, 5, 9, 9, 9]);
    }

    #[test]
    #[cfg(feature = "std")]
    fn into_inner_gives_exactly_one() {
        for n in [1, 2, 4, 8] {
            for _ in 0..if cfg!(miri) { 2 } else { 50 } {
                assert_eq!(takers(n), 1);
            }
        }
    }

    #[cfg(feature = "std")]
    static BROKEN: std::sync::LazyLock<u32> =
        std::sync::LazyLock::new(|| panic!("датчик не ответил"));

    #[test]
    #[cfg(feature = "std")]
    fn lazylock_is_poisoned_after_panic() {
        let first = std::panic::catch_unwind(|| *BROKEN).unwrap_err();
        assert_eq!(first.downcast_ref::<&str>(), Some(&"датчик не ответил"));
        let second = std::panic::catch_unwind(|| *BROKEN).unwrap_err();
        let msg = second
            .downcast_ref::<&str>()
            .copied()
            .or(second.downcast_ref::<String>().map(|s| s.as_str()));
        assert_eq!(msg, Some("LazyLock instance has previously been poisoned"));
    }
}
