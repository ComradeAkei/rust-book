//! Раздел 30.3: программа для профилировщика — три фазы разной цены.

// ANCHOR: noise
/// `n` псевдослучайных байт: xorshift64. Без `#[inline]` функция не
/// встраивается в код другого крейта — программа `profile` вызывает её,
/// и в профиле она видна отдельно (раздел 30.5).
pub fn noise(n: usize, seed: u64) -> Vec<u8> {
    let mut x = seed | 1;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        out.push((x >> 56) as u8);
    }
    out
}
// ANCHOR_END: noise

// ANCHOR: crc
/// CRC-16/CCITT-FALSE побитово: восемь шагов на байт.
#[inline(never)]
pub fn crc_bitwise(data: &[u8]) -> u16 {
    let mut crc = 0xFFFFu16;
    for &b in data {
        crc ^= u16::from(b) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}
// ANCHOR_END: crc

// ANCHOR: histogram
/// Сколько раз встретился каждый байт.
#[inline(never)]
pub fn histogram(data: &[u8]) -> [u32; 256] {
    let mut h = [0u32; 256];
    for &b in data {
        h[usize::from(b)] += 1;
    }
    h
}
// ANCHOR_END: histogram

// ANCHOR: sorted
/// Отсортированная копия.
#[inline(never)]
pub fn sorted(data: &[u8]) -> Vec<u8> {
    let mut v = data.to_vec();
    v.sort_unstable();
    v
}
// ANCHOR_END: sorted

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases() {
        assert_eq!(crc_bitwise(b"123456789"), 0x29B1);
        let d = noise(10_000, 7);
        assert_eq!(histogram(&d).iter().sum::<u32>(), 10_000);
        assert!(sorted(&d).is_sorted());
    }
}
