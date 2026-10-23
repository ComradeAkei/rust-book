//! Разделы 14.1, 14.3, 14.7 и 14.8: сколько раз и какими блоками примеры
//! обращаются к распределителю. Распределитель-счётчик записывает размер и
//! выравнивание каждого `alloc` и `dealloc` в этом потоке.
//!
//! Число обращений и `Layout` значений `Box` и `Cow` — следствие их
//! определения. Размер блока `Rc` — наблюдение: `RcInner` закрыт, и тесты
//! с ним названы `observed_`.

use ch14::{cow, rc, tricks};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::RefCell;
use std::rc::{Rc, Weak};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Alloc(usize, usize),
    Dealloc(usize, usize),
}

thread_local! {
    static LOG: RefCell<Option<Vec<Op>>> = const { RefCell::new(None) };
}

struct Recording;

fn record(op: Op) {
    // try_with: распределитель зовут и при уничтожении thread_local.
    let _ = LOG.try_with(|log| {
        // Запись в журнал сама выделяет память: на это время журнал вынут.
        if let Ok(mut slot) = log.try_borrow_mut()
            && let Some(mut v) = slot.take()
        {
            v.push(op);
            *slot = Some(v);
        }
    });
}

// SAFETY: все вызовы передаются системному распределителю как есть.
unsafe impl GlobalAlloc for Recording {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(Op::Alloc(layout.size(), layout.align()));
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        record(Op::Dealloc(layout.size(), layout.align()));
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static RECORDING: Recording = Recording;

/// Обращения к распределителю, которые сделала `f`, вместе с уничтожением
/// её результата.
fn ops<T>(f: impl FnOnce() -> T) -> Vec<Op> {
    LOG.with(|l| *l.borrow_mut() = Some(Vec::with_capacity(64)));
    drop(f());
    LOG.with(|l| l.borrow_mut().take().unwrap())
}

use Op::{Alloc, Dealloc};

const W: usize = size_of::<usize>();

#[test]
fn box_allocates_exactly_the_value() {
    assert_eq!(ops(|| Box::new(())), [], "у типа нулевого размера блока нет");
    assert_eq!(ops(|| Box::new(5u32)), [Alloc(4, 4), Dealloc(4, 4)]);
    assert_eq!(ops(|| Box::new([0u64; 3])), [Alloc(24, 8), Dealloc(24, 8)]);
    // Box<dyn Trait> выделяет только значение: таблица методов статическая.
    let f = || Box::new(7u16) as Box<dyn std::fmt::Debug>;
    assert_eq!(ops(f), [Alloc(2, 2), Dealloc(2, 2)]);
}

#[test]
fn observed_rc_block() {
    assert_eq!(ops(|| Rc::new(5u32)), [Alloc(2 * W + 8, W), Dealloc(2 * W + 8, W)]);
    assert_eq!(ops(|| Rc::new(())), [Alloc(2 * W, W), Dealloc(2 * W, W)]);
    assert_eq!(ops(|| Rc::<str>::from("adc0")), [Alloc(2 * W + 8, W), Dealloc(2 * W + 8, W)]);
    assert_eq!(ops(Weak::<u32>::new), [], "пустой Weak не выделяет память");
    let clone_downgrade = || {
        let a = Rc::new(1u64);
        let w = Rc::downgrade(&a);
        (Rc::clone(&a), w)
    };
    assert_eq!(ops(clone_downgrade), [Alloc(3 * W, W), Dealloc(3 * W, W)]);
}

#[test]
fn observed_rc_slice_vs_rc_vec() {
    let samples = || vec![1i16; 10];
    let v = ops(|| tricks::shared_vec(samples()));
    assert_eq!(
        v,
        [Alloc(20, 2), Alloc(2 * W + 3 * W, W), Dealloc(20, 2), Dealloc(5 * W, W)],
        "два блока: буфер вектора и RcInner с вектором"
    );
    let s = ops(|| tricks::shared_slice(samples()));
    assert_eq!(
        s,
        [Alloc(20, 2), Alloc(2 * W + 24, W), Dealloc(20, 2), Dealloc(2 * W + 24, W)],
        "один блок: отсчёты скопированы за счётчики, буфер вектора освобождён сразу"
    );
}

#[test]
fn observed_make_mut() {
    let unique = || {
        let mut c = Rc::new(rc::Config { name: String::new(), period_ms: 1 });
        rc::retune(&mut c, 2);
        c
    };
    assert_eq!(ops(unique).iter().filter(|o| matches!(o, Alloc(..))).count(), 1);
    let shared = || {
        let mut c = Rc::new(rc::Config { name: String::new(), period_ms: 1 });
        let keep = Rc::clone(&c);
        rc::retune(&mut c, 2);
        (c, keep)
    };
    assert_eq!(ops(shared).iter().filter(|o| matches!(o, Alloc(..))).count(), 2);
}

#[test]
fn cow_allocates_only_when_owned() {
    assert_eq!(ops(|| cow::csv_field("adc0")), []);
    assert_eq!(ops(|| cow::lowercase("adc0")), []);
    assert_eq!(ops(|| cow::unslip(&[1, 2, 3])), []);
    let owned = ops(|| cow::csv_field("a,b"));
    assert_eq!(owned, [Alloc(5, 1), Dealloc(5, 1)], "длина + 2 кавычки");
    assert_eq!(ops(|| cow::lowercase("ADC0")), [Alloc(4, 1), Dealloc(4, 1)]);
}
