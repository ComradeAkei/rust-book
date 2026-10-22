//! Задача 4.D: обходные пути для двух корректных программ, которые
//! проверка заимствований rustc 1.97 отвергает. Сами программы пишутся в
//! `polonius/acquire.rs` и `polonius/push_back.rs`.

use alloc::boxed::Box;
use alloc::vec::Vec;

/// Слот пула буферов.
pub struct Slot {
    pub busy: bool,
    pub data: [u8; 16],
}

/// Первый свободный слот, помеченный занятым; если свободных нет — новый в
/// конце пула. Версия, которую принимает stable.
pub fn acquire(pool: &mut Vec<Slot>) -> &mut Slot {
    todo!()
}

/// Узел односвязного списка.
pub struct Node {
    pub val: u32,
    pub next: Option<Box<Node>>,
}

/// Добавить значение в конец списка. Версия, которую принимает stable.
pub fn push_back(head: &mut Node, val: u32) {
    todo!()
}
