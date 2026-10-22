/// Сколько строк в `text`; `lines` очищается для следующего прохода.
pub fn count_lines(text: &str, lines: &mut Vec<&str>) -> usize {
    lines.clear();
    text.lines().count()
}
