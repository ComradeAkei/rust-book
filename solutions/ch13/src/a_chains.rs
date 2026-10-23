//! Задача A: ленивость и ранний выход. Ответы и объяснения — в приложении
//! E.

// ANCHOR: chains
use core::cell::Cell;

pub const DATA: [i32; 6] = [3, 8, 1, 9, 4, 7];

/// Восемь цепочек над `DATA`: результат каждой — одно число (`None` —
/// это −1, `Err(e)` — это −e) и сколько раз вызвано замыкание `f` внутри
/// `map`.
// Цепочки нарочно написаны так, как их не пишут: `last` у двустороннего
// итератора идёт до конца, и задача об этом и спрашивает.
#[allow(clippy::double_ended_iterator_last)]
pub fn measured() -> [(i32, usize); 8] {
    let calls = Cell::new(0);
    let f = |x: &i32| {
        calls.set(calls.get() + 1);
        x * 10
    };
    let run = |chain: &dyn Fn() -> i32| {
        calls.set(0);
        let r = chain();
        (r, calls.get())
    };
    let opt = |o: Option<i32>| o.unwrap_or(-1);
    [
        run(&|| opt(DATA.iter().map(&f).find(|&x| x > 50))),
        run(&|| DATA.iter().map(&f).rev().take(2).sum()),
        run(&|| DATA.iter().map(&f).count() as i32),
        run(&|| DATA.iter().map(&f).len() as i32),
        run(&|| DATA.iter().map(&f).take_while(|&x| x < 50).count() as i32),
        run(&|| DATA.iter().map(&f).skip_while(|&x| x < 50).count() as i32),
        run(&|| {
            let r: Result<i32, i32> =
                DATA.iter().map(&f).map(|x| if x > 85 { Err(x) } else { Ok(x) }).sum();
            r.unwrap_or_else(|e| -e)
        }),
        run(&|| opt(DATA.iter().filter(|&&x| x % 2 == 1).map(&f).last())),
    ]
}
// ANCHOR_END: chains

// ANCHOR: predicted
/// Результаты и числа вызовов цепочек 1–8.
pub const fn predicted() -> [(i32, usize); 8] {
    [
        (80, 2),  //  find: 3 и 8, на 80 — выход
        (110, 2), //  с конца: 7 и 4
        (6, 6),   //  count перебирает всё и зовёт f у каждого
        (6, 0),   //  len берёт длину у среза, f не нужен
        (1, 2),   //  take_while забрал и 80, на котором остановился
        (5, 6),   //  skip_while проверил 3 и 8, остальные отдал
        (-90, 4), //  sum по Result остановился на первой ошибке
        (70, 4),  //  нечётных четыре, last дошёл до конца
    ]
}
// ANCHOR_END: predicted
