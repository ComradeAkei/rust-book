//! Таймер общего назначения (GPTM) в 32-битном периодическом режиме.
//! Четыре таймера — один тип регистров по разным адресам, поэтому
//! `Timer` обобщён по адресу так же, как вывод порта.

use crate::pac;
use crate::sysctl::{Gate, Sysctl};

/// Блок регистров таймера по адресу `A`: так PAC называет и `Timer0`, и
/// `Timer3`.
pub type Regs<const A: usize> = pac::generic::Periph<pac::timer0::RegisterBlock, A>;

pub struct Timer<const A: usize> {
    tim: Regs<A>,
}

impl<const A: usize> Timer<A> {
    /// Номер таймера по адресу: 0x4003_0000, 0x4003_1000, …
    const INDEX: u8 = {
        assert!(A >= 0x4003_0000 && A <= 0x4003_3000 && A % 0x1000 == 0, "это не таймер");
        ((A - 0x4003_0000) / 0x1000) as u8
    };

    // ANCHOR: periodic
    /// Период — `ticks` тактов ядра. Таймер настроен, но не запущен.
    pub fn periodic(tim: Regs<A>, ticks: u32, sc: &mut Sysctl) -> Timer<A> {
        sc.enable(Gate::Timer(Self::INDEX));
        tim.ctl().reset();
        tim.cfg().write(|w| w.gptmcfg().bits32());
        tim.tamr().write(|w| w.tamr().periodic());
        tim.tailr().write(|w| w.tailr().set(ticks - 1));
        Timer { tim }
    }
    // ANCHOR_END: periodic

    /// Однократный режим: досчитав `ticks` тактов, таймер
    /// останавливается.
    pub fn one_shot(tim: Regs<A>, ticks: u32, sc: &mut Sysctl) -> Timer<A> {
        sc.enable(Gate::Timer(Self::INDEX));
        tim.ctl().reset();
        tim.cfg().write(|w| w.gptmcfg().bits32());
        tim.tamr().write(|w| w.tamr().one_shot());
        tim.tailr().write(|w| w.tailr().set(ticks - 1));
        Timer { tim }
    }

    pub fn start(&mut self) {
        self.tim.ctl().modify(|_, w| w.taen().set_bit());
    }

    pub fn stop(&mut self) {
        self.tim.ctl().modify(|_, w| w.taen().clear_bit());
    }

    /// Просить прерывание TIMERnA, когда таймер досчитал.
    pub fn listen(&mut self) {
        self.tim.imr().modify(|_, w| w.tatoim().set_bit());
    }

    /// Запускать АЦП каждым периодом (бит TAOTE).
    pub fn trigger_adc(&mut self) {
        self.tim.ctl().modify(|_, w| w.taote().set_bit());
    }

    // ANCHOR: clear
    /// Сбросить запрос «досчитал». Обработчик прерывания обязан это
    /// сделать: запрос держится, пока его не сбросят, и NVIC снова и
    /// снова входит в обработчик (раздел 27.6).
    pub fn clear(&mut self) {
        self.tim.icr().write(|w| w.tatocint().set_bit());
    }
    // ANCHOR_END: clear

    /// Досчитал ли таймер с прошлого вызова; запрос сбрасывается.
    pub fn expired(&mut self) -> bool {
        let hit = self.tim.ris().read().tatoris().bit_is_set();
        if hit {
            self.clear();
        }
        hit
    }

    pub fn free(self) -> Regs<A> {
        self.tim.ctl().reset();
        self.tim
    }
}
