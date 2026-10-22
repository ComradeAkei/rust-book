//! Задача 4.D: обходные пути для двух корректных программ из `polonius/`,
//! которые проверка заимствований rustc 1.97 отвергает.

use alloc::boxed::Box;
use alloc::vec::Vec;

/// Слот пула буферов.
pub struct Slot {
    pub busy: bool,
    pub data: [u8; 16],
}

/// Первый свободный слот, помеченный занятым; если свободных нет — новый в
/// конце пула.
///
/// Поиск возвращает номер, а не ссылку, поэтому заимствование для поиска
/// кончается до `push`. Цена — повторная проверка границ в `pool[i]`.
pub fn acquire(pool: &mut Vec<Slot>) -> &mut Slot {
    let i = match pool.iter().position(|s| !s.busy) {
        Some(i) => i,
        None => {
            pool.push(Slot { busy: false, data: [0; 16] });
            pool.len() - 1
        }
    };
    let s = &mut pool[i];
    s.busy = true;
    s
}

/// Узел односвязного списка.
pub struct Node {
    pub val: u32,
    pub next: Option<Box<Node>>,
}

/// Добавить значение в конец списка.
///
/// Условие цикла только проверяет `next` и заимствование не сохраняет.
/// Курсор сдвигается новым заимствованием из `cur`, и после цикла `cur`
/// свободен.
pub fn push_back(head: &mut Node, val: u32) {
    let mut cur = head;
    while cur.next.is_some() {
        cur = cur.next.as_mut().unwrap();
    }
    cur.next = Some(Box::new(Node { val, next: None }));
}
