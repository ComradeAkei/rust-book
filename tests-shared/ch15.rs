// Общие тесты главы 15 для задач A и C: подключаются через include! в
// крейты заготовок и решений, после строки `use <крейт> as sut;`. Задача B
// ставит свой распределитель и живёт в отдельном файле ch15_count.rs.
//
// Задача A сверяет предсказание с настоящим `Vec`: его ёмкостью и
// обращениями к распределителю, которые считает распределитель этого
// файла. Задача C сверяет два дерева одной формулы, проверяет разбор,
// выравнивание и рост кусков `Bump`; под Miri — ещё и отсутствие
// неопределённого поведения и утечек.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    /// Вызовы alloc, realloc и dealloc в этом потоке.
    static CALLS: Cell<[usize; 3]> = const { Cell::new([0; 3]) };
}

fn bump_call(i: usize) {
    let _ = CALLS.try_with(|c| {
        let mut v = c.get();
        v[i] += 1;
        c.set(v);
    });
}

struct Calls;

// SAFETY: все вызовы передаются системному распределителю как есть.
unsafe impl GlobalAlloc for Calls {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        bump_call(0);
        // SAFETY: контракт alloc тот же, что у вызывающего.
        unsafe { System.alloc(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        bump_call(1);
        // SAFETY: контракт realloc тот же, что у вызывающего.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        bump_call(2);
        // SAFETY: ptr выделен System с тем же layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static CALLS_ALLOC: Calls = Calls;

mod task_a {
    use super::CALLS;
    use super::sut::a_predict::{Op, Prediction, predict};
    use std::iter::repeat_n;

    /// Те же операции над настоящим `Vec`, и что он при этом сделал.
    fn real<T: Copy>(x: T, ops: &[Op]) -> Prediction {
        let mut v: Vec<T> = Vec::new();
        let before = CALLS.with(|c| c.get());
        for op in ops {
            match *op {
                Op::Push(n) => {
                    for _ in 0..n {
                        v.push(x);
                    }
                }
                // repeat_n знает свою длину, как срез; filter — нет.
                Op::ExtendExact(n) => v.extend(repeat_n(x, n)),
                Op::ExtendUnknown(n) => v.extend(repeat_n(x, n).filter(|_| true)),
                Op::Reserve(n) => v.reserve(n),
                Op::ReserveExact(n) => v.reserve_exact(n),
                Op::ShrinkToFit => v.shrink_to_fit(),
                Op::ShrinkTo(n) => v.shrink_to(n),
                Op::Truncate(n) => v.truncate(n),
                Op::Clear => v.clear(),
            }
        }
        let after = CALLS.with(|c| c.get());
        Prediction {
            len: v.len(),
            cap: v.capacity(),
            allocs: after[0] - before[0],
            reallocs: after[1] - before[1],
            deallocs: after[2] - before[2],
        }
    }

    fn check(ops: &[Op]) {
        assert_eq!(predict(1, ops), real(0u8, ops), "u8: {ops:?}");
        assert_eq!(predict(4, ops), real(0u32, ops), "u32: {ops:?}");
        assert_eq!(predict(2000, ops), real([0u8; 2000], ops), "[u8; 2000]: {ops:?}");
        assert_eq!(predict(0, ops), real((), ops), "(): {ops:?}");
    }

    #[test]
    fn task_a_examples_from_text() {
        use Op::*;
        assert_eq!(predict(4, &[Push(9)]), Prediction { len: 9, cap: 16, allocs: 1, reallocs: 2, deallocs: 0 });
        assert_eq!(predict(1, &[Push(9)]).cap, 16);
        assert_eq!(predict(2000, &[Push(3)]), Prediction { len: 3, cap: 4, allocs: 1, reallocs: 2, deallocs: 0 });
        assert_eq!(predict(4, &[Reserve(1)]).cap, 4);
        assert_eq!(predict(4, &[ReserveExact(1)]).cap, 1);
        // Как vec![1, 2, 3]: ёмкость ровно 3.
        assert_eq!(predict(4, &[ReserveExact(3), ExtendExact(3), Reserve(1)]).cap, 6);
        assert_eq!(predict(4, &[ReserveExact(3), ExtendExact(3), Reserve(10)]).cap, 13);
        assert_eq!(predict(4, &[ExtendExact(3), Reserve(1)]).cap, 4);
        assert_eq!(predict(4, &[ExtendExact(4), ExtendExact(5)]).cap, 9);
        assert_eq!(predict(4, &[ExtendUnknown(9)]).cap, 16);
        assert_eq!(predict(0, &[Push(1000)]).cap, usize::MAX);
        for ops in [
            &[Push(9)][..],
            &[ReserveExact(3), ExtendExact(3), Reserve(1), Reserve(10), ReserveExact(10), ShrinkToFit, Clear],
            &[Push(100), Clear, Push(100)],
            &[Push(5), ShrinkToFit, Truncate(0), ShrinkToFit, Push(1)],
        ] {
            check(ops);
        }
    }

    #[test]
    fn task_a_shrink_to_same_size() {
        use Op::*;
        // len == cap, но ёмкость больше min: realloc с прежним размером.
        let ops = [ReserveExact(4), Push(4), ShrinkTo(2)];
        assert_eq!(predict(4, &ops), Prediction { len: 4, cap: 4, allocs: 1, reallocs: 1, deallocs: 0 });
        check(&ops);
        check(&[ReserveExact(8), ShrinkTo(3), ShrinkToFit, ShrinkTo(0)]);
    }

    #[test]
    fn task_a_random_sequences() {
        let mut s = 0x2545_F491_4F6C_DD1Du64;
        let mut next = |m: u64| {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s % m) as usize
        };
        let (runs, len) = if cfg!(miri) { (5, 8) } else { (400, 30) };
        for _ in 0..runs {
            let ops: Vec<Op> = (0..len)
                .map(|_| match next(9) {
                    0 => Op::Push(next(20)),
                    1 => Op::ExtendExact(next(40)),
                    2 => Op::ExtendUnknown(next(20)),
                    3 => Op::Reserve(next(50)),
                    4 => Op::ReserveExact(next(50)),
                    5 => Op::ShrinkToFit,
                    6 => Op::ShrinkTo(next(30)),
                    7 => Op::Truncate(next(30)),
                    _ => Op::Clear,
                })
                .collect();
            check(&ops);
        }
    }
}

mod task_c {
    use super::sut::c_bump::{BinOp, BoxExpr, Boxes, Bump, Expr, InBump, ParseError, gen_source, parse};

    fn vars(name: &str) -> i64 {
        match name {
            "adc0" => 700,
            "adc1" => -3,
            "i2c_t" => 25,
            _ => 1,
        }
    }

    fn both(src: &str) -> (i64, i64) {
        let boxed: Box<BoxExpr> = parse(src, Boxes).unwrap();
        let bump = Bump::new();
        let tree: &Expr = parse(src, InBump(&bump)).unwrap();
        (boxed.eval(&vars), tree.eval(&vars))
    }

    #[test]
    fn task_c_precedence_and_vars() {
        for (src, want) in [
            ("1 + 2 * 3", 7),
            ("(1 + 2) * 3", 9),
            ("-2 * 3", -6),
            ("10 - 4 - 3", 3),
            ("100 / 7 / 2", 7),
            ("10 / 0", 0),
            ("(adc0 - 512) * 3 / 4 + -adc1", 144),
            ("  i2c_t*2  ", 50),
        ] {
            assert_eq!(both(src), (want, want), "{src}");
        }
    }

    #[test]
    fn task_c_errors() {
        for (src, at) in [("1 +", 3), ("(1", 2), ("1 $ 2", 2), ("", 0), ("2 3", 2), ("99999999999999999999", 0)] {
            assert_eq!(parse(src, Boxes).err(), Some(ParseError { at }), "{src:?}");
            let bump = Bump::new();
            assert_eq!(parse(src, InBump(&bump)).err(), Some(ParseError { at }), "{src:?}");
        }
    }

    #[test]
    fn task_c_trees_agree() {
        let n = if cfg!(miri) { 3 } else { 200 };
        for seed in 0..n {
            let src = gen_source(1 + (seed as usize * 7) % 300, seed);
            let (a, b) = both(&src);
            assert_eq!(a, b, "{src}");
        }
    }

    #[test]
    fn task_c_tree_shape() {
        let bump = Bump::new();
        let e = parse("a - -b * 2", InBump(&bump)).unwrap();
        let Expr::Bin(BinOp::Sub, Expr::Var("a"), Expr::Bin(BinOp::Mul, Expr::Neg(Expr::Var("b")), Expr::Num(2))) = e
        else {
            panic!("{e:?}");
        };
    }

    #[test]
    fn task_c_alignment_and_strings() {
        let bump = Bump::new();
        let a = bump.alloc(1u8);
        let b = bump.alloc(2u64);
        let c = bump.alloc(3u16);
        assert_eq!(b as *const u64 as usize % 8, 0);
        assert_eq!(c as *const u16 as usize % 2, 0);
        *a += 1;
        *b += 1;
        assert_eq!((*a, *b, *c), (2, 3, 3));
        let s = bump.alloc_str("датчик");
        assert_eq!(s, "датчик");
        assert_eq!(bump.alloc_str(""), "");
        #[repr(align(64))]
        struct Line([u8; 64]);
        let l = bump.alloc(Line([7; 64]));
        assert_eq!(l as *const Line as usize % 64, 0);
        assert_eq!(l.0[63], 7);
    }

    #[test]
    fn task_c_chunks_grow() {
        let bump = Bump::new();
        assert_eq!(bump.footprint(), (0, 0));
        let n = if cfg!(miri) { 2_000 } else { 100_000 };
        let all: Vec<&mut u64> = (0..n).map(|i| bump.alloc(i)).collect();
        assert!(all.iter().enumerate().all(|(i, x)| **x == i as u64));
        let (chunks, bytes) = bump.footprint();
        // Куски растут вдвое: их число — логарифм, лишнее место — не больше
        // самого большого куска.
        assert!(bytes >= n as usize * 8 && bytes <= 4 * n as usize * 8 + 8192, "{bytes}");
        assert!(chunks <= 2 + (n as usize * 8 / 4096).ilog2() as usize, "{chunks}");
        // Блок больше куска помещается.
        let big = bump.alloc([1u8; 100_000]);
        assert_eq!(big[99_999], 1);
        assert_eq!(*bump.alloc(5u32), 5);
    }
}

mod task_d {
    use super::sut::d_table::{OpenTable, filled, probe_stats};

    #[test]
    fn task_d_basic_ops() {
        let mut t: OpenTable<&str, u32> = OpenTable::with_capacity(8);
        assert!(t.is_empty());
        assert_eq!(t.insert("adc0", 1), None);
        assert_eq!(t.insert("adc0", 2), Some(1)); // обновление
        assert_eq!(t.insert("i2c0", 3), None);
        assert_eq!((t.len(), t.get(&"adc0"), t.get(&"i2c0")), (2, Some(&2), Some(&3)));
        assert_eq!(t.get(&"nope"), None);
        assert_eq!(t.remove(&"adc0"), Some(2));
        assert_eq!((t.len(), t.get(&"adc0")), (1, None));
        // Надгробие не обрывает пробу: сосед по-прежнему находится.
        assert_eq!(t.get(&"i2c0"), Some(&3));
        // Вставка переиспользует надгробие, не увеличивая used сверх меры.
        assert_eq!(t.insert("adc1", 9), None);
        assert_eq!(t.get(&"adc1"), Some(&9));
    }

    #[test]
    fn task_d_full_table_rejects() {
        // Маленькая таблица без роста: заполнить целиком и упереться.
        let mut t: OpenTable<u32, u32> = OpenTable::with_capacity(8);
        let cap = t.capacity() as u32;
        for i in 0..cap {
            assert!(t.insert_within_capacity(i, i).is_ok());
        }
        assert_eq!(t.insert_within_capacity(cap, cap), Err((cap, cap)));
        // Существующий ключ обновляется даже в полной таблице.
        assert_eq!(t.insert_within_capacity(0, 100), Ok(Some(0)));
    }

    #[test]
    fn task_d_insert_grows_and_keeps_keys() {
        let mut t: OpenTable<u32, u32> = OpenTable::with_capacity(8);
        for i in 0..1000 {
            t.insert(i, i * 2);
        }
        assert_eq!(t.len(), 1000);
        assert!(t.capacity() >= 1024);
        assert!((0..1000).all(|i| t.get(&i) == Some(&(i * 2))));
        assert_eq!(t.get(&1000), None);
    }

    #[test]
    fn task_d_probe_len_grows_with_load() {
        // Средняя длина пробы успешного поиска при линейном пробировании
        // растёт с загрузкой (формула Кнута ≈ (1 + 1/(1−α))/2).
        let cap = if cfg!(miri) { 256 } else { 8192 };
        let (half, _) = filled(cap, 0.5, 1);
        let (heavy, _) = filled(cap, 0.9, 1);
        let (lo, _) = probe_stats(&half);
        let (hi, _) = probe_stats(&heavy);
        assert!((1.0..2.5).contains(&lo), "загрузка 0.5: {lo}");
        assert!(hi > lo + 1.0, "0.9 ({hi}) должна быть заметно дороже 0.5 ({lo})");
    }

    #[test]
    fn task_d_tombstones_inflate_then_compaction_helps() {
        let cap = if cfg!(miri) { 256 } else { 4096 };
        let (mut t, _) = filled(cap, 0.7, 7);
        let (before, _) = probe_stats(&t);
        // Удалить половину ключей: на их месте остаются надгробия.
        let keys: Vec<u64> = t.keys().copied().collect();
        for k in keys.iter().step_by(2) {
            t.remove(k);
        }
        let (with_tombs, _) = probe_stats(&t);
        // Проба оставшихся не короче: надгробия всё равно осматриваются.
        assert!(with_tombs >= before - 0.01, "надгробия: {with_tombs} против {before}");
        // Перехеширование (вставка новых до порога роста) выбрасывает
        // надгробия, и проба снова короткая.
        for i in 0..cap as u64 {
            t.insert(1_000_000 + i, i);
        }
        let short = probe_stats(&t).0;
        assert!(short < with_tombs + 3.0);
    }
}
