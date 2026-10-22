/// Строка без завершающего `\r`.
pub fn trim_cr(line: &str) -> &str {
    line.strip_suffix('\r').unwrap_or(line)
}
