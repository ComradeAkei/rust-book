//! Раздел 10.6: встраивание через границу обобщения.

// ANCHOR: apply
/// Применить `f` к каждому отсчёту. `F` — тип самой функции: у каждого
/// замыкания и у каждой функции он свой, поэтому копия `apply` для
/// `calibrate` знает, что вызывает, и встраивает тело.
pub fn apply<F: Fn(u32) -> u32>(v: &mut [u32], f: F) {
    for x in v {
        *x = f(*x);
    }
}

/// То же через указатель на функцию, как компаратор у `qsort`. Функция
/// одна на все `f`, и что по указателю, она не знает.
/// `#[inline(never)]` изображает библиотеку, собранную отдельно, как
/// `qsort` в libc.
#[inline(never)]
pub fn apply_ptr(v: &mut [u32], f: fn(u32) -> u32) {
    for x in v {
        *x = f(*x);
    }
}

/// Калибровка отсчёта: усиление 3, смещение 1.
pub fn calibrate(x: u32) -> u32 {
    x.wrapping_mul(3).wrapping_add(1)
}

#[unsafe(no_mangle)]
pub fn ch10_cal_generic(v: &mut [u32]) {
    apply(v, calibrate);
}

#[unsafe(no_mangle)]
pub fn ch10_cal_ptr(v: &mut [u32]) {
    apply_ptr(v, calibrate);
}
// ANCHOR_END: apply

// ANCHOR: closure-types
/// Тип функции `calibrate` — особый тип нулевого размера, а не указатель.
/// Указателем он становится, только если его привести к `fn(u32) -> u32`.
pub fn sizes() -> [usize; 3] {
    let item = calibrate;
    let ptr: fn(u32) -> u32 = calibrate;
    let k = 3;
    let closure = move |x: u32| x * k;
    [size_of_val(&item), size_of_val(&ptr), size_of_val(&closure)]
}
// ANCHOR_END: closure-types

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_result() {
        let mut a = [0u32, 1, 2, u32::MAX];
        let mut b = a;
        ch10_cal_generic(&mut a);
        ch10_cal_ptr(&mut b);
        assert_eq!(a, [1, 4, 7, u32::MAX.wrapping_mul(3).wrapping_add(1)]);
        assert_eq!(a, b);
        let mut c = [5u32; 3];
        apply(&mut c, |x| x / 5);
        assert_eq!(c, [1; 3]);
    }

    #[test]
    fn function_item_is_zero_sized() {
        assert_eq!(sizes(), [0, size_of::<usize>(), 4]);
    }
}
