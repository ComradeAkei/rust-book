//! Раздел 3.5: перемещение в машинном коде.
//!
//! Модуль без `std`: те же функции глава разбирает для x86_64 и для
//! Cortex-M. Прошивка `firmware/ch03-stack` меряет, сколько стека они
//! занимают на Cortex-M3.

// ANCHOR: frame
/// Кадр телеметрии: 64 байта, не `Copy`.
pub struct Frame {
    pub seq: u32,
    pub data: [u8; 60],
}

/// Простая контрольная сумма: сумма байтов по модулю 2^32.
pub fn sum(data: &[u8]) -> u32 {
    data.iter().map(|&b| b as u32).sum()
}
// ANCHOR_END: frame

// ANCHOR: seal
/// Запечатывает кадр: кладёт контрольную сумму в последний байт. Функция
/// меняет свой параметр, поэтому ей нужна собственная копия кадра.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_seal(mut f: Frame) -> u32 {
    f.data[59] = sum(&f.data[..59]) as u8;
    f.seq ^ sum(&f.data)
}
// ANCHOR_END: seal

// ANCHOR: move_local
/// Кадр собран здесь и перемещён дважды: в `g`, затем в `ch03_seal`.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_build_and_seal(seq: u32) -> u32 {
    let f = Frame { seq, data: [seq as u8; 60] };
    let g = f;
    ch03_seal(g)
}
// ANCHOR_END: move_local

// ANCHOR: copy_from_ref
/// Кадр чужой, вызывающему его только показали. Чтобы отдать
/// `ch03_seal` значение, нужна копия.
#[inline(never)]
#[unsafe(no_mangle)]
pub fn ch03_seal_copy(f: &Frame) -> u32 {
    ch03_seal(Frame { seq: f.seq, data: f.data })
}
// ANCHOR_END: copy_from_ref

// ANCHOR: builder
/// Блок журнала для записи во флеш: 1 КиБ, не `Copy`.
pub struct Block {
    pub header: [u8; 16],
    pub payload: [u8; 1004],
    pub crc: u32,
}

impl Block {
    pub const fn empty() -> Self {
        Block { header: [0; 16], payload: [0; 1004], crc: 0 }
    }
}

/// Сборка блока «по значению»: каждый шаг принимает блок и возвращает его
/// же. Читается удобно, но каждый шаг может стоить копии на стеке.
#[inline(never)]
pub fn with_header(mut b: Block, seq: u32) -> Block {
    b.header[..4].copy_from_slice(&seq.to_le_bytes());
    b
}

#[inline(never)]
pub fn with_payload(mut b: Block, byte: u8) -> Block {
    b.payload.fill(byte);
    b
}

#[inline(never)]
pub fn with_crc(mut b: Block) -> Block {
    b.crc = sum(&b.header) ^ sum(&b.payload);
    b
}

#[inline(never)]
pub fn build_by_value(seq: u32, byte: u8) -> Block {
    with_crc(with_payload(with_header(Block::empty(), seq), byte))
}

/// Та же сборка через `&mut`: блок не двигается с места.
#[inline(never)]
pub fn set_header(b: &mut Block, seq: u32) {
    b.header[..4].copy_from_slice(&seq.to_le_bytes());
}

#[inline(never)]
pub fn set_payload(b: &mut Block, byte: u8) {
    b.payload.fill(byte);
}

#[inline(never)]
pub fn set_crc(b: &mut Block) {
    b.crc = sum(&b.header) ^ sum(&b.payload);
}

#[inline(never)]
pub fn build_in_place(b: &mut Block, seq: u32, byte: u8) {
    set_header(b, seq);
    set_payload(b, byte);
    set_crc(b);
}
// ANCHOR_END: builder

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seal_variants_agree() {
        let f = Frame { seq: 9, data: [9; 60] };
        assert_eq!(ch03_seal_copy(&f), ch03_build_and_seal(9));
        assert_eq!(size_of::<Frame>(), 64);
    }

    #[test]
    fn block_builders_agree() {
        let a = build_by_value(7, 0xA5);
        let mut b = Block::empty();
        build_in_place(&mut b, 7, 0xA5);
        assert_eq!((a.header, a.payload, a.crc), (b.header, b.payload, b.crc));
        assert_eq!(size_of::<Block>(), 1024);
    }
}
