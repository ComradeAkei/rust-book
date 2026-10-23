//! Раздел 12.3: функции как значения. Тип функции и указатель на функцию,
//! приведение замыкания без захвата, таблица обработчиков во flash и
//! трамплин, через который замыкание уходит в C.

// ANCHOR: items
pub fn double(x: u8) -> u8 {
    x.wrapping_mul(2)
}

pub fn invert(x: u8) -> u8 {
    !x
}

/// Имя функции — значение типа нулевого размера, указатель — слово, а
/// `None` у `Option<fn>` занимает нулевой адрес, которого у функции нет.
pub fn item_sizes() -> [usize; 3] {
    let item = double; //               fn(u8) -> u8 {double}
    let ptr: fn(u8) -> u8 = double; //  fn(u8) -> u8
    [size_of_val(&item), size_of_val(&ptr), size_of::<Option<fn(u8) -> u8>>()]
}
// ANCHOR_END: items

/// У каждой функции свой тип, и переменной типа `double` другую функцию
/// не присвоить.
///
/// ```compile_fail,E0308
/// use ch12::fnptr::{double, invert};
/// let mut f = double;
/// f = invert;
/// ```
///
/// Замыкание с захватом указателем на функцию не станет: состояние
/// некуда деть.
///
/// ```compile_fail,E0308
/// let k = 3u8;
/// let f: fn(u8) -> u8 = |x| x * k;
/// ```
pub const fn item_types() {}

// ANCHOR: coerce
/// Замыкание без захвата приводится к указателю, и в таблицу встают
/// вперемешку функции и замыкания. Таблица — константа и лежит во flash.
pub static HANDLERS: [fn(u8) -> u8; 4] = [double, invert, |x| x >> 1, |x| x.rotate_left(4)];

#[unsafe(no_mangle)]
pub fn ch12_dispatch(op: usize, x: u8) -> u8 {
    match HANDLERS.get(op) {
        Some(f) => f(x),
        None => x,
    }
}
// ANCHOR_END: coerce

// ANCHOR: ctors
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reading {
    Raw(u16),
    Missing,
}

/// Конструктор варианта с полями — функция: `Reading::Raw` имеет тип
/// `fn(u16) -> Reading {Reading::Raw}`, `Some` — тоже. Их можно передать
/// туда, где ждут замыкание.
pub fn ctors(raw: [u16; 2]) -> [Option<Reading>; 2] {
    raw.map(Reading::Raw).map(Some)
}

/// Так же ведут себя методы: `u8::count_ones` — функция `fn(u8) -> u32`.
pub fn ones(bytes: [u8; 3]) -> [u32; 3] {
    bytes.map(u8::count_ones)
}
// ANCHOR_END: ctors

// ANCHOR: compare
/// Сравнивать указатели на функции через `==` можно, но rustc
/// предупреждает: две одинаковые функции компоновщик может слить в одну,
/// а одну функцию — продублировать в разных единицах трансляции.
/// `fn_addr_eq` говорит то же самое явно.
pub fn same(a: fn(u8) -> u8, b: fn(u8) -> u8) -> bool {
    core::ptr::fn_addr_eq(a, b)
}
// ANCHOR_END: compare

#[cfg(all(feature = "std", target_os = "linux", target_env = "gnu"))]
pub use c_sort::{sort_desc, sort_with};

#[cfg(all(feature = "std", target_os = "linux", target_env = "gnu"))]
mod c_sort {
    // ANCHOR: tramp
    use core::cmp::Ordering;
    use core::ffi::{c_int, c_void};

    type Compar = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void) -> c_int;

    unsafe extern "C" {
        /// glibc: `void qsort_r(void *base, size_t nmemb, size_t size,
        /// int (*compar)(const void *, const void *, void *), void *arg)`.
        fn qsort_r(base: *mut c_void, nmemb: usize, size: usize, compar: Compar, arg: *mut c_void);
    }

    /// Своя копия трамплина для каждого `F`. C зовёт её по указателю, а она
    /// знает, что за `ctx` лежит именно `F`, и вызывает его напрямую.
    unsafe extern "C" fn tramp<T, F: FnMut(&T, &T) -> Ordering>(
        a: *const c_void,
        b: *const c_void,
        ctx: *mut c_void,
    ) -> c_int {
        // SAFETY: ctx — адрес `f` из `sort_with`, живой до её возврата;
        // a и b — адреса двух элементов типа T, которые сравнивает qsort_r.
        let (f, a, b) = unsafe { (&mut *ctx.cast::<F>(), &*a.cast::<T>(), &*b.cast::<T>()) };
        f(a, b) as c_int
    }

    pub fn sort_with<T, F: FnMut(&T, &T) -> Ordering>(v: &mut [T], mut f: F) {
        let ctx = (&raw mut f).cast::<c_void>();
        // SAFETY: qsort_r переставляет элементы копированием байтов, а
        // перемещение в Rust и есть копирование байтов; трамплин получает
        // ровно тот F, для которого инстанцирован.
        unsafe { qsort_r(v.as_mut_ptr().cast(), v.len(), size_of::<T>(), tramp::<T, F>, ctx) }
    }
    // ANCHOR_END: tramp

    // ANCHOR: sort-desc
    /// По убыванию, а равные — по порядку старших бит: замыкание считает
    /// сравнения в своём состоянии.
    pub fn sort_desc(v: &mut [u32]) -> usize {
        let mut calls = 0;
        sort_with(v, |a, b| {
            calls += 1;
            b.cmp(a)
        });
        calls
    }
    // ANCHOR_END: sort-desc
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = size_of::<usize>();

    #[test]
    fn functions_as_values() {
        assert_eq!(item_sizes(), [0, W, W]);
        assert_eq!(
            [0, 1, 2, 3, 4].map(|op| ch12_dispatch(op, 0x96)),
            [0x2c, 0x69, 0x4b, 0x69, 0x96]
        );
        assert_eq!(ctors([7, 9]), [Some(Reading::Raw(7)), Some(Reading::Raw(9))]);
        assert_ne!(Reading::Missing, Reading::Raw(0));
        assert_eq!(ones([0, 0xff, 0x81]), [0, 8, 2]);
        // Язык не обещает, что два приведения одной функции дадут один
        // адрес, и Miri нарочно даёт разные: там сравнение ложно.
        if !cfg!(miri) {
            assert!(same(double, double));
        }
        assert!(!same(double, invert));
    }

    #[test]
    #[cfg(all(feature = "std", target_os = "linux", target_env = "gnu"))]
    #[cfg_attr(miri, ignore = "qsort_r — функция glibc, Miri её не исполняет")]
    fn qsort_r_with_closure() {
        let mut v = [5u32, 1, 4, 1, 5, 9, 2, 6];
        let calls = sort_desc(&mut v);
        assert_eq!(v, [9, 6, 5, 5, 4, 2, 1, 1]);
        assert!(calls >= v.len() - 1, "{calls}");
        let mut words = ["кадр", "ack", "измерение"];
        sort_with(&mut words, |a, b| a.len().cmp(&b.len()));
        assert_eq!(words, ["ack", "кадр", "измерение"]);
    }

    /// Паника не может пройти через кадр функции `extern "C"`: процесс
    /// завершается. Тест запускает сам себя с переменной окружения и
    /// смотрит на код выхода и сообщение.
    #[test]
    #[cfg(all(feature = "std", target_os = "linux", target_env = "gnu"))]
    #[cfg_attr(miri, ignore = "qsort_r — функция glibc, Miri её не исполняет")]
    fn panic_in_callback_aborts() {
        if std::env::var_os("CH12_PANIC_IN_QSORT").is_some() {
            let mut v = [3u8, 1, 2];
            sort_with(&mut v, |_, _| panic!("сравнение сломалось"));
            return;
        }
        let out = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "fnptr::tests::panic_in_callback_aborts", "--nocapture"])
            .env("CH12_PANIC_IN_QSORT", "1")
            .output()
            .unwrap();
        let err = String::from_utf8_lossy(&out.stderr);
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(out.status.signal(), Some(6), "SIGABRT: {err}");
        assert!(err.contains("сравнение сломалось"), "{err}");
        assert!(err.contains("panic in a function that cannot unwind"), "{err}");
    }
}
