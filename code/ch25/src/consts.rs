//! Раздел 25.1: `const` и `static`.
//!
//! `const` — значение, у которого нет своего места в памяти. Его можно
//! поставить в образец `match`, а `static` — нельзя: имя в образце, которое
//! совпадает со `static`, rustc отвергает, а не связывает заново.
//!
//! ```compile_fail,E0530
//! pub static OFFSET: u16 = 5;
//!
//! pub fn is_offset(x: u16) -> bool {
//!     match x {
//!         OFFSET => true, // match bindings cannot shadow statics
//!         _ => false,
//!     }
//! }
//! ```
//!
//! `static` — один объект на всю программу, и обращаться к нему могут
//! все потоки сразу. Поэтому тип `static` обязан быть `Sync`:
//!
//! ```compile_fail,E0277
//! use core::cell::Cell;
//!
//! pub static LAST: Cell<u32> = Cell::new(0); // `Cell<u32>` cannot be shared between threads safely
//! ```
//!
//! Ссылку на временное значение вызова `const fn` вне контекста констант
//! компилятор не продвигает в `'static`: функция может быть вызвана и при
//! выполнении, и `&square(4)` — ссылка на временную переменную функции.
//!
//! ```compile_fail,E0515
//! const fn square(x: u32) -> u32 {
//!     x * x
//! }
//!
//! pub fn sixteen() -> &'static u32 {
//!     &square(4) // cannot return reference to temporary value
//! }
//! ```
//!
//! Блок `const { … }` делает то же выражение константой, и ссылка на неё
//! живёт всю программу — как ссылка на литерал `&(2 + 2)`:
//!
//! ```
//! const fn square(x: u32) -> u32 {
//!     x * x
//! }
//!
//! pub fn sixteen() -> &'static u32 {
//!     &const { square(4) }
//! }
//!
//! pub fn four() -> &'static u32 {
//!     &(2 + 2)
//! }
//!
//! assert_eq!(*sixteen() + *four(), 20);
//! ```
//!
//! Повторить в массиве можно только `Copy`-значение — или константу: тогда
//! каждый элемент получает своё вычисление константы.
//!
//! ```compile_fail,E0277
//! pub struct Slot {
//!     pub used: bool,
//! }
//!
//! pub fn slots() -> [Slot; 4] {
//!     [Slot { used: false }; 4] // the trait bound `Slot: Copy` is not satisfied
//! }
//! ```

// ANCHOR: items
/// Длина заголовка кадра: значение, а не объект. Подставляется в каждое
/// место использования — в длину массива, в образец, в выражение.
pub const HEADER_LEN: usize = 8;

/// Коэффициенты усиления: один объект с адресом на всё время работы.
pub static GAINS: [u16; 4] = [100, 200, 400, 800];

/// Константа в типе: длина массива — константное выражение.
pub type Header = [u8; HEADER_LEN];

/// Константа в образце: `HEADER_LEN` здесь — значение 8, а не новое имя.
pub fn describe(len: usize) -> &'static str {
    match len {
        0 => "пусто",
        HEADER_LEN => "только заголовок",
        _ => "с нагрузкой",
    }
}

/// `static` в выражении: чтение из объекта по его адресу.
pub fn gain(range: usize) -> u16 {
    GAINS[range % GAINS.len()]
}
// ANCHOR_END: items

// ANCHOR: static-in-const
/// С Rust 1.83 константа может читать неизменяемый `static` без
/// внутренней изменяемости: значение `static` известно при компиляции.
pub static RING_LEN: usize = 16;

/// Длина массива из `static`: тоже константное выражение.
pub type Ring = [u8; RING_LEN];
// ANCHOR_END: static-in-const

// ANCHOR: fresh
/// Ячейка драйвера. Не `Copy`: в ней буфер, который нельзя случайно
/// размножить.
#[derive(Debug, PartialEq)]
pub struct Slot {
    pub used: bool,
    pub data: [u8; 8],
}

impl Slot {
    /// Свободная ячейка — ассоциированная константа.
    pub const FREE: Slot = Slot { used: false, data: [0; 8] };
}

/// `[Slot::FREE; 4]` компилируется, хотя `Slot` не `Copy`: повторяется не
/// значение, а константа, и каждый из четырёх элементов — её новое
/// вычисление. То же даёт `[const { Slot { … } }; 4]`.
pub fn slots() -> [Slot; 4] {
    [Slot::FREE; 4]
}
// ANCHOR_END: fresh

// ANCHOR: trap
pub mod trap {
    use core::sync::atomic::AtomicU32;
    #[cfg(target_has_atomic = "32")]
    use core::sync::atomic::Ordering;

    /// Ошибка: `const` с внутренней изменяемостью. Каждое упоминание
    /// `COUNTER` — новый временный атомик, и `fetch_add` меняет временный
    /// объект, который тут же исчезает. rustc предупреждает об этом
    /// (`const_item_interior_mutations`), clippy тоже; здесь предупреждения
    /// подавлены, чтобы показать поведение.
    #[allow(clippy::declare_interior_mutable_const)]
    pub const COUNTER: AtomicU32 = AtomicU32::new(0);

    #[cfg(target_has_atomic = "32")]
    #[allow(const_item_interior_mutations, clippy::borrow_interior_mutable_const)]
    pub fn bump_const() -> u32 {
        COUNTER.fetch_add(1, Ordering::Relaxed);
        COUNTER.load(Ordering::Relaxed)
    }

    /// Правильно: один объект в ОЗУ, общий для всех обращений.
    pub static SHARED: AtomicU32 = AtomicU32::new(0);

    #[cfg(target_has_atomic = "32")]
    pub fn bump_static() -> u32 {
        SHARED.fetch_add(1, Ordering::Relaxed) + 1
    }
}
// ANCHOR_END: trap

// ANCHOR: address
/// Пороги срабатывания: константа и `static` с тем же значением.
pub const LIMITS: [u16; 4] = [10, 20, 40, 80];
pub static LIMITS_STATIC: [u16; 4] = LIMITS;

/// Ссылка на константу — ссылка на безымянную копию значения, которую
/// компилятор создаёт сам (продвижение в `'static`).
pub fn limits_a() -> &'static [u16; 4] {
    &LIMITS
}

pub fn limits_b() -> &'static [u16; 4] {
    &LIMITS
}

/// Ссылка на `static` — всегда на один и тот же объект.
pub fn limits_static() -> &'static [u16; 4] {
    &LIMITS_STATIC
}
// ANCHOR_END: address

#[cfg(test)]
mod tests {
    use super::*;
    use core::ptr;

    #[test]
    fn values_and_objects() {
        assert_eq!(core::mem::size_of::<Header>(), 8);
        assert_eq!(core::mem::size_of::<Ring>(), 16);
        assert_eq!(describe(8), "только заголовок");
        assert_eq!(describe(12), "с нагрузкой");
        assert_eq!(gain(5), 200);
        assert_eq!(slots(), [Slot::FREE, Slot::FREE, Slot::FREE, Slot::FREE]);
    }

    /// Гарантия языка: изменение временного атомика не видно следующему
    /// упоминанию константы, а `static` — один объект.
    #[test]
    fn guaranteed_const_is_a_fresh_value() {
        assert_eq!(trap::bump_const(), 0);
        assert_eq!(trap::bump_const(), 0);
        let before = trap::SHARED.load(core::sync::atomic::Ordering::Relaxed);
        assert_eq!(trap::bump_static(), before + 1);
        assert!(trap::bump_static() > before + 1);
    }

    /// Гарантия языка: ссылки на один `static` равны, а два живых
    /// временных объекта не могут занимать одно место.
    #[test]
    #[allow(clippy::borrow_interior_mutable_const)]
    fn guaranteed_addresses() {
        assert!(ptr::eq(limits_static(), &LIMITS_STATIC));
        assert!(!ptr::eq(&trap::COUNTER, &trap::COUNTER));
    }

    /// Наблюдение, rustc 1.97: две ссылки на `LIMITS` указывают на одну
    /// копию — компилятор хранит значения констант без повторов. Язык
    /// этого не обещает: Reference прямо говорит, что ссылки на одну
    /// константу не обязаны совпадать. Miri, который держится обещаний
    /// языка, выдаёт двум копиям разные адреса, и тест под ним падает.
    #[test]
    #[cfg_attr(miri, ignore = "у Miri копии константы — разные объекты")]
    fn observed_const_references_shared() {
        assert!(ptr::eq(limits_a(), limits_b()));
        assert_eq!(limits_a(), limits_static());
    }
}
