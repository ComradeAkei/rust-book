//! UART0 на выводах PA0 (приём) и PA1 (передача). Чтение и запись —
//! блокирующие, трейты `embedded_io::Read` и `Write`: в embedded-hal 1.0
//! последовательного порта нет, он переехал в embedded-io.

use crate::gpio::{Alternate, PA0, PA1};
use crate::pac;
use crate::sysctl::{CLOCK_HZ, Gate, Sysctl};
use ch25::const_fn::{Divisor, uart_divisor};
use core::convert::Infallible;

// ANCHOR: baud
/// Делитель для скорости `baud` при частоте ядра: функция раздела 25.2.
/// Вызванная в `const`, она проверяет ошибку скорости при сборке.
pub const fn baud(rate: u32) -> Divisor {
    uart_divisor(CLOCK_HZ, rate)
}
// ANCHOR_END: baud

/// События, по которым UART может просить прерывание.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// Пришёл байт или приёмник молчит дольше 32 бит при непустом FIFO.
    Rx,
    /// В FIFO передатчика освободилось место.
    Tx,
}

pub struct Serial {
    uart: pac::Uart0,
    pins: (PA0<Alternate>, PA1<Alternate>),
}

impl Serial {
    // ANCHOR: new
    /// Владеет и блоком UART0, и обоими выводами: пока `Serial` жив,
    /// перевести PA0 или PA1 в другой режим нечем.
    pub fn new(
        uart: pac::Uart0,
        pins: (PA0<Alternate>, PA1<Alternate>),
        div: Divisor,
        sc: &mut Sysctl,
    ) -> Serial {
        sc.enable(Gate::Uart(0));
        // Настраивать PL011 можно только выключенным (раздел 10.7).
        uart.ctl().reset();
        uart.ibrd().write(|w| w.divint().set(div.ibrd));
        uart.fbrd().write(|w| w.divfrac().set(div.fbrd));
        // 8 бит, без чётности, один стоп-бит, FIFO включены. LCRH пишется
        // после делителей: так PL011 их защёлкивает.
        uart.lcrh().write(|w| w.wlen().set(3).fen().set_bit());
        uart.ctl().write(|w| w.uarten().set_bit().txe().set_bit().rxe().set_bit());
        Serial { uart, pins }
    }
    // ANCHOR_END: new

    pub fn listen(&mut self, event: Event) {
        match event {
            Event::Rx => self.uart.im().modify(|_, w| w.rxim().set_bit().rtim().set_bit()),
            Event::Tx => self.uart.im().modify(|_, w| w.txim().set_bit()),
        };
    }

    pub fn unlisten(&mut self, event: Event) {
        match event {
            Event::Rx => self.uart.im().modify(|_, w| w.rxim().clear_bit().rtim().clear_bit()),
            Event::Tx => self.uart.im().modify(|_, w| w.txim().clear_bit()),
        };
    }

    /// Вернуть блок и выводы: так их можно перенастроить.
    pub fn free(self) -> (pac::Uart0, (PA0<Alternate>, PA1<Alternate>)) {
        self.uart.ctl().reset();
        (self.uart, self.pins)
    }
}

impl embedded_io::ErrorType for Serial {
    type Error = Infallible;
}

// ANCHOR: io
// Методы не обобщённые: без `#[inline]` они не встраиваются в код другого
// крейта без LTO, и запись одного байта — вызов (задача 27.B).
impl embedded_io::Write for Serial {
    /// Ждёт места хотя бы для одного байта, затем пишет, пока FIFO
    /// передатчика не заполнится. Возвращает, сколько записано: трейт
    /// разрешает записать меньше, чем просили, но не ноль.
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Infallible> {
        let Some((&first, rest)) = buf.split_first() else { return Ok(0) };
        while self.uart.fr().read().txff().bit_is_set() {}
        self.uart.dr().write(|w| w.data().set(first));
        let mut n = 1;
        for &b in rest {
            if self.uart.fr().read().txff().bit_is_set() {
                break;
            }
            self.uart.dr().write(|w| w.data().set(b));
            n += 1;
        }
        Ok(n)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Infallible> {
        while self.uart.fr().read().busy().bit_is_set() {}
        Ok(())
    }
}

impl embedded_io::Read for Serial {
    /// Ждёт хотя бы один байт и забирает всё, что уже есть в FIFO.
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Infallible> {
        if buf.is_empty() {
            return Ok(0);
        }
        while self.uart.fr().read().rxfe().bit_is_set() {}
        let mut n = 0;
        while n < buf.len() && self.uart.fr().read().rxfe().bit_is_clear() {
            buf[n] = self.uart.dr().read().data().bits();
            n += 1;
        }
        Ok(n)
    }
}
// ANCHOR_END: io

impl core::fmt::Write for Serial {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        embedded_io::Write::write_all(self, s.as_bytes()).map_err(|e| match e {})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divisor_at_12_mhz() {
        const D: Divisor = baud(115_200);
        assert_eq!((D.ibrd, D.fbrd), (6, 33));
        assert_eq!(D.error_ppm, -799);
    }
}
