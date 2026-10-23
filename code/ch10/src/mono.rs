//! Раздел 10.5: мономорфизация и её цена.

// ANCHOR: checksum
/// Сумма по модулю 2³² значений любого типа, который расширяется до `u32`.
/// `#[inline(never)]` оставляет каждую копию отдельной функцией, чтобы её
/// было видно в `nm`.
#[inline(never)]
pub fn checksum<T: Copy + Into<u32>>(data: &[T]) -> u32 {
    data.iter().fold(0u32, |s, &x| s.wrapping_add(x.into()))
}

#[unsafe(no_mangle)]
pub fn ch10_sum_u8(d: &[u8]) -> u32 {
    checksum(d)
}

#[unsafe(no_mangle)]
pub fn ch10_sum_u16(d: &[u16]) -> u32 {
    checksum(d)
}

#[unsafe(no_mangle)]
pub fn ch10_sum_u32(d: &[u32]) -> u32 {
    checksum(d)
}

/// `char` и `u32` устроены одинаково, и `char → u32` ничего не делает.
#[unsafe(no_mangle)]
pub fn ch10_sum_char(d: &[char]) -> u32 {
    checksum(d)
}

/// Второй вызов с `u8` новой копии не создаёт.
#[unsafe(no_mangle)]
pub fn ch10_sum_u8_again(d: &[u8]) -> u32 {
    checksum(d).rotate_left(1)
}
// ANCHOR_END: checksum

// ANCHOR: slip
/// Конец кадра и экранирующий байт протокола SLIP (RFC 1055).
pub const END: u8 = 0xC0;
pub const ESC: u8 = 0xDB;

/// Записать кадр SLIP: байты `END` и `ESC` внутри данных экранируются, в
/// конце ставится `END`. Всё тело обобщено по `D`, и каждый тип данных
/// получает свою копию целиком.
pub fn slip_fat<D: AsRef<[u8]>>(data: D, out: &mut [u8]) -> Option<usize> {
    let mut n = 0;
    for &b in data.as_ref() {
        let pair = [ESC, if b == END { 0xDC } else { 0xDD }];
        let src: &[u8] = if b == END || b == ESC { &pair } else { core::slice::from_ref(&b) };
        out.get_mut(n..n + src.len())?.copy_from_slice(src);
        n += src.len();
    }
    *out.get_mut(n)? = END;
    Some(n + 1)
}

/// То же с внутренней необобщённой функцией: обобщённая обёртка только
/// вызывает `as_ref`, а тело существует в одном экземпляре.
pub fn slip_thin<D: AsRef<[u8]>>(data: D, out: &mut [u8]) -> Option<usize> {
    fn inner(data: &[u8], out: &mut [u8]) -> Option<usize> {
        let mut n = 0;
        for &b in data {
            let pair = [ESC, if b == END { 0xDC } else { 0xDD }];
            let src: &[u8] = if b == END || b == ESC { &pair } else { core::slice::from_ref(&b) };
            out.get_mut(n..n + src.len())?.copy_from_slice(src);
            n += src.len();
        }
        *out.get_mut(n)? = END;
        Some(n + 1)
    }
    inner(data.as_ref(), out)
}
// ANCHOR_END: slip

/// Полезная нагрузка кадра телеметрии со своей реализацией `AsRef<[u8]>`.
pub struct Payload(pub [u8; 6]);

impl AsRef<[u8]> for Payload {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// ANCHOR: slip-users
/// Четыре вызывающих с четырьмя разными типами данных.
#[unsafe(no_mangle)]
pub fn ch10_slip_fat(out: &mut [u8], s: &[u8], a: [u8; 4], t: &str, p: &Payload) -> usize {
    let mut n = slip_fat(s, out).unwrap_or(0);
    n += slip_fat(a, out).unwrap_or(0);
    n += slip_fat(t, out).unwrap_or(0);
    n + slip_fat(p, out).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub fn ch10_slip_thin(out: &mut [u8], s: &[u8], a: [u8; 4], t: &str, p: &Payload) -> usize {
    let mut n = slip_thin(s, out).unwrap_or(0);
    n += slip_thin(a, out).unwrap_or(0);
    n += slip_thin(t, out).unwrap_or(0);
    n + slip_thin(p, out).unwrap_or(0)
}
// ANCHOR_END: slip-users

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums() {
        assert_eq!(ch10_sum_u8(&[1, 2, 255]), 258);
        assert_eq!(ch10_sum_u16(&[u16::MAX, 1]), 65536);
        assert_eq!(ch10_sum_u32(&[u32::MAX, 2]), 1);
        assert_eq!(ch10_sum_char(&['А', 'Я']), 0x410 + 0x42F);
        assert_eq!(ch10_sum_u8_again(&[1]), 2);
    }

    #[test]
    fn slip() {
        let mut out = [0u8; 16];
        assert_eq!(slip_fat([1, END, 2, ESC], &mut out), Some(7));
        assert_eq!(out[..7], [1, ESC, 0xDC, 2, ESC, 0xDD, END]);
        let mut out2 = [0u8; 16];
        assert_eq!(slip_thin(&[1, END, 2, ESC][..], &mut out2), Some(7));
        assert_eq!(out, out2);
        assert_eq!(slip_thin("ok", &mut out), Some(3));
        assert_eq!(slip_fat(Payload([END; 6]), &mut out), Some(13));
        assert_eq!(slip_fat(Payload([END; 6]), &mut out[..12]), None);
        assert_eq!(slip_thin([0u8; 15], &mut out), Some(16));
        let p = Payload([1, 2, 3, 4, 5, 6]);
        let mut big = [0u8; 64];
        let a = ch10_slip_fat(&mut big, &[END], [1; 4], "ab", &p);
        let b = ch10_slip_thin(&mut big, &[END], [1; 4], "ab", &p);
        assert_eq!((a, b), (3 + 5 + 3 + 7, 3 + 5 + 3 + 7));
    }
}
