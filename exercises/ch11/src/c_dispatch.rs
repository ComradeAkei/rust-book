//! Задача C: перечисление, `dyn` и обобщения на 2, 8 и 64 типах.
//!
//! Один и тот же обобщённый цикл `run` должен проходить три вида
//! элементов: `Box<dyn Op>` (вызов через таблицу), перечисление (`match`)
//! и однородную группу `OpK<K>` (прямой вызов). Типы операций,
//! перечисления и группы уже написаны, нужны функции из блока
//! `student`. Время меряет пример `c_bench`.

use alloc::boxed::Box;
use alloc::vec::Vec;

// ANCHOR: op
pub trait Op {
    fn apply(&self, x: u32) -> u32;
}

/// Операция номер `k` над входом `x` и значением элемента `v`. Формула
/// задана условием: по ней тесты считают ответ.
pub const fn formula(k: u32, v: u32, x: u32) -> u32 {
    (x ^ v).wrapping_mul(2 * k + 1).rotate_left(k % 31 + 1) ^ k
}

/// Операция `K`: у каждого `K` свой тип и своя копия `apply`.
#[derive(Clone, Copy)]
pub struct OpK<const K: u32> {
    pub v: u32,
}

impl<const K: u32> Op for OpK<K> {
    fn apply(&self, x: u32) -> u32 {
        formula(K, self.v, x)
    }
}

/// Генератор xorshift32 для перемешивания.
pub fn xorshift(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}
// ANCHOR_END: op

// ANCHOR: family
/// Набор из `N` типов операций `OpK<0>` … `OpK<N−1>`. Реализации для
/// [`Two`], [`Eight`] и [`SixtyFour`] пишет макрос `family!` ниже.
pub trait Family {
    const N: u8;
    /// Перечисление с вариантом на каждый тип набора.
    type Enum: Op + Copy;
    /// Однородные группы: `Vec<OpK<K>>` на каждый `K`.
    type Groups;
    /// Вариант перечисления для типа `k` со значением `v`.
    fn variant(k: u8, v: u32) -> Self::Enum;
    /// Объект в куче для типа `k`: зовёт `make_dyn::<k>`.
    fn boxed(k: u8, v: u32) -> Box<dyn Op>;
    /// Разложить элементы по группам: элемент `i` типа `kinds[i]` получает
    /// значение `v = i`.
    fn groups(kinds: &[u8]) -> Self::Groups;
    /// Сумма `run` по всем группам.
    fn run_groups(g: &Self::Groups, x: u32) -> u32;
}
// ANCHOR_END: family

// ANCHOR: student
/// Номера типов для `len` элементов: каждый из `n` типов встречается
/// `len / n` раз. `sorted` — по возрастанию, иначе — перестановка
/// Фишера — Йетса тех же номеров генератором `xorshift` с состоянием 1:
/// для `i` от `len − 1` до 1 обменять элементы `i` и `xorshift() % (i + 1)`.
pub fn kinds(n: u8, len: usize, sorted: bool) -> Vec<u8> {
    todo!()
}

/// Пересылка: объект в куче — тоже операция.
impl Op for Box<dyn Op> {
    fn apply(&self, x: u32) -> u32 {
        todo!()
    }
}

/// Сумма `apply(x)` по всем элементам с переносом по модулю 2³². Одна
/// функция на все три вида элементов.
pub fn run<T: Op>(items: &[T], x: u32) -> u32 {
    todo!()
}

pub fn make_dyn<const K: u32>(v: u32) -> Box<dyn Op> {
    todo!()
}

/// Элемент `i` типа `kinds[i]` со значением `v = i`.
pub fn build_dyn<F: Family>(kinds: &[u8]) -> Vec<Box<dyn Op>> {
    todo!()
}

pub fn build_enum<F: Family>(kinds: &[u8]) -> Vec<F::Enum> {
    todo!()
}
// ANCHOR_END: student

/// Перечисление, группы и реализация `Family` для набора типов. Макросы
/// разбирает глава 23; здесь он только избавляет от 64 одинаковых строк.
macro_rules! family {
    ($fam:ident, $enum:ident, $groups:ident; $($var:ident $k:literal)*) => {
        pub struct $fam;

        #[derive(Clone, Copy)]
        pub enum $enum {
            $($var(OpK<$k>),)*
        }

        impl Op for $enum {
            fn apply(&self, x: u32) -> u32 {
                match self {
                    $($enum::$var(o) => o.apply(x),)*
                }
            }
        }

        #[allow(non_snake_case)]
        #[derive(Default)]
        pub struct $groups {
            $(pub $var: Vec<OpK<$k>>,)*
        }

        impl Family for $fam {
            const N: u8 = [$($k),*].len() as u8;
            type Enum = $enum;
            type Groups = $groups;

            fn variant(k: u8, v: u32) -> $enum {
                match k {
                    $($k => $enum::$var(OpK { v }),)*
                    _ => panic!("нет типа {k}"),
                }
            }

            fn boxed(k: u8, v: u32) -> Box<dyn Op> {
                match k {
                    $($k => make_dyn::<$k>(v),)*
                    _ => panic!("нет типа {k}"),
                }
            }

            fn groups(kinds: &[u8]) -> $groups {
                let mut g = $groups::default();
                for (i, &k) in kinds.iter().enumerate() {
                    match k {
                        $($k => g.$var.push(OpK { v: i as u32 }),)*
                        _ => panic!("нет типа {k}"),
                    }
                }
                g
            }

            fn run_groups(g: &$groups, x: u32) -> u32 {
                0u32 $(.wrapping_add(run(&g.$var, x)))*
            }
        }
    };
}

family!(Two, Any2, Groups2; K0 0 K1 1);
family!(Eight, Any8, Groups8; K0 0 K1 1 K2 2 K3 3 K4 4 K5 5 K6 6 K7 7);
family!(SixtyFour, Any64, Groups64;
    K0 0 K1 1 K2 2 K3 3 K4 4 K5 5 K6 6 K7 7 K8 8 K9 9 K10 10 K11 11 K12 12 K13 13 K14 14 K15 15
    K16 16 K17 17 K18 18 K19 19 K20 20 K21 21 K22 22 K23 23 K24 24 K25 25 K26 26 K27 27 K28 28
    K29 29 K30 30 K31 31 K32 32 K33 33 K34 34 K35 35 K36 36 K37 37 K38 38 K39 39 K40 40 K41 41
    K42 42 K43 43 K44 44 K45 45 K46 46 K47 47 K48 48 K49 49 K50 50 K51 51 K52 52 K53 53 K54 54
    K55 55 K56 56 K57 57 K58 58 K59 59 K60 60 K61 61 K62 62 K63 63);

// ANCHOR: asm
/// Точки для ассемблера: тот же цикл для трёх видов элементов.
#[unsafe(no_mangle)]
pub fn ch11_c_run_dyn(items: &[Box<dyn Op>], x: u32) -> u32 {
    run(items, x)
}

#[unsafe(no_mangle)]
pub fn ch11_c_run_enum2(items: &[Any2], x: u32) -> u32 {
    run(items, x)
}

#[unsafe(no_mangle)]
pub fn ch11_c_run_enum8(items: &[Any8], x: u32) -> u32 {
    run(items, x)
}

#[unsafe(no_mangle)]
pub fn ch11_c_run_enum64(items: &[Any64], x: u32) -> u32 {
    run(items, x)
}

#[unsafe(no_mangle)]
pub fn ch11_c_run_static(items: &[OpK<5>], x: u32) -> u32 {
    run(items, x)
}
// ANCHOR_END: asm
