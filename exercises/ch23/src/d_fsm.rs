//! Задача D: TT-muncher для конечных автоматов.
//!
//! Образец разбирает объявления перечислений и генерирует их вместе со
//! списком `STATES`, но переходы пока не разбирает: `next` — заглушка, а
//! таблица и граф пусты. Допишите правила, которые съедают переходы по
//! одному: `a --e--> b`, `a | b --e--> c` и `_ --e--> c` («из любого
//! состояния»; конкретные переходы важнее, где бы `_` ни стоял). Ветки
//! `match` макрос по одной выдать не может — копите их и выдавайте разом.
//! Повтор пары (состояние, событие) должен быть ошибкой компиляции.
//!
//! ```
//! ch23_exercises::fsm! {
//!     pub enum Door { Closed, Opened }
//!     pub enum DoorEvent { Open, Close }
//!     Closed --Open--> Opened,
//!     Opened --Close--> Closed,
//! }
//! assert_eq!(Door::Closed.next(DoorEvent::Open), Some(Door::Opened));
//! assert_eq!(Door::Closed.next(DoorEvent::Close), None);
//! // `next` — `const fn`: переход вычисляется и при компиляции.
//! const AFTER: Option<Door> = Door::Opened.next(DoorEvent::Close);
//! assert_eq!(AFTER, Some(Door::Closed));
//! ```
//!
//! Состояние, которого нет в списке, — ошибка компиляции с указанием на
//! имя в переходе:
//!
//! ```compile_fail,E0599
//! ch23_exercises::fsm! {
//!     pub enum Door { Closed, Opened }
//!     pub enum DoorEvent { Open, Close }
//!     Closed --Open--> Openned, // no variant named `Openned`
//! }
//! ```
//!
//! Два перехода из одного состояния по одному событию — недетерминизм.
//! Вторая ветка `match` недостижима, но предупреждение `unreachable_patterns`
//! в коде из макроса чужого крейта компилятор не показывает. Поэтому
//! таблицу проверяет `const`-блок — ошибка компиляции, но с указанием на
//! проверку в макросе, а не на повторный переход:
//!
//! ```compile_fail,E0080
//! ch23_exercises::fsm! {
//!     pub enum Door { Closed, Opened, Locked }
//!     pub enum DoorEvent { Open, Lock }
//!     Closed --Open--> Opened,
//!     Closed --Open--> Locked, // два перехода из одного состояния по одному событию
//! }
//! ```

// ANCHOR: skeleton
#[macro_export]
macro_rules! fsm {
    (
        $(#[$sm:meta])*
        $svis:vis enum $state:ident { $($s:ident),* $(,)? }
        $(#[$em:meta])*
        $evis:vis enum $event:ident { $($e:ident),* $(,)? }
        $($rules:tt)*
    ) => {
        $(#[$sm])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $svis enum $state {
            $($s),*
        }

        $(#[$em])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $evis enum $event {
            $($e),*
        }

        impl $state {
            pub const STATES: &'static [Self] = &[$(Self::$s),*];

            /// Следующее состояние или `None`, если перехода нет.
            pub const fn next(self, event: $event) -> ::core::option::Option<Self> {
                ::core::panic!("не решено: раздел 23.7, задача D")
            }

            /// Таблица переходов; `None` в источнике — «из любого».
            pub const TRANSITIONS: &'static [(::core::option::Option<Self>, $event, Self)] = &[];

            /// Граф для Graphviz.
            pub const DOT: &'static str = "";
        }
    };
}
// ANCHOR_END: skeleton
