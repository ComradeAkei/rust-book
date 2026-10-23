//! Разделы 15.1–15.4 и 15.6: какие блоки просят у распределителя
//! коллекции. Распределитель — `Trace` из библиотеки: он же разобран в
//! разделе 15.6.
//!
//! Гарантии документации: `with_capacity(n)` и `vec![…; n]` просят ровно
//! n элементов, `Vec` сам не сжимается, `HashMap::new` и `Vec<()>` не
//! выделяют память. Остальное — стратегия роста, раскладка таблицы и узлов
//! дерева — наблюдение для rustc 1.97, такие тесты названы `observed_`.

use ch15::alloc_api::{Event, Trace};
use std::collections::{BTreeMap, HashMap, HashSet, LinkedList, VecDeque};
use std::hint::black_box;

#[global_allocator]
static TRACE: Trace = Trace::new();

use Event::*;

/// Обращения к распределителю, которые сделала `f`, вместе с уничтожением
/// её результата.
fn events<R>(f: impl FnOnce() -> R) -> Vec<Event> {
    TRACE.record(|| drop(f())).1.expect("больше 256 обращений")
}

#[test]
fn exact_requests() {
    assert_eq!(
        events(|| black_box(Vec::<u32>::with_capacity(10))),
        [Alloc { size: 40, align: 4 }, Dealloc { size: 40, align: 4 }]
    );
    let (v, e) = TRACE.record(|| black_box(vec![1u8, 2, 3]));
    assert_eq!(e.unwrap(), [Alloc { size: 3, align: 1 }]);
    assert_eq!(v.capacity(), 3);
    let (s, e) = TRACE.record(|| String::from("датчик"));
    assert_eq!(e.unwrap(), [Alloc { size: 12, align: 1 }]);
    assert_eq!(events(|| drop(s)), [Dealloc { size: 12, align: 1 }]);
}

#[test]
fn no_allocation_at_all() {
    assert!(events(HashMap::<u32, u32>::new).is_empty());
    assert!(events(Vec::<u64>::new).is_empty());
    assert!(events(String::new).is_empty());
    assert!(
        events(|| {
            let mut v = Vec::new();
            for _ in 0..1000 {
                v.push(());
            }
            assert_eq!(v.capacity(), usize::MAX);
        })
        .is_empty()
    );
}

#[test]
fn never_shrinks_by_itself() {
    let mut v: Vec<u32> = (0..100).collect();
    assert!(
        events(|| {
            v.clear();
            v.extend(0..100);
            v.truncate(1);
        })
        .is_empty()
    );
    assert_eq!(events(|| v.shrink_to_fit()), [Realloc { size: 400, new_size: 4, align: 4 }]);
}

#[test]
fn observed_growth_doubles() {
    let e = events(|| {
        let mut v = Vec::new();
        for i in 0..17u32 {
            v.push(i);
        }
        v
    });
    assert_eq!(
        e,
        [
            Alloc { size: 16, align: 4 },
            Realloc { size: 16, new_size: 32, align: 4 },
            Realloc { size: 32, new_size: 64, align: 4 },
            Realloc { size: 64, new_size: 128, align: 4 },
            Dealloc { size: 128, align: 4 },
        ]
    );
}

#[test]
fn observed_zeroed() {
    // vec![0; n] просит обнулённый блок: calloc, который на больших
    // размерах берёт свежие страницы у ядра и не пишет нули сам.
    assert_eq!(
        events(|| black_box(vec![0u64; 1000])),
        [Zeroed { size: 8000, align: 8 }, Dealloc { size: 8000, align: 8 }]
    );
    assert_eq!(
        events(|| black_box(vec![1u64; 1000])),
        [Alloc { size: 8000, align: 8 }, Dealloc { size: 8000, align: 8 }]
    );
}

#[test]
fn observed_boxed_slice_shrinks() {
    let e = events(|| {
        let mut v = Vec::<u8>::with_capacity(100);
        v.extend_from_slice(b"adc0");
        v.into_boxed_slice()
    });
    assert_eq!(
        e,
        [
            Alloc { size: 100, align: 1 },
            Realloc { size: 100, new_size: 4, align: 1 },
            Dealloc { size: 4, align: 1 }
        ]
    );
}

// ANCHOR: swiss_sizes
/// Блок таблицы: корзины, выровненные до 16, затем по управляющему байту на
/// корзину и ещё 16 — копия начала для чтения группы с любой позиции.
fn swiss_block(buckets: usize, pair: usize) -> usize {
    (buckets * pair).next_multiple_of(16) + buckets + 16
}
// ANCHOR_END: swiss_sizes

#[test]
// Без SSE2 — под Miri, на Cortex-M, на aarch64 — группа у hashbrown из 8
// байт, и блоки другие: выравнивание 8 и хвост в 8 байт.
#[cfg_attr(miri, ignore = "группа без SSE2")]
fn observed_swiss_table_blocks() {
    let e = events(|| {
        let mut m: HashMap<u64, u64> = HashMap::new();
        for k in 0..15 {
            m.insert(k, k);
        }
        m
    });
    let allocs: Vec<usize> = e
        .iter()
        .filter_map(|e| match e {
            Alloc { size, align: 16 } => Some(*size),
            _ => None,
        })
        .collect();
    assert_eq!(allocs, [84, 152, 288, 560]);
    assert_eq!(allocs, [4, 8, 16, 32].map(|b| swiss_block(b, 16)));
    // Маленьким парам минимум корзин больше: место под выравнивание до 16
    // всё равно было бы потрачено.
    assert_eq!(
        events(|| HashSet::from([1u8])),
        [Alloc { size: 48, align: 16 }, Dealloc { size: 48, align: 16 }]
    );
    assert_eq!(swiss_block(16, 1), 48);
    assert_eq!(events(|| HashMap::from([(1u32, 1u32)]))[0], Alloc { size: 52, align: 16 });
    assert_eq!(swiss_block(4, 8), 52);
}

#[test]
fn observed_swiss_capacity() {
    // Ёмкость — 7/8 корзин, у маленьких таблиц — на одну меньше корзин.
    let caps: Vec<usize> =
        [1, 4, 8, 15, 29, 100].map(|n| HashMap::<u64, u64>::with_capacity(n).capacity()).into();
    assert_eq!(caps, [3, 7, 14, 28, 56, 112]);
}

#[test]
fn observed_btree_nodes() {
    let mut m = BTreeMap::new();
    let e = events(|| {
        for k in 0..12u32 {
            m.insert(k, k);
        }
    });
    // Лист: родитель, индекс в родителе, длина и по 11 ключей и значений.
    // Внутренний узел — тот же лист и 12 указателей на детей.
    assert_eq!(
        e,
        [
            Alloc { size: 104, align: 8 },
            Alloc { size: 104, align: 8 },
            Alloc { size: 200, align: 8 }
        ]
    );
    assert_eq!(8 + 2 + 2 + 11 * 4 + 11 * 4, 100);
    assert_eq!(104 + 12 * 8, 200);
}

#[test]
fn observed_list_and_deque() {
    let mut l = LinkedList::new();
    assert_eq!(events(|| l.push_back(1u32)), [Alloc { size: 24, align: 8 }]);
    let mut d: VecDeque<u32> = VecDeque::with_capacity(4);
    d.extend([1, 2, 3]);
    d.pop_front();
    d.pop_front();
    d.extend([4, 5, 6]); // голова в середине, хвост в начале буфера
    let (a, b) = d.as_slices();
    assert_eq!((a, b), (&[3, 4][..], &[5, 6][..]));
    assert_eq!(events(|| d.push_back(7)), [Realloc { size: 16, new_size: 32, align: 4 }]);
    assert_eq!(d.iter().copied().collect::<Vec<_>>(), [3, 4, 5, 6, 7]);
}
