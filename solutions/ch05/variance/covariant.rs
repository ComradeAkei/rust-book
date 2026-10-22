// Задача 5.C, часть 1: функция, которая компилируется только благодаря
// ковариантности.

/// Порты от пользователя или встроенный список. `builtin` имеет тип
/// `&'a Vec<&'static str>` и возвращается как `&'a Vec<&'a str>`: `&'a T` и
/// `Vec<T>` ковариантны по `T`, а `&'static str` годится вместо `&'a str`.
pub fn choose<'a>(user: &'a Vec<&'a str>, builtin: &'a Vec<&'static str>) -> &'a Vec<&'a str> {
    if user.is_empty() { builtin } else { user }
}
