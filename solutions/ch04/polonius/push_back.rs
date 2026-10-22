// Задача 4.D, часть 2: курсор по списку. Программа корректна: после цикла
// заимствование next из последней итерации уже не нужно. Её отвергают и
// rustc 1.97, и Polonius в режиме -Zpolonius=next (E0506).
use super::Node;

pub fn push_back(head: &mut Node, val: u32) {
    let mut cur = head;
    while let Some(next) = &mut cur.next {
        cur = next;
    }
    cur.next = Some(Box::new(Node { val, next: None }));
}
