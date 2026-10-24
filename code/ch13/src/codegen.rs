//! Раздел 13.5: во что компилируются итераторы. Цепочка адаптеров —
//! один цикл; внешняя итерация по `Chain` против внутренней; проверки
//! границ в цикле по индексам и без них; SSE2, AVX2 и DSP-расширение
//! Cortex-M4; `chunks` против `chunks_exact`. Сам конвейер с `filter` и
//! `map` — `ch13_mean_mv` в модуле [`adapters`](crate::adapters).

// ANCHOR: chain
/// Сумма двух буферов циклом `for` по `chain`: каждый шаг спрашивает
/// `Chain::next`, какая половина сейчас.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_chain_for(a: &[u32], b: &[u32]) -> u32 {
    let mut s = 0u32;
    for x in a.iter().chain(b) {
        s = s.wrapping_add(*x);
    }
    s
}

/// Та же сумма через `fold`: `Chain::fold` сворачивает сначала первую
/// половину, потом вторую.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_chain_fold(a: &[u32], b: &[u32]) -> u32 {
    a.iter().chain(b).fold(0u32, |s, x| s.wrapping_add(*x))
}
// ANCHOR_END: chain

// ANCHOR: mix
/// Сведение двух каналов с насыщением, по индексам: `a[i]` и `b[i]`
/// проверяются, потому что длины срезов могут не совпасть с `out`.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_mix_index(out: &mut [i16], a: &[i16], b: &[i16]) {
    for i in 0..out.len() {
        out[i] = a[i].saturating_add(b[i]);
    }
}

/// То же с `assert!` перед циклом: после него компилятор знает, что все
/// индексы цикла в пределах срезов.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_mix_assert(out: &mut [i16], a: &[i16], b: &[i16]) {
    let n = out.len();
    assert!(a.len() >= n && b.len() >= n);
    for i in 0..n {
        out[i] = a[i].saturating_add(b[i]);
    }
}

#[inline(always)]
fn mix_body(out: &mut [i16], a: &[i16], b: &[i16]) {
    for (o, (x, y)) in out.iter_mut().zip(a.iter().zip(b)) {
        *o = x.saturating_add(*y);
    }
}

/// То же через `zip`: длина обхода — наименьшая из трёх, индексов нет.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_mix_zip(out: &mut [i16], a: &[i16], b: &[i16]) {
    mix_body(out, a, b);
}
// ANCHOR_END: mix

// ANCHOR: avx2
/// Тот же цикл, собранный с AVX2: регистры `ymm` вдвое шире `xmm`.
/// Объявить такую функцию безопасно, а вызвать вне кода с тем же
/// `target_feature` можно только в `unsafe`: обещать, что процессор умеет
/// AVX2, должен вызывающий.
///
/// # Safety
///
/// Вызывать только на процессоре с AVX2: иначе первая же инструкция с
/// `ymm` — исключение «недопустимая инструкция».
#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
#[inline(never)]
#[target_feature(enable = "avx2")]
pub fn ch13_mix_avx2(out: &mut [i16], a: &[i16], b: &[i16]) {
    mix_body(out, a, b);
}

/// Выбор версии во время работы: проверка CPUID кэшируется, и после первого
/// вызова это одно чтение статической переменной.
#[cfg(all(feature = "std", target_arch = "x86_64"))]
pub fn mix(out: &mut [i16], a: &[i16], b: &[i16]) {
    if std::is_x86_feature_detected!("avx2") {
        // SAFETY: процессор поддерживает AVX2, это только что проверено.
        unsafe { ch13_mix_avx2(out, a, b) }
    } else {
        ch13_mix_zip(out, a, b)
    }
}
// ANCHOR_END: avx2

/// На других целях и без `std` выбора нет.
#[cfg(not(all(feature = "std", target_arch = "x86_64")))]
pub fn mix(out: &mut [i16], a: &[i16], b: &[i16]) {
    ch13_mix_zip(out, a, b)
}

// ANCHOR: dot
/// Отвод КИХ-фильтра: скалярное произведение отсчётов и коэффициентов
/// Q15 с накоплением в `i32`.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_dot_i16(x: &[i16], h: &[i16]) -> i32 {
    x.iter().zip(h).map(|(&a, &b)| i32::from(a) * i32::from(b)).sum()
}
// ANCHOR_END: dot

// ANCHOR: peaks
/// Пиковое значение каждого блока из 16 отсчётов. Последний кусок у
/// `chunks` может быть короче, и длина внутреннего цикла неизвестна.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_peaks_chunks(x: &[i16], out: &mut [i16]) {
    for (o, c) in out.iter_mut().zip(x.chunks(16)) {
        *o = c.iter().copied().max().unwrap_or(i16::MIN);
    }
}

/// То же через `chunks_exact`: каждый кусок ровно из 16 отсчётов, и
/// внутренний цикл исчезает.
#[unsafe(no_mangle)]
#[inline(never)]
pub fn ch13_peaks_exact(x: &[i16], out: &mut [i16]) {
    for (o, c) in out.iter_mut().zip(x.chunks_exact(16)) {
        *o = c.iter().copied().max().unwrap_or(i16::MIN);
    }
}
// ANCHOR_END: peaks

#[cfg(test)]
mod tests {
    use super::*;

    fn samples(n: usize, seed: u32) -> Vec<i16> {
        let mut s = seed;
        (0..n)
            .map(|_| {
                s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (s >> 16) as i16
            })
            .collect()
    }

    #[test]
    fn chain_sums_agree() {
        for (n, m) in [(0, 0), (1, 0), (0, 3), (7, 9), (100, 37)] {
            let a: Vec<u32> = (0..n).map(|i: u32| i.wrapping_mul(2_654_435_761)).collect();
            let b: Vec<u32> = (0..m).map(|i| !i).collect();
            let want = a.iter().chain(&b).fold(0u32, |s, x| s.wrapping_add(*x));
            assert_eq!(ch13_chain_for(&a, &b), want);
            assert_eq!(ch13_chain_fold(&a, &b), want);
        }
    }

    #[test]
    fn mixes_agree() {
        let n = if cfg!(miri) { 37 } else { 1000 };
        let a = samples(n, 1);
        let b = samples(n, 2);
        let mut want = vec![0; n];
        for i in 0..n {
            want[i] = (i32::from(a[i]) + i32::from(b[i])).clamp(-32768, 32767) as i16;
        }
        for f in [ch13_mix_index, ch13_mix_assert, ch13_mix_zip, mix] {
            let mut out = vec![0; n];
            f(&mut out, &a, &b);
            assert_eq!(out, want);
        }
    }

    #[test]
    #[should_panic(expected = "index out of bounds: the len is 2 but the index is 2")]
    fn mix_index_panics_on_short_input() {
        let mut out = [0; 3];
        ch13_mix_index(&mut out, &[1, 2, 3], &[1, 2]);
    }

    #[test]
    #[should_panic(expected = "assertion failed: a.len() >= n && b.len() >= n")]
    fn mix_assert_panics_before_loop() {
        let mut out = [0; 3];
        ch13_mix_assert(&mut out, &[1, 2, 3], &[1, 2]);
    }

    #[test]
    fn mix_zip_truncates() {
        let mut out = [9; 3];
        ch13_mix_zip(&mut out, &[1, 2, 3], &[1, 2]);
        assert_eq!(out, [2, 4, 9]);
    }

    #[test]
    fn mix_index_writes_before_panic() {
        let mut out = [9; 3];
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ch13_mix_index(&mut out, &[1, 2, 3], &[1, 2])
        }));
        assert!(r.is_err());
        assert_eq!(out, [2, 4, 9]);
    }

    #[test]
    fn dot() {
        // 12-битные отсчёты: 203 произведения по модулю меньше 2^22 каждое,
        // и сумма помещается в i32.
        let x: Vec<i16> = samples(203, 3).iter().map(|v| v >> 4).collect();
        let h: Vec<i16> = samples(203, 4).iter().map(|v| v >> 4).collect();
        let want: i64 = x.iter().zip(&h).map(|(&a, &b)| i64::from(a) * i64::from(b)).sum();
        assert_ne!(want, 0);
        assert_eq!(i64::from(ch13_dot_i16(&x, &h)), want);
        assert_eq!(ch13_dot_i16(&x[..3], &h), ch13_dot_i16(&x[..3], &h[..3]));
    }

    #[test]
    fn peaks_agree() {
        for n in [0, 1, 15, 16, 17, 64, 100] {
            let x = samples(n, n as u32);
            let mut a = vec![0; 8];
            let mut b = vec![0; 8];
            ch13_peaks_chunks(&x, &mut a);
            ch13_peaks_exact(&x, &mut b);
            let full = n / 16;
            assert_eq!(a[..full], b[..full], "n = {n}");
            for (i, c) in x.chunks(16).enumerate().take(8) {
                assert_eq!(a[i], *c.iter().max().unwrap());
            }
            // Неполный хвост chunks_exact пропускает.
            assert!(b[full..].iter().all(|&v| v == 0));
        }
    }
}
