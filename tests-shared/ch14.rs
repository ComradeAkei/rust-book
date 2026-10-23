// Общие тесты главы 14: подключаются через include! в крейты заготовок и
// решений, после строки `use <крейт> as sut;`.
//
// Задача A сверяет предсказания с тем, что фрагменты делают на деле: ловит
// панику `RefCell` и считает обращения к распределителю. Задача B
// проверяет счётчики, уничтожение значения и освобождение блока своего
// `Rc`; под Miri — ещё и отсутствие неопределённого поведения и утечек.
// Задача C сверяет оба списка с моделью на `VecDeque` на случайных
// последовательностях операций. Задача D проверяет, что замеры работают;
// сами числа и их объяснение — в отчёте.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

thread_local! {
    /// Вызовы alloc, realloc и dealloc в этом потоке.
    static CALLS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

fn bump(i: usize) {
    let _ = CALLS.try_with(|c| {
        let mut v = c.get();
        v[i] += 1;
        c.set(v);
    });
}

struct Counting;

// SAFETY: все вызовы передаются системному распределителю как есть.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump(0);
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        bump(2);
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        bump(1);
        // SAFETY: контракт realloc тот же, что у вызывающего.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static COUNTING: Counting = Counting;

/// Обращения `f` к распределителю в этом потоке: alloc, realloc, dealloc.
fn calls<T>(f: impl FnOnce() -> T) -> ([usize; 3], T) {
    let before = CALLS.with(Cell::get);
    let r = f();
    let after = CALLS.with(Cell::get);
    ([0, 1, 2].map(|i| after[i] - before[i]), r)
}

// ---------------------------------------------------------------- задача A

mod task_a {
    use super::*;
    use std::cell::RefCell;
    use sut::a_predict::{Outcome, RC, REFCELL, predicted_rc, predicted_refcell};

    fn outcome(f: fn(&RefCell<Vec<u8>>)) -> Outcome {
        let c = RefCell::new(vec![1, 2, 3]);
        match catch_unwind(AssertUnwindSafe(|| f(&c))) {
            Ok(()) => Outcome::Ok,
            Err(p) => {
                let msg = p
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| p.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                match msg.as_str() {
                    "RefCell already borrowed" => Outcome::AlreadyBorrowed,
                    "RefCell already mutably borrowed" => Outcome::AlreadyMutablyBorrowed,
                    other => panic!("неожиданная паника: {other}"),
                }
            }
        }
    }

    #[test]
    fn task_a_refcell() {
        let answers = predicted_refcell();
        let wrong: Vec<String> = (0..6)
            .filter_map(|i| {
                let actual = outcome(REFCELL[i]);
                (actual != answers[i]).then(|| {
                    format!("фрагмент {}: ответ {:?}, на деле {actual:?}", i + 1, answers[i])
                })
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    #[test]
    fn task_a_rc() {
        let answers = predicted_rc();
        let wrong: Vec<String> = (0..6)
            .filter_map(|i| {
                let ([alloc, realloc, _], (strong, weak)) = calls(RC[i]);
                let actual = (alloc + realloc, strong, weak);
                (actual != answers[i]).then(|| {
                    format!(
                        "фрагмент {}: ответ (память {}, strong {}, weak {}), на деле {actual:?}",
                        i + 1,
                        answers[i].0,
                        answers[i].1,
                        answers[i].2
                    )
                })
            })
            .collect();
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}

// ---------------------------------------------------------------- задача B

mod task_b {
    use super::*;
    use std::cell::RefCell;
    use sut::b_rc::{MyRc, MyWeak};

    /// Значение, которое считает свои уничтожения.
    struct Noisy(Rc<Cell<u32>>);

    impl Drop for Noisy {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }

    #[test]
    fn task_b_counts() {
        let a = MyRc::new(5u32);
        assert_eq!((MyRc::strong_count(&a), MyRc::weak_count(&a)), (1, 0));
        let b = a.clone();
        let w = MyRc::downgrade(&a);
        let w2 = w.clone();
        assert_eq!((MyRc::strong_count(&b), MyRc::weak_count(&b)), (2, 2));
        assert_eq!((w.strong_count(), w.weak_count()), (2, 2));
        assert!(MyRc::ptr_eq(&a, &b));
        assert_eq!(*a + *b, 10);
        drop((a, w2));
        assert_eq!((MyRc::strong_count(&b), MyRc::weak_count(&b)), (1, 1));
        drop(b);
        assert_eq!((w.strong_count(), w.weak_count()), (0, 0));
        assert!(w.upgrade().is_none());
    }

    #[test]
    fn task_b_value_dropped_once_block_freed_last() {
        let drops = Rc::new(Cell::new(0));
        let (c, (a, w)) = calls(|| {
            let a = MyRc::new(Noisy(Rc::clone(&drops)));
            let w = MyRc::downgrade(&a);
            (a, w)
        });
        assert_eq!(c, [1, 0, 0], "один блок на значение и оба счётчика");
        let (c, ()) = calls(|| drop(a));
        assert_eq!(drops.get(), 1, "значение уничтожено вместе с последним MyRc");
        assert_eq!(c[2], 0, "блок жив: его держит MyWeak");
        assert!(w.upgrade().is_none());
        let (c, ()) = calls(|| drop(w));
        assert_eq!(c[2], 1, "последний MyWeak освободил блок");
        assert_eq!(drops.get(), 1, "и не уничтожил значение второй раз");
    }

    #[test]
    fn task_b_upgrade() {
        let a = MyRc::new(String::from("adc0"));
        let w = MyRc::downgrade(&a);
        let b = w.upgrade().unwrap();
        assert_eq!(MyRc::strong_count(&a), 2);
        assert_eq!(*b, "adc0");
        drop((a, b));
        assert!(w.upgrade().is_none());
    }

    #[test]
    fn task_b_weak_new() {
        let (c, w) = calls(MyWeak::<u64>::new);
        assert_eq!(c, [0, 0, 0], "пустой MyWeak не выделяет память");
        assert!(w.upgrade().is_none());
        assert_eq!((w.strong_count(), w.weak_count()), (0, 0));
        let w2 = w.clone();
        let (c, ()) = calls(|| drop((w, w2)));
        assert_eq!(c, [0, 0, 0]);
        let d: MyWeak<u64> = MyWeak::default();
        assert!(d.upgrade().is_none());
    }

    #[test]
    fn task_b_get_mut() {
        let mut a = MyRc::new(vec![1, 2]);
        MyRc::get_mut(&mut a).unwrap().push(3);
        let b = a.clone();
        assert!(MyRc::get_mut(&mut a).is_none(), "есть второй MyRc");
        drop(b);
        let w = MyRc::downgrade(&a);
        assert!(MyRc::get_mut(&mut a).is_none(), "есть MyWeak");
        drop(w);
        assert_eq!(MyRc::get_mut(&mut a).map(|v| v.len()), Some(3));
    }

    #[test]
    fn task_b_try_unwrap() {
        let drops = Rc::new(Cell::new(0));
        let a = MyRc::new(Noisy(Rc::clone(&drops)));
        let b = a.clone();
        let a = MyRc::try_unwrap(a).err().expect("ссылок две");
        drop(b);
        let w = MyRc::downgrade(&a);
        let (c, value) = calls(|| MyRc::try_unwrap(a).ok().expect("ссылка одна"));
        assert_eq!(c[2], 0, "блок держит MyWeak");
        assert_eq!(drops.get(), 0, "значение отдано, а не уничтожено");
        assert!(w.upgrade().is_none());
        drop(value);
        assert_eq!(drops.get(), 1);
        let (c, ()) = calls(|| drop(w));
        assert_eq!(c[2], 1);
        let lone = MyRc::new(7u8);
        let (c, v) = calls(|| MyRc::try_unwrap(lone).ok());
        assert_eq!((v, c[2]), (Some(7), 1), "без слабых ссылок блок освобождён сразу");
    }

    struct TreeNode {
        parent: RefCell<MyWeak<TreeNode>>,
        children: RefCell<Vec<MyRc<TreeNode>>>,
        _drops: Noisy,
    }

    #[test]
    fn task_b_tree_without_leaks() {
        let drops = Rc::new(Cell::new(0));
        let node = || {
            MyRc::new(TreeNode {
                parent: RefCell::new(MyWeak::new()),
                children: RefCell::new(Vec::new()),
                _drops: Noisy(Rc::clone(&drops)),
            })
        };
        let root = node();
        for _ in 0..3 {
            let child = node();
            *child.parent.borrow_mut() = MyRc::downgrade(&root);
            root.children.borrow_mut().push(child);
        }
        let first = root.children.borrow()[0].clone();
        assert!(MyRc::ptr_eq(&first.parent.borrow().upgrade().unwrap(), &root));
        assert_eq!(MyRc::weak_count(&root), 3);
        drop(root);
        assert_eq!(drops.get(), 3, "корень и два ребёнка; первый жив");
        assert!(first.parent.borrow().upgrade().is_none());
        drop(first);
        assert_eq!(drops.get(), 4);
    }

    /// Значение держит слабую ссылку на свой же блок: при уничтожении
    /// значения умирает последний MyWeak, а блок ещё нужен.
    struct SelfRef {
        me: RefCell<MyWeak<SelfRef>>,
    }

    #[test]
    fn task_b_value_holds_own_weak() {
        let a = MyRc::new(SelfRef { me: RefCell::new(MyWeak::new()) });
        *a.me.borrow_mut() = MyRc::downgrade(&a);
        assert_eq!(MyRc::weak_count(&a), 1);
        let (c, ()) = calls(|| drop(a));
        assert_eq!(c[2], 1, "блок освобождён ровно один раз");
    }

    #[test]
    fn task_b_layout_and_threads() {
        assert_eq!(size_of::<MyRc<u64>>(), size_of::<usize>());
        assert_eq!(size_of::<Option<MyRc<u64>>>(), size_of::<usize>());
        assert_eq!(size_of::<MyWeak<u64>>(), size_of::<usize>());
        // Компилируется, только если MyRc и MyWeak не Send и не Sync:
        // иначе подходят обе реализации, и вызов неоднозначен.
        trait AmbiguousIfImpl<A> {
            fn check() {}
        }
        impl<T: ?Sized> AmbiguousIfImpl<()> for T {}
        struct IsSend;
        struct IsSync;
        impl<T: ?Sized + Send> AmbiguousIfImpl<IsSend> for T {}
        impl<T: ?Sized + Sync> AmbiguousIfImpl<IsSync> for T {}
        <MyRc<u64> as AmbiguousIfImpl<_>>::check();
        <MyWeak<u64> as AmbiguousIfImpl<_>>::check();
    }

    #[test]
    fn task_b_many_clones() {
        let n = if cfg!(miri) { 100 } else { 10_000 };
        let a = MyRc::new(1u8);
        let all: Vec<_> = (0..n)
            .map(|i| if i % 2 == 0 { Ok(a.clone()) } else { Err(MyRc::downgrade(&a)) })
            .collect();
        assert_eq!(MyRc::strong_count(&a), 1 + n / 2);
        assert_eq!(MyRc::weak_count(&a), n / 2);
        drop(all);
        assert_eq!((MyRc::strong_count(&a), MyRc::weak_count(&a)), (1, 0));
    }
}

// ---------------------------------------------------------------- задача C

mod task_c {
    use super::*;
    use sut::c_list::{ArenaList, Handle, RcHandle, RcList};

    /// Общий интерфейс двух списков для одного теста с моделью.
    trait List {
        type H: Clone;
        fn new() -> Self;
        fn len(&self) -> usize;
        fn push_front(&mut self, v: u32) -> Self::H;
        fn push_back(&mut self, v: u32) -> Self::H;
        fn pop_front(&mut self) -> Option<u32>;
        fn pop_back(&mut self) -> Option<u32>;
        fn remove(&mut self, h: &Self::H) -> Option<u32>;
        fn move_to_front(&mut self, h: &Self::H) -> bool;
        fn to_vec(&self) -> Vec<u32>;
    }

    impl List for RcList<u32> {
        type H = RcHandle<u32>;
        fn new() -> Self {
            RcList::new()
        }
        fn len(&self) -> usize {
            RcList::len(self)
        }
        fn push_front(&mut self, v: u32) -> Self::H {
            RcList::push_front(self, v)
        }
        fn push_back(&mut self, v: u32) -> Self::H {
            RcList::push_back(self, v)
        }
        fn pop_front(&mut self) -> Option<u32> {
            RcList::pop_front(self)
        }
        fn pop_back(&mut self) -> Option<u32> {
            RcList::pop_back(self)
        }
        fn remove(&mut self, h: &Self::H) -> Option<u32> {
            RcList::remove(self, h)
        }
        fn move_to_front(&mut self, h: &Self::H) -> bool {
            RcList::move_to_front(self, h)
        }
        fn to_vec(&self) -> Vec<u32> {
            RcList::to_vec(self)
        }
    }

    impl List for ArenaList<u32> {
        type H = Handle;
        fn new() -> Self {
            ArenaList::new()
        }
        fn len(&self) -> usize {
            ArenaList::len(self)
        }
        fn push_front(&mut self, v: u32) -> Self::H {
            ArenaList::push_front(self, v)
        }
        fn push_back(&mut self, v: u32) -> Self::H {
            ArenaList::push_back(self, v)
        }
        fn pop_front(&mut self) -> Option<u32> {
            ArenaList::pop_front(self)
        }
        fn pop_back(&mut self) -> Option<u32> {
            ArenaList::pop_back(self)
        }
        fn remove(&mut self, h: &Self::H) -> Option<u32> {
            ArenaList::remove(self, *h)
        }
        fn move_to_front(&mut self, h: &Self::H) -> bool {
            ArenaList::move_to_front(self, *h)
        }
        fn to_vec(&self) -> Vec<u32> {
            ArenaList::to_vec(self)
        }
    }

    /// Случайные операции над списком и моделью. Значения уникальны, и
    /// ручка в модели — это значение, на которое она указывает.
    fn against_model<L: List>(seed: u32, steps: usize) {
        let mut s = seed;
        let mut rnd = |n: u32| {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            s % n
        };
        let mut list = L::new();
        let mut model: VecDeque<u32> = VecDeque::new();
        let mut handles: Vec<(u32, L::H)> = Vec::new();
        for next in 0..steps as u32 {
            match rnd(6) {
                0 => {
                    handles.push((next, list.push_front(next)));
                    model.push_front(next);
                }
                1 => {
                    handles.push((next, list.push_back(next)));
                    model.push_back(next);
                }
                2 => assert_eq!(list.pop_front(), model.pop_front()),
                3 => assert_eq!(list.pop_back(), model.pop_back()),
                4 if !handles.is_empty() => {
                    let (v, h) = handles[rnd(handles.len() as u32) as usize].clone();
                    let expect =
                        model.iter().position(|&x| x == v).map(|i| model.remove(i).unwrap());
                    assert_eq!(list.remove(&h), expect, "remove({v})");
                }
                5 if !handles.is_empty() => {
                    let (v, h) = handles[rnd(handles.len() as u32) as usize].clone();
                    let expect = match model.iter().position(|&x| x == v) {
                        Some(i) => {
                            model.remove(i);
                            model.push_front(v);
                            true
                        }
                        None => false,
                    };
                    assert_eq!(list.move_to_front(&h), expect, "move_to_front({v})");
                }
                _ => {}
            }
            assert_eq!(list.len(), model.len());
            if steps < 500 || next % 64 == 0 {
                assert_eq!(list.to_vec(), Vec::from(model.clone()));
            }
        }
        assert_eq!(list.to_vec(), Vec::from(model));
    }

    fn steps() -> usize {
        if cfg!(miri) { 200 } else { 20_000 }
    }

    #[test]
    fn task_c_rc_list_model() {
        for seed in 1..=3 {
            against_model::<RcList<u32>>(seed, steps());
        }
    }

    #[test]
    fn task_c_arena_list_model() {
        for seed in 1..=3 {
            against_model::<ArenaList<u32>>(seed, steps());
        }
    }

    #[test]
    fn task_c_stale_handles() {
        let mut a = ArenaList::new();
        let h = a.push_back(1u32);
        assert_eq!(a.remove(h), Some(1));
        let h2 = a.push_back(2);
        assert_eq!(a.remove(h), None, "старая ручка на переиспользованный слот");
        assert!(!a.move_to_front(h));
        assert_eq!(a.to_vec(), [2]);
        assert_eq!(a.remove(h2), Some(2));
        let mut r = RcList::new();
        let h = r.push_front(1u32);
        assert_eq!(r.pop_back(), Some(1));
        assert_eq!(r.remove(&h), None);
        assert!(!r.move_to_front(&h));
        assert!(r.is_empty() && a.is_empty());
    }

    #[test]
    fn task_c_allocations_and_no_leaks() {
        let n = if cfg!(miri) { 100 } else { 1000 };
        let ([alloc, realloc, dealloc], ()) = calls(|| {
            let mut l = RcList::new();
            for i in 0..n {
                l.push_back(i);
            }
        });
        assert_eq!((alloc, realloc), (n as usize, 0), "RcList: блок на узел");
        assert_eq!(dealloc, alloc, "все узлы освобождены");
        let ([alloc, realloc, dealloc], ()) = calls(|| {
            let mut l = ArenaList::new();
            for i in 0..n {
                l.push_back(i);
            }
        });
        assert!(alloc + realloc <= 12, "ArenaList: удвоение ёмкости, {alloc} + {realloc}");
        assert_eq!(dealloc, alloc);
    }

    #[test]
    fn task_c_long_rc_list_drops_without_recursion() {
        let n = if cfg!(miri) { 1_000 } else { 1_000_000 };
        let mut l = RcList::new();
        for i in 0..n {
            l.push_back(i);
        }
        drop(l);
    }
}

// ---------------------------------------------------------------- задача D

#[cfg(feature = "std")]
mod task_d {
    use super::sut::d_arc::{borrowed, lines, private, private_padded, shared};

    #[test]
    fn task_d_measurements_run() {
        let iters = if cfg!(miri) { 20 } else { 100_000 };
        for threads in [1, 2, 4] {
            for f in [shared, private, private_padded, borrowed] {
                let ns = f(threads, iters);
                assert!(ns.is_finite() && ns > 0.0, "{threads} потоков: {ns}");
            }
        }
    }

    #[test]
    fn task_d_padded_lines_differ() {
        let l = lines(8, true);
        let mut u = l.clone();
        u.sort_unstable();
        u.dedup();
        assert_eq!(u.len(), 8, "у каждого блока своя линия: {l:?}");
        assert_eq!(lines(8, false).len(), 8);
    }
}
