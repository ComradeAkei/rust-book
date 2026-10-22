//! Раздел 4.6: где проверка заимствований ошибается.
//!
//! Программа, которую проверка отвергает, лежит в
//! `code/ch04/rejected/get_or_insert.rs` и в крейт не входит: doc-тест
//! `compile_fail` здесь не годится, потому что nightly 1.101 её уже
//! принимает. Тест `tests/rejected.rs` собирает её на stable (ошибка E0502)
//! и на nightly с `-Zpolonius=next` (без ошибок). Здесь — обходные пути,
//! которые компилируются везде.

use alloc::collections::BTreeMap;
use alloc::string::String;

// ANCHOR: workaround
/// Обход первый: искать дважды. Ссылка, которую возвращает ранний выход,
/// берётся уже после проверки и не пересекается со вставкой.
pub fn get_or_insert_twice(map: &mut BTreeMap<u32, String>, k: u32) -> &String {
    if map.contains_key(&k) {
        return &map[&k];
    }
    map.insert(k, String::new());
    &map[&k]
}

/// Обход второй и обычный: `entry` ищет один раз и отдаёт место для
/// вставки.
pub fn get_or_insert_entry(map: &mut BTreeMap<u32, String>, k: u32) -> &String {
    map.entry(k).or_default()
}
// ANCHOR_END: workaround

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workarounds() {
        let mut m = BTreeMap::new();
        m.insert(1, String::from("один"));
        assert_eq!(get_or_insert_twice(&mut m, 1), "один");
        assert_eq!(get_or_insert_twice(&mut m, 2), "");
        assert_eq!(get_or_insert_entry(&mut m, 1), "один");
        assert_eq!(get_or_insert_entry(&mut m, 3), "");
        assert_eq!(m.len(), 3);
    }
}
