//! Задача D2: разбор регистров сбоев. CFSR (0xE000_ED28) — три регистра в
//! одном слове: MMFSR в байте 0, BFSR в байте 1, UFSR в старших 16 битах.
//! HFSR (0xE000_ED2C) — причина HardFault. Прошивка
//! `firmware/ch27-tasks`, программа `d2_faults`, вызывает три сбоя,
//! разбирает их этим кодом и продолжает работу.

/// Бит CFSR или HFSR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    IaccViol,
    DaccViol,
    MUnstkErr,
    MStkErr,
    MLspErr,
    MmarValid,
    IBusErr,
    PreciseErr,
    ImpreciseErr,
    UnstkErr,
    StkErr,
    LspErr,
    BfarValid,
    UndefInstr,
    InvState,
    InvPc,
    NoCp,
    Unaligned,
    DivByZero,
    VectTbl,
    Forced,
    DebugEvt,
}

impl Fault {
    /// Имя в виде «регистр.бит», как в документации ARM.
    pub const fn name(self) -> &'static str {
        match self {
            Fault::IaccViol => "MMFSR.IACCVIOL",
            Fault::DaccViol => "MMFSR.DACCVIOL",
            Fault::MUnstkErr => "MMFSR.MUNSTKERR",
            Fault::MStkErr => "MMFSR.MSTKERR",
            Fault::MLspErr => "MMFSR.MLSPERR",
            Fault::MmarValid => "MMFSR.MMARVALID",
            Fault::IBusErr => "BFSR.IBUSERR",
            Fault::PreciseErr => "BFSR.PRECISERR",
            Fault::ImpreciseErr => "BFSR.IMPRECISERR",
            Fault::UnstkErr => "BFSR.UNSTKERR",
            Fault::StkErr => "BFSR.STKERR",
            Fault::LspErr => "BFSR.LSPERR",
            Fault::BfarValid => "BFSR.BFARVALID",
            Fault::UndefInstr => "UFSR.UNDEFINSTR",
            Fault::InvState => "UFSR.INVSTATE",
            Fault::InvPc => "UFSR.INVPC",
            Fault::NoCp => "UFSR.NOCP",
            Fault::Unaligned => "UFSR.UNALIGNED",
            Fault::DivByZero => "UFSR.DIVBYZERO",
            Fault::VectTbl => "HFSR.VECTTBL",
            Fault::Forced => "HFSR.FORCED",
            Fault::DebugEvt => "HFSR.DEBUGEVT",
        }
    }
}

/// Установленные биты: сначала CFSR от младшего к старшему, затем HFSR.
/// Зарезервированные биты пропускаются.
pub fn faults(cfsr: u32, hfsr: u32) -> Faults {
    Faults { cfsr, hfsr, at: 0 }
}

/// Итератор по причинам сбоя без кучи: номер следующей строки таблиц.
pub struct Faults {
    cfsr: u32,
    hfsr: u32,
    at: usize,
}

impl Iterator for Faults {
    type Item = Fault;

    /// Номера битов — в ARMv7-M Architecture Reference Manual, раздел
    /// B3.2.15 (CFSR) и B3.2.16 (HFSR).
    fn next(&mut self) -> Option<Fault> {
        let _ = (self.cfsr, self.hfsr, &mut self.at);
        todo!("задача D2: next")
    }
}

/// Длина инструкции Thumb по её первому полуслову: 32-битные начинаются
/// с 0b11101, 0b11110 или 0b11111 в старших пяти битах, остальные — 16.
pub const fn insn_len(first: u16) -> u32 {
    let _ = first;
    // `todo!` с текстом в const fn нельзя, `panic!` с литералом — можно.
    panic!("задача D2: insn_len")
}
