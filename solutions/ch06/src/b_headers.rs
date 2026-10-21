//! Задача 6.B: разбор заголовков IPv4 и TCP.
//!
//! Часть 1 разбирает байты без unsafe: каждое многобайтовое поле читается
//! явным `from_be_bytes`, битовые поля выделяются сдвигами и масками.
//! Часть 2 накладывает `repr(C)`-структуру прямо на буфер и показывает,
//! почему без проверки выравнивания этого делать нельзя.

use core::mem::{align_of, size_of};

// ANCHOR: error
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeaderError {
    /// Буфер короче, чем требует заголовок или поле длины.
    Truncated { need: usize, have: usize },
    /// Поле Version не равно 4.
    Version(u8),
    /// IHL меньше 5: заголовок IPv4 не бывает короче 20 байт.
    HeaderLen(u8),
    /// Total Length меньше длины заголовка.
    TotalLen(u16),
    /// Контрольная сумма заголовка не сошлась.
    Checksum { stored: u16, computed: u16 },
    /// Data Offset TCP меньше 5 или выходит за пределы сегмента.
    DataOffset(u8),
}
// ANCHOR_END: error

// ANCHOR: ipv4
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Ipv4<'a> {
    pub dscp: u8,
    pub ecn: u8,
    pub total_len: u16,
    pub ident: u16,
    pub dont_fragment: bool,
    pub more_fragments: bool,
    /// Смещение фрагмента в 8-байтовых блоках.
    pub frag_offset: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src: [u8; 4],
    pub dst: [u8; 4],
    pub options: &'a [u8],
    pub payload: &'a [u8],
}
// ANCHOR_END: ipv4

// ANCHOR: checksum
/// Сумма 16-битных слов в сетевом порядке. Нечётный хвост дополняется нулём.
/// Аккумулятор u64 не переполнится ни на каком реальном буфере.
fn sum_words(data: &[u8], mut acc: u64) -> u64 {
    let mut words = data.chunks_exact(2);
    for w in &mut words {
        acc += u16::from_be_bytes([w[0], w[1]]) as u64;
    }
    if let [last] = words.remainder() {
        acc += (*last as u64) << 8;
    }
    acc
}

/// Свёртка переносов и инверсия: сложение в обратном коде по RFC 1071.
fn fold(mut acc: u64) -> u16 {
    while acc > 0xFFFF {
        acc = (acc & 0xFFFF) + (acc >> 16);
    }
    !(acc as u16)
}

/// Контрольная сумма Интернета. Для заголовка с верным полем суммы
/// результат равен нулю.
pub fn internet_checksum(data: &[u8]) -> u16 {
    fold(sum_words(data, 0))
}
// ANCHOR_END: checksum

// ANCHOR: parse_ipv4
pub fn parse_ipv4(packet: &[u8]) -> Result<Ipv4<'_>, HeaderError> {
    let have = packet.len();
    let fixed: &[u8; 20] = packet.first_chunk().ok_or(HeaderError::Truncated { need: 20, have })?;

    let version = fixed[0] >> 4;
    if version != 4 {
        return Err(HeaderError::Version(version));
    }
    let ihl = fixed[0] & 0x0F;
    if ihl < 5 {
        return Err(HeaderError::HeaderLen(ihl));
    }
    let header_len = ihl as usize * 4;
    if have < header_len {
        return Err(HeaderError::Truncated { need: header_len, have });
    }
    let total_len = u16::from_be_bytes([fixed[2], fixed[3]]);
    if (total_len as usize) < header_len {
        return Err(HeaderError::TotalLen(total_len));
    }
    if have < total_len as usize {
        return Err(HeaderError::Truncated { need: total_len as usize, have });
    }

    // Сумму считаем без поля checksum (байты 10..12), чтобы в ошибке
    // сообщить правильное значение, а не просто «не ноль».
    let stored = u16::from_be_bytes([fixed[10], fixed[11]]);
    let computed = fold(sum_words(&packet[12..header_len], sum_words(&fixed[..10], 0)));
    if computed != stored {
        return Err(HeaderError::Checksum { stored, computed });
    }

    let flags_frag = u16::from_be_bytes([fixed[6], fixed[7]]);
    Ok(Ipv4 {
        dscp: fixed[1] >> 2,
        ecn: fixed[1] & 0b11,
        total_len,
        ident: u16::from_be_bytes([fixed[4], fixed[5]]),
        dont_fragment: flags_frag & 0x4000 != 0,
        more_fragments: flags_frag & 0x2000 != 0,
        frag_offset: flags_frag & 0x1FFF,
        ttl: fixed[8],
        protocol: fixed[9],
        checksum: stored,
        src: [fixed[12], fixed[13], fixed[14], fixed[15]],
        dst: [fixed[16], fixed[17], fixed[18], fixed[19]],
        options: &packet[20..header_len],
        // Всё после total_len (например, дополнение Ethernet) не наше.
        payload: &packet[header_len..total_len as usize],
    })
}
// ANCHOR_END: parse_ipv4

// ANCHOR: tcp
/// Девять битов флагов TCP (RFC 9293 и RFC 3168).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TcpFlags(pub u16);

impl TcpFlags {
    pub const FIN: u16 = 1 << 0;
    pub const SYN: u16 = 1 << 1;
    pub const RST: u16 = 1 << 2;
    pub const PSH: u16 = 1 << 3;
    pub const ACK: u16 = 1 << 4;
    pub const URG: u16 = 1 << 5;
    pub const ECE: u16 = 1 << 6;
    pub const CWR: u16 = 1 << 7;
    /// Бит 8: бывший NS из RFC 3540, который RFC 8311 перевёл в исторические.
    pub const NS: u16 = 1 << 8;

    pub const fn contains(self, mask: u16) -> bool {
        self.0 & mask == mask
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Tcp<'a> {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq: u32,
    pub ack: u32,
    pub flags: TcpFlags,
    pub window: u16,
    pub checksum: u16,
    pub urgent: u16,
    pub options: &'a [u8],
    pub payload: &'a [u8],
}
// ANCHOR_END: tcp

// ANCHOR: parse_tcp
pub fn parse_tcp(segment: &[u8]) -> Result<Tcp<'_>, HeaderError> {
    let have = segment.len();
    let fixed: &[u8; 20] =
        segment.first_chunk().ok_or(HeaderError::Truncated { need: 20, have })?;
    let be16 = |i: usize| u16::from_be_bytes([fixed[i], fixed[i + 1]]);
    let be32 = |i: usize| u32::from_be_bytes([fixed[i], fixed[i + 1], fixed[i + 2], fixed[i + 3]]);

    // Слово 12..14: data offset (4 бита) | 3 зарезервированных бита | 9 флагов.
    let off_flags = be16(12);
    let data_offset = (off_flags >> 12) as u8;
    let header_len = data_offset as usize * 4;
    if data_offset < 5 {
        return Err(HeaderError::DataOffset(data_offset));
    }
    if have < header_len {
        return Err(HeaderError::Truncated { need: header_len, have });
    }
    Ok(Tcp {
        src_port: be16(0),
        dst_port: be16(2),
        seq: be32(4),
        ack: be32(8),
        flags: TcpFlags(off_flags & 0x01FF),
        window: be16(14),
        checksum: be16(16),
        urgent: be16(18),
        options: &segment[20..header_len],
        payload: &segment[header_len..],
    })
}

/// Значение поля checksum, которое должно стоять в сегменте: сумма по
/// псевдозаголовку (адреса, протокол 6, длина) и сегменту без самого поля.
pub fn tcp_checksum(src: [u8; 4], dst: [u8; 4], segment: &[u8]) -> Result<u16, HeaderError> {
    let have = segment.len();
    if have < 20 {
        return Err(HeaderError::Truncated { need: 20, have });
    }
    let Ok(len) = u16::try_from(have) else {
        return Err(HeaderError::TotalLen(u16::MAX));
    };
    let mut acc = sum_words(&src, 0);
    acc = sum_words(&dst, acc);
    acc += 6; // ноль и номер протокола: слово 0x0006
    acc += len as u64;
    acc = sum_words(&segment[..16], acc);
    acc = sum_words(&segment[18..], acc);
    Ok(fold(acc))
}
// ANCHOR_END: parse_tcp

// ANCHOR: raw
/// Часть 2. Заголовок IPv4 как `repr(C)`-структура. Многобайтовые поля
/// хранятся в сетевом порядке и переводятся методами-аксессорами.
#[repr(C)]
#[derive(Debug)]
pub struct Ipv4Raw {
    pub ver_ihl: u8,
    pub tos: u8,
    pub total_len: u16,
    pub ident: u16,
    pub flags_frag: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src: [u8; 4],
    pub dst: [u8; 4],
}

// Ровно 20 байт без единого байта выравнивания: значит, любой набор из
// 20 байт является допустимым значением Ipv4Raw.
const _: () = assert!(size_of::<Ipv4Raw>() == 20);
const _: () = assert!(align_of::<Ipv4Raw>() == 2);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewError {
    Truncated { need: usize, have: usize },
    Misaligned { addr: usize, align: usize },
}

pub fn view_ipv4(packet: &[u8]) -> Result<&Ipv4Raw, ViewError> {
    let need = size_of::<Ipv4Raw>();
    if packet.len() < need {
        return Err(ViewError::Truncated { need, have: packet.len() });
    }
    let ptr = packet.as_ptr().cast::<Ipv4Raw>();
    if !ptr.is_aligned() {
        return Err(ViewError::Misaligned { addr: ptr as usize, align: align_of::<Ipv4Raw>() });
    }
    // SAFETY: длина и выравнивание проверены выше; все поля — целые и
    // массивы байт без ниш, выравнивающих байтов нет (см. const assert);
    // время жизни результата привязано к `packet`, а через `&` его не изменить.
    Ok(unsafe { &*ptr })
}

impl Ipv4Raw {
    pub fn version(&self) -> u8 {
        self.ver_ihl >> 4
    }
    pub fn header_len(&self) -> usize {
        (self.ver_ihl & 0x0F) as usize * 4
    }
    pub fn total_len(&self) -> u16 {
        u16::from_be(self.total_len)
    }
    pub fn ident(&self) -> u16 {
        u16::from_be(self.ident)
    }
    pub fn dont_fragment(&self) -> bool {
        u16::from_be(self.flags_frag) & 0x4000 != 0
    }
    pub fn checksum(&self) -> u16 {
        u16::from_be(self.checksum)
    }
}
// ANCHOR_END: raw

// ANCHOR: wire
/// Вне задачи: тот же заголовок из полей выравнивания 1. Его можно
/// наложить на буфер по любому адресу, и проверка выравнивания не нужна.
/// Так устроены типы `network_endian::U16` в крейте zerocopy.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Be16([u8; 2]);

impl Be16 {
    pub const fn get(self) -> u16 {
        u16::from_be_bytes(self.0)
    }
}

#[repr(C)]
#[derive(Debug)]
pub struct Ipv4Wire {
    pub ver_ihl: u8,
    pub tos: u8,
    pub total_len: Be16,
    pub ident: Be16,
    pub flags_frag: Be16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: Be16,
    pub src: [u8; 4],
    pub dst: [u8; 4],
}

const _: () = assert!(size_of::<Ipv4Wire>() == 20);
const _: () = assert!(align_of::<Ipv4Wire>() == 1);

pub fn view_ipv4_wire(packet: &[u8]) -> Option<&Ipv4Wire> {
    let bytes: &[u8; 20] = packet.first_chunk()?;
    // SAFETY: выравнивание Ipv4Wire равно 1, размер 20, все битовые шаблоны
    // допустимы, время жизни привязано к `packet`.
    Some(unsafe { &*(bytes as *const [u8; 20]).cast::<Ipv4Wire>() })
}
// ANCHOR_END: wire

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_view_at_odd_address() {
        let mut buf = [0u8; 21];
        buf[1..].copy_from_slice(&[
            0x45, 0x00, 0x00, 0x73, 0x00, 0x00, 0x40, 0x00, 0x40, 0x11, 0xb8, 0x61, 0xc0, 0xa8,
            0x00, 0x01, 0xc0, 0xa8, 0x00, 0xc7,
        ]);
        let h = view_ipv4_wire(&buf[1..]).unwrap();
        assert_eq!(h.total_len.get(), 0x73);
        assert_eq!(h.checksum.get(), 0xb861);
    }
}
