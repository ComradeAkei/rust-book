//! Задача D2: разбор регистров сбоев. CFSR (0xE000_ED28) — три регистра в
//! одном слове: MMFSR в байте 0, BFSR в байте 1, UFSR в старших 16 битах.
//! HFSR (0xE000_ED2C) — причина HardFault. Прошивка
//! `firmware/ch27-tasks`, программа `d2_faults`, вызывает три сбоя,
//! разбирает их этим кодом и продолжает работу.

// ANCHOR: fault
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
// ANCHOR_END: fault

/// Биты CFSR по порядку и их значения.
const CFSR: [(u32, Fault); 19] = [
    (0, Fault::IaccViol),
    (1, Fault::DaccViol),
    (3, Fault::MUnstkErr),
    (4, Fault::MStkErr),
    (5, Fault::MLspErr),
    (7, Fault::MmarValid),
    (8, Fault::IBusErr),
    (9, Fault::PreciseErr),
    (10, Fault::ImpreciseErr),
    (11, Fault::UnstkErr),
    (12, Fault::StkErr),
    (13, Fault::LspErr),
    (15, Fault::BfarValid),
    (16, Fault::UndefInstr),
    (17, Fault::InvState),
    (18, Fault::InvPc),
    (19, Fault::NoCp),
    (24, Fault::Unaligned),
    (25, Fault::DivByZero),
];

const HFSR: [(u32, Fault); 3] = [(1, Fault::VectTbl), (30, Fault::Forced), (31, Fault::DebugEvt)];

impl Fault {
    // ANCHOR: name
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
    // ANCHOR_END: name
}

// ANCHOR: faults
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

    fn next(&mut self) -> Option<Fault> {
        while self.at < CFSR.len() + HFSR.len() {
            let i = self.at;
            self.at += 1;
            let (reg, (bit, fault)) = match CFSR.get(i) {
                Some(&row) => (self.cfsr, row),
                None => (self.hfsr, HFSR[i - CFSR.len()]),
            };
            if reg & (1 << bit) != 0 {
                return Some(fault);
            }
        }
        None
    }
}
// ANCHOR_END: faults

// ANCHOR: skip
/// Длина инструкции Thumb по её первому полуслову: 32-битные начинаются
/// с 0b11101, 0b11110 или 0b11111 в старших пяти битах, остальные — 16.
pub const fn insn_len(first: u16) -> u32 {
    match first >> 11 {
        0b11101..=0b11111 => 4,
        _ => 2,
    }
}
// ANCHOR_END: skip
