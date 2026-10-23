//! Раздел 15.3: `HashMap`. Таблица SwissTable с ключом SipHash-1-3,
//! случайным для каждого процесса, API `entry` и хешер из K&R, на котором
//! видно, от чего этот ключ защищает.
//!
//! Указатель на значение, как `struct entry *` в C, не переживает вставку:
//!
//! ```compile_fail,E0499
//! use std::collections::HashMap;
//! let mut m: HashMap<&str, u32> = HashMap::new();
//! m.insert("adc0", 0);
//! let adc = m.get_mut("adc0").unwrap();
//! m.insert("i2c0", 0); // таблица может вырасти
//! *adc += 1;
//! ```

use std::collections::HashMap;
use std::hash::{BuildHasher, BuildHasherDefault, Hasher};

// ANCHOR: entry
/// Число измерений по каждому датчику: один поиск на измерение, без
/// `contains_key` перед `insert`.
pub fn count_by_sensor<'a>(names: &[&'a str]) -> HashMap<&'a str, u32> {
    let mut m = HashMap::new();
    for &n in names {
        *m.entry(n).or_insert(0) += 1;
    }
    m
}
// ANCHOR_END: entry

// ANCHOR: kr
/// Хеш из K&R, раздел 6.6: `h = c + 31 * h` по байтам. Здесь в 64 битах.
#[derive(Default, Clone, Copy)]
pub struct KrHasher(u64);

impl Hasher for KrHasher {
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 = (b as u64).wrapping_add(self.0.wrapping_mul(31));
        }
    }

    fn finish(&self) -> u64 {
        self.0
    }
}

/// `HashMap<K, V, Kr>` — таблица с хешем K&R вместо SipHash.
pub type Kr = BuildHasherDefault<KrHasher>;
// ANCHOR_END: kr

// ANCHOR: colliding
/// Имя из `k` блоков «Aa» и «BB», как в `table.c`: биты `i` выбирают блоки.
/// У «Aa» и «BB» один хеш K&R, поэтому и у всех 2^k имён тоже: хеш
/// склейки равных по длине блоков с равными хешами не различается.
pub fn colliding_name(i: u32, k: u32) -> String {
    (0..k).map(|b| if (i >> b) & 1 == 1 { "BB" } else { "Aa" }).collect()
}
// ANCHOR_END: colliding

// ANCHOR: h1h2
/// Как SwissTable делит хеш: младшие биты выбирают группу, с которой
/// начинается поиск, старшие семь — байт-метку в управляющем массиве.
pub fn h1_h2(hash: u64, buckets: usize) -> (usize, u8) {
    debug_assert!(buckets.is_power_of_two());
    (hash as usize & (buckets - 1), (hash >> 57) as u8)
}
// ANCHOR_END: h1h2

/// Хеш ключа той функцией, которой его считает таблица `m`.
pub fn hash_of<K: std::hash::Hash, V, S: BuildHasher>(m: &HashMap<K, V, S>, k: &K) -> u64 {
    m.hasher().hash_one(k)
}

// ANCHOR: order
/// Ключи в порядке обхода таблицы. У каждой `HashMap::new()` свой ключ
/// SipHash, поэтому порядок разный от таблицы к таблице и от запуска к
/// запуску.
pub fn iteration_order(n: u32) -> Vec<u32> {
    let m: HashMap<u32, ()> = (0..n).map(|k| (k, ())).collect();
    m.into_keys().collect()
}
// ANCHOR_END: order

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_counts() {
        let m = count_by_sensor(&["adc0", "i2c1", "adc0"]);
        assert_eq!((m["adc0"], m["i2c1"]), (2, 1));
    }

    #[test]
    fn kr_collides_like_c() {
        let b = Kr::default();
        assert_eq!(b.hash_one("Aa"), b.hash_one("BB"));
        let h = b.hash_one(colliding_name(0, 13));
        for i in [1, 77, 8191] {
            assert_eq!(b.hash_one(colliding_name(i, 13)), h);
        }
        assert_eq!(colliding_name(0b10, 3), "AaBBAa");
    }

    #[test]
    fn remove_keeps_neighbours() {
        // То, на чём ломается table_del: у Aa и BB один хеш.
        let mut m: HashMap<&str, u32, Kr> = HashMap::default();
        m.insert("Aa", 1);
        m.insert("BB", 2);
        m.remove("Aa");
        assert_eq!(m.get("BB"), Some(&2));
    }

    #[test]
    fn h1_h2_split() {
        assert_eq!(h1_h2(0xFE00_0000_0000_0013, 16), (3, 0x7f));
    }
}
