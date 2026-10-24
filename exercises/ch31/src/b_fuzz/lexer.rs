//! Копия токенизатора конфигурации из задачи 5.B
//! (`solutions/ch05/src/b_lexer.rs`) с одной внедрённой ошибкой. Найдите
//! её фаззингом; копию в главе 5 не открывайте, пока не найдёте.

/// Лексема. Строки внутри — срезы исходного текста, `'a` — его время жизни.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Token<'a> {
    /// `[uart]` → `Section("uart")`.
    Section(&'a str),
    /// `baud = 9600` → `Key("baud")`, затем `Value("9600")`.
    Key(&'a str),
    /// Значение без пробелов по краям и без комментария; у значения в
    /// кавычках — содержимое кавычек.
    Value(&'a str),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LexErrorKind {
    /// `[uart` — нет `]`.
    UnclosedSection,
    /// Пустое имя секции или ключа либо символ не из `A–Z a–z 0–9 _ . -`.
    BadName,
    /// После ключа нет `=`.
    MissingEquals,
    /// `"…` без закрывающей кавычки.
    UnclosedQuote,
    /// После `]` или закрывающей кавычки что-то кроме пробелов и комментария.
    TrailingGarbage,
}

/// Ошибка с позицией: строка и столбец с 1, столбец — в символах, не в
/// байтах.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct LexError {
    pub line: u32,
    pub col: u32,
    pub kind: LexErrorKind,
}

/// Лексер держит только ссылку на текст и счётчики. После ошибки он
/// переходит к следующей строке, так что можно собрать все ошибки файла.
pub struct Lexer<'a> {
    /// Ещё не прочитанные строки.
    rest: &'a str,
    /// Номер последней прочитанной строки.
    line: u32,
    /// Значение, которое выдаётся следующим после ключа.
    pending: Option<Token<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { rest: src, line: 0, pending: None }
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token<'a>, LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(t) = self.pending.take() {
            return Some(Ok(t));
        }
        loop {
            let line = self.next_line()?;
            if let Some(r) = self.lex_line(line) {
                return Some(r);
            }
        }
    }
}

fn is_name(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')
}

/// Остаток строки после `]` или кавычки: `None`, если там только пробелы
/// и комментарий, иначе срез с первого лишнего символа.
fn garbage(after: &str) -> Option<&str> {
    let t = after.trim_start();
    if t.is_empty() || t.starts_with(['#', ';']) { None } else { Some(t) }
}

/// Начало комментария в значении: `#` или `;` в начале или после пробела.
fn comment_start(v: &str) -> Option<usize> {
    let mut prev_space = true;
    for (i, c) in v.chars().enumerate() {
        if prev_space && (c == '#' || c == ';') {
            return Some(i);
        }
        prev_space = c.is_whitespace();
    }
    None
}

impl<'a> Lexer<'a> {
    /// Следующая строка без `\n` и `\r` на конце.
    fn next_line(&mut self) -> Option<&'a str> {
        if self.rest.is_empty() {
            return None;
        }
        let (line, rest) = self.rest.split_once('\n').unwrap_or((self.rest, ""));
        self.rest = rest;
        self.line += 1;
        Some(line.strip_suffix('\r').unwrap_or(line))
    }

    /// Лексема из строки или ошибка; `None` для пустой строки и комментария.
    fn lex_line(&mut self, line: &'a str) -> Option<Result<Token<'a>, LexError>> {
        let n = self.line;
        // `at` — хвост строки, начинающийся с ошибочного символа.
        let err = |at: &str, kind| {
            let col = line[..line.len() - at.len()].chars().count() as u32 + 1;
            Some(Err(LexError { line: n, col, kind }))
        };
        let s = line.trim_start();
        if s.is_empty() || s.starts_with(['#', ';']) {
            return None;
        }
        if let Some(body) = s.strip_prefix('[') {
            let Some(end) = body.find(']') else {
                return err(s, LexErrorKind::UnclosedSection);
            };
            let name = &body[..end];
            if name.is_empty() {
                return err(&body[end..], LexErrorKind::BadName);
            }
            if let Some(i) = name.find(|c| !is_name(c)) {
                return err(&body[i..], LexErrorKind::BadName);
            }
            if let Some(g) = garbage(&body[end + 1..]) {
                return err(g, LexErrorKind::TrailingGarbage);
            }
            return Some(Ok(Token::Section(name)));
        }
        let key_end = s.find(|c| !is_name(c)).unwrap_or(s.len());
        let (key, after) = s.split_at(key_end);
        if key.is_empty() {
            return err(s, LexErrorKind::BadName);
        }
        if after.starts_with(|c: char| !c.is_whitespace() && c != '=') {
            return err(after, LexErrorKind::BadName);
        }
        let t = after.trim_start();
        let Some(v) = t.strip_prefix('=') else {
            return err(t, LexErrorKind::MissingEquals);
        };
        let v = v.trim_start();
        let value = if let Some(q) = v.strip_prefix('"') {
            let Some(end) = q.find('"') else {
                return err(v, LexErrorKind::UnclosedQuote);
            };
            if let Some(g) = garbage(&q[end + 1..]) {
                return err(g, LexErrorKind::TrailingGarbage);
            }
            &q[..end]
        } else {
            v[..comment_start(v).unwrap_or(v.len())].trim_end()
        };
        self.pending = Some(Token::Value(value));
        Some(Ok(Token::Key(key)))
    }
}
