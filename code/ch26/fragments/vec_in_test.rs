// Раздел 26.6: тест no_std-крейта собирается с std, но в области
// видимости её нет.
#![no_std]

pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[cfg(test)]
mod tests {
    #[test]
    fn adds() {
        let v = vec![super::add(2, 2)];
        assert_eq!(v[0], 4);
    }
}
