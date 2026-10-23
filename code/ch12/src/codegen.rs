//! Раздел 12.5: замыкание в машинном коде. Где лежит окружение, что
//! получает копия обобщённой функции и как выглядит вызов через
//! `&mut dyn FnMut` и `Box<dyn FnOnce>`.

/// Калибровка отсчёта АЦП, как в C-примере введения: `x · k + b`.
#[derive(Clone, Copy)]
pub struct Affine {
    pub k: i16,
    pub b: i16,
}

// ANCHOR: env
/// Не встраивается, чтобы в ассемблере было видно, что получает копия для
/// каждого замыкания.
#[inline(never)]
pub fn apply_twice<F: Fn(u32) -> u32>(f: F, x: u32) -> u32 {
    f(f(x))
}

/// Без захвата: окружение нулевого размера, передавать нечего.
#[unsafe(no_mangle)]
pub fn ch12_twice_zst(x: u32) -> u32 {
    apply_twice(|v| v.wrapping_mul(3), x)
}

/// Захват по ссылке: окружение — указатель на `k`.
#[unsafe(no_mangle)]
pub fn ch12_twice_ref(x: u32, k: &u32) -> u32 {
    apply_twice(|v| v.wrapping_mul(*k), x)
}

/// Захват по значению: окружение — само число.
#[unsafe(no_mangle)]
pub fn ch12_twice_val(x: u32, k: u32) -> u32 {
    apply_twice(move |v| v.wrapping_mul(k), x)
}
// ANCHOR_END: env

// ANCHOR: map
#[inline]
pub fn map_in_place<F: FnMut(i16) -> i16>(a: &mut [i16], mut f: F) {
    for x in a {
        *x = f(*x);
    }
}

/// Замыкание держит `&Affine`. `&mut [i16]` и `&Affine` по правилам
/// заимствования не пересекаются, LLVM знает это и выносит чтение `k` и
/// `b` из цикла — то, что в C пришлось сделать руками.
#[unsafe(no_mangle)]
pub fn ch12_scale_generic(a: &mut [i16], p: &Affine) {
    map_in_place(a, |x| x.wrapping_mul(p.k).wrapping_add(p.b));
}
// ANCHOR_END: map

// ANCHOR: dyn
/// Одна копия на все замыкания: окружение приходит адресом, тело — из
/// таблицы.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch12_map_dyn(a: &mut [i16], f: &mut dyn FnMut(i16) -> i16) {
    for x in a {
        *x = f(*x);
    }
}

#[unsafe(no_mangle)]
pub fn ch12_scale_dyn(a: &mut [i16], p: &Affine) {
    ch12_map_dyn(a, &mut |x| x.wrapping_mul(p.k).wrapping_add(p.b));
}
// ANCHOR_END: dyn

// ANCHOR: fnptr
/// Указатель на функцию: ни окружения, ни таблицы, только адрес.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch12_map_fn(a: &mut [i16], f: fn(i16) -> i16) {
    for x in a {
        *x = f(*x);
    }
}
// ANCHOR_END: fnptr

#[cfg(feature = "alloc")]
// ANCHOR: once
/// Вызов `Box<dyn FnOnce>`: тело получает окружение по адресу и забирает
/// его, а память освобождает вызывающий.
#[unsafe(no_mangle)]
pub fn ch12_call_boxed(f: alloc::boxed::Box<dyn FnOnce() -> u32>) -> u32 {
    f()
}
// ANCHOR_END: once

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_results() {
        assert_eq!(ch12_twice_zst(5), 45);
        assert_eq!(ch12_twice_ref(5, &2), 20);
        assert_eq!(ch12_twice_val(5, 2), 20);
        let p = Affine { k: 3, b: 1 };
        let mut a: [i16; 100] = core::array::from_fn(|i| i as i16 - 50);
        let mut b = a;
        let mut c = a;
        ch12_scale_generic(&mut a, &p);
        ch12_scale_dyn(&mut b, &p);
        ch12_map_fn(&mut c, |x| x.wrapping_mul(3).wrapping_add(1));
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(a[0], -149);
        #[cfg(feature = "alloc")]
        {
            let s = alloc::string::String::from("кадр");
            assert_eq!(ch12_call_boxed(alloc::boxed::Box::new(move || s.len() as u32)), 8);
            assert_eq!(ch12_call_boxed(alloc::boxed::Box::new(|| 7)), 7);
        }
    }
}
