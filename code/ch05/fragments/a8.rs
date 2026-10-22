pub struct Parser<'s> {
    src: &'s str,
    word: String,
}

impl<'s> Parser<'s> {
    /// Следующее слово в нижнем регистре. Слово собирается в `self.word`.
    pub fn next_token(&mut self) -> Option<&str> {
        let src = self.src.trim_start();
        let len = src.find(' ').unwrap_or(src.len());
        if len == 0 {
            return None;
        }
        self.word = src[..len].to_lowercase();
        self.src = &src[len..];
        Some(&self.word)
    }
}
