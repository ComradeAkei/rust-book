//! Задача B: параллельный подсчёт частот слов. Текст режется на куски по
//! числу потоков; каждый поток считает частоты в СВОЙ `HashMap` (без общего
//! замка на горячем пути), затем локальные таблицы сливаются в одну. Так
//! конкуренция за блокировку возникает только при слиянии, а не на каждом
//! слове.

use std::collections::HashMap;
use std::thread;

// ANCHOR: solution
/// Частоты слов во всём тексте, посчитанные на `workers` потоках.
/// Слово — последовательность не-пробельных символов; регистр сохраняется.
pub fn word_frequencies(text: &str, workers: usize) -> HashMap<String, usize> {
    let lines: Vec<&str> = text.lines().collect();
    let workers = workers.max(1);
    let chunk = lines.len().div_ceil(workers).max(1);

    // Области потоков ([20.4]) одалживают срез строк без Arc: все потоки
    // завершатся до выхода из scope.
    let partials: Vec<HashMap<String, usize>> = thread::scope(|s| {
        let handles: Vec<_> = lines
            .chunks(chunk)
            .map(|part| {
                s.spawn(move || {
                    let mut local = HashMap::new();
                    for line in part {
                        for word in line.split_whitespace() {
                            *local.entry(word.to_owned()).or_insert(0) += 1;
                        }
                    }
                    local
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });

    // Слияние локальных таблиц — единственное место, где данные сходятся.
    let mut total: HashMap<String, usize> = HashMap::new();
    for local in partials {
        for (word, count) in local {
            *total.entry(word).or_insert(0) += count;
        }
    }
    total
}
// ANCHOR_END: solution
