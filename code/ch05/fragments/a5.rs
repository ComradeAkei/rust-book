/// Фильтр строк по префиксу. Префикс копируется в замыкание.
pub fn make_filter(prefix: &str) -> Box<dyn Fn(&str) -> bool> {
    let prefix = prefix.to_owned();
    Box::new(move |s: &str| s.starts_with(prefix.as_str()))
}
