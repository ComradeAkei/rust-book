// Задача 4.D, часть 1. Напишите acquire так, как написали бы на C: цикл по
// пулу с ранним возвратом найденного слота и push после цикла. rustc 1.97
// должен её отвергнуть, Polonius (-Zpolonius=next на nightly) — принять.
use super::Slot;

pub fn acquire(pool: &mut Vec<Slot>) -> &mut Slot {
    todo!()
}
