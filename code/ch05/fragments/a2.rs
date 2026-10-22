/// `"baud=9600"` → `("baud", "9600")`; без разделителя значение пустое.
pub fn split_kv(line: &str, sep: char) -> (&str, &str) {
    line.split_once(sep).unwrap_or((line, ""))
}
