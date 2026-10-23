//! Раздел 27.1: что выполняет процессор и куда он вернётся. Номер
//! исключения процессор держит в IPSR, адрес возврата из обработчика —
//! особое значение в LR (EXC_RETURN), а регистры прерванного кода — в
//! кадре на стеке.

// ANCHOR: exception
/// Номер исключения из IPSR (биты 0–8 xPSR): 0 — код вне обработчиков,
/// 1–15 — исключения ядра, 16 и выше — внешние прерывания NVIC.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Exception {
    Thread,
    Reset,
    Nmi,
    HardFault,
    MemManage,
    BusFault,
    UsageFault,
    SvCall,
    DebugMonitor,
    PendSv,
    SysTick,
    /// Внешнее прерывание с номером NVIC: IPSR минус 16.
    Irq(u16),
    /// Номера 7–10 и 13 зарезервированы.
    Reserved(u16),
}

impl Exception {
    pub const fn from_ipsr(ipsr: u32) -> Exception {
        match (ipsr & 0x1FF) as u16 {
            0 => Exception::Thread,
            1 => Exception::Reset,
            2 => Exception::Nmi,
            3 => Exception::HardFault,
            4 => Exception::MemManage,
            5 => Exception::BusFault,
            6 => Exception::UsageFault,
            11 => Exception::SvCall,
            12 => Exception::DebugMonitor,
            14 => Exception::PendSv,
            15 => Exception::SysTick,
            n @ 16.. => Exception::Irq(n - 16),
            n => Exception::Reserved(n),
        }
    }
}
// ANCHOR_END: exception

// ANCHOR: exc_return
/// Куда вернётся процессор, когда обработчик выполнит `bx lr` с этим
/// значением LR. Процессор кладёт его в LR сам при входе в исключение.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExcReturn {
    /// Бит 3: в режим Thread (иначе — в прерванный обработчик).
    pub to_thread: bool,
    /// Бит 2: кадр лежит на PSP (иначе — на MSP).
    pub psp: bool,
    /// Бит 4 равен нулю: в кадре ещё и регистры FPU (Cortex-M4F).
    pub fpu_frame: bool,
}

impl ExcReturn {
    /// `None` — это не EXC_RETURN: обычный адрес возврата или запрещённое
    /// сочетание битов (режим Handler всегда работает на MSP).
    pub const fn decode(lr: u32) -> Option<ExcReturn> {
        if lr & 0xFFFF_FFE3 != 0xFFFF_FFE1 {
            return None;
        }
        let r = ExcReturn {
            to_thread: lr & (1 << 3) != 0,
            psp: lr & (1 << 2) != 0,
            fpu_frame: lr & (1 << 4) == 0,
        };
        if r.psp && !r.to_thread { None } else { Some(r) }
    }
}
// ANCHOR_END: exc_return

// ANCHOR: frame
/// Кадр исключения: восемь слов, которые процессор сохраняет сам, до
/// первой инструкции обработчика. Младший адрес — `r0`.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Frame {
    pub r0: u32,
    pub r1: u32,
    pub r2: u32,
    pub r3: u32,
    pub r12: u32,
    pub lr: u32,
    pub pc: u32,
    pub xpsr: u32,
}

impl Frame {
    /// SP прерванного кода, если кадр лежит по адресу `at`: над кадром,
    /// и ещё на слово выше, если процессор выравнивал стек на 8 (бит 9
    /// сохранённого xPSR).
    pub const fn sp_before(&self, at: u32) -> u32 {
        let pad = if self.xpsr & (1 << 9) != 0 { 4 } else { 0 };
        at + core::mem::size_of::<Frame>() as u32 + pad
    }
}

/// Число из инструкции `svc #imm8` (кодировка Thumb `0xDFxx`). Сохранённый
/// PC указывает на инструкцию после `svc`, сама она — на 2 байта раньше.
pub const fn svc_number(insn: u16) -> Option<u8> {
    if insn & 0xFF00 == 0xDF00 { Some(insn as u8) } else { None }
}
// ANCHOR_END: frame

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipsr_numbers() {
        assert_eq!(Exception::from_ipsr(0), Exception::Thread);
        assert_eq!(Exception::from_ipsr(3), Exception::HardFault);
        assert_eq!(Exception::from_ipsr(11), Exception::SvCall);
        assert_eq!(Exception::from_ipsr(14), Exception::PendSv);
        assert_eq!(Exception::from_ipsr(15), Exception::SysTick);
        assert_eq!(Exception::from_ipsr(16), Exception::Irq(0));
        assert_eq!(Exception::from_ipsr(16 + 19), Exception::Irq(19));
        assert_eq!(Exception::from_ipsr(7), Exception::Reserved(7));
        // В регистре xPSR выше IPSR — флаги и бит Thumb: они не мешают.
        assert_eq!(Exception::from_ipsr(0x6100_000B), Exception::SvCall);
    }

    #[test]
    fn exc_return_values() {
        let thread_msp = ExcReturn { to_thread: true, psp: false, fpu_frame: false };
        let thread_psp = ExcReturn { to_thread: true, psp: true, fpu_frame: false };
        let handler = ExcReturn { to_thread: false, psp: false, fpu_frame: false };
        assert_eq!(ExcReturn::decode(0xFFFF_FFF9), Some(thread_msp));
        assert_eq!(ExcReturn::decode(0xFFFF_FFFD), Some(thread_psp));
        assert_eq!(ExcReturn::decode(0xFFFF_FFF1), Some(handler));
        assert_eq!(
            ExcReturn::decode(0xFFFF_FFED),
            Some(ExcReturn { fpu_frame: true, ..thread_psp })
        );
        // Handler на PSP не бывает; обычный адрес возврата — не EXC_RETURN.
        assert_eq!(ExcReturn::decode(0xFFFF_FFF5), None);
        assert_eq!(ExcReturn::decode(0x0000_0E85), None);
        assert_eq!(ExcReturn::decode(0xFFFF_FFFB), None);
    }

    #[test]
    fn frame_layout() {
        assert_eq!(core::mem::size_of::<Frame>(), 32);
        assert_eq!(core::mem::offset_of!(Frame, pc), 24);
        assert_eq!(core::mem::offset_of!(Frame, xpsr), 28);
        let mut f = Frame { r0: 0, r1: 0, r2: 0, r3: 0, r12: 0, lr: 0, pc: 0, xpsr: 0x0100_0000 };
        assert_eq!(f.sp_before(0x2000_3F48), 0x2000_3F68);
        f.xpsr |= 1 << 9;
        assert_eq!(f.sp_before(0x2000_3F48), 0x2000_3F6C);
    }

    #[test]
    fn svc_immediate() {
        assert_eq!(svc_number(0xDF07), Some(7));
        assert_eq!(svc_number(0xDF00), Some(0));
        assert_eq!(svc_number(0xBF30), None); // wfi
    }
}
