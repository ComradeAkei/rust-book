pub fn channel(frame: &[u8]) -> Result<u8, String> {
    let kind = frame.first().copied().filter(|&k| k == 1).ok_or("не измерение")?;
    let ch = frame.get(2)?;
    Ok(kind + *ch)
}
