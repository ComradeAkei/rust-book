// Общие тесты главы 18 для задач A–D: подключаются через include! в крейты
// заготовок и решений после строки `use <крейт> as sut;`.
//
// Задачи A, B и D зовут C (libc, glibc, zlib) и помечены #[cfg_attr(miri,
// ignore)] — Miri чужой код не исполняет. Задача C делится надвое: логику
// FIFO проверяет обычный Rust-тест здесь, а экспорт в C — драйвер c/run.sh.

use std::cmp::Ordering;
use std::ffi::CString;
use std::panic;

// --- Задача A: каталог объявлений ---

#[test]
#[cfg_attr(miri, ignore = "вызывает libc")]
fn a_libc_wrappers_match_c() {
    let s = CString::new("hello").unwrap();
    assert_eq!(sut::a_decls::c_strlen(&s), 5);
    assert_eq!(sut::a_decls::c_abs(-7), Some(7));
    assert_eq!(sut::a_decls::c_abs(i32::MIN), None); // непредставимо
    assert_eq!(sut::a_decls::c_memcmp(b"abc", b"abc"), 0);
    assert!(sut::a_decls::c_memcmp(b"abc", b"abd") < 0);
    assert_eq!(sut::a_decls::c_toupper(b'x'), b'X');
    assert_eq!(sut::a_decls::c_toupper(b'7'), b'7'); // не буква — без изменений
    assert_eq!(sut::a_decls::c_strchr(&s, b'l'), Some(2));
    assert_eq!(sut::a_decls::c_strchr(&s, b'z'), None);
}

// --- Задача B: qsort_r с замыканием ---

#[test]
#[cfg_attr(miri, ignore = "вызывает glibc")]
fn b_sorts_with_closure() {
    let mut v = [5, 3, 9, 1, 7];
    sut::b_qsort::sort_by(&mut v, |a, b| a.cmp(b));
    assert_eq!(v, [1, 3, 5, 7, 9]);

    // Порядок по убыванию через захваченное направление.
    let mut v2 = [5, 3, 9, 1, 7];
    let descending = true;
    sut::b_qsort::sort_by(&mut v2, |a, b| if descending { b.cmp(a) } else { a.cmp(b) });
    assert_eq!(v2, [9, 7, 5, 3, 1]);
}

#[test]
#[cfg_attr(miri, ignore = "вызывает glibc")]
fn b_panic_in_comparator_is_caught() {
    let mut v = [3, 1, 2];
    let prev = panic::take_hook();
    panic::set_hook(Box::new(|_| {})); // не печатать панику в тесте
    // Компаратор паникует на каждом сравнении: граница C пересечься не должна
    // (это было бы UB) — паника ловится, qsort_r завершается.
    sut::b_qsort::sort_by(&mut v, |_, _| -> Ordering { panic!("порядок не задан") });
    panic::set_hook(prev);
    assert_eq!(v.len(), 3); // процесс жив, массив цел
}

// --- Задача C: кольцевой буфер (логика FIFO) ---

#[test]
fn c_ring_is_fifo_and_bounded() {
    let mut rb = sut::c_ring::RingBuffer::with_capacity(3);
    assert!(rb.is_empty());
    assert!(rb.push(1));
    assert!(rb.push(2));
    assert!(rb.push(3));
    assert!(!rb.push(4)); // полон
    assert_eq!(rb.len(), 3);

    assert_eq!(rb.pop(), Some(1));
    assert!(rb.push(4)); // место освободилось, индекс пошёл по кругу
    assert_eq!(rb.pop(), Some(2));
    assert_eq!(rb.pop(), Some(3));
    assert_eq!(rb.pop(), Some(4));
    assert_eq!(rb.pop(), None);
    assert!(rb.is_empty());
}

// --- Задача D: обёртка над zlib ---

#[test]
#[cfg_attr(miri, ignore = "вызывает zlib")]
fn d_zlib_roundtrip() {
    let data = b"telemetry frame \x00\x01\x02 telemetry frame telemetry frame".repeat(16);
    let packed = sut::d_zlib::compress(&data, 6);
    assert!(packed.len() < data.len(), "повторяющиеся данные обязаны сжаться");
    let back = sut::d_zlib::decompress(&packed, data.len()).unwrap();
    assert_eq!(back, data);
}

#[test]
#[cfg_attr(miri, ignore = "вызывает zlib")]
fn d_zlib_rejects_garbage() {
    // Явно не-zlib-поток: uncompress обязан вернуть ошибку, а не мусор.
    assert!(sut::d_zlib::decompress(b"not a zlib stream at all", 64).is_err());
}
