// Задача 5.C, часть 2: та же функция, но строки лежат в `Cell`.
use std::cell::Cell;

/// Если бы это компилировалось, вызывающий мог бы через результат
/// записать `Cell::set` короткую `&'a str` в ячейку встроенного списка, а
/// тот, кто держит `&Vec<Cell<&'static str>>`, прочитал бы её как
/// `&'static str` после смерти строки.
pub fn choose<'a>(
    user: &'a Vec<Cell<&'a str>>,
    builtin: &'a Vec<Cell<&'static str>>,
) -> &'a Vec<Cell<&'a str>> {
    if user.is_empty() { builtin } else { user }
}
