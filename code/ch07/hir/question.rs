// Раздел 7.1: во что превращается `?`. Снимок промежуточного представления
// HIR: rustc +nightly --edition 2024 --crate-type lib -Zunpretty=hir question.rs
pub fn first(v: &[u8]) -> Result<u8, &'static str> {
    let x = v.first().ok_or("empty")?;
    Ok(*x)
}
