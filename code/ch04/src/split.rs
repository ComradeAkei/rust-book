//! Раздел 4.3: раздельные заимствования.
//!
//! Метод заимствует `self` целиком, даже если читает одно поле:
//!
//! ```compile_fail,E0502
//! use ch04::split::Uart;
//!
//! fn echo(u: &mut Uart) {
//!     let rx = u.received();
//!     let tx = u.tx_mut();
//!     tx[..rx.len()].copy_from_slice(rx);
//! }
//! ```
//!
//! Индексация заимствует весь срез, а не один элемент:
//!
//! ```compile_fail,E0499
//! let mut v = [1u8, 2, 3];
//! core::mem::swap(&mut v[0], &mut v[2]);
//! ```

// ANCHOR: fields
/// Буферы UART: что принято и что отправить.
pub struct Uart {
    pub rx: [u8; 16],
    pub rx_len: usize,
    pub tx: [u8; 16],
    pub tx_len: usize,
}

/// Поля заимствуются раздельно: `&` на одно поле и `&mut` на другое живут
/// одновременно.
pub fn echo(u: &mut Uart) {
    let rx = &u.rx[..u.rx_len];
    let tx = &mut u.tx;
    tx[..rx.len()].copy_from_slice(rx);
    u.tx_len = rx.len();
}
// ANCHOR_END: fields

// ANCHOR: methods
impl Uart {
    pub const fn new() -> Self {
        Uart { rx: [0; 16], rx_len: 0, tx: [0; 16], tx_len: 0 }
    }

    pub fn received(&self) -> &[u8] {
        &self.rx[..self.rx_len]
    }

    pub fn tx_mut(&mut self) -> &mut [u8; 16] {
        &mut self.tx
    }

    /// Один метод отдаёт оба поля сразу: внутри него компилятор видит, что
    /// поля разные.
    pub fn split(&mut self) -> (&[u8], &mut [u8; 16]) {
        (&self.rx[..self.rx_len], &mut self.tx)
    }
}

impl Default for Uart {
    fn default() -> Self {
        Self::new()
    }
}

pub fn echo_via_methods(u: &mut Uart) {
    let (rx, tx) = u.split();
    tx[..rx.len()].copy_from_slice(rx);
    u.tx_len = u.rx_len;
}
// ANCHOR_END: methods

// ANCHOR: halves
/// Двойной буфер DMA: пока контроллер пишет в одну половину, программа
/// обрабатывает другую. Возвращает сумму готовой половины и обнуляет её.
pub fn drain_ready_half(buf: &mut [u8], dma_in_second: bool) -> u32 {
    let mid = buf.len() / 2;
    let (first, second) = buf.split_at_mut(mid);
    let (ready, _busy) = if dma_in_second { (first, second) } else { (second, first) };
    let mut sum = 0;
    for b in ready.iter_mut() {
        sum += *b as u32;
        *b = 0;
    }
    sum
}
// ANCHOR_END: halves

// ANCHOR: last
/// Контрольная сумма в последнем байте кадра: тело читается, пока последний
/// байт пишется.
pub fn seal(frame: &mut [u8]) {
    if let Some((crc, body)) = frame.split_last_mut() {
        *crc = body.iter().fold(0, |acc, b| acc ^ b);
    }
}
// ANCHOR_END: last

// ANCHOR: chunks
/// Слова в big-endian из провода — в порядок байтов процессора, на месте.
/// Возвращает число байт в неполном хвосте.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch04_words_from_be(buf: &mut [u8]) -> usize {
    let (words, tail) = buf.as_chunks_mut::<4>();
    for w in words {
        *w = u32::from_be_bytes(*w).to_ne_bytes();
    }
    tail.len()
}

/// То же через `chunks_exact_mut`: куски — срезы, длина 4 известна только
/// во время выполнения.
pub fn words_from_be_chunks(buf: &mut [u8]) -> usize {
    let mut chunks = buf.chunks_exact_mut(4);
    for w in &mut chunks {
        let v = u32::from_be_bytes([w[0], w[1], w[2], w[3]]);
        w.copy_from_slice(&v.to_ne_bytes());
    }
    chunks.into_remainder().len()
}
// ANCHOR_END: chunks

// ANCHOR: disjoint
/// Два элемента по индексам сразу: `get_disjoint_mut` проверяет, что
/// индексы разные и в границах.
pub fn swap_channels(v: &mut [u16], i: usize, j: usize) -> bool {
    match v.get_disjoint_mut([i, j]) {
        Ok([a, b]) => {
            core::mem::swap(a, b);
            true
        }
        Err(_) => false,
    }
}
// ANCHOR_END: disjoint

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_and_methods() {
        let mut u = Uart::new();
        u.rx[..3].copy_from_slice(b"abc");
        u.rx_len = 3;
        echo(&mut u);
        assert_eq!(&u.tx[..u.tx_len], b"abc");
        let mut u2 = Uart::new();
        u2.rx[..2].copy_from_slice(b"hi");
        u2.rx_len = 2;
        echo_via_methods(&mut u2);
        assert_eq!(&u2.tx[..u2.tx_len], b"hi");
    }

    #[test]
    fn halves_and_last() {
        let mut buf = [1u8, 2, 3, 4, 5, 6];
        assert_eq!(drain_ready_half(&mut buf, true), 6);
        assert_eq!(buf, [0, 0, 0, 4, 5, 6]);
        assert_eq!(drain_ready_half(&mut buf, false), 15);
        assert_eq!(buf, [0; 6]);
        let mut f = [0x12, 0x34, 0x56, 0];
        seal(&mut f);
        assert_eq!(f[3], 0x12 ^ 0x34 ^ 0x56);
        seal(&mut []);
    }

    #[test]
    fn chunks() {
        let mut a = [0x12, 0x34, 0x56, 0x78, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE];
        let mut b = a;
        assert_eq!(ch04_words_from_be(&mut a), 1);
        assert_eq!(words_from_be_chunks(&mut b), 1);
        assert_eq!(a, b);
        assert_eq!(u32::from_ne_bytes([a[0], a[1], a[2], a[3]]), 0x1234_5678);
        assert_eq!(a[8], 0xEE);
    }

    #[test]
    fn disjoint() {
        let mut v = [10, 20, 30];
        assert!(swap_channels(&mut v, 0, 2));
        assert_eq!(v, [30, 20, 10]);
        assert!(!swap_channels(&mut v, 1, 1));
        assert!(!swap_channels(&mut v, 1, 3));
    }
}
