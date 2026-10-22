// Раздел 4.6: условный возврат заимствования. Программа корректна, но
// проверка заимствований в rustc её отвергает (E0502); Polonius принимает.
use std::collections::BTreeMap;

pub fn get_or_insert(map: &mut BTreeMap<u32, String>, k: u32) -> &String {
    if let Some(v) = map.get(&k) {
        return v;
    }
    map.insert(k, String::new());
    map.get(&k).unwrap()
}
