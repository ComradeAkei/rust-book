//! Раздел 4.1: ссылки `&T` и `&mut T`, правило «много читателей или один
//! писатель».
//!
//! Две изменяемые ссылки на одно значение, которыми пользуются вперемешку:
//!
//! ```compile_fail,E0499
//! let mut v = vec![1u16, 2, 3];
//! let a = &mut v;
//! let b = &mut v;
//! a.push(4);
//! b.push(5);
//! ```
//!
//! Читатель и писатель одновременно — пример из введения к главе:
//!
//! ```compile_fail,E0502
//! let mut temp: Vec<u16> = (1000..1008).collect();
//! let peak = temp.iter().max().unwrap(); // ссылка на максимум
//! temp.push(1500); // девятый отсчёт: буфер растёт
//! println!("максимум {peak}");
//! ```
//!
//! Присваивание значению, на которое ещё смотрит ссылка:
//!
//! ```compile_fail,E0506
//! let mut limit = 100;
//! let r = &limit;
//! limit = 200;
//! println!("{r}");
//! ```

// ANCHOR: rules
/// Калибровка канала АЦП: смещение нуля и масштаб.
pub struct Calib {
    pub offset: i32,
    pub scale: i32,
}

/// Чтение через разделяемую ссылку: таких ссылок может быть сколько угодно.
pub fn apply(c: &Calib, raw: i32) -> i32 {
    (raw - c.offset) * c.scale
}

/// Изменение через уникальную ссылку: пока она жива, других путей к
/// значению нет.
pub fn trim(c: &mut Calib, delta: i32) {
    c.offset += delta;
}

pub fn readers_then_writer() -> (i32, i32) {
    let mut c = Calib { offset: 10, scale: 2 };
    let r1 = &c;
    let r2 = &c; // второй читатель
    let before = apply(r1, 15) + apply(r2, 15);
    let w = &mut c; // писатель: r1 и r2 дальше не используются
    trim(w, 5);
    (before, apply(&c, 15))
}
// ANCHOR_END: rules

// ANCHOR: machine
/// Ссылка в машинном коде — адрес. Разыменование — загрузка.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_apply(c: &Calib, raw: i32) -> i32 {
    apply(c, raw)
}

/// `Option<&T>` — тот же адрес, `None` — ноль (раздел 6.4).
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_apply_opt(c: Option<&Calib>, raw: i32) -> i32 {
    match c {
        Some(c) => apply(c, raw),
        None => raw,
    }
}
// ANCHOR_END: machine

// ANCHOR: late_null_check
#[inline(never)]
fn log_byte(r: &u8) {
    core::hint::black_box(*r);
}

/// Сначала ссылка, потом проверка указателя.
///
/// # Safety
///
/// `p` указывает на байт, доступный для чтения.
#[inline(never)]
#[unsafe(no_mangle)]
pub unsafe fn ch04_late_null_check(p: *const u8) -> bool {
    // SAFETY: условие функции. Проверять его после `&*p` поздно.
    log_byte(unsafe { &*p });
    p.is_null()
}
// ANCHOR_END: late_null_check

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules() {
        assert_eq!(readers_then_writer(), (20, 0));
    }

    // ANCHOR: sizes
    #[test]
    fn reference_sizes() {
        // Ссылка на значение с известным размером — один адрес.
        assert_eq!(size_of::<&Calib>(), size_of::<usize>());
        assert_eq!(size_of::<&mut Calib>(), size_of::<usize>());
        // Ссылка не бывает нулевой, и `None` занимает этот ноль.
        assert_eq!(size_of::<Option<&Calib>>(), size_of::<usize>());
        // Ссылка на срез — адрес и длина (раздел 1.6).
        assert_eq!(size_of::<&[u8]>(), 2 * size_of::<usize>());
    }
    // ANCHOR_END: sizes

    #[test]
    fn option_slice_ref() {
        // Ниша — нулевой адрес; длина в ней не участвует.
        assert_eq!(size_of::<Option<&[u8]>>(), size_of::<&[u8]>());
        let b = 7u8;
        // SAFETY: `&b` указывает на читаемый байт.
        assert!(!unsafe { ch04_late_null_check(&b) });
    }

    #[test]
    fn machine() {
        let c = Calib { offset: 1, scale: 3 };
        assert_eq!(ch04_apply(&c, 5), 12);
        assert_eq!(ch04_apply_opt(Some(&c), 5), 12);
        assert_eq!(ch04_apply_opt(None, 5), 5);
    }
}
