//! Раздел A.7: функции, указатели на функции, обратные вызовы.

// ANCHOR: callback
/// C: `void for_each(const uint32_t *v, size_t n, void (*f)(void *, uint32_t), void *ctx);`
/// Контекст не теряет тип: замыкание само несёт захваченные переменные,
/// а `FnMut` разрешает их менять.
pub fn for_each(data: &[u32], mut f: impl FnMut(u32)) {
    for &x in data {
        f(x);
    }
}
// ANCHOR_END: callback

// ANCHOR: sort
/// `qsort(v, n, sizeof *v, cmp_int)` без компаратора через вычитание:
/// `cmp` возвращает `Ordering` и переполниться не может.
pub fn sort_desc(v: &mut [i32]) {
    v.sort_unstable_by(|a, b| b.cmp(a));
}
// ANCHOR_END: sort

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{size_of, size_of_val};

    #[test]
    fn closure_carries_context() {
        let mut sum = 0u64;
        let mut max = 0;
        for_each(&[3, 9, 4], |x| {
            sum += u64::from(x);
            max = max.max(x);
        });
        assert_eq!((sum, max), (16, 9));
    }

    // ANCHOR: fn_sizes
    #[test]
    fn function_pointers_and_closures() {
        let word = size_of::<usize>();
        assert_eq!(size_of::<fn(u32) -> u32>(), word);
        assert_eq!(size_of::<Option<fn(u32) -> u32>>(), word); // NULL = None
        // Замыкание без захвата — тип нулевого размера, и оно приводится
        // к указателю на функцию.
        let inc = |x: u32| x + 1;
        assert_eq!(size_of_val(&inc), 0);
        let f: fn(u32) -> u32 = inc;
        assert_eq!(f(1), 2);
        // Замыкание с захватом хранит захваченное, как структура.
        let k = 3u32;
        let scale = move |x: u32| x * k;
        assert_eq!(size_of_val(&scale), 4);
        assert_eq!(scale(2), 6);
    }
    // ANCHOR_END: fn_sizes

    #[test]
    fn sort_extremes() {
        let mut v = [i32::MAX, -1, i32::MIN, 0];
        sort_desc(&mut v);
        assert_eq!(v, [i32::MAX, 0, -1, i32::MIN]);
        v.sort_unstable();
        assert_eq!(v, [i32::MIN, -1, 0, i32::MAX]);
    }
}
