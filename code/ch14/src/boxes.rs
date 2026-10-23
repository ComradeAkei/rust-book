//! Раздел 14.1: `Box` — владеющий указатель на значение в куче.
//!
//! Рекурсивный тип, который без указателя не имеет размера; размеры
//! `Box` от тонкого до толстого; выход значения из `Box` разыменованием;
//! `leak` для данных на всё время работы; `into_raw` и `from_raw` для
//! передачи владения через границу, которая знает только адреса; большой
//! буфер сразу в куче.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec;

// ANCHOR: expr
/// Условие тревоги из конфигурации станции — дерево выражений над
/// показаниями каналов. Узел, который содержит узлы того же типа, может
/// хранить их только за указателем: иначе размер типа бесконечен.
#[derive(Debug)]
pub enum Expr {
    /// Показание канала с этим номером.
    Channel(u8),
    Const(i32),
    Add(Box<Expr>, Box<Expr>),
    /// 1, если левое больше правого, иначе 0.
    Gt(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
}

impl Expr {
    pub fn eval(&self, ch: &[i32]) -> i32 {
        match self {
            Expr::Channel(i) => ch[usize::from(*i)],
            Expr::Const(c) => *c,
            Expr::Add(a, b) => a.eval(ch).wrapping_add(b.eval(ch)),
            Expr::Gt(a, b) => i32::from(a.eval(ch) > b.eval(ch)),
            Expr::And(a, b) => i32::from(a.eval(ch) != 0 && b.eval(ch) != 0),
        }
    }
}
// ANCHOR_END: expr

/// Без `Box` у перечисления бесконечный размер:
///
/// ```compile_fail,E0072
/// enum Expr {
///     Const(i32),
///     Add(Expr, Expr),
/// }
/// ```
// ANCHOR: alarm
/// «Температура (канал 0) выше 80 и давление (канал 1) с поправкой
/// выше 1000».
pub fn alarm() -> Expr {
    use Expr::*;
    let b = Box::new;
    And(
        b(Gt(b(Channel(0)), b(Const(80)))),
        b(Gt(b(Add(b(Channel(1)), b(Const(-12)))), b(Const(1000)))),
    )
}
// ANCHOR_END: alarm

// ANCHOR: unbox
/// `Box` — единственный умный указатель, из которого значение можно
/// вынуть разыменованием: `*b` перемещает строку, а память самого `Box`
/// освобождается.
#[allow(clippy::boxed_local)] // Box в параметре — предмет примера
pub fn unbox(b: Box<String>) -> String {
    *b
}
// ANCHOR_END: unbox

/// С `Rc` так нельзя: значением владеют все клоны сразу.
///
/// ```compile_fail,E0507
/// use std::rc::Rc;
/// fn unrc(r: Rc<String>) -> String {
///     *r
/// }
/// ```
pub struct Config {
    pub name: String,
    pub period_ms: u32,
}

// ANCHOR: leak
/// Конфигурация, которую прочли при старте и которая нужна до конца
/// работы: `leak` превращает `Box` в ссылку с любым временем жизни, а
/// память не освобождается никогда.
pub fn leak_config(name: &str, period_ms: u32) -> &'static mut Config {
    Box::leak(Box::new(Config { name: name.into(), period_ms }))
}
// ANCHOR_END: leak

// ANCHOR: raw
/// Передача владения через границу, которая знает только адреса: в C, в
/// контекст прерывания, в дескриптор DMA. `into_raw` отдаёт указатель и
/// забывает о значении, деструктор не вызывается.
pub fn to_raw(c: Box<Config>) -> *mut Config {
    Box::into_raw(c)
}

/// Возвращает владение, и значение снова уничтожится, когда `Box` умрёт.
///
/// # Safety
///
/// `p` получен из [`to_raw`] и ещё не возвращён: второй `from_raw` с тем
/// же адресом — двойное освобождение.
pub unsafe fn from_raw(p: *mut Config) -> Box<Config> {
    // SAFETY: по контракту функции.
    unsafe { Box::from_raw(p) }
}
// ANCHOR_END: raw

// ANCHOR: big
/// 16 МиБ — вдвое больше стека главного потока в Linux.
pub const BIG: usize = 16 << 20;

/// Буфер сразу в куче: `new_zeroed` просит у распределителя обнулённую
/// память (для больших блоков — `calloc`, то есть свежие страницы `mmap`).
pub fn big_zeroed() -> Box<[u8; BIG]> {
    // SAFETY: нулевые байты — допустимое значение массива `u8`.
    unsafe { Box::new_zeroed().assume_init() }
}

/// То же через `vec!`: длина известна только во время работы.
pub fn big_from_vec(len: usize) -> Box<[u8]> {
    vec![0; len].into_boxed_slice()
}
// ANCHOR_END: big

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;

    #[test]
    fn alarm_fires_only_when_both_hold() {
        let e = alarm();
        assert_eq!(e.eval(&[81, 1013]), 1);
        assert_eq!(e.eval(&[80, 1013]), 0);
        assert_eq!(e.eval(&[95, 1012]), 0);
        assert_eq!(e.eval(&[95, 1500]), 1);
    }

    // Гарантии документации: Box<T> для T: Sized — один указатель, и
    // Option<Box<T>> того же размера (None — нулевой адрес). У среза и
    // трейт-объекта указатель толстый: адрес и длина или таблица методов.
    #[test]
    fn guaranteed_box_sizes() {
        let w = size_of::<usize>();
        assert_eq!(size_of::<Box<u64>>(), w);
        assert_eq!(size_of::<Option<Box<u64>>>(), w);
        assert_eq!(size_of::<Box<[u8]>>(), 2 * w);
        assert_eq!(size_of::<Box<str>>(), 2 * w);
        assert_eq!(size_of::<Box<dyn Fn()>>(), 2 * w);
        assert_eq!(size_of::<Option<Box<dyn Fn()>>>(), 2 * w);
    }

    // Наблюдение rustc 1.97 для x86-64: тег и два указателя.
    #[test]
    #[cfg(target_pointer_width = "64")]
    fn observed_expr_size() {
        assert_eq!(size_of::<Expr>(), 24);
    }

    #[test]
    fn unbox_moves_string_out() {
        let b = Box::new(String::from("adc0"));
        let p = b.as_ptr();
        let s = unbox(b);
        assert_eq!(s, "adc0");
        assert_eq!(s.as_ptr(), p, "строка не копировалась");
    }

    #[test]
    fn leak_and_reclaim() {
        let c = leak_config("adc0", 100);
        c.period_ms = 250;
        assert_eq!((c.name.as_str(), c.period_ms), ("adc0", 250));
        // Ссылка от leak — единственная, и владение можно вернуть: иначе
        // Miri сообщил бы об утечке в конце теста.
        // SAFETY: `c` получена из Box::leak и больше нигде не используется.
        drop(unsafe { Box::from_raw(c) });
    }

    #[test]
    fn raw_round_trip() {
        let p = to_raw(Box::new(Config { name: "adc1".into(), period_ms: 10 }));
        // SAFETY: `p` из to_raw, возвращается один раз.
        let b = unsafe { from_raw(p) };
        assert_eq!(b.period_ms, 10);
    }

    #[test]
    #[cfg_attr(miri, ignore = "под Miri 16 МиБ обнуляются минутами")]
    fn big_buffers_without_stack() {
        assert!(big_zeroed().iter().all(|&b| b == 0));
        assert_eq!(big_from_vec(BIG).len(), BIG);
        let arr: Box<[u8; 4]> = big_from_vec(4).try_into().unwrap();
        assert_eq!(*arr, [0; 4]);
    }
}
