//! Задача B: параллельный подсчёт частот слов. Условие — раздел 20.7.

use std::collections::HashMap;

// ANCHOR: skeleton
/// Частоты слов во всём тексте на `workers` потоках. Режьте текст на куски,
/// считайте в свой `HashMap` на поток через `thread::scope` ([20.4]), затем
/// слейте локальные таблицы. Слово — последовательность не-пробельных
/// символов (`split_whitespace`), регистр сохраняется.
pub fn word_frequencies(text: &str, workers: usize) -> HashMap<String, usize> {
    let _ = (text, workers);
    todo!("раздел 20.7, задача B")
}
// ANCHOR_END: skeleton
