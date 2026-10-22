// ANCHOR: line
/// Строка журнала для одного измерения.
pub fn line(seq: u32, value: i32) -> String {
    if cfg!(feature = "hex") {
        format!("кадр {seq}: {value:#x}")
    } else {
        format!("кадр {seq}: {value}")
    }
}
// ANCHOR_END: line
