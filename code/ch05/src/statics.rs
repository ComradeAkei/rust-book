//! Раздел 5.3: `'static` — данные на всё время работы программы и
//! ограничение `T: 'static`.
//!
//! `T: 'static` требует, чтобы в типе не было ссылок короче `'static`.
//! Ссылка на локальную строку не подходит, даже если строка живёт до конца
//! `main`:
//!
//! ```compile_fail,E0597
//! use ch05::statics::keep;
//!
//! let local = String::from("uart1");
//! let r = keep(&local);
//! println!("{r}");
//! ```
//!
//! Продвигается в статическую память только константное выражение.
//! Массив со значением из аргумента остаётся временным на стеке:
//!
//! ```compile_fail,E0515
//! pub fn gains(n: u16) -> &'static [u16; 2] {
//!     &[n, 2]
//! }
//! ```
//!
//! `Box<dyn Trait>` без указанного времени жизни означает
//! `Box<dyn Trait + 'static>`, и параметр типа без ограничения в него не
//! кладётся:
//!
//! ```compile_fail,E0310
//! use std::fmt::Display;
//!
//! pub fn boxed<T: Display>(t: T) -> Box<dyn Display> {
//!     Box::new(t)
//! }
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::any::Any;
use core::fmt::Display;

// ANCHOR: data
/// Строковый литерал имеет тип `&'static str`: байты лежат в `.rodata`
/// (на микроконтроллере — во flash) и не освобождаются никогда.
pub static BANNER: &str = "станция телеметрии";
pub static LIMITS: [u16; 4] = [100, 200, 400, 800];

/// Таблица, которую функция отдаёт по ссылке. `[1, 2, 4, 8]` — константное
/// выражение, компилятор кладёт его в статическую память, и ссылка на него
/// получает время жизни `'static` (продвижение констант).
pub fn gains() -> &'static [u16; 4] {
    &[1, 2, 4, 8]
}
// ANCHOR_END: data

// ANCHOR: bound
/// Пропускает значение насквозь. Ограничение `T: 'static` говорит о типе:
/// в нём нет ссылок короче `'static`. `String`, `u32` и `Vec<u8>` ему
/// удовлетворяют и уничтожаются когда угодно.
pub fn keep<T: 'static>(t: T) -> T {
    t
}

/// Настройки разных модулей прошивки, каждая своего типа. `dyn Any`
/// требует `'static`: тип с короткой ссылкой внутри нельзя будет потом
/// безопасно достать обратно.
#[derive(Default)]
pub struct Settings {
    items: Vec<Box<dyn Any>>,
}

impl Settings {
    pub fn store<T: 'static>(&mut self, v: T) {
        self.items.push(Box::new(v));
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.items.iter().find_map(|b| b.downcast_ref())
    }
}

/// Второй способ: назвать время жизни объекта. `T: 'a` — в `T` нет ссылок
/// короче `'a`.
pub fn boxed<'a, T: Display + 'a>(t: T) -> Box<dyn Display + 'a> {
    Box::new(t)
}
// ANCHOR_END: bound

// ANCHOR: leak
/// Имя станции читается из конфигурации один раз при старте и нужно до
/// выключения. `Box::leak` отдаёт `&'static mut str` и никогда не
/// освобождает память.
pub fn leak_name(cfg: &str) -> &'static mut str {
    let name = cfg.lines().find_map(|l| l.strip_prefix("name=")).unwrap_or("безымянная");
    Box::leak(String::from(name).into_boxed_str())
}
// ANCHOR_END: leak

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;
    use alloc::rc::Rc;
    use core::cell::Cell;

    /// Значение `'static`-типа, которое замечает своё уничтожение.
    struct Probe(Rc<Cell<bool>>);

    impl Drop for Probe {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn static_data() {
        assert_eq!(BANNER.len(), 35); // 17 кириллических букв по 2 байта и пробел
        assert_eq!(LIMITS[3], 800);
        // Два вызова возвращают один и тот же адрес: таблица одна на программу.
        assert!(core::ptr::eq(gains(), gains()));
    }

    #[test]
    fn static_bound_is_not_forever() {
        let dropped = Rc::new(Cell::new(false));
        let mut s = Settings::default();
        s.store(Probe(Rc::clone(&dropped)));
        s.store(keep(String::from("uart1")));
        s.store(9600u32);
        assert_eq!(s.get::<String>().map(String::as_str), Some("uart1"));
        assert_eq!(s.get::<u32>(), Some(&9600));
        assert!(s.get::<u16>().is_none());
        assert!(!dropped.get());
        drop(s); // Probe: 'static, но уничтожен вместе с хранилищем
        assert!(dropped.get());
    }

    #[test]
    fn boxed_with_lifetime() {
        let unit = String::from("°C");
        let local = format!("21.5 {unit}");
        let b = boxed(&local); // &String: Display + 'a, не 'static
        assert_eq!(format!("{b}"), "21.5 °C");
    }

    #[test]
    fn leaked_name() {
        let name = leak_name("baud=9600\nname=meteo-3\n");
        name.make_ascii_uppercase(); // &'static mut: менять можно
        assert_eq!(name, "METEO-3");
        let fallback = leak_name("baud=9600\n");
        assert_eq!(fallback, "безымянная");
        // Только в тесте: вернуть память, чтобы Miri не сообщал об утечке.
        // SAFETY: обе ссылки получены из Box::leak(Box<str>) и больше не
        // используются.
        unsafe {
            drop(Box::from_raw(name as *mut str));
            drop(Box::from_raw(fallback as *mut str));
        }
    }
}
