//! Задача 0.A: что останется в release-бинарнике `examples/vanish.rs`.

// ANCHOR: answers
/// `Some(true)`, если символ функции есть в release-бинарнике примера.
pub const SURVIVES: [Option<bool>; 5] = [
    Some(false), // 1 crc_step: аргумент известен, при thin local LTO результат посчитан при компиляции
    Some(true),  // 2 checksum: black_box прячет данные, вызов остаётся
    Some(false), // 3 popcount: результат не нужен, вызов без побочных эффектов удалён
    Some(false), // 4 ch00a_exported: в объектном файле есть, компоновщик выбросил
    Some(true),  // 5 triple: адрес ушёл в black_box, тело обязано существовать
];
// ANCHOR_END: answers
