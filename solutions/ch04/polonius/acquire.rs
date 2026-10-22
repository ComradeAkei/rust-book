// Задача 4.D, часть 1: acquire так, как её пишут на C. Программа
// корректна: ссылка s уходит наружу только через return, а push
// выполняется, когда цикл кончился без возврата. rustc 1.97 отвергает её
// (E0499), Polonius принимает.
use super::Slot;

pub fn acquire(pool: &mut Vec<Slot>) -> &mut Slot {
    for s in pool.iter_mut() {
        if !s.busy {
            s.busy = true;
            return s;
        }
    }
    pool.push(Slot { busy: true, data: [0; 16] });
    pool.last_mut().unwrap()
}
