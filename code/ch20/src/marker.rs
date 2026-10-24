//! Раздел 20.2: `Send` и `Sync`. Это два маркерных трейта без методов,
//! которые компилятор выводит автоматически. `Send` — «значение можно
//! переместить в другой поток»; `Sync` — «на значение можно иметь ссылку
//! `&T` из нескольких потоков» (равносильно `&T: Send`). Тип с ними —
//! потоко-безопасен по построению; без них компилятор не пустит значение
//! через границу потока. Так отсутствие гонок данных становится свойством
//! типа, а не дисциплины.
//!
//! `Rc<T>` не `Send`: его счётчик ссылок меняется без атомарности, и два
//! потока, клонируя один `Rc`, затёрли бы счётчик. Компилятор ловит это на
//! `spawn`, а не в рантайме:
//!
//! ```compile_fail,E0277
//! use std::rc::Rc;
//! use std::thread;
//! let shared = Rc::new(5);
//! let moved = Rc::clone(&shared);
//! thread::spawn(move || {
//!     // error[E0277]: `Rc<i32>` cannot be sent between threads safely
//!     println!("{}", moved);
//! });
//! ```
//!
//! Замена — `Arc<T>` ([20.3](super::shared)): тот же общий указатель, но со
//! счётчиком на атомиках, поэтому `Arc<T>: Send + Sync` (когда `T: Send +
//! Sync`).

use std::thread;

// ANCHOR: bounds
/// Выполнить `f` на фоновом потоке. Границы `Send + 'static` — это и есть
/// требование потокобезопасности: замыкание (вместе со всем, что оно
/// захватило) обязано быть пересылаемым в другой поток и не одалживать
/// ничего, что может исчезнуть. Именно эти границы стоят на `thread::spawn`.
pub fn in_background<F>(f: F)
where
    F: FnOnce() + Send + 'static,
{
    thread::spawn(f).join().unwrap();
}
// ANCHOR_END: bounds

// ANCHOR: unsafe-impl
/// Обёртка над сырым указателем. Сырой указатель НЕ `Send` и НЕ `Sync` —
/// компилятор не знает, что за ним и безопасно ли делить его между потоками.
/// Если мы отвечаем за это сами, маркер можно проставить вручную — `unsafe
/// impl`, потому что гарантию теперь даёт человек, а не вывод компилятора.
pub struct Shared<T> {
    ptr: *const T,
}

// SAFETY: указатель только читается, а делить `&T` между потоками безопасно
// ровно тогда, когда это безопасно для самого T, — отсюда граница `T: Sync`.
unsafe impl<T: Sync> Send for Shared<T> {}
// SAFETY: `&Shared<T>` даёт лишь общий доступ к `*const T` на чтение; это
// безопасно между потоками при `T: Sync`.
unsafe impl<T: Sync> Sync for Shared<T> {}

impl<T> Shared<T> {
    /// # Safety
    ///
    /// `ptr` действителен и переживёт все обращения через `Shared`, а данные
    /// за ним никто не меняет, пока существует `Shared`.
    pub unsafe fn new(ptr: *const T) -> Self {
        Shared { ptr }
    }

    /// Прочитать общие данные.
    ///
    /// # Safety
    ///
    /// Как у [`new`](Self::new): `ptr` действителен и данные за ним неизменны
    /// на всё время существования возвращённой ссылки.
    pub unsafe fn get(&self) -> &T {
        // SAFETY: инвариант конструктора — ptr валиден и данные неизменны.
        unsafe { &*self.ptr }
    }
}
// ANCHOR_END: unsafe-impl

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_runs_closure() {
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let f = flag.clone();
        in_background(move || f.store(true, std::sync::atomic::Ordering::SeqCst));
        assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn shared_pointer_crosses_scope() {
        let value = 42u32;
        // SAFETY: value живёт весь scope, читается только на чтение.
        let shared = unsafe { Shared::new(&value) };
        std::thread::scope(|s| {
            s.spawn(|| {
                // SAFETY: value жив, неизменен.
                assert_eq!(*unsafe { shared.get() }, 42);
            });
        });
    }
}
