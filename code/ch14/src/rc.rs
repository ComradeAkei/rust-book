//! Раздел 14.3: `Rc` и `Weak`.
//!
//! Конфигурация из `kref.c`, общая для журнала и отправителя; модель
//! раскладки `Rc` в памяти; шина подписок на слабых ссылках; объект,
//! который подписывает сам себя через `new_cyclic`; цикл сильных ссылок
//! и его разрыв; копирование при записи через `make_mut`.

use alloc::rc::{Rc, Weak};
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};

// ANCHOR: config
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub name: String,
    pub period_ms: u32,
}

/// Журнал держит свою ссылку на конфигурацию, пока открыт.
pub struct Logger {
    cfg: Rc<Config>,
}

impl Logger {
    /// `writable` — удалось ли открыть файл журнала.
    pub fn open(cfg: &Rc<Config>, writable: bool) -> Result<Logger, &'static str> {
        let cfg = Rc::clone(cfg);
        if !writable {
            return Err("журнал не открыт"); // клон уничтожен здесь
        }
        Ok(Logger { cfg })
    }

    pub fn period(&self) -> u32 {
        self.cfg.period_ms
    }
}

/// Отправитель забирает ссылку вызывающего: это видно из типа параметра.
pub struct Sender {
    cfg: Rc<Config>,
}

impl Sender {
    pub fn start(cfg: Rc<Config>) -> Sender {
        Sender { cfg }
    }

    pub fn period(&self) -> u32 {
        self.cfg.period_ms
    }
}
// ANCHOR_END: config

/// Отданную ссылку нельзя отдать ещё раз — сценарий `double` из `kref.c`:
///
/// ```compile_fail,E0382
/// use ch14::rc::{Config, Sender};
/// use std::rc::Rc;
/// let cfg = Rc::new(Config { name: "adc0".into(), period_ms: 100 });
/// let sender = Sender::start(cfg);
/// drop(sender);
/// drop(cfg);
/// ```
///
/// И `Rc` нельзя отправить в другой поток — сценарий `race`:
///
/// ```compile_fail,E0277
/// use ch14::rc::Config;
/// use std::rc::Rc;
/// let cfg = Rc::new(Config { name: "adc0".into(), period_ms: 100 });
/// let mine = Rc::clone(&cfg);
/// std::thread::spawn(move || mine.period_ms);
/// ```
pub fn kref_in_rust() -> (usize, u32, u32) {
    // ANCHOR: kref
    let cfg = Rc::new(Config { name: "adc0".into(), period_ms: 100 });
    assert!(Logger::open(&cfg, false).is_err());
    let after_error = Rc::strong_count(&cfg); // 1: клон журнала уже уничтожен
    let logger = Logger::open(&cfg, true).unwrap();
    let sender = Sender::start(cfg); // `cfg` перемещён, ссылок по-прежнему две
    // ANCHOR_END: kref
    (after_error, logger.period(), sender.period())
}

// ANCHOR: model
/// Модель раскладки: так `Rc` хранит значение (`alloc/src/rc.rs`, тип
/// `RcInner`). Счётчики и значение лежат в одном блоке, и `Rc` — указатель
/// на его начало.
#[repr(C)]
pub struct RcInner<T: ?Sized> {
    pub strong: Cell<usize>,
    /// На единицу больше числа `Weak`: все сильные ссылки вместе держат
    /// одну слабую, чтобы блок не освободили, пока жив хоть один `Rc`.
    pub weak: Cell<usize>,
    pub value: T,
}
// ANCHOR_END: model

// ANCHOR: bus
pub trait Listener {
    fn on_period(&self, ms: u32);
}

/// Шина изменений конфигурации. Подписка не продлевает жизнь подписчика:
/// умерший подписчик пропускается и вычищается при рассылке.
#[derive(Default)]
pub struct Bus {
    subs: Vec<Weak<dyn Listener>>,
}

impl Bus {
    pub fn subscribe(&mut self, l: Weak<dyn Listener>) {
        self.subs.push(l);
    }

    /// Рассылает новый период и возвращает число живых подписчиков.
    pub fn publish(&mut self, ms: u32) -> usize {
        self.subs.retain(|w| match w.upgrade() {
            Some(l) => {
                l.on_period(ms);
                true
            }
            None => false,
        });
        self.subs.len()
    }
}
// ANCHOR_END: bus

// ANCHOR: cyclic
/// Канал, который сам подписывается на шину: ему нужна слабая ссылка на
/// себя, а она есть только у `Rc`, которого ещё нет. `new_cyclic` даёт
/// её до того, как значение построено.
pub struct Channel {
    pub period_ms: Cell<u32>,
    me: Weak<Channel>,
}

impl Channel {
    pub fn new(period_ms: u32) -> Rc<Channel> {
        Rc::new_cyclic(|me| Channel { period_ms: Cell::new(period_ms), me: me.clone() })
    }

    pub fn attach(&self, bus: &mut Bus) {
        bus.subscribe(self.me.clone());
    }
}

impl Listener for Channel {
    fn on_period(&self, ms: u32) {
        self.period_ms.set(ms);
    }
}
// ANCHOR_END: cyclic

// ANCHOR: cycle
/// Узел, который ссылается на соседа сильной ссылкой.
pub struct Node {
    pub peer: RefCell<Option<Rc<Node>>>,
    drops: Rc<Cell<u32>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        self.drops.set(self.drops.get() + 1);
    }
}

/// Два узла ссылаются друг на друга. Возвращает слабую ссылку на первый,
/// чтобы было видно, что он пережил функцию.
pub fn make_cycle(drops: &Rc<Cell<u32>>) -> Weak<Node> {
    let node =
        |drops: &Rc<Cell<u32>>| Rc::new(Node { peer: RefCell::new(None), drops: Rc::clone(drops) });
    let a = node(drops);
    let b = node(drops);
    *a.peer.borrow_mut() = Some(Rc::clone(&b));
    *b.peer.borrow_mut() = Some(Rc::clone(&a));
    Rc::downgrade(&a)
} // a и b уничтожены, у каждого узла осталось по одной сильной ссылке
// ANCHOR_END: cycle

// ANCHOR: retune
/// Правка конфигурации: пока ссылка одна, `make_mut` меняет значение на
/// месте, а если конфигурацию держит кто-то ещё, сначала копирует её, и
/// держатели старой копии изменения не видят.
pub fn retune(cfg: &mut Rc<Config>, period_ms: u32) {
    Rc::make_mut(cfg).period_ms = period_ms;
}
// ANCHOR_END: retune

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::{align_of, size_of};

    #[test]
    fn kref_without_leaks_and_double_puts() {
        assert_eq!(kref_in_rust(), (1, 100, 100));
    }

    #[test]
    fn counts_follow_clones_and_weaks() {
        let a = Rc::new(5u32);
        let b = Rc::clone(&a);
        let w = Rc::downgrade(&a);
        assert_eq!((Rc::strong_count(&a), Rc::weak_count(&a)), (2, 1));
        drop(a);
        assert_eq!(w.upgrade().as_deref(), Some(&5));
        drop(b);
        assert!(w.upgrade().is_none());
        // Значения нет — и счётчики у Weak нулевые, хотя блок ещё занят.
        assert_eq!((w.strong_count(), w.weak_count()), (0, 0));
    }

    #[test]
    fn model_matches_rc() {
        // Rc::as_ptr указывает на значение, а значение в модели лежит после
        // двух счётчиков. Это наблюдение: поля RcInner закрыты.
        let rc = Rc::new(7u32);
        let model = size_of::<RcInner<u32>>();
        assert_eq!(model, 3 * size_of::<usize>());
        assert_eq!(align_of::<RcInner<u32>>(), align_of::<usize>());
        let inner = (Rc::as_ptr(&rc) as *const u8).wrapping_sub(2 * size_of::<usize>());
        // SAFETY: наблюдение за rustc 1.97: блок начинается двумя счётчиками.
        let strong = unsafe { *(inner as *const usize) };
        assert_eq!(strong, 1);
    }

    struct Probe(Cell<u32>);

    impl Listener for Probe {
        fn on_period(&self, ms: u32) {
            self.0.set(ms);
        }
    }

    #[test]
    fn bus_skips_dead_listeners() {
        let mut bus = Bus::default();
        let a: Rc<Probe> = Rc::new(Probe(Cell::new(0)));
        let b: Rc<Probe> = Rc::new(Probe(Cell::new(0)));
        bus.subscribe(Rc::downgrade(&a) as Weak<dyn Listener>);
        bus.subscribe(Rc::downgrade(&b) as Weak<dyn Listener>);
        assert_eq!(bus.publish(200), 2);
        drop(b);
        assert_eq!(bus.publish(300), 1);
        assert_eq!(a.0.get(), 300);
        assert_eq!(Rc::strong_count(&a), 1, "подписка не держит подписчика");
    }

    #[test]
    fn channel_subscribes_itself() {
        let mut bus = Bus::default();
        let ch = Channel::new(100);
        ch.attach(&mut bus);
        assert_eq!(bus.publish(500), 1);
        assert_eq!(ch.period_ms.get(), 500);
        assert_eq!((Rc::strong_count(&ch), Rc::weak_count(&ch)), (1, 2));
        drop(ch);
        assert_eq!(bus.publish(600), 0);
    }

    #[test]
    fn cycle_leaks_until_broken() {
        let drops = Rc::new(Cell::new(0));
        let a = make_cycle(&drops);
        assert_eq!(drops.get(), 0, "узлы пережили функцию");
        let a = a.upgrade().expect("первый узел жив");
        assert_eq!(Rc::strong_count(&a), 2); // своя ссылка и ссылка соседа
        // Разрываем цикл, иначе узлы не освободятся никогда — и Miri
        // сообщит об утечке.
        let b = a.peer.borrow_mut().take().unwrap();
        drop(b.peer.borrow_mut().take());
        drop((a, b));
        assert_eq!(drops.get(), 2);
    }

    #[test]
    fn make_mut_copies_only_when_shared() {
        let mut cfg = Rc::new(Config { name: "adc0".into(), period_ms: 100 });
        let before = Rc::as_ptr(&cfg);
        retune(&mut cfg, 200);
        assert_eq!(Rc::as_ptr(&cfg), before, "единственная ссылка — на месте");
        let snapshot = Rc::clone(&cfg);
        retune(&mut cfg, 300);
        assert_ne!(Rc::as_ptr(&cfg), before, "общую — скопировал");
        assert_eq!((snapshot.period_ms, cfg.period_ms), (200, 300));
        assert_eq!(Rc::strong_count(&snapshot), 1);
    }

    #[test]
    fn unwrap_variants() {
        let a = Rc::new(String::from("adc0"));
        let b = Rc::clone(&a);
        let a = Rc::try_unwrap(a).unwrap_err(); // ссылок две — вернуть Rc
        drop(b);
        assert_eq!(Rc::try_unwrap(a).unwrap(), "adc0");
        let c = Rc::new(String::from("adc1"));
        let d = Rc::clone(&c);
        assert_eq!(Rc::unwrap_or_clone(c), "adc1"); // клонирует строку
        assert_eq!(Rc::into_inner(d).as_deref(), Some("adc1"));
    }
}
