/// Более длинный из двух кадров.
pub fn pick(a: &[u8], b: &[u8]) -> &[u8] {
    if a.len() >= b.len() { a } else { b }
}
