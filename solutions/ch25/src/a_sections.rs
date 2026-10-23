//! Задача A: секции шести объявлений из
//! `code/ch25/fragments/a_sections.rs` под thumbv7m-none-eabi.

/// Секция объектного файла.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Неизменяемые данные: flash, ОЗУ не занимают.
    Rodata,
    /// Изменяемые с ненулевым начальным значением: ОЗУ и образ во flash.
    Data,
    /// Изменяемые, начинаются с нуля: только ОЗУ, обнуляет код запуска.
    Bss,
    /// Ответа ещё нет.
    Unknown,
}

// ANCHOR: answers
pub const ANSWERS: [(&str, Section); 6] = [
    // Неизменяемый static без внутренней изменяемости.
    ("GAINS", Section::Rodata),
    // Тоже неизменяемый: нули не повод переезжать в .bss — .bss
    // записываемая, а этот объект писать нельзя.
    ("ZEROS", Section::Rodata),
    // Атомик меняется через общую ссылку, начальное значение 1.
    ("BOOTS", Section::Data),
    // Атомик с нулём.
    ("EVENTS", Section::Bss),
    // static mut с нулями.
    ("SCRATCH", Section::Bss),
    // static mut с ненулевыми значениями.
    ("CALIB", Section::Data),
];
// ANCHOR_END: answers
