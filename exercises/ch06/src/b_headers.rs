//! Задача 6.B. Разбор заголовков IPv4 (RFC 791) и TCP (RFC 9293).
//!
//! Часть 1, без unsafe: реализуйте `internet_checksum`, `parse_ipv4`,
//! `parse_tcp` и `tcp_checksum`. Все многобайтовые поля в сети идут в
//! порядке big-endian. Возвращаемые срезы `options` и `payload` должны
//! указывать во входной буфер: копировать данные нельзя.
//!
//! Часть 2: реализуйте `view_ipv4`, которая накладывает `Ipv4Raw` прямо на
//! буфер. Здесь понадобится один блок unsafe; напишите к нему комментарий
//! `// SAFETY:` с перечнем всех условий, которые вы проверили. Запустите
//! тесты под Miri: `cargo +nightly miri test -p ch06-exercises task_b`.

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
    /// Контрольная сумма заголовка не сошлась. `computed` — значение,
    /// которое должно было стоять в поле.
    Checksum { stored: u16, computed: u16 },
    /// Data Offset TCP меньше 5.
    DataOffset(u8),
}

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
    /// Данные до `total_len`; байты после него (дополнение Ethernet) не входят.
    pub payload: &'a [u8],
}

/// Контрольная сумма Интернета (RFC 1071): сумма 16-битных слов в обратном
/// коде с инверсией. Нечётный последний байт дополняется нулём справа.
pub fn internet_checksum(data: &[u8]) -> u16 {
    todo!()
}

pub fn parse_ipv4(packet: &[u8]) -> Result<Ipv4<'_>, HeaderError> {
    todo!()
}

/// Девять битов флагов TCP.
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
    pub const NS: u16 = 1 << 8;

    /// Установлены ли все биты маски.
    pub const fn contains(self, mask: u16) -> bool {
        todo!()
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

pub fn parse_tcp(segment: &[u8]) -> Result<Tcp<'_>, HeaderError> {
    todo!()
}

/// Значение, которое должно стоять в поле checksum сегмента: сумма по
/// псевдозаголовку (src, dst, ноль, протокол 6, длина сегмента) и по
/// сегменту без самого поля checksum (байты 16..18).
pub fn tcp_checksum(src: [u8; 4], dst: [u8; 4], segment: &[u8]) -> Result<u16, HeaderError> {
    todo!()
}

/// Часть 2. Заголовок IPv4 в раскладке C. Многобайтовые поля хранятся в
/// сетевом порядке, аксессоры переводят их в порядок процессора.
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

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewError {
    Truncated { need: usize, have: usize },
    Misaligned { addr: usize, align: usize },
}

/// Вернуть ссылку на заголовок внутри `packet` без копирования.
/// Перед этим добавьте `const _: () = assert!(...)` на размер и выравнивание
/// `Ipv4Raw` и объясните, зачем каждая проверка нужна для корректности.
pub fn view_ipv4(packet: &[u8]) -> Result<&Ipv4Raw, ViewError> {
    todo!()
}

impl Ipv4Raw {
    pub fn version(&self) -> u8 {
        todo!()
    }
    pub fn header_len(&self) -> usize {
        todo!()
    }
    pub fn total_len(&self) -> u16 {
        todo!()
    }
    pub fn ident(&self) -> u16 {
        todo!()
    }
    pub fn dont_fragment(&self) -> bool {
        todo!()
    }
    pub fn checksum(&self) -> u16 {
        todo!()
    }
}
