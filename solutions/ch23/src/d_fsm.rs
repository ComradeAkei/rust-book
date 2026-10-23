//! Задача 23.D: TT-muncher для конечных автоматов.
//!
//! ```
//! ch23_solutions::fsm! {
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
//! ch23_solutions::fsm! {
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
//! ch23_solutions::fsm! {
//!     pub enum Door { Closed, Opened, Locked }
//!     pub enum DoorEvent { Open, Lock }
//!     Closed --Open--> Opened,
//!     Closed --Open--> Locked, // два перехода из одного состояния по одному событию
//! }
//! ```

// ANCHOR: solution
/// Перечисления состояний и событий задаёт вызывающий: `macro_rules!` не
/// умеет ни сравнивать имена (убрать повторы из переходов), ни склеивать
/// их (`Door` → `DoorEvent`). Переходы разбирает TT-muncher: по одному за
/// шаг, раскладывая в три накопителя — конкретные, `_` («из любого
/// состояния») и рёбра графа для `_`.
#[macro_export]
macro_rules! fsm {
    // Вход съеден — выдать всё разом: ветки `match` по одной не выдать.
    (@munch $state:ident $event:ident [$($s:ident)*] [$($spec:tt)*] [$($any:tt)*] [$($edge:tt)*]) => {
        $crate::fsm!(@emit $state $event [$($spec)*] [$($any)*] [$($edge)*]);
    };
    // `_ --e--> t`: переход из любого состояния; для графа — ребро из
    // каждого состояния списка.
    (@munch $state:ident $event:ident [$($s:ident)*] [$($spec:tt)*] [$($any:tt)*] [$($edge:tt)*]
        _ --$ev:ident--> $to:ident $(, $($rest:tt)*)?) => {
        $crate::fsm!(@munch $state $event [$($s)*] [$($spec)*] [$($any)* ($ev $to)]
            [$($edge)* $(($s $ev $to))*] $($($rest)*)?);
    };
    // `a | b --e--> t`: по переходу на каждый источник.
    (@munch $state:ident $event:ident [$($s:ident)*] [$($spec:tt)*] [$($any:tt)*] [$($edge:tt)*]
        $($from:ident)|+ --$ev:ident--> $to:ident $(, $($rest:tt)*)?) => {
        $crate::fsm!(@munch $state $event [$($s)*] [$($spec)* $(($from $ev $to))+] [$($any)*]
            [$($edge)* $(($from $ev $to))+] $($($rest)*)?);
    };
    (@emit $state:ident $event:ident
        [$(($from:ident $ev:ident $to:ident))*]
        [$(($aev:ident $ato:ident))*]
        [$(($ef:ident $ee:ident $et:ident))*]) => {
        impl $state {
            /// Следующее состояние или `None`, если перехода нет. Конкретные
            /// переходы стоят раньше `_`, в каком бы порядке их ни написали.
            pub const fn next(self, event: $event) -> ::core::option::Option<Self> {
                match (self, event) {
                    $((Self::$from, $event::$ev) => ::core::option::Option::Some(Self::$to),)*
                    $((_, $event::$aev) => ::core::option::Option::Some(Self::$ato),)*
                    #[allow(unreachable_patterns)]
                    _ => ::core::option::Option::None,
                }
            }

            /// Таблица переходов; `None` в источнике — «из любого».
            pub const TRANSITIONS: &'static [(::core::option::Option<Self>, $event, Self)] = &[
                $((::core::option::Option::Some(Self::$from), $event::$ev, Self::$to),)*
                $((::core::option::Option::None, $event::$aev, Self::$ato),)*
            ];

            /// Граф для Graphviz: `dot -Tsvg`.
            pub const DOT: &'static str = ::core::concat!(
                "digraph ", ::core::stringify!($state), " {\n",
                $("  ", ::core::stringify!($ef), " -> ", ::core::stringify!($et),
                  " [label=\"", ::core::stringify!($ee), "\"];\n",)*
                "}\n"
            );
        }

        // Детерминизм: пара (источник, событие) встречается один раз;
        // конкретный переход вправе перекрыть `_` с тем же событием.
        const _: () = {
            let t = $state::TRANSITIONS;
            let mut i = 0;
            while i < t.len() {
                let mut j = i + 1;
                while j < t.len() {
                    let same_from = match (t[i].0, t[j].0) {
                        (::core::option::Option::Some(a), ::core::option::Option::Some(b)) => {
                            a as usize == b as usize
                        }
                        (::core::option::Option::None, ::core::option::Option::None) => true,
                        _ => false,
                    };
                    ::core::assert!(
                        !(same_from && t[i].1 as usize == t[j].1 as usize),
                        "два перехода из одного состояния по одному событию"
                    );
                    j += 1;
                }
                i += 1;
            }
        };
    };
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
        }

        $crate::fsm!(@munch $state $event [$($s)*] [] [] [] $($rules)*);
    };
}
// ANCHOR_END: solution
