//! Задача C: дайте `StrBuf` честный метод `view`, чья сигнатура не лжёт.
//! Условие — раздел 16.7. Подумайте, какое время жизни результат может
//! обещать, ничего не нарушая, и нужен ли здесь `unsafe` вообще.

// ANCHOR: skeleton
pub struct StrBuf {
    inner: String,
}

impl StrBuf {
    pub fn new(s: &str) -> Self {
        StrBuf { inner: s.to_string() }
    }

    /// Верните взгляд на содержимое со временем жизни, связанным с `&self`.
    pub fn view(&self) -> &str {
        let _ = &self.inner;
        todo!("раздел 16.7, задача C")
    }
}
// ANCHOR_END: skeleton
