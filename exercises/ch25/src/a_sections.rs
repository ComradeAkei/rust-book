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
// TODO(задача A): замените Unknown своими ответами. Сначала ответьте, потом
// запускайте тест: он собирает фрагмент и сверяет секции.
pub const ANSWERS: [(&str, Section); 6] = [
    ("GAINS", Section::Unknown),
    ("ZEROS", Section::Unknown),
    ("BOOTS", Section::Unknown),
    ("EVENTS", Section::Unknown),
    ("SCRATCH", Section::Unknown),
    ("CALIB", Section::Unknown),
];
// ANCHOR_END: answers
